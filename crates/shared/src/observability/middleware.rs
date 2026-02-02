//! HTTP 和 gRPC 中间件
//!
//! 提供请求追踪和指标收集的中间件。
//! 具体实现将在 Task 3.4 中完善。

use std::time::Instant;

use axum::{extract::Request, middleware::Next, response::Response};
use tracing::{Instrument, info_span};

use super::metrics;

/// HTTP 请求追踪和指标中间件
///
/// 为每个请求创建追踪 span 并记录指标。
/// 自动进行路径规范化，将动态路径参数替换为占位符，避免指标基数爆炸。
///
/// # 路径规范化规则
///
/// - `/api/users/123` → `/api/users/:id`
/// - `/api/badges/abc-def/grants` → `/api/badges/:id/grants`
/// - `/api/orders/ORD-2024-001` → `/api/orders/:id`
///
/// # Example
///
/// ```ignore
/// use axum::{Router, middleware};
/// use badge_shared::observability::middleware::http_tracing;
///
/// let app = Router::new()
///     .route("/api/health", get(health))
///     .layer(middleware::from_fn(http_tracing));
/// ```
pub async fn http_tracing(request: Request, next: Next) -> Response {
    let method = request.method().to_string();
    let raw_path = request.uri().path().to_string();
    let normalized_path = normalize_path(&raw_path);

    // 创建追踪 span
    let span = info_span!(
        "http_request",
        method = %method,
        path = %normalized_path,
        raw_path = %raw_path,
        status = tracing::field::Empty,
        latency_ms = tracing::field::Empty,
    );

    let start = Instant::now();

    // 执行请求
    let response = next.run(request).instrument(span.clone()).await;

    let latency = start.elapsed();
    let status = response.status().as_u16();

    // 记录到 span
    span.record("status", status);
    span.record("latency_ms", latency.as_millis() as i64);

    // 使用规范化路径记录指标，避免高基数问题
    metrics::record_http_request(&method, &normalized_path, status, latency.as_secs_f64());

    response
}

/// 规范化 HTTP 路径，将动态参数替换为占位符
///
/// 这对于指标收集非常重要，避免因为大量不同的 ID 导致指标基数爆炸。
fn normalize_path(path: &str) -> String {
    let segments: Vec<&str> = path.split('/').collect();
    let normalized: Vec<String> = segments
        .into_iter()
        .map(|segment| {
            if segment.is_empty() {
                segment.to_string()
            } else if looks_like_id(segment) {
                ":id".to_string()
            } else {
                segment.to_string()
            }
        })
        .collect();

    normalized.join("/")
}

/// 判断路径段是否看起来像 ID
///
/// 匹配规则：
/// - 纯数字：123, 456789
/// - UUID 格式：550e8400-e29b-41d4-a716-446655440000
/// - 短 UUID：abc123def
/// - 带前缀的 ID：ORD-2024-001, BG250131ABC
fn looks_like_id(segment: &str) -> bool {
    // 空段不是 ID
    if segment.is_empty() {
        return false;
    }

    // 纯数字是 ID
    if segment.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }

    // UUID 格式（带连字符）
    if segment.len() == 36 && segment.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return true;
    }

    // 短 UUID 或混合 ID（8-32 字符的十六进制）
    if (8..=32).contains(&segment.len()) && segment.chars().all(|c| c.is_ascii_hexdigit()) {
        return true;
    }

    // 带前缀的 ID（如 ORD-123, BG250131ABC）
    // 特征：包含数字，且长度适中
    if (6..=20).contains(&segment.len())
        && segment.chars().any(|c| c.is_ascii_digit())
        && segment.chars().any(|c| c.is_ascii_alphabetic())
    {
        return true;
    }

    false
}

/// 简化的请求 ID 中间件
///
/// 为每个请求添加唯一 ID，便于日志关联。
pub async fn request_id(mut request: Request, next: Next) -> Response {
    // 尝试从 header 获取请求 ID，没有则生成新的
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // 将请求 ID 存入 extensions 供后续使用
    request
        .extensions_mut()
        .insert(RequestId(request_id.clone()));

    let mut response = next.run(request).await;

    // 在响应头中返回请求 ID
    response.headers_mut().insert(
        "x-request-id",
        request_id
            .parse()
            .unwrap_or_else(|_| "unknown".parse().unwrap()),
    );

    response
}

/// 请求 ID 包装类型
#[derive(Clone, Debug)]
pub struct RequestId(pub String);

