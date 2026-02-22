//! 依赖关系管理 API 处理器
//!
//! 实现徽章依赖关系的 CRUD 操作，支持前置条件、消耗关系和互斥关系的配置

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use crate::middleware::AuditContext;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::info;

use crate::{dto::ApiResponse, error::AdminError, state::AppState};

/// 创建依赖关系请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDependencyRequest {
    /// 依赖的徽章 ID（该徽章必须先获得）
    pub depends_on_badge_id: i64,
    /// 依赖类型：prerequisite（前置条件）、consume（消耗）、exclusive（互斥）
    pub dependency_type: String,
    /// 需要的数量，默认为 1
    #[serde(default = "default_quantity")]
    pub required_quantity: i32,
    /// 互斥组 ID（仅当 dependency_type 为 exclusive 时必填）
    pub exclusive_group_id: Option<String>,
    /// 是否自动触发级联评估
    #[serde(default)]
    pub auto_trigger: bool,
    /// 优先级，数值越小越先处理
    #[serde(default)]
    pub priority: i32,
    /// 依赖组 ID，同组内的条件是 AND 关系，不同组是 OR 关系
    pub dependency_group_id: String,
}

fn default_quantity() -> i32 {
    1
}

/// 依赖关系响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyResponse {
    pub id: i64,
    pub badge_id: i64,
    pub depends_on_badge_id: i64,
    pub dependency_type: String,
    pub required_quantity: i32,
    pub exclusive_group_id: Option<String>,
    pub auto_trigger: bool,
    pub priority: i32,
    pub dependency_group_id: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// 有效的依赖类型
const VALID_DEPENDENCY_TYPES: [&str; 3] = ["prerequisite", "consume", "exclusive"];

/// 创建依赖关系
///
/// POST /api/admin/badges/{badge_id}/dependencies
///
/// 为指定徽章添加依赖关系。依赖类型包括：
/// - prerequisite: 前置条件，用户必须先拥有依赖徽章
/// - consume: 消耗关系，获得徽章时会消耗指定数量的依赖徽章
/// - exclusive: 互斥关系，同一互斥组内的徽章只能拥有一个
pub async fn create_dependency(
    State(state): State<AppState>,
    Path(badge_id): Path<i64>,
    Json(req): Json<CreateDependencyRequest>,
) -> Result<(StatusCode, Json<ApiResponse<DependencyResponse>>), AdminError> {
    // 验证 dependency_type
    if !VALID_DEPENDENCY_TYPES.contains(&req.dependency_type.as_str()) {
        return Err(AdminError::Validation(format!(
            "无效的依赖类型: {}，有效值为: prerequisite, consume, exclusive",
            req.dependency_type
        )));
    }

    // 互斥类型必须指定互斥组 ID
    if req.dependency_type == "exclusive" && req.exclusive_group_id.is_none() {
        return Err(AdminError::Validation(
            "互斥类型必须指定 exclusiveGroupId".to_string(),
        ));
    }

    // 验证必填数量
    if req.required_quantity < 1 {
        return Err(AdminError::Validation(
            "requiredQuantity 必须大于等于 1".to_string(),
        ));
    }

    // 防止自引用
    if badge_id == req.depends_on_badge_id {
        return Err(AdminError::Validation("徽章不能依赖自己".to_string()));
    }

    let dependency_repo = state.dependency_repo()?;

    // 检测循环依赖
    let has_cycle = dependency_repo
        .check_circular_dependency(badge_id, req.depends_on_badge_id)
        .await?;
    if has_cycle {
        return Err(AdminError::Validation(
            "添加此依赖关系会导致循环依赖".to_string(),
        ));
    }

    let create_req = badge_management::repository::CreateDependencyRequest {
        badge_id,
        depends_on_badge_id: req.depends_on_badge_id,
        dependency_type: req.dependency_type.clone(),
        required_quantity: req.required_quantity,
        exclusive_group_id: req.exclusive_group_id.clone(),
        auto_trigger: req.auto_trigger,
        priority: req.priority,
        dependency_group_id: req.dependency_group_id.clone(),
    };

    let row = dependency_repo.create(&create_req).await?;

    info!(
        badge_id = badge_id,
        depends_on = req.depends_on_badge_id,
        dependency_type = %req.dependency_type,
        "Dependency created"
    );

    let response = DependencyResponse::from(row);
    Ok((StatusCode::CREATED, Json(ApiResponse::success(response))))
}

