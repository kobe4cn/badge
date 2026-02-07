//! 深度嵌套规则测试套件
//!
//! 验证 3-5 层规则嵌套的创建、持久化、查询和规则引擎执行。
//! 这是生产就绪验证的关键测试。

use crate::data::*;
use crate::helpers::*;
use crate::setup::TestEnvironment;
use serde_json::{json, Value};

/// 生成 N 层嵌套规则 JSON
///
/// # 参数
/// - `depth`: 嵌套深度 (1-5)
/// - `breadth`: 每层子节点数量 (通常为 2)
fn generate_nested_rule(depth: usize, breadth: usize) -> Value {
    build_nested_node(depth, breadth, 0)
}

fn build_nested_node(depth: usize, breadth: usize, level: usize) -> Value {
    if depth == 0 {
        // 叶子节点：条件节点
        json!({
            "type": "condition",
            "field": format!("field_L{}_D{}", level, depth),
            "operator": "gte",
            "value": (level + 1) * 100
        })
    } else {
        // 交替使用 AND/OR
        let operator = if depth % 2 == 0 { "AND" } else { "OR" };
        let children: Vec<Value> = (0..breadth)
            .map(|i| build_nested_node(depth - 1, breadth, i))
            .collect();

        json!({
            "type": "group",
            "operator": operator,
            "children": children
        })
    }
}

/// 计算规则树的实际深度
fn calculate_depth(node: &Value) -> usize {
    match node.get("type").and_then(|t| t.as_str()) {
        Some("condition") => 1,
        Some("group") => {
            let children = node
                .get("children")
                .and_then(|c| c.as_array())
                .map(|arr| arr.iter().map(calculate_depth).max().unwrap_or(0))
                .unwrap_or(0);
            1 + children
        }
        _ => 0,
    }
}

/// 统计条件节点数量
fn count_conditions(node: &Value) -> usize {
    match node.get("type").and_then(|t| t.as_str()) {
        Some("condition") => 1,
        Some("group") => node
            .get("children")
            .and_then(|c| c.as_array())
            .map(|arr| arr.iter().map(count_conditions).sum())
            .unwrap_or(0),
        _ => 0,
    }
}

#[cfg(test)]
mod nesting_3_layer_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_create_3_layer_nested_rule() {
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
            .create_badge(&TestBadges::spending_1000(series.id))
            .await
            .unwrap();

        // 生成 3 层嵌套规则
        let rule_json = generate_nested_rule(3, 2);

        // 验证生成的结构
        assert_eq!(calculate_depth(&rule_json), 4); // 3 层 group + 1 层 condition
        assert_eq!(count_conditions(&rule_json), 8); // 2^3 = 8

        let req = CreateRuleRequest {
            badge_id: badge.id,
            rule_code: format!("test_3layer_{}", badge.id),
            name: "Test3层嵌套规则".to_string(),
            event_type: "purchase".to_string(),
            rule_json: rule_json.clone(),
            start_time: None,
            end_time: None,
            max_count_per_user: None,
            global_quota: None,
        };

        let rule = env.api.create_rule(&req).await.unwrap();
        assert!(rule.id > 0);

        // 从数据库验证持久化
        let db_rule = env.db.get_rule(rule.id).await.unwrap().unwrap();
        let saved_json: Value = serde_json::from_value(db_rule.rule_json).unwrap();
        assert_eq!(calculate_depth(&saved_json), 4);
        assert_eq!(count_conditions(&saved_json), 8);

        env.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_query_3_layer_rule_preserves_structure() {
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

        let original_json = generate_nested_rule(3, 2);
        let req = CreateRuleRequest {
            badge_id: badge.id,
            rule_code: format!("test_query_3layer_{}", badge.id),
            name: "Test查询3层规则".to_string(),
            event_type: "purchase".to_string(),
            rule_json: original_json.clone(),
            start_time: None,
            end_time: None,
            max_count_per_user: None,
            global_quota: None,
        };

        let created = env.api.create_rule(&req).await.unwrap();

        // 通过 API 查询规则
        let queried = env.api.get_rule(created.id).await.unwrap();
        let queried_json = queried.rule_json;

        // 验证结构完全一致
        assert_eq!(calculate_depth(&queried_json), calculate_depth(&original_json));
        assert_eq!(
            count_conditions(&queried_json),
            count_conditions(&original_json)
        );

        // 验证顶层结构
        assert_eq!(queried_json["type"], "group");
        assert_eq!(queried_json["children"].as_array().unwrap().len(), 2);

        env.cleanup().await.unwrap();
    }
}

#[cfg(test)]
mod nesting_4_layer_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_create_4_layer_nested_rule() {
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

        let rule_json = generate_nested_rule(4, 2);

