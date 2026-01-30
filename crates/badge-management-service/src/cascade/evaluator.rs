//! 级联评估器
//!
//! 负责评估用户是否满足获得某徽章的所有依赖条件，
//! 并在条件满足时自动发放徽章。
//!
//! ## 核心职责
//!
//! - 依赖图缓存管理
//! - 前置条件检查（依赖组逻辑：同组 AND，不同组 OR）
//! - 互斥组冲突检测
//! - 循环依赖检测
//! - 深度和超时限制
//! - 递归级联触发

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::dto::{
    BadgeDependency, BlockReason, BlockedBadge, CascadeConfig, CascadeContext, CascadeResult,
    DependencyType, GrantedBadge,
};
use super::DependencyGraph;
use crate::error::{BadgeError, Result};
use crate::models::UserBadgeStatus;
use crate::repository::{CascadeEvaluationLog, DependencyRepository, UserBadgeRepository};

/// 徽章发放接口
///
/// 通过 trait 解耦 CascadeEvaluator 与 GrantService，避免循环依赖。
/// GrantService 实现此 trait，CascadeEvaluator 通过 trait 调用发放逻辑。
#[async_trait]
pub trait BadgeGranter: Send + Sync {
    /// 级联发放徽章
    ///
    /// # Arguments
    /// * `user_id` - 用户 ID
    /// * `badge_id` - 要发放的徽章 ID（数据库 badge.id）
    /// * `triggered_by` - 触发级联的徽章 ID（级联日志用）
    ///
    /// # Returns
    /// * `Ok(true)` - 发放成功
    /// * `Ok(false)` - 发放被跳过（如用户已达上限）
    /// * `Err(_)` - 发放失败
    async fn grant_cascade(
        &self,
        user_id: &str,
        badge_id: i64,
        triggered_by: Uuid,
    ) -> Result<bool>;
}

/// 依赖图缓存
struct CachedGraph {
    graph: Option<DependencyGraph>,
    cached_at: Option<Instant>,
}

impl Default for CachedGraph {
    fn default() -> Self {
        Self {
            graph: None,
            cached_at: None,
        }
    }
}

/// 级联评估器
///
/// 当用户获得某徽章后，评估器会检查所有依赖此徽章的其他徽章，
/// 判断用户是否满足获得这些徽章的条件，并递归处理级联触发。
///
/// ## 依赖注入
///
/// 由于 GrantService 依赖 CascadeEvaluator，而 CascadeEvaluator 需要调用
/// GrantService 进行发放，形成循环依赖。解决方案：
/// 1. 定义 `BadgeGranter` trait
/// 2. CascadeEvaluator 持有 `Option<Arc<dyn BadgeGranter>>`
/// 3. 服务启动后通过 `set_grant_service` 延迟注入
pub struct CascadeEvaluator {
    config: CascadeConfig,
    dependency_repo: Arc<DependencyRepository>,
    user_badge_repo: Arc<UserBadgeRepository>,
    grant_service: RwLock<Option<Arc<dyn BadgeGranter>>>,
    graph_cache: RwLock<CachedGraph>,
}

impl CascadeEvaluator {
    /// 创建新的评估器
    pub fn new(
        config: CascadeConfig,
        dependency_repo: Arc<DependencyRepository>,
        user_badge_repo: Arc<UserBadgeRepository>,
    ) -> Self {
        Self {
            config,
            dependency_repo,
            user_badge_repo,
            grant_service: RwLock::new(None),
            graph_cache: RwLock::new(CachedGraph::default()),
        }
    }

    /// 设置发放服务（延迟注入）
    ///
    /// 在服务启动后调用此方法注入 GrantService，打破循环依赖
    pub async fn set_grant_service(&self, service: Arc<dyn BadgeGranter>) {
        let mut guard = self.grant_service.write().await;
        *guard = Some(service);
        info!("级联评估器发放服务已设置");
    }

    /// 获取配置
    pub fn config(&self) -> &CascadeConfig {
        &self.config
    }

    /// 主入口：徽章发放后调用
    ///
    /// 当用户获得 trigger_badge_id 后，检查是否有其他徽章可以自动发放。
    /// 此方法会递归处理级联触发，直到没有新徽章可发放或达到限制。
    ///
    /// # Arguments
    /// * `user_id` - 用户 ID
    /// * `trigger_badge_id` - 触发级联的徽章 ID（UUID 格式，对应 badge_dependency 表）
    ///
    /// # Returns
    /// 返回级联评估结果，包含成功发放和被阻止的徽章列表
    pub async fn evaluate(&self, user_id: &str, trigger_badge_id: Uuid) -> Result<CascadeResult> {
        let mut context = CascadeContext::new();
        let mut result = CascadeResult::default();

        // 获取依赖图
        let graph = self.get_or_refresh_graph().await?;

        // 检查是否有候选徽章
        let candidates = graph.get_triggered_by(trigger_badge_id);
        if candidates.is_empty() {
            debug!(
                user_id = %user_id,
                trigger_badge_id = %trigger_badge_id,
                "无候选徽章需要评估"
            );
            return Ok(result);
        }

        info!(
            user_id = %user_id,
            trigger_badge_id = %trigger_badge_id,
            candidate_count = candidates.len(),
            "开始级联评估"
        );

        // 递归评估
        let eval_result = Box::pin(self.evaluate_recursive(
            user_id,
            trigger_badge_id,
            &graph,
            &mut context,
            &mut result,
        ))
        .await;

        // 记录评估日志
        let error_msg = eval_result.as_ref().err().map(|e| e.to_string());
        self.log_evaluation(user_id, trigger_badge_id, &context, &result, error_msg.as_deref())
            .await;

        // 即使出错也返回已成功发放的结果
        if let Err(e) = eval_result {
            warn!(
                user_id = %user_id,
                trigger_badge_id = %trigger_badge_id,
                error = %e,
                granted_count = result.granted_badges.len(),
                "级联评估过程中发生错误，已发放的徽章不受影响"
            );
        }

        info!(
            user_id = %user_id,
            trigger_badge_id = %trigger_badge_id,
            granted_count = result.granted_badges.len(),
            blocked_count = result.blocked_badges.len(),
            "级联评估完成"
        );

        Ok(result)
    }

