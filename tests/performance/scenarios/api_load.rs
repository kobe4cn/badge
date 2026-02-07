//! API 负载测试场景
//!
//! 测试 REST API 在高并发下的表现。

use super::super::{LoadTestConfig, LoadTestRunner, PerformanceAssertions};
use std::time::{Duration, Instant};

#[cfg(test)]
mod api_load_tests {
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

    /// 徽章查询 API 负载测试
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_badge_query_load() {
        let config = LoadTestConfig {
            concurrent_users: 100,
            duration: Duration::from_secs(60),
            requests_per_second: Some(1000),
            warmup_duration: Duration::from_secs(5),
            request_timeout: Duration::from_secs(10),
        };

        let runner = LoadTestRunner::new(config.clone());
        let client = reqwest::Client::new();
        let base_url =
            std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

        let token = get_auth_token(&base_url).await;

        let metrics = runner
            .run(move || {
                let client = client.clone();
                let url = format!("{}/api/admin/badges", base_url);
                let token = token.clone();
                async move {
                    let start = Instant::now();
                    let response = client
                        .get(&url)
                        .header("Authorization", format!("Bearer {}", token))
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

        // 断言: 成功率 >= 99.9%, P99 <= 200ms, 吞吐量 >= 500 req/s
        PerformanceAssertions::assert_success_rate(&metrics, 99.9);
        PerformanceAssertions::assert_p99_latency(&metrics, 200.0);
        PerformanceAssertions::assert_throughput(&metrics, config.duration, 500.0);
    }

    /// 规则配置 API 负载测试
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_rule_config_load() {
        let config = LoadTestConfig {
            concurrent_users: 50,
            duration: Duration::from_secs(30),
            requests_per_second: Some(200),
            warmup_duration: Duration::from_secs(5),
            request_timeout: Duration::from_secs(10),
        };

        let runner = LoadTestRunner::new(config.clone());
        let client = reqwest::Client::new();
        let base_url =
            std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

        let token = get_auth_token(&base_url).await;

        let metrics = runner
            .run(move || {
                let client = client.clone();
                let url = format!("{}/api/admin/rules", base_url);
                let token = token.clone();
                async move {
                    let start = Instant::now();
                    let response = client
                        .get(&url)
                        .header("Authorization", format!("Bearer {}", token))
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

        PerformanceAssertions::assert_success_rate(&metrics, 99.5);
        PerformanceAssertions::assert_p99_latency(&metrics, 300.0);
    }

    /// 用户徽章查询 API 负载测试
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_user_badges_query_load() {
        let config = LoadTestConfig {
            concurrent_users: 200,
            duration: Duration::from_secs(60),
            requests_per_second: Some(2000),
            warmup_duration: Duration::from_secs(10),
            request_timeout: Duration::from_secs(5),
        };

        let runner = LoadTestRunner::new(config.clone());
        let client = reqwest::Client::new();
        let base_url =
            std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

        let token = get_auth_token(&base_url).await;

        let metrics = runner
            .run(move || {
                let client = client.clone();
                // 使用随机用户 ID 模拟真实场景
                let user_id = format!("user_{}", rand::random::<u32>() % 10000);
                let url = format!("{}/api/admin/users/{}/badges", base_url, user_id);
                let token = token.clone();
                async move {
                    let start = Instant::now();
                    let response = client
                        .get(&url)
                        .header("Authorization", format!("Bearer {}", token))
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

        // 用户徽章查询是热点接口，要求更高
        PerformanceAssertions::assert_success_rate(&metrics, 99.99);
        PerformanceAssertions::assert_p99_latency(&metrics, 100.0);
        PerformanceAssertions::assert_throughput(&metrics, config.duration, 1000.0);
    }
}
