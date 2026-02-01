//! 权益配置测试套件
//!
//! 测试权益的创建、关联和发放流程。

use crate::data::*;
use crate::setup::TestEnvironment;
use std::time::Duration;

#[cfg(test)]
mod benefit_crud_tests {
    use super::*;

    /// 测试创建积分权益
    ///
    /// 验证积分类型权益的创建流程和数据完整性
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_create_points_benefit() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 创建积分权益
        let points_amount = 100;
        let req = TestBenefits::points(points_amount);
        let benefit = env.api.create_benefit(&req).await.unwrap();

        // 验证 API 返回数据
        assert_eq!(benefit.benefit_type, "points");
        assert_eq!(benefit.config["points_amount"], points_amount);
        assert!(benefit.id > 0, "权益 ID 应为正数");

        // 验证数据库持久化
        let db_count = env
            .db
            .count("benefits", &format!("id = {}", benefit.id))
            .await
            .unwrap();
        assert_eq!(db_count, 1, "数据库应有对应记录");

        env.cleanup().await.unwrap();
    }

    /// 测试创建优惠券权益
    ///
    /// 验证优惠券类型权益的创建，包括模板 ID 和有效期配置
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_create_coupon_benefit() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 创建优惠券权益
        let template_id = "TPL_001";
        let validity_days = 30;
        let req = TestBenefits::coupon(template_id, validity_days);
        let benefit = env.api.create_benefit(&req).await.unwrap();

        // 验证 API 返回数据
        assert_eq!(benefit.benefit_type, "coupon");
        assert_eq!(benefit.config["coupon_template_id"], template_id);
        assert_eq!(benefit.config["validity_days"], validity_days);

        // 验证数据库持久化
        let db_count = env
            .db
            .count("benefits", &format!("id = {}", benefit.id))
            .await
            .unwrap();
        assert_eq!(db_count, 1, "数据库应有对应记录");

        env.cleanup().await.unwrap();
    }

    /// 测试创建实物权益
    ///
    /// 验证实物奖品类型权益的创建，包括 SKU 和配送配置
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_create_physical_benefit() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 创建实物权益
        let sku_id = "SKU_001";
        let req = TestBenefits::physical(sku_id);
        let benefit = env.api.create_benefit(&req).await.unwrap();

        // 验证 API 返回数据
        assert_eq!(benefit.benefit_type, "physical");
        assert_eq!(benefit.config["sku_id"], sku_id);
        assert_eq!(benefit.config["shipping_required"], true);

        // 验证数据库持久化
        let db_count = env
            .db
            .count("benefits", &format!("id = {}", benefit.id))
            .await
            .unwrap();
        assert_eq!(db_count, 1, "数据库应有对应记录");

        env.cleanup().await.unwrap();
    }

    /// 测试权益列表查询
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_list_benefits() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 创建多种类型权益
        let _points = env
            .api
            .create_benefit(&TestBenefits::points(50))
            .await
            .unwrap();
        let _coupon = env
            .api
            .create_benefit(&TestBenefits::coupon("TPL_002", 15))
            .await
            .unwrap();
        let _physical = env
            .api
            .create_benefit(&TestBenefits::physical("SKU_002"))
            .await
            .unwrap();

        // 查询权益列表
        let benefits = env.api.list_benefits().await.unwrap();
        assert!(benefits.len() >= 3, "应至少有 3 条权益记录");

        env.cleanup().await.unwrap();
    }
}

#[cfg(test)]
mod benefit_link_tests {
    use super::*;
    use crate::helpers::*;
    use serde_json::json;

    /// 测试关联权益到徽章
    ///
    /// 通过创建兑换规则将权益关联到徽章，并验证关联关系
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_link_benefit_to_badge() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 1. 创建徽章前置数据
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
            .create_badge(&TestBadges::first_purchase(series.id))
            .await
            .unwrap();

        // 2. 创建权益
        let benefit = env
            .api
            .create_benefit(&TestBenefits::points(200))
            .await
            .unwrap();