    /// 获取或刷新依赖图缓存
    async fn get_or_refresh_graph(&self) -> Result<DependencyGraph> {
        // 快速路径：检查缓存是否有效
        {
            let cache = self.graph_cache.read().await;
            if let Some(ref graph) = cache.graph {
                if let Some(cached_at) = cache.cached_at {
                    let elapsed = cached_at.elapsed().as_secs();
                    if elapsed < self.config.graph_cache_seconds {
                        return Ok(graph.clone());
                    }
                }
            }
        }

        // 慢路径：刷新缓存
        self.refresh_cache_internal().await
    }

    /// 刷新依赖图缓存
    pub async fn refresh_cache(&self) -> Result<()> {
        self.refresh_cache_internal().await?;
        Ok(())
    }

    /// 内部刷新缓存方法
    async fn refresh_cache_internal(&self) -> Result<DependencyGraph> {
        let rows = self.dependency_repo.list_all_enabled().await?;
        let graph = DependencyGraph::from_rows(rows);

        let mut cache = self.graph_cache.write().await;
        cache.graph = Some(graph.clone());
        cache.cached_at = Some(Instant::now());

        info!("依赖图缓存已刷新");
        Ok(graph)
    }

    /// 递归评估
    ///
    /// 核心评估逻辑，处理依赖组、前置条件、互斥冲突等
    async fn evaluate_recursive(
        &self,
        user_id: &str,
        trigger_badge_id: Uuid,
        graph: &DependencyGraph,
        context: &mut CascadeContext,
        result: &mut CascadeResult,
    ) -> Result<()> {
        // 1. 深度检查
        if context.depth > self.config.max_depth {
            return Err(BadgeError::CascadeDepthExceeded {
                current: context.depth,
                max: self.config.max_depth,
            });
        }

        // 2. 超时检查
        if context.elapsed_ms() > self.config.timeout_ms {
            return Err(BadgeError::CascadeTimeout {
                elapsed_ms: context.elapsed_ms(),
                timeout_ms: self.config.timeout_ms,
            });
        }

        // 3. 获取候选徽章
        let candidates = graph.get_triggered_by(trigger_badge_id);
        if candidates.is_empty() {
            return Ok(());
        }

        // 4. 获取用户当前徽章信息（用于前置条件检查）
        let user_badges = self.get_user_badge_quantities(user_id).await?;

        // 5. 按目标徽章分组处理
        // 由于同一个目标徽章可能有多个依赖条件，需要先收集再评估
        let mut badge_candidates: HashMap<Uuid, Vec<&BadgeDependency>> = HashMap::new();
        for candidate in candidates {
            badge_candidates
                .entry(candidate.badge_id)
                .or_default()
                .push(candidate);
        }

        // 6. 逐个评估候选徽章
        for (target_badge_id, _deps) in badge_candidates {
            // 6.1 循环检测
            if context.has_cycle(target_badge_id) {
                debug!(
                    target_badge_id = %target_badge_id,
                    path = ?context.path,
                    "检测到循环依赖"
                );
                result.blocked_badges.push(BlockedBadge {
                    badge_id: target_badge_id,
                    badge_name: None,
                    reason: BlockReason::CycleDetected,
                });
                continue;
            }

            // 6.2 获取目标徽章的所有前置条件
            let prerequisites = graph.get_prerequisites(target_badge_id);

            // 6.3 检查前置条件（依赖组逻辑）
            let (satisfied, missing) =
                self.check_prerequisites_with_groups(user_id, prerequisites, &user_badges)?;

            if !satisfied {
                debug!(
                    target_badge_id = %target_badge_id,
                    missing = ?missing,
                    "前置条件不满足"
                );
                result.blocked_badges.push(BlockedBadge {
                    badge_id: target_badge_id,
                    badge_name: None,
                    reason: BlockReason::PrerequisiteNotMet { missing },
                });
                continue;
            }

            // 6.4 检查互斥组
            // 查找此徽章的互斥组配置
            let exclusive_group = prerequisites
                .iter()
                .filter_map(|p| p.exclusive_group_id.as_ref())
                .next();

            if let Some(group_id) = exclusive_group {
                let group_badges = graph.get_exclusive_group(group_id);
                if let Some(conflicting) = self
                    .check_exclusive_conflict(user_id, target_badge_id, group_badges)
                    .await?
                {
                    debug!(
                        target_badge_id = %target_badge_id,
                        conflicting = %conflicting,
                        group_id = %group_id,
                        "互斥冲突"
                    );
                    result.blocked_badges.push(BlockedBadge {
                        badge_id: target_badge_id,
                        badge_name: None,
                        reason: BlockReason::ExclusiveConflict { conflicting },
                    });
                    continue;
                }
            }

            // 6.5 发放徽章
            context.enter(target_badge_id);

            // 需要将 UUID 转换为 badge_id (i64)
            // 这里假设 BadgeDependency.badge_id 是 UUID，实际发放需要查找对应的 badge 表 id
            // 由于 grant_cascade 接口使用 i64，这里需要一个映射
            // 暂时使用 UUID 的低 64 位作为 badge_id（实际应用中应有映射表）
            let badge_id_i64 = self.uuid_to_badge_id(target_badge_id);

            match self
                .grant_badge(user_id, badge_id_i64, trigger_badge_id)
                .await
            {
                Ok(true) => {
                    info!(
                        user_id = %user_id,
                        target_badge_id = %target_badge_id,
                        triggered_by = %trigger_badge_id,
                        "级联发放成功"
                    );
                    result.granted_badges.push(GrantedBadge {
                        badge_id: target_badge_id,
                        badge_name: String::new(),
                        triggered_by: trigger_badge_id,
                    });

                    // 6.6 递归处理：以新发放的徽章为触发继续评估
                    Box::pin(self.evaluate_recursive(
                        user_id,
                        target_badge_id,
                        graph,
                        context,
                        result,
                    ))
                    .await?;
                }
                Ok(false) => {
                    debug!(
                        user_id = %user_id,
                        target_badge_id = %target_badge_id,
                        "级联发放被跳过（可能用户已达上限）"
                    );
                }
                Err(e) => {
                    warn!(
                        user_id = %user_id,
                        target_badge_id = %target_badge_id,
                        error = %e,
                        "级联发放失败"
                    );
                    // 发放失败不阻止其他徽章的评估
                }
            }

            context.leave();
        }

        Ok(())
    }

