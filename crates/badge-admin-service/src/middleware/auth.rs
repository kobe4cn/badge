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
    let path = request.uri().path();

    // 公开路由列表（不需要认证）
    let public_paths = ["/api/admin/auth/login", "/api/admin/health", "/health"];

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

    // 验证 Token
    match state.jwt_manager.verify_token(token) {
        Ok(claims) => {
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

/// 从请求扩展中提取 Claims
///
/// 用于在处理器中获取当前用户信息
pub fn extract_claims(request: &Request<Body>) -> Option<&Claims> {
    request.extensions().get::<Claims>()
}
