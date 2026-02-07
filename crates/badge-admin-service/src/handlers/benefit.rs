//! 权益管理 API 处理器
//!
//! 实现权益的 CRUD 操作，包括关联徽章和用户权益查询

use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;
use validator::Validate;

use crate::{
    dto::{ApiResponse, PageResponse, PaginationParams},
    error::AdminError,
    state::AppState,
};
use badge_management::models::{BenefitStatus, BenefitType};

// ==================== DTO 定义 ====================

/// 权益响应 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenefitDto {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub benefit_type: BenefitType,
    pub external_id: Option<String>,
    pub external_system: Option<String>,
    pub total_stock: Option<i64>,
    pub remaining_stock: Option<i64>,
    pub status: BenefitStatus,
    pub config: Option<Value>,
    pub icon_url: Option<String>,
    pub redeemed_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建权益请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateBenefitRequest {
    #[validate(length(min = 1, max = 50, message = "权益编码长度必须在1-50个字符之间"))]
    pub code: String,
    #[validate(length(min = 1, max = 100, message = "权益名称长度必须在1-100个字符之间"))]
    pub name: String,
    pub description: Option<String>,
    pub benefit_type: BenefitType,
    pub external_id: Option<String>,
    pub external_system: Option<String>,
    pub total_stock: Option<i64>,
    pub config: Option<Value>,
    pub icon_url: Option<String>,
}

/// 更新权益请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBenefitRequest {
    #[validate(length(min = 1, max = 100, message = "权益名称长度必须在1-100个字符之间"))]
    pub name: Option<String>,
    pub description: Option<String>,
    pub external_id: Option<String>,
    pub external_system: Option<String>,
    pub total_stock: Option<i64>,
    pub config: Option<Value>,
    pub icon_url: Option<String>,
    pub status: Option<BenefitStatus>,
}

/// 权益查询过滤
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenefitQueryFilter {
    pub benefit_type: Option<BenefitType>,
    pub status: Option<BenefitStatus>,
    pub keyword: Option<String>,
}

/// 关联徽章请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LinkBadgeRequest {
    pub badge_id: i64,
    #[validate(range(min = 1, message = "数量必须大于0"))]
    pub quantity: i32,
}

/// 用户权益响应 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserBenefitDto {
    pub grant_id: i64,
    pub grant_no: String,
    pub benefit_id: i64,
    pub benefit_name: String,
    pub benefit_type: String,
    pub status: String,
    pub granted_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// 权益发放记录响应 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenefitGrantDto {
    pub id: i64,
    pub grant_no: String,
    pub user_id: String,
    pub benefit_id: i64,
    pub benefit_name: String,
    pub benefit_type: String,
    pub source_type: String,
    pub source_id: Option<String>,
    pub quantity: i32,
    pub status: String,
    pub granted_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// 权益发放记录查询过滤
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenefitGrantQueryFilter {
    pub user_id: Option<String>,
    pub benefit_id: Option<i64>,
    pub status: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

/// 权益同步日志 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenefitSyncLogDto {
    pub id: i64,
    pub sync_type: String,
    pub status: String,
    pub total_count: i32,
    pub success_count: i32,
    pub failed_count: i32,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// 触发同步请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerSyncRequest {
    pub sync_type: Option<String>,
    pub benefit_ids: Option<Vec<i64>>,
}

/// 同步结果响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResultDto {
    pub sync_id: i64,
    pub status: String,
    pub message: String,
}

// ==================== 数据库查询结构 ====================

