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

// ============================================================================
// 追踪上下文传播
// ============================================================================

use opentelemetry::propagation::{Extractor, Injector, TextMapPropagator};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use std::collections::HashMap;

/// HTTP Header 提取器
struct HeaderExtractor<'a>(&'a HashMap<String, String>);

impl<'a> Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(|s| s.as_str())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|s| s.as_str()).collect()
    }
}

/// HTTP Header 注入器
struct HeaderInjector<'a>(&'a mut HashMap<String, String>);

impl<'a> Injector for HeaderInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.to_string(), value);
    }
}

/// 从 HTTP headers 提取追踪上下文
///
/// 用于在接收请求时恢复上游的追踪上下文，实现分布式追踪的链路串联。
/// 支持 W3C Trace Context 标准（traceparent, tracestate）。
///
/// # 示例
///
/// ```ignore
/// let headers = extract_headers_from_request(&request);
/// let context = extract_from_headers(&headers);
/// let span = tracing::info_span!("handle_request");
/// span.set_parent(context);
/// ```
pub fn extract_from_headers(headers: &HashMap<String, String>) -> opentelemetry::Context {
    let propagator = TraceContextPropagator::new();
    propagator.extract(&HeaderExtractor(headers))
}

/// 将当前追踪上下文注入到 HTTP headers
///
/// 用于在发起下游请求时传播追踪上下文，实现分布式追踪的链路传递。
/// 注入 W3C Trace Context 标准格式的 headers。
///
/// # 示例
///
/// ```ignore
/// let mut headers = HashMap::new();
/// inject_to_headers(&mut headers);
/// // headers 现在包含 traceparent 和 tracestate
/// ```
pub fn inject_to_headers(headers: &mut HashMap<String, String>) {
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    let span = tracing::Span::current();
    let context = span.context();

    let propagator = TraceContextPropagator::new();
    propagator.inject_context(&context, &mut HeaderInjector(headers));
}

/// 从 HTTP headers 提取并设置当前 span 的父上下文
///
/// 这是 extract_from_headers 的便捷版本，直接设置当前 span 的父上下文。
pub fn set_parent_from_headers(headers: &HashMap<String, String>) {
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    let context = extract_from_headers(headers);
    tracing::Span::current().set_parent(context);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_trace_id_without_init() {
        // 没有初始化时应该返回 None
        assert!(current_trace_id().is_none());
    }

    #[test]
    fn test_extract_from_empty_headers() {
        let headers = HashMap::new();
        let context = extract_from_headers(&headers);
        // 空 headers 应该返回空的 context，不应该 panic
        assert!(!context.has_active_span());
    }

    #[test]
    fn test_inject_to_headers_without_context() {
        let mut headers = HashMap::new();
        // 没有活动 span 时注入应该是安全的
        inject_to_headers(&mut headers);
        // 可能不会添加任何 header（因为没有有效的 span context）
    }

    #[test]
    fn test_extract_with_traceparent() {
        let mut headers = HashMap::new();
        // 有效的 W3C Trace Context 格式
        headers.insert(
            "traceparent".to_string(),
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
        );

        let context = extract_from_headers(&headers);
        // 应该成功提取 context
        use opentelemetry::trace::TraceContextExt;
        let span_context = context.span().span_context().clone();
        assert!(span_context.is_valid());
        assert_eq!(
            span_context.trace_id().to_string(),
            "0af7651916cd43dd8448eb211c80319c"
        );
    }
}
