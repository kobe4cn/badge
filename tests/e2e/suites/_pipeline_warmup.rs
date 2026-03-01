//! 管道预热测试
//!
//! 通过下划线前缀确保此模块在所有测试套件中最先执行（按字母序排列）。
//! 发送真实事件并验证徽章发放，迫使 Kafka 消费者完成分区分配、规则加载
//! 和 gRPC 连接建立，避免后续测试因冷启动延迟而超时。
//!
//! 此模块同时预热 transaction 和 engagement 两条管道。

use crate::data::*;
use crate::helpers::*;
use crate::setup::TestEnvironment;
use std::time::Duration;

#[cfg(test)]
mod warmup_tests {
    use super::*;

    /// 预热交易事件管道（event-transaction-service）
    ///
    /// 创建最简规则，发送购买事件并验证徽章发放。
    /// 使用多轮重试和详细诊断，确保在 CI 环境下可靠通过。
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_0_warmup_transaction_pipeline() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 创建最简单的消费场景
        let category = env
            .api
            .create_category(&TestCategories::consumption())
            .await
            .unwrap();
        let series = env
            .api
            .create_series(&TestSeries::spending(category.id))
            .await
            .unwrap();
        let badge = env
            .api
            .create_badge(
                &CreateBadgeRequest::new(series.id, "TestWarmupBadge", "NORMAL")
                    .with_description("管道预热用临时徽章"),
            )
            .await
            .unwrap();

        let rule = env
            .api
            .create_rule(&CreateRuleRequest {
                badge_id: badge.id,
                rule_code: format!("warmup_purchase_{}", badge.id),
                name: "Warmup购买规则".to_string(),
                event_type: "purchase".to_string(),
                rule_json: serde_json::json!({
                    "type": "condition",
                    "field": "amount",
                    "operator": "gte",
                    "value": 1
                }),
                start_time: None,
                end_time: None,
                max_count_per_user: None,
                global_quota: None,
            })
            .await
            .unwrap();

        env.api.publish_rule(rule.id).await.unwrap();
        env.api
            .update_badge_status(badge.id, "active")
            .await
            .unwrap();

        // 诊断：验证规则在 DB 中存在且状态正确
        let db_rule = env.db.get_rule(rule.id).await.unwrap();
        assert!(
            db_rule.is_some(),
            "规则 {} 应存在于数据库",
            rule.id
        );
        let db_rule = db_rule.unwrap();
        assert!(db_rule.enabled, "规则 {} 应为启用状态", rule.id);

        // 同时向两个服务组发送规则刷新，确保 transaction 服务收到
        env.kafka
            .send_rule_reload_for_group("transaction")
            .await
            .unwrap();
        env.kafka.send_rule_reload().await.unwrap();
        env.wait_for_rule_reload().await.unwrap();

        // 多轮尝试：CI 环境下消费者可能在首次事件时仍未就绪
        let user_id = UserGenerator::user_id();
        let mut granted = false;

        for attempt in 0..3 {
            let event = TransactionEvent::purchase(
                &user_id,
                &OrderGenerator::order_id(),
                100 + attempt * 10,
            );
            env.kafka.send_transaction_event(event).await.unwrap();

            let timeout = if attempt == 0 {
                Duration::from_secs(20)
            } else {
                Duration::from_secs(15)
            };

            match env.wait_for_badge(&user_id, badge.id, timeout).await {
                Ok(()) => {
                    tracing::info!(
                        attempt,
                        "交易管道预热成功：徽章 {} 已发放给用户 {}",
                        badge.id,
                        user_id
                    );
                    granted = true;
                    break;
                }
                Err(e) => {
                    tracing::warn!(
                        attempt,
                        badge_id = badge.id,
                        user_id = %user_id,
                        error = %e,
                        "交易管道预热尝试 {}/3 失败，将重试",
                        attempt + 1,
                    );
                    // 重试前再次触发规则刷新
                    let _ = env.kafka.send_rule_reload_for_group("transaction").await;
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            }
        }

        if !granted {
            // 最终失败时输出详细诊断
            let rule_count = env
                .db
                .count_enabled_rules("purchase")
                .await
                .unwrap_or(-1);
            let badge_record = env.db.get_badge(badge.id).await.unwrap();
            tracing::error!(
                badge_id = badge.id,
                user_id = %user_id,
                enabled_purchase_rules = rule_count,
                badge_exists = badge_record.is_some(),
                badge_status = badge_record.as_ref().map(|b| b.status.as_str()).unwrap_or("N/A"),
                "交易管道预热最终失败"
            );
        }

        env.cleanup().await.unwrap();
        assert!(granted, "交易管道预热失败：3 次尝试均未能在超时内发放徽章");
    }