/// 权益完整信息查询结果
#[derive(sqlx::FromRow)]
struct BenefitRow {
    id: i64,
    code: String,
    name: String,
    description: Option<String>,
    benefit_type: BenefitType,
    external_id: Option<String>,
    external_system: Option<String>,
    total_stock: Option<i64>,
    remaining_stock: Option<i64>,
    status: BenefitStatus,
    config: Option<Value>,
    icon_url: Option<String>,
    redeemed_count: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<BenefitRow> for BenefitDto {
    fn from(row: BenefitRow) -> Self {
        Self {
            id: row.id,
            code: row.code,
            name: row.name,
            description: row.description,
            benefit_type: row.benefit_type,
            external_id: row.external_id,
            external_system: row.external_system,
            total_stock: row.total_stock,
            remaining_stock: row.remaining_stock,
            status: row.status,
            config: row.config,
            icon_url: row.icon_url,
            redeemed_count: row.redeemed_count,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// 用户权益查询结果
#[derive(sqlx::FromRow)]
struct UserBenefitRow {
    grant_id: i64,
    grant_no: String,
    benefit_id: i64,
    benefit_name: String,
    benefit_type: String,
    status: String,
    granted_at: Option<DateTime<Utc>>,
    expires_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl From<UserBenefitRow> for UserBenefitDto {
    fn from(row: UserBenefitRow) -> Self {
        Self {
            grant_id: row.grant_id,
            grant_no: row.grant_no,
            benefit_id: row.benefit_id,
            benefit_name: row.benefit_name,
            benefit_type: row.benefit_type,
            status: row.status,
            granted_at: row.granted_at,
            expires_at: row.expires_at,
            created_at: row.created_at,
        }
    }
}

/// 同步日志查询结果，与 benefit_sync_logs 表一一对应
#[derive(sqlx::FromRow)]
struct BenefitSyncLogRow {
    id: i64,
    sync_type: String,
    status: String,
    total_count: i32,
    success_count: i32,
    failed_count: i32,
    error_message: Option<String>,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}

impl From<BenefitSyncLogRow> for BenefitSyncLogDto {
    fn from(row: BenefitSyncLogRow) -> Self {
        Self {
            id: row.id,
            sync_type: row.sync_type,
            status: row.status,
            total_count: row.total_count,
            success_count: row.success_count,
            failed_count: row.failed_count,
            error_message: row.error_message,
            started_at: row.started_at,
            completed_at: row.completed_at,
        }
    }
}

/// 权益发放记录查询结果
#[derive(sqlx::FromRow)]
struct BenefitGrantRow {
    id: i64,
    grant_no: String,
    user_id: String,
    benefit_id: i64,
    benefit_name: String,
    benefit_type: String,
    source_type: String,
    source_id: Option<String>,
    quantity: i32,
    status: String,
    granted_at: Option<DateTime<Utc>>,
    expires_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl From<BenefitGrantRow> for BenefitGrantDto {
    fn from(row: BenefitGrantRow) -> Self {
        Self {
            id: row.id,
            grant_no: row.grant_no,
            user_id: row.user_id,
            benefit_id: row.benefit_id,
            benefit_name: row.benefit_name,
            benefit_type: row.benefit_type,
            source_type: row.source_type,
            source_id: row.source_id,
            quantity: row.quantity,
            status: row.status,
            granted_at: row.granted_at,
            expires_at: row.expires_at,
            created_at: row.created_at,
        }
    }
}

// ==================== 辅助函数 ====================

/// 权益完整信息查询 SQL
const BENEFIT_FULL_SQL: &str = r#"
    SELECT
        id, code, name, description, benefit_type,
        external_id, external_system, total_stock, remaining_stock,
        status, config, icon_url, redeemed_count,
        created_at, updated_at
    FROM benefits
"#;

/// 通过 ID 查询权益完整信息
async fn fetch_benefit_by_id(pool: &sqlx::PgPool, id: i64) -> Result<BenefitDto, AdminError> {
    let sql = format!("{} WHERE id = $1", BENEFIT_FULL_SQL);

    let row = sqlx::query_as::<_, BenefitRow>(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AdminError::BenefitNotFound(id))?;

    Ok(row.into())
}

// ==================== API 处理器 ====================

/// 创建权益
///
/// POST /api/admin/benefits
pub async fn create_benefit(
    State(state): State<AppState>,
    Json(req): Json<CreateBenefitRequest>,
) -> Result<Json<ApiResponse<BenefitDto>>, AdminError> {
    req.validate()?;

    // 检查 code 是否已存在
    let code_exists: (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM benefits WHERE code = $1)")
            .bind(&req.code)
            .fetch_one(&state.pool)
            .await?;

    if code_exists.0 {
        return Err(AdminError::Validation(format!(
            "权益编码 '{}' 已存在",
            req.code
        )));
    }

    // 计算初始剩余库存
    let remaining_stock = req.total_stock;

    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO benefits (code, name, description, benefit_type, external_id, external_system,
                             total_stock, remaining_stock, status, config, icon_url, enabled)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'active', $9, $10, true)
        RETURNING id
        "#,
    )
    .bind(&req.code)
    .bind(&req.name)
    .bind(&req.description)
    .bind(req.benefit_type)
    .bind(&req.external_id)
    .bind(&req.external_system)
    .bind(req.total_stock)
    .bind(remaining_stock)
    .bind(&req.config)
    .bind(&req.icon_url)
    .fetch_one(&state.pool)
    .await?;

    info!(benefit_id = row.0, code = %req.code, "Benefit created");

    let dto = fetch_benefit_by_id(&state.pool, row.0).await?;
    Ok(Json(ApiResponse::success(dto)))
}

/// 获取权益列表（分页）
///
/// GET /api/admin/benefits
pub async fn list_benefits(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
    Query(filter): Query<BenefitQueryFilter>,
) -> Result<Json<ApiResponse<PageResponse<BenefitDto>>>, AdminError> {
    let offset = pagination.offset();
    let limit = pagination.limit();

    // 构建模糊搜索参数
    let keyword_pattern = filter.keyword.as_ref().map(|k| format!("%{}%", k));
    let benefit_type_str = filter.benefit_type.map(|t| {
        serde_json::to_value(t)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_lowercase()))
            .unwrap_or_default()
    });
    let status_str = filter.status.map(|s| {
        serde_json::to_value(s)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_lowercase()))
            .unwrap_or_default()
    });

