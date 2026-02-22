//! API Key 管理 API 处理器
//!
//! 提供外部系统 API Key 的创建、查询、删除和重新生成功能。
//! API Key 采用 SHA256 哈希存储，仅在创建和重新生成时返回明文。

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use crate::middleware::AuditContext;
use chrono::{DateTime, Utc};
use rand::Rng;
use sha2::{Digest, Sha256};
use tracing::info;
use validator::Validate;

use crate::{
    dto::{ApiResponse, PageResponse, PaginationParams},
    error::AdminError,
    state::AppState,
};

/// API Key 前缀，用于识别 key 类型
const KEY_PREFIX: &str = "bk_";

/// API Key 长度（不含前缀）
const KEY_LENGTH: usize = 32;

/// 创建 API Key 请求
#[derive(Debug, serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyRequest {
    /// Key 名称（用于标识用途）
    #[validate(length(min = 1, max = 100, message = "名称长度应在 1-100 个字符"))]
    pub name: String,
    /// 允许的权限码列表
    pub permissions: Vec<String>,
    /// 每分钟请求限制
    pub rate_limit: Option<i32>,
    /// 过期时间（可选）
    pub expires_at: Option<DateTime<Utc>>,
}

/// API Key DTO（列表展示，不含完整 key）
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyDto {
    pub id: i64,
    pub name: String,
    pub key_prefix: String,
    pub permissions: Vec<String>,
    pub rate_limit: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// 创建 API Key 响应（含完整 key，仅此一次展示）
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyResponse {
    pub id: i64,
    pub name: String,
    /// 完整的 API Key（仅在创建或重新生成时返回）
    pub api_key: String,
    pub permissions: Vec<String>,
    pub rate_limit: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// 数据库查询行
#[derive(sqlx::FromRow)]
struct ApiKeyRow {
    id: i64,
    name: String,
    key_prefix: String,
    permissions: sqlx::types::JsonValue,
    rate_limit: Option<i32>,
    expires_at: Option<DateTime<Utc>>,
    enabled: bool,
    last_used_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl From<ApiKeyRow> for ApiKeyDto {
    fn from(row: ApiKeyRow) -> Self {
        let permissions: Vec<String> = serde_json::from_value(row.permissions)
            .unwrap_or_default();
        Self {
            id: row.id,
            name: row.name,
            key_prefix: row.key_prefix,
            permissions,
            rate_limit: row.rate_limit,
            expires_at: row.expires_at,
            enabled: row.enabled,
            last_used_at: row.last_used_at,
            created_at: row.created_at,
        }
    }
}

/// 生成随机 API Key
fn generate_api_key() -> String {
    let mut rng = rand::rng();
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
        .chars()
        .collect();
    let key: String = (0..KEY_LENGTH)
        .map(|_| chars[rng.random_range(0..chars.len())])
        .collect();
    format!("{}{}", KEY_PREFIX, key)
}

/// 计算 API Key 的 SHA256 哈希
fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// 获取 API Key 列表
///
/// GET /api/admin/system/api-keys
pub async fn list_api_keys(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<ApiResponse<PageResponse<ApiKeyDto>>>, AdminError> {
    let offset = pagination.offset();
    let limit = pagination.limit();

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM api_key")
        .fetch_one(&state.pool)
        .await?;

    if total.0 == 0 {
        return Ok(Json(ApiResponse::success(PageResponse::empty(
            pagination.page,
            pagination.page_size,
        ))));
    }

    let rows = sqlx::query_as::<_, ApiKeyRow>(
        r#"
        SELECT id, name, key_prefix, permissions, rate_limit, expires_at,
               enabled, last_used_at, created_at
        FROM api_key
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let items: Vec<ApiKeyDto> = rows.into_iter().map(Into::into).collect();
    let response = PageResponse::new(items, total.0, pagination.page, pagination.page_size);
    Ok(Json(ApiResponse::success(response)))
}

/// 创建 API Key
///
/// POST /api/admin/system/api-keys
///
/// 创建成功后返回完整的 API Key，此 key 仅展示一次
pub async fn create_api_key(
    State(state): State<AppState>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiResponse<CreateApiKeyResponse>>, AdminError> {
    req.validate()?;

    // 生成 key 和哈希
    let api_key = generate_api_key();
    let key_hash = hash_api_key(&api_key);
    let permissions_json = serde_json::to_value(&req.permissions)
        .map_err(|e| AdminError::Internal(format!("序列化权限失败: {}", e)))?;

    let now = Utc::now();

    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO api_key (name, key_prefix, key_hash, permissions, rate_limit, expires_at, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id
        "#,
    )
    .bind(&req.name)
    .bind(KEY_PREFIX)
    .bind(&key_hash)
    .bind(&permissions_json)
    .bind(req.rate_limit.unwrap_or(1000))
    .bind(req.expires_at)
    .bind(now)
    .fetch_one(&state.pool)
    .await?;

    info!(api_key_id = row.0, name = %req.name, "API Key created");

    let response = CreateApiKeyResponse {
        id: row.0,
        name: req.name,
        api_key,
        permissions: req.permissions,
        rate_limit: req.rate_limit,
        expires_at: req.expires_at,
        created_at: now,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 删除 API Key
///
/// DELETE /api/admin/system/api-keys/:id
pub async fn delete_api_key(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(audit_ctx): Extension<AuditContext>,
) -> Result<Json<ApiResponse<()>>, AdminError> {
    // 审计快照：记录变更前状态
    audit_ctx.snapshot(&state.pool, "api_key", id).await;

    let result = sqlx::query("DELETE FROM api_key WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AdminError::NotFound(format!("API Key {} 不存在", id)));
    }

    info!(api_key_id = id, "API Key deleted");
    Ok(Json(ApiResponse::<()>::success_empty()))
}

/// 重新生成 API Key
///
/// POST /api/admin/system/api-keys/:id/regenerate
///
/// 生成新的 key 并更新哈希，旧 key 立即失效
pub async fn regenerate_api_key(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<CreateApiKeyResponse>>, AdminError> {
    // 检查 key 是否存在
    #[allow(clippy::type_complexity)]
    let existing: Option<(String, sqlx::types::JsonValue, Option<i32>, Option<DateTime<Utc>>)> =
        sqlx::query_as("SELECT name, permissions, rate_limit, expires_at FROM api_key WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.pool)
            .await?;

    let existing = existing.ok_or_else(|| AdminError::NotFound(format!("API Key {} 不存在", id)))?;

    // 生成新 key
    let api_key = generate_api_key();
    let key_hash = hash_api_key(&api_key);
    let now = Utc::now();

    sqlx::query(
        "UPDATE api_key SET key_hash = $2, last_used_at = NULL WHERE id = $1",
    )
    .bind(id)
    .bind(&key_hash)
    .execute(&state.pool)
    .await?;

    info!(api_key_id = id, "API Key regenerated");

    let permissions: Vec<String> = serde_json::from_value(existing.1).unwrap_or_default();

    let response = CreateApiKeyResponse {
        id,
        name: existing.0,
        api_key,
        permissions,
        rate_limit: existing.2,
        expires_at: existing.3,
        created_at: now,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 启用/禁用 API Key
///
/// PATCH /api/admin/system/api-keys/:id/status
pub async fn toggle_api_key_status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<ToggleStatusRequest>,
) -> Result<Json<ApiResponse<ApiKeyDto>>, AdminError> {
    let result = sqlx::query("UPDATE api_key SET enabled = $2 WHERE id = $1")
        .bind(id)
        .bind(req.enabled)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AdminError::NotFound(format!("API Key {} 不存在", id)));
    }

    let row = sqlx::query_as::<_, ApiKeyRow>(
        r#"
        SELECT id, name, key_prefix, permissions, rate_limit, expires_at,
               enabled, last_used_at, created_at
        FROM api_key
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await?;

    info!(api_key_id = id, enabled = req.enabled, "API Key status updated");
    Ok(Json(ApiResponse::success(row.into())))
}

/// 状态切换请求
#[derive(Debug, serde::Deserialize)]
pub struct ToggleStatusRequest {
    pub enabled: bool,
}
