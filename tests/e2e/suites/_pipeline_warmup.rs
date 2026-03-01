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
    /// 使用较长超时（30s）容纳首次分区分配和规则加载的冷启动开销。
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

        // 触发规则热加载并给予充足时间
        env.kafka.send_rule_reload().await.unwrap();
        env.wait_for_rule_reload().await.unwrap();

        // 发送购买事件
        let user_id = UserGenerator::user_id();
        let event = TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 100);
        env.kafka.send_transaction_event(event).await.unwrap();

        // 首次管道调用使用加倍超时（30s），容纳冷启动延迟
        let result = env
            .wait_for_badge(&user_id, badge.id, Duration::from_secs(30))
            .await;

        match &result {
            Ok(()) => {
                tracing::info!("交易管道预热成功：徽章 {} 已发放给用户 {}", badge.id, user_id);
            }
            Err(e) => {
                // 输出诊断信息帮助排查
                let rule_count = env
                    .db
                    .count_enabled_rules("purchase")
                    .await
                    .unwrap_or(-1);
                tracing::error!(
                    "交易管道预热失败：{}\n  badge_id={}, user_id={}, enabled_purchase_rules={}",
                    e,
                    badge.id,
                    user_id,
                    rule_count,
                );
            }
        }

        env.cleanup().await.unwrap();
        result.unwrap();
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

        env.kafka.send_rule_reload().await.unwrap();
        env.wait_for_rule_reload().await.unwrap();

        let user_id = UserGenerator::user_id();
        let event = EngagementEvent::checkin(&user_id);
        env.kafka.send_engagement_event(event).await.unwrap();

        let result = env
            .wait_for_badge(&user_id, badge.id, Duration::from_secs(30))
            .await;

        match &result {
            Ok(()) => {
                tracing::info!("Engagement 管道预热成功：徽章 {} 已发放", badge.id);
            }
            Err(e) => {
                let rule_count = env
                    .db
                    .count_enabled_rules("checkin")
                    .await
                    .unwrap_or(-1);
                tracing::error!(
                    "Engagement 管道预热失败：{}\n  badge_id={}, user_id={}, enabled_checkin_rules={}",
                    e,
                    badge.id,
                    user_id,
                    rule_count,
                );
            }
        }

        env.cleanup().await.unwrap();
        result.unwrap();
    }
}
