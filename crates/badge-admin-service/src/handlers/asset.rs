//! 素材库管理 API
//!
//! 提供素材的 CRUD 操作，支持图片、动画、视频和 3D 模型

use axum::{
    Extension,
    extract::{Path, Query, State},
    Json,
};
use crate::middleware::AuditContext;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::error;
use validator::Validate;

use crate::{
    dto::response::{ApiResponse, PageResponse},
    error::AdminError,
    state::AppState,
};

// ============ DTO 定义 ============

/// 素材类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AssetType {
    /// 图片
    Image,
    /// 动画（GIF、Lottie 等）
    Animation,
    /// 视频
    Video,
    /// 3D 模型
    Model3d,
}

impl AssetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetType::Image => "IMAGE",
            AssetType::Animation => "ANIMATION",
            AssetType::Video => "VIDEO",
            AssetType::Model3d => "MODEL_3D",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "IMAGE" => Some(AssetType::Image),
            "ANIMATION" => Some(AssetType::Animation),
            "VIDEO" => Some(AssetType::Video),
            "MODEL_3D" => Some(AssetType::Model3d),
            _ => None,
        }
    }
}

/// 素材 DTO
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDto {
    pub id: i64,
    pub name: String,
    pub asset_type: String,
    pub file_url: String,
    pub thumbnail_url: Option<String>,
    pub file_size: i64,
    pub file_format: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub metadata: Option<serde_json::Value>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub status: String,
    pub usage_count: i32,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建素材请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateAssetRequest {
    #[validate(length(min = 1, max = 100, message = "素材名称长度必须在 1-100 个字符之间"))]
    pub name: String,
    pub asset_type: String,
    #[validate(url(message = "文件地址必须是有效的 URL"))]
    pub file_url: String,
    pub thumbnail_url: Option<String>,
    pub file_size: Option<i64>,
    pub file_format: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub metadata: Option<serde_json::Value>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// 更新素材请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAssetRequest {
    #[validate(length(min = 1, max = 100, message = "素材名称长度必须在 1-100 个字符之间"))]
    pub name: Option<String>,
    pub thumbnail_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub status: Option<String>,
}

/// 素材查询参数
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetQueryParams {
    pub asset_type: Option<String>,
    pub category: Option<String>,
    pub tag: Option<String>,
    pub status: Option<String>,
    pub keyword: Option<String>,
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    20
}

// ============ 数据库行结构 ============

#[derive(sqlx::FromRow)]
struct AssetRow {
    id: i64,
    name: String,
    asset_type: String,
    file_url: String,
    thumbnail_url: Option<String>,
    file_size: Option<i64>,
    file_format: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
    metadata: Option<serde_json::Value>,
    category: Option<String>,
    tags: Option<Vec<String>>,
    status: Option<String>,
    usage_count: Option<i32>,
    created_by: Option<String>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
}

impl From<AssetRow> for AssetDto {
    fn from(row: AssetRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            asset_type: row.asset_type,
            file_url: row.file_url,
            thumbnail_url: row.thumbnail_url,
            file_size: row.file_size.unwrap_or(0),
            file_format: row.file_format,
            width: row.width,
            height: row.height,
            metadata: row.metadata,
            category: row.category,
            tags: row.tags,
            status: row.status.unwrap_or_else(|| "active".to_string()),
            usage_count: row.usage_count.unwrap_or(0),
            created_by: row.created_by,
            created_at: row.created_at.unwrap_or_else(Utc::now),
            updated_at: row.updated_at.unwrap_or_else(Utc::now),
        }
    }
}

// ============ API Handler ============

/// 获取素材列表
///
/// GET /api/admin/assets
pub async fn list_assets(
    State(state): State<AppState>,
    Query(params): Query<AssetQueryParams>,
) -> Result<Json<ApiResponse<PageResponse<AssetDto>>>, AdminError> {
    let pool = &state.pool;
    let offset = (params.page - 1).max(0) * params.page_size;
    let limit = params.page_size.clamp(1, 100);

    let total = count_assets(pool, &params).await?;
    let items = fetch_assets(pool, &params, limit, offset).await?;

    Ok(Json(ApiResponse::success(PageResponse::new(
        items,
        total,
        params.page,
        params.page_size,
    ))))
}

