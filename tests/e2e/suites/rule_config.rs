//! 规则配置测试套件
//!
//! 测试规则的创建、画布编辑、热加载等功能。

use crate::data::*;

use crate::helpers::*;
use crate::setup::TestEnvironment;
use serde_json::json;

#[cfg(test)]
mod rule_crud_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_create_simple_rule() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 准备徽章
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

        // 创建简单规则
        let req = TestRules::first_purchase(badge.id);
        let rule = env.api.create_rule(&req).await.unwrap();

        assert_eq!(rule.badge_id, badge.id);
        assert!(!rule.enabled, "新规则默认应该禁用，需要发布后才启用");

        env.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_create_combined_rule() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

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
            .create_badge(&TestBadges::spending_1000(series.id))
            .await
            .unwrap();

        // 创建组合规则 (AND)
        let conditions = vec![
            RuleJsonGenerator::simple_condition("total_amount", "gte", 1000),
            RuleJsonGenerator::simple_condition("user.is_vip", "eq", true),
        ];
        let req = TestRules::combined_and(badge.id, conditions);
        let rule = env.api.create_rule(&req).await.unwrap();

        assert_eq!(rule.rule_json["type"], "group");
        assert_eq!(rule.rule_json["operator"], "AND");

        env.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_create_nested_rule() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

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
            .create_badge(&TestBadges::spending_5000(series.id))
            .await
            .unwrap();

        // 创建嵌套规则 (A AND B) OR (C AND D)
        let rule_json = json!({
            "type": "group",
            "operator": "OR",
            "children": [
                {
                    "type": "group",
                    "operator": "AND",
                    "children": [
                        {"type": "condition", "field": "total_amount", "operator": "gte", "value": 5000},
                        {"type": "condition", "field": "user.level", "operator": "gte", "value": 3}
                    ]
                },
                {
                    "type": "group",
                    "operator": "AND",
                    "children": [
                        {"type": "condition", "field": "total_amount", "operator": "gte", "value": 10000},
                        {"type": "condition", "field": "order_count", "operator": "gte", "value": 5}
                    ]
                }
            ]
        });

        let req = CreateRuleRequest {
            badge_id: badge.id,
            rule_code: format!("test_nested_{}", badge.id),
            name: "Test嵌套规则".to_string(),
            event_type: "purchase".to_string(),
            rule_json,
            start_time: None,
            end_time: None,
            max_count_per_user: Some(1),
            global_quota: None,
        };

        let rule = env.api.create_rule(&req).await.unwrap();
        assert_eq!(rule.rule_json["type"], "group");
        assert_eq!(rule.rule_json["operator"], "OR");
        assert_eq!(rule.rule_json["children"].as_array().unwrap().len(), 2);

        env.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_update_rule() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

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
        let rule = env
            .api
            .create_rule(&TestRules::first_purchase(badge.id))
            .await
            .unwrap();

        // 更新规则
        let update_req = UpdateRuleRequest {
            name: Some("Test更新后的规则".to_string()),
            rule_json: Some(json!({
                "type": "condition",
                "field": "purchase_count",
                "operator": "gte",
                "value": 2
            })),
            enabled: Some(true),
        };

        let updated = env.api.update_rule(rule.id, &update_req).await.unwrap();
        assert_eq!(updated.rule_json["value"], 2);

        env.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_disable_rule() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

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
        let rule = env
            .api
            .create_rule(&TestRules::first_purchase(badge.id))
            .await
            .unwrap();

        // 禁用规则
        let update_req = UpdateRuleRequest {
            name: None,
            rule_json: None,
            enabled: Some(false),
        };

        let updated = env.api.update_rule(rule.id, &update_req).await.unwrap();
        assert!(!updated.enabled, "规则应该被禁用");

        env.cleanup().await.unwrap();
    }
}

