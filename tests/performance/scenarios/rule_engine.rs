//! 规则引擎压力测试
//!
//! 测试规则引擎在高并发事件下的处理能力和规则匹配性能。

use super::super::{LoadTestConfig, LoadTestRunner, PerformanceAssertions};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(test)]
mod rule_engine_tests {
    use super::*;

    /// 规则评估性能测试
    /// 模拟大量事件触发规则评估
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_rule_evaluation_performance() {
        let config = LoadTestConfig {
            concurrent_users: 100,
            duration: Duration::from_secs(60),
            requests_per_second: Some(1000),
            warmup_duration: Duration::from_secs(10),
            request_timeout: Duration::from_secs(5),
        };

        let runner = LoadTestRunner::new(config.clone());
        let client = reqwest::Client::new();
        let base_url = std::env::var("EVENT_ENGAGEMENT_URL")
            .unwrap_or_else(|_| "http://localhost:8081".to_string());

        let event_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

        let metrics = runner
            .run(move || {
                let client = client.clone();
                let counter = event_counter.clone();
                let event_id = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let user_id = format!("perf_user_{}", event_id % 1000);
                let url = format!("{}/api/events", base_url);

                async move {
                    let start = Instant::now();
                    let response = client
                        .post(&url)
                        .json(&serde_json::json!({
                            "event_type": "purchase",
                            "user_id": user_id,
                            "event_id": format!("evt_{}", event_id),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                            "data": {
                                "order_id": format!("order_{}", event_id),
                                "amount": (event_id % 10000) as i64 + 100,
                                "currency": "CNY"
                            }
                        }))
                        .send()
                        .await;
                    let latency = start.elapsed();

                    match response {
                        Ok(resp) if resp.status().is_success() => Ok(latency),
                        Ok(resp) => Err(format!("HTTP {}", resp.status())),
                        Err(e) => Err(e.to_string()),
                    }
                }
            })
            .await;

        // 规则评估性能目标: 成功率 >= 99.5%, P99 <= 500ms
        PerformanceAssertions::assert_success_rate(&metrics, 99.5);
        PerformanceAssertions::assert_p99_latency(&metrics, 500.0);
        PerformanceAssertions::assert_throughput(&metrics, config.duration, 500.0);
    }

    /// 复杂规则链评估测试
    /// 测试包含多个条件和动作的复杂规则
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_complex_rule_chain() {
        let config = LoadTestConfig {
            concurrent_users: 50,
            duration: Duration::from_secs(30),
            requests_per_second: Some(200),
            warmup_duration: Duration::from_secs(5),
            request_timeout: Duration::from_secs(10),
        };

        let runner = LoadTestRunner::new(config.clone());
        let client = reqwest::Client::new();
        let base_url = std::env::var("EVENT_ENGAGEMENT_URL")
            .unwrap_or_else(|_| "http://localhost:8081".to_string());

        let event_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

        let metrics = runner
            .run(move || {
                let client = client.clone();
                let counter = event_counter.clone();
                let event_id = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                // 使用固定用户以触发累计规则
                let user_id = format!("complex_user_{}", event_id % 100);
                let url = format!("{}/api/events", base_url);

                async move {
                    let start = Instant::now();
                    let response = client
                        .post(&url)
                        .json(&serde_json::json!({
                            "event_type": "purchase",
                            "user_id": user_id,
                            "event_id": format!("complex_evt_{}", event_id),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                            "data": {
                                "order_id": format!("complex_order_{}", event_id),
                                "amount": 1000,  // 固定金额触发多条规则
                                "currency": "CNY",
                                "category": "electronics",
                                "is_first_purchase": event_id % 100 == 0
                            }
                        }))
                        .send()
                        .await;
                    let latency = start.elapsed();

                    match response {
                        Ok(resp) if resp.status().is_success() => Ok(latency),
                        Ok(resp) => Err(format!("HTTP {}", resp.status())),
                        Err(e) => Err(e.to_string()),
                    }
                }
            })
            .await;

        // 复杂规则容许更高延迟
        PerformanceAssertions::assert_success_rate(&metrics, 99.0);
        PerformanceAssertions::assert_p99_latency(&metrics, 1000.0);
    }