    // 查询总数
    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM benefits
        WHERE ($1::text IS NULL OR benefit_type::text ILIKE $1)
          AND ($2::text IS NULL OR status::text = $2)
          AND ($3::text IS NULL OR name ILIKE $3 OR code ILIKE $3)
        "#,
    )
    .bind(&benefit_type_str)
    .bind(&status_str)
    .bind(&keyword_pattern)
    .fetch_one(&state.pool)
    .await?;

    // 如果没有数据，直接返回空结果
    if total.0 == 0 {
        return Ok(Json(ApiResponse::success(PageResponse::empty(
            pagination.page,
            pagination.page_size,
        ))));
    }

    // 查询列表
    let sql = format!(
        r#"{}
        WHERE ($1::text IS NULL OR benefit_type::text ILIKE $1)
          AND ($2::text IS NULL OR status::text = $2)
          AND ($3::text IS NULL OR name ILIKE $3 OR code ILIKE $3)
        ORDER BY created_at DESC
        LIMIT $4 OFFSET $5
        "#,
        BENEFIT_FULL_SQL
    );

    let rows = sqlx::query_as::<_, BenefitRow>(&sql)
        .bind(&benefit_type_str)
        .bind(&status_str)
        .bind(&keyword_pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await?;

    let items: Vec<BenefitDto> = rows.into_iter().map(Into::into).collect();

    let response = PageResponse::new(items, total.0, pagination.page, pagination.page_size);
    Ok(Json(ApiResponse::success(response)))
}

/// 获取权益详情
///
/// GET /api/admin/benefits/:id
pub async fn get_benefit(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<BenefitDto>>, AdminError> {
    let dto = fetch_benefit_by_id(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(dto)))
}

/// 更新权益
///
/// PUT /api/admin/benefits/:id
pub async fn update_benefit(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateBenefitRequest>,
) -> Result<Json<ApiResponse<BenefitDto>>, AdminError> {
    req.validate()?;

    // 检查权益是否存在
    let exists: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM benefits WHERE id = $1)")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;

    if !exists.0 {
        return Err(AdminError::BenefitNotFound(id));
    }

    let status_str = req.status.map(|s| {
        serde_json::to_value(s)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_lowercase()))
            .unwrap_or_default()
    });

    // 如果更新了 total_stock，需要同步更新 remaining_stock
    // remaining_stock = new_total_stock - (old_total_stock - old_remaining_stock)
    sqlx::query(
        r#"
        UPDATE benefits
        SET
            name = COALESCE($2, name),
            description = COALESCE($3, description),
            external_id = COALESCE($4, external_id),
            external_system = COALESCE($5, external_system),
            total_stock = COALESCE($6, total_stock),
            remaining_stock = CASE
                WHEN $6 IS NOT NULL THEN $6 - COALESCE(redeemed_count, 0)
                ELSE remaining_stock
            END,
            config = COALESCE($7, config),
            icon_url = COALESCE($8, icon_url),
            status = COALESCE($9, status),
            enabled = CASE
                WHEN $9 IS NOT NULL THEN ($9 = 'active')
                ELSE enabled
            END,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.external_id)
    .bind(&req.external_system)
    .bind(req.total_stock)
    .bind(&req.config)
    .bind(&req.icon_url)
    .bind(&status_str)
    .execute(&state.pool)
    .await?;

    info!(benefit_id = id, "Benefit updated");

    let dto = fetch_benefit_by_id(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(dto)))
}

