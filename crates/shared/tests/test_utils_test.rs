//! test_utils 模块的集成测试
//!
//! 验证测试工具模块的功能正确性

use badge_shared::test_utils::*;
use serde_json::json;

// ==================== 测试数据生成器测试 ====================

#[test]
fn test_badge_test_data_generation() {
    let badge = TestDataGenerator::badge("Premium Badge");
    assert_eq!(badge.name, "Premium Badge");
    assert_eq!(badge.status, "ACTIVE");
    assert_eq!(badge.badge_type, "NORMAL");
    assert!(badge.max_supply.is_none());
    assert_eq!(badge.issued_count, 0);
}

#[test]
fn test_limited_badge_generation() {
    let badge = TestDataGenerator::limited_badge("Limited Edition", 500);
    assert_eq!(badge.name, "Limited Edition");
    assert_eq!(badge.max_supply, Some(500));
    assert_eq!(badge.badge_type, "LIMITED");
}

#[test]
fn test_timed_badge_generation() {
    let badge = TestDataGenerator::timed_badge("Seasonal Badge", 30);
    assert_eq!(badge.validity_config["validityType"], "RELATIVE_DAYS");
    assert_eq!(badge.validity_config["relativeDays"], 30);
}

#[test]
fn test_purchase_event_generation() {
    let event = TestDataGenerator::purchase_event("test-user-123", 1000);

    assert_eq!(event["event"]["type"], "PURCHASE");
    assert_eq!(event["order"]["amount"], 1000);
    assert_eq!(event["user"]["id"], "test-user-123");
    assert_eq!(event["user"]["is_vip"], false);
}

#[test]
fn test_vip_purchase_event_generation() {
    let event = TestDataGenerator::vip_purchase_event("vip-user", 5000);

    assert_eq!(event["event"]["type"], "PURCHASE");
    assert_eq!(event["order"]["amount"], 5000);
    assert_eq!(event["user"]["id"], "vip-user");
    assert_eq!(event["user"]["is_vip"], true);
    assert_eq!(event["user"]["level"], "gold");
}

#[test]
fn test_engagement_event_generation() {
    let event = TestDataGenerator::engagement_event("user-001", "CHECK_IN");

    assert_eq!(event["event"]["type"], "CHECK_IN");
    assert_eq!(event["user"]["id"], "user-001");
    assert_eq!(event["activity"]["type"], "check_in");
}

// ==================== 规则生成器测试 ====================

#[test]
fn test_simple_rule_generation() {
    let rule = TestDataGenerator::simple_rule(
        "purchase-rule",
        "event.type",
        "eq",
        json!("PURCHASE"),
    );

    assert_eq!(rule["id"], "purchase-rule");
    assert_eq!(rule["root"]["type"], "condition");
    assert_eq!(rule["root"]["field"], "event.type");
    assert_eq!(rule["root"]["operator"], "eq");
    assert_eq!(rule["root"]["value"], "PURCHASE");
}

#[test]
fn test_and_rule_generation() {
    let conditions = vec![
        TestDataGenerator::condition("event.type", "eq", json!("PURCHASE")),
        TestDataGenerator::condition("order.amount", "gte", json!(1000)),
    ];

    let rule = TestDataGenerator::and_rule("and-test", conditions);

    assert_eq!(rule["id"], "and-test");
    assert_eq!(rule["root"]["type"], "group");
    assert_eq!(rule["root"]["operator"], "AND");

    let children = rule["root"]["children"].as_array().unwrap();
    assert_eq!(children.len(), 2);
}

#[test]
fn test_or_rule_generation() {
    let conditions = vec![
        TestDataGenerator::condition("user.is_vip", "eq", json!(true)),
        TestDataGenerator::condition("user.level", "eq", json!("gold")),
    ];

    let rule = TestDataGenerator::or_rule("or-test", conditions);

    assert_eq!(rule["root"]["operator"], "OR");
}

