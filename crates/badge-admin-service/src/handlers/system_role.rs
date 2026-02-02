//! 角色和权限管理 HTTP 处理器
//!
//! 提供角色的 CRUD 操作和权限分配

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

use crate::dto::{ApiResponse, PageResponse, PaginationParams};
use crate::error::{AdminError, Result};
use crate::state::AppState;

// ============================================
// 请求/响应 DTO
// ============================================

/// 创建角色请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateRoleRequest {
    #[validate(length(min = 2, max = 50, message = "角色编码长度必须在 2-50 之间"))]
    pub code: String,
    #[validate(length(min = 2, max = 100, message = "角色名称长度必须在 2-100 之间"))]
    pub name: String,
    pub description: Option<String>,
    /// 权限 ID 列表
    pub permission_ids: Option<Vec<i64>>,
}

/// 更新角色请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRoleRequest {
    #[validate(length(min = 2, max = 100, message = "角色名称长度必须在 2-100 之间"))]
    pub name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    /// 权限 ID 列表（如果提供则替换现有权限）
    pub permission_ids: Option<Vec<i64>>,
}

/// 角色查询参数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleQueryParams {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    pub code: Option<String>,
    pub name: Option<String>,
    pub enabled: Option<bool>,
}

/// 角色列表项 DTO
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleListItem {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub enabled: bool,
    pub user_count: i64,
    pub permission_count: i64,
    pub created_at: DateTime<Utc>,
}

