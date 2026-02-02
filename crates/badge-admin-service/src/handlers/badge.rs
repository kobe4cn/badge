//! 徽章管理 API 处理器
//!
//! 实现徽章的 CRUD 操作及状态管理（发布/下线）

use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Utc};
use serde_json::Value;
use tracing::info;
use validator::Validate;

use crate::{
    BadgeAssets, BadgeStatus, BadgeType,
    dto::{
        ApiResponse, BadgeAdminDto, BadgeQueryFilter, CreateBadgeRequest, PageResponse,
        PaginationParams, UpdateBadgeRequest,
    },
    error::AdminError,
    state::AppState,
};

/// 徽章数据库查询结果（带关联分类/系列信息）
#[derive(sqlx::FromRow)]
struct BadgeFullRow {
    id: i64,
    series_id: i64,
    series_name: String,
    category_id: i64,
    category_name: String,
    badge_type: BadgeType,
    name: String,
    description: Option<String>,
    obtain_description: Option<String>,
    assets: Value,
    validity_config: Value,
    max_supply: Option<i64>,
    issued_count: i64,
    status: BadgeStatus,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<BadgeFullRow> for BadgeAdminDto {
    fn from(row: BadgeFullRow) -> Self {
        Self {
            id: row.id,
            series_id: row.series_id,
            series_name: row.series_name,
            category_id: row.category_id,
            category_name: row.category_name,
            badge_type: row.badge_type,
            name: row.name,
            description: row.description,
            obtain_description: row.obtain_description,
            assets: serde_json::from_value(row.assets).unwrap_or(BadgeAssets {
                icon_url: String::new(),
                image_url: None,
                animation_url: None,
                disabled_icon_url: None,
            }),
            validity_config: serde_json::from_value(row.validity_config).unwrap_or_default(),
            max_supply: row.max_supply.map(|v| v as i32),
            issued_count: row.issued_count as i32,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// 徽章完整信息的查询 SQL（复用于详情/更新后回查）
const BADGE_FULL_SQL: &str = r#"
    SELECT
        b.id,
        b.series_id,
        s.name as series_name,
        s.category_id,
        c.name as category_name,
        b.badge_type,
        b.name,
        b.description,
        b.obtain_description,
        b.assets,
        b.validity_config,
        b.max_supply,
        b.issued_count,
        b.status,
        b.created_at,
        b.updated_at
    FROM badges b
    JOIN badge_series s ON s.id = b.series_id
    JOIN badge_categories c ON c.id = s.category_id
"#;

/// 通过 ID 查询徽章完整信息
async fn fetch_badge_by_id(pool: &sqlx::PgPool, id: i64) -> Result<BadgeAdminDto, AdminError> {
    let sql = format!("{} WHERE b.id = $1", BADGE_FULL_SQL);

    let row = sqlx::query_as::<_, BadgeFullRow>(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AdminError::BadgeNotFound(id))?;

    Ok(row.into())
}

/// 创建徽章
///
/// POST /api/admin/badges
pub async fn create_badge(
    State(state): State<AppState>,
    Json(req): Json<CreateBadgeRequest>,
) -> Result<Json<ApiResponse<BadgeAdminDto>>, AdminError> {
    req.validate()?;

    // 验证系列存在
    let series_exists: (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM badge_series WHERE id = $1)")
            .bind(req.series_id)
            .fetch_one(&state.pool)
            .await?;

    if !series_exists.0 {
        return Err(AdminError::SeriesNotFound(req.series_id));
    }

    let assets_json = serde_json::to_value(&req.assets)
        .map_err(|e| AdminError::Internal(format!("Failed to serialize assets: {}", e)))?;
    let validity_json = serde_json::to_value(&req.validity_config)
        .map_err(|e| AdminError::Internal(format!("Failed to serialize validity_config: {}", e)))?;

    // 新建徽章默认草稿状态
    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO badges (series_id, badge_type, name, description, obtain_description, assets, validity_config, max_supply, status)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'draft')
        RETURNING id
        "#,
    )
    .bind(req.series_id)
    .bind(req.badge_type)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.obtain_description)
    .bind(&assets_json)
    .bind(&validity_json)
    .bind(req.max_supply.map(|v| v as i64))
    .fetch_one(&state.pool)
    .await?;

    info!(badge_id = row.0, name = %req.name, "Badge created");

    let dto = fetch_badge_by_id(&state.pool, row.0).await?;
    Ok(Json(ApiResponse::success(dto)))
}

/// 获取徽章列表（分页）
///
/// GET /api/admin/badges
pub async fn list_badges(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
    Query(filter): Query<BadgeQueryFilter>,
) -> Result<Json<ApiResponse<PageResponse<BadgeAdminDto>>>, AdminError> {
    let offset = pagination.offset();
    let limit = pagination.limit();

    // 构建模糊搜索参数
    let keyword_pattern = filter.keyword.as_ref().map(|k| format!("%{}%", k));
    // 枚举序列化为数据库存储的小写字符串
    let badge_type_str = filter.badge_type.map(|t| {
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
        FROM badges b
        JOIN badge_series s ON s.id = b.series_id
        WHERE ($1::bigint IS NULL OR s.category_id = $1)
          AND ($2::bigint IS NULL OR b.series_id = $2)
          AND ($3::text IS NULL OR b.badge_type::text = $3)
          AND ($4::text IS NULL OR b.status::text = $4)
          AND ($5::text IS NULL OR b.name ILIKE $5)
        "#,
    )
    .bind(filter.category_id)
    .bind(filter.series_id)
    .bind(&badge_type_str)
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
        WHERE ($1::bigint IS NULL OR s.category_id = $1)
          AND ($2::bigint IS NULL OR b.series_id = $2)
          AND ($3::text IS NULL OR b.badge_type::text = $3)
          AND ($4::text IS NULL OR b.status::text = $4)
          AND ($5::text IS NULL OR b.name ILIKE $5)
        ORDER BY b.created_at DESC
        LIMIT $6 OFFSET $7
        "#,
        BADGE_FULL_SQL
    );

    let rows = sqlx::query_as::<_, BadgeFullRow>(&sql)
        .bind(filter.category_id)
        .bind(filter.series_id)
        .bind(&badge_type_str)
        .bind(&status_str)
        .bind(&keyword_pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await?;

    let items: Vec<BadgeAdminDto> = rows.into_iter().map(Into::into).collect();

    let response = PageResponse::new(items, total.0, pagination.page, pagination.page_size);
    Ok(Json(ApiResponse::success(response)))
}

/// 获取徽章详情
///
/// GET /api/admin/badges/:id
pub async fn get_badge(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<BadgeAdminDto>>, AdminError> {
    let dto = fetch_badge_by_id(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(dto)))
}

/// 更新徽章
///
/// PUT /api/admin/badges/:id
pub async fn update_badge(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateBadgeRequest>,
) -> Result<Json<ApiResponse<BadgeAdminDto>>, AdminError> {
    req.validate()?;

    // 检查徽章是否存在
    let exists: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM badges WHERE id = $1)")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;

    if !exists.0 {
        return Err(AdminError::BadgeNotFound(id));
    }

    // 序列化可选字段
    let assets_json = req
        .assets
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| AdminError::Internal(format!("Failed to serialize assets: {}", e)))?;

