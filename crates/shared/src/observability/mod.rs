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
///
/// 支持从 toml 配置文件反序列化，字段命名与 `config/default.toml` 中的 `[observability]` 保持一致。
/// 服务启动时通过 `AppConfig::load()` 加载后，使用 `with_service_name()` 注入服务名。
#[derive(Debug, Clone, Deserialize)]
pub struct ObservabilityConfig {
    /// 服务名称，用于标识追踪和指标的来源
    /// 通常由 AppConfig 在加载后自动设置，toml 中无需配置
    #[serde(default)]
    pub service_name: String,

    /// 日志级别（如 "info", "debug"）
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// 日志输出格式：json（结构化）或 pretty（人类可读）
    /// 当为 "json" 时等价于 json_logs = true
    #[serde(default = "default_log_format")]
    pub log_format: String,

    /// 是否启用 Prometheus 指标
    #[serde(default = "default_metrics_enabled")]
    pub metrics_enabled: bool,

    /// Prometheus 指标导出端口，默认 9090
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,

    /// 是否启用分布式追踪
    #[serde(default)]
    pub tracing_enabled: bool,

    /// OpenTelemetry OTLP 端点（如 Jaeger），为空时禁用追踪导出
    #[serde(default)]
    pub tracing_endpoint: Option<String>,
}

fn default_metrics_port() -> u16 {
    9090
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "pretty".to_string()
}

fn default_metrics_enabled() -> bool {
    true
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            service_name: "unknown-service".to_string(),
            log_level: default_log_level(),
            log_format: default_log_format(),
            metrics_enabled: default_metrics_enabled(),
            metrics_port: default_metrics_port(),
            tracing_enabled: false,
            tracing_endpoint: None,
        }
    }
}

impl ObservabilityConfig {
    /// 设置服务名称，返回新的配置实例
    ///
    /// 从 `AppConfig` 加载后调用此方法注入服务名称
    pub fn with_service_name(mut self, service_name: &str) -> Self {
        self.service_name = service_name.to_string();
        self
    }

    /// 是否使用 JSON 格式日志
    pub fn json_logs(&self) -> bool {
        self.log_format == "json"
    }

    /// 获取 OTLP 端点（兼容旧 API）
    pub fn otlp_endpoint(&self) -> Option<&str> {
        if self.tracing_enabled {
            self.tracing_endpoint.as_deref()
        } else {
            None
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
        otlp_endpoint = ?config.otlp_endpoint(),
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
        assert!(!config.json_logs()); 
    }
}