        // 4 层 group + 1 层 condition = 深度 5
        assert_eq!(calculate_depth(&rule_json), 5);
        // 2^4 = 16 个条件节点
        assert_eq!(count_conditions(&rule_json), 16);

        let req = CreateRuleRequest {
            badge_id: badge.id,
            rule_code: format!("test_4layer_{}", badge.id),
            name: "Test4层嵌套规则".to_string(),
            event_type: "purchase".to_string(),
            rule_json,
            start_time: None,
            end_time: None,
            max_count_per_user: Some(1),
            global_quota: None,
        };

        let rule = env.api.create_rule(&req).await.unwrap();
        assert!(rule.id > 0);

        // 验证数据库
        let db_rule = env.db.get_rule(rule.id).await.unwrap().unwrap();
        let saved_json: Value = serde_json::from_value(db_rule.rule_json).unwrap();
        assert_eq!(calculate_depth(&saved_json), 5);
        assert_eq!(count_conditions(&saved_json), 16);

        env.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_publish_4_layer_rule() {
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
            .create_badge(&TestBadges::first_purchase(series.id))
            .await
            .unwrap();

        // 上线徽章
        env.api
            .update_badge_status(badge.id, "active")
            .await
            .unwrap();

        let rule_json = generate_nested_rule(4, 2);
        let req = CreateRuleRequest {
            badge_id: badge.id,
            rule_code: format!("test_publish_4layer_{}", badge.id),
            name: "Test发布4层规则".to_string(),
            event_type: "engagement".to_string(),
            rule_json,
            start_time: None,
            end_time: None,
            max_count_per_user: None,
            global_quota: None,
        };

        let rule = env.api.create_rule(&req).await.unwrap();
        assert!(!rule.enabled, "新规则应该默认禁用");

        // 发布规则
        env.api.publish_rule(rule.id).await.unwrap();

        // 验证已发布
        let published = env.api.get_rule(rule.id).await.unwrap();
        assert!(published.enabled, "发布后规则应该启用");

        env.cleanup().await.unwrap();
    }
}

#[cfg(test)]
mod nesting_5_layer_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_create_5_layer_nested_rule() {
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
            .create_badge(&CreateBadgeRequest::new(series.id, "Test5层徽章", "NORMAL"))
            .await
            .unwrap();

        let rule_json = generate_nested_rule(5, 2);

        // 5 层 group + 1 层 condition = 深度 6
        assert_eq!(calculate_depth(&rule_json), 6);
        // 2^5 = 32 个条件节点
        assert_eq!(count_conditions(&rule_json), 32);

        let req = CreateRuleRequest {
            badge_id: badge.id,
            rule_code: format!("test_5layer_{}", badge.id),
            name: "Test5层嵌套规则".to_string(),
            event_type: "purchase".to_string(),
            rule_json: rule_json.clone(),
            start_time: None,
            end_time: None,
            max_count_per_user: None,
            global_quota: None,
        };

        let rule = env.api.create_rule(&req).await.unwrap();
        assert!(rule.id > 0);

        // 验证 JSON 序列化大小（32 个条件节点应该产生较大的 JSON）
        let json_string = serde_json::to_string(&rule.rule_json).unwrap();
        assert!(
            json_string.len() > 1000,
            "5层嵌套规则的 JSON 应该足够大"
        );

        // 验证数据库存储
        let db_rule = env.db.get_rule(rule.id).await.unwrap().unwrap();
        let saved_json: Value = serde_json::from_value(db_rule.rule_json).unwrap();

        // 深度和条件数量必须保持不变
        assert_eq!(calculate_depth(&saved_json), 6);
        assert_eq!(count_conditions(&saved_json), 32);