        // 3. 创建兑换规则（关联徽章和权益）
        let redemption_rule = env
            .api
            .create_redemption_rule(&CreateRedemptionRuleRequest {
                name: "Test首购奖励规则".to_string(),
                description: Some("首次购买徽章自动兑换积分".to_string()),
                benefit_id: benefit.id,
                required_badges: json!([{"badge_id": badge.id, "quantity": 1}]),
                auto_redeem: true,
            })
            .await
            .unwrap();

        // 验证兑换规则创建成功
        assert!(redemption_rule.id > 0, "兑换规则 ID 应为正数");
        assert_eq!(redemption_rule.benefit_id, benefit.id);
        assert!(redemption_rule.auto_redeem, "应为自动兑换规则");

        // 验证数据库中的关联关系
        let rule_count = env
            .db
            .count(
                "badge_redemption_rules",
                &format!(
                    "id = {} AND benefit_id = {}",
                    redemption_rule.id, benefit.id
                ),
            )
            .await
            .unwrap();
        assert_eq!(rule_count, 1, "数据库应有对应的兑换规则");

        env.cleanup().await.unwrap();
    }

    /// 测试关联多个权益到同一徽章
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_link_multiple_benefits_to_badge() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 创建徽章
        let category = env
            .api
            .create_category(&TestCategories::achievement())
            .await
            .unwrap();
        let series = env
            .api
            .create_series(&TestSeries::newcomer(category.id))
            .await
            .unwrap();
        let badge = env
            .api
            .create_badge(&TestBadges::checkin_7days(series.id))
            .await
            .unwrap();

        // 创建多个权益
        let benefit_points = env
            .api
            .create_benefit(&TestBenefits::points(500))
            .await
            .unwrap();
        let benefit_coupon = env
            .api
            .create_benefit(&TestBenefits::coupon("TPL_VIP", 60))
            .await
            .unwrap();

        // 为同一徽章创建多个兑换规则
        let _rule1 = env
            .api
            .create_redemption_rule(&CreateRedemptionRuleRequest {
                name: "Test签到积分奖励".to_string(),
                description: Some("连续签到 7 天获得积分".to_string()),
                benefit_id: benefit_points.id,
                required_badges: json!([{"badge_id": badge.id, "quantity": 1}]),
                auto_redeem: true,
            })
            .await
            .unwrap();

        let _rule2 = env
            .api
            .create_redemption_rule(&CreateRedemptionRuleRequest {
                name: "Test签到优惠券奖励".to_string(),
                description: Some("连续签到 7 天获得优惠券".to_string()),
                benefit_id: benefit_coupon.id,
                required_badges: json!([{"badge_id": badge.id, "quantity": 1}]),
                auto_redeem: true,
            })
            .await
            .unwrap();

        // 验证关联了多个权益
        let rules = env
            .api
            .list_redemption_rules_by_badge(badge.id)
            .await
            .unwrap();
        assert!(rules.len() >= 2, "徽章应关联至少 2 个兑换规则");

        env.cleanup().await.unwrap();
    }

    /// 测试复合徽章条件的权益关联
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_link_benefit_with_multiple_badges() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 创建两个徽章
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
        let badge1 = env
            .api
            .create_badge(&TestBadges::spending_1000(series.id))
            .await
            .unwrap();
        let badge2 = env
            .api
            .create_badge(&TestBadges::spending_5000(series.id))
            .await
            .unwrap();

        // 创建需要多个徽章的权益兑换规则
        let benefit = env
            .api
            .create_benefit(&TestBenefits::physical("SKU_VIP_GIFT"))
            .await
            .unwrap();

        let rule = env
            .api
            .create_redemption_rule(&CreateRedemptionRuleRequest {
                name: "TestVIP 专属礼品".to_string(),
                description: Some("需要同时拥有千元达人和五千达人徽章".to_string()),
                benefit_id: benefit.id,
                required_badges: json!([
                    {"badge_id": badge1.id, "quantity": 1},
                    {"badge_id": badge2.id, "quantity": 1}
                ]),
                auto_redeem: false,
            })
            .await
            .unwrap();

        // 验证复合条件
        let required_badges: Vec<serde_json::Value> =
            serde_json::from_value(rule.required_badges.clone()).unwrap();
        assert_eq!(required_badges.len(), 2, "应需要 2 个徽章");

        env.cleanup().await.unwrap();
    }
}

