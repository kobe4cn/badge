//! 竞争兑换服务
//!
//! 处理需要消耗徽章的竞争兑换场景，通过分布式锁和行级锁确保并发安全。
//!
//! ## 核心流程
//!
//! 1. 获取分布式锁 -> 2. 获取依赖关系 -> 3. 检查互斥组约束
//! 4. 开启事务 + FOR UPDATE NOWAIT -> 5. 扣减消耗徽章 -> 6. 发放目标徽章
//! 7. 提交事务 -> 8. 释放分布式锁
//!
//! ## 并发控制策略
//!
//! - 分布式锁：防止同一用户同时发起多次兑换请求
//! - FOR UPDATE NOWAIT：数据库行级锁，快速失败避免死锁等待
//! - 事务原子性：扣减和发放在同一事务中，保证一致性

use std::sync::Arc;

use chrono::Utc;
use sqlx::PgPool;
use tracing::{info, instrument};
use uuid::Uuid;

use crate::error::{BadgeError, Result};
use crate::lock::LockManager;
use crate::repository::{BadgeDependencyRow, DependencyRepository, UserBadgeRepository};

/// 竞争兑换请求
#[derive(Debug)]
pub struct CompetitiveRedeemRequest {
    pub user_id: String,
    /// 目标徽章（要获得的徽章）
    pub target_badge_id: Uuid,
    /// 规则ID（用于锁定，可选）
    pub rule_id: Option<String>,
}

impl CompetitiveRedeemRequest {
    pub fn new(user_id: impl Into<String>, target_badge_id: Uuid) -> Self {
        Self {
            user_id: user_id.into(),
            target_badge_id,
            rule_id: None,
        }
    }

    pub fn with_rule_id(mut self, rule_id: impl Into<String>) -> Self {
        self.rule_id = Some(rule_id.into());
        self
    }
}

/// 竞争兑换响应
#[derive(Debug)]
pub struct CompetitiveRedeemResponse {
    pub success: bool,
    pub target_badge_id: Uuid,
    /// 消耗的徽章列表
    pub consumed_badges: Vec<ConsumedBadge>,
    /// 失败原因（仅在 success=false 时有值）
    pub failure_reason: Option<String>,
}

impl CompetitiveRedeemResponse {
    fn success(target_badge_id: Uuid, consumed_badges: Vec<ConsumedBadge>) -> Self {
        Self {
            success: true,
            target_badge_id,
            consumed_badges,
            failure_reason: None,
        }
    }

    fn failure(target_badge_id: Uuid, reason: impl Into<String>) -> Self {
        Self {
            success: false,
            target_badge_id,
            consumed_badges: vec![],
            failure_reason: Some(reason.into()),
        }
    }
}

/// 消耗的徽章信息
#[derive(Debug, Clone)]
pub struct ConsumedBadge {
    pub badge_id: Uuid,
    pub quantity: i32,
}

/// 用户徽章行数据（用于事务内查询）
#[derive(Debug, sqlx::FromRow)]
struct UserBadgeRow {
    id: i64,
    #[allow(dead_code)]
    user_id: String,
    #[allow(dead_code)]
    badge_id: i64,
    quantity: i32,
    #[allow(dead_code)]
    status: String,
}

/// 竞争兑换服务
///
/// 处理需要消耗徽章才能获得目标徽章的场景。通过双重锁机制（分布式锁 + 行级锁）
/// 确保在高并发情况下的数据一致性。
pub struct CompetitiveRedemptionService {
    pool: PgPool,
    lock_manager: Arc<LockManager>,
    user_badge_repo: Arc<UserBadgeRepository>,
    dependency_repo: Arc<DependencyRepository>,
}

impl CompetitiveRedemptionService {
    pub fn new(
        pool: PgPool,
        lock_manager: Arc<LockManager>,
        user_badge_repo: Arc<UserBadgeRepository>,
        dependency_repo: Arc<DependencyRepository>,
    ) -> Self {
        Self {
            pool,
            lock_manager,
            user_badge_repo,
            dependency_repo,
        }
    }