        env.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_5_layer_rule_json_integrity() {
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
            .create_badge(&CreateBadgeRequest::new(
                series.id,
                "Test5层完整性徽章",
                "NORMAL",
            ))
            .await
            .unwrap();

        let original_json = generate_nested_rule(5, 2);
        let original_string = serde_json::to_string(&original_json).unwrap();

        let req = CreateRuleRequest {
            badge_id: badge.id,
            rule_code: format!("test_integrity_5layer_{}", badge.id),
            name: "Test5层完整性规则".to_string(),
            event_type: "purchase".to_string(),
            rule_json: original_json,
            start_time: None,
            end_time: None,
            max_count_per_user: None,
            global_quota: None,
        };

        let rule = env.api.create_rule(&req).await.unwrap();

        // 查询并比较
        let queried = env.api.get_rule(rule.id).await.unwrap();
        let queried_string = serde_json::to_string(&queried.rule_json).unwrap();

        // JSON 字符串长度应该相近（允许格式化差异）
        let len_diff = (original_string.len() as i64 - queried_string.len() as i64).abs();
        assert!(
            len_diff < 100,
            "JSON 长度差异应该很小，原始: {}, 查询: {}",
            original_string.len(),
            queried_string.len()
        );

        // 递归验证每个节点
        fn validate_node(original: &Value, queried: &Value) {
            assert_eq!(original["type"], queried["type"]);
            match original.get("type").and_then(|t| t.as_str()) {
                Some("condition") => {
                    assert_eq!(original["field"], queried["field"]);
                    assert_eq!(original["operator"], queried["operator"]);
                    assert_eq!(original["value"], queried["value"]);
                }
                Some("group") => {
                    assert_eq!(original["operator"], queried["operator"]);
                    let orig_children = original["children"].as_array().unwrap();
                    let query_children = queried["children"].as_array().unwrap();
                    assert_eq!(orig_children.len(), query_children.len());
                    for (o, q) in orig_children.iter().zip(query_children.iter()) {
                        validate_node(o, q);
                    }
                }
                _ => panic!("未知节点类型"),
            }
        }

        let orig: Value = serde_json::from_str(&original_string).unwrap();
        validate_node(&orig, &queried.rule_json);

        env.cleanup().await.unwrap();
    }
}

#[cfg(test)]
mod mixed_nesting_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_asymmetric_nesting() {
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
            .create_badge(&CreateBadgeRequest::new(series.id, "Test非对称徽章", "NORMAL"))
            .await
            .unwrap();

        // 非对称结构：左侧 4 层，右侧 1 层
        let rule_json = json!({
            "type": "group",
            "operator": "OR",
            "children": [
                generate_nested_rule(4, 2),
                {
                    "type": "condition",
                    "field": "simple_field",
                    "operator": "eq",
                    "value": "simple_value"
                }
            ]
        });

        let req = CreateRuleRequest {
            badge_id: badge.id,
            rule_code: format!("test_asymmetric_{}", badge.id),
            name: "Test非对称嵌套规则".to_string(),
            event_type: "purchase".to_string(),
            rule_json: rule_json.clone(),
            start_time: None,
            end_time: None,
            max_count_per_user: None,
            global_quota: None,
        };

        let rule = env.api.create_rule(&req).await.unwrap();

        // 验证保存的结构
        let saved = env.api.get_rule(rule.id).await.unwrap();
        assert_eq!(saved.rule_json["children"][0]["type"], "group");
        assert_eq!(saved.rule_json["children"][1]["type"], "condition");

        env.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_complex_vip_rule() {
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
            .create_badge(&CreateBadgeRequest::new(series.id, "TestVIP徽章", "SPECIAL"))
            .await
            .unwrap();

        // 复杂业务场景规则
        let rule_json = json!({
            "type": "group",
            "operator": "OR",
            "children": [
                {
                    "type": "group",
                    "operator": "AND",
                    "children": [
                        {"type": "condition", "field": "user.level", "operator": "gte", "value": 3},
                        {"type": "condition", "field": "user.total_spent", "operator": "gte", "value": 10000}
                    ]
                },
                {
                    "type": "group",
                    "operator": "AND",
                    "children": [
                        {"type": "condition", "field": "user.level", "operator": "gte", "value": 2},
                        {"type": "condition", "field": "user.total_spent", "operator": "gte", "value": 5000},
                        {"type": "condition", "field": "user.consecutive_checkin", "operator": "gte", "value": 30}
                    ]
                },
                {
                    "type": "group",
                    "operator": "AND",
                    "children": [
                        {"type": "condition", "field": "user.invited_friends", "operator": "gte", "value": 10},
                        {
                            "type": "group",
                            "operator": "OR",
                            "children": [
                                {"type": "condition", "field": "user.total_spent", "operator": "gte", "value": 3000},
                                {"type": "condition", "field": "user.consecutive_checkin", "operator": "gte", "value": 60}
                            ]
                        }
                    ]
                }
            ]
        });

        let req = CreateRuleRequest {
            badge_id: badge.id,
            rule_code: format!("test_vip_complex_{}", badge.id),
            name: "TestVIP复杂规则".to_string(),
            event_type: "purchase".to_string(),
            rule_json,
            start_time: None,
            end_time: None,
            max_count_per_user: Some(1),
            global_quota: None,
        };

        let rule = env.api.create_rule(&req).await.unwrap();

        // 验证结构
        let saved = env.api.get_rule(rule.id).await.unwrap();
        assert_eq!(saved.rule_json["operator"], "OR");
        assert_eq!(saved.rule_json["children"].as_array().unwrap().len(), 3);

        // 验证第三个分支包含嵌套 OR
        let third_branch = &saved.rule_json["children"][2];
        assert_eq!(third_branch["children"][1]["type"], "group");
        assert_eq!(third_branch["children"][1]["operator"], "OR");

        env.cleanup().await.unwrap();
    }
}

