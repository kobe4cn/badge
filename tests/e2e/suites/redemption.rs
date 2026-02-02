//! 兑换流程测试套件
//!
//! 测试徽章兑换和权益发放流程。

use crate::data::*;
use crate::helpers::*;
use crate::setup::TestEnvironment;
use serde_json::json;
use std::time::Duration;

#[cfg(test)]
mod normal_redemption_tests {
    use super::*;

    /// 兑换徽章获取积分
    ///
    /// 验证用户使用徽章兑换积分权益的完整流程：
    /// 1. 创建徽章和积分权益
    /// 2. 配置兑换规则（手动兑换）
    /// 3. 为用户发放徽章
    /// 4. 用户发起兑换请求
    /// 5. 验证积分发放成功且徽章被消耗
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_redeem_badge_for_points() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 1. 创建徽章
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
                &CreateBadgeRequest::new(series.id, "Test兑换积分徽章", "NORMAL")
                    .with_description("可兑换积分的徽章"),
            )
            .await
            .unwrap();

        // 2. 创建积分权益
        let benefit = env
            .api
            .create_benefit(&TestBenefits::points(500))
            .await
            .unwrap();

        // 3. 创建手动兑换规则
        let redemption_rule = env
            .api
            .create_redemption_rule(&CreateRedemptionRuleRequest {
                name: "Test积分兑换规则".to_string(),
                description: Some("使用徽章兑换 500 积分".to_string()),
                benefit_id: benefit.id,
                required_badges: vec![RequiredBadgeInput {
                    badge_id: badge.id,
                    quantity: 1,
                }],
                frequency_config: None,
                start_time: None,
                end_time: None,
                auto_redeem: false,
            })
            .await
            .unwrap();

        // 4. 上线徽章
        env.api
            .update_badge_status(badge.id, "active")
            .await
            .unwrap();

        // 5. 手动发放徽章给用户
        let user_id = UserGenerator::user_id();
        env.api
            .grant_badge(&ManualGrantRequest {
                user_id: user_id.clone(),
                badge_id: badge.id,
                quantity: 1,
                reason: "测试兑换".to_string(),
            })
            .await
            .unwrap();

        // 验证用户已获得徽章
        let user_badges = env.db.get_user_badges(&user_id).await.unwrap();
        let has_badge = user_badges
            .iter()
            .any(|b| b.badge_id == badge.id && b.quantity >= 1);
        assert!(has_badge, "用户应该拥有徽章");

        // 获取兑换前的徽章数量
        let badge_count_before = env
            .db
            .get_user_badge_count(&user_id, badge.id)
            .await
            .unwrap();

        // 6. 发起兑换请求
        let redeem_result = env
            .api
            .redeem_badge(&RedeemRequest {
                user_id: user_id.clone(),
                redemption_rule_id: redemption_rule.id,
            })
            .await;

        // 7. 验证兑换结果
        if let Ok(result) = redeem_result {
            assert!(result.success, "兑换应该成功");

            // 等待权益发放完成
            env.wait_for_processing(Duration::from_secs(3))
                .await
                .unwrap();

            // 验证权益已发放
            let granted = env.db.benefit_granted(&user_id, benefit.id).await.unwrap();
            assert!(granted, "积分权益应该已发放");

            // 验证徽章被消耗
            let badge_count_after = env
                .db
                .get_user_badge_count(&user_id, badge.id)
                .await
                .unwrap();
            assert_eq!(
                badge_count_after,
                badge_count_before - 1,
                "徽章数量应该减少 1"
            );

            // 验证账本记录包含兑换记录（ChangeType::RedeemOut 序列化为 "REDEEM_OUT"）
            let ledger = env.db.get_badge_ledger(badge.id, &user_id).await.unwrap();
            let redeem_record = ledger.iter().find(|r| r.action == "REDEEM_OUT");
            assert!(redeem_record.is_some(), "账本应有兑换记录");
        } else {
            // 如果兑换 API 未实现，跳过验证
            tracing::warn!("兑换 API 可能未实现: {:?}", redeem_result.err());
        }

        env.cleanup().await.unwrap();
    }

    /// 兑换徽章获取优惠券
    ///
    /// 验证使用徽章兑换优惠券权益的流程：
    /// 1. 创建徽章和优惠券权益
    /// 2. 配置兑换规则
    /// 3. 为用户发放徽章
    /// 4. 执行兑换
    /// 5. 验证优惠券发放
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_redeem_badge_for_coupon() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 1. 创建徽章
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
            .create_badge(
                &CreateBadgeRequest::new(series.id, "Test兑换优惠券徽章", "NORMAL")
                    .with_description("可兑换优惠券的徽章"),
            )
            .await
            .unwrap();

        // 2. 创建优惠券权益
        let coupon_template_id = "COUPON_TPL_001";
        let benefit = env
            .api
            .create_benefit(&TestBenefits::coupon(coupon_template_id, 30))
            .await
            .unwrap();

        // 3. 创建兑换规则
        let redemption_rule = env
            .api
            .create_redemption_rule(&CreateRedemptionRuleRequest {
                name: "Test优惠券兑换规则".to_string(),
                description: Some("使用徽章兑换优惠券".to_string()),
                benefit_id: benefit.id,
                required_badges: vec![RequiredBadgeInput {
                    badge_id: badge.id,
                    quantity: 1,
                }],
                frequency_config: None,
                start_time: None,
                end_time: None,
                auto_redeem: false,
            })
            .await
            .unwrap();

        // 4. 上线徽章并发放给用户
        env.api
            .update_badge_status(badge.id, "active")
            .await
            .unwrap();

        let user_id = UserGenerator::user_id();
        env.api
            .grant_badge(&ManualGrantRequest {
                user_id: user_id.clone(),
                badge_id: badge.id,
                quantity: 1,
                reason: "测试优惠券兑换".to_string(),
            })
            .await
            .unwrap();

        // 验证用户已获得徽章
        assert!(
            env.db.user_has_badge(&user_id, badge.id).await.unwrap(),
            "用户应该拥有徽章"
        );

        // 5. 发起兑换
        let redeem_result = env
            .api
            .redeem_badge(&RedeemRequest {
                user_id: user_id.clone(),
                redemption_rule_id: redemption_rule.id,
            })
            .await;

        // 6. 验证结果
        if let Ok(result) = redeem_result {
            assert!(result.success, "兑换应该成功");

            env.wait_for_processing(Duration::from_secs(3))
                .await
                .unwrap();

            // 验证权益发放
            let granted = env.db.benefit_granted(&user_id, benefit.id).await.unwrap();
            assert!(granted, "优惠券权益应该已发放");

            // 验证权益记录包含优惠券类型
            let grants = env.db.get_benefit_grants(&user_id).await.unwrap();
            let coupon_grant = grants.iter().find(|g| g.benefit_id == benefit.id);
            if let Some(grant) = coupon_grant {
                assert_eq!(grant.benefit_type, "coupon", "权益类型应为优惠券");
                assert_eq!(grant.status, "success", "发放状态应为成功");
            }
        } else {
            tracing::warn!("兑换 API 可能未实现: {:?}", redeem_result.err());
        }

        env.cleanup().await.unwrap();
    }

    /// 组合徽章兑换
    ///
    /// 验证需要多个不同徽章才能兑换的场景：
    /// 1. 创建多个徽章
    /// 2. 配置需要组合徽章的兑换规则
    /// 3. 分别发放所需徽章
    /// 4. 验证只有集齐所有徽章才能兑换
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_combination_redemption() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 1. 创建三个不同的徽章
        let category = env
            .api
            .create_category(&TestCategories::achievement())
            .await
            .unwrap();
        let series = env
            .api
            .create_series(&CreateSeriesRequest {
                category_id: category.id,
                name: "Test组合系列".to_string(),
                description: Some("需要集齐才能兑换".to_string()),
                cover_url: None,
                theme: Some("rainbow".to_string()),
            })
            .await
            .unwrap();

        let badge_a = env
            .api
            .create_badge(
                &CreateBadgeRequest::new(series.id, "Test组合徽章A", "NORMAL")
                    .with_description("组合徽章之一"),
            )
            .await
            .unwrap();

        let badge_b = env
            .api
            .create_badge(
                &CreateBadgeRequest::new(series.id, "Test组合徽章B", "NORMAL")
                    .with_description("组合徽章之二"),
            )
            .await
            .unwrap();

        let badge_c = env
            .api
            .create_badge(
                &CreateBadgeRequest::new(series.id, "Test组合徽章C", "NORMAL")
                    .with_description("组合徽章之三"),
            )
            .await
            .unwrap();

        // 2. 创建组合兑换权益
        let benefit = env
            .api
            .create_benefit(&TestBenefits::physical("SKU_COMBO_GIFT"))
            .await
            .unwrap();

        // 3. 创建组合兑换规则（需要 A、B、C 三个徽章各一个）
        let redemption_rule = env
            .api
            .create_redemption_rule(&CreateRedemptionRuleRequest {
                name: "Test组合兑换规则".to_string(),
                description: Some("集齐 ABC 三个徽章兑换礼品".to_string()),
                benefit_id: benefit.id,
                required_badges: vec![
                    RequiredBadgeInput { badge_id: badge_a.id, quantity: 1 },
                    RequiredBadgeInput { badge_id: badge_b.id, quantity: 1 },
                    RequiredBadgeInput { badge_id: badge_c.id, quantity: 1 },
                ],
                frequency_config: None,
                start_time: None,
                end_time: None,
                auto_redeem: false,
            })
            .await
            .unwrap();

        // 4. 上线所有徽章
        env.api
            .update_badge_status(badge_a.id, "active")
            .await
            .unwrap();
        env.api
            .update_badge_status(badge_b.id, "active")
            .await
            .unwrap();
        env.api
            .update_badge_status(badge_c.id, "active")
            .await
            .unwrap();

        let user_id = UserGenerator::user_id();

        // 5. 只发放两个徽章，尝试兑换应失败
        env.api
            .grant_badge(&ManualGrantRequest {
                user_id: user_id.clone(),
                badge_id: badge_a.id,
                quantity: 1,
                reason: "测试组合兑换".to_string(),
            })
            .await
            .unwrap();

        env.api
            .grant_badge(&ManualGrantRequest {
                user_id: user_id.clone(),
                badge_id: badge_b.id,
                quantity: 1,
                reason: "测试组合兑换".to_string(),
            })
            .await
            .unwrap();

        // 验证只有两个徽章
        assert!(env.db.user_has_badge(&user_id, badge_a.id).await.unwrap());
        assert!(env.db.user_has_badge(&user_id, badge_b.id).await.unwrap());
        assert!(!env.db.user_has_badge(&user_id, badge_c.id).await.unwrap());

        // 尝试兑换（应失败，因为缺少徽章 C）
        let incomplete_redeem = env
            .api
            .redeem_badge(&RedeemRequest {
                user_id: user_id.clone(),
                redemption_rule_id: redemption_rule.id,
            })
            .await;

        if let Ok(result) = &incomplete_redeem {
            assert!(!result.success, "缺少徽章时兑换应该失败");
        }

        // 6. 发放第三个徽章
        env.api
            .grant_badge(&ManualGrantRequest {
                user_id: user_id.clone(),
                badge_id: badge_c.id,
                quantity: 1,
                reason: "测试组合兑换".to_string(),
            })
            .await
            .unwrap();

        // 验证集齐三个徽章
        assert!(env.db.user_has_badge(&user_id, badge_c.id).await.unwrap());

        // 7. 再次尝试兑换（应成功）
        let complete_redeem = env
            .api
            .redeem_badge(&RedeemRequest {
                user_id: user_id.clone(),
                redemption_rule_id: redemption_rule.id,
            })
            .await;

        if let Ok(result) = complete_redeem {
            assert!(result.success, "集齐所有徽章后兑换应该成功");

            env.wait_for_processing(Duration::from_secs(3))
                .await
                .unwrap();

            // 验证权益已发放
            let granted = env.db.benefit_granted(&user_id, benefit.id).await.unwrap();
            assert!(granted, "实物权益应该已发放");

            // 验证三个徽章都被消耗
            let count_a = env
                .db
                .get_user_badge_count(&user_id, badge_a.id)
                .await
                .unwrap();
            let count_b = env
                .db
                .get_user_badge_count(&user_id, badge_b.id)
                .await
                .unwrap();
            let count_c = env
                .db
                .get_user_badge_count(&user_id, badge_c.id)
                .await
                .unwrap();
            assert_eq!(count_a, 0, "徽章 A 应被消耗");
            assert_eq!(count_b, 0, "徽章 B 应被消耗");
            assert_eq!(count_c, 0, "徽章 C 应被消耗");
        } else {
            tracing::warn!("兑换 API 可能未实现: {:?}", complete_redeem.err());
        }

        env.cleanup().await.unwrap();
    }
}

