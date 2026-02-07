//! 并发发放测试场景
//!
//! 测试徽章发放在高并发下的一致性和性能。

use super::super::{LoadTestConfig, LoadTestRunner, PerformanceAssertions};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[cfg(test)]
mod concurrent_grant_tests {
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

    /// 并发发放同一徽章测试 - 验证幂等性
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_concurrent_grant_idempotency() {
        let config = LoadTestConfig {
            concurrent_users: 50,
            duration: Duration::from_secs(30),
            requests_per_second: Some(100),
            warmup_duration: Duration::from_secs(5),
            request_timeout: Duration::from_secs(10),
        };

        let runner = LoadTestRunner::new(config.clone());
        let client = reqwest::Client::new();
        let base_url =
            std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

        let token = get_auth_token(&base_url).await;

        // 固定用户和徽章，测试幂等性
        let user_id = "test_user_idempotency";
        let badge_id = 1;

        let metrics = runner
            .run(move || {
                let client = client.clone();
                let url = format!("{}/api/admin/grants/manual", base_url);
                let token = token.clone();
                let user_id = user_id.to_string();
                async move {
                    let start = Instant::now();
                    let response = client
                        .post(&url)
                        .header("Authorization", format!("Bearer {}", token))
                        .json(&serde_json::json!({
                            "userId": user_id,
                            "badgeId": badge_id,
                            "sourceType": "test",
                            "sourceId": "perf_test"
                        }))
                        .send()
                        .await;
                    let latency = start.elapsed();

                    match response {
                        // 200 或 409(已存在) 都算成功
                        Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 409 => {
                            Ok(latency)
                        }
                        Ok(resp) => Err(format!("HTTP {}", resp.status())),
                        Err(e) => Err(e.to_string()),
                    }
                }
            })
            .await;

        // 幂等操作应该全部成功
        PerformanceAssertions::assert_success_rate(&metrics, 100.0);
    }

    /// 并发发放不同用户测试 - 验证吞吐量
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_concurrent_grant_throughput() {
        let config = LoadTestConfig {
            concurrent_users: 100,
            duration: Duration::from_secs(60),
            requests_per_second: Some(500),
            warmup_duration: Duration::from_secs(10),
            request_timeout: Duration::from_secs(10),
        };

        let runner = LoadTestRunner::new(config.clone());
        let client = reqwest::Client::new();
        let base_url =
            std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

        let token = get_auth_token(&base_url).await;
        let user_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let badge_id = 1;

        let metrics = runner
            .run(move || {
                let client = client.clone();
                let counter = user_counter.clone();
                let user_id = format!(
                    "perf_user_{}",
                    counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                );
                let url = format!("{}/api/admin/grants/manual", base_url);
                let token = token.clone();
                async move {
                    let start = Instant::now();
                    let response = client
                        .post(&url)
                        .header("Authorization", format!("Bearer {}", token))
                        .json(&serde_json::json!({
                            "userId": user_id,
                            "badgeId": badge_id,
                            "sourceType": "test",
                            "sourceId": "perf_test"
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

        // 徽章发放吞吐量目标: >= 200 req/s
        PerformanceAssertions::assert_success_rate(&metrics, 99.0);
        PerformanceAssertions::assert_throughput(&metrics, config.duration, 200.0);
    }

    /// 竞争兑换测试 - 验证库存无超卖
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_competitive_redemption_no_oversell() {
        let stock_limit = 100;
        let concurrent_requests = 500;

        let client = reqwest::Client::new();
        let base_url =
            std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

        let token = get_auth_token(&base_url).await;
        let successful_redemptions = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let redeemed_users = Arc::new(Mutex::new(HashSet::<String>::new()));

        let mut handles = Vec::new();

        for i in 0..concurrent_requests {
            let client = client.clone();
            let base_url = base_url.clone();
            let token = token.clone();
            let successful = successful_redemptions.clone();
            let users = redeemed_users.clone();
            let user_id = format!("redeem_user_{}", i);

            let handle = tokio::spawn(async move {
                let url = format!("{}/api/admin/redemption/redeem", base_url);
                let response = client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", token))
                    .json(&serde_json::json!({
                        "user_id": user_id,
                        "rule_id": 1,
                        "badge_ids": [1, 2, 3]
                    }))
                    .send()
                    .await;

                if let Ok(resp) = response {
                    if resp.status().is_success() {
                        successful.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        let mut guard = users.lock().await;
                        guard.insert(user_id);
                    }
                }
            });

            handles.push(handle);
        }

        // 等待所有请求完成
        for handle in handles {
            let _ = handle.await;
        }

        let total_successful = successful_redemptions.load(std::sync::atomic::Ordering::SeqCst);
        let unique_users = redeemed_users.lock().await.len();

        println!("成功兑换数: {}", total_successful);
        println!("唯一用户数: {}", unique_users);
        println!("库存限制: {}", stock_limit);

        // 验证无超卖
        assert!(
            total_successful <= stock_limit as u64,
            "发生超卖: {} > {}",
            total_successful,
            stock_limit
        );

        // 验证每个用户只兑换一次
        assert_eq!(total_successful as usize, unique_users, "存在重复兑换");
    }
}