#[cfg(test)]
mod nesting_edge_cases {
    use super::*;

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_single_condition_no_nesting() {
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
            .create_badge(&CreateBadgeRequest::new(series.id, "Test单条件徽章", "NORMAL"))
            .await
            .unwrap();

        // 最简单的规则：单条件
        let rule_json = json!({
            "type": "condition",
            "field": "amount",
            "operator": "gte",
            "value": 100
        });

        let req = CreateRuleRequest {
            badge_id: badge.id,
            rule_code: format!("test_single_condition_{}", badge.id),
            name: "Test单条件规则".to_string(),
            event_type: "purchase".to_string(),
            rule_json,
            start_time: None,
            end_time: None,
            max_count_per_user: None,
            global_quota: None,
        };

        let rule = env.api.create_rule(&req).await.unwrap();
        assert_eq!(rule.rule_json["type"], "condition");
        assert_eq!(calculate_depth(&rule.rule_json), 1);
        assert_eq!(count_conditions(&rule.rule_json), 1);

        env.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_all_operators_in_nested_rule() {
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
            .create_badge(&CreateBadgeRequest::new(
                series.id,
                "Test全操作符徽章",
                "NORMAL",
            ))
            .await
            .unwrap();

        // 包含所有操作符的规则
        let rule_json = json!({
            "type": "group",
            "operator": "OR",
            "children": [
                {"type": "condition", "field": "f1", "operator": "eq", "value": 1},
                {"type": "condition", "field": "f2", "operator": "neq", "value": 2},
                {"type": "condition", "field": "f3", "operator": "gt", "value": 3},
                {"type": "condition", "field": "f4", "operator": "gte", "value": 4},
                {"type": "condition", "field": "f5", "operator": "lt", "value": 5},
                {"type": "condition", "field": "f6", "operator": "lte", "value": 6},
                {"type": "condition", "field": "f7", "operator": "in", "value": ["a", "b"]},
                {"type": "condition", "field": "f8", "operator": "notIn", "value": ["c", "d"]},
                {"type": "condition", "field": "f9", "operator": "contains", "value": "str"},
                {"type": "condition", "field": "f10", "operator": "startsWith", "value": "pre"},
                {"type": "condition", "field": "f11", "operator": "endsWith", "value": "suf"},
                {"type": "condition", "field": "f12", "operator": "between", "value": [10, 20]},
                {"type": "condition", "field": "f13", "operator": "isEmpty", "value": null},
                {"type": "condition", "field": "f14", "operator": "isNotEmpty", "value": null}
            ]
        });

        let req = CreateRuleRequest {
            badge_id: badge.id,
            rule_code: format!("test_all_operators_{}", badge.id),
            name: "Test全操作符规则".to_string(),
            event_type: "test".to_string(),
            rule_json,
            start_time: None,
            end_time: None,
            max_count_per_user: None,
            global_quota: None,
        };

        let rule = env.api.create_rule(&req).await.unwrap();
        assert_eq!(rule.rule_json["children"].as_array().unwrap().len(), 14);

        env.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_unicode_values_in_nested_rule() {
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
            .create_badge(&CreateBadgeRequest::new(
                series.id,
                "TestUnicode徽章",
                "NORMAL",
            ))
            .await
            .unwrap();

        // 包含中文等 Unicode 字符的规则
        let rule_json = json!({
            "type": "group",
            "operator": "AND",
            "children": [
                {"type": "condition", "field": "user.name", "operator": "contains", "value": "测试用户"},
                {"type": "condition", "field": "product.category", "operator": "in", "value": ["电子产品", "服装", "食品"]},
                {
                    "type": "group",
                    "operator": "OR",
                    "children": [
                        {"type": "condition", "field": "region", "operator": "eq", "value": "北京市"},
                        {"type": "condition", "field": "region", "operator": "eq", "value": "上海市"}
                    ]
                }
            ]
        });

        let req = CreateRuleRequest {
            badge_id: badge.id,
            rule_code: format!("test_unicode_{}", badge.id),
            name: "Test中文规则".to_string(),
            event_type: "purchase".to_string(),
            rule_json,
            start_time: None,
            end_time: None,
            max_count_per_user: None,
            global_quota: None,
        };

        let rule = env.api.create_rule(&req).await.unwrap();

        // 验证中文正确保存
        let saved = env.api.get_rule(rule.id).await.unwrap();
        assert_eq!(saved.rule_json["children"][0]["value"], "测试用户");
        assert_eq!(
            saved.rule_json["children"][1]["value"].as_array().unwrap()[0],
            "电子产品"
        );

        env.cleanup().await.unwrap();
    }
}