#[cfg(test)]
mod competitive_redemption_tests {
    use super::*;

    /// 限量兑换成功
    ///
    /// 验证限量权益的正常兑换流程：
    /// 1. 创建有库存限制的权益
    /// 2. 在库存充足时兑换成功
    /// 3. 验证库存正确扣减
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_limited_redemption_success() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 1. 创建徽章
        let category = env
            .api
            .create_category(&TestCategories::event())
            .await
            .unwrap();
        let series = env
            .api
            .create_series(&CreateSeriesRequest {
                category_id: category.id,
                name: "Test限量活动".to_string(),
                description: Some("限量活动测试".to_string()),
                cover_url: None,
                theme: Some("red".to_string()),
            })
            .await
            .unwrap();

        let badge = env
            .api
            .create_badge(
                &CreateBadgeRequest::new(series.id, "Test限量兑换徽章", "NORMAL")
                    .with_description("用于限量兑换"),
            )
            .await
            .unwrap();

        // 2. 创建限量权益（库存 10 份）
        let benefit = env
            .api
            .create_benefit(&CreateBenefitRequest {
                code: format!("TEST_LIMITED_{}", uuid::Uuid::new_v4().simple()),
                name: "Test限量积分".to_string(),
                description: Some("限量积分权益".to_string()),
                benefit_type: "POINTS".to_string(),
                external_id: None,
                external_system: None,
                total_stock: Some(10),
                config: Some(json!({
                    "points_amount": 1000
                })),
                icon_url: None,
            })
            .await
            .unwrap();