    let validity_json = req
        .validity_config
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| AdminError::Internal(format!("Failed to serialize validity_config: {}", e)))?;

    let status_str = req.status.map(|s| {
        serde_json::to_value(s)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_lowercase()))
            .unwrap_or_default()
    });

    // 使用 COALESCE 实现部分更新，NULL 参数表示不更新该字段
    sqlx::query(
        r#"
        UPDATE badges
        SET
            name = COALESCE($2, name),
            description = COALESCE($3, description),
            obtain_description = COALESCE($4, obtain_description),
            assets = COALESCE($5, assets),
            validity_config = COALESCE($6, validity_config),
            max_supply = COALESCE($7, max_supply),
            status = COALESCE($8, status),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.obtain_description)
    .bind(&assets_json)
    .bind(&validity_json)
    .bind(req.max_supply.map(|v| v as i64))
    .bind(&status_str)
    .execute(&state.pool)
    .await?;

    info!(badge_id = id, "Badge updated");

    let dto = fetch_badge_by_id(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(dto)))
}

/// 删除徽章
///
/// DELETE /api/admin/badges/:id
///
/// 仅允许删除草稿状态的徽章，已发布的徽章应使用下线操作
pub async fn delete_badge(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, AdminError> {
    // 检查徽章状态
    let badge: Option<(BadgeStatus,)> = sqlx::query_as("SELECT status FROM badges WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?;

    let badge = badge.ok_or(AdminError::BadgeNotFound(id))?;

    // 只有草稿状态的徽章才能删除
    if badge.0 != BadgeStatus::Draft {
        return Err(AdminError::BadgeAlreadyPublished);
    }

    sqlx::query("DELETE FROM badges WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    info!(badge_id = id, "Badge deleted");

    Ok(Json(ApiResponse::<()>::success_empty()))
}

/// 发布徽章
///
/// POST /api/admin/badges/:id/publish
///
/// 将草稿状态的徽章发布为上线状态，发布后对 C 端用户可见
pub async fn publish_badge(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<BadgeAdminDto>>, AdminError> {
    // 只有草稿状态才能发布
    let badge: Option<(BadgeStatus,)> = sqlx::query_as("SELECT status FROM badges WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?;

    let badge = badge.ok_or(AdminError::BadgeNotFound(id))?;

    if badge.0 != BadgeStatus::Draft {
        return Err(AdminError::Validation(format!(
            "只有草稿状态的徽章才能发布，当前状态: {:?}",
            badge.0
        )));
    }

    sqlx::query("UPDATE badges SET status = 'active', updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    info!(badge_id = id, "Badge published");

    let dto = fetch_badge_by_id(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(dto)))
}

/// 下线徽章
///
/// POST /api/admin/badges/:id/offline
///
/// 将上线状态的徽章下线（停止发放，已获取的仍可展示）
pub async fn offline_badge(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<BadgeAdminDto>>, AdminError> {
    // 只有上线状态才能下线
    let badge: Option<(BadgeStatus,)> = sqlx::query_as("SELECT status FROM badges WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?;

    let badge = badge.ok_or(AdminError::BadgeNotFound(id))?;

    if badge.0 != BadgeStatus::Active {
        return Err(AdminError::Validation(format!(
            "只有上线状态的徽章才能下线，当前状态: {:?}",
            badge.0
        )));
    }

    sqlx::query("UPDATE badges SET status = 'inactive', updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    info!(badge_id = id, "Badge offline");

    let dto = fetch_badge_by_id(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(dto)))
}

/// 归档徽章
///
/// POST /api/admin/badges/:id/archive
///
/// 将已下线的徽章归档，归档后不再展示
pub async fn archive_badge(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<BadgeAdminDto>>, AdminError> {
    // 只有下线状态才能归档
    let badge: Option<(BadgeStatus,)> = sqlx::query_as("SELECT status FROM badges WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?;

    let badge = badge.ok_or(AdminError::BadgeNotFound(id))?;

    if badge.0 != BadgeStatus::Inactive {
        return Err(AdminError::Validation(format!(
            "只有已下线的徽章才能归档，当前状态: {:?}",
            badge.0
        )));
    }

    sqlx::query("UPDATE badges SET status = 'archived', updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    info!(badge_id = id, "Badge archived");

    let dto = fetch_badge_by_id(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(dto)))
}

/// 更新徽章排序
///
/// PATCH /api/admin/badges/:id/sort
///
/// 更新徽章的排序值
pub async fn update_badge_sort(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateSortRequest>,
) -> Result<Json<ApiResponse<()>>, AdminError> {
    let result = sqlx::query(
        "UPDATE badges SET sort_order = $2, updated_at = NOW() WHERE id = $1",
    )
    .bind(id)
    .bind(req.sort_order)
    .execute(&state.pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AdminError::BadgeNotFound(id));
    }

    info!(badge_id = id, sort_order = req.sort_order, "Badge sort order updated");
    Ok(Json(ApiResponse::<()>::success_empty()))
}

/// 排序更新请求
#[derive(Debug, serde::Deserialize)]
pub struct UpdateSortRequest {
    pub sort_order: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ValidityConfig;

    #[test]
    fn test_create_badge_request_validation() {
        let valid = CreateBadgeRequest {
            series_id: 1,
            badge_type: BadgeType::Normal,
            name: "测试徽章".to_string(),
            description: Some("描述".to_string()),
            obtain_description: None,
            assets: BadgeAssets {
                icon_url: "https://example.com/icon.png".to_string(),
                image_url: None,
                animation_url: None,
                disabled_icon_url: None,
            },
            validity_config: ValidityConfig::default(),
            max_supply: Some(100),
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateBadgeRequest {
            series_id: 1,
            badge_type: BadgeType::Normal,
            name: "".to_string(), // 空名称
            description: None,
            obtain_description: None,
            assets: BadgeAssets {
                icon_url: "https://example.com/icon.png".to_string(),
                image_url: None,
                animation_url: None,
                disabled_icon_url: None,
            },
            validity_config: ValidityConfig::default(),
            max_supply: None,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_badge_full_row_conversion() {
        let row = BadgeFullRow {
            id: 1,
            series_id: 1,
            series_name: "系列1".to_string(),
            category_id: 1,
            category_name: "分类1".to_string(),
            badge_type: BadgeType::Normal,
            name: "测试徽章".to_string(),
            description: None,
            obtain_description: None,
            assets: serde_json::json!({"iconUrl": "https://example.com/icon.png"}),
            validity_config: serde_json::json!({"validityType": "PERMANENT"}),
            max_supply: Some(100),
            issued_count: 10,
            status: BadgeStatus::Draft,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let dto: BadgeAdminDto = row.into();
        assert_eq!(dto.id, 1);
        assert_eq!(dto.name, "测试徽章");
        assert_eq!(dto.max_supply, Some(100));
        assert_eq!(dto.issued_count, 10);
        assert_eq!(dto.status, BadgeStatus::Draft);
    }
}
