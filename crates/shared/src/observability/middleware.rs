//! HTTP 和 gRPC 中间件
//!
//! 提供请求追踪和指标收集的中间件。
//! 具体实现将在 Task 3.4 中完善。

use std::time::Instant;

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use tracing::{info_span, Instrument};

use super::metrics;

/// HTTP 请求追踪和指标中间件
///
/// 为每个请求创建追踪 span 并记录指标。
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
    let uri = request.uri().path().to_string();

    // 创建追踪 span
    let span = info_span!(
        "http_request",
        method = %method,
        uri = %uri,
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

    // 记录指标
    metrics::record_http_request(&method, &uri, status, latency.as_secs_f64());

    response
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
    request.extensions_mut().insert(RequestId(request_id.clone()));

    let mut response = next.run(request).await;

    // 在响应头中返回请求 ID
    response.headers_mut().insert(
        "x-request-id",
        request_id.parse().unwrap_or_else(|_| "unknown".parse().unwrap()),
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
    #[test]
    fn test_request_id_generation() {
        let id1 = uuid::Uuid::new_v4().to_string();
        let id2 = uuid::Uuid::new_v4().to_string();
        assert_ne!(id1, id2);
    }
}