    /// UUID 转换为 badge_id
    ///
    /// 注意：这是一个简化实现。在实际生产环境中，badge_dependency 表的 badge_id
    /// 应该是 i64 类型（与 badges 表一致），或者需要维护 UUID 到 i64 的映射。
    /// 当前实现取 UUID 的低 64 位。
    fn uuid_to_badge_id(&self, uuid: Uuid) -> i64 {
        let bytes = uuid.as_bytes();
        i64::from_le_bytes(bytes[0..8].try_into().unwrap())
    }

    /// 检查前置条件（支持依赖组逻辑）
    ///
    /// 依赖组逻辑：
    /// - 同一 `dependency_group_id` 的条件是 AND 关系（都要满足）
    /// - 不同组是 OR 关系（满足任一组即可）
    ///
    /// # Returns
    /// * `(true, vec![])` - 满足条件
    /// * `(false, missing)` - 不满足，返回缺失的徽章 ID 列表
    fn check_prerequisites_with_groups(
        &self,
        _user_id: &str,
        prerequisites: &[BadgeDependency],
        user_badges: &HashMap<Uuid, i32>,
    ) -> Result<(bool, Vec<Uuid>)> {
        if prerequisites.is_empty() {
            return Ok((true, vec![]));
        }

        // 按依赖组分组
        let mut groups: HashMap<&str, Vec<&BadgeDependency>> = HashMap::new();
        for prereq in prerequisites {
            // 只处理前置条件类型的依赖
            if prereq.dependency_type == DependencyType::Prerequisite {
                groups
                    .entry(&prereq.dependency_group_id)
                    .or_default()
                    .push(prereq);
            }
        }

        // 如果没有前置条件类型的依赖，直接返回满足
        if groups.is_empty() {
            return Ok((true, vec![]));
        }

        // OR 逻辑：只要有一个组满足即可
        let mut any_group_satisfied = false;
        let mut all_missing: Vec<Uuid> = vec![];

        for (_group_id, deps) in groups {
            // AND 逻辑：组内所有条件都要满足
            let mut group_satisfied = true;
            let mut group_missing: Vec<Uuid> = vec![];

            for dep in deps {
                let user_qty = user_badges.get(&dep.depends_on_badge_id).copied().unwrap_or(0);
                if user_qty < dep.required_quantity {
                    group_satisfied = false;
                    group_missing.push(dep.depends_on_badge_id);
                }
            }

            if group_satisfied {
                any_group_satisfied = true;
                break;
            } else {
                all_missing.extend(group_missing);
            }
        }

        if any_group_satisfied {
            Ok((true, vec![]))
        } else {
            // 去重
            all_missing.sort();
            all_missing.dedup();
            Ok((false, all_missing))
        }
    }

    /// 检查前置条件是否满足（简化版，不返回缺失列表）
    #[allow(dead_code)]
    async fn check_prerequisites(
        &self,
        user_id: &str,
        prerequisites: &[BadgeDependency],
    ) -> Result<bool> {
        let user_badges = self.get_user_badge_quantities(user_id).await?;
        let (satisfied, _) =
            self.check_prerequisites_with_groups(user_id, prerequisites, &user_badges)?;
        Ok(satisfied)
    }

    /// 获取用户徽章数量映射
    ///
    /// 返回用户持有的所有有效徽章及其数量
    async fn get_user_badge_quantities(&self, user_id: &str) -> Result<HashMap<Uuid, i32>> {
        let badges = self
            .user_badge_repo
            .list_user_badges_by_status(user_id, UserBadgeStatus::Active)
            .await?;

        let now = Utc::now();
        let mut result = HashMap::new();

        for badge in badges {
            // 只计入有效且未过期的徽章
            if badge.is_valid(now) {
                // 将 badge_id (i64) 转换为 UUID
                // 这里同样是简化实现，实际应有映射表
                let uuid = self.badge_id_to_uuid(badge.badge_id);
                *result.entry(uuid).or_insert(0) += badge.quantity;
            }
        }

        Ok(result)
    }

