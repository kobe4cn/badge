//! 事件触发测试套件
//!
//! 测试从 Kafka 事件到徽章发放的完整链路。

use crate::data::*;

use crate::helpers::*;
use crate::setup::TestEnvironment;
use std::time::Duration;

#[cfg(test)]
mod purchase_event_tests {
    use super::*;

    /// 购买触发徽章发放测试
    ///
    /// 验证用户完成购买后，满足规则条件时能正确获得徽章。
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_purchase_trigger_badge_grant() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 构建消费升级场景，包含 500/1000/5000 三档消费徽章
        let scenario = ScenarioBuilder::new(&env.api)
            .spending_upgrade()
            .await
            .unwrap();

        // 触发规则引擎重新加载规则
        env.kafka.send_rule_reload().await.unwrap();
        env.wait_for_rule_reload().await.unwrap();

        // 发送购买事件 (600元，应触发 500 元徽章)
        let user_id = UserGenerator::user_id();
        let event = TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 600);
        env.kafka.send_transaction_event(event).await.unwrap();

        // 等待异步处理完成
        env.wait_for_processing(Duration::from_secs(5))
            .await
            .unwrap();

        // 验证徽章已发放
        let has_badge = env
            .db
            .user_has_badge(&user_id, scenario.badge_500.id)
            .await
            .unwrap();
        assert!(has_badge, "用户应该获得 500 元徽章");

        // 验证账本记录正确
        let ledger = env
            .db
            .get_badge_ledger(scenario.badge_500.id, &user_id)
            .await
            .unwrap();
        assert!(!ledger.is_empty(), "应该有账本记录");
        assert_eq!(ledger[0].delta, 1);
        assert_eq!(ledger[0].action, "grant");

        env.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_cumulative_spending_upgrade() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        let scenario = ScenarioBuilder::new(&env.api)
            .spending_upgrade()
            .await
            .unwrap();

        env.kafka.send_rule_reload().await.unwrap();
        env.wait_for_rule_reload().await.unwrap();

        let user_id = UserGenerator::user_id();

        // 第一笔 600 元
        let event1 = TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 600);
        env.kafka.send_transaction_event(event1).await.unwrap();
        env.wait_for_processing(Duration::from_secs(3))
            .await
            .unwrap();

        // 应该获得 500 元徽章
        assert!(
            env.db
                .user_has_badge(&user_id, scenario.badge_500.id)
                .await
                .unwrap()
        );
        assert!(
            !env.db
                .user_has_badge(&user_id, scenario.badge_1000.id)
                .await
                .unwrap()
        );

        // 第二笔 500 元 (累计 1100 元)
        let event2 = TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 500);
        env.kafka.send_transaction_event(event2).await.unwrap();
        env.wait_for_processing(Duration::from_secs(3))
            .await
            .unwrap();

        // 应该获得 1000 元徽章
        assert!(
            env.db
                .user_has_badge(&user_id, scenario.badge_1000.id)
                .await
                .unwrap()
        );
        assert!(
            !env.db
                .user_has_badge(&user_id, scenario.badge_5000.id)
                .await
                .unwrap()
        );

        env.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_single_order_amount_rule() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 创建单笔消费规则
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
            .create_badge(&CreateBadgeRequest {
                series_id: series.id,
                name: "Test大额订单".to_string(),
                description: Some("单笔消费满 500 元".to_string()),
                badge_type: "normal".to_string(),
                icon_url: None,
                max_supply: None,
            })
            .await
            .unwrap();

        let _rule = env
            .api
            .create_rule(&TestRules::single_order(badge.id, 500))
            .await
            .unwrap();
        env.api
            .update_badge_status(badge.id, "active")
            .await
            .unwrap();

        env.kafka.send_rule_reload().await.unwrap();
        env.wait_for_rule_reload().await.unwrap();

        let user_id = UserGenerator::user_id();

        // 400 元订单不触发
        let event1 = TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 400);
        env.kafka.send_transaction_event(event1).await.unwrap();
        env.wait_for_processing(Duration::from_secs(3))
            .await
            .unwrap();
        assert!(!env.db.user_has_badge(&user_id, badge.id).await.unwrap());

        // 500 元订单触发
        let event2 = TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 500);
        env.kafka.send_transaction_event(event2).await.unwrap();
        env.wait_for_processing(Duration::from_secs(3))
            .await
            .unwrap();
        assert!(env.db.user_has_badge(&user_id, badge.id).await.unwrap());

        env.cleanup().await.unwrap();
    }
}

