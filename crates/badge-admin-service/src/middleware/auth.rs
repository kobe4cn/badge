//! JWT 认证中间件
//!
//! 验证请求中的 Bearer Token 并将用户信息注入请求扩展

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::auth::Claims;
use crate::state::AppState;

/// 认证中间件
///
/// 从 Authorization header 中提取 Bearer Token，验证后将 Claims 注入请求扩展。
/// 对于公开路由（如 /auth/login），跳过验证。
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();

    // 公开路由列表（不需要 JWT 认证）
    // "/api/v1/" 是面向外部系统的 API 路由，使用独立的 API Key 认证机制，
    // 在 routes::external_api_routes 的路由层通过 api_key_auth 中间件保护，
    // 因此此处跳过 JWT 验证以避免双重认证冲突。
    let public_paths = [
        "/api/admin/auth/login",
        "/api/admin/health",
        "/api/v1/",
        "/health",
        "/ready",
        "/metrics",
    ];

    // 检查是否是公开路由
    if public_paths.iter().any(|p| path.starts_with(p)) {
        return next.run(request).await;
    }

    // 从 Authorization header 提取 Token
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => &header[7..],
        _ => {
            return unauthorized_response("缺少认证 Token");
        }
    };

    // 验证 Token 签名和有效期
    match state.jwt_manager.verify_token(token) {
        Ok(claims) => {
            // 已登出的 Token 虽然签名仍有效，但必须拒绝使用
            if crate::handlers::auth::is_token_blacklisted(&state.cache, token).await {
                return unauthorized_response("Token 已被注销");
            }

            // 使用默认密码的用户只允许访问认证相关接口（修改密码、查看个人信息、登出），
            // 阻止其执行任何业务操作，从后端层面强制修改密码。
            if claims.must_change_password && !path.starts_with("/api/admin/auth/") {
                return password_change_required_response();
            }

            // 将 Claims 注入请求扩展，供后续处理器使用
            request.extensions_mut().insert(claims);
            next.run(request).await
        }
        Err(e) => unauthorized_response(&e.to_string()),
    }
}

/// 生成 401 未授权响应
fn unauthorized_response(message: &str) -> Response {
    let body = json!({
        "success": false,
        "code": "UNAUTHORIZED",
        "message": message,
        "data": null
    });

    (
        StatusCode::UNAUTHORIZED,
        axum::Json(body),
    )
        .into_response()
}

/// 默认密码未修改时返回 403，前端通过 PASSWORD_CHANGE_REQUIRED 错误码跳转修改密码页面
fn password_change_required_response() -> Response {
    let body = json!({
        "success": false,
        "code": "PASSWORD_CHANGE_REQUIRED",
        "message": "请先修改默认密码",
        "data": null
    });

    (
        StatusCode::FORBIDDEN,
        axum::Json(body),
    )
        .into_response()
}

/// 从请求扩展中提取 Claims
///
/// 用于在处理器中获取当前用户信息
#[allow(dead_code)]
pub fn extract_claims(request: &Request<Body>) -> Option<&Claims> {
    request.extensions().get::<Claims>()
}
