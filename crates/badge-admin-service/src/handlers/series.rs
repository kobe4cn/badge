//! 系列管理 API 处理器
//!
//! 实现徽章系列的 CRUD 操作

use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Utc};
use tracing::info;
use validator::Validate;

use crate::{
    CategoryStatus,
    dto::{
        ApiResponse, CreateSeriesRequest, PageResponse, SeriesDto,
        UpdateSeriesRequest,
    },
    error::AdminError,
    state::AppState,
};

/// 系列数据库查询结果行
#[derive(sqlx::FromRow)]
struct SeriesRow {
    id: i64,
    category_id: i64,
    name: String,
    description: Option<String>,
    cover_url: Option<String>,
    sort_order: i32,
    status: CategoryStatus,
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// 带关联信息的系列查询结果
#[derive(sqlx::FromRow)]
struct SeriesWithInfo {
    id: i64,
    category_id: i64,
    category_name: String,
    name: String,
    description: Option<String>,
    cover_url: Option<String>,
    sort_order: i32,
    status: CategoryStatus,
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
    badge_count: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<SeriesWithInfo> for SeriesDto {
    fn from(row: SeriesWithInfo) -> Self {
        Self {
            id: row.id,
            category_id: row.category_id,
            category_name: row.category_name,
            name: row.name,
            description: row.description,
            cover_url: row.cover_url,
            sort_order: row.sort_order,
            status: row.status,
            start_time: row.start_time,
            end_time: row.end_time,
            badge_count: row.badge_count,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// 系列列表/详情的查询 SQL
const SERIES_WITH_INFO_SQL: &str = r#"
    SELECT
        s.id,
        s.category_id,
        c.name as category_name,
        s.name,
        s.description,
        s.cover_url,
        s.sort_order,
        s.status,
        s.start_time,
        s.end_time,
        s.created_at,
        s.updated_at,
        COALESCE(badge_counts.count, 0) as badge_count
    FROM badge_series s
    JOIN badge_categories c ON c.id = s.category_id
    LEFT JOIN (
        SELECT series_id, COUNT(*) as count
        FROM badges
        GROUP BY series_id
    ) badge_counts ON badge_counts.series_id = s.id
"#;

/// 创建系列
///
/// POST /api/admin/series
pub async fn create_series(
    State(state): State<AppState>,
    Json(req): Json<CreateSeriesRequest>,
) -> Result<Json<ApiResponse<SeriesDto>>, AdminError> {
    req.validate()?;

    // 验证分类存在
    let category: Option<(i64, String)> =
        sqlx::query_as("SELECT id, name FROM badge_categories WHERE id = $1")
            .bind(req.category_id)
            .fetch_optional(&state.pool)
            .await?;

    let category = category.ok_or(AdminError::CategoryNotFound(req.category_id))?;

    let row = sqlx::query_as::<_, SeriesRow>(
        r#"
        INSERT INTO badge_series (category_id, name, description, cover_url, start_time, end_time, status)
        VALUES ($1, $2, $3, $4, $5, $6, 'active')
        RETURNING id, category_id, name, description, cover_url, sort_order, status,
                  start_time, end_time, created_at, updated_at
        "#,
    )
    .bind(req.category_id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.cover_url)
    .bind(req.start_time)
    .bind(req.end_time)
    .fetch_one(&state.pool)
    .await?;

    info!(series_id = row.id, name = %req.name, "Series created");

    let dto = SeriesDto {
        id: row.id,
        category_id: row.category_id,
        category_name: category.1,
        name: row.name,
        description: row.description,
        cover_url: row.cover_url,
        sort_order: row.sort_order,
        status: row.status,
        start_time: row.start_time,
        end_time: row.end_time,
        badge_count: 0, // 新创建的系列没有徽章
        created_at: row.created_at,
        updated_at: row.updated_at,
    };

    Ok(Json(ApiResponse::success(dto)))
}

/// 系列列表查询参数
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSeriesParams {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
    pub name: Option<String>,
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    20
}

impl ListSeriesParams {
    /// 计算数据库查询的 offset
    pub fn offset(&self) -> i64 {
        (self.page - 1).max(0) * self.page_size
    }

    /// 获取限制条数（最大100）
    pub fn limit(&self) -> i64 {
        self.page_size.clamp(1, 100)
    }
}

/// 获取系列列表（分页）
///
/// GET /api/admin/series
pub async fn list_series(
    State(state): State<AppState>,
    Query(params): Query<ListSeriesParams>,
) -> Result<Json<ApiResponse<PageResponse<SeriesDto>>>, AdminError> {
    let offset = params.offset();
    let limit = params.limit();

    // 构建模糊搜索参数：当提供 name 时，添加 % 通配符用于 ILIKE 查询
    let name_pattern = params.name.as_ref().map(|n| format!("%{}%", n));

    // 查询总数，支持按名称过滤
    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM badge_series WHERE ($1::text IS NULL OR name ILIKE $1)"
    )
    .bind(&name_pattern)
    .fetch_one(&state.pool)
    .await?;

    if total.0 == 0 {
        return Ok(Json(ApiResponse::success(PageResponse::empty(
            params.page,
            params.page_size,
        ))));
    }

    // 查询列表，支持按名称过滤
    let sql = format!(
        "{} WHERE ($1::text IS NULL OR s.name ILIKE $1) ORDER BY s.sort_order ASC, s.id ASC LIMIT $2 OFFSET $3",
        SERIES_WITH_INFO_SQL
    );

    let rows = sqlx::query_as::<_, SeriesWithInfo>(&sql)
        .bind(&name_pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await?;

    let series: Vec<SeriesDto> = rows.into_iter().map(Into::into).collect();
    let response = PageResponse::new(series, total.0, params.page, params.page_size);
    Ok(Json(ApiResponse::success(response)))
}

/// 获取全部系列（不分页）
///
/// GET /api/admin/series/all
///
/// 用于下拉选择等场景
pub async fn list_all_series(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<SeriesDto>>>, AdminError> {
    let sql = format!(
        "{} WHERE s.status = 'active' ORDER BY s.sort_order ASC, s.id ASC",
        SERIES_WITH_INFO_SQL
    );

    let rows = sqlx::query_as::<_, SeriesWithInfo>(&sql)
        .fetch_all(&state.pool)
        .await?;

    let series: Vec<SeriesDto> = rows.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(series)))
}

/// 获取系列详情
///
/// GET /api/admin/series/:id
pub async fn get_series(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<SeriesDto>>, AdminError> {
    let sql = format!("{} WHERE s.id = $1", SERIES_WITH_INFO_SQL);

    let row = sqlx::query_as::<_, SeriesWithInfo>(&sql)
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AdminError::SeriesNotFound(id))?;

    Ok(Json(ApiResponse::success(row.into())))
}

/// 更新系列
///
/// PUT /api/admin/series/:id
pub async fn update_series(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateSeriesRequest>,
) -> Result<Json<ApiResponse<SeriesDto>>, AdminError> {
    req.validate()?;

    // 检查系列是否存在
    let exists: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM badge_series WHERE id = $1)")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;

    if !exists.0 {
        return Err(AdminError::SeriesNotFound(id));
    }

    // 如果更新了 category_id，验证新分类存在
    if let Some(category_id) = req.category_id {
        let category_exists: (bool,) =
            sqlx::query_as("SELECT EXISTS(SELECT 1 FROM badge_categories WHERE id = $1)")
                .bind(category_id)
                .fetch_one(&state.pool)
                .await?;

        if !category_exists.0 {
            return Err(AdminError::CategoryNotFound(category_id));
        }
    }

    sqlx::query_as::<_, SeriesRow>(
        r#"
        UPDATE badge_series
        SET
            category_id = COALESCE($2, category_id),
            name = COALESCE($3, name),
            description = COALESCE($4, description),
            cover_url = COALESCE($5, cover_url),
            start_time = COALESCE($6, start_time),
            end_time = COALESCE($7, end_time),
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, category_id, name, description, cover_url, sort_order,
                  status, start_time, end_time, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(req.category_id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.cover_url)
    .bind(req.start_time)
    .bind(req.end_time)
    .fetch_one(&state.pool)
    .await?;

    info!(series_id = id, "Series updated");

    // 重新查询完整信息
    let sql = format!("{} WHERE s.id = $1", SERIES_WITH_INFO_SQL);
    let row = sqlx::query_as::<_, SeriesWithInfo>(&sql)
        .bind(id)
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(ApiResponse::success(row.into())))
}

/// 删除系列
///
/// DELETE /api/admin/series/:id
///
/// 仅允许删除没有关联徽章的系列
pub async fn delete_series(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, AdminError> {
    // 检查是否有关联的徽章
    let badge_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM badges WHERE series_id = $1")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;

    if badge_count.0 > 0 {
        return Err(AdminError::Validation(format!(
            "系列下存在 {} 个徽章，无法删除",
            badge_count.0
        )));
    }

    let result = sqlx::query("DELETE FROM badge_series WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AdminError::SeriesNotFound(id));
    }

    info!(series_id = id, "Series deleted");

    Ok(Json(ApiResponse::<()>::success_empty()))
}

/// 更新系列状态
///
/// PATCH /api/admin/series/:id/status
pub async fn update_series_status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateStatusRequest>,
) -> Result<Json<ApiResponse<SeriesDto>>, AdminError> {
    let result = sqlx::query(
        "UPDATE badge_series SET status = $2, updated_at = NOW() WHERE id = $1",
    )
    .bind(id)
    .bind(&req.status)
    .execute(&state.pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AdminError::SeriesNotFound(id));
    }

    info!(series_id = id, status = ?req.status, "Series status updated");

    let sql = format!("{} WHERE s.id = $1", SERIES_WITH_INFO_SQL);
    let row = sqlx::query_as::<_, SeriesWithInfo>(&sql)
        .bind(id)
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(ApiResponse::success(row.into())))
}

/// 更新系列排序
///
/// PATCH /api/admin/series/:id/sort
pub async fn update_series_sort(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateSortRequest>,
) -> Result<Json<ApiResponse<()>>, AdminError> {
    let result = sqlx::query(
        "UPDATE badge_series SET sort_order = $2, updated_at = NOW() WHERE id = $1",
    )
    .bind(id)
    .bind(req.sort_order)
    .execute(&state.pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AdminError::SeriesNotFound(id));
    }