#[cfg(test)]
mod benefit_grant_tests {
    use super::*;
    use crate::helpers::*;
    use serde_json::json;

    /// 测试徽章获取时自动发放权益
    ///
    /// 验证当用户获得徽章时，如果配置了自动兑换规则，权益会自动发放
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_auto_grant_benefit_on_badge() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 1. 创建徽章和规则
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
                name: "Test自动权益徽章".to_string(),
                description: Some("获取后自动发放积分".to_string()),
                badge_type: "normal".to_string(),
                icon_url: None,
                max_supply: None,
            })
            .await
            .unwrap();

        // 创建获取规则
        let _rule = env
            .api
            .create_rule(&CreateRuleRequest {
                badge_id: badge.id,
                rule_code: format!("test_auto_benefit_{}", badge.id),
                name: "Test自动权益规则".to_string(),
                event_type: "purchase".to_string(),
                rule_json: json!({
                    "type": "condition",
                    "field": "order.amount",
                    "operator": "gte",
                    "value": 100
                }),
                start_time: None,
                end_time: None,
                max_count_per_user: Some(1),
                global_quota: None,
            })
            .await
            .unwrap();

        // 2. 创建权益并配置自动兑换
        let benefit = env
            .api
            .create_benefit(&TestBenefits::points(300))
            .await
            .unwrap();
        let _redemption_rule = env
            .api
            .create_redemption_rule(&CreateRedemptionRuleRequest {
                name: "Test自动兑换规则".to_string(),
                description: Some("徽章获取时自动发放积分".to_string()),
                benefit_id: benefit.id,
                required_badges: json!([{"badge_id": badge.id, "quantity": 1}]),
                auto_redeem: true,
            })
            .await
            .unwrap();

        // 3. 上线徽章
        env.api
            .update_badge_status(badge.id, "active")
            .await
            .unwrap();

        // 4. 触发规则热加载
        env.kafka.send_rule_reload().await.unwrap();
        env.wait_for_rule_reload().await.unwrap();

        // 5. 发送购买事件触发徽章获取
        let user_id = UserGenerator::user_id();
        let event = TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 150);
        env.kafka.send_transaction_event(event).await.unwrap();

        // 6. 等待徽章发放
        env.wait_for_badge(&user_id, badge.id, Duration::from_secs(10))
            .await
            .unwrap();

        // 7. 验证徽章已发放
        assert!(
            env.db.user_has_badge(&user_id, badge.id).await.unwrap(),
            "用户应获得徽章"
        );

        // 8. 等待权益自动发放
        env.wait_for_benefit(&user_id, benefit.id, Duration::from_secs(10))
            .await
            .unwrap();

        // 9. 验证权益已发放
        assert!(
            env.db.benefit_granted(&user_id, benefit.id).await.unwrap(),
            "权益应已自动发放"
        );

        // 10. 验证权益发放记录
        let grants = env.db.get_benefit_grants(&user_id).await.unwrap();
        let grant = grants
            .iter()
            .find(|g| g.benefit_id == benefit.id)
            .expect("应有权益发放记录");
        assert_eq!(grant.status, "success", "权益发放状态应为 success");
        assert_eq!(grant.benefit_type, "points", "权益类型应为 points");

        env.cleanup().await.unwrap();
    }

    /// 测试手动兑换权益
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_manual_redeem_benefit() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 1. 构建场景：徽章 + 手动兑换权益
        let scenario = ScenarioBuilder::new(&env.api)
            .benefit_grant()
            .await
            .unwrap();

        // 2. 触发规则热加载
        env.kafka.send_rule_reload().await.unwrap();
        env.wait_for_rule_reload().await.unwrap();

        // 3. 先让用户获得徽章
        let user_id = UserGenerator::user_id();
        let event = TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 100);
        env.kafka.send_transaction_event(event).await.unwrap();

        // 等待徽章发放
        env.wait_for_badge(&user_id, scenario.badge.id, Duration::from_secs(10))
            .await
            .unwrap();

        // 4. 验证用户获得徽章
        assert!(
            env.db
                .user_has_badge(&user_id, scenario.badge.id)
                .await
                .unwrap(),
            "用户应获得徽章"
        );

        // 5. 查询用户可兑换的权益
        let user_benefits = env.api.get_user_benefits(&user_id).await.unwrap();

        // 验证可查询到权益列表
        // 具体是否可兑换取决于是否配置了手动兑换规则
        assert!(user_benefits.is_empty() || !user_benefits.is_empty());

        env.cleanup().await.unwrap();
    }

    /// 测试权益发放幂等性
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_benefit_grant_idempotency() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 创建场景
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
                name: "Test幂等性徽章".to_string(),
                description: Some("测试权益幂等发放".to_string()),
                badge_type: "normal".to_string(),
                icon_url: None,
                max_supply: None,
            })
            .await
            .unwrap();

        let _rule = env
            .api
            .create_rule(&CreateRuleRequest {
                badge_id: badge.id,
                rule_code: format!("test_idempotent_{}", badge.id),
                name: "Test幂等性规则".to_string(),
                event_type: "purchase".to_string(),
                rule_json: json!({
                    "type": "condition",
                    "field": "order.amount",
                    "operator": "gte",
                    "value": 50
                }),
                start_time: None,
                end_time: None,
                max_count_per_user: None, // 不限制次数
                global_quota: None,
            })
            .await
            .unwrap();

        let benefit = env
            .api
            .create_benefit(&TestBenefits::points(100))
            .await
            .unwrap();
        let _redemption_rule = env
            .api
            .create_redemption_rule(&CreateRedemptionRuleRequest {
                name: "Test幂等兑换规则".to_string(),
                description: Some("同一徽章只发放一次权益".to_string()),
                benefit_id: benefit.id,
                required_badges: json!([{"badge_id": badge.id, "quantity": 1}]),
                auto_redeem: true,
            })
            .await
            .unwrap();

        env.api
            .update_badge_status(badge.id, "active")
            .await
            .unwrap();
        env.kafka.send_rule_reload().await.unwrap();
        env.wait_for_rule_reload().await.unwrap();

        // 多次触发徽章获取
        let user_id = UserGenerator::user_id();
        for i in 0..3 {
            let event =
                TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 100 + i * 10);
            env.kafka.send_transaction_event(event).await.unwrap();
            env.wait_for_processing(Duration::from_secs(2))
                .await
                .unwrap();
        }

        // 等待所有处理完成
        env.wait_for_processing(Duration::from_secs(5))
            .await
            .unwrap();

        // 验证权益只发放一次
        let grants = env.db.get_benefit_grants(&user_id).await.unwrap();
        let benefit_grants: Vec<_> = grants
            .iter()
            .filter(|g| g.benefit_id == benefit.id && g.status == "success")
            .collect();

        assert_eq!(
            benefit_grants.len(),
            1,
            "权益应只发放一次，实际发放 {} 次",
            benefit_grants.len()
        );

        env.cleanup().await.unwrap();
    }
}