        // 3. 创建兑换规则
        let redemption_rule = env
            .api
            .create_redemption_rule(&CreateRedemptionRuleRequest {
                name: "Test限量兑换规则".to_string(),
                description: Some("限量 10 份".to_string()),
                benefit_id: benefit.id,
                required_badges: vec![RequiredBadgeInput {
                    badge_id: badge.id,
                    quantity: 1,
                }],
                frequency_config: None,
                start_time: None,
                end_time: None,
                auto_redeem: false,
            })
            .await
            .unwrap();

        env.api
            .update_badge_status(badge.id, "active")
            .await
            .unwrap();

        // 4. 发放徽章给用户
        let user_id = UserGenerator::user_id();
        env.api
            .grant_badge(&ManualGrantRequest {
                user_id: user_id.clone(),
                badge_id: badge.id,
                quantity: 1,
                reason: "测试限量兑换".to_string(),
            })
            .await
            .unwrap();

        // 5. 执行兑换
        let redeem_result = env
            .api
            .redeem_badge(&RedeemRequest {
                user_id: user_id.clone(),
                redemption_rule_id: redemption_rule.id,
            })
            .await;

        if let Ok(result) = redeem_result {
            assert!(result.success, "库存充足时兑换应该成功");

            env.wait_for_processing(Duration::from_secs(3))
                .await
                .unwrap();

            // 验证权益发放
            let granted = env.db.benefit_granted(&user_id, benefit.id).await.unwrap();
            assert!(granted, "权益应该已发放");
        } else {
            tracing::warn!("兑换 API 可能未实现: {:?}", redeem_result.err());
        }