    /// 预热行为事件管道（event-engagement-service）
    ///
    /// 使用签到事件预热 engagement 管道，确保其 Kafka 消费者就绪。
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_1_warmup_engagement_pipeline() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        let category = env
            .api
            .create_category(&TestCategories::achievement())
            .await
            .unwrap();
        let series = env
            .api
            .create_series(&TestSeries::checkin(category.id))
            .await
            .unwrap();
        let badge = env
            .api
            .create_badge(
                &CreateBadgeRequest::new(series.id, "TestWarmupCheckin", "NORMAL")
                    .with_description("Engagement 管道预热用临时徽章"),
            )
            .await
            .unwrap();

        let rule = env
            .api
            .create_rule(&CreateRuleRequest {
                badge_id: badge.id,
                rule_code: format!("warmup_checkin_{}", badge.id),
                name: "Warmup签到规则".to_string(),
                event_type: "checkin".to_string(),
                rule_json: serde_json::json!({
                    "type": "condition",
                    "field": "consecutive_days",
                    "operator": "gte",
                    "value": 1
                }),
                start_time: None,
                end_time: None,
                max_count_per_user: None,
                global_quota: None,
            })
            .await
            .unwrap();

        env.api.publish_rule(rule.id).await.unwrap();
        env.api
            .update_badge_status(badge.id, "active")
            .await
            .unwrap();

        // 诊断：验证规则在 DB 中存在
        let db_rule = env.db.get_rule(rule.id).await.unwrap();
        assert!(db_rule.is_some(), "规则应存在于数据库");
        assert!(db_rule.unwrap().enabled, "规则应为启用状态");

        env.kafka
            .send_rule_reload_for_group("engagement")
            .await
            .unwrap();
        env.kafka.send_rule_reload().await.unwrap();
        env.wait_for_rule_reload().await.unwrap();

        let user_id = UserGenerator::user_id();
        let mut granted = false;

        for attempt in 0..3 {
            let event = EngagementEvent::checkin(&user_id);
            env.kafka.send_engagement_event(event).await.unwrap();

            let timeout = if attempt == 0 {
                Duration::from_secs(20)
            } else {
                Duration::from_secs(15)
            };

            match env.wait_for_badge(&user_id, badge.id, timeout).await {
                Ok(()) => {
                    tracing::info!(
                        attempt,
                        "Engagement 管道预热成功：徽章 {} 已发放",
                        badge.id
                    );
                    granted = true;
                    break;
                }
                Err(e) => {
                    tracing::warn!(
                        attempt,
                        badge_id = badge.id,
                        user_id = %user_id,
                        error = %e,
                        "Engagement 管道预热尝试 {}/3 失败，将重试",
                        attempt + 1,
                    );
                    let _ = env.kafka.send_rule_reload_for_group("engagement").await;
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            }
        }

        if !granted {
            let rule_count = env
                .db
                .count_enabled_rules("checkin")
                .await
                .unwrap_or(-1);
            tracing::error!(
                badge_id = badge.id,
                user_id = %user_id,
                enabled_checkin_rules = rule_count,
                "Engagement 管道预热最终失败"
            );
        }

        env.cleanup().await.unwrap();
        assert!(granted, "Engagement 管道预热失败：3 次尝试均未能在超时内发放徽章");
    }
}