/// 获取徽章的所有依赖关系
///
/// GET /api/admin/badges/{badge_id}/dependencies
///
/// 返回指定徽章的所有前置依赖条件
pub async fn list_dependencies(
    State(state): State<AppState>,
    Path(badge_id): Path<i64>,
) -> Result<Json<ApiResponse<Vec<DependencyResponse>>>, AdminError> {
    let dependency_repo = state.dependency_repo()?;

    let rows = dependency_repo.get_prerequisites(badge_id).await?;
    let responses: Vec<DependencyResponse> =
        rows.into_iter().map(DependencyResponse::from).collect();

    Ok(Json(ApiResponse::success(responses)))
}

/// 删除依赖关系
///
/// DELETE /api/admin/dependencies/{id}
///
/// 删除指定的依赖关系配置
pub async fn delete_dependency(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(audit_ctx): Extension<AuditContext>,
) -> Result<Json<ApiResponse<()>>, AdminError> {
    let dependency_repo = state.dependency_repo()?;

    // 审计快照：记录变更前状态
    audit_ctx.snapshot(&state.pool, "badge_dependencies", id).await;

    let deleted = dependency_repo.delete(id).await?;

    if deleted {
        info!(dependency_id = id, "Dependency deleted");
        Ok(Json(ApiResponse::<()>::success_empty()))
    } else {
        Err(AdminError::DependencyNotFound(id))
    }
}