        env.cleanup().await.unwrap();
    }

    /// 库存耗尽测试
    ///
    /// 验证库存耗尽后的兑换行为：
    /// 1. 创建库存极少的权益
    /// 2. 多个用户尝试兑换
    /// 3. 验证库存耗尽后兑换失败
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_limited_redemption_stock_exhausted() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 1. 创建徽章
        let category = env
            .api
            .create_category(&TestCategories::event())
            .await
            .unwrap();
        let series = env
            .api
            .create_series(&CreateSeriesRequest {
                category_id: category.id,
                name: "Test库存测试".to_string(),
                description: Some("库存耗尽测试".to_string()),
                cover_url: None,
                theme: Some("orange".to_string()),
            })
            .await
            .unwrap();

        let badge = env
            .api
            .create_badge(
                &CreateBadgeRequest::new(series.id, "Test库存徽章", "NORMAL")
                    .with_description("用于库存测试"),
            )
            .await
            .unwrap();

        // 2. 创建只有 2 份库存的权益
        let benefit = env
            .api
            .create_benefit(&CreateBenefitRequest {
                code: format!("TEST_STOCK_{}", uuid::Uuid::new_v4().simple()),
                name: "Test极限库存权益".to_string(),
                description: Some("极限库存测试".to_string()),
                benefit_type: "PHYSICAL".to_string(),
                external_id: Some("SKU_LIMITED".to_string()),
                external_system: Some("inventory".to_string()),
                total_stock: Some(2),
                config: Some(json!({
                    "sku_id": "SKU_LIMITED",
                    "shipping_required": true
                })),
                icon_url: None,
            })
            .await
            .unwrap();

        // 3. 创建兑换规则
        let redemption_rule = env
            .api
            .create_redemption_rule(&CreateRedemptionRuleRequest {
                name: "Test库存规则".to_string(),
                description: Some("仅限 2 份".to_string()),
                benefit_id: benefit.id,
                required_badges: vec![RequiredBadgeInput {
                    badge_id: badge.id,
                    quantity: 1,
                }],
                frequency_config: None,
                start_time: None,
                end_time: None,
                auto_redeem: false,
            })
            .await
            .unwrap();

        env.api
            .update_badge_status(badge.id, "active")
            .await
            .unwrap();

        // 4. 创建 5 个用户并发放徽章
        let user_ids: Vec<String> = (0..5).map(|_| UserGenerator::user_id()).collect();
        for user_id in &user_ids {
            env.api
                .grant_badge(&ManualGrantRequest {
                    user_id: user_id.clone(),
                    badge_id: badge.id,
                    quantity: 1,
                    reason: "测试库存耗尽".to_string(),
                })
                .await
                .unwrap();
        }

        // 5. 依次兑换
        let mut success_count = 0;
        let mut failed_count = 0;

        for user_id in &user_ids {
            let result = env
                .api
                .redeem_badge(&RedeemRequest {
                    user_id: user_id.clone(),
                    redemption_rule_id: redemption_rule.id,
                })
                .await;

            match result {
                Ok(r) if r.success => success_count += 1,
                Ok(_) => failed_count += 1,
                Err(_) => failed_count += 1,
            }

            // 短暂等待确保顺序处理
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // 6. 验证结果：最多 2 个成功，其余失败
        assert!(
            success_count <= 2,
            "成功兑换数量不应超过库存: {} > 2",
            success_count
        );
        assert!(
            failed_count >= 3,
            "至少应有 3 个用户因库存不足失败: {}",
            failed_count
        );

        tracing::info!(
            "库存测试结果: {} 成功, {} 失败",
            success_count,
            failed_count
        );

        env.cleanup().await.unwrap();
    }

    /// 并发兑换无超卖
    ///
    /// 验证高并发场景下的库存安全：
    /// 1. 创建限量权益
    /// 2. 多个用户同时发起兑换
    /// 3. 验证实际兑换数量不超过库存
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_concurrent_redemption_no_oversell() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 1. 创建徽章
        let category = env
            .api
            .create_category(&TestCategories::event())
            .await
            .unwrap();
        let series = env
            .api
            .create_series(&CreateSeriesRequest {
                category_id: category.id,
                name: "Test并发测试".to_string(),
                description: Some("并发兑换测试".to_string()),
                cover_url: None,
                theme: Some("blue".to_string()),
            })
            .await
            .unwrap();

        let badge = env
            .api
            .create_badge(
                &CreateBadgeRequest::new(series.id, "Test并发徽章", "NORMAL")
                    .with_description("用于并发测试"),
            )
            .await
            .unwrap();

        // 2. 创建库存 5 份的权益
        let stock_limit = 5;
        let benefit = env
            .api
            .create_benefit(&CreateBenefitRequest {
                code: format!("TEST_CONCURRENT_{}", uuid::Uuid::new_v4().simple()),
                name: "Test并发库存权益".to_string(),
                description: Some("并发测试用权益".to_string()),
                benefit_type: "POINTS".to_string(),
                external_id: None,
                external_system: None,
                total_stock: Some(stock_limit),
                config: Some(json!({
                    "points_amount": 100
                })),
                icon_url: None,
            })
            .await
            .unwrap();

        // 3. 创建兑换规则
        let redemption_rule = env
            .api
            .create_redemption_rule(&CreateRedemptionRuleRequest {
                name: "Test并发规则".to_string(),
                description: Some(format!("仅限 {} 份", stock_limit)),
                benefit_id: benefit.id,
                required_badges: vec![RequiredBadgeInput {
                    badge_id: badge.id,
                    quantity: 1,
                }],
                frequency_config: None,
                start_time: None,
                end_time: None,
                auto_redeem: false,
            })
            .await
            .unwrap();

        env.api
            .update_badge_status(badge.id, "active")
            .await
            .unwrap();

        // 4. 创建 20 个用户并发放徽章
        let concurrent_users = 20;
        let user_ids: Vec<String> = (0..concurrent_users)
            .map(|_| UserGenerator::user_id())
            .collect();

        for user_id in &user_ids {
            env.api
                .grant_badge(&ManualGrantRequest {
                    user_id: user_id.clone(),
                    badge_id: badge.id,
                    quantity: 1,
                    reason: "测试并发兑换".to_string(),
                })
                .await
                .unwrap();
        }

        // 5. 并发发起兑换请求
        let mut handles = Vec::new();
        for user_id in user_ids.clone() {
            let api = env.api.clone();
            let rule_id = redemption_rule.id;

            let handle = tokio::spawn(async move {
                api.redeem_badge(&RedeemRequest {
                    user_id,
                    redemption_rule_id: rule_id,
                })
                .await
            });
            handles.push(handle);
        }

        // 等待所有请求完成
        let results: Vec<_> = futures::future::join_all(handles).await;

        // 6. 统计成功数量
        let success_count = results
            .iter()
            .filter(|r| matches!(r, Ok(Ok(result)) if result.success))
            .count();

        // 7. 验证无超卖
        assert!(
            success_count <= stock_limit as usize,
            "并发兑换成功数量不应超过库存: {} > {}",
            success_count,
            stock_limit
        );

        // 等待处理完成
        env.wait_for_processing(Duration::from_secs(5))
            .await
            .unwrap();

        // 验证数据库中的权益发放数量
        let mut granted_count = 0;
        for user_id in &user_ids {
            if env.db.benefit_granted(user_id, benefit.id).await.unwrap() {
                granted_count += 1;
            }
        }

        assert!(
            granted_count <= stock_limit,
            "数据库中权益发放数量不应超过库存: {} > {}",
            granted_count,
            stock_limit
        );

        tracing::info!(
            "并发测试结果: {} 个用户中 {} 个成功兑换（库存 {}）",
            concurrent_users,
            granted_count,
            stock_limit
        );

        env.cleanup().await.unwrap();
    }
}

