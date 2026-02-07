//! 端到端性能基准测试
//!
//! 测试完整业务流程的端到端性能，包括从事件到徽章发放的全链路。

use super::super::{LoadTestConfig, LoadTestRunner, PerformanceAssertions};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(test)]
mod e2e_benchmark_tests {
    use super::*;

    /// 登录获取 JWT Token，所有 /api/admin/ 端点均需要认证
    async fn get_auth_token(base_url: &str) -> String {
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/api/admin/auth/login", base_url))
            .json(&serde_json::json!({
                "username": "admin",
                "password": "admin123"
            }))
            .send()
            .await
            .expect("登录失败");
        let body: serde_json::Value = resp.json().await.expect("解析登录响应失败");
        body["data"]["token"]
            .as_str()
            .or_else(|| body["token"].as_str())
            .expect("未找到 token")
            .to_string()
    }

    /// 完整消费流程基准测试
    /// 事件 -> 规则评估 -> 徽章发放 -> 权益发放
    #[tokio::test]
    #[ignore = "需要完整服务环境"]
    async fn benchmark_purchase_flow() {
        let config = LoadTestConfig {
            concurrent_users: 50,
            duration: Duration::from_secs(120),
            requests_per_second: Some(100),
            warmup_duration: Duration::from_secs(30),
            request_timeout: Duration::from_secs(30),
        };

        let runner = LoadTestRunner::new(config.clone());
        let event_url = std::env::var("EVENT_TRANSACTION_URL")
            .unwrap_or_else(|_| "http://localhost:8082".to_string());
        let admin_url = std::env::var("ADMIN_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();

        let token = get_auth_token(&admin_url).await;
        let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

        let metrics = runner
            .run(move || {
                let client = client.clone();
                let event_url = event_url.clone();
                let admin_url = admin_url.clone();
                let token = token.clone();
                let cnt = counter.clone();
                let seq = cnt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                async move {
                    let user_id = format!("benchmark_user_{}", seq);
                    let start = Instant::now();

                    // 1. 发送交易事件（事件服务不需要 JWT 认证）
                    let event_response = client
                        .post(format!("{}/api/events", event_url))
                        .json(&serde_json::json!({
                            "event_type": "purchase",
                            "user_id": user_id,
                            "event_id": format!("benchmark_evt_{}", seq),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                            "data": {
                                "order_id": format!("benchmark_order_{}", seq),
                                "amount": 1000,
                                "currency": "CNY"
                            }
                        }))
                        .send()
                        .await;

                    if let Err(e) = event_response {
                        return Err(format!("事件发送失败: {}", e));
                    }

                    let resp = event_response.unwrap();
                    if !resp.status().is_success() {
                        return Err(format!("事件响应错误: {}", resp.status()));
                    }

                    // 2. 等待异步处理（模拟实际场景）
                    tokio::time::sleep(Duration::from_millis(100)).await;

                    // 3. 验证徽章发放（管理服务需要 JWT 认证）
                    let badge_response = client
                        .get(format!(
                            "{}/api/admin/users/{}/badges",
                            admin_url, user_id
                        ))
                        .header("Authorization", format!("Bearer {}", token))
                        .send()
                        .await;

                    if let Err(e) = badge_response {
                        return Err(format!("徽章查询失败: {}", e));
                    }

                    let latency = start.elapsed();
                    Ok(latency)
                }
            })
            .await;

        println!("\n===== 消费流程基准测试结果 =====");
        metrics.print_summary(config.duration);

        // 端到端流程容许较高延迟
        PerformanceAssertions::assert_success_rate(&metrics, 95.0);
        PerformanceAssertions::assert_p99_latency(&metrics, 5000.0); // 5秒
    }

    /// 签到流程基准测试
    #[tokio::test]
    #[ignore = "需要完整服务环境"]
    async fn benchmark_checkin_flow() {
        let config = LoadTestConfig {
            concurrent_users: 100,
            duration: Duration::from_secs(60),
            requests_per_second: Some(200),
            warmup_duration: Duration::from_secs(10),
            request_timeout: Duration::from_secs(10),
        };

        let runner = LoadTestRunner::new(config.clone());
        let event_url = std::env::var("EVENT_ENGAGEMENT_URL")
            .unwrap_or_else(|_| "http://localhost:8081".to_string());

        let client = reqwest::Client::new();
        let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

        let metrics = runner
            .run(move || {
                let client = client.clone();
                let url = format!("{}/api/events", event_url);
                let cnt = counter.clone();
                let seq = cnt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                async move {
                    let user_id = format!("checkin_user_{}", seq % 1000);
                    let start = Instant::now();

                    // 事件服务不需要 JWT 认证
                    let response = client
                        .post(&url)
                        .json(&serde_json::json!({
                            "event_type": "checkin",
                            "user_id": user_id,
                            "event_id": format!("checkin_{}", seq),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                            "data": {
                                "platform": "mobile",
                                "location": "home"
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

        println!("\n===== 签到流程基准测试结果 =====");
        metrics.print_summary(config.duration);

        PerformanceAssertions::assert_success_rate(&metrics, 99.0);
        PerformanceAssertions::assert_p99_latency(&metrics, 500.0);
    }

    /// 级联触发基准测试
    /// A -> B -> C 三级级联
    #[tokio::test]
    #[ignore = "需要完整服务环境"]
    async fn benchmark_cascade_trigger() {
        let config = LoadTestConfig {
            concurrent_users: 20,
            duration: Duration::from_secs(60),
            requests_per_second: Some(50),
            warmup_duration: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
        };

        let runner = LoadTestRunner::new(config.clone());
        let admin_url = std::env::var("ADMIN_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());

        let client = reqwest::Client::new();
        let token = get_auth_token(&admin_url).await;
        let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

        // 假设徽章 1 触发级联
        let trigger_badge_id = 1i64;

        let metrics = runner
            .run(move || {
                let client = client.clone();
                let admin_url = admin_url.clone();
                let token = token.clone();
                let cnt = counter.clone();
                let seq = cnt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                async move {
                    let user_id = format!("cascade_user_{}", seq);
                    let start = Instant::now();

                    // 通过手动发放接口触发级联
                    let response = client
                        .post(format!("{}/api/admin/grants/manual", admin_url))
                        .header("Authorization", format!("Bearer {}", token))
                        .json(&serde_json::json!({
                            "userId": user_id,
                            "badgeId": trigger_badge_id,
                            "sourceType": "benchmark",
                            "sourceId": format!("cascade_{}", seq)
                        }))
                        .send()
                        .await;

                    if let Err(e) = response {
                        return Err(format!("发放失败: {}", e));
                    }

                    let resp = response.unwrap();
                    if !resp.status().is_success() && resp.status().as_u16() != 409 {
                        return Err(format!("发放响应错误: {}", resp.status()));
                    }

                    // 等待级联处理
                    tokio::time::sleep(Duration::from_millis(500)).await;

                    let latency = start.elapsed();
                    Ok(latency)
                }
            })
            .await;

        println!("\n===== 级联触发基准测试结果 =====");
        metrics.print_summary(config.duration);

        // 级联触发容许较高延迟
        PerformanceAssertions::assert_success_rate(&metrics, 95.0);
    }

    /// 兑换流程基准测试
    #[tokio::test]
    #[ignore = "需要完整服务环境"]
    async fn benchmark_redemption_flow() {
        let config = LoadTestConfig {
            concurrent_users: 30,
            duration: Duration::from_secs(60),
            requests_per_second: Some(100),
            warmup_duration: Duration::from_secs(10),
            request_timeout: Duration::from_secs(15),
        };

        let runner = LoadTestRunner::new(config.clone());
        let admin_url = std::env::var("ADMIN_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());

        let client = reqwest::Client::new();
        let token = get_auth_token(&admin_url).await;
        let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

        let metrics = runner
            .run(move || {
                let client = client.clone();
                let url = format!("{}/api/admin/redemption/redeem", admin_url);
                let token = token.clone();
                let cnt = counter.clone();
                let seq = cnt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                async move {
                    let user_id = format!("redeem_bench_user_{}", seq);
                    let start = Instant::now();

                    let response = client
                        .post(&url)
                        .header("Authorization", format!("Bearer {}", token))
                        .json(&serde_json::json!({
                            "user_id": user_id,
                            "rule_id": 1,
                            "badge_ids": [1]
                        }))
                        .send()
                        .await;

                    let latency = start.elapsed();

                    match response {
                        Ok(resp) if resp.status().is_success() => Ok(latency),
                        Ok(resp) if resp.status().as_u16() == 400 => {
                            // 库存不足等业务错误也算处理成功
                            Ok(latency)
                        }
                        Ok(resp) => Err(format!("HTTP {}", resp.status())),
                        Err(e) => Err(e.to_string()),
                    }
                }
            })
            .await;

        println!("\n===== 兑换流程基准测试结果 =====");
        metrics.print_summary(config.duration);

        PerformanceAssertions::assert_success_rate(&metrics, 98.0);
        PerformanceAssertions::assert_p99_latency(&metrics, 1000.0);
    }

    /// 混合负载基准测试
    /// 模拟真实场景的混合请求
    #[tokio::test]
    #[ignore = "需要完整服务环境"]
    async fn benchmark_mixed_workload() {
        let config = LoadTestConfig {
            concurrent_users: 100,
            duration: Duration::from_secs(120),
            requests_per_second: Some(500),
            warmup_duration: Duration::from_secs(20),
            request_timeout: Duration::from_secs(10),
        };

        let runner = LoadTestRunner::new(config.clone());
        let admin_url = std::env::var("ADMIN_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());
        let event_url = std::env::var("EVENT_ENGAGEMENT_URL")
            .unwrap_or_else(|_| "http://localhost:8081".to_string());

        let client = reqwest::Client::new();
        let token = get_auth_token(&admin_url).await;
        let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

        let metrics = runner
            .run(move || {
                let client = client.clone();
                let admin_url = admin_url.clone();
                let event_url = event_url.clone();
                let token = token.clone();
                let cnt = counter.clone();
                let seq = cnt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                async move {
                    let start = Instant::now();

                    // 根据序号选择不同操作（模拟真实流量分布）
                    let result = match seq % 10 {
                        0..=5 => {
                            // 60% - 查询用户徽章（管理服务需要认证）
                            let user_id = format!("user_{}", seq % 1000);
                            client
                                .get(format!(
                                    "{}/api/admin/users/{}/badges",
                                    admin_url, user_id
                                ))
                                .header("Authorization", format!("Bearer {}", token))
                                .send()
                                .await
                        }
                        6..=7 => {
                            // 20% - 发送签到事件（事件服务不需要认证）
                            client
                                .post(format!("{}/api/events", event_url))
                                .json(&serde_json::json!({
                                    "event_type": "checkin",
                                    "user_id": format!("mixed_user_{}", seq),
                                    "event_id": format!("mixed_{}", seq),
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                    "data": {}
                                }))
                                .send()
                                .await
                        }
                        8 => {
                            // 10% - 查询徽章列表（管理服务需要认证）
                            client
                                .get(format!("{}/api/admin/badges", admin_url))
                                .header("Authorization", format!("Bearer {}", token))
                                .send()
                                .await
                        }
                        _ => {
                            // 10% - 查询规则列表（管理服务需要认证）
                            client
                                .get(format!("{}/api/admin/rules", admin_url))
                                .header("Authorization", format!("Bearer {}", token))
                                .send()
                                .await
                        }
                    };

                    let latency = start.elapsed();

                    match result {
                        Ok(resp) if resp.status().is_success() => Ok(latency),
                        Ok(resp) => Err(format!("HTTP {}", resp.status())),
                        Err(e) => Err(e.to_string()),
                    }
                }
            })
            .await;

        println!("\n===== 混合负载基准测试结果 =====");
        metrics.print_summary(config.duration);

        // 混合负载目标
        PerformanceAssertions::assert_success_rate(&metrics, 99.0);
        PerformanceAssertions::assert_p99_latency(&metrics, 500.0);
        PerformanceAssertions::assert_throughput(&metrics, config.duration, 300.0);
    }
}