/// 刷新依赖关系缓存
///
/// POST /api/admin/cache/dependencies/refresh
///
/// 强制刷新级联评估器的依赖图缓存。当依赖关系配置发生变化后，
/// 可以调用此接口立即生效，而无需等待缓存自动过期。
///
/// 此方法会同时刷新：
/// 1. admin-service 本地的级联评估器缓存（如果已配置）
/// 2. badge-management-service 的级联评估器缓存（通过 gRPC 调用）
pub async fn refresh_dependency_cache(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<()>>, AdminError> {
    // 1. 刷新本地缓存（如果有）
    if let Some(ref evaluator) = state.cascade_evaluator {
        evaluator
            .refresh_cache()
            .await
            .map_err(|e| AdminError::Internal(format!("刷新本地依赖缓存失败: {}", e)))?;
        info!("Local dependency cache refreshed");
    }

    // 2. 通过 gRPC 刷新 badge-management-service 的缓存（受熔断器保护）
    let client_guard = state.badge_management_client.read().await;
    if let Some(client) = client_guard.clone() {
        drop(client_guard);
        use badge_proto::badge::RefreshDependencyCacheRequest;
        let cb = &state.badge_mgmt_circuit_breaker;
        let result = cb.call(|| {
            let mut c = client.clone();
            async move { c.refresh_dependency_cache(RefreshDependencyCacheRequest {}).await }
        }).await;

        match result {
            Ok(response) => {
                let resp = response.into_inner();
                if resp.success {
                    info!("Badge-management-service dependency cache refreshed: {}", resp.message);
                } else {
                    tracing::warn!("Badge-management-service cache refresh returned false: {}", resp.message);
                }
            }
            Err(e) => {
                // 熔断器跳闸或 gRPC 调用失败均不阻塞响应
                tracing::warn!("Failed to refresh badge-management-service cache via gRPC: {}", e);
            }
        }
    } else {
        drop(client_guard);
        tracing::debug!("Badge-management-service gRPC client not configured, skipping remote cache refresh");
    }

    Ok(Json(ApiResponse::<()>::success_empty()))
}

/// 刷新自动权益规则缓存
///
/// POST /api/admin/cache/auto-benefit/refresh
///
/// 强制刷新自动权益评估器的规则缓存。当兑换规则配置发生变化后
/// （特别是 auto_redeem=true 的规则），可以调用此接口立即生效。
pub async fn refresh_auto_benefit_cache(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<AutoBenefitCacheRefreshResult>>, AdminError> {
    // 通过 gRPC 刷新 badge-management-service 的缓存（受熔断器保护）
    let client_guard = state.badge_management_client.read().await;
    if let Some(client) = client_guard.clone() {
        drop(client_guard);
        use badge_proto::badge::RefreshAutoBenefitCacheRequest;
        let cb = &state.badge_mgmt_circuit_breaker;
        let result = cb.call(|| {
            let mut c = client.clone();
            async move { c.refresh_auto_benefit_cache(RefreshAutoBenefitCacheRequest {}).await }
        }).await;

        match result {
            Ok(response) => {
                let resp = response.into_inner();
                if resp.success {
                    info!(
                        rules_loaded = resp.rules_loaded,
                        "Badge-management-service auto-benefit cache refreshed: {}", resp.message
                    );
                    return Ok(Json(ApiResponse::success(AutoBenefitCacheRefreshResult {
                        rules_loaded: resp.rules_loaded,
                        message: resp.message,
                    })));
                } else {
                    tracing::warn!("Badge-management-service cache refresh returned false: {}", resp.message);
                    return Err(AdminError::Internal(resp.message));
                }
            }
            Err(e) => {
                tracing::error!("Failed to refresh badge-management-service auto-benefit cache via gRPC: {}", e);
                return Err(AdminError::Internal(format!("gRPC 调用失败: {}", e)));
            }
        }
    }
    drop(client_guard);

    // 没有配置 gRPC 客户端
    Err(AdminError::Internal("Badge-management-service gRPC 客户端未配置".to_string()))
}

/// 自动权益缓存刷新结果
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoBenefitCacheRefreshResult {
    pub rules_loaded: i32,
    pub message: String,
}

impl From<badge_management::repository::BadgeDependencyRow> for DependencyResponse {
    fn from(row: badge_management::repository::BadgeDependencyRow) -> Self {
        Self {
            id: row.id,
            badge_id: row.badge_id,
            depends_on_badge_id: row.depends_on_badge_id,
            dependency_type: row.dependency_type,
            required_quantity: row.required_quantity,
            exclusive_group_id: row.exclusive_group_id,
            auto_trigger: row.auto_trigger,
            priority: row.priority,
            dependency_group_id: row.dependency_group_id,
            enabled: row.enabled,
            created_at: row.created_at.to_rfc3339(),
            updated_at: row.updated_at.to_rfc3339(),
        }
    }
}

/// 更新依赖关系请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDependencyRequest {
    /// 依赖类型
    pub dependency_type: Option<String>,
    /// 需要的数量
    pub required_quantity: Option<i32>,
    /// 互斥组 ID
    pub exclusive_group_id: Option<String>,
    /// 是否自动触发
    pub auto_trigger: Option<bool>,
    /// 优先级
    pub priority: Option<i32>,
    /// 依赖组 ID
    pub dependency_group_id: Option<String>,
    /// 是否启用
    pub enabled: Option<bool>,
}

/// 更新依赖关系
///
/// PUT /api/admin/dependencies/{id}
///
/// 更新指定的依赖关系配置，支持部分更新
pub async fn update_dependency(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(audit_ctx): Extension<AuditContext>,
    Json(req): Json<UpdateDependencyRequest>,
) -> Result<Json<ApiResponse<DependencyResponse>>, AdminError> {
    // 验证 dependency_type
    if let Some(ref dep_type) = req.dependency_type {
        if !VALID_DEPENDENCY_TYPES.contains(&dep_type.as_str()) {
            return Err(AdminError::Validation(format!(
                "无效的依赖类型: {}，有效值为: prerequisite, consume, exclusive",
                dep_type
            )));
        }

        // 互斥类型必须指定互斥组 ID
        if dep_type == "exclusive" && req.exclusive_group_id.is_none() {
            return Err(AdminError::Validation(
                "互斥类型必须指定 exclusiveGroupId".to_string(),
            ));
        }
    }

    // 验证数量：最小值限制确保依赖数量有效
    if let Some(qty) = req.required_quantity
        && qty < 1
    {
        return Err(AdminError::Validation(
            "requiredQuantity 必须大于等于 1".to_string(),
        ));
    }

    let dependency_repo = state.dependency_repo()?;

    // 审计快照：记录变更前状态
    audit_ctx.snapshot(&state.pool, "badge_dependencies", id).await;

    let update_req = badge_management::repository::UpdateDependencyRequest {
        id,
        dependency_type: req.dependency_type,
        required_quantity: req.required_quantity,
        exclusive_group_id: req.exclusive_group_id,
        auto_trigger: req.auto_trigger,
        priority: req.priority,
        dependency_group_id: req.dependency_group_id,
        enabled: req.enabled,
    };

    let row = dependency_repo
        .update(&update_req)
        .await?
        .ok_or(AdminError::DependencyNotFound(id))?;

    info!(dependency_id = id, "Dependency updated");

    let response = DependencyResponse::from(row);
    Ok(Json(ApiResponse::success(response)))
}

/// 依赖图节点
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyGraphNode {
    pub id: String,
    pub badge_id: i64,
    pub label: String,
    /// 节点类型：root（根节点，查询的徽章）、prerequisite（前置条件）、dependent（依赖此徽章）
    pub node_type: String,
}

/// 依赖图边
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyGraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    /// 边类型：prerequisite、consume、exclusive
    pub edge_type: String,
    pub label: String,
}