impl RequestId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// gRPC 拦截器（占位）
///
/// 实际实现需要 tonic 的 Interceptor trait。
/// 将在 Task 3.4 中完善。
pub mod grpc {
    use tonic::{Request, Status};

    /// gRPC 请求追踪拦截器函数
    ///
    /// # Example
    ///
    /// ```ignore
    /// use tonic::transport::Server;
    /// use badge_shared::observability::middleware::grpc::tracing_interceptor;
    ///
    /// Server::builder()
    ///     .add_service(MyServiceServer::with_interceptor(service, tracing_interceptor))
    ///     .serve(addr)
    ///     .await?;
    /// ```
    #[allow(clippy::result_large_err)] // tonic 的 Status 类型较大，但这是 Interceptor trait 的签名要求
    pub fn tracing_interceptor<T>(request: Request<T>) -> Result<Request<T>, Status> {
        // 从 metadata 提取 trace context（W3C Trace Context）
        // 这里是占位实现，完整实现在 Task 3.4

        // 记录请求开始
        tracing::debug!("gRPC request received");

        Ok(request)
    }

    /// gRPC 指标记录辅助函数
    pub fn record_grpc_metrics(service: &str, method: &str, status: &str, duration_secs: f64) {
        super::metrics::record_grpc_request(service, method, status, duration_secs);
    }
}

/// Kafka 消息追踪辅助（占位）
///
/// 用于在 Kafka 消息中传播追踪上下文。
/// 将在 Task 3.4 中完善。
pub mod kafka {
    use std::collections::HashMap;

    /// 注入追踪上下文到 Kafka 消息头
    pub fn inject_trace_context(headers: &mut HashMap<String, String>) {
        // 占位实现：将当前 trace context 注入到 headers
        if let Some(trace_id) = super::super::tracing::current_trace_id() {
            headers.insert("traceparent".to_string(), trace_id);
        }
    }

    /// 从 Kafka 消息头提取追踪上下文
    pub fn extract_trace_context(_headers: &HashMap<String, String>) -> Option<String> {
        // 占位实现：从 headers 提取 trace context
        // 完整实现需要使用 opentelemetry-propagator
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_generation() {
        let id1 = uuid::Uuid::new_v4().to_string();
        let id2 = uuid::Uuid::new_v4().to_string();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_normalize_path_numeric_id() {
        assert_eq!(normalize_path("/api/users/123"), "/api/users/:id");
        assert_eq!(
            normalize_path("/api/badges/456/grants"),
            "/api/badges/:id/grants"
        );
    }

    #[test]
    fn test_normalize_path_uuid() {
        assert_eq!(
            normalize_path("/api/users/550e8400-e29b-41d4-a716-446655440000"),
            "/api/users/:id"
        );
    }

    #[test]
    fn test_normalize_path_short_hex() {
        assert_eq!(normalize_path("/api/badges/abc123def"), "/api/badges/:id");
    }

    #[test]
    fn test_normalize_path_prefixed_id() {
        assert_eq!(
            normalize_path("/api/orders/ORD-2024-001"),
            "/api/orders/:id"
        );
        assert_eq!(normalize_path("/api/grants/BG250131ABC"), "/api/grants/:id");
    }

    #[test]
    fn test_normalize_path_static() {
        // 静态路径不应被规范化
        assert_eq!(normalize_path("/api/health"), "/api/health");
        assert_eq!(normalize_path("/api/admin/badges"), "/api/admin/badges");
        assert_eq!(normalize_path("/metrics"), "/metrics");
    }

    #[test]
    fn test_normalize_path_empty_and_root() {
        assert_eq!(normalize_path("/"), "/");
        assert_eq!(normalize_path(""), "");
    }

    #[test]
    fn test_looks_like_id() {
        // 数字 ID
        assert!(looks_like_id("123"));
        assert!(looks_like_id("456789"));

        // UUID
        assert!(looks_like_id("550e8400-e29b-41d4-a716-446655440000"));

        // 短 hex
        assert!(looks_like_id("abc123def"));
        assert!(looks_like_id("deadbeef"));

        // 带前缀的 ID
        assert!(looks_like_id("ORD-2024-001"));
        assert!(looks_like_id("BG250131ABC"));

        // 不是 ID
        assert!(!looks_like_id(""));
        assert!(!looks_like_id("api"));
        assert!(!looks_like_id("users"));
        assert!(!looks_like_id("badges"));
        assert!(!looks_like_id("health"));
    }
}