#[test]
fn test_condition_generation() {
    let condition = TestDataGenerator::condition("user.tags", "contains", json!("premium"));

    assert_eq!(condition["type"], "condition");
    assert_eq!(condition["field"], "user.tags");
    assert_eq!(condition["operator"], "contains");
    assert_eq!(condition["value"], "premium");
}

// ==================== Mock 规则引擎测试 ====================

#[tokio::test]
async fn test_mock_rule_engine_default() {
    let engine = MockRuleEngine::new();
    let context = json!({"test": true});

    let result = engine.evaluate("any-rule", &context).await;
    assert!(result.matched); // 默认匹配
}

#[tokio::test]
async fn test_mock_rule_engine_always_match() {
    let engine = MockRuleEngine::always_match();
    let context = json!({});

    let result = engine.evaluate("rule-1", &context).await;
    assert!(result.matched);

    let result = engine.evaluate("rule-2", &context).await;
    assert!(result.matched);
}

#[tokio::test]
async fn test_mock_rule_engine_never_match() {
    let engine = MockRuleEngine::never_match();
    let context = json!({});

    let result = engine.evaluate("rule-1", &context).await;
    assert!(!result.matched);

    let result = engine.evaluate("rule-2", &context).await;
    assert!(!result.matched);
}

#[tokio::test]
async fn test_mock_rule_engine_custom_result() {
    let engine = MockRuleEngine::new();

    let custom_result = MockEvaluationResult {
        matched: false,
        rule_id: "custom-rule".to_string(),
        rule_name: "Custom Rule".to_string(),
        matched_conditions: vec!["cond-1".to_string()],
        evaluation_time_ms: 10,
    };

    engine.set_rule_result("custom-rule", custom_result).await;

    let context = json!({});

    // 自定义结果
    let result = engine.evaluate("custom-rule", &context).await;
    assert!(!result.matched);
    assert_eq!(result.rule_id, "custom-rule");

    // 其他规则使用默认结果
    let result = engine.evaluate("other-rule", &context).await;
    assert!(result.matched);
}

#[tokio::test]
async fn test_mock_rule_engine_batch_evaluate() {
    let engine = MockRuleEngine::new();

    // 设置部分规则结果
    engine.set_rule_result("rule-1", MockEvaluationResult {
        matched: true,
        rule_id: "rule-1".to_string(),
        ..Default::default()
    }).await;

    engine.set_rule_result("rule-2", MockEvaluationResult {
        matched: false,
        rule_id: "rule-2".to_string(),
        ..Default::default()
    }).await;

    let context = json!({});
    let rule_ids = vec![
        "rule-1".to_string(),
        "rule-2".to_string(),
        "rule-3".to_string(),
    ];

    let results = engine.batch_evaluate(&rule_ids, &context).await;

    assert_eq!(results.len(), 3);
    assert!(results[0].matched);
    assert!(!results[1].matched);
    assert!(results[2].matched); // 默认结果
}

// ==================== Test Fixture 测试 ====================

#[test]
fn test_fixture_new() {
    let fixture = TestFixture::new();
    assert!(fixture.users.is_empty());
    assert!(fixture.badges.is_empty());
    assert!(fixture.rules.is_empty());
}

#[test]
fn test_fixture_with_user() {
    let fixture = TestFixture::new()
        .with_user("user-001")
        .with_user("user-002");

    assert_eq!(fixture.users.len(), 2);
    assert!(fixture.users.contains(&"user-001".to_string()));
    assert!(fixture.users.contains(&"user-002".to_string()));
}

#[test]
fn test_fixture_with_users() {
    let fixture = TestFixture::new().with_users(5);

    assert_eq!(fixture.users.len(), 5);
    // 验证用户 ID 都是唯一的
    let unique_count = fixture.users.iter().collect::<std::collections::HashSet<_>>().len();
    assert_eq!(unique_count, 5);
}

