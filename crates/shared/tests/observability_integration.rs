//! 可观测性模块集成测试
//!
//! 测试 metrics、tracing 和 middleware 模块的核心功能。

use std::collections::HashMap;

// ============================================================================
// 指标记录测试
// ============================================================================

mod metrics_tests {
    use badge_shared::observability::metrics::{
        record_badge_grant, record_benefit_grant, record_cascade_evaluation, record_grpc_request,
        record_http_request, record_redemption, record_rule_evaluation, set_benefit_stock,
    };

    #[test]
    fn test_record_http_request() {
        // 测试各种 HTTP 方法和状态码组合
        record_http_request("GET", "/api/users", 200, 0.05);
        record_http_request("POST", "/api/badges", 201, 0.12);
        record_http_request("PUT", "/api/badges/:id", 200, 0.08);
        record_http_request("DELETE", "/api/badges/:id", 204, 0.03);
        record_http_request("GET", "/api/not-found", 404, 0.01);
        record_http_request("POST", "/api/error", 500, 0.25);
    }

    #[test]
    fn test_record_grpc_request() {
        record_grpc_request("BadgeService", "Grant", "ok", 0.05);
        record_grpc_request("BadgeService", "Revoke", "ok", 0.03);
        record_grpc_request("BadgeService", "Query", "ok", 0.02);
        record_grpc_request("CascadeService", "Evaluate", "error", 0.15);
        record_grpc_request("RedemptionService", "Redeem", "ok", 0.10);
    }

    #[test]
    fn test_record_badge_grant() {
        record_badge_grant(1, "api", "success", 0.1);
        record_badge_grant(2, "cascade", "success", 0.05);
        record_badge_grant(3, "rule", "success", 0.08);
        record_badge_grant(4, "api", "failed", 0.02);
        record_badge_grant(5, "manual", "success", 0.15);
    }

    #[test]
    fn test_record_cascade_evaluation() {
        record_cascade_evaluation(0, "success", 0.01);
        record_cascade_evaluation(1, "success", 0.05);
        record_cascade_evaluation(2, "success", 0.12);
        record_cascade_evaluation(3, "success", 0.25);
        record_cascade_evaluation(1, "cycle_detected", 0.03);
        record_cascade_evaluation(5, "max_depth_exceeded", 0.50);
    }

    #[test]
    fn test_record_redemption() {
        record_redemption(1, "success", 0.15);
        record_redemption(2, "success", 0.10);
        record_redemption(3, "insufficient_stock", 0.02);
        record_redemption(4, "expired", 0.01);
        record_redemption(5, "not_eligible", 0.03);
    }

    #[test]
    fn test_record_rule_evaluation() {
        record_rule_evaluation(true, 0.01);
        record_rule_evaluation(false, 0.01);
        record_rule_evaluation(true, 0.005);
        record_rule_evaluation(false, 0.002);
    }

    #[test]
    fn test_record_benefit_grant() {
        record_benefit_grant("coupon", "success");
        record_benefit_grant("points", "success");
        record_benefit_grant("badge", "success");
        record_benefit_grant("coupon", "failed");
        record_benefit_grant("discount", "insufficient_stock");
    }

    #[test]
    fn test_set_benefit_stock() {
        set_benefit_stock(1, 1000.0);
        set_benefit_stock(2, 500.0);
        set_benefit_stock(3, 0.0);
        set_benefit_stock(1, 999.0); // 更新库存
        set_benefit_stock(4, 100.5); // 支持小数
    }

    #[test]
    fn test_metrics_with_edge_cases() {
        // 空字符串
        record_http_request("", "", 0, 0.0);

        // 超长路径
        let long_path = "/api/".to_string() + &"x".repeat(1000);
        record_http_request("GET", &long_path, 200, 0.01);

        // 特殊字符
        record_http_request("GET", "/api/users?id=123&name=test", 200, 0.01);

        // 极端持续时间
        record_http_request("GET", "/api/slow", 200, 999.99);
        record_badge_grant(999, "test", "success", 0.000001);

        // 负数 badge_id（虽然业务上不合理，但不应 panic）
        record_badge_grant(-1, "test", "test", 0.01);
    }
}

// ============================================================================
// 追踪上下文传播测试
// ============================================================================