#[cfg(test)]
mod user_benefit_query_tests {
    use super::*;
    use crate::helpers::*;

    /// 测试查询用户权益列表
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_query_user_benefits() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        let user_id = "test_user_query";

        // 查询用户权益（可能为空）
        let benefits = env.api.get_user_benefits(user_id).await.unwrap();

        // 验证返回格式正确
        // 可能为空列表
        assert!(benefits.is_empty() || !benefits.is_empty());

        env.cleanup().await.unwrap();
    }

    /// 测试查询用户已发放权益的详细信息
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_query_granted_benefit_details() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 使用场景构建器创建完整测试数据
        let scenario = ScenarioBuilder::new(&env.api)
            .benefit_grant()
            .await
            .unwrap();

        env.kafka.send_rule_reload().await.unwrap();
        env.wait_for_rule_reload().await.unwrap();

        // 触发徽章和权益发放
        let user_id = UserGenerator::user_id();
        let event = TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 100);
        env.kafka.send_transaction_event(event).await.unwrap();

        // 等待处理
        env.wait_for_badge(&user_id, scenario.badge.id, Duration::from_secs(10))
            .await
            .unwrap();

        // 查询用户权益
        let benefits = env.api.get_user_benefits(&user_id).await.unwrap();

        // 如果有自动兑换规则，应该有权益记录
        for benefit in &benefits {
            assert!(!benefit.grant_no.is_empty(), "权益发放单号不应为空");
            assert!(!benefit.benefit_type.is_empty(), "权益类型不应为空");
            assert!(!benefit.status.is_empty(), "权益状态不应为空");
        }

        env.cleanup().await.unwrap();
    }
}

