//! 系统用户管理 HTTP 处理器
//!
//! 提供用户的 CRUD 操作和密码管理

use axum::{
    extract::{Path, Query, Request, State},
    Extension, Json,
};
use crate::middleware::AuditContext;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

use crate::auth::{hash_password, Claims};
use crate::dto::{ApiResponse, PageResponse, PaginationParams};
use crate::error::{AdminError, Result};
use crate::state::AppState;

// ============================================
// 请求/响应 DTO
// ============================================

/// 创建用户请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    #[validate(length(min = 2, max = 50, message = "用户名长度必须在 2-50 之间"))]
    pub username: String,
    #[validate(length(min = 8, max = 100, message = "密码长度必须在 8-100 之间"))]
    pub password: String,
    #[validate(email(message = "邮箱格式不正确"))]
    pub email: Option<String>,
    #[validate(length(max = 100, message = "显示名称最长 100 字符"))]
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    /// 角色 ID 列表
    pub role_ids: Option<Vec<i64>>,
}

/// 更新用户请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
    #[validate(email(message = "邮箱格式不正确"))]
    pub email: Option<String>,
    #[validate(length(max = 100, message = "显示名称最长 100 字符"))]
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub status: Option<String>,
    /// 角色 ID 列表（如果提供则替换现有角色）
    pub role_ids: Option<Vec<i64>>,
}

/// 重置密码请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordRequest {
    #[validate(length(min = 8, max = 100, message = "密码长度必须在 8-100 之间"))]
    pub new_password: String,
}

/// 用户查询参数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserQueryParams {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    pub username: Option<String>,
    pub status: Option<String>,
    pub role_id: Option<i64>,
}