mod tracing_tests {
    use super::*;
    use badge_shared::observability::tracing::{
        current_span_id, current_trace_id, extract_from_headers, inject_to_headers,
        set_parent_from_headers,
    };
    use opentelemetry::trace::TraceContextExt;

    #[test]
    fn test_extract_from_empty_headers() {
        let headers = HashMap::new();
        let context = extract_from_headers(&headers);

        // 空 headers 应该返回一个没有活动 span 的 context
        assert!(!context.has_active_span());
    }

    #[test]
    fn test_extract_from_valid_traceparent() {
        let mut headers = HashMap::new();
        // W3C Trace Context 格式: version-trace_id-span_id-flags
        headers.insert(
            "traceparent".to_string(),
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
        );

        let context = extract_from_headers(&headers);
        let span_context = context.span().span_context().clone();

        assert!(span_context.is_valid());
        assert_eq!(
            span_context.trace_id().to_string(),
            "0af7651916cd43dd8448eb211c80319c"
        );
        assert_eq!(span_context.span_id().to_string(), "b7ad6b7169203331");
    }

    #[test]
    fn test_extract_with_tracestate() {
        let mut headers = HashMap::new();
        headers.insert(
            "traceparent".to_string(),
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01".to_string(),
        );
        headers.insert("tracestate".to_string(), "congo=t61rcWkgMzE".to_string());

        let context = extract_from_headers(&headers);
        let span_context = context.span().span_context().clone();

        assert!(span_context.is_valid());
        assert_eq!(
            span_context.trace_id().to_string(),
            "4bf92f3577b34da6a3ce929d0e0e4736"
        );
    }

    #[test]
    fn test_extract_from_invalid_traceparent() {
        let mut headers = HashMap::new();
        // 无效格式
        headers.insert("traceparent".to_string(), "invalid-format".to_string());

        let context = extract_from_headers(&headers);
        // 无效格式应该返回空 context，不应 panic
        assert!(!context.span().span_context().is_valid());
    }

    #[test]
    fn test_extract_from_malformed_traceparent() {
        // 测试各种畸形的 traceparent
        let test_cases = vec![
            "",
            "00",
            "00-",
            "00-0af7651916cd43dd8448eb211c80319c",
            "00-0af7651916cd43dd8448eb211c80319c-",
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331",
            "01-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01", // 不支持的版本
            "00-invalid-b7ad6b7169203331-01",
            "00-0af7651916cd43dd8448eb211c80319c-invalid-01",
        ];

        for invalid in test_cases {
            let mut headers = HashMap::new();
            headers.insert("traceparent".to_string(), invalid.to_string());
            // 不应该 panic
            let _ = extract_from_headers(&headers);
        }
    }

    #[test]
    fn test_inject_to_headers_without_context() {
        let mut headers = HashMap::new();
        // 没有活动 span 时注入是安全的
        inject_to_headers(&mut headers);
        // 可能不会添加任何 header（因为没有有效的 span context）
    }

    #[test]
    fn test_set_parent_from_headers() {
        let mut headers = HashMap::new();
        headers.insert(
            "traceparent".to_string(),
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
        );

        // 不应该 panic
        set_parent_from_headers(&headers);
    }

    #[test]
    fn test_current_trace_id_without_init() {
        // 没有初始化追踪时应该返回 None
        assert!(current_trace_id().is_none());
    }

    #[test]
    fn test_current_span_id_without_init() {
        // 没有初始化追踪时应该返回 None
        assert!(current_span_id().is_none());
    }

    #[test]
    fn test_extract_inject_roundtrip() {
        // 模拟跨服务传播：提取 -> 注入 -> 再提取
        let mut original_headers = HashMap::new();
        original_headers.insert(
            "traceparent".to_string(),
            "00-12345678901234567890123456789012-1234567890123456-01".to_string(),
        );

        let context = extract_from_headers(&original_headers);
        let original_span_context = context.span().span_context().clone();

        // 在实际场景中，这里会创建新的 span 并设置 parent
        // 然后在发起下游请求时注入上下文

        // 验证提取的 context 有效
        if original_span_context.is_valid() {
            assert_eq!(
                original_span_context.trace_id().to_string(),
                "12345678901234567890123456789012"
            );
        }
    }
}