    /// 执行竞争兑换
    ///
    /// 完整流程：
    /// 1. 获取分布式锁（防止同一用户并发兑换）
    /// 2. 查询目标徽章的消耗类型依赖
    /// 3. 检查互斥组约束
    /// 4. 在事务中执行扣减和发放
    /// 5. 释放分布式锁
    #[instrument(skip(self), fields(user_id = %request.user_id, target = %request.target_badge_id))]
    pub async fn redeem(&self, request: CompetitiveRedeemRequest) -> Result<CompetitiveRedeemResponse> {
        // 1. 获取分布式锁，防止同一用户同时发起多次兑换
        let lock_key = format!("redeem:{}:{}", request.user_id, request.target_badge_id);
        let lock_guard = self.lock_manager.acquire(&lock_key, None).await?;

        // 2. 获取依赖关系（需要消耗的徽章）
        let prerequisites = self
            .dependency_repo
            .get_prerequisites(request.target_badge_id)
            .await?;

        // 筛选出消耗类型的依赖
        let consume_deps: Vec<_> = prerequisites
            .into_iter()
            .filter(|d| d.dependency_type.to_lowercase() == "consume")
            .collect();

        if consume_deps.is_empty() {
            lock_guard.release().await?;
            return Err(BadgeError::Validation(
                "目标徽章无需消耗其他徽章".to_string(),
            ));
        }

        // 3. 检查互斥组约束
        if let Err(e) = self.check_exclusive_constraints(&request.user_id, &consume_deps).await {
            lock_guard.release().await?;
            return Err(e);
        }

        // 4. 开启事务，使用 FOR UPDATE NOWAIT 锁定消耗徽章
        let result = self
            .execute_redemption_tx(&request.user_id, request.target_badge_id, &consume_deps)
            .await;

        // 5. 释放分布式锁
        lock_guard.release().await?;

        result
    }