/// 构建素材查询的动态 WHERE 子句和绑定参数
///
/// 使用参数化占位符（$N）防止 SQL 注入，所有用户输入均通过 bind 传入。
/// 返回 (where_clause, bind_values)，调用方拼接 SELECT 前缀后统一绑定。
fn build_asset_where(params: &AssetQueryParams) -> (String, Vec<String>) {
    let mut conditions = Vec::new();
    let mut binds: Vec<String> = Vec::new();

    if let Some(ref asset_type) = params.asset_type {
        binds.push(asset_type.clone());
        conditions.push(format!("asset_type = ${}", binds.len()));
    }
    if let Some(ref category) = params.category {
        binds.push(category.clone());
        conditions.push(format!("category = ${}", binds.len()));
    }
    if let Some(ref tag) = params.tag {
        binds.push(tag.clone());
        conditions.push(format!("${} = ANY(tags)", binds.len()));
    }
    if let Some(ref status) = params.status {
        binds.push(status.clone());
        conditions.push(format!("status = ${}", binds.len()));
    }
    if let Some(ref keyword) = params.keyword {
        binds.push(format!("%{}%", keyword));
        conditions.push(format!("name ILIKE ${}", binds.len()));
    }

    let where_clause = if conditions.is_empty() {
        " WHERE 1=1".to_string()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    (where_clause, binds)
}

/// 辅助函数：统计素材数量
async fn count_assets(pool: &PgPool, params: &AssetQueryParams) -> Result<i64, AdminError> {
    let (where_clause, binds) = build_asset_where(params);
    let sql = format!("SELECT COUNT(*) FROM assets{}", where_clause);

    let mut query = sqlx::query_as::<_, (i64,)>(&sql);
    for val in &binds {
        query = query.bind(val);
    }

    let row = query.fetch_one(pool).await.map_err(|e| {
        error!(error = %e, "查询素材总数失败");
        AdminError::Database(e)
    })?;

    Ok(row.0)
}

/// 辅助函数：获取素材列表
async fn fetch_assets(
    pool: &PgPool,
    params: &AssetQueryParams,
    limit: i64,
    offset: i64,
) -> Result<Vec<AssetDto>, AdminError> {
    let (where_clause, binds) = build_asset_where(params);
    let next_idx = binds.len() + 1;
    let sql = format!(
        "SELECT * FROM assets{} ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
        where_clause, next_idx, next_idx + 1
    );

    let mut query = sqlx::query_as::<_, AssetRow>(&sql);
    for val in &binds {
        query = query.bind(val);
    }
    query = query.bind(limit).bind(offset);

    let rows: Vec<AssetRow> = query.fetch_all(pool).await.map_err(|e| {
        error!(error = %e, "查询素材列表失败");
        AdminError::Database(e)
    })?;

    Ok(rows.into_iter().map(AssetDto::from).collect())
}

/// 获取单个素材详情
///
/// GET /api/admin/assets/:id
pub async fn get_asset(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<AssetDto>>, AdminError> {
    let pool = &state.pool;

    let row: AssetRow = sqlx::query_as(
        "SELECT * FROM assets WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        error!(error = %e, asset_id = id, "查询素材详情失败");
        AdminError::Database(e)
    })?
    .ok_or_else(|| AdminError::NotFound(format!("素材 {} 不存在", id)))?;

    Ok(Json(ApiResponse::success(AssetDto::from(row))))
}

/// 创建素材
///
/// POST /api/admin/assets
pub async fn create_asset(
    State(state): State<AppState>,
    Json(req): Json<CreateAssetRequest>,
) -> Result<Json<ApiResponse<AssetDto>>, AdminError> {
    req.validate().map_err(|e| AdminError::Validation(e.to_string()))?;

    // 验证素材类型
    if AssetType::from_str(&req.asset_type).is_none() {
        return Err(AdminError::Validation(format!(
            "无效的素材类型: {}，支持 IMAGE, ANIMATION, VIDEO, MODEL_3D",
            req.asset_type
        )));
    }

    let pool = &state.pool;

    let row: AssetRow = sqlx::query_as(
        r#"
        INSERT INTO assets (
            name, asset_type, file_url, thumbnail_url, file_size, file_format,
            width, height, metadata, category, tags, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW(), NOW())
        RETURNING *
        "#,
    )
    .bind(&req.name)
    .bind(&req.asset_type.to_uppercase())
    .bind(&req.file_url)
    .bind(&req.thumbnail_url)
    .bind(req.file_size.unwrap_or(0))
    .bind(&req.file_format)
    .bind(req.width)
    .bind(req.height)
    .bind(&req.metadata)
    .bind(&req.category)
    .bind(&req.tags)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        error!(error = %e, name = req.name, "创建素材失败");
        AdminError::Database(e)
    })?;

    Ok(Json(ApiResponse::success(AssetDto::from(row))))
}