/// 角色详情 DTO
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleDetail {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub enabled: bool,
    pub permissions: Vec<PermissionInfo>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 权限信息 DTO
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PermissionInfo {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub module: String,
    pub action: String,
    pub description: Option<String>,
}

/// 权限列表 DTO
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionListItem {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub module: String,
    pub action: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub enabled: bool,
}

/// 权限树节点
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionTreeNode {
    pub module: String,
    pub module_name: String,
    pub permissions: Vec<PermissionListItem>,
}

// ============================================
// 数据库模型
// ============================================

#[derive(Debug, FromRow)]
struct RoleRow {
    id: i64,
    code: String,
    name: String,
    description: Option<String>,
    is_system: bool,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct RoleListRow {
    id: i64,
    code: String,
    name: String,
    description: Option<String>,
    is_system: bool,
    enabled: bool,
    user_count: i64,
    permission_count: i64,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct PermissionRow {
    id: i64,
    code: String,
    name: String,
    module: String,
    action: String,
    description: Option<String>,
    sort_order: i32,
    enabled: bool,
}

#[derive(Debug, FromRow)]
struct CountRow {
    count: i64,
}

// ============================================
// 角色 API 处理器
// ============================================

/// 获取角色列表
///
/// GET /api/admin/system/roles
pub async fn list_roles(
    State(state): State<AppState>,
    Query(params): Query<RoleQueryParams>,
) -> Result<Json<ApiResponse<PageResponse<RoleListItem>>>> {
    let page = params.pagination.page.max(1);
    let page_size = params.pagination.page_size.min(100).max(1);
    let offset = (page - 1) * page_size;

    // 构建查询条件
    let mut conditions = vec!["1=1".to_string()];
    let mut bind_index = 1;

    if params.code.is_some() {
        conditions.push(format!("r.code ILIKE ${}", bind_index));
        bind_index += 1;
    }
    if params.name.is_some() {
        conditions.push(format!("r.name ILIKE ${}", bind_index));
        bind_index += 1;
    }
    if params.enabled.is_some() {
        conditions.push(format!("r.enabled = ${}", bind_index));
    }

    let where_clause = conditions.join(" AND ");

    // 查询总数
    let count_sql = format!("SELECT COUNT(*) as count FROM role r WHERE {}", where_clause);

    let mut count_query = sqlx::query_as::<_, CountRow>(&count_sql);
    if let Some(ref code) = params.code {
        count_query = count_query.bind(format!("%{}%", code));
    }
    if let Some(ref name) = params.name {
        count_query = count_query.bind(format!("%{}%", name));
    }
    if let Some(enabled) = params.enabled {
        count_query = count_query.bind(enabled);
    }

    let total = count_query.fetch_one(&state.pool).await?.count;

    // 查询角色列表（带统计）
    let list_sql = format!(
        r#"
        SELECT r.id, r.code, r.name, r.description, r.is_system, r.enabled, r.created_at,
               (SELECT COUNT(*) FROM user_role ur WHERE ur.role_id = r.id) as user_count,
               (SELECT COUNT(*) FROM role_permission rp WHERE rp.role_id = r.id) as permission_count
        FROM role r
        WHERE {}
        ORDER BY r.created_at ASC
        LIMIT {} OFFSET {}
        "#,
        where_clause, page_size, offset
    );

    let mut list_query = sqlx::query_as::<_, RoleListRow>(&list_sql);
    if let Some(ref code) = params.code {
        list_query = list_query.bind(format!("%{}%", code));
    }
    if let Some(ref name) = params.name {
        list_query = list_query.bind(format!("%{}%", name));
    }
    if let Some(enabled) = params.enabled {
        list_query = list_query.bind(enabled);
    }

    let roles = list_query.fetch_all(&state.pool).await?;

    let items: Vec<RoleListItem> = roles
        .into_iter()
        .map(|r| RoleListItem {
            id: r.id,
            code: r.code,
            name: r.name,
            description: r.description,
            is_system: r.is_system,
            enabled: r.enabled,
            user_count: r.user_count,
            permission_count: r.permission_count,
            created_at: r.created_at,
        })
        .collect();

    Ok(Json(ApiResponse::success(PageResponse::new(
        items,
        total,
        page,
        page_size,
    ))))
}

/// 获取角色详情
///
/// GET /api/admin/system/roles/:id
pub async fn get_role(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<RoleDetail>>> {
    let role: RoleRow = sqlx::query_as(
        r#"
        SELECT id, code, name, description, is_system, enabled, created_at, updated_at
        FROM role
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AdminError::Validation(format!("角色不存在: {}", id)))?;

    // 获取角色权限
    let permissions: Vec<PermissionRow> = sqlx::query_as(
        r#"
        SELECT p.id, p.code, p.name, p.module, p.action, p.description, p.sort_order, p.enabled
        FROM permission p
        INNER JOIN role_permission rp ON p.id = rp.permission_id
        WHERE rp.role_id = $1
        ORDER BY p.sort_order ASC
        "#,
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;

    let detail = RoleDetail {
        id: role.id,
        code: role.code,
        name: role.name,
        description: role.description,
        is_system: role.is_system,
        enabled: role.enabled,
        permissions: permissions
            .into_iter()
            .map(|p| PermissionInfo {
                id: p.id,
                code: p.code,
                name: p.name,
                module: p.module,
                action: p.action,
                description: p.description,
            })
            .collect(),
        created_at: role.created_at,
        updated_at: role.updated_at,
    };

    Ok(Json(ApiResponse::success(detail)))
}

/// 创建角色
///
/// POST /api/admin/system/roles
pub async fn create_role(
    State(state): State<AppState>,
    Json(req): Json<CreateRoleRequest>,
) -> Result<Json<ApiResponse<RoleDetail>>> {
    req.validate()?;

    // 检查角色编码是否已存在
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM role WHERE code = $1)")
        .bind(&req.code)
        .fetch_one(&state.pool)
        .await?;

    if exists {
        return Err(AdminError::Validation(format!(
            "角色编码 '{}' 已存在",
            req.code
        )));
    }

    // 创建角色
    let role_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO role (code, name, description, is_system, enabled)
        VALUES ($1, $2, $3, FALSE, TRUE)
        RETURNING id
        "#,
    )
    .bind(&req.code)
    .bind(&req.name)
    .bind(&req.description)
    .fetch_one(&state.pool)
    .await?;

    // 分配权限
    if let Some(ref permission_ids) = req.permission_ids {
        for perm_id in permission_ids {
            sqlx::query(
                "INSERT INTO role_permission (role_id, permission_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(role_id)
            .bind(perm_id)
            .execute(&state.pool)
            .await?;
        }
    }

    // 返回创建的角色详情
    get_role(State(state), Path(role_id)).await
}

/// 更新角色
///
/// PUT /api/admin/system/roles/:id
pub async fn update_role(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateRoleRequest>,
) -> Result<Json<ApiResponse<RoleDetail>>> {
    req.validate()?;

    // 检查角色是否存在
    let role: RoleRow = sqlx::query_as(
        "SELECT id, code, name, description, is_system, enabled, created_at, updated_at FROM role WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AdminError::Validation(format!("角色不存在: {}", id)))?;

    // 系统角色不允许修改编码和删除
    if role.is_system && req.enabled == Some(false) {
        return Err(AdminError::Validation("系统角色不能禁用".to_string()));
    }

    // 更新角色信息
    sqlx::query(
        r#"
        UPDATE role
        SET name = COALESCE($1, name),
            description = COALESCE($2, description),
            enabled = COALESCE($3, enabled),
            updated_at = NOW()
        WHERE id = $4
        "#,
    )
    .bind(&req.name)
    .bind(&req.description)
    .bind(req.enabled)
    .bind(id)
    .execute(&state.pool)
    .await?;

    // 更新权限
    if let Some(ref permission_ids) = req.permission_ids {
        // 删除现有权限
        sqlx::query("DELETE FROM role_permission WHERE role_id = $1")
            .bind(id)
            .execute(&state.pool)
            .await?;

        // 添加新权限
        for perm_id in permission_ids {
            sqlx::query(
                "INSERT INTO role_permission (role_id, permission_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(perm_id)
            .execute(&state.pool)
            .await?;
        }
    }

    // 返回更新后的角色详情
    get_role(State(state), Path(id)).await
}

/// 删除角色
///
/// DELETE /api/admin/system/roles/:id
pub async fn delete_role(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<()>>> {
    // 检查是否是系统角色
    let is_system: bool = sqlx::query_scalar("SELECT is_system FROM role WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .unwrap_or(false);

    if is_system {
        return Err(AdminError::Validation("系统角色不能删除".to_string()));
    }

    let result = sqlx::query("DELETE FROM role WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AdminError::Validation(format!("角色不存在: {}", id)));
    }

    Ok(Json(ApiResponse::success(())))
}

// ============================================
// 权限 API 处理器
// ============================================

/// 获取权限列表
///
/// GET /api/admin/system/permissions
pub async fn list_permissions(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<PermissionListItem>>>> {
    let permissions: Vec<PermissionRow> = sqlx::query_as(
        r#"
        SELECT id, code, name, module, action, description, sort_order, enabled
        FROM permission
        WHERE enabled = TRUE
        ORDER BY sort_order ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    let items: Vec<PermissionListItem> = permissions
        .into_iter()
        .map(|p| PermissionListItem {
            id: p.id,
            code: p.code,
            name: p.name,
            module: p.module,
            action: p.action,
            description: p.description,
            sort_order: p.sort_order,
            enabled: p.enabled,
        })
        .collect();

    Ok(Json(ApiResponse::success(items)))
}

/// 获取权限树
///
/// GET /api/admin/system/permissions/tree
pub async fn get_permission_tree(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<PermissionTreeNode>>>> {
    let permissions: Vec<PermissionRow> = sqlx::query_as(
        r#"
        SELECT id, code, name, module, action, description, sort_order, enabled
        FROM permission
        WHERE enabled = TRUE
        ORDER BY module ASC, sort_order ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    // 按模块分组
    let mut module_map: std::collections::HashMap<String, Vec<PermissionListItem>> =
        std::collections::HashMap::new();

    for p in permissions {
        module_map
            .entry(p.module.clone())
            .or_default()
            .push(PermissionListItem {
                id: p.id,
                code: p.code,
                name: p.name,
                module: p.module,
                action: p.action,
                description: p.description,
                sort_order: p.sort_order,
                enabled: p.enabled,
            });
    }

    // 模块名称映射
    let module_names: std::collections::HashMap<&str, &str> = [
        ("system", "系统管理"),
        ("badge", "徽章管理"),
        ("rule", "规则管理"),
        ("grant", "发放管理"),
        ("benefit", "权益管理"),
        ("user", "用户视图"),
        ("stats", "统计报表"),
        ("log", "操作日志"),
    ]
    .into_iter()
    .collect();

    let tree: Vec<PermissionTreeNode> = module_map
        .into_iter()
        .map(|(module, permissions)| PermissionTreeNode {
            module_name: module_names
                .get(module.as_str())
                .unwrap_or(&module.as_str())
                .to_string(),
            module,
            permissions,
        })
        .collect();

    Ok(Json(ApiResponse::success(tree)))
}
