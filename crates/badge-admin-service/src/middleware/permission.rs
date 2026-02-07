//! 权限检查中间件
//!
//! 检查用户是否拥有指定的权限

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::future::Future;
use std::pin::Pin;

use crate::auth::Claims;

/// 权限检查中间件工厂
///
/// 创建一个检查指定权限的中间件函数
///
/// # 示例
/// ```ignore
/// .route("/users", get(list_users).layer(axum::middleware::from_fn(require_permission("system:user:read"))))
/// ```
pub fn require_permission(
    permission: &'static str,
) -> impl Fn(
    Request<Body>,
    Next,
) -> Pin<Box<dyn Future<Output = Response> + Send>>
       + Clone
       + Send {
    move |request: Request<Body>, next: Next| {
        let permission = permission;
        Box::pin(async move { check_permission(request, next, permission).await })
    }
}

/// 检查用户是否拥有指定权限
async fn check_permission(request: Request<Body>, next: Next, required_permission: &str) -> Response {
    // 从请求扩展中获取 Claims（由 auth_middleware 注入）
    let claims = match request.extensions().get::<Claims>() {
        Some(claims) => claims.clone(),
        None => {
            return unauthorized_response("未认证");
        }
    };

    // 检查用户是否拥有 admin 角色（admin 拥有所有权限）
    if claims.roles.iter().any(|r| r == "admin") {
        return next.run(request).await;
    }

    // 检查用户是否拥有指定权限
    if claims.permissions.contains(&required_permission.to_string()) {
        return next.run(request).await;
    }

    // 检查通配符权限（如 system:user:* 匹配 system:user:read）
    let permission_parts: Vec<&str> = required_permission.split(':').collect();
    if permission_parts.len() >= 2 {
        let wildcard = format!("{}:{}:*", permission_parts[0], permission_parts[1]);
        if claims.permissions.contains(&wildcard) {
            return next.run(request).await;
        }
    }

    forbidden_response(&format!("缺少权限: {}", required_permission))
}

/// 生成 401 未授权响应
fn unauthorized_response(message: &str) -> Response {
    let body = json!({
        "success": false,
        "code": "UNAUTHORIZED",
        "message": message,
        "data": null
    });

    (StatusCode::UNAUTHORIZED, axum::Json(body)).into_response()
}

/// 生成 403 禁止访问响应
fn forbidden_response(message: &str) -> Response {
    let body = json!({
        "success": false,
        "code": "FORBIDDEN",
        "message": message,
        "data": null
    });

    (StatusCode::FORBIDDEN, axum::Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_permission_matching() {
        let permissions = vec![
            "system:user:read".to_string(),
            "badge:badge:write".to_string(),
        ];

        // 直接匹配
        assert!(permissions.contains(&"system:user:read".to_string()));
        assert!(!permissions.contains(&"system:user:write".to_string()));
    }
}