    /// 规则热更新压力测试
    /// 测试规则更新时不影响正常事件处理
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_rule_hot_reload_under_load() {
        let config = LoadTestConfig {
            concurrent_users: 50,
            duration: Duration::from_secs(30),
            requests_per_second: Some(500),
            warmup_duration: Duration::from_secs(5),
            request_timeout: Duration::from_secs(5),
        };

        let runner = LoadTestRunner::new(config.clone());
        let client = reqwest::Client::new();
        let event_url = std::env::var("EVENT_ENGAGEMENT_URL")
            .unwrap_or_else(|_| "http://localhost:8081".to_string());
        let admin_url = std::env::var("ADMIN_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());

        // 启动后台任务定期刷新规则
        let reload_client = reqwest::Client::new();
        let reload_url = format!("{}/api/cache/rules/refresh", admin_url);
        let reload_handle = tokio::spawn(async move {
            for _ in 0..5 {
                tokio::time::sleep(Duration::from_secs(5)).await;
                let _ = reload_client.post(&reload_url).send().await;
                println!("规则热更新触发");
            }
        });

        let event_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

        let metrics = runner
            .run(move || {
                let client = client.clone();
                let counter = event_counter.clone();
                let event_id = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let user_id = format!("reload_user_{}", event_id);
                let url = format!("{}/api/events", event_url);

                async move {
                    let start = Instant::now();
                    let response = client
                        .post(&url)
                        .json(&serde_json::json!({
                            "event_type": "checkin",
                            "user_id": user_id,
                            "event_id": format!("reload_evt_{}", event_id),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                            "data": {}
                        }))
                        .send()
                        .await;
                    let latency = start.elapsed();

                    match response {
                        Ok(resp) if resp.status().is_success() => Ok(latency),
                        Ok(resp) => Err(format!("HTTP {}", resp.status())),
                        Err(e) => Err(e.to_string()),
                    }
                }
            })
            .await;

        reload_handle.abort();

        // 规则热更新期间不应影响成功率
        PerformanceAssertions::assert_success_rate(&metrics, 99.0);
    }

    /// gRPC 规则服务负载测试
    #[tokio::test]
    #[ignore = "需要运行服务和 gRPC 客户端"]
    async fn test_grpc_rule_engine_load() {
        // gRPC 负载测试需要 tonic 客户端
        // 这里使用 HTTP 代理测试作为替代
        let config = LoadTestConfig {
            concurrent_users: 100,
            duration: Duration::from_secs(60),
            requests_per_second: Some(2000),
            warmup_duration: Duration::from_secs(10),
            request_timeout: Duration::from_secs(3),
        };

        let runner = LoadTestRunner::new(config.clone());
        let client = reqwest::Client::new();
        let base_url = std::env::var("ADMIN_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());

        let metrics = runner
            .run(move || {
                let client = client.clone();
                let url = format!("{}/api/rules/evaluate", base_url);

                async move {
                    let start = Instant::now();
                    let response = client
                        .post(&url)
                        .json(&serde_json::json!({
                            "user_id": format!("user_{}", rand::random::<u32>() % 10000),
                            "event_type": "purchase",
                            "event_data": {
                                "amount": rand::random::<u32>() % 10000
                            }
                        }))
                        .send()
                        .await;
                    let latency = start.elapsed();

                    match response {
                        Ok(resp) if resp.status().is_success() => Ok(latency),
                        Ok(resp) => Err(format!("HTTP {}", resp.status())),
                        Err(e) => Err(e.to_string()),
                    }
                }
            })
            .await;

        // gRPC 应该有更高的吞吐量
        PerformanceAssertions::assert_success_rate(&metrics, 99.9);
        PerformanceAssertions::assert_p99_latency(&metrics, 50.0);
        PerformanceAssertions::assert_throughput(&metrics, config.duration, 1000.0);
    }
}