#[cfg(test)]
mod checkin_event_tests {
    use super::*;

    /// 签到触发徽章发放测试
    ///
    /// 验证连续签到达到指定天数后能正确获得签到徽章。
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_checkin_trigger_badge_grant() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 构建签到场景，包含连续签到 7 天徽章
        let scenario = ScenarioBuilder::new(&env.api).checkin().await.unwrap();

        env.kafka.send_rule_reload().await.unwrap();
        env.wait_for_rule_reload().await.unwrap();

        let user_id = UserGenerator::user_id();

        // 连续签到 6 天，未达到阈值不应触发
        let event6 = EngagementEvent {
            event_id: EventGenerator::event_id(),
            event_type: "checkin".to_string(),
            user_id: user_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            data: serde_json::json!({"consecutive_days": 6}),
        };
        env.kafka.send_engagement_event(event6).await.unwrap();
        env.wait_for_processing(Duration::from_secs(3))
            .await
            .unwrap();
        assert!(
            !env.db
                .user_has_badge(&user_id, scenario.badge_7days.id)
                .await
                .unwrap()
        );

        // 连续签到 7 天，达到阈值应触发徽章发放
        let event7 = EngagementEvent {
            event_id: EventGenerator::event_id(),
            event_type: "checkin".to_string(),
            user_id: user_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            data: serde_json::json!({"consecutive_days": 7}),
        };
        env.kafka.send_engagement_event(event7).await.unwrap();
        env.wait_for_processing(Duration::from_secs(3))
            .await
            .unwrap();
        assert!(
            env.db
                .user_has_badge(&user_id, scenario.badge_7days.id)
                .await
                .unwrap()
        );

        env.cleanup().await.unwrap();
    }
}

#[cfg(test)]
mod idempotency_tests {
    use super::*;

    /// 重复事件幂等性测试
    ///
    /// 验证相同 event_id 的事件多次发送时，徽章只发放一次。
    /// 幂等性是分布式系统的关键特性，防止网络重试导致重复发放。
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_event_idempotency() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        let scenario = ScenarioBuilder::new(&env.api)
            .spending_upgrade()
            .await
            .unwrap();

        env.kafka.send_rule_reload().await.unwrap();
        env.wait_for_rule_reload().await.unwrap();

        let user_id = UserGenerator::user_id();
        let event_id = EventGenerator::event_id();
        let order_id = OrderGenerator::order_id();

        // 使用相同 event_id 构建事件，模拟网络重试场景
        let event = TransactionEvent {
            event_id: event_id.clone(),
            event_type: "purchase".to_string(),
            user_id: user_id.clone(),
            order_id: order_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            data: serde_json::json!({"amount": 600}),
        };

        // 首次发送事件
        env.kafka
            .send_transaction_event(event.clone())
            .await
            .unwrap();
        env.wait_for_processing(Duration::from_secs(3))
            .await
            .unwrap();

        // 重复发送相同事件
        env.kafka.send_transaction_event(event).await.unwrap();
        env.wait_for_processing(Duration::from_secs(3))
            .await
            .unwrap();

        // 验证徽章只发放一次
        let quantity = env
            .db
            .get_user_badge_count(&user_id, scenario.badge_500.id)
            .await
            .unwrap();
        assert_eq!(quantity, 1, "重复事件不应该重复发放徽章");

        // 验证账本记录也只有一条
        let ledger = env
            .db
            .get_badge_ledger(scenario.badge_500.id, &user_id)
            .await
            .unwrap();
        assert_eq!(ledger.len(), 1, "账本应该只有一条发放记录");

        env.cleanup().await.unwrap();
    }
}

#[cfg(test)]
mod quota_tests {
    use super::*;