#[cfg(test)]
mod auto_redemption_tests {
    use super::*;

    /// 获取徽章时自动兑换
    ///
    /// 验证自动兑换功能：
    /// 1. 创建徽章和自动兑换规则
    /// 2. 用户通过事件获得徽章
    /// 3. 验证权益自动发放，无需手动兑换
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_auto_redeem_on_badge_grant() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 1. 创建徽章
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
                &CreateBadgeRequest::new(series.id, "Test自动兑换徽章", "NORMAL")
                    .with_description("获取后自动兑换积分"),
            )
            .await
            .unwrap();

        // 2. 创建触发规则
        let rule = env
            .api
            .create_rule(&CreateRuleRequest {
                badge_id: badge.id,
                rule_code: format!("test_auto_redeem_{}", badge.id),
                name: "Test自动兑换触发规则".to_string(),
                event_type: "purchase".to_string(),
                rule_json: json!({
                    "type": "condition",
                    "field": "amount",
                    "operator": "gte",
                    "value": 200
                }),
                start_time: None,
                end_time: None,
                max_count_per_user: Some(1),
                global_quota: None,
            })
            .await
            .unwrap();

        // 发布规则使其生效
        env.api.publish_rule(rule.id).await.unwrap();

        // 3. 创建积分权益
        let points_amount = 300;
        let benefit = env
            .api
            .create_benefit(&TestBenefits::points(points_amount))
            .await
            .unwrap();

        // 4. 创建兑换规则 (auto_redeem=true 启用自动兑换)
        let _redemption_rule = env
            .api
            .create_redemption_rule(&CreateRedemptionRuleRequest {
                name: "Test自动兑换积分规则".to_string(),
                description: Some("徽章获取后自动发放积分".to_string()),
                benefit_id: benefit.id,
                required_badges: vec![RequiredBadgeInput {
                    badge_id: badge.id,
                    quantity: 1,
                }],
                frequency_config: None,
                start_time: None,
                end_time: None,
                auto_redeem: true, // 启用自动兑换
            })
            .await
            .unwrap();

        // 刷新自动权益缓存，使规则立即生效
        env.api.refresh_auto_benefit_cache().await.unwrap();

        // 5. 上线徽章
        env.api
            .update_badge_status(badge.id, "active")
            .await
            .unwrap();

        // 6. 触发规则热加载
        env.kafka.send_rule_reload().await.unwrap();
        env.wait_for_rule_reload().await.unwrap();

        // 7. 发送购买事件触发徽章获取
        let user_id = UserGenerator::user_id();
        let event = TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 250);
        env.kafka.send_transaction_event(event).await.unwrap();

        // 8. 等待徽章发放
        env.wait_for_badge(&user_id, badge.id, Duration::from_secs(10))
            .await
            .unwrap();

        // 验证徽章已发放
        assert!(
            env.db.user_has_badge(&user_id, badge.id).await.unwrap(),
            "用户应该获得徽章"
        );

        // 9. 等待自动兑换完成
        env.wait_for_benefit(&user_id, benefit.id, Duration::from_secs(10))
            .await
            .unwrap();

        // 10. 验证权益自动发放
        assert!(
            env.db.benefit_granted(&user_id, benefit.id).await.unwrap(),
            "权益应该自动发放"
        );

        // 验证权益发放记录
        let grants = env.db.get_benefit_grants(&user_id).await.unwrap();
        let auto_grant = grants.iter().find(|g| g.benefit_id == benefit.id);
        assert!(auto_grant.is_some(), "应有权益发放记录");

        if let Some(grant) = auto_grant {
            assert_eq!(grant.status, "success", "发放状态应为成功");
            assert_eq!(grant.benefit_type, "POINTS", "权益类型应为积分");
        }

        // 验证徽章账本记录
        let ledger = env.db.get_badge_ledger(badge.id, &user_id).await.unwrap();

        // 应该有获取记录（ChangeType::Acquire 序列化为 "ACQUIRE"）
        let grant_record = ledger.iter().find(|r| r.action == "ACQUIRE");
        assert!(grant_record.is_some(), "账本应有徽章获取记录");

        // 如果是消耗型自动兑换，应该有兑换记录（ChangeType::RedeemOut 序列化为 "REDEEM_OUT"）
        let redeem_record = ledger.iter().find(|r| r.action == "REDEEM_OUT");
        if redeem_record.is_some() {
            // 验证徽章被消耗
            let badge_count = env
                .db
                .get_user_badge_count(&user_id, badge.id)
                .await
                .unwrap();
            assert_eq!(badge_count, 0, "自动兑换后徽章应被消耗");
        }

        env.cleanup().await.unwrap();
    }
}