    info!(series_id = id, sort_order = req.sort_order, "Series sort order updated");
    Ok(Json(ApiResponse::<()>::success_empty()))
}

/// 获取系列下的徽章列表
///
/// GET /api/admin/series/:id/badges
pub async fn list_series_badges(
    State(state): State<AppState>,
    Path(series_id): Path<i64>,
) -> Result<Json<ApiResponse<Vec<SeriesBadgeDto>>>, AdminError> {
    // 验证系列存在
    let exists: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM badge_series WHERE id = $1)")
        .bind(series_id)
        .fetch_optional(&state.pool)
        .await?
        .unwrap_or((false,));

    if !exists.0 {
        return Err(AdminError::SeriesNotFound(series_id));
    }

    let badges = sqlx::query_as::<_, SeriesBadgeRow>(
        r#"
        SELECT id, name, badge_type, status, issued_count, sort_order, created_at
        FROM badges
        WHERE series_id = $1
        ORDER BY sort_order ASC, id ASC
        "#,
    )
    .bind(series_id)
    .fetch_all(&state.pool)
    .await?;

    let items: Vec<SeriesBadgeDto> = badges.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(items)))
}

/// 状态更新请求
#[derive(Debug, serde::Deserialize)]
pub struct UpdateStatusRequest {
    pub status: CategoryStatus,
}

