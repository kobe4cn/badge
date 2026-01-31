//! OpenTelemetry 追踪模块
//!
//! 提供分布式追踪的初始化和配置。
//! 支持 OTLP 协议导出到 Jaeger/Tempo 等后端。

use anyhow::Result;
use opentelemetry::{trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    runtime,
    trace::{RandomIdGenerator, Sampler, TracerProvider},
    Resource,
};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

use super::ObservabilityConfig;

/// Tracing 资源守卫
///
/// 持有 TracerProvider，在 Drop 时优雅关闭并刷新待发送的 span。
pub struct TracingGuard {
    provider: Option<TracerProvider>,
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        if let Some(provider) = self.provider.take() {
            // 优雅关闭 provider，确保所有 span 都被导出
            if let Err(e) = provider.shutdown() {
                eprintln!("Error shutting down tracer provider: {:?}", e);
            }
        }
    }
}

/// 初始化 tracing（日志 + 追踪）
pub fn init(config: &ObservabilityConfig) -> Result<TracingGuard> {
    // 构建环境过滤器
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // 构建日志层
    let fmt_layer = if config.json_logs {
        fmt::layer()
            .json()
            .with_span_events(FmtSpan::CLOSE)
            .with_target(true)
            .with_thread_ids(true)
            .boxed()
    } else {
        fmt::layer()
            .with_target(true)
            .with_thread_ids(false)
            .with_ansi(true)
            .boxed()
    };

    // 根据是否配置 OTLP 端点决定是否启用分布式追踪
    let (otel_layer, provider) = if let Some(endpoint) = &config.otlp_endpoint {
        let provider = init_tracer_provider(&config.service_name, endpoint)?;
        let tracer = provider.tracer(config.service_name.clone());
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        (Some(otel_layer), Some(provider))
    } else {
        (None, None)
    };

    // 组合所有层并初始化
    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer);

    if let Some(otel_layer) = otel_layer {
        subscriber.with(otel_layer).try_init()?;
    } else {
        subscriber.try_init()?;
    }

    Ok(TracingGuard { provider })
}

/// 初始化 OpenTelemetry TracerProvider
fn init_tracer_provider(service_name: &str, endpoint: &str) -> Result<TracerProvider> {
    let resource = Resource::new(vec![KeyValue::new(SERVICE_NAME, service_name.to_string())]);

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()?;

    let provider = TracerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_sampler(Sampler::AlwaysOn)
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(resource)
        .build();

    // 设置为全局 provider
    opentelemetry::global::set_tracer_provider(provider.clone());

    Ok(provider)
}

/// 从当前 span 获取 trace ID（用于日志关联）
pub fn current_trace_id() -> Option<String> {
    use opentelemetry::trace::TraceContextExt;
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    let span = tracing::Span::current();
    let context = span.context();
    let span_ref = context.span();
    let span_context = span_ref.span_context();

    if span_context.is_valid() {
        Some(span_context.trace_id().to_string())
    } else {
        None
    }
}

/// 从当前 span 获取 span ID
pub fn current_span_id() -> Option<String> {
    use opentelemetry::trace::TraceContextExt;
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    let span = tracing::Span::current();
    let context = span.context();
    let span_ref = context.span();
    let span_context = span_ref.span_context();

    if span_context.is_valid() {
        Some(span_context.span_id().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_trace_id_without_init() {
        // 没有初始化时应该返回 None
        assert!(current_trace_id().is_none());
    }
}
