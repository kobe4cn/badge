//! 统一可观测性模块
//!
//! 提供 metrics、tracing、logging 的统一初始化和管理。
//! 所有服务通过单一入口点配置可观测性，确保一致的指标命名和追踪传播。

pub mod metrics;
pub mod middleware;
pub mod tracing;

use ::tracing::info;
use anyhow::Result;
use serde::Deserialize;

/// 可观测性配置
#[derive(Debug, Clone, Deserialize)]
pub struct ObservabilityConfig {
    /// 服务名称，用于标识追踪和指标的来源
    pub service_name: String,

    /// OpenTelemetry OTLP 端点（如 Jaeger）
    /// 为空时禁用分布式追踪导出
    pub otlp_endpoint: Option<String>,

    /// Prometheus 指标导出端口
    /// 默认 9090
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,

    /// 日志级别（如 "info", "debug"）
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// 是否启用 JSON 格式日志
    #[serde(default)]
    pub json_logs: bool,
}

fn default_metrics_port() -> u16 {
    9090
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            service_name: "unknown-service".to_string(),
            otlp_endpoint: None,
            metrics_port: default_metrics_port(),
            log_level: default_log_level(),
            json_logs: false,
        }
    }
}

impl ObservabilityConfig {
    /// 从环境变量加载配置
    pub fn from_env(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
            otlp_endpoint: std::env::var("OTLP_ENDPOINT").ok(),
            metrics_port: std::env::var("METRICS_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_metrics_port),
            log_level: std::env::var("RUST_LOG").unwrap_or_else(|_| default_log_level()),
            json_logs: std::env::var("JSON_LOGS")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
        }
    }
}

/// 可观测性资源守卫
///
/// 持有各种可观测性资源的生命周期。
/// 当 Guard 被 drop 时，会优雅关闭追踪 provider 并刷新待发送数据。
pub struct ObservabilityGuard {
    _metrics_handle: Option<metrics::MetricsHandle>,
    _tracing_guard: Option<tracing::TracingGuard>,
}

impl ObservabilityGuard {
    /// 创建一个空的 Guard（用于测试或禁用可观测性时）
    pub fn empty() -> Self {
        Self {
            _metrics_handle: None,
            _tracing_guard: None,
        }
    }
}

impl Drop for ObservabilityGuard {
    fn drop(&mut self) {
        // TracingGuard 和 MetricsHandle 的 Drop 实现会处理清理工作
        info!("Shutting down observability...");
    }
}

/// 统一初始化可观测性
///
/// 初始化顺序：
/// 1. Tracing（日志和追踪）
/// 2. Metrics（Prometheus 指标）
///
/// # Example
///
/// ```ignore
/// use badge_shared::observability::{init, ObservabilityConfig};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let config = ObservabilityConfig::from_env("badge-admin-service");
///     let _guard = init(&config).await?;
///
///     // 应用逻辑...
///
///     Ok(())
/// }
/// ```
pub async fn init(config: &ObservabilityConfig) -> Result<ObservabilityGuard> {
    // 1. 初始化 tracing
    let tracing_guard = tracing::init(config)?;

    info!(
        service = %config.service_name,
        metrics_port = %config.metrics_port,
        otlp_endpoint = ?config.otlp_endpoint,
        "Observability initialized"
    );

    // 2. 初始化 metrics
    let metrics_handle = metrics::init(config).await?;

    Ok(ObservabilityGuard {
        _metrics_handle: Some(metrics_handle),
        _tracing_guard: Some(tracing_guard),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ObservabilityConfig::default();
        assert_eq!(config.metrics_port, 9090);
        assert_eq!(config.log_level, "info");
        assert!(!config.json_logs);
    }
}
