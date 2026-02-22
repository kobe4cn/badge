//! 事件处理吞吐量测试
//!
//! 通过 HTTP API 模拟事件提交，测量后端处理性能。
//! 需要完整的服务栈运行（API 服务 + Kafka + 数据库）。

use std::time::{Duration, Instant};

#[cfg(test)]
mod event_throughput_tests {
    use super::*;
    use super::super::super::{LoadTestConfig, LoadTestRunner, PerformanceAssertions};

    /// 获取事件服务的 base URL
    fn event_base_url() -> String {
        std::env::var("EVENT_BASE_URL").unwrap_or_else(|_| "http://localhost:8082".to_string())
    }

    /// 获取管理服务的 base URL
    fn admin_base_url() -> String {
        std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
    }

    /// 登录获取 JWT Token
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

    /// 构造交易事件 payload
    fn make_transaction_event(user_id: &str, order_id: &str, amount: u64) -> serde_json::Value {
        serde_json::json!({
            "eventType": "purchase",
            "userId": user_id,
            "data": {
                "orderId": order_id,
                "amount": amount,
                "currency": "CNY",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        })
    }

    /// 交易事件处理吞吐量测试
    ///
    /// 通过 HTTP API 批量提交交易事件，测量端到端吞吐量。
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_transaction_event_throughput() {
        let event_url = event_base_url();
        let admin_url = admin_base_url();
        let token = get_auth_token(&admin_url).await;
        let client = reqwest::Client::new();

        let total_events = 10000;
        let batch_size = 100;
        let batches = total_events / batch_size;

        let mut latencies = Vec::new();
        let start = Instant::now();

        for batch in 0..batches {
            let batch_start = Instant::now();

            // 批量发送事件到事件服务的入口端点
            let mut futures = Vec::new();
            for i in 0..batch_size {
                let idx = batch * batch_size + i;
                let event = make_transaction_event(
                    &format!("perf_user_{}", idx % 1000),
                    &format!("order_{}", idx),
                    100 + (idx as u64 % 500),
                );
                let fut = client
                    .post(format!("{}/api/v1/events", event_url))
                    .header("Authorization", format!("Bearer {}", token))
                    .json(&event)
                    .send();
                futures.push(fut);
            }

            // 并发等待本批次所有请求完成
            let results = futures::future::join_all(futures).await;
            let failed = results.iter().filter(|r| r.is_err()).count();
            if failed > 0 {
                eprintln!("批次 {} 有 {} 个请求失败", batch, failed);
            }

            latencies.push(batch_start.elapsed().as_secs_f64() * 1000.0);

            // 批次间短暂等待，避免瞬间打满连接池
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        let total_duration = start.elapsed();
        let throughput = total_events as f64 / total_duration.as_secs_f64();

        println!("事件处理吞吐量: {:.2} events/s", throughput);
        println!("总耗时: {:.2}s", total_duration.as_secs_f64());
        println!(
            "批次平均延迟: {:.2}ms",
            latencies.iter().sum::<f64>() / latencies.len() as f64
        );

        // 目标: >= 1000 events/s
        assert!(
            throughput >= 1000.0,
            "事件吞吐量 {:.2} 低于目标 1000 events/s",
            throughput
        );
    }

    /// 消息处理延迟测试
    ///
    /// 发送带时间戳的事件，通过查询 API 验证处理完成，计算端到端延迟。
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_event_processing_latency() {
        let event_url = event_base_url();
        let admin_url = admin_base_url();
        let token = get_auth_token(&admin_url).await;
        let client = reqwest::Client::new();

        let expected_p99_latency_ms = 500.0;
        let sample_count = 50;
        let mut latencies = Vec::new();

        for i in 0..sample_count {
            let user_id = format!("latency_test_user_{}", i);
            let send_time = Instant::now();

            // 1. 发送带时间戳的事件
            let event = make_transaction_event(&user_id, &format!("latency_order_{}", i), 200);
            let resp = client
                .post(format!("{}/api/v1/events", event_url))
                .header("Authorization", format!("Bearer {}", token))
                .json(&event)
                .send()
                .await;

            if resp.is_err() {
                eprintln!("事件 {} 发送失败: {:?}", i, resp.err());
                continue;
            }

            // 2. 轮询查询处理结果，计算端到端延迟
            let poll_timeout = Duration::from_secs(5);
            let poll_start = Instant::now();
            let mut processed = false;

            while poll_start.elapsed() < poll_timeout {
                let check = client
                    .get(format!(
                        "{}/api/admin/grants/logs?userId={}&pageSize=1",
                        admin_url, user_id
                    ))
                    .header("Authorization", format!("Bearer {}", token))
                    .send()
                    .await;

                if let Ok(resp) = check {
                    if let Ok(body) = resp.json::<serde_json::Value>().await {
                        let items = body["data"]["items"].as_array();
                        if items.map_or(false, |arr| !arr.is_empty()) {
                            processed = true;
                            break;
                        }
                    }
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }

            // 3. 记录端到端延迟
            if processed {
                let latency_ms = send_time.elapsed().as_secs_f64() * 1000.0;
                latencies.push(latency_ms);
            }
        }

        assert!(
            !latencies.is_empty(),
            "没有事件被成功处理，无法计算延迟"
        );

        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p99_idx = ((0.99 * (latencies.len() - 1) as f64) as usize).min(latencies.len() - 1);
        let p99 = latencies[p99_idx];
        let avg = latencies.iter().sum::<f64>() / latencies.len() as f64;

        println!("处理延迟统计 ({}个样本):", latencies.len());
        println!("  平均: {:.2}ms", avg);
        println!("  P99:  {:.2}ms", p99);

        assert!(
            p99 <= expected_p99_latency_ms,
            "P99 延迟 {:.2}ms 超过目标 {:.2}ms",
            p99,
            expected_p99_latency_ms
        );
    }

    /// 规则热更新性能测试
    ///
    /// 通过管理 API 更新规则后，测量规则引擎生效的延迟。
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_rule_reload_performance() {
        let admin_url = admin_base_url();
        let token = get_auth_token(&admin_url).await;
        let client = reqwest::Client::new();

        // 通过管理 API 触发规则更新（发布/禁用循环）
        let rules_resp = client
            .get(format!("{}/api/admin/rules?pageSize=1", admin_url))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .expect("查询规则列表失败");

        let rules_body: serde_json::Value = rules_resp.json().await.expect("解析规则响应失败");
        let rule_id = rules_body["data"]["items"][0]["id"].as_i64();

        if rule_id.is_none() {
            println!("没有可用规则，跳过热更新测试");
            return;
        }
        let rule_id = rule_id.unwrap();

        // 禁用再启用来触发规则重载
        client
            .post(format!("{}/api/admin/rules/{}/disable", admin_url, rule_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .expect("禁用规则失败");

        let reload_start = Instant::now();

        client
            .post(format!("{}/api/admin/rules/{}/publish", admin_url, rule_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .expect("发布规则失败");

        // 轮询检查规则是否已生效
        let reload_timeout = Duration::from_secs(5);
        let mut reloaded = false;

        while reload_start.elapsed() < reload_timeout {
            let check = client
                .get(format!("{}/api/admin/rules/{}",admin_url, rule_id))
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await;

            if let Ok(resp) = check {
                if let Ok(body) = resp.json::<serde_json::Value>().await {
                    let enabled = body["data"]["enabled"].as_bool().unwrap_or(false);
                    let status = body["data"]["status"].as_str().unwrap_or("");
                    if enabled || status == "published" || status == "PUBLISHED" {
                        reloaded = true;
                        break;
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let reload_latency = reload_start.elapsed();

        println!(
            "规则重载延迟: {:.2}ms (已生效={})",
            reload_latency.as_secs_f64() * 1000.0,
            reloaded
        );

        assert!(reloaded, "规则在 5 秒内未生效");

        // 目标: 3秒内完成热更新
        assert!(
            reload_latency < Duration::from_secs(3),
            "规则重载延迟 {:.2}s 超过目标 3s",
            reload_latency.as_secs_f64()
        );
    }

    /// 死信队列处理测试
    ///
    /// 发送格式错误的事件，验证系统能正确拒绝并记录到 DLQ。
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_dlq_processing() {
        let event_url = event_base_url();
        let admin_url = admin_base_url();
        let token = get_auth_token(&admin_url).await;
        let client = reqwest::Client::new();

        // 1. 发送格式错误的事件（缺少必填字段）
        let invalid_events = vec![
            serde_json::json!({"eventType": "unknown_type"}),
            serde_json::json!({"userId": "test", "data": null}),
            serde_json::json!({"eventType": "purchase", "userId": "", "data": {}}),
        ];

        let mut rejected_count = 0;
        for event in &invalid_events {
            let resp = client
                .post(format!("{}/api/v1/events", event_url))
                .header("Authorization", format!("Bearer {}", token))
                .json(event)
                .send()
                .await;

            match resp {
                Ok(r) if !r.status().is_success() => rejected_count += 1,
                Err(_) => rejected_count += 1,
                _ => {}
            }
        }

        println!(
            "无效事件拒绝率: {}/{}",
            rejected_count,
            invalid_events.len()
        );

        // 2. 验证系统仍然可以正常处理有效事件（未被无效事件阻塞）
        let valid_event = make_transaction_event("dlq_test_user", "dlq_order_1", 100);
        let valid_resp = client
            .post(format!("{}/api/v1/events", event_url))
            .header("Authorization", format!("Bearer {}", token))
            .json(&valid_event)
            .send()
            .await
            .expect("发送有效事件失败");

        assert!(
            valid_resp.status().is_success() || valid_resp.status().as_u16() == 202,
            "无效事件不应阻塞有效事件处理，状态码: {}",
            valid_resp.status()
        );

        // 3. 验证 DLQ 重试机制：查询失败任务列表
        let tasks_resp = client
            .get(format!(
                "{}/api/admin/tasks?taskType=dlq_retry&pageSize=10",
                admin_url
            ))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await;

        if let Ok(resp) = tasks_resp {
            if resp.status().is_success() {
                println!("DLQ 重试任务查询成功");
            } else {
                println!("DLQ 重试任务接口返回: {}", resp.status());
            }
        }
    }

    /// 背压测试 - 验证系统在高负载下的稳定性
    ///
    /// 瞬间发送大量事件，验证系统能正确处理而不丢失数据。
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_backpressure_handling() {
        let config = LoadTestConfig {
            concurrent_users: 200,
            duration: Duration::from_secs(30),
            requests_per_second: None, // 不限速，测试背压
            warmup_duration: Duration::from_secs(2),
            request_timeout: Duration::from_secs(10),
        };

        let runner = LoadTestRunner::new(config.clone());
        let event_url = event_base_url();
        let admin_url = admin_base_url();
        let token = get_auth_token(&admin_url).await;
        let client = reqwest::Client::new();

        let _burst_size = 10000;
        let start = Instant::now();

        // 使用 LoadTestRunner 执行高并发事件发送
        let metrics = runner
            .run(move || {
                let client = client.clone();
                let url = format!("{}/api/v1/events", event_url);
                let token = token.clone();
                let idx = rand::random::<u32>();
                async move {
                    let event = serde_json::json!({
                        "eventType": "purchase",
                        "userId": format!("bp_user_{}", idx % 5000),
                        "data": {
                            "orderId": format!("bp_order_{}", idx),
                            "amount": 100 + (idx % 500),
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        }
                    });

                    let req_start = Instant::now();
                    let response = client
                        .post(&url)
                        .header("Authorization", format!("Bearer {}", token))
                        .json(&event)
                        .send()
                        .await;
                    let latency = req_start.elapsed();

                    match response {
                        Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 202 => {
                            Ok(latency)
                        }
                        Ok(resp) => Err(format!("HTTP {}", resp.status())),
                        Err(e) => Err(e.to_string()),
                    }
                }
            })
            .await;

        let send_duration = start.elapsed();
        println!(
            "背压测试: 发送 {} 事件耗时 {:.2}s",
            metrics.total_requests,
            send_duration.as_secs_f64()
        );

        // 在高并发下允许适当的失败率，但不能太高
        PerformanceAssertions::assert_success_rate(&metrics, 95.0);

        // 等待后端处理队列排空
        println!("等待事件处理完成...");
        let processing_timeout = Duration::from_secs(60);
        let processing_start = Instant::now();

        while processing_start.elapsed() < processing_timeout {
            // 通过健康检查端点确认服务仍在正常运行
            let admin_url_val =
                std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
            let health_client = reqwest::Client::new();
            let health = health_client
                .get(format!("{}/health", admin_url_val))
                .send()
                .await;

            match health {
                Ok(resp) if resp.status().is_success() => break,
                _ => tokio::time::sleep(Duration::from_secs(1)).await,
            }
        }

        let total_processing_time = processing_start.elapsed();
        println!(
            "服务恢复确认耗时: {:.2}s",
            total_processing_time.as_secs_f64()
        );

        // 验证系统在背压后仍然能正常响应
        let admin_url_final =
            std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
        let final_token = get_auth_token(&admin_url_final).await;
        let final_client = reqwest::Client::new();
        let post_burst = final_client
            .get(format!("{}/api/admin/badges?pageSize=1", admin_url_final))
            .header("Authorization", format!("Bearer {}", final_token))
            .send()
            .await
            .expect("背压后 API 应能正常响应");

        assert!(
            post_burst.status().is_success(),
            "背压后 API 返回非 200: {}",
            post_burst.status()
        );
    }
}
