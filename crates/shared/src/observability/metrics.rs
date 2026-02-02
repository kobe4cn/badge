//! Prometheus 指标模块
//!
//! 基于 metrics crate 和 metrics-exporter-prometheus 实现指标收集与导出。
//! 指标通过独立的 HTTP 端口暴露，供 Prometheus 抓取。

use anyhow::Result;
use axum::{Router, routing::get};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::net::SocketAddr;
use std::sync::OnceLock;
use tokio::net::TcpListener;
use tracing::{error, info};

use super::ObservabilityConfig;

/// 全局 Prometheus handle，用于渲染指标
static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Metrics 资源守卫
pub struct MetricsHandle {
    _server_handle: tokio::task::JoinHandle<()>,
}

/// 初始化 Prometheus 指标导出
///
/// 启动一个独立的 HTTP 服务器在指定端口暴露 `/metrics` 端点。
pub async fn init(config: &ObservabilityConfig) -> Result<MetricsHandle> {
    // 构建 Prometheus recorder
    let builder = PrometheusBuilder::new();
    let handle = builder.install_recorder()?;

    // 保存到全局，供其他地方获取指标快照
    let _ = PROMETHEUS_HANDLE.set(handle.clone());

    // 注册服务级别的标签
    register_common_metrics(&config.service_name);

    // 启动指标 HTTP 服务器
    let addr = SocketAddr::from(([0, 0, 0, 0], config.metrics_port));
    let server_handle = start_metrics_server(addr, handle).await?;

    Ok(MetricsHandle {
        _server_handle: server_handle,
    })
}

/// 注册通用指标（预定义的业务指标）
fn register_common_metrics(service_name: &str) {
    // 使用 metrics crate 的宏来描述指标
    // 这些描述会出现在 /metrics 端点的 HELP 注释中

    metrics::describe_counter!("http_requests_total", "Total number of HTTP requests");
    metrics::describe_histogram!(
        "http_request_duration_seconds",
        "HTTP request duration in seconds"
    );

    metrics::describe_counter!("grpc_requests_total", "Total number of gRPC requests");
    metrics::describe_histogram!(
        "grpc_request_duration_seconds",
        "gRPC request duration in seconds"
    );

    metrics::describe_counter!("badge_grants_total", "Total number of badge grants");
    metrics::describe_histogram!(
        "badge_grant_duration_seconds",
        "Badge grant duration in seconds"
    );

    metrics::describe_counter!(
        "cascade_evaluations_total",
        "Total number of cascade evaluations"
    );
    metrics::describe_histogram!(
        "cascade_evaluation_duration_seconds",
        "Cascade evaluation duration in seconds"
    );

    metrics::describe_counter!("redemptions_total", "Total number of redemptions");
    metrics::describe_histogram!(
        "redemption_duration_seconds",
        "Redemption duration in seconds"
    );

    metrics::describe_counter!("rule_evaluations_total", "Total number of rule evaluations");
    metrics::describe_histogram!(
        "rule_evaluation_duration_seconds",
        "Rule evaluation duration in seconds"
    );

    metrics::describe_counter!("benefit_grants_total", "Total number of benefit grants");
    metrics::describe_gauge!("benefit_remaining_stock", "Remaining stock for benefits");

    // 记录服务启动
    metrics::counter!("service_starts_total", "service" => service_name.to_string()).increment(1);
}

/// 启动指标 HTTP 服务器
async fn start_metrics_server(
    addr: SocketAddr,
    handle: PrometheusHandle,
) -> Result<tokio::task::JoinHandle<()>> {
    let app = Router::new()
        .route("/metrics", get(move || std::future::ready(handle.render())))
        .route("/health", get(|| async { "OK" }));

    let listener = TcpListener::bind(addr).await?;
    info!("Metrics server listening on {}", addr);

    let server_handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            error!("Metrics server error: {}", e);
        }
    });

    Ok(server_handle)
}

/// 获取全局 Prometheus handle（用于自定义渲染）
pub fn get_handle() -> Option<&'static PrometheusHandle> {
    PROMETHEUS_HANDLE.get()
}