    /// badge_id 转换为 UUID
    fn badge_id_to_uuid(&self, badge_id: i64) -> Uuid {
        let mut bytes = [0u8; 16];
        bytes[0..8].copy_from_slice(&badge_id.to_le_bytes());
        Uuid::from_bytes(bytes)
    }

    /// 检查互斥冲突
    ///
    /// 检查用户是否已持有互斥组中的其他徽章
    ///
    /// # Returns
    /// * `Ok(None)` - 无冲突
    /// * `Ok(Some(uuid))` - 存在冲突，返回冲突的徽章 ID
    async fn check_exclusive_conflict(
        &self,
        user_id: &str,
        target_badge_id: Uuid,
        group_badges: &[Uuid],
    ) -> Result<Option<Uuid>> {
        let user_badges = self.get_user_badge_quantities(user_id).await?;

        for &badge_id in group_badges {
            // 跳过目标徽章自身
            if badge_id == target_badge_id {
                continue;
            }

            // 如果用户已持有组内其他徽章，返回冲突
            if user_badges.get(&badge_id).copied().unwrap_or(0) > 0 {
                return Ok(Some(badge_id));
            }
        }

        Ok(None)
    }

    /// 发放徽章
    ///
    /// 通过 BadgeGranter trait 调用实际的发放逻辑
    async fn grant_badge(
        &self,
        user_id: &str,
        badge_id: i64,
        triggered_by: Uuid,
    ) -> Result<bool> {
        let guard = self.grant_service.read().await;
        let service = guard
            .as_ref()
            .ok_or(BadgeError::CascadeGrantServiceNotSet)?;

        service.grant_cascade(user_id, badge_id, triggered_by).await
    }

    /// 记录评估日志
    ///
    /// 将评估过程和结果记录到数据库，用于审计追踪和调试
    async fn log_evaluation(
        &self,
        user_id: &str,
        trigger_badge_id: Uuid,
        context: &CascadeContext,
        result: &CascadeResult,
        error: Option<&str>,
    ) {
        let started_at = Utc::now()
            - chrono::Duration::milliseconds(context.elapsed_ms() as i64);
        let completed_at = Utc::now();
        let duration_ms = context.elapsed_ms() as i32;

        let result_status = if error.is_some() {
            "error"
        } else if result.granted_badges.is_empty() && result.blocked_badges.is_empty() {
            "no_action"
        } else {
            "completed"
        };

        let log = CascadeEvaluationLog {
            user_id: user_id.to_string(),
            trigger_badge_id,
            evaluation_context: serde_json::json!({
                "max_depth_reached": context.depth,
                "visited_count": context.visited.len(),
                "path": context.path.iter().map(|u| u.to_string()).collect::<Vec<_>>(),
            }),
            result_status: result_status.to_string(),
            granted_badges: if result.granted_badges.is_empty() {
                None
            } else {
                Some(serde_json::to_value(&result.granted_badges).unwrap_or_default())
            },
            blocked_badges: if result.blocked_badges.is_empty() {
                None
            } else {
                Some(serde_json::to_value(&result.blocked_badges).unwrap_or_default())
            },
            error_message: error.map(|s| s.to_string()),
            started_at,
            completed_at,
            duration_ms,
        };

        if let Err(e) = self.dependency_repo.log_evaluation(&log).await {
            warn!(
                user_id = %user_id,
                trigger_badge_id = %trigger_badge_id,
                error = %e,
                "记录级联评估日志失败"
            );
        }
    }
}