/// 排序更新请求
#[derive(Debug, serde::Deserialize)]
pub struct UpdateSortRequest {
    pub sort_order: i32,
}

/// 系列下徽章的简要信息
#[derive(Debug, serde::Serialize)]
pub struct SeriesBadgeDto {
    pub id: i64,
    pub name: String,
    pub badge_type: crate::BadgeType,
    pub status: crate::BadgeStatus,
    pub issued_count: i64,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct SeriesBadgeRow {
    id: i64,
    name: String,
    badge_type: crate::BadgeType,
    status: crate::BadgeStatus,
    issued_count: i64,
    sort_order: i32,
    created_at: DateTime<Utc>,
}

impl From<SeriesBadgeRow> for SeriesBadgeDto {
    fn from(row: SeriesBadgeRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            badge_type: row.badge_type,
            status: row.status,
            issued_count: row.issued_count,
            sort_order: row.sort_order,
            created_at: row.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_series_request_validation() {
        let valid = CreateSeriesRequest {
            category_id: 1,
            name: "测试系列".to_string(),
            description: Some("描述".to_string()),
            cover_url: None,
            start_time: None,
            end_time: None,
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateSeriesRequest {
            category_id: 1,
            name: "".to_string(), // 空名称
            description: None,
            cover_url: None,
            start_time: None,
            end_time: None,
        };
        assert!(invalid.validate().is_err());
    }
}
