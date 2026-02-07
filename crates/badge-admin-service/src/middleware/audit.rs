//! 审计日志中间件
//!
//! 自动记录所有写操作（POST/PUT/PATCH/DELETE）到 operation_logs 表，
//! 实现运营操作的全链路审计追溯。

use axum::{
    extract::State,
    http::{Method, Request},
    middleware::Next,
    response::Response,
};
use sqlx::PgPool;
use tracing::{error, debug};

use crate::auth::Claims;
use crate::state::AppState;

/// 审计中间件：在写操作成功后异步写入操作日志
///
/// 采用 fire-and-forget 模式，日志写入失败不影响业务响应，
/// 避免审计功能故障导致正常业务不可用。
pub async fn audit_middleware(
    State(state): State<AppState>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let method = request.method().clone();

    // 只拦截写操作，读操作（GET/HEAD/OPTIONS）无需审计
    if !is_write_method(&method) {
        return next.run(request).await;
    }

    let path = request.uri().path().to_string();

    // 认证相关路由属于系统行为，不属于业务操作范畴
    if path.starts_with("/api/admin/auth/") {
        return next.run(request).await;
    }

    // 从 auth 中间件注入的 Claims 中提取操作人信息
    // 未认证的请求（如被 auth 中间件拦截的）不会到达这里
    let claims = request.extensions().get::<Claims>().cloned();

    let ip_address = extract_client_ip(&request);
    let user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let response = next.run(request).await;

    // 只记录成功的写操作，失败操作无实际变更，记录意义不大
    if response.status().is_success() {
        if let Some(claims) = claims {
            let pool = state.pool.clone();
            let (module, action) = parse_module_action(&path, &method);
            let (target_type, target_id) = extract_target(&path);

            // 异步写入避免阻塞业务响应，日志丢失可接受（极端情况下数据库短暂不可用）
            tokio::spawn(async move {
                write_audit_log(
                    &pool,
                    &claims.sub,
                    claims.display_name.as_deref().or(Some(&claims.username)),
                    &module,
                    &action,
                    target_type.as_deref(),
                    target_id.as_deref(),
                    ip_address.as_deref(),
                    user_agent.as_deref(),
                )
                .await;
            });
        }
    }

    response
}

fn is_write_method(method: &Method) -> bool {
    matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
}

/// 从路径中解析业务模块和操作类型
///
/// 约定路径格式为 /api/admin/{module}/...，第一段即为模块名，
/// HTTP 方法映射为标准操作动词（create/update/delete）。
fn parse_module_action(path: &str, method: &Method) -> (String, String) {
    let stripped = path.strip_prefix("/api/admin/").unwrap_or(path);
    let segments: Vec<&str> = stripped.split('/').filter(|s| !s.is_empty()).collect();
    let module = segments.first().unwrap_or(&"unknown").to_string();
    let action = match *method {
        Method::POST => "create".to_string(),
        Method::PUT | Method::PATCH => "update".to_string(),
        Method::DELETE => "delete".to_string(),
        _ => "unknown".to_string(),
    };
    (module, action)
}

/// 从路径中提取操作目标信息
///
/// 如 /api/admin/badges/123 解析为 target_type="badge", target_id="123"，
/// 仅当第二段为纯数字时认为是资源 ID，避免误将子路由名解析为 ID。
fn extract_target(path: &str) -> (Option<String>, Option<String>) {
    let stripped = path.strip_prefix("/api/admin/").unwrap_or(path);
    let segments: Vec<&str> = stripped.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() >= 2 {
        let potential_id = segments[1];
        if potential_id.chars().all(|c| c.is_ascii_digit()) {
            // 复数模块名转单数作为 target_type（badges -> badge）
            let target_type = segments[0].strip_suffix('s').unwrap_or(segments[0]);
            return (Some(target_type.to_string()), Some(potential_id.to_string()));
        }
    }
    (None, None)
}

/// 优先从反向代理设置的 X-Forwarded-For 头提取真实客户端 IP，
/// 因为生产环境通常经过 Nginx/ALB 等代理，直连 IP 是代理地址。
fn extract_client_ip(request: &Request<axum::body::Body>) -> Option<String> {
    request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        // X-Forwarded-For 可能包含多级代理 IP，第一个是真实客户端 IP
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        .or_else(|| {
            request
                .headers()
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
}

/// 写入审计日志到 operation_logs 表
async fn write_audit_log(
    pool: &PgPool,
    operator_id: &str,
    operator_name: Option<&str>,
    module: &str,
    action: &str,
    target_type: Option<&str>,
    target_id: Option<&str>,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) {
    let result = sqlx::query(
        r#"
        INSERT INTO operation_logs
            (operator_id, operator_name, module, action, target_type, target_id, ip_address, user_agent)
        VALUES
            ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(operator_id)
    .bind(operator_name)
    .bind(module)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(ip_address)
    .bind(user_agent)
    .execute(pool)
    .await;

    match result {
        Ok(_) => {
            debug!(
                operator_id = operator_id,
                module = module,
                action = action,
                "审计日志已记录"
            );
        }
        Err(e) => {
            // 审计日志写入失败仅记录错误，不影响业务
            error!(
                error = %e,
                operator_id = operator_id,
                module = module,
                action = action,
                "审计日志写入失败"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_module_action() {
        let (module, action) = parse_module_action("/api/admin/badges", &Method::POST);
        assert_eq!(module, "badges");
        assert_eq!(action, "create");

        let (module, action) = parse_module_action("/api/admin/categories/5", &Method::PUT);
        assert_eq!(module, "categories");
        assert_eq!(action, "update");

        let (module, action) = parse_module_action("/api/admin/series/10", &Method::DELETE);
        assert_eq!(module, "series");
        assert_eq!(action, "delete");
    }

    #[test]
    fn test_extract_target_with_id() {
        let (t, id) = extract_target("/api/admin/badges/123");
        assert_eq!(t, Some("badge".to_string()));
        assert_eq!(id, Some("123".to_string()));
    }

    #[test]
    fn test_extract_target_without_id() {
        let (t, id) = extract_target("/api/admin/badges");
        assert_eq!(t, None);
        assert_eq!(id, None);
    }

    #[test]
    fn test_extract_target_with_sub_resource() {
        // "status" 不是数字，不应被当作 ID
        let (t, id) = extract_target("/api/admin/badges/status");
        assert_eq!(t, None);
        assert_eq!(id, None);
    }

    #[test]
    fn test_is_write_method() {
        assert!(is_write_method(&Method::POST));
        assert!(is_write_method(&Method::PUT));
        assert!(is_write_method(&Method::PATCH));
        assert!(is_write_method(&Method::DELETE));
        assert!(!is_write_method(&Method::GET));
        assert!(!is_write_method(&Method::HEAD));
        assert!(!is_write_method(&Method::OPTIONS));
    }
}