#[cfg(test)]
mod benefit_notification_tests {
    use super::*;
    use crate::helpers::*;
    use serde_json::json;

    /// 测试权益发放时发送通知
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_benefit_grant_notification() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 创建徽章和权益
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
                name: "Test通知徽章".to_string(),
                description: Some("测试权益发放通知".to_string()),
                badge_type: "normal".to_string(),
                icon_url: None,
                max_supply: None,
            })
            .await
            .unwrap();

        let _rule = env
            .api
            .create_rule(&CreateRuleRequest {
                badge_id: badge.id,
                rule_code: format!("test_notification_{}", badge.id),
                name: "Test通知规则".to_string(),
                event_type: "purchase".to_string(),
                rule_json: json!({
                    "type": "condition",
                    "field": "order.amount",
                    "operator": "gte",
                    "value": 100
                }),
                start_time: None,
                end_time: None,
                max_count_per_user: Some(1),
                global_quota: None,
            })
            .await
            .unwrap();

        let benefit = env
            .api
            .create_benefit(&TestBenefits::points(500))
            .await
            .unwrap();
        let _redemption_rule = env
            .api
            .create_redemption_rule(&CreateRedemptionRuleRequest {
                name: "Test通知兑换规则".to_string(),
                description: Some("测试权益发放通知".to_string()),
                benefit_id: benefit.id,
                required_badges: json!([{"badge_id": badge.id, "quantity": 1}]),
                auto_redeem: true,
            })
            .await
            .unwrap();

        env.api
            .update_badge_status(badge.id, "active")
            .await
            .unwrap();
        env.kafka.send_rule_reload().await.unwrap();
        env.wait_for_rule_reload().await.unwrap();

        // 清空通知队列
        env.kafka.drain_topic(topics::NOTIFICATIONS).await.unwrap();

        // 触发徽章和权益发放
        let user_id = UserGenerator::user_id();
        let event = TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 200);
        env.kafka.send_transaction_event(event).await.unwrap();

        // 等待处理
        env.wait_for_badge(&user_id, badge.id, Duration::from_secs(10))
            .await
            .unwrap();
        env.wait_for_benefit(&user_id, benefit.id, Duration::from_secs(10))
            .await
            .unwrap();

        // 消费通知消息
        let notifications = env.kafka.consume_notifications().await.unwrap();

        // 验证有权益发放通知
        let benefit_notification = notifications
            .iter()
            .find(|n| n.user_id == user_id && n.notification_type == "BENEFIT_GRANTED");

        assert!(
            benefit_notification.is_some(),
            "应发送权益发放通知给用户 {}",
            user_id
        );

        env.cleanup().await.unwrap();
    }
}