/// 删除权益
///
/// DELETE /api/admin/benefits/:id
///
/// 仅允许删除未被使用的权益
pub async fn delete_benefit(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, AdminError> {
    // 检查权益是否存在
    let exists: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM benefits WHERE id = $1)")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;

    if !exists.0 {
        return Err(AdminError::BenefitNotFound(id));
    }

    // 检查是否有关联的兑换规则
    let has_rules: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM badge_redemption_rules WHERE benefit_id = $1)",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await?;

    if has_rules.0 {
        return Err(AdminError::Validation(
            "该权益已被兑换规则使用，无法删除".to_string(),
        ));
    }

    // 检查是否有发放记录
    let has_grants: (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM benefit_grants WHERE benefit_id = $1)")
            .bind(id)
            .fetch_one(&state.pool)
            .await?;

    if has_grants.0 {
        return Err(AdminError::Validation(
            "该权益已有发放记录，无法删除".to_string(),
        ));
    }

    sqlx::query("DELETE FROM benefits WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    info!(benefit_id = id, "Benefit deleted");

    Ok(Json(ApiResponse::<()>::success_empty()))
}

/// 查询用户权益
///
/// GET /api/admin/users/:user_id/benefits
pub async fn get_user_benefits(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<ApiResponse<PageResponse<UserBenefitDto>>>, AdminError> {
    let offset = pagination.offset();
    let limit = pagination.limit();

    // 查询总数
    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM benefit_grants
        WHERE user_id = $1
        "#,
    )
    .bind(&user_id)
    .fetch_one(&state.pool)
    .await?;

    // 如果没有数据，直接返回空结果
    if total.0 == 0 {
        return Ok(Json(ApiResponse::success(PageResponse::empty(
            pagination.page,
            pagination.page_size,
        ))));
    }

    // 查询用户权益列表
    let rows = sqlx::query_as::<_, UserBenefitRow>(
        r#"
        SELECT
            bg.id as grant_id,
            bg.grant_no,
            bg.benefit_id,
            b.name as benefit_name,
            b.benefit_type::text as benefit_type,
            bg.status,
            bg.granted_at,
            bg.expires_at,
            bg.created_at
        FROM benefit_grants bg
        JOIN benefits b ON b.id = bg.benefit_id
        WHERE bg.user_id = $1
        ORDER BY bg.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(&user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let items: Vec<UserBenefitDto> = rows.into_iter().map(Into::into).collect();

    let response = PageResponse::new(items, total.0, pagination.page, pagination.page_size);
    Ok(Json(ApiResponse::success(response)))
}

/// 查询权益发放记录
///
/// GET /api/admin/benefit-grants
pub async fn list_benefit_grants(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
    Query(filter): Query<BenefitGrantQueryFilter>,
) -> Result<Json<ApiResponse<PageResponse<BenefitGrantDto>>>, AdminError> {
    let offset = pagination.offset();
    let limit = pagination.limit();

    // 解析日期过滤
    let start_date = filter
        .start_date
        .as_ref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc());
    let end_date = filter
        .end_date
        .as_ref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .map(|d| d.and_hms_opt(23, 59, 59).unwrap().and_utc());

    // 查询总数
    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM benefit_grants bg
        WHERE ($1::text IS NULL OR bg.user_id = $1)
          AND ($2::bigint IS NULL OR bg.benefit_id = $2)
          AND ($3::text IS NULL OR bg.status = $3)
          AND ($4::timestamptz IS NULL OR bg.created_at >= $4)
          AND ($5::timestamptz IS NULL OR bg.created_at <= $5)
        "#,
    )
    .bind(&filter.user_id)
    .bind(filter.benefit_id)
    .bind(&filter.status)
    .bind(start_date)
    .bind(end_date)
    .fetch_one(&state.pool)
    .await?;

    if total.0 == 0 {
        return Ok(Json(ApiResponse::success(PageResponse::empty(
            pagination.page,
            pagination.page_size,
        ))));
    }

    // 查询列表
    let rows = sqlx::query_as::<_, BenefitGrantRow>(
        r#"
        SELECT
            bg.id,
            bg.grant_no,
            bg.user_id,
            bg.benefit_id,
            b.name as benefit_name,
            b.benefit_type::text as benefit_type,
            COALESCE(bg.source_type, 'UNKNOWN') as source_type,
            bg.source_id,
            COALESCE(bg.quantity, 1) as quantity,
            bg.status,
            bg.granted_at,
            bg.expires_at,
            bg.created_at
        FROM benefit_grants bg
        JOIN benefits b ON b.id = bg.benefit_id
        WHERE ($1::text IS NULL OR bg.user_id = $1)
          AND ($2::bigint IS NULL OR bg.benefit_id = $2)
          AND ($3::text IS NULL OR bg.status = $3)
          AND ($4::timestamptz IS NULL OR bg.created_at >= $4)
          AND ($5::timestamptz IS NULL OR bg.created_at <= $5)
        ORDER BY bg.created_at DESC
        LIMIT $6 OFFSET $7
        "#,
    )
    .bind(&filter.user_id)
    .bind(filter.benefit_id)
    .bind(&filter.status)
    .bind(start_date)
    .bind(end_date)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let items: Vec<BenefitGrantDto> = rows.into_iter().map(Into::into).collect();

    let response = PageResponse::new(items, total.0, pagination.page, pagination.page_size);
    Ok(Json(ApiResponse::success(response)))
}