// ============================================================================
// 便捷的指标记录宏和函数
// ============================================================================

/// 记录 HTTP 请求
#[inline]
pub fn record_http_request(method: &str, path: &str, status: u16, duration_secs: f64) {
    let status_str = status.to_string();
    metrics::counter!(
        "http_requests_total",
        "method" => method.to_string(),
        "path" => path.to_string(),
        "status" => status_str.clone()
    )
    .increment(1);

    metrics::histogram!(
        "http_request_duration_seconds",
        "method" => method.to_string(),
        "path" => path.to_string(),
        "status" => status_str
    )
    .record(duration_secs);
}

/// 记录 gRPC 请求
#[inline]
pub fn record_grpc_request(service: &str, method: &str, status: &str, duration_secs: f64) {
    metrics::counter!(
        "grpc_requests_total",
        "service" => service.to_string(),
        "method" => method.to_string(),
        "status" => status.to_string()
    )
    .increment(1);

    metrics::histogram!(
        "grpc_request_duration_seconds",
        "service" => service.to_string(),
        "method" => method.to_string(),
        "status" => status.to_string()
    )
    .record(duration_secs);
}

/// 记录徽章发放
#[inline]
pub fn record_badge_grant(badge_id: i64, source: &str, status: &str, duration_secs: f64) {
    metrics::counter!(
        "badge_grants_total",
        "badge_id" => badge_id.to_string(),
        "source" => source.to_string(),
        "status" => status.to_string()
    )
    .increment(1);

    metrics::histogram!(
        "badge_grant_duration_seconds",
        "badge_id" => badge_id.to_string(),
        "source" => source.to_string()
    )
    .record(duration_secs);
}

/// 记录级联评估
#[inline]
pub fn record_cascade_evaluation(depth: u32, status: &str, duration_secs: f64) {
    metrics::counter!(
        "cascade_evaluations_total",
        "depth" => depth.to_string(),
        "status" => status.to_string()
    )
    .increment(1);

    metrics::histogram!(
        "cascade_evaluation_duration_seconds",
        "depth" => depth.to_string()
    )
    .record(duration_secs);
}

/// 记录兑换
#[inline]
pub fn record_redemption(rule_id: i64, status: &str, duration_secs: f64) {
    metrics::counter!(
        "redemptions_total",
        "rule_id" => rule_id.to_string(),
        "status" => status.to_string()
    )
    .increment(1);

    metrics::histogram!(
        "redemption_duration_seconds",
        "rule_id" => rule_id.to_string()
    )
    .record(duration_secs);
}

/// 记录规则评估
#[inline]
pub fn record_rule_evaluation(matched: bool, duration_secs: f64) {
    metrics::counter!(
        "rule_evaluations_total",
        "matched" => matched.to_string()
    )
    .increment(1);

    metrics::histogram!("rule_evaluation_duration_seconds").record(duration_secs);
}

/// 记录权益发放
#[inline]
pub fn record_benefit_grant(benefit_type: &str, status: &str) {
    metrics::counter!(
        "benefit_grants_total",
        "benefit_type" => benefit_type.to_string(),
        "status" => status.to_string()
    )
    .increment(1);
}

/// 更新权益库存
#[inline]
pub fn set_benefit_stock(benefit_id: i64, stock: f64) {
    metrics::gauge!(
        "benefit_remaining_stock",
        "benefit_id" => benefit_id.to_string()
    )
    .set(stock);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_functions_do_not_panic() {
        // 即使没有初始化 recorder，这些函数也不应该 panic
        record_http_request("GET", "/api/test", 200, 0.1);
        record_grpc_request("BadgeService", "Grant", "ok", 0.05);
        record_badge_grant(1, "api", "success", 0.2);
        record_cascade_evaluation(2, "success", 0.15);
        record_redemption(1, "success", 0.3);
        record_rule_evaluation(true, 0.01);
        record_benefit_grant("coupon", "success");
        set_benefit_stock(1, 100.0);
    }
}
