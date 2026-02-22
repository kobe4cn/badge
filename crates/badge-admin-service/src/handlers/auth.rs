//! 认证相关的 HTTP 处理器
//!
//! 提供登录、登出、获取当前用户和刷新 Token 的 API

use axum::{
    extract::{Request, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use std::time::Duration;
use validator::Validate;

use crate::auth::{hash_password, verify_password, Claims};
use crate::dto::ApiResponse;
use crate::error::{AdminError, Result};
use crate::state::AppState;

/// Token 黑名单在 Redis 中的 key 前缀
const TOKEN_BLACKLIST_PREFIX: &str = "token_blacklist:";

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
    /// 前端收到 true 时应强制跳转到修改密码页面
    pub must_change_password: bool,
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
    must_change_password: bool,
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
               status, failed_login_attempts, locked_until, last_login_at, created_at,
               must_change_password
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
    if let Some(locked_until) = user.locked_until
        && locked_until > Utc::now()
    {
        return Err(AdminError::UserLocked);
    }

    // 验证密码
    let password_valid = verify_password(&req.password, &user.password_hash)?;
    if !password_valid {
        // 允许通过环境变量调整防暴力破解策略，便于不同部署环境差异化配置
        let max_attempts: i32 = std::env::var("BADGE_MAX_LOGIN_ATTEMPTS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);
        let lock_duration_mins: i64 = std::env::var("BADGE_LOCK_DURATION_MINS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);

        // 更新失败次数
        let new_attempts = user.failed_login_attempts + 1;
        let locked_until = if new_attempts >= max_attempts {
            Some(Utc::now() + chrono::Duration::minutes(lock_duration_mins))
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

        tracing::warn!(
            username = %req.username,
            attempts = new_attempts,
            max_attempts = max_attempts,
            "登录失败"
        );

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
        user.must_change_password,
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
        must_change_password: user.must_change_password,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 用户登出
///
/// POST /api/admin/auth/logout
///
/// 将当前 Token 加入 Redis 黑名单，使其在过期前也无法再被使用。
/// TTL 设为 Token 剩余有效期，过期后 Redis 自动清理，避免黑名单无限增长。
pub async fn logout(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<ApiResponse<()>>> {
    // 从 Authorization header 提取原始 token
    let token = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| AdminError::Unauthorized("缺少认证 Token".to_string()))?;

    // 解析 token 获取过期时间，用于计算黑名单 TTL
    let claims = state.jwt_manager.verify_token(token)?;

    let now = Utc::now().timestamp();
    let remaining_secs = (claims.exp - now).max(0) as u64;

    if remaining_secs > 0 {
        // 使用 SHA-256 摘要作为 Redis key，避免将完整 token 暴露在缓存键中
        let token_hash = sha256_hex(token);
        let key = format!("{}{}", TOKEN_BLACKLIST_PREFIX, token_hash);

        // 值为 true 即可，重要的是 key 的存在性
        state
            .cache
            .set(&key, &true, Duration::from_secs(remaining_secs))
            .await
            .map_err(|e| AdminError::Redis(e.to_string()))?;

        tracing::info!(
            username = %claims.username,
            remaining_secs = remaining_secs,
            "用户主动登出，Token 已加入黑名单"
        );
    }

    Ok(Json(ApiResponse::success(())))
}

/// 检查 Token 是否在黑名单中
///
/// 供认证中间件调用，在 JWT 签名校验通过后做二次检查
pub async fn is_token_blacklisted(cache: &badge_shared::cache::Cache, token: &str) -> bool {
    let token_hash = sha256_hex(token);
    let key = format!("{}{}", TOKEN_BLACKLIST_PREFIX, token_hash);
    cache.exists(&key).await.unwrap_or(false)
}

/// 对 token 做 SHA-256 哈希，返回十六进制字符串
fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
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
               status, failed_login_attempts, locked_until, last_login_at, created_at,
               must_change_password
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

/// 修改密码请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    #[validate(length(min = 1, message = "旧密码不能为空"))]
    pub old_password: String,
    #[validate(length(min = 8, max = 100, message = "新密码长度必须在 8-100 之间"))]
    pub new_password: String,
}

/// 修改密码
///
/// POST /api/admin/auth/change-password
///
/// 验证旧密码正确后更新密码哈希，同时清除 must_change_password 标记。
/// 种子用户首次登录后会被前端强制跳转到此接口。
pub async fn change_password(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<ApiResponse<()>>> {
    let claims = request
        .extensions()
        .get::<Claims>()
        .ok_or_else(|| AdminError::Unauthorized("未认证".to_string()))?;

    let user_id: i64 = claims
        .sub
        .parse()
        .map_err(|_| AdminError::Internal("无效的用户 ID".to_string()))?;

    // 需要从 body 中提取 JSON，但 request 已经被 extensions 使用
    // 使用 axum::body::to_bytes 手动解析
    let (_, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, 1024 * 1024)
        .await
        .map_err(|_| AdminError::Validation("请求体解析失败".to_string()))?;
    let req: ChangePasswordRequest = serde_json::from_slice(&bytes)
        .map_err(|e| AdminError::Validation(format!("请求格式错误: {}", e)))?;

    req.validate()?;

    // 查询当前密码哈希
    let user: (String,) =
        sqlx::query_as("SELECT password_hash FROM admin_user WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| AdminError::UserNotFound(user_id.to_string()))?;

    // 验证旧密码
    if !verify_password(&req.old_password, &user.0)? {
        return Err(AdminError::InvalidCredentials);
    }

    // 生成新密码哈希
    let new_hash = hash_password(&req.new_password)?;

    // 更新密码并清除强制修改标记
    sqlx::query(
        r#"
        UPDATE admin_user
        SET password_hash = $1, must_change_password = FALSE,
            password_changed_at = NOW(), updated_at = NOW()
        WHERE id = $2
        "#,
    )
    .bind(&new_hash)
    .bind(user_id)
    .execute(&state.pool)
    .await?;

    tracing::info!(user_id = user_id, "用户已修改密码");

    Ok(Json(ApiResponse::success(())))
}