#[test]
fn test_fixture_with_badge() {
    let badge = TestDataGenerator::badge("Test Badge");
    let fixture = TestFixture::new().with_badge(badge);

    assert_eq!(fixture.badges.len(), 1);
    assert_eq!(fixture.badges[0].name, "Test Badge");
}

#[test]
fn test_fixture_with_rule() {
    let rule = TestDataGenerator::simple_rule("test-rule", "a", "eq", json!(1));
    let fixture = TestFixture::new().with_rule(rule);

    assert_eq!(fixture.rules.len(), 1);
    assert_eq!(fixture.rules[0]["id"], "test-rule");
}

#[test]
fn test_fixture_vip_purchase_scenario() {
    let fixture = TestFixture::vip_purchase_scenario();

    assert_eq!(fixture.users.len(), 3);
    assert_eq!(fixture.badges.len(), 1);
    assert_eq!(fixture.rules.len(), 1);

    // 验证规则结构
    let rule = &fixture.rules[0];
    assert_eq!(rule["root"]["type"], "group");
    assert_eq!(rule["root"]["operator"], "AND");
}

#[test]
fn test_fixture_first_purchase_scenario() {
    let fixture = TestFixture::first_purchase_scenario();

    assert_eq!(fixture.users.len(), 2);
    assert_eq!(fixture.badges.len(), 1);
    assert_eq!(fixture.rules.len(), 1);
}

// ==================== Test Assertions 测试 ====================

#[test]
fn test_assert_json_field_eq_success() {
    let json1 = json!({"name": "test", "value": 42});
    let json2 = json!({"name": "test", "other": 100});

    TestAssertions::assert_json_field_eq(&json1, &json2, "name");
}

#[test]
#[should_panic(expected = "Field 'value' mismatch")]
fn test_assert_json_field_eq_failure() {
    let json1 = json!({"value": 42});
    let json2 = json!({"value": 100});

    TestAssertions::assert_json_field_eq(&json1, &json2, "value");
}

#[test]
fn test_assert_json_has_field_success() {
    let json = json!({"name": "test", "nested": {"key": "value"}});

    TestAssertions::assert_json_has_field(&json, "name");
    TestAssertions::assert_json_has_field(&json, "nested");
}

#[test]
#[should_panic(expected = "Expected JSON to have field")]
fn test_assert_json_has_field_failure() {
    let json = json!({"name": "test"});

    TestAssertions::assert_json_has_field(&json, "missing");
}

#[test]
fn test_assert_time_within_success() {
    use chrono::{Duration, Utc};

    let now = Utc::now();
    let close_time = now + Duration::milliseconds(500);

    TestAssertions::assert_time_within(now, close_time, Duration::seconds(1));
}

#[test]
#[should_panic(expected = "Time difference")]
fn test_assert_time_within_failure() {
    use chrono::{Duration, Utc};

    let now = Utc::now();
    let far_time = now + Duration::hours(1);

    TestAssertions::assert_time_within(now, far_time, Duration::seconds(1));
}

// ==================== 辅助函数测试 ====================

#[test]
fn test_user_id_uniqueness() {
    let ids: Vec<String> = (0..100).map(|_| test_user_id()).collect();
    let unique_count = ids.iter().collect::<std::collections::HashSet<_>>().len();

    assert_eq!(unique_count, 100, "生成的用户 ID 应该唯一");
}

#[test]
fn test_badge_id_generation() {
    let id1 = test_badge_id();
    let id2 = test_badge_id();

    // 由于使用时间戳，短时间内可能相同，但应该是有效的正整数
    assert!(id1 > 0);
    assert!(id2 > 0);
}

#[test]
fn test_database_config_creation() {
    let config = test_database_config();

    assert!(config.url.contains("postgres://"));
    assert!(config.max_connections > 0);
    assert!(config.connect_timeout_seconds > 0);
}

#[test]
fn test_redis_config_creation() {
    let config = test_redis_config();

    assert!(config.url.contains("redis://"));
    assert!(config.pool_size > 0);
}