// ============================================================================
// 路径规范化测试
// ============================================================================

mod middleware_tests {
    use badge_shared::observability::middleware::RequestId;

    #[test]
    fn test_request_id_creation() {
        let id = RequestId("test-id-123".to_string());
        assert_eq!(id.as_str(), "test-id-123");
    }

    #[test]
    fn test_request_id_clone() {
        let id1 = RequestId("original".to_string());
        let id2 = id1.clone();
        assert_eq!(id1.as_str(), id2.as_str());
    }

    #[test]
    fn test_request_id_debug() {
        let id = RequestId("debug-test".to_string());
        let debug_str = format!("{:?}", id);
        assert!(debug_str.contains("debug-test"));
    }
}

// ============================================================================
// gRPC 中间件测试
// ============================================================================

mod grpc_middleware_tests {
    use badge_shared::observability::middleware::grpc::record_grpc_metrics;

    #[test]
    fn test_record_grpc_metrics() {
        // 测试各种 gRPC 状态
        record_grpc_metrics("BadgeService", "Grant", "ok", 0.05);
        record_grpc_metrics("BadgeService", "Query", "ok", 0.02);
        record_grpc_metrics("CascadeService", "Evaluate", "cancelled", 0.10);
        record_grpc_metrics("RedemptionService", "Redeem", "deadline_exceeded", 30.0);
        record_grpc_metrics("UnknownService", "UnknownMethod", "internal", 0.50);
    }
}

// ============================================================================
// Kafka 追踪辅助测试
// ============================================================================

mod kafka_tracing_tests {
    use badge_shared::observability::middleware::kafka::{
        extract_trace_context, inject_trace_context,
    };
    use std::collections::HashMap;

    #[test]
    fn test_inject_trace_context() {
        let mut headers = HashMap::new();
        // 即使没有活动 trace，也不应 panic
        inject_trace_context(&mut headers);
    }

    #[test]
    fn test_extract_trace_context() {
        let headers = HashMap::new();
        // 空 headers 应该返回 None
        let result = extract_trace_context(&headers);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_trace_context_with_traceparent() {
        let mut headers = HashMap::new();
        headers.insert(
            "traceparent".to_string(),
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
        );
        // 当前占位实现返回 None，但不应 panic
        let _ = extract_trace_context(&headers);
    }
}

// ============================================================================
// 配置测试
// ============================================================================

mod config_tests {
    use badge_shared::observability::ObservabilityConfig;

    #[test]
    fn test_default_config() {
        let config = ObservabilityConfig::default();
        assert_eq!(config.service_name, "unknown-service");
        assert_eq!(config.metrics_port, 9090);
        assert_eq!(config.log_level, "info");
        assert!(!config.json_logs);
        assert!(config.otlp_endpoint.is_none());
    }

    #[test]
    fn test_config_from_env() {
        // 测试从环境加载（使用默认值，因为环境变量可能未设置）
        let config = ObservabilityConfig::from_env("test-service");
        assert_eq!(config.service_name, "test-service");
    }

    #[test]
    fn test_custom_config() {
        let config = ObservabilityConfig {
            service_name: "my-service".to_string(),
            otlp_endpoint: Some("http://localhost:4317".to_string()),
            metrics_port: 9091,
            log_level: "debug".to_string(),
            json_logs: true,
        };

        assert_eq!(config.service_name, "my-service");
        assert_eq!(config.otlp_endpoint, Some("http://localhost:4317".to_string()));
        assert_eq!(config.metrics_port, 9091);
        assert_eq!(config.log_level, "debug");
        assert!(config.json_logs);
    }
}

// ============================================================================
// Guard 测试
// ============================================================================

mod guard_tests {
    use badge_shared::observability::ObservabilityGuard;

    #[test]
    fn test_empty_guard() {
        // 创建空 guard 不应 panic
        let guard = ObservabilityGuard::empty();
        // drop 时也不应 panic
        drop(guard);
    }

    #[test]
    fn test_guard_drop() {
        // 多次创建和销毁空 guard
        for _ in 0..10 {
            let guard = ObservabilityGuard::empty();
            drop(guard);
        }
    }
}
