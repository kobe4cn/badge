//! 认证相关的 HTTP 处理器
//!
//! 提供登录、登出、获取当前用户和刷新 Token 的 API

use axum::{
    extract::{Request, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

use crate::auth::{verify_password, Claims};
use crate::dto::ApiResponse;
use crate::error::{AdminError, Result};
use crate::state::AppState;

// ============================================
// 请求/响应 DTO
// ============================================

/// 登录请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    #[validate(length(min = 1, max = 50, message = "用户名长度必须在 1-50 之间"))]
    pub username: String,
    #[validate(length(min = 1, max = 100, message = "密码长度必须在 1-100 之间"))]
    pub password: String,
}

/// 登录响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub token: String,
    pub user: AdminUserDto,
    pub permissions: Vec<String>,
    pub expires_at: i64,
}

/// 当前用户响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentUserResponse {
    pub user: AdminUserDto,
    pub permissions: Vec<String>,
    pub roles: Vec<RoleDto>,
}

/// Token 刷新响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResponse {
    pub token: String,
    pub expires_at: i64,
}

/// 系统用户 DTO
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserDto {
    pub id: i64,
    pub username: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub status: String,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// 角色 DTO
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RoleDto {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
}

// ============================================
// 数据库模型
// ============================================

/// 数据库用户记录
#[derive(Debug, FromRow)]
struct AdminUserRow {
    id: i64,
    username: String,
    password_hash: String,
    email: Option<String>,
    display_name: Option<String>,
    avatar_url: Option<String>,
    status: String,
    failed_login_attempts: i32,
    locked_until: Option<DateTime<Utc>>,
    last_login_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

/// 数据库角色记录
#[derive(Debug, FromRow)]
struct RoleRow {
    id: i64,
    code: String,
    name: String,
    description: Option<String>,
}

/// 数据库权限记录
#[derive(Debug, FromRow)]
struct PermissionRow {
    code: String,
}

// ============================================
// API 处理器
// ============================================

/// 用户登录
///
/// POST /api/admin/auth/login
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>> {
    req.validate()?;

    // 查询用户
    let user: AdminUserRow = sqlx::query_as(
        r#"
        SELECT id, username, password_hash, email, display_name, avatar_url,
               status, failed_login_attempts, locked_until, last_login_at, created_at
        FROM admin_user
        WHERE username = $1
        "#,
    )
    .bind(&req.username)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AdminError::InvalidCredentials)?;

    // 检查用户状态
    if user.status == "DISABLED" {
        return Err(AdminError::UserDisabled);
    }

    // 检查是否被锁定
    if let Some(locked_until) = user.locked_until {
        if locked_until > Utc::now() {
            return Err(AdminError::UserLocked);
        }
    }

    // 验证密码
    let password_valid = verify_password(&req.password, &user.password_hash)?;
    if !password_valid {
        // 更新失败次数
        let new_attempts = user.failed_login_attempts + 1;
        let locked_until = if new_attempts >= 5 {
            // 锁定 30 分钟
            Some(Utc::now() + chrono::Duration::minutes(30))
        } else {
            None
        };

        sqlx::query(
            r#"
            UPDATE admin_user
            SET failed_login_attempts = $1, locked_until = $2, updated_at = NOW()
            WHERE id = $3
            "#,
        )
        .bind(new_attempts)
        .bind(locked_until)
        .bind(user.id)
        .execute(&state.pool)
        .await?;

        return Err(AdminError::InvalidCredentials);
    }

    // 重置失败次数，更新最后登录时间
    sqlx::query(
        r#"
        UPDATE admin_user
        SET failed_login_attempts = 0, locked_until = NULL, last_login_at = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(user.id)
    .execute(&state.pool)
    .await?;

    // 获取用户角色
    let roles: Vec<RoleRow> = sqlx::query_as(
        r#"
        SELECT r.id, r.code, r.name, r.description
        FROM role r
        INNER JOIN user_role ur ON r.id = ur.role_id
        WHERE ur.user_id = $1 AND r.enabled = TRUE
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;

    let role_codes: Vec<String> = roles.iter().map(|r| r.code.clone()).collect();

    // 获取用户权限
    let permissions: Vec<PermissionRow> = sqlx::query_as(
        r#"
        SELECT DISTINCT p.code
        FROM permission p
        INNER JOIN role_permission rp ON p.id = rp.permission_id
        INNER JOIN user_role ur ON rp.role_id = ur.role_id
        WHERE ur.user_id = $1 AND p.enabled = TRUE
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;

    let permission_codes: Vec<String> = permissions.iter().map(|p| p.code.clone()).collect();

    // 生成 Token
    let (token, expires_at) = state.jwt_manager.generate_token(
        user.id,
        &user.username,
        user.display_name.as_deref(),
        role_codes,
        permission_codes.clone(),
    )?;

    let response = LoginResponse {
        token,
        user: AdminUserDto {
            id: user.id,
            username: user.username,
            email: user.email,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
            status: user.status,
            last_login_at: Some(Utc::now()),
            created_at: user.created_at,
        },
        permissions: permission_codes,
        expires_at,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 用户登出
///
/// POST /api/admin/auth/logout
pub async fn logout() -> Result<Json<ApiResponse<()>>> {
    // JWT 是无状态的，登出只需前端清除 Token
    // 如果需要 Token 黑名单，可在此处实现
    Ok(Json(ApiResponse::success(())))
}

/// 获取当前用户信息
///
/// GET /api/admin/auth/me
pub async fn get_current_user(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<ApiResponse<CurrentUserResponse>>> {
    // 从请求扩展中获取 Claims（由认证中间件注入）
    let claims = request
        .extensions()
        .get::<Claims>()
        .ok_or_else(|| AdminError::Unauthorized("未认证".to_string()))?;

    let user_id: i64 = claims
        .sub
        .parse()
        .map_err(|_| AdminError::Internal("无效的用户 ID".to_string()))?;

    // 查询用户
    let user: AdminUserRow = sqlx::query_as(
        r#"
        SELECT id, username, password_hash, email, display_name, avatar_url,
               status, failed_login_attempts, locked_until, last_login_at, created_at
        FROM admin_user
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AdminError::UserNotFound(user_id.to_string()))?;

    // 获取用户角色
    let roles: Vec<RoleRow> = sqlx::query_as(
        r#"
        SELECT r.id, r.code, r.name, r.description
        FROM role r
        INNER JOIN user_role ur ON r.id = ur.role_id
        WHERE ur.user_id = $1 AND r.enabled = TRUE
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?;

    // 获取用户权限
    let permissions: Vec<PermissionRow> = sqlx::query_as(
        r#"
        SELECT DISTINCT p.code
        FROM permission p
        INNER JOIN role_permission rp ON p.id = rp.permission_id
        INNER JOIN user_role ur ON rp.role_id = ur.role_id
        WHERE ur.user_id = $1 AND p.enabled = TRUE
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?;

    let response = CurrentUserResponse {
        user: AdminUserDto {
            id: user.id,
            username: user.username,
            email: user.email,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
            status: user.status,
            last_login_at: user.last_login_at,
            created_at: user.created_at,
        },
        permissions: permissions.iter().map(|p| p.code.clone()).collect(),
        roles: roles
            .iter()
            .map(|r| RoleDto {
                id: r.id,
                code: r.code.clone(),
                name: r.name.clone(),
                description: r.description.clone(),
            })
            .collect(),
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 刷新 Token
///
/// POST /api/admin/auth/refresh
pub async fn refresh_token(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<ApiResponse<RefreshResponse>>> {
    // 从请求扩展中获取 Claims
    let claims = request
        .extensions()
        .get::<Claims>()
        .ok_or_else(|| AdminError::Unauthorized("未认证".to_string()))?;

    // 生成新 Token
    let (token, expires_at) = state.jwt_manager.refresh_token(claims)?;

    Ok(Json(ApiResponse::success(RefreshResponse {
        token,
        expires_at,
    })))
}