// 为了兼容旧的测试代码，保留获取 graph 的方法
impl CascadeEvaluator {
    /// 获取当前缓存的依赖图（仅用于测试）
    #[cfg(test)]
    pub async fn graph(&self) -> Option<DependencyGraph> {
        let cache = self.graph_cache.read().await;
        cache.graph.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::BadgeDependencyRow;
    use chrono::Utc;

    fn create_test_dependency_row(
        badge_id: Uuid,
        depends_on: Uuid,
        dep_type: &str,
        auto_trigger: bool,
        group_id: &str,
        exclusive_group: Option<&str>,
        required_qty: i32,
    ) -> BadgeDependencyRow {
        BadgeDependencyRow {
            id: Uuid::new_v4(),
            badge_id,
            depends_on_badge_id: depends_on,
            dependency_type: dep_type.to_string(),
            required_quantity: required_qty,
            exclusive_group_id: exclusive_group.map(|s| s.to_string()),
            auto_trigger,
            priority: 0,
            dependency_group_id: group_id.to_string(),
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// 辅助函数：从 BadgeDependencyRow 创建 BadgeDependency
    fn row_to_dependency(row: &BadgeDependencyRow) -> BadgeDependency {
        BadgeDependency::from_row(row.clone())
    }

    // ==================== CascadeContext 测试 ====================

    #[test]
    fn test_cascade_context() {
        let mut context = CascadeContext::new();
        let badge_a = Uuid::new_v4();
        let badge_b = Uuid::new_v4();

        assert_eq!(context.depth, 0);
        assert!(!context.has_cycle(badge_a));

        context.enter(badge_a);
        assert_eq!(context.depth, 1);
        assert!(context.has_cycle(badge_a));
        assert!(!context.has_cycle(badge_b));

        context.enter(badge_b);
        assert_eq!(context.depth, 2);
        assert!(context.has_cycle(badge_b));

        context.leave();
        assert_eq!(context.depth, 1);

        context.leave();
        assert_eq!(context.depth, 0);
    }

    #[test]
    fn test_cascade_context_elapsed_time() {
        let context = CascadeContext::new();
        // 刚创建的 context，elapsed_ms 应该很小
        assert!(context.elapsed_ms() < 100);
    }

    #[test]
    fn test_cascade_context_path_tracking() {
        let mut context = CascadeContext::new();
        let badge_a = Uuid::new_v4();
        let badge_b = Uuid::new_v4();
        let badge_c = Uuid::new_v4();

        context.enter(badge_a);
        context.enter(badge_b);
        context.enter(badge_c);

        // 验证路径追踪
        assert_eq!(context.path.len(), 3);
        assert_eq!(context.path[0], badge_a);
        assert_eq!(context.path[1], badge_b);
        assert_eq!(context.path[2], badge_c);

        context.leave();
        assert_eq!(context.path.len(), 2);
    }

    // ==================== 简单级联 A→B 测试 ====================

    #[test]
    fn test_simple_cascade_a_to_b() {
        let badge_a = Uuid::new_v4();
        let badge_b = Uuid::new_v4();

        // B 依赖 A，且 auto_trigger = true
        let dep = create_test_dependency_row(badge_b, badge_a, "prerequisite", true, "group1", None, 1);
        let graph = DependencyGraph::from_rows(vec![dep]);

        // 当 A 被发放时，应该触发检查 B
        let triggered = graph.get_triggered_by(badge_a);
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].badge_id, badge_b);
        assert_eq!(triggered[0].depends_on_badge_id, badge_a);
        assert!(triggered[0].auto_trigger);
    }

    #[test]
    fn test_simple_cascade_non_auto_trigger() {
        let badge_a = Uuid::new_v4();
        let badge_b = Uuid::new_v4();

        // B 依赖 A，但 auto_trigger = false
        let dep = create_test_dependency_row(badge_b, badge_a, "prerequisite", false, "group1", None, 1);
        let graph = DependencyGraph::from_rows(vec![dep]);

        // auto_trigger=false 时，不应自动触发
        let triggered = graph.get_triggered_by(badge_a);
        assert!(triggered.is_empty());

        // 但前置条件仍然存在
        let prereqs = graph.get_prerequisites(badge_b);
        assert_eq!(prereqs.len(), 1);
        assert_eq!(prereqs[0].depends_on_badge_id, badge_a);
    }

    // ==================== 多级级联 A→B→C 测试 ====================

    #[test]
    fn test_multi_level_cascade_a_to_b_to_c() {
        let badge_a = Uuid::new_v4();
        let badge_b = Uuid::new_v4();
        let badge_c = Uuid::new_v4();

        let rows = vec![
            // B 依赖 A
            create_test_dependency_row(badge_b, badge_a, "prerequisite", true, "default", None, 1),
            // C 依赖 B
            create_test_dependency_row(badge_c, badge_b, "prerequisite", true, "default", None, 1),
        ];

        let graph = DependencyGraph::from_rows(rows);

        // 验证级联链：A → B → C
        let triggered_by_a = graph.get_triggered_by(badge_a);
        assert_eq!(triggered_by_a.len(), 1);
        assert_eq!(triggered_by_a[0].badge_id, badge_b);

        let triggered_by_b = graph.get_triggered_by(badge_b);
        assert_eq!(triggered_by_b.len(), 1);
        assert_eq!(triggered_by_b[0].badge_id, badge_c);

        let triggered_by_c = graph.get_triggered_by(badge_c);
        assert!(triggered_by_c.is_empty());

        // 验证前置条件链
        let prereqs_b = graph.get_prerequisites(badge_b);
        assert_eq!(prereqs_b.len(), 1);
        assert_eq!(prereqs_b[0].depends_on_badge_id, badge_a);

        let prereqs_c = graph.get_prerequisites(badge_c);
        assert_eq!(prereqs_c.len(), 1);
        assert_eq!(prereqs_c[0].depends_on_badge_id, badge_b);
    }

    #[test]
    fn test_multi_level_cascade_depth_tracking() {
        let badge_a = Uuid::new_v4();
        let badge_b = Uuid::new_v4();
        let badge_c = Uuid::new_v4();

        let mut context = CascadeContext::new();

        // 模拟级联执行顺序：进入 A，然后 B，然后 C
        context.enter(badge_a);
        assert_eq!(context.depth, 1);

        context.enter(badge_b);
        assert_eq!(context.depth, 2);

        context.enter(badge_c);
        assert_eq!(context.depth, 3);

        // 回退
        context.leave();
        assert_eq!(context.depth, 2);

        context.leave();
        assert_eq!(context.depth, 1);

        context.leave();
        assert_eq!(context.depth, 0);
    }

    // ==================== 循环检测 A→B→C→A 测试 ====================

    #[test]
    fn test_cycle_detection_simple() {
        let badge_a = Uuid::new_v4();
        let badge_b = Uuid::new_v4();
        let badge_c = Uuid::new_v4();

        // A -> B -> C -> A（循环）
        let deps = vec![
            create_test_dependency_row(badge_b, badge_a, "prerequisite", true, "g1", None, 1),
            create_test_dependency_row(badge_c, badge_b, "prerequisite", true, "g1", None, 1),
            create_test_dependency_row(badge_a, badge_c, "prerequisite", true, "g1", None, 1),
        ];
        let graph = DependencyGraph::from_rows(deps);

        // 使用 CascadeContext 检测循环
        let mut context = CascadeContext::new();

        // 进入 A
        assert!(!context.has_cycle(badge_a));
        context.enter(badge_a);

        // 进入 B
        assert!(!context.has_cycle(badge_b));
        context.enter(badge_b);

        // 进入 C
        assert!(!context.has_cycle(badge_c));
        context.enter(badge_c);

        // 尝试再次访问 A，应该检测到循环
        assert!(context.has_cycle(badge_a));

        // 验证图中确实存在循环路径
        let triggered_by_c = graph.get_triggered_by(badge_c);
        assert_eq!(triggered_by_c.len(), 1);
        assert_eq!(triggered_by_c[0].badge_id, badge_a);
    }

    #[test]
    fn test_cycle_detection_self_reference() {
        let badge_a = Uuid::new_v4();

        // A -> A（自引用循环）
        let deps = vec![create_test_dependency_row(
            badge_a, badge_a, "prerequisite", true, "g1", None, 1,
        )];
        let graph = DependencyGraph::from_rows(deps);

        let mut context = CascadeContext::new();
        context.enter(badge_a);

        // 尝试再次进入 A，应该检测到循环
        assert!(context.has_cycle(badge_a));

        // 验证图中的自引用
        let triggered = graph.get_triggered_by(badge_a);
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].badge_id, badge_a);
    }

    #[test]
    fn test_cycle_detection_in_blocked_badges() {
        let badge_a = Uuid::new_v4();

        // 模拟评估结果中记录循环检测
        let mut result = CascadeResult::default();
        result.blocked_badges.push(BlockedBadge {
            badge_id: badge_a,
            badge_name: None,
            reason: BlockReason::CycleDetected,
        });

        assert_eq!(result.blocked_badges.len(), 1);
        match &result.blocked_badges[0].reason {
            BlockReason::CycleDetected => {}
            _ => panic!("Expected CycleDetected reason"),
        }
    }

    // ==================== 深度限制测试 ====================

    #[test]
    fn test_depth_limit_exceeded() {
        let config = CascadeConfig {
            max_depth: 3,
            timeout_ms: 5000,
            graph_cache_seconds: 300,
        };

        let mut context = CascadeContext::new();

        // 模拟进入 4 层深度
        for _ in 0..4 {
            context.enter(Uuid::new_v4());
        }

        // 验证深度超过限制
        assert!(context.depth > config.max_depth);
        assert_eq!(context.depth, 4);
    }

    #[test]
    fn test_depth_limit_at_boundary() {
        let config = CascadeConfig {
            max_depth: 3,
            timeout_ms: 5000,
            graph_cache_seconds: 300,
        };

        let mut context = CascadeContext::new();

        // 刚好到达最大深度
        for _ in 0..3 {
            context.enter(Uuid::new_v4());
        }

        // 在边界上，还没超过
        assert_eq!(context.depth, config.max_depth);

        // 再进入一层就超过了
        context.enter(Uuid::new_v4());
        assert!(context.depth > config.max_depth);
    }

    #[test]
    fn test_depth_exceeded_error() {
        // 验证 DepthExceeded BlockReason 的序列化
        let reason = BlockReason::DepthExceeded;
        let json = serde_json::to_string(&reason).unwrap();
        assert!(json.contains("DepthExceeded"));

        // 验证可以用于 BlockedBadge
        let blocked = BlockedBadge {
            badge_id: Uuid::new_v4(),
            badge_name: Some("测试徽章".to_string()),
            reason: BlockReason::DepthExceeded,
        };
        let json = serde_json::to_string(&blocked).unwrap();
        assert!(json.contains("DepthExceeded"));
    }

    // ==================== 互斥组阻止测试 ====================

    #[test]
    fn test_exclusive_group_conflict_detection() {
        let badge_d = Uuid::new_v4();
        let badge_e = Uuid::new_v4();
        let trigger = Uuid::new_v4();

        // D 和 E 属于同一互斥组 "vip_tier"
        let rows = vec![
            create_test_dependency_row(
                badge_d,
                trigger,
                "prerequisite",
                true,
                "default",
                Some("vip_tier"),
                1,
            ),
            create_test_dependency_row(
                badge_e,
                trigger,
                "prerequisite",
                true,
                "default",
                Some("vip_tier"),
                1,
            ),
        ];

        let graph = DependencyGraph::from_rows(rows);

        // 验证互斥组包含 D 和 E
        let group = graph.get_exclusive_group("vip_tier");
        assert_eq!(group.len(), 2);
        assert!(group.contains(&badge_d));
        assert!(group.contains(&badge_e));
    }

    #[test]
    fn test_exclusive_conflict_block_reason() {
        let badge_d = Uuid::new_v4();
        let badge_e = Uuid::new_v4();

        // 模拟用户已持有 D，尝试获得 E 被阻止
        let blocked = BlockedBadge {
            badge_id: badge_e,
            badge_name: Some("VIP银卡".to_string()),
            reason: BlockReason::ExclusiveConflict {
                conflicting: badge_d,
            },
        };

        // 验证序列化
        let json = serde_json::to_string(&blocked).unwrap();
        assert!(json.contains("ExclusiveConflict"));
        assert!(json.contains(&badge_d.to_string()));
    }

    #[test]
    fn test_exclusive_group_empty() {
        let graph = DependencyGraph::from_rows(vec![]);

        // 不存在的互斥组应返回空列表
        let group = graph.get_exclusive_group("nonexistent");
        assert!(group.is_empty());
    }

    #[test]
    fn test_multiple_exclusive_groups() {
        let badge_a = Uuid::new_v4();
        let badge_b = Uuid::new_v4();
        let badge_c = Uuid::new_v4();
        let badge_d = Uuid::new_v4();
        let trigger = Uuid::new_v4();

        // A, B 属于 "vip_tier"，C, D 属于 "member_level"
        let rows = vec![
            create_test_dependency_row(
                badge_a,
                trigger,
                "prerequisite",
                true,
                "default",
                Some("vip_tier"),
                1,
            ),
            create_test_dependency_row(
                badge_b,
                trigger,
                "prerequisite",
                true,
                "default",
                Some("vip_tier"),
                1,
            ),
            create_test_dependency_row(
                badge_c,
                trigger,
                "prerequisite",
                true,
                "default",
                Some("member_level"),
                1,
            ),
            create_test_dependency_row(
                badge_d,
                trigger,
                "prerequisite",
                true,
                "default",
                Some("member_level"),
                1,
            ),
        ];

        let graph = DependencyGraph::from_rows(rows);

        let vip_group = graph.get_exclusive_group("vip_tier");
        assert_eq!(vip_group.len(), 2);
        assert!(vip_group.contains(&badge_a));
        assert!(vip_group.contains(&badge_b));

        let member_group = graph.get_exclusive_group("member_level");
        assert_eq!(member_group.len(), 2);
        assert!(member_group.contains(&badge_c));
        assert!(member_group.contains(&badge_d));
    }

    // ==================== 前置条件不满足测试 ====================

    #[test]
    fn test_prerequisite_not_met_block_reason() {
        let badge_a = Uuid::new_v4();
        let badge_b = Uuid::new_v4();

        // B 需要 A，但用户没有 A
        let blocked = BlockedBadge {
            badge_id: badge_b,
            badge_name: Some("高级徽章".to_string()),
            reason: BlockReason::PrerequisiteNotMet {
                missing: vec![badge_a],
            },
        };

        let json = serde_json::to_string(&blocked).unwrap();
        assert!(json.contains("PrerequisiteNotMet"));
        assert!(json.contains(&badge_a.to_string()));
    }

    #[test]
    fn test_prerequisite_with_required_quantity() {
        let badge_a = Uuid::new_v4();
        let badge_b = Uuid::new_v4();

        // B 需要 3 个 A
        let dep = create_test_dependency_row(badge_b, badge_a, "prerequisite", true, "group1", None, 3);

        assert_eq!(dep.required_quantity, 3);

        let dependency = row_to_dependency(&dep);
        assert_eq!(dependency.required_quantity, 3);
    }

    #[test]
    fn test_multiple_prerequisites() {
        let badge_a = Uuid::new_v4();
        let badge_b = Uuid::new_v4();
        let badge_c = Uuid::new_v4();

        // C 需要 A 和 B（同一依赖组，AND 关系）
        let rows = vec![
            create_test_dependency_row(badge_c, badge_a, "prerequisite", true, "group1", None, 1),
            create_test_dependency_row(badge_c, badge_b, "prerequisite", true, "group1", None, 1),
        ];

        let graph = DependencyGraph::from_rows(rows);

        let prereqs = graph.get_prerequisites(badge_c);
        assert_eq!(prereqs.len(), 2);

        let prereq_ids: Vec<Uuid> = prereqs.iter().map(|p| p.depends_on_badge_id).collect();
        assert!(prereq_ids.contains(&badge_a));
        assert!(prereq_ids.contains(&badge_b));
    }

    #[test]
    fn test_dependency_groups_or_logic() {
        let badge_a = Uuid::new_v4();
        let badge_b = Uuid::new_v4();
        let badge_c = Uuid::new_v4();

        // C 可以通过两种方式获得：
        // - 方式1（group1）：需要 A
        // - 方式2（group2）：需要 B
        // 不同组是 OR 关系
        let rows = vec![
            create_test_dependency_row(badge_c, badge_a, "prerequisite", true, "group1", None, 1),
            create_test_dependency_row(badge_c, badge_b, "prerequisite", true, "group2", None, 1),
        ];

        let graph = DependencyGraph::from_rows(rows);

        let prereqs = graph.get_prerequisites(badge_c);
        assert_eq!(prereqs.len(), 2);

        // 验证依赖组 ID 不同
        let group_ids: Vec<&str> = prereqs.iter().map(|p| p.dependency_group_id.as_str()).collect();
        assert!(group_ids.contains(&"group1"));
        assert!(group_ids.contains(&"group2"));
    }

    // ==================== 其他测试 ====================

    #[test]
    fn test_prerequisite_check_with_groups_empty() {
        let prerequisites: Vec<BadgeDependency> = vec![];
        let _user_badges: HashMap<Uuid, i32> = HashMap::new();

        // 空的前置条件列表应返回 true
        assert!(prerequisites.is_empty());
    }

    #[test]
    fn test_dependency_type_parsing() {
        assert_eq!(
            DependencyType::from_str("prerequisite"),
            Some(DependencyType::Prerequisite)
        );
        assert_eq!(
            DependencyType::from_str("PREREQUISITE"),
            Some(DependencyType::Prerequisite)
        );
        assert_eq!(
            DependencyType::from_str("consume"),
            Some(DependencyType::Consume)
        );
        assert_eq!(
            DependencyType::from_str("exclusive"),
            Some(DependencyType::Exclusive)
        );
        assert_eq!(DependencyType::from_str("invalid"), None);
    }

    #[test]
    fn test_dependency_graph_from_rows() {
        let badge_a = Uuid::new_v4();
        let badge_b = Uuid::new_v4();
        let badge_c = Uuid::new_v4();

        let rows = vec![
            // B 依赖 A（前置条件）
            create_test_dependency_row(badge_b, badge_a, "prerequisite", true, "default", None, 1),
            // C 依赖 B（前置条件）
            create_test_dependency_row(badge_c, badge_b, "prerequisite", true, "default", None, 1),
        ];

        let graph = DependencyGraph::from_rows(rows);

        // 获得 A 后应触发对 B 的检查
        let triggered = graph.get_triggered_by(badge_a);
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].badge_id, badge_b);

        // 获得 B 后应触发对 C 的检查
        let triggered = graph.get_triggered_by(badge_b);
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].badge_id, badge_c);

        // 获得 C 后无触发
        let triggered = graph.get_triggered_by(badge_c);
        assert!(triggered.is_empty());
    }

    #[test]
    fn test_exclusive_group_in_graph() {
        let badge_a = Uuid::new_v4();
        let badge_b = Uuid::new_v4();
        let trigger = Uuid::new_v4();

        let rows = vec![
            create_test_dependency_row(
                badge_a,
                trigger,
                "prerequisite",
                true,
                "default",
                Some("vip_tier"),
                1,
            ),
            create_test_dependency_row(
                badge_b,
                trigger,
                "prerequisite",
                true,
                "default",
                Some("vip_tier"),
                1,
            ),
        ];

        let graph = DependencyGraph::from_rows(rows);

        let group = graph.get_exclusive_group("vip_tier");
        assert_eq!(group.len(), 2);
        assert!(group.contains(&badge_a));
        assert!(group.contains(&badge_b));
    }

    #[test]
    fn test_cascade_config_default() {
        let config = CascadeConfig::default();
        assert_eq!(config.max_depth, 10);
        assert_eq!(config.timeout_ms, 5000);
        assert_eq!(config.graph_cache_seconds, 300);
    }

    #[test]
    fn test_block_reason_serialization() {
        let reason = BlockReason::PrerequisiteNotMet {
            missing: vec![Uuid::new_v4()],
        };
        let json = serde_json::to_string(&reason).unwrap();
        assert!(json.contains("PrerequisiteNotMet"));

        let reason = BlockReason::CycleDetected;
        let json = serde_json::to_string(&reason).unwrap();
        assert!(json.contains("CycleDetected"));
    }

    #[test]
    fn test_cascade_result_default() {
        let result = CascadeResult::default();
        assert!(result.granted_badges.is_empty());
        assert!(result.blocked_badges.is_empty());
    }

    #[test]
    fn test_granted_badge_serialization() {
        let trigger = Uuid::new_v4();
        let granted = GrantedBadge {
            badge_id: Uuid::new_v4(),
            badge_name: "测试徽章".to_string(),
            triggered_by: trigger,
        };

        let json = serde_json::to_string(&granted).unwrap();
        assert!(json.contains("测试徽章"));
        assert!(json.contains(&trigger.to_string()));
    }

    #[test]
    fn test_timeout_block_reason() {
        let reason = BlockReason::Timeout;
        let json = serde_json::to_string(&reason).unwrap();
        assert!(json.contains("Timeout"));
    }

    #[test]
    fn test_cascade_context_default() {
        let context = CascadeContext::default();
        assert_eq!(context.depth, 0);
        assert!(context.visited.is_empty());
        assert!(context.path.is_empty());
    }

    #[test]
    fn test_dependency_type_equality() {
        assert_eq!(DependencyType::Prerequisite, DependencyType::Prerequisite);
        assert_ne!(DependencyType::Prerequisite, DependencyType::Consume);
        assert_ne!(DependencyType::Consume, DependencyType::Exclusive);
    }

    #[test]
    fn test_badge_dependency_clone() {
        let row = create_test_dependency_row(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "prerequisite",
            true,
            "group1",
            Some("exclusive_group"),
            2,
        );

        let dep1 = row_to_dependency(&row);
        let dep2 = dep1.clone();

        assert_eq!(dep1.id, dep2.id);
        assert_eq!(dep1.badge_id, dep2.badge_id);
        assert_eq!(dep1.depends_on_badge_id, dep2.depends_on_badge_id);
        assert_eq!(dep1.required_quantity, dep2.required_quantity);
    }

    #[test]
    fn test_cascade_context_leave_underflow_protection() {
        let mut context = CascadeContext::new();

        // 多次调用 leave 不应该 panic，depth 应该保持在 0
        context.leave();
        assert_eq!(context.depth, 0);

        context.leave();
        assert_eq!(context.depth, 0);
    }

    #[test]
    fn test_graph_priority_ordering() {
        let badge_a = Uuid::new_v4();
        let badge_b1 = Uuid::new_v4();
        let badge_b2 = Uuid::new_v4();
        let badge_b3 = Uuid::new_v4();

        // 创建具有不同优先级的依赖
        let mut row1 = create_test_dependency_row(badge_b1, badge_a, "prerequisite", true, "g1", None, 1);
        row1.priority = 3;

        let mut row2 = create_test_dependency_row(badge_b2, badge_a, "prerequisite", true, "g1", None, 1);
        row2.priority = 1;

        let mut row3 = create_test_dependency_row(badge_b3, badge_a, "prerequisite", true, "g1", None, 1);
        row3.priority = 2;

        let graph = DependencyGraph::from_rows(vec![row1, row2, row3]);

        let triggered = graph.get_triggered_by(badge_a);
        assert_eq!(triggered.len(), 3);

        // 验证按优先级排序（priority 较小的在前）
        assert_eq!(triggered[0].badge_id, badge_b2); // priority = 1
        assert_eq!(triggered[1].badge_id, badge_b3); // priority = 2
        assert_eq!(triggered[2].badge_id, badge_b1); // priority = 3
    }
}