/// 查询权益同步日志
///
/// GET /api/admin/benefits/sync-logs
pub async fn list_sync_logs(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<ApiResponse<PageResponse<BenefitSyncLogDto>>>, AdminError> {
    let offset = pagination.offset();
    let limit = pagination.limit();

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM benefit_sync_logs")
        .fetch_one(&state.pool)
        .await?;

    if total.0 == 0 {
        return Ok(Json(ApiResponse::success(PageResponse::empty(
            pagination.page,
            pagination.page_size,
        ))));
    }

    let rows = sqlx::query_as::<_, BenefitSyncLogRow>(
        "SELECT id, sync_type, status, total_count, success_count, failed_count, error_message, started_at, completed_at FROM benefit_sync_logs ORDER BY created_at DESC LIMIT $1 OFFSET $2"
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let items: Vec<BenefitSyncLogDto> = rows.into_iter().map(Into::into).collect();
    let response = PageResponse::new(items, total.0, pagination.page, pagination.page_size);
    Ok(Json(ApiResponse::success(response)))
}

/// 触发权益同步
///
/// POST /api/admin/benefits/sync
pub async fn trigger_sync(
    State(state): State<AppState>,
    Json(req): Json<TriggerSyncRequest>,
) -> Result<Json<ApiResponse<SyncResultDto>>, AdminError> {
    info!(
        sync_type = ?req.sync_type,
        benefit_ids = ?req.benefit_ids,
        "Benefit sync triggered"
    );

    // 持久化同步任务记录，后续异步 worker 可根据此记录执行实际同步
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO benefit_sync_logs (sync_type, status, started_at, created_at) VALUES ($1, 'PENDING', NOW(), NOW()) RETURNING id"
    )
    .bind(req.sync_type.as_deref().unwrap_or("full"))
    .fetch_one(&state.pool)
    .await?;

    let result = SyncResultDto {
        sync_id: row.0,
        status: "PENDING".to_string(),
        message: "同步任务已提交".to_string(),
    };

    Ok(Json(ApiResponse::success(result)))
}

/// 关联徽章到权益（用于自动发放配置）
///
/// POST /api/admin/benefits/:id/link-badge
pub async fn link_badge_to_benefit(
    State(state): State<AppState>,
    Path(benefit_id): Path<i64>,
    Json(req): Json<LinkBadgeRequest>,
) -> Result<Json<ApiResponse<()>>, AdminError> {
    req.validate()?;

    // 验证权益存在
    let benefit_exists: (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM benefits WHERE id = $1)")
            .bind(benefit_id)
            .fetch_one(&state.pool)
            .await?;

    if !benefit_exists.0 {
        return Err(AdminError::BenefitNotFound(benefit_id));
    }

    // 验证徽章存在
    let badge_exists: (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM badges WHERE id = $1)")
            .bind(req.badge_id)
            .fetch_one(&state.pool)
            .await?;

    if !badge_exists.0 {
        return Err(AdminError::BadgeNotFound(req.badge_id));
    }

    // 使用 upsert 避免重复关联时报错，同时支持更新数量
    sqlx::query(
        "INSERT INTO badge_benefit_links (badge_id, benefit_id, quantity, created_at) VALUES ($1, $2, $3, NOW()) ON CONFLICT (badge_id, benefit_id) DO UPDATE SET quantity = $3"
    )
    .bind(req.badge_id)
    .bind(benefit_id)
    .bind(req.quantity)
    .execute(&state.pool)
    .await?;

    info!(
        benefit_id = benefit_id,
        badge_id = req.badge_id,
        quantity = req.quantity,
        "Badge linked to benefit"
    );

    Ok(Json(ApiResponse::<()>::success_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_benefit_request_validation() {
        let valid = CreateBenefitRequest {
            code: "TEST_001".to_string(),
            name: "测试权益".to_string(),
            description: Some("描述".to_string()),
            benefit_type: BenefitType::Coupon,
            external_id: None,
            external_system: None,
            total_stock: Some(100),
            config: None,
            icon_url: None,
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateBenefitRequest {
            code: "".to_string(), // 空编码
            name: "测试权益".to_string(),
            description: None,
            benefit_type: BenefitType::Coupon,
            external_id: None,
            external_system: None,
            total_stock: None,
            config: None,
            icon_url: None,
        };
        assert!(invalid.validate().is_err());
    }
}