/// 依赖图响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyGraphResponse {
    pub nodes: Vec<DependencyGraphNode>,
    pub edges: Vec<DependencyGraphEdge>,
}

/// 获取依赖图数据
///
/// GET /api/admin/dependencies/graph?badgeId={id}
///
/// 返回徽章依赖关系的图数据，用于前端可视化
pub async fn get_dependency_graph(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<DependencyGraphQuery>,
) -> Result<Json<ApiResponse<DependencyGraphResponse>>, AdminError> {
    let dependency_repo = state.dependency_repo()?;

    let all_deps = dependency_repo.list_all_enabled().await?;

    let mut nodes: Vec<DependencyGraphNode> = Vec::new();
    let mut edges: Vec<DependencyGraphEdge> = Vec::new();
    let mut node_ids: HashSet<i64> = HashSet::new();

    // 构建依赖图（正向：badge_id -> depends_on_badge_id 表示前者依赖后者）
    let mut deps_map: HashMap<i64, Vec<&badge_management::repository::BadgeDependencyRow>> =
        HashMap::new();
    // 反向：depends_on_badge_id -> badge_id 表示哪些徽章依赖这个徽章
    let mut reverse_deps_map: HashMap<i64, Vec<&badge_management::repository::BadgeDependencyRow>> =
        HashMap::new();

    for dep in &all_deps {
        deps_map.entry(dep.badge_id).or_default().push(dep);
        reverse_deps_map
            .entry(dep.depends_on_badge_id)
            .or_default()
            .push(dep);
    }

    // 如果指定了 badgeId，只返回相关的子图
    if let Some(badge_id) = params.badge_id {
        // 添加根节点
        nodes.push(DependencyGraphNode {
            id: format!("node_{}", badge_id),
            badge_id,
            label: format!("徽章 #{}", badge_id),
            node_type: "root".to_string(),
        });
        node_ids.insert(badge_id);

        // 递归添加前置条件（这个徽章依赖哪些徽章）
        fn add_prerequisites(
            badge_id: i64,
            deps_map: &HashMap<i64, Vec<&badge_management::repository::BadgeDependencyRow>>,
            nodes: &mut Vec<DependencyGraphNode>,
            edges: &mut Vec<DependencyGraphEdge>,
            node_ids: &mut HashSet<i64>,
            depth: i32,
        ) {
            if depth > 10 {
                return; // 防止过深递归
            }

            if let Some(deps) = deps_map.get(&badge_id) {
                for dep in deps {
                    let prereq_id = dep.depends_on_badge_id;

                    // 添加边
                    edges.push(DependencyGraphEdge {
                        id: format!("edge_{}", dep.id),
                        source: format!("node_{}", badge_id),
                        target: format!("node_{}", prereq_id),
                        edge_type: dep.dependency_type.clone(),
                        label: match dep.dependency_type.as_str() {
                            "prerequisite" => format!("需要 x{}", dep.required_quantity),
                            "consume" => format!("消耗 x{}", dep.required_quantity),
                            "exclusive" => format!("互斥: {}", dep.exclusive_group_id.as_deref().unwrap_or("-")),
                            _ => dep.dependency_type.clone(),
                        },
                    });

                    // 添加节点（如果尚未添加）
                    if !node_ids.contains(&prereq_id) {
                        nodes.push(DependencyGraphNode {
                            id: format!("node_{}", prereq_id),
                            badge_id: prereq_id,
                            label: format!("徽章 #{}", prereq_id),
                            node_type: "prerequisite".to_string(),
                        });
                        node_ids.insert(prereq_id);

                        // 继续递归
                        add_prerequisites(prereq_id, deps_map, nodes, edges, node_ids, depth + 1);
                    }
                }
            }
        }

        // 递归添加依赖者（哪些徽章依赖这个徽章）
        fn add_dependents(
            badge_id: i64,
            reverse_deps_map: &HashMap<i64, Vec<&badge_management::repository::BadgeDependencyRow>>,
            nodes: &mut Vec<DependencyGraphNode>,
            edges: &mut Vec<DependencyGraphEdge>,
            node_ids: &mut HashSet<i64>,
            depth: i32,
        ) {
            if depth > 10 {
                return;
            }

            if let Some(deps) = reverse_deps_map.get(&badge_id) {
                for dep in deps {
                    let dependent_id = dep.badge_id;

                    // 添加边
                    edges.push(DependencyGraphEdge {
                        id: format!("edge_{}", dep.id),
                        source: format!("node_{}", dependent_id),
                        target: format!("node_{}", badge_id),
                        edge_type: dep.dependency_type.clone(),
                        label: match dep.dependency_type.as_str() {
                            "prerequisite" => format!("需要 x{}", dep.required_quantity),
                            "consume" => format!("消耗 x{}", dep.required_quantity),
                            "exclusive" => format!("互斥: {}", dep.exclusive_group_id.as_deref().unwrap_or("-")),
                            _ => dep.dependency_type.clone(),
                        },
                    });

                    // 添加节点（如果尚未添加）
                    if !node_ids.contains(&dependent_id) {
                        nodes.push(DependencyGraphNode {
                            id: format!("node_{}", dependent_id),
                            badge_id: dependent_id,
                            label: format!("徽章 #{}", dependent_id),
                            node_type: "dependent".to_string(),
                        });
                        node_ids.insert(dependent_id);

                        // 继续递归
                        add_dependents(dependent_id, reverse_deps_map, nodes, edges, node_ids, depth + 1);
                    }
                }
            }
        }

        add_prerequisites(badge_id, &deps_map, &mut nodes, &mut edges, &mut node_ids, 0);
        add_dependents(badge_id, &reverse_deps_map, &mut nodes, &mut edges, &mut node_ids, 0);
    } else {
        // 返回完整的依赖图
        for dep in &all_deps {
            // 添加源节点
            if !node_ids.contains(&dep.badge_id) {
                nodes.push(DependencyGraphNode {
                    id: format!("node_{}", dep.badge_id),
                    badge_id: dep.badge_id,
                    label: format!("徽章 #{}", dep.badge_id),
                    node_type: "badge".to_string(),
                });
                node_ids.insert(dep.badge_id);
            }

            // 添加目标节点
            if !node_ids.contains(&dep.depends_on_badge_id) {
                nodes.push(DependencyGraphNode {
                    id: format!("node_{}", dep.depends_on_badge_id),
                    badge_id: dep.depends_on_badge_id,
                    label: format!("徽章 #{}", dep.depends_on_badge_id),
                    node_type: "badge".to_string(),
                });
                node_ids.insert(dep.depends_on_badge_id);
            }

            // 添加边
            edges.push(DependencyGraphEdge {
                id: format!("edge_{}", dep.id),
                source: format!("node_{}", dep.badge_id),
                target: format!("node_{}", dep.depends_on_badge_id),
                edge_type: dep.dependency_type.clone(),
                label: match dep.dependency_type.as_str() {
                    "prerequisite" => format!("需要 x{}", dep.required_quantity),
                    "consume" => format!("消耗 x{}", dep.required_quantity),
                    "exclusive" => format!("互斥: {}", dep.exclusive_group_id.as_deref().unwrap_or("-")),
                    _ => dep.dependency_type.clone(),
                },
            });
        }
    }

    Ok(Json(ApiResponse::success(DependencyGraphResponse { nodes, edges })))
}

/// 依赖图查询参数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyGraphQuery {
    /// 可选的徽章 ID，如果提供则只返回相关的子图
    pub badge_id: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_dependency_types() {
        assert!(VALID_DEPENDENCY_TYPES.contains(&"prerequisite"));
        assert!(VALID_DEPENDENCY_TYPES.contains(&"consume"));
        assert!(VALID_DEPENDENCY_TYPES.contains(&"exclusive"));
        assert!(!VALID_DEPENDENCY_TYPES.contains(&"invalid"));
    }

    #[test]
    fn test_default_quantity() {
        assert_eq!(default_quantity(), 1);
    }
}
