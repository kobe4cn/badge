//! 依赖关系管理 API 处理器
//!
//! 实现徽章依赖关系的 CRUD 操作，支持前置条件、消耗关系和互斥关系的配置

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    dto::ApiResponse,
    error::AdminError,
    state::AppState,
};

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
        return Err(AdminError::Validation(
            "徽章不能依赖自己".to_string(),
        ));
    }

    let dependency_repo = state.dependency_repo()?;

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
    let responses: Vec<DependencyResponse> = rows.into_iter().map(DependencyResponse::from).collect();

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
) -> Result<Json<ApiResponse<()>>, AdminError> {
    let dependency_repo = state.dependency_repo()?;

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
pub async fn refresh_dependency_cache(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<()>>, AdminError> {
    if let Some(ref evaluator) = state.cascade_evaluator {
        evaluator.refresh_cache().await.map_err(|e| {
            AdminError::Internal(format!("刷新依赖缓存失败: {}", e))
        })?;
        info!("Dependency cache refreshed");
    }

    Ok(Json(ApiResponse::<()>::success_empty()))
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