/// 更新素材
///
/// PUT /api/admin/assets/:id
pub async fn update_asset(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(audit_ctx): Extension<AuditContext>,
    Json(req): Json<UpdateAssetRequest>,
) -> Result<Json<ApiResponse<AssetDto>>, AdminError> {
    req.validate().map_err(|e| AdminError::Validation(e.to_string()))?;

    let pool = &state.pool;

    // 检查素材是否存在
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM assets WHERE id = $1)")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            error!(error = %e, asset_id = id, "检查素材存在性失败");
            AdminError::Database(e)
        })?;

    if !exists {
        return Err(AdminError::NotFound(format!("素材 {} 不存在", id)));
    }

    // 审计快照：记录变更前状态
    audit_ctx.snapshot(&state.pool, "asset_library", id).await;

    let row: AssetRow = sqlx::query_as(
        r#"
        UPDATE assets SET
            name = COALESCE($2, name),
            thumbnail_url = COALESCE($3, thumbnail_url),
            metadata = COALESCE($4, metadata),
            category = COALESCE($5, category),
            tags = COALESCE($6, tags),
            status = COALESCE($7, status),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.thumbnail_url)
    .bind(&req.metadata)
    .bind(&req.category)
    .bind(&req.tags)
    .bind(&req.status)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        error!(error = %e, asset_id = id, "更新素材失败");
        AdminError::Database(e)
    })?;

    Ok(Json(ApiResponse::success(AssetDto::from(row))))
}

/// 删除素材
///
/// DELETE /api/admin/assets/:id
pub async fn delete_asset(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(audit_ctx): Extension<AuditContext>,
) -> Result<Json<ApiResponse<()>>, AdminError> {
    let pool = &state.pool;

    // 审计快照：记录变更前状态
    audit_ctx.snapshot(&state.pool, "asset_library", id).await;

    let result = sqlx::query("DELETE FROM assets WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| {
            error!(error = %e, asset_id = id, "删除素材失败");
            AdminError::Database(e)
        })?;

    if result.rows_affected() == 0 {
        return Err(AdminError::NotFound(format!("素材 {} 不存在", id)));
    }

    Ok(Json(ApiResponse::success(())))
}

/// 增加素材使用次数
///
/// POST /api/admin/assets/:id/use
pub async fn increment_usage(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, AdminError> {
    let pool = &state.pool;

    let result = sqlx::query(
        "UPDATE assets SET usage_count = usage_count + 1, updated_at = NOW() WHERE id = $1",
    )
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| {
        error!(error = %e, asset_id = id, "增加素材使用次数失败");
        AdminError::Database(e)
    })?;

    if result.rows_affected() == 0 {
        return Err(AdminError::NotFound(format!("素材 {} 不存在", id)));
    }

    Ok(Json(ApiResponse::success(())))
}

/// 获取素材分类列表
///
/// GET /api/admin/assets/categories
pub async fn list_categories(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<String>>>, AdminError> {
    let pool = &state.pool;

    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT category FROM assets WHERE category IS NOT NULL ORDER BY category",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        error!(error = %e, "查询素材分类失败");
        AdminError::Database(e)
    })?;

    let categories: Vec<String> = rows.into_iter().map(|(c,)| c).collect();

    Ok(Json(ApiResponse::success(categories)))
}