/// 用户列表项 DTO
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserListItem {
    pub id: i64,
    pub username: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub status: String,
    pub roles: Vec<RoleInfo>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// 用户详情 DTO
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDetail {
    pub id: i64,
    pub username: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub status: String,
    pub roles: Vec<RoleInfo>,
    pub permissions: Vec<String>,
    pub failed_login_attempts: i32,
    pub locked_until: Option<DateTime<Utc>>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub password_changed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 角色简要信息
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RoleInfo {
    pub id: i64,
    pub code: String,
    pub name: String,
}

// ============================================
// 数据库模型
// ============================================

#[derive(Debug, FromRow)]
struct UserRow {
    id: i64,
    username: String,
    email: Option<String>,
    display_name: Option<String>,
    avatar_url: Option<String>,
    status: String,
    failed_login_attempts: i32,
    locked_until: Option<DateTime<Utc>>,
    last_login_at: Option<DateTime<Utc>>,
    password_changed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct RoleRow {
    id: i64,
    code: String,
    name: String,
}

#[derive(Debug, FromRow)]
struct CountRow {
    count: i64,
}

// ============================================
// API 处理器
// ============================================

/// 获取用户列表
///
/// GET /api/admin/system/users
pub async fn list_users(
    State(state): State<AppState>,
    Query(params): Query<UserQueryParams>,
) -> Result<Json<ApiResponse<PageResponse<UserListItem>>>> {
    let page = params.pagination.page.max(1);
    let page_size = params.pagination.page_size.clamp(1, 100);
    let offset = (page - 1) * page_size;

    // 构建查询条件
    let mut conditions = vec!["1=1".to_string()];
    let mut bind_index = 1;

    if params.username.is_some() {
        conditions.push(format!("u.username ILIKE ${}", bind_index));
        bind_index += 1;
    }
    if params.status.is_some() {
        conditions.push(format!("u.status = ${}", bind_index));
        bind_index += 1;
    }
    if params.role_id.is_some() {
        conditions.push(format!(
            "EXISTS (SELECT 1 FROM user_role ur WHERE ur.user_id = u.id AND ur.role_id = ${})",
            bind_index
        ));
        bind_index += 1;
    }

    let where_clause = conditions.join(" AND ");

    // LIMIT/OFFSET 也必须使用绑定参数，避免 SQL 注入风险
    let limit_param = bind_index;
    let offset_param = bind_index + 1;

    // 查询总数
    let count_sql = format!(
        "SELECT COUNT(*) as count FROM admin_user u WHERE {}",
        where_clause
    );

    let mut count_query = sqlx::query_as::<_, CountRow>(&count_sql);
    if let Some(ref username) = params.username {
        count_query = count_query.bind(format!("%{}%", username));
    }
    if let Some(ref status) = params.status {
        count_query = count_query.bind(status);
    }
    if let Some(role_id) = params.role_id {
        count_query = count_query.bind(role_id);
    }

    let total = count_query.fetch_one(&state.pool).await?.count;

    // 查询用户列表
    let list_sql = format!(
        r#"
        SELECT u.id, u.username, u.email, u.display_name, u.avatar_url,
               u.status, u.failed_login_attempts, u.locked_until,
               u.last_login_at, u.password_changed_at, u.created_at, u.updated_at
        FROM admin_user u
        WHERE {}
        ORDER BY u.created_at DESC
        LIMIT ${} OFFSET ${}
        "#,
        where_clause, limit_param, offset_param
    );

    let mut list_query = sqlx::query_as::<_, UserRow>(&list_sql);
    if let Some(ref username) = params.username {
        list_query = list_query.bind(format!("%{}%", username));
    }
    if let Some(ref status) = params.status {
        list_query = list_query.bind(status);
    }
    if let Some(role_id) = params.role_id {
        list_query = list_query.bind(role_id);
    }
    list_query = list_query.bind(page_size).bind(offset);

    let users = list_query.fetch_all(&state.pool).await?;

    // 获取用户角色
    let user_ids: Vec<i64> = users.iter().map(|u| u.id).collect();
    let roles_map = get_user_roles(&state.pool, &user_ids).await?;

    let items: Vec<UserListItem> = users
        .into_iter()
        .map(|u| UserListItem {
            id: u.id,
            username: u.username,
            email: u.email,
            display_name: u.display_name,
            avatar_url: u.avatar_url,
            status: u.status,
            roles: roles_map.get(&u.id).cloned().unwrap_or_default(),
            last_login_at: u.last_login_at,
            created_at: u.created_at,
        })
        .collect();

    Ok(Json(ApiResponse::success(PageResponse::new(
        items,
        total,
        page,
        page_size,
    ))))
}

/// 获取用户详情
///
/// GET /api/admin/system/users/:id
pub async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<UserDetail>>> {
    let user: UserRow = sqlx::query_as(
        r#"
        SELECT id, username, email, display_name, avatar_url, status,
               failed_login_attempts, locked_until, last_login_at,
               password_changed_at, created_at, updated_at
        FROM admin_user
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AdminError::UserNotFound(id.to_string()))?;

    // 获取用户角色
    let roles: Vec<RoleRow> = sqlx::query_as(
        r#"
        SELECT r.id, r.code, r.name
        FROM role r
        INNER JOIN user_role ur ON r.id = ur.role_id
        WHERE ur.user_id = $1 AND r.enabled = TRUE
        "#,
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;

    // 获取用户权限
    let permissions: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT DISTINCT p.code
        FROM permission p
        INNER JOIN role_permission rp ON p.id = rp.permission_id
        INNER JOIN user_role ur ON rp.role_id = ur.role_id
        WHERE ur.user_id = $1 AND p.enabled = TRUE
        "#,
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;

    let detail = UserDetail {
        id: user.id,
        username: user.username,
        email: user.email,
        display_name: user.display_name,
        avatar_url: user.avatar_url,
        status: user.status,
        roles: roles
            .into_iter()
            .map(|r| RoleInfo {
                id: r.id,
                code: r.code,
                name: r.name,
            })
            .collect(),
        permissions,
        failed_login_attempts: user.failed_login_attempts,
        locked_until: user.locked_until,
        last_login_at: user.last_login_at,
        password_changed_at: user.password_changed_at,
        created_at: user.created_at,
        updated_at: user.updated_at,
    };

    Ok(Json(ApiResponse::success(detail)))
}

/// 创建用户
///
/// POST /api/admin/system/users
pub async fn create_user(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<ApiResponse<UserDetail>>> {
    // 从 JWT Claims 中提取当前操作者 ID，用于记录创建者
    let claims = request
        .extensions()
        .get::<Claims>()
        .ok_or_else(|| AdminError::Unauthorized("未认证".to_string()))?
        .clone();
    let created_by: i64 = claims
        .sub
        .parse()
        .map_err(|_| AdminError::Internal("无效的用户 ID".to_string()))?;

    // 手动解析请求体为 CreateUserRequest
    let body = axum::body::to_bytes(request.into_body(), 1024 * 64).await
        .map_err(|_| AdminError::Validation("请求体过大或读取失败".to_string()))?;
    let req: CreateUserRequest = serde_json::from_slice(&body)
        .map_err(|e| AdminError::Validation(format!("请求体解析失败: {}", e)))?;
    req.validate()?;

    // 检查用户名是否已存在
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM admin_user WHERE username = $1)")
        .bind(&req.username)
        .fetch_one(&state.pool)
        .await?;

    if exists {
        return Err(AdminError::Validation(format!(
            "用户名 '{}' 已存在",
            req.username
        )));
    }

    // 加密密码
    let password_hash = hash_password(&req.password)?;

    // 创建用户
    let user_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO admin_user (username, password_hash, email, display_name, avatar_url, status, password_changed_at, created_by)
        VALUES ($1, $2, $3, $4, $5, 'ACTIVE', NOW(), $6)
        RETURNING id
        "#,
    )
    .bind(&req.username)
    .bind(&password_hash)
    .bind(&req.email)
    .bind(&req.display_name)
    .bind(&req.avatar_url)
    .bind(created_by)
    .fetch_one(&state.pool)
    .await?;

    // 分配角色
    if let Some(ref role_ids) = req.role_ids {
        for role_id in role_ids {
            sqlx::query("INSERT INTO user_role (user_id, role_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
                .bind(user_id)
                .bind(role_id)
                .execute(&state.pool)
                .await?;
        }
    }

    // 返回创建的用户详情
    get_user(State(state), Path(user_id)).await
}

/// 更新用户
///
/// PUT /api/admin/system/users/:id
pub async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(audit_ctx): Extension<AuditContext>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<ApiResponse<UserDetail>>> {
    req.validate()?;

    // 检查用户是否存在
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM admin_user WHERE id = $1)")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;

    if !exists {
        return Err(AdminError::UserNotFound(id.to_string()));
    }

    // 审计快照：记录变更前状态
    audit_ctx.snapshot(&state.pool, "admin_user", id).await;

    // 更新用户信息
    sqlx::query(
        r#"
        UPDATE admin_user
        SET email = COALESCE($1, email),
            display_name = COALESCE($2, display_name),
            avatar_url = COALESCE($3, avatar_url),
            status = COALESCE($4, status),
            updated_at = NOW()
        WHERE id = $5
        "#,
    )
    .bind(&req.email)
    .bind(&req.display_name)
    .bind(&req.avatar_url)
    .bind(&req.status)
    .bind(id)
    .execute(&state.pool)
    .await?;

    // 更新角色
    if let Some(ref role_ids) = req.role_ids {
        // 删除现有角色
        sqlx::query("DELETE FROM user_role WHERE user_id = $1")
            .bind(id)
            .execute(&state.pool)
            .await?;

        // 添加新角色
        for role_id in role_ids {
            sqlx::query("INSERT INTO user_role (user_id, role_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
                .bind(id)
                .bind(role_id)
                .execute(&state.pool)
                .await?;
        }
    }

    // 返回更新后的用户详情
    get_user(State(state), Path(id)).await
}

/// 删除用户
///
/// DELETE /api/admin/system/users/:id
pub async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(audit_ctx): Extension<AuditContext>,
) -> Result<Json<ApiResponse<()>>> {
    // 检查是否是系统管理员（ID=1 不允许删除）
    if id == 1 {
        return Err(AdminError::Validation("不能删除系统管理员".to_string()));
    }

    // 审计快照：记录变更前状态
    audit_ctx.snapshot(&state.pool, "admin_user", id).await;

    let result = sqlx::query("DELETE FROM admin_user WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AdminError::UserNotFound(id.to_string()));
    }

    Ok(Json(ApiResponse::success(())))
}

/// 重置用户密码
///
/// POST /api/admin/system/users/:id/reset-password
pub async fn reset_password(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<ResetPasswordRequest>,
) -> Result<Json<ApiResponse<()>>> {
    req.validate()?;

    // 加密新密码
    let password_hash = hash_password(&req.new_password)?;

    // 重置密码后强制用户下次登录修改密码，避免管理员分配的临时密码长期使用
    let result = sqlx::query(
        r#"
        UPDATE admin_user
        SET password_hash = $1,
            must_change_password = TRUE,
            password_changed_at = NOW(),
            failed_login_attempts = 0,
            locked_until = NULL,
            updated_at = NOW()
        WHERE id = $2
        "#,
    )
    .bind(&password_hash)
    .bind(id)
    .execute(&state.pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AdminError::UserNotFound(id.to_string()));
    }

    Ok(Json(ApiResponse::success(())))
}

// ============================================
// 辅助函数
// ============================================

/// 批量获取用户角色
async fn get_user_roles(
    pool: &sqlx::PgPool,
    user_ids: &[i64],
) -> Result<std::collections::HashMap<i64, Vec<RoleInfo>>> {
    if user_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    #[derive(FromRow)]
    struct UserRoleRow {
        user_id: i64,
        role_id: i64,
        code: String,
        name: String,
    }

    let placeholders: Vec<String> = (1..=user_ids.len()).map(|i| format!("${}", i)).collect();
    let sql = format!(
        r#"
        SELECT ur.user_id, r.id as role_id, r.code, r.name
        FROM user_role ur
        INNER JOIN role r ON ur.role_id = r.id
        WHERE ur.user_id IN ({}) AND r.enabled = TRUE
        "#,
        placeholders.join(", ")
    );

    let mut query = sqlx::query_as::<_, UserRoleRow>(&sql);
    for id in user_ids {
        query = query.bind(id);
    }

    let rows = query.fetch_all(pool).await?;

    let mut map: std::collections::HashMap<i64, Vec<RoleInfo>> = std::collections::HashMap::new();
    for row in rows {
        map.entry(row.user_id).or_default().push(RoleInfo {
            id: row.role_id,
            code: row.code,
            name: row.name,
        });
    }

    Ok(map)
}