    /// 配额限制测试
    ///
    /// 验证用户配额和全局配额都能正确限制徽章发放。
    /// 配额机制是防止超发的关键保障。
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_quota_enforcement() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 测试用户配额限制
        {
            let scenario = ScenarioBuilder::new(&env.api)
                .spending_upgrade()
                .await
                .unwrap();

            env.kafka.send_rule_reload().await.unwrap();
            env.wait_for_rule_reload().await.unwrap();

            let user_id = UserGenerator::user_id();

            // 第一次购买，应获得徽章
            let event1 = TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 600);
            env.kafka.send_transaction_event(event1).await.unwrap();
            env.wait_for_processing(Duration::from_secs(3))
                .await
                .unwrap();
            assert!(
                env.db
                    .user_has_badge(&user_id, scenario.badge_500.id)
                    .await
                    .unwrap()
            );

            // 第二次购买，用户配额应阻止再次发放
            let event2 = TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 600);
            env.kafka.send_transaction_event(event2).await.unwrap();
            env.wait_for_processing(Duration::from_secs(3))
                .await
                .unwrap();

            // 验证徽章数量仍为 1
            let quantity = env
                .db
                .get_user_badge_count(&user_id, scenario.badge_500.id)
                .await
                .unwrap();
            assert_eq!(quantity, 1, "用户配额应阻止重复发放");
        }

        // 清理后测试全局配额限制
        env.prepare_test_data().await.unwrap();

        {
            // 创建限量徽章场景，全局配额为 3
            let scenario = ScenarioBuilder::new(&env.api)
                .limited_redemption(3)
                .await
                .unwrap();

            env.kafka.send_rule_reload().await.unwrap();
            env.wait_for_rule_reload().await.unwrap();

            // 5 个用户同时尝试获取
            let user_ids: Vec<String> = (0..5).map(|_| UserGenerator::user_id()).collect();

            for user_id in &user_ids {
                let event = TransactionEvent::purchase(user_id, &OrderGenerator::order_id(), 200);
                env.kafka.send_transaction_event(event).await.unwrap();
            }

            env.wait_for_processing(Duration::from_secs(5))
                .await
                .unwrap();

            // 验证只有 3 个用户获得徽章
            let mut granted_count = 0;
            for user_id in &user_ids {
                if env
                    .db
                    .user_has_badge(user_id, scenario.badge.id)
                    .await
                    .unwrap()
                {
                    granted_count += 1;
                }
            }

            assert_eq!(granted_count, 3, "全局配额应限制发放数量为 3");

            // 验证徽章的已发放数量
            let badge_stats = env.db.get_badge_stats(scenario.badge.id).await.unwrap();
            assert_eq!(badge_stats.granted_count, 3, "徽章统计应显示发放 3 个");
        }

        env.cleanup().await.unwrap();
    }
}

#[cfg(test)]
mod notification_tests {
    use super::*;

    /// 发放后通知测试
    ///
    /// 验证徽章发放成功后会向用户发送通知消息。
    /// 通知是用户感知徽章获取的重要途径。
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_notification_after_grant() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        let scenario = ScenarioBuilder::new(&env.api)
            .spending_upgrade()
            .await
            .unwrap();

        env.kafka.send_rule_reload().await.unwrap();
        env.wait_for_rule_reload().await.unwrap();

        // 清空通知队列，确保只捕获本次测试产生的通知
        env.kafka.drain_topic(topics::NOTIFICATIONS).await.unwrap();

        let user_id = UserGenerator::user_id();
        let event = TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 600);
        env.kafka.send_transaction_event(event).await.unwrap();

        env.wait_for_processing(Duration::from_secs(5))
            .await
            .unwrap();

        // 验证徽章已发放
        assert!(
            env.db
                .user_has_badge(&user_id, scenario.badge_500.id)
                .await
                .unwrap()
        );

        // 验证通知已发送
        let notifications = env.kafka.consume_notifications().await.unwrap();
        let badge_notification = notifications
            .iter()
            .find(|n| n.user_id == user_id && n.notification_type == "BADGE_GRANTED");

        assert!(badge_notification.is_some(), "应该发送徽章获取通知");

        // 验证通知内容包含徽章信息
        if let Some(notification) = badge_notification {
            assert!(!notification.title.is_empty(), "通知标题不应为空");
            assert!(!notification.body.is_empty(), "通知内容不应为空");
        }

        env.cleanup().await.unwrap();
    }
}
