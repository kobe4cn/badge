//! 分类管理 API 处理器
//!
//! 实现徽章分类的 CRUD 操作

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
        ApiResponse, CategoryDto, CreateCategoryRequest, PageResponse,
        UpdateCategoryRequest,
    },
    error::AdminError,
    state::AppState,
};

/// 数据库查询结果行结构
#[derive(sqlx::FromRow)]
struct CategoryRow {
    id: i64,
    name: String,
    icon_url: Option<String>,
    sort_order: i32,
    status: CategoryStatus,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// 带徽章数量的分类查询结果
#[derive(sqlx::FromRow)]
struct CategoryWithCount {
    id: i64,
    name: String,
    icon_url: Option<String>,
    sort_order: i32,
    status: CategoryStatus,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    badge_count: i64,
}

/// 创建分类
///
/// POST /api/admin/categories
pub async fn create_category(
    State(state): State<AppState>,
    Json(req): Json<CreateCategoryRequest>,
) -> Result<Json<ApiResponse<CategoryDto>>, AdminError> {
    req.validate()?;

    let sort_order = req.sort_order.unwrap_or(0);

    let row = sqlx::query_as::<_, CategoryRow>(
        r#"
        INSERT INTO badge_categories (name, icon_url, sort_order, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id, name, icon_url, sort_order, status, created_at, updated_at
        "#,
    )
    .bind(&req.name)
    .bind(&req.icon_url)
    .bind(sort_order)
    .fetch_one(&state.pool)
    .await?;

    info!(category_id = row.id, name = %req.name, "Category created");

    let dto = CategoryDto {
        id: row.id,
        name: row.name,
        icon_url: row.icon_url,
        sort_order: row.sort_order,
        status: row.status,
        badge_count: 0, // 新创建的分类没有徽章
        created_at: row.created_at,
        updated_at: row.updated_at,
    };

    Ok(Json(ApiResponse::success(dto)))
}

/// 分类列表查询参数
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCategoriesParams {
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

impl ListCategoriesParams {
    /// 计算数据库查询的 offset
    pub fn offset(&self) -> i64 {
        (self.page - 1).max(0) * self.page_size
    }

    /// 获取限制条数（最大100）
    pub fn limit(&self) -> i64 {
        self.page_size.clamp(1, 100)
    }
}

/// 获取分类列表（分页）
///
/// GET /api/admin/categories
pub async fn list_categories(
    State(state): State<AppState>,
    Query(params): Query<ListCategoriesParams>,
) -> Result<Json<ApiResponse<PageResponse<CategoryDto>>>, AdminError> {
    let offset = params.offset();
    let limit = params.limit();

    // 构建模糊搜索参数：当提供 name 时，添加 % 通配符用于 ILIKE 查询
    let name_pattern = params.name.as_ref().map(|n| format!("%{}%", n));

    // 查询总数，支持按名称过滤
    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM badge_categories WHERE ($1::text IS NULL OR name ILIKE $1)"
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

    // 查询分类并统计每个分类下的徽章数量，支持按名称过滤
    let rows = sqlx::query_as::<_, CategoryWithCount>(
        r#"
        SELECT
            c.id,
            c.name,
            c.icon_url,
            c.sort_order,
            c.status,
            c.created_at,
            c.updated_at,
            COALESCE(badge_counts.count, 0) as badge_count
        FROM badge_categories c
        LEFT JOIN (
            SELECT s.category_id, COUNT(b.id) as count
            FROM badge_series s
            LEFT JOIN badges b ON b.series_id = s.id
            GROUP BY s.category_id
        ) badge_counts ON badge_counts.category_id = c.id
        WHERE ($1::text IS NULL OR c.name ILIKE $1)
        ORDER BY c.sort_order ASC, c.id ASC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(&name_pattern)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let categories: Vec<CategoryDto> = rows
        .into_iter()
        .map(|row| CategoryDto {
            id: row.id,
            name: row.name,
            icon_url: row.icon_url,
            sort_order: row.sort_order,
            status: row.status,
            badge_count: row.badge_count,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect();

    let response = PageResponse::new(categories, total.0, params.page, params.page_size);
    Ok(Json(ApiResponse::success(response)))
}

/// 获取全部分类（不分页）
///
/// GET /api/admin/categories/all
///
/// 用于下拉选择等场景
pub async fn list_all_categories(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<CategoryDto>>>, AdminError> {
    let rows = sqlx::query_as::<_, CategoryWithCount>(
        r#"
        SELECT
            c.id,
            c.name,
            c.icon_url,
            c.sort_order,
            c.status,
            c.created_at,
            c.updated_at,
            COALESCE(badge_counts.count, 0) as badge_count
        FROM badge_categories c
        LEFT JOIN (
            SELECT s.category_id, COUNT(b.id) as count
            FROM badge_series s
            LEFT JOIN badges b ON b.series_id = s.id
            GROUP BY s.category_id
        ) badge_counts ON badge_counts.category_id = c.id
        WHERE c.status = 'active'
        ORDER BY c.sort_order ASC, c.id ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    let categories: Vec<CategoryDto> = rows
        .into_iter()
        .map(|row| CategoryDto {
            id: row.id,
            name: row.name,
            icon_url: row.icon_url,
            sort_order: row.sort_order,
            status: row.status,
            badge_count: row.badge_count,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect();

    Ok(Json(ApiResponse::success(categories)))
}

/// 获取分类详情
///
/// GET /api/admin/categories/:id
pub async fn get_category(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<CategoryDto>>, AdminError> {
    let row = sqlx::query_as::<_, CategoryWithCount>(
        r#"
        SELECT
            c.id,
            c.name,
            c.icon_url,
            c.sort_order,
            c.status,
            c.created_at,
            c.updated_at,
            COALESCE(badge_counts.count, 0) as badge_count
        FROM badge_categories c
        LEFT JOIN (
            SELECT s.category_id, COUNT(b.id) as count
            FROM badge_series s
            LEFT JOIN badges b ON b.series_id = s.id
            GROUP BY s.category_id
        ) badge_counts ON badge_counts.category_id = c.id
        WHERE c.id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AdminError::CategoryNotFound(id))?;

    let dto = CategoryDto {
        id: row.id,
        name: row.name,
        icon_url: row.icon_url,
        sort_order: row.sort_order,
        status: row.status,
        badge_count: row.badge_count,
        created_at: row.created_at,
        updated_at: row.updated_at,
    };

    Ok(Json(ApiResponse::success(dto)))
}

/// 更新分类
///
/// PUT /api/admin/categories/:id
pub async fn update_category(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateCategoryRequest>,
) -> Result<Json<ApiResponse<CategoryDto>>, AdminError> {
    req.validate()?;

    // 先检查分类是否存在
    let exists: (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM badge_categories WHERE id = $1)")
            .bind(id)
            .fetch_one(&state.pool)
            .await?;

    if !exists.0 {
        return Err(AdminError::CategoryNotFound(id));
    }

    // 动态构建更新字段
    let row = sqlx::query_as::<_, CategoryRow>(
        r#"
        UPDATE badge_categories
        SET
            name = COALESCE($2, name),
            icon_url = COALESCE($3, icon_url),
            sort_order = COALESCE($4, sort_order),
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, name, icon_url, sort_order, status, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.icon_url)
    .bind(req.sort_order)
    .fetch_one(&state.pool)
    .await?;

    info!(category_id = id, "Category updated");

    // 查询徽章数量
    let badge_count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(b.id)
        FROM badge_series s
        LEFT JOIN badges b ON b.series_id = s.id
        WHERE s.category_id = $1
        "#,
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await?;

    let dto = CategoryDto {
        id: row.id,
        name: row.name,
        icon_url: row.icon_url,
        sort_order: row.sort_order,
        status: row.status,
        badge_count: badge_count.0,
        created_at: row.created_at,
        updated_at: row.updated_at,
    };

    Ok(Json(ApiResponse::success(dto)))
}

/// 删除分类
///
/// DELETE /api/admin/categories/:id
///
/// 仅允许删除没有关联系列的分类
pub async fn delete_category(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, AdminError> {
    // 检查是否有关联的系列
    let series_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM badge_series WHERE category_id = $1")
            .bind(id)
            .fetch_one(&state.pool)
            .await?;

    if series_count.0 > 0 {
        return Err(AdminError::Validation(format!(
            "分类下存在 {} 个系列，无法删除",
            series_count.0
        )));
    }

    let result = sqlx::query("DELETE FROM badge_categories WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AdminError::CategoryNotFound(id));
    }

    info!(category_id = id, "Category deleted");

    Ok(Json(ApiResponse::<()>::success_empty()))
}

/// 更新分类状态
///
/// PATCH /api/admin/categories/:id/status
pub async fn update_category_status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateStatusRequest>,
) -> Result<Json<ApiResponse<CategoryDto>>, AdminError> {
    let result = sqlx::query(
        "UPDATE badge_categories SET status = $2, updated_at = NOW() WHERE id = $1",
    )
    .bind(id)
    .bind(&req.status)
    .execute(&state.pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AdminError::CategoryNotFound(id));
    }

    info!(category_id = id, status = ?req.status, "Category status updated");

    // 重新查询完整信息
    let row = sqlx::query_as::<_, CategoryWithCount>(
        r#"
        SELECT
            c.id,
            c.name,
            c.icon_url,
            c.sort_order,
            c.status,
            c.created_at,
            c.updated_at,
            COALESCE(badge_counts.count, 0) as badge_count
        FROM badge_categories c
        LEFT JOIN (
            SELECT s.category_id, COUNT(b.id) as count
            FROM badge_series s
            LEFT JOIN badges b ON b.series_id = s.id
            GROUP BY s.category_id
        ) badge_counts ON badge_counts.category_id = c.id
        WHERE c.id = $1
        "#,
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await?;

    let dto = CategoryDto {
        id: row.id,
        name: row.name,
        icon_url: row.icon_url,
        sort_order: row.sort_order,
        status: row.status,
        badge_count: row.badge_count,
        created_at: row.created_at,
        updated_at: row.updated_at,
    };

    Ok(Json(ApiResponse::success(dto)))
}

/// 更新分类排序
///
/// PATCH /api/admin/categories/:id/sort
pub async fn update_category_sort(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateSortRequest>,
) -> Result<Json<ApiResponse<()>>, AdminError> {
    let result = sqlx::query(
        "UPDATE badge_categories SET sort_order = $2, updated_at = NOW() WHERE id = $1",
    )
    .bind(id)
    .bind(req.sort_order)
    .execute(&state.pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AdminError::CategoryNotFound(id));
    }

    info!(category_id = id, sort_order = req.sort_order, "Category sort order updated");
    Ok(Json(ApiResponse::<()>::success_empty()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_category_request_validation() {
        let valid = CreateCategoryRequest {
            name: "测试分类".to_string(),
            icon_url: None,
            sort_order: Some(1),
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateCategoryRequest {
            name: "".to_string(), // 空名称
            icon_url: None,
            sort_order: None,
        };
        assert!(invalid.validate().is_err());
    }
}