    /// 检查互斥组约束
    ///
    /// 确保用户不会同时持有互斥组内的多个目标徽章
    async fn check_exclusive_constraints(
        &self,
        user_id: &str,
        consume_deps: &[BadgeDependencyRow],
    ) -> Result<()> {
        for dep in consume_deps {
            if let Some(ref group_id) = dep.exclusive_group_id {
                let group_badges = self.dependency_repo.get_exclusive_group(group_id).await?;

                // 检查用户是否已持有互斥组中的其他目标徽章
                for other_badge_id in group_badges {
                    // 跳过当前目标徽章
                    if other_badge_id != dep.badge_id {
                        if self.user_has_badge(user_id, other_badge_id).await? {
                            return Err(BadgeError::Validation(format!(
                                "互斥冲突：用户已持有互斥组 {} 中的徽章 {}",
                                group_id, other_badge_id
                            )));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 检查用户是否持有指定徽章
    async fn user_has_badge(&self, user_id: &str, badge_id: Uuid) -> Result<bool> {
        // badge_id 是 Uuid，需要转换为 i64 来查询
        // 实际项目中可能需要调整表结构或查询方式
        let badge_id_i64 = (badge_id.as_u128() & 0x7FFFFFFFFFFFFFFF) as i64;

        let user_badge = self.user_badge_repo.get_user_badge(user_id, badge_id_i64).await?;

        match user_badge {
            Some(ub) => Ok(ub.quantity > 0),
            None => Ok(false),
        }
    }

    /// 在事务中执行兑换
    ///
    /// 使用 FOR UPDATE NOWAIT 实现行级锁：
    /// - 如果行已被锁定，立即返回错误而不是等待
    /// - 避免死锁和长时间阻塞
    #[instrument(skip(self, consume_deps))]
    async fn execute_redemption_tx(
        &self,
        user_id: &str,
        target_badge_id: Uuid,
        consume_deps: &[BadgeDependencyRow],
    ) -> Result<CompetitiveRedeemResponse> {
        let mut tx = self.pool.begin().await?;
        let mut consumed = Vec::new();

        for dep in consume_deps {
            // FOR UPDATE NOWAIT 锁定用户徽章记录
            // 如果锁不可用立即失败，避免死锁等待
            let badge_id_i64 = (dep.depends_on_badge_id.as_u128() & 0x7FFFFFFFFFFFFFFF) as i64;

            let user_badge = sqlx::query_as::<_, UserBadgeRow>(
                r#"
                SELECT id, user_id, badge_id, quantity, status::text as status
                FROM user_badges
                WHERE user_id = $1 AND badge_id = $2 AND status = 'ACTIVE'
                FOR UPDATE NOWAIT
                "#,
            )
            .bind(user_id)
            .bind(badge_id_i64)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| {
                // PostgreSQL 在 NOWAIT 失败时返回特定错误码
                let err_str = e.to_string();
                if err_str.contains("could not obtain lock")
                    || err_str.contains("55P03") // lock_not_available
                {
                    BadgeError::LockConflict {
                        resource: format!("user_badge:{}:{}", user_id, dep.depends_on_badge_id),
                    }
                } else {
                    BadgeError::Database(e)
                }
            })?;

            let user_badge = match user_badge {
                Some(ub) => ub,
                None => {
                    // 缺少必需徽章，回滚事务并返回失败响应
                    tx.rollback().await?;
                    return Ok(CompetitiveRedeemResponse::failure(
                        target_badge_id,
                        format!("缺少必需徽章: {}", dep.depends_on_badge_id),
                    ));
                }
            };

            // 检查数量是否足够
            if user_badge.quantity < dep.required_quantity {
                tx.rollback().await?;
                return Ok(CompetitiveRedeemResponse::failure(
                    target_badge_id,
                    format!(
                        "徽章 {} 数量不足: 需要 {}, 拥有 {}",
                        dep.depends_on_badge_id, dep.required_quantity, user_badge.quantity
                    ),
                ));
            }

            // 计算扣减后的数量
            let new_quantity = user_badge.quantity - dep.required_quantity;

            // 扣减徽章数量
            if new_quantity > 0 {
                sqlx::query(
                    r#"UPDATE user_badges SET quantity = $1, updated_at = NOW() WHERE id = $2"#,
                )
                .bind(new_quantity)
                .bind(user_badge.id)
                .execute(&mut *tx)
                .await?;
            } else {
                // 数量归零，将状态标记为已兑换
                sqlx::query(
                    r#"UPDATE user_badges SET quantity = 0, status = 'REDEEMED', updated_at = NOW() WHERE id = $1"#,
                )
                .bind(user_badge.id)
                .execute(&mut *tx)
                .await?;
            }

            consumed.push(ConsumedBadge {
                badge_id: dep.depends_on_badge_id,
                quantity: dep.required_quantity,
            });

            info!(
                user_id = %user_id,
                badge_id = %dep.depends_on_badge_id,
                consumed_qty = dep.required_quantity,
                remaining_qty = new_quantity,
                "徽章已扣减"
            );
        }

        // 发放目标徽章
        // 使用 UPSERT 模式：如果已存在则增加数量，否则创建新记录
        let target_badge_id_i64 = (target_badge_id.as_u128() & 0x7FFFFFFFFFFFFFFF) as i64;
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO user_badges (user_id, badge_id, status, quantity, acquired_at, created_at, updated_at)
            VALUES ($1, $2, 'ACTIVE', 1, $3, $3, $3)
            ON CONFLICT (user_id, badge_id) DO UPDATE SET
                quantity = user_badges.quantity + 1,
                updated_at = $3
            "#,
        )
        .bind(user_id)
        .bind(target_badge_id_i64)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        info!(
            user_id = %user_id,
            target_badge_id = %target_badge_id,
            consumed_count = consumed.len(),
            "目标徽章已发放"
        );

        // 提交事务
        tx.commit().await?;

        Ok(CompetitiveRedeemResponse::success(target_badge_id, consumed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_competitive_redeem_request_new() {
        let request = CompetitiveRedeemRequest::new("user-123", Uuid::new_v4());
        assert_eq!(request.user_id, "user-123");
        assert!(request.rule_id.is_none());
    }

    #[test]
    fn test_competitive_redeem_request_with_rule_id() {
        let badge_id = Uuid::new_v4();
        let request = CompetitiveRedeemRequest::new("user-123", badge_id)
            .with_rule_id("rule-001");
        assert_eq!(request.user_id, "user-123");
        assert_eq!(request.target_badge_id, badge_id);
        assert_eq!(request.rule_id, Some("rule-001".to_string()));
    }

    #[test]
    fn test_competitive_redeem_request_builder_pattern() {
        // 测试链式调用的 builder 模式
        let badge_id = Uuid::new_v4();
        let request = CompetitiveRedeemRequest::new("user-456", badge_id)
            .with_rule_id("complex-rule-id");

        assert_eq!(request.user_id, "user-456");
        assert_eq!(request.target_badge_id, badge_id);
        assert_eq!(request.rule_id, Some("complex-rule-id".to_string()));
    }

    #[test]
    fn test_competitive_redeem_request_different_user_types() {
        // 测试不同类型的 user_id 输入（Into<String> trait）
        let badge_id = Uuid::new_v4();

        // 使用 &str
        let request1 = CompetitiveRedeemRequest::new("user-str", badge_id);
        assert_eq!(request1.user_id, "user-str");

        // 使用 String
        let request2 = CompetitiveRedeemRequest::new(String::from("user-string"), badge_id);
        assert_eq!(request2.user_id, "user-string");
    }

    #[test]
    fn test_competitive_redeem_response_success() {
        let badge_id = Uuid::new_v4();
        let consumed = vec![ConsumedBadge {
            badge_id: Uuid::new_v4(),
            quantity: 2,
        }];
        let response = CompetitiveRedeemResponse::success(badge_id, consumed);

        assert!(response.success);
        assert_eq!(response.target_badge_id, badge_id);
        assert_eq!(response.consumed_badges.len(), 1);
        assert!(response.failure_reason.is_none());
    }

    #[test]
    fn test_competitive_redeem_response_success_multiple_badges() {
        // 测试成功响应包含多个消耗徽章
        let target_badge_id = Uuid::new_v4();
        let consumed_badge_1 = ConsumedBadge {
            badge_id: Uuid::new_v4(),
            quantity: 1,
        };
        let consumed_badge_2 = ConsumedBadge {
            badge_id: Uuid::new_v4(),
            quantity: 3,
        };
        let consumed_badge_3 = ConsumedBadge {
            badge_id: Uuid::new_v4(),
            quantity: 2,
        };

        let response = CompetitiveRedeemResponse::success(
            target_badge_id,
            vec![consumed_badge_1.clone(), consumed_badge_2.clone(), consumed_badge_3.clone()],
        );

        assert!(response.success);
        assert_eq!(response.consumed_badges.len(), 3);
        assert_eq!(response.consumed_badges[0].quantity, 1);
        assert_eq!(response.consumed_badges[1].quantity, 3);
        assert_eq!(response.consumed_badges[2].quantity, 2);
        assert!(response.failure_reason.is_none());
    }

    #[test]
    fn test_competitive_redeem_response_success_empty_consumed() {
        // 边界情况：成功但没有消耗徽章（理论上不应发生，但结构上允许）
        let badge_id = Uuid::new_v4();
        let response = CompetitiveRedeemResponse::success(badge_id, vec![]);

        assert!(response.success);
        assert!(response.consumed_badges.is_empty());
        assert!(response.failure_reason.is_none());
    }

    #[test]
    fn test_competitive_redeem_response_failure() {
        let badge_id = Uuid::new_v4();
        let response = CompetitiveRedeemResponse::failure(badge_id, "徽章数量不足");

        assert!(!response.success);
        assert_eq!(response.target_badge_id, badge_id);
        assert!(response.consumed_badges.is_empty());
        assert_eq!(response.failure_reason, Some("徽章数量不足".to_string()));
    }

    #[test]
    fn test_competitive_redeem_response_failure_various_reasons() {
        // 测试不同的失败原因
        let badge_id = Uuid::new_v4();

        let response1 = CompetitiveRedeemResponse::failure(badge_id, "缺少必需徽章");
        assert_eq!(response1.failure_reason, Some("缺少必需徽章".to_string()));

        let response2 = CompetitiveRedeemResponse::failure(badge_id, format!("徽章 {} 数量不足: 需要 5, 拥有 2", badge_id));
        assert!(response2.failure_reason.unwrap().contains("数量不足"));

        let response3 = CompetitiveRedeemResponse::failure(badge_id, "互斥冲突：用户已持有互斥组中的徽章");
        assert!(response3.failure_reason.unwrap().contains("互斥冲突"));
    }

    #[test]
    fn test_consumed_badge_creation() {
        let badge_id = Uuid::new_v4();
        let consumed = ConsumedBadge {
            badge_id,
            quantity: 3,
        };
        assert_eq!(consumed.badge_id, badge_id);
        assert_eq!(consumed.quantity, 3);
    }

    #[test]
    fn test_consumed_badge_clone() {
        // 测试 ConsumedBadge 的 Clone trait
        let badge_id = Uuid::new_v4();
        let consumed = ConsumedBadge {
            badge_id,
            quantity: 5,
        };

        let cloned = consumed.clone();

        assert_eq!(cloned.badge_id, consumed.badge_id);
        assert_eq!(cloned.quantity, consumed.quantity);
    }

    #[test]
    fn test_consumed_badge_quantity_edge_cases() {
        // 测试数量边界值
        let badge_id = Uuid::new_v4();

        // 最小有效数量
        let consumed_min = ConsumedBadge {
            badge_id,
            quantity: 1,
        };
        assert_eq!(consumed_min.quantity, 1);

        // 较大数量
        let consumed_large = ConsumedBadge {
            badge_id,
            quantity: 1000,
        };
        assert_eq!(consumed_large.quantity, 1000);

        // 零数量（边界情况）
        let consumed_zero = ConsumedBadge {
            badge_id,
            quantity: 0,
        };
        assert_eq!(consumed_zero.quantity, 0);
    }

    #[test]
    fn test_lock_key_format() {
        // 验证锁 key 的格式符合预期
        let user_id = "user_123";
        let target_badge_id = Uuid::new_v4();

        let lock_key = format!("redeem:{}:{}", user_id, target_badge_id);

        assert!(lock_key.starts_with("redeem:"));
        assert!(lock_key.contains(user_id));
        assert!(lock_key.contains(&target_badge_id.to_string()));

        // 验证 key 的结构是 "redeem:{user_id}:{badge_id}"
        let parts: Vec<&str> = lock_key.split(':').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "redeem");
        assert_eq!(parts[1], user_id);
        assert_eq!(parts[2], target_badge_id.to_string());
    }

    #[test]
    fn test_lock_key_uniqueness() {
        // 测试不同用户/徽章组合产生不同的锁 key
        let user_id_1 = "user_A";
        let user_id_2 = "user_B";
        let badge_id_1 = Uuid::new_v4();
        let badge_id_2 = Uuid::new_v4();

        let key_1 = format!("redeem:{}:{}", user_id_1, badge_id_1);
        let key_2 = format!("redeem:{}:{}", user_id_2, badge_id_1);
        let key_3 = format!("redeem:{}:{}", user_id_1, badge_id_2);
        let key_4 = format!("redeem:{}:{}", user_id_1, badge_id_1);

        // 不同用户+相同徽章 -> 不同 key
        assert_ne!(key_1, key_2);
        // 相同用户+不同徽章 -> 不同 key
        assert_ne!(key_1, key_3);
        // 相同用户+相同徽章 -> 相同 key
        assert_eq!(key_1, key_4);
    }

    #[test]
    fn test_exclusive_group_conflict_scenario() {
        // 模拟互斥组冲突检查的场景
        // 假设用户持有徽章 A，而徽章 A 和徽章 B 在同一互斥组中
        // 当用户尝试兑换徽章 B 时应该失败

        let badge_id_a = Uuid::new_v4();
        let badge_id_b = Uuid::new_v4();
        let group_id = "exclusive-group-1";

        // 模拟错误消息格式
        let error_message = format!(
            "互斥冲突：用户已持有互斥组 {} 中的徽章 {}",
            group_id, badge_id_a
        );

        assert!(error_message.contains("互斥冲突"));
        assert!(error_message.contains(group_id));
        assert!(error_message.contains(&badge_id_a.to_string()));

        // 验证失败响应能正确携带错误信息
        let response = CompetitiveRedeemResponse::failure(badge_id_b, &error_message);
        assert!(!response.success);
        assert_eq!(response.target_badge_id, badge_id_b);
        assert!(response.failure_reason.as_ref().unwrap().contains("互斥冲突"));
    }

    #[test]
    fn test_badge_id_conversion_consistency() {
        // 测试 UUID 到 i64 转换的一致性（用于数据库查询）
        let badge_id = Uuid::new_v4();
        let badge_id_i64 = (badge_id.as_u128() & 0x7FFFFFFFFFFFFFFF) as i64;

        // 同一 UUID 应该总是产生相同的 i64
        let badge_id_i64_again = (badge_id.as_u128() & 0x7FFFFFFFFFFFFFFF) as i64;
        assert_eq!(badge_id_i64, badge_id_i64_again);

        // i64 应该是非负数（因为使用了掩码）
        assert!(badge_id_i64 >= 0);
    }
}