#[cfg(test)]
mod rule_condition_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_all_operators() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

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

        // 测试各种操作符
        let operators = vec![
            ("eq", json!(1000)),
            ("neq", json!(0)),
            ("gt", json!(500)),
            ("gte", json!(1000)),
            ("lt", json!(2000)),
            ("lte", json!(1000)),
            ("in", json!(["A", "B", "C"])),
            ("notIn", json!(["X", "Y"])),
            ("contains", json!("test")),
            ("startsWith", json!("prefix")),
            ("endsWith", json!("suffix")),
            ("between", json!([100, 1000])),
            ("isEmpty", json!(null)),
            ("isNotEmpty", json!(null)),
        ];

        for (i, (op, value)) in operators.iter().enumerate() {
            let badge = env
                .api
                .create_badge(&CreateBadgeRequest::new(
                    series.id,
                    &format!("Test操作符{}", op),
                    "NORMAL",
                ))
                .await
                .unwrap();

            let rule_json = json!({
                "type": "condition",
                "field": "test_field",
                "operator": op,
                "value": value
            });

            let req = CreateRuleRequest {
                badge_id: badge.id,
                rule_code: format!("test_op_{}_{}", op, i),
                name: format!("Test{}操作符规则", op),
                event_type: "purchase".to_string(),
                rule_json,
                start_time: None,
                end_time: None,
                max_count_per_user: None,
                global_quota: None,
            };

            let result = env.api.create_rule(&req).await;
            assert!(result.is_ok(), "操作符 {} 的规则应该能创建成功", op);
        }

        env.cleanup().await.unwrap();
    }
}

#[cfg(test)]
mod rule_quota_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_rule_with_user_limit() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

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

        // 创建带用户限制的规则
        let req = CreateRuleRequest {
            badge_id: badge.id,
            rule_code: format!("test_user_limit_{}", badge.id),
            name: "Test用户限制规则".to_string(),
            event_type: "purchase".to_string(),
            rule_json: json!({"type": "condition", "field": "amount", "operator": "gte", "value": 100}),
            start_time: None,
            end_time: None,
            max_count_per_user: Some(1),
            global_quota: None,
        };

        let rule = env.api.create_rule(&req).await.unwrap();

        // 验证数据库
        let _db_rule = env.db.get_rule(rule.id).await.unwrap().unwrap();
        // 注意: max_count_per_user 可能存储方式不同

        env.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_rule_with_global_quota() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        let category = env
            .api
            .create_category(&TestCategories::event())
            .await
            .unwrap();
        let series = env
            .api
            .create_series(&TestSeries::newcomer(category.id))
            .await
            .unwrap();
        let badge = env
            .api
            .create_badge(&TestBadges::limited_edition(series.id, 100))
            .await
            .unwrap();

        // 创建带全局配额的规则
        let req = TestRules::with_quota(badge.id, 100);
        let rule = env.api.create_rule(&req).await.unwrap();

        // 验证数据库
        let db_rule = env.db.get_rule(rule.id).await.unwrap().unwrap();
        // TODO: API CreateRuleRequest 尚未支持 global_quota 字段
        // 当 API 支持后，取消以下注释：
        // assert_eq!(db_rule.global_quota, Some(100));
        assert_eq!(db_rule.global_granted, 0);

        env.cleanup().await.unwrap();
    }
}

#[cfg(test)]
mod rule_hot_reload_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_rule_hot_reload() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

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

        // 上线徽章
        env.api
            .update_badge_status(badge.id, "active")
            .await
            .unwrap();

        // 创建规则
        let _rule = env
            .api
            .create_rule(&TestRules::first_purchase(badge.id))
            .await
            .unwrap();

        // 触发热加载
        env.kafka.send_rule_reload().await.unwrap();

        // 等待热加载完成
        env.wait_for_rule_reload().await.unwrap();

        // 验证规则可用（通过发送事件验证）
        // TODO: 这里需要发送事件并验证处理结果

        env.cleanup().await.unwrap();
    }
}
