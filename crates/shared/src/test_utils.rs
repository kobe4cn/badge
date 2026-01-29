//! 测试工具模块
//!
//! 提供集成测试所需的辅助函数、Mock 实现和测试数据生成器。
//! 用于简化测试代码编写，提高测试的可重复性和可维护性。

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::config::{DatabaseConfig, RedisConfig};

// ==================== 测试配置辅助 ====================

/// 创建测试用数据库配置
///
/// 优先使用环境变量，否则使用默认测试数据库
pub fn test_database_config() -> DatabaseConfig {
    DatabaseConfig {
        url: std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://badge:badge_secret@localhost:5432/badge_test".to_string()),
        max_connections: 5,
        min_connections: 1,
        connect_timeout_seconds: 10,
        idle_timeout_seconds: 300,
    }
}

/// 创建测试用 Redis 配置
pub fn test_redis_config() -> RedisConfig {
    RedisConfig {
        url: std::env::var("TEST_REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379/1".to_string()),
        pool_size: 5,
    }
}

/// 生成唯一的测试用户 ID
pub fn test_user_id() -> String {
    format!("test-user-{}", Uuid::new_v4())
}

/// 生成唯一的测试徽章 ID
///
/// 使用原子计数器确保并行测试时的唯一性
pub fn test_badge_id() -> i64 {
    use std::sync::atomic::{AtomicI64, Ordering};
    static COUNTER: AtomicI64 = AtomicI64::new(0);
    let base = Utc::now().timestamp_micros() % 1_000_000_000;
    base + COUNTER.fetch_add(1, Ordering::SeqCst)
}

// ==================== Mock 规则引擎 ====================

/// Mock 规则评估结果
#[derive(Debug, Clone)]
pub struct MockEvaluationResult {
    pub matched: bool,
    pub rule_id: String,
    pub rule_name: String,
    pub matched_conditions: Vec<String>,
    pub evaluation_time_ms: i64,
}

impl Default for MockEvaluationResult {
    fn default() -> Self {
        Self {
            matched: true,
            rule_id: "mock-rule".to_string(),
            rule_name: "Mock Rule".to_string(),
            matched_conditions: vec!["condition-1".to_string()],
            evaluation_time_ms: 1,
        }
    }
}

/// Mock 规则引擎
///
/// 用于无外部依赖的单元测试，预设规则评估结果
#[derive(Debug, Default)]
pub struct MockRuleEngine {
    rules: Arc<RwLock<HashMap<String, MockEvaluationResult>>>,
    default_result: MockEvaluationResult,
}

impl MockRuleEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建总是返回匹配的 Mock 引擎
    pub fn always_match() -> Self {
        Self {
            default_result: MockEvaluationResult {
                matched: true,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// 创建总是返回不匹配的 Mock 引擎
    pub fn never_match() -> Self {
        Self {
            default_result: MockEvaluationResult {
                matched: false,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// 预设特定规则的评估结果
    pub async fn set_rule_result(&self, rule_id: &str, result: MockEvaluationResult) {
        let mut rules = self.rules.write().await;
        rules.insert(rule_id.to_string(), result);
    }

    /// 评估规则
    pub async fn evaluate(&self, rule_id: &str, _context: &Value) -> MockEvaluationResult {
        let rules = self.rules.read().await;
        rules
            .get(rule_id)
            .cloned()
            .unwrap_or_else(|| self.default_result.clone())
    }

    /// 批量评估规则
    pub async fn batch_evaluate(
        &self,
        rule_ids: &[String],
        context: &Value,
    ) -> Vec<MockEvaluationResult> {
        let mut results = Vec::with_capacity(rule_ids.len());
        for rule_id in rule_ids {
            results.push(self.evaluate(rule_id, context).await);
        }
        results
    }
}

// ==================== 测试数据生成器 ====================

/// 测试数据生成器
///
/// 提供生成测试用徽章、用户、事件等数据的便捷方法
pub struct TestDataGenerator;

impl TestDataGenerator {
    /// 生成测试用徽章数据
    pub fn badge(name: &str) -> BadgeTestData {
        BadgeTestData {
            id: test_badge_id(),
            name: name.to_string(),
            description: format!("Test badge: {}", name),
            badge_type: "NORMAL".to_string(),
            status: "ACTIVE".to_string(),
            max_supply: None,
            issued_count: 0,
            assets: json!({
                "iconUrl": format!("https://example.com/badges/{}.png", name.to_lowercase().replace(' ', "-")),
                "imageUrl": null,
                "animationUrl": null
            }),
            validity_config: json!({
                "validityType": "PERMANENT"
            }),
        }
    }

    /// 生成限量徽章数据
    pub fn limited_badge(name: &str, max_supply: i64) -> BadgeTestData {
        let mut badge = Self::badge(name);
        badge.max_supply = Some(max_supply);
        badge.badge_type = "LIMITED".to_string();
        badge
    }

    /// 生成有时效的徽章数据
    pub fn timed_badge(name: &str, days: i32) -> BadgeTestData {
        let mut badge = Self::badge(name);
        badge.validity_config = json!({
            "validityType": "RELATIVE_DAYS",
            "relativeDays": days
        });
        badge
    }

    /// 生成测试用户徽章数据
    pub fn user_badge(user_id: &str, badge_id: i64) -> UserBadgeTestData {
        UserBadgeTestData {
            id: 0, // 由数据库生成
            user_id: user_id.to_string(),
            badge_id,
            status: "ACTIVE".to_string(),
            quantity: 1,
            acquired_at: Utc::now(),
            expires_at: None,
        }
    }

    /// 生成购买事件上下文
    pub fn purchase_event(user_id: &str, amount: i64) -> Value {
        json!({
            "event": {
                "type": "PURCHASE",
                "timestamp": Utc::now().to_rfc3339(),
                "source": "test"
            },
            "order": {
                "id": format!("order-{}", Uuid::new_v4()),
                "amount": amount,
                "currency": "CNY",
                "category": "general"
            },
            "user": {
                "id": user_id,
                "level": "standard",
                "is_vip": false,
                "tags": []
            }
        })
    }

    /// 生成 VIP 购买事件上下文
    pub fn vip_purchase_event(user_id: &str, amount: i64) -> Value {
        json!({
            "event": {
                "type": "PURCHASE",
                "timestamp": Utc::now().to_rfc3339(),
                "source": "test"
            },
            "order": {
                "id": format!("order-{}", Uuid::new_v4()),
                "amount": amount,
                "currency": "CNY",
                "category": "premium"
            },
            "user": {
                "id": user_id,
                "level": "gold",
                "is_vip": true,
                "tags": ["vip", "premium_member"]
            }
        })
    }

    /// 生成互动事件上下文
    pub fn engagement_event(user_id: &str, event_type: &str) -> Value {
        json!({
            "event": {
                "type": event_type,
                "timestamp": Utc::now().to_rfc3339(),
                "source": "test"
            },
            "user": {
                "id": user_id,
                "level": "standard"
            },
            "activity": {
                "type": event_type.to_lowercase(),
                "count": 1
            }
        })
    }

    /// 生成规则 JSON
    pub fn simple_rule(rule_id: &str, field: &str, operator: &str, value: Value) -> Value {
        json!({
            "id": rule_id,
            "name": format!("Rule: {}", rule_id),
            "version": "1.0",
            "root": {
                "type": "condition",
                "field": field,
                "operator": operator,
                "value": value
            }
        })
    }

    /// 生成复合规则 JSON (AND)
    pub fn and_rule(rule_id: &str, conditions: Vec<Value>) -> Value {
        json!({
            "id": rule_id,
            "name": format!("Rule: {}", rule_id),
            "version": "1.0",
            "root": {
                "type": "group",
                "operator": "AND",
                "children": conditions
            }
        })
    }

    /// 生成复合规则 JSON (OR)
    pub fn or_rule(rule_id: &str, conditions: Vec<Value>) -> Value {
        json!({
            "id": rule_id,
            "name": format!("Rule: {}", rule_id),
            "version": "1.0",
            "root": {
                "type": "group",
                "operator": "OR",
                "children": conditions
            }
        })
    }

    /// 生成条件节点 JSON
    pub fn condition(field: &str, operator: &str, value: Value) -> Value {
        json!({
            "type": "condition",
            "field": field,
            "operator": operator,
            "value": value
        })
    }
}

/// 徽章测试数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgeTestData {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub badge_type: String,
    pub status: String,
    pub max_supply: Option<i64>,
    pub issued_count: i64,
    pub assets: Value,
    pub validity_config: Value,
}

/// 用户徽章测试数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserBadgeTestData {
    pub id: i64,
    pub user_id: String,
    pub badge_id: i64,
    pub status: String,
    pub quantity: i32,
    pub acquired_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

// ==================== 断言辅助 ====================

/// 测试断言辅助结构
pub struct TestAssertions;

impl TestAssertions {
    /// 断言两个 JSON 值在指定字段上相等
    pub fn assert_json_field_eq(actual: &Value, expected: &Value, field: &str) {
        let actual_val = actual.get(field);
        let expected_val = expected.get(field);
        assert_eq!(
            actual_val, expected_val,
            "Field '{}' mismatch: actual={:?}, expected={:?}",
            field, actual_val, expected_val
        );
    }

    /// 断言 JSON 包含指定字段
    pub fn assert_json_has_field(value: &Value, field: &str) {
        assert!(
            value.get(field).is_some(),
            "Expected JSON to have field '{}', but it was missing. Value: {:?}",
            field,
            value
        );
    }

    /// 断言时间在指定范围内
    pub fn assert_time_within(actual: DateTime<Utc>, expected: DateTime<Utc>, tolerance: Duration) {
        let diff = if actual > expected {
            actual - expected
        } else {
            expected - actual
        };
        assert!(
            diff < tolerance,
            "Time difference {:?} exceeds tolerance {:?}. Actual: {}, Expected: {}",
            diff,
            tolerance,
            actual,
            expected
        );
    }
}

// ==================== 测试 Fixture ====================

/// 测试 Fixture 构建器
///
/// 用于快速构建测试场景所需的数据结构
pub struct TestFixture {
    pub users: Vec<String>,
    pub badges: Vec<BadgeTestData>,
    pub rules: Vec<Value>,
}

impl TestFixture {
    pub fn new() -> Self {
        Self {
            users: Vec::new(),
            badges: Vec::new(),
            rules: Vec::new(),
        }
    }

    /// 添加测试用户
    pub fn with_user(mut self, user_id: &str) -> Self {
        self.users.push(user_id.to_string());
        self
    }

    /// 添加多个测试用户
    pub fn with_users(mut self, count: usize) -> Self {
        for _ in 0..count {
            self.users.push(test_user_id());
        }
        self
    }

    /// 添加测试徽章
    pub fn with_badge(mut self, badge: BadgeTestData) -> Self {
        self.badges.push(badge);
        self
    }

    /// 添加测试规则
    pub fn with_rule(mut self, rule: Value) -> Self {
        self.rules.push(rule);
        self
    }

    /// 构建标准测试场景：VIP 高额消费徽章
    pub fn vip_purchase_scenario() -> Self {
        let badge = TestDataGenerator::badge("VIP Purchase Badge");
        let rule = TestDataGenerator::and_rule(
            "vip-purchase-rule",
            vec![
                TestDataGenerator::condition("event.type", "eq", json!("PURCHASE")),
                TestDataGenerator::condition("user.is_vip", "eq", json!(true)),
                TestDataGenerator::condition("order.amount", "gte", json!(1000)),
            ],
        );

        Self::new()
            .with_users(3)
            .with_badge(badge)
            .with_rule(rule)
    }

    /// 构建标准测试场景：首次购买徽章
    pub fn first_purchase_scenario() -> Self {
        let badge = TestDataGenerator::badge("First Purchase Badge");
        let rule = TestDataGenerator::simple_rule(
            "first-purchase-rule",
            "event.type",
            "eq",
            json!("PURCHASE"),
        );

        Self::new()
            .with_users(2)
            .with_badge(badge)
            .with_rule(rule)
    }
}

impl Default for TestFixture {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_user_id_uniqueness() {
        let id1 = test_user_id();
        let id2 = test_user_id();
        assert_ne!(id1, id2, "Generated user IDs should be unique");
    }

    #[test]
    fn test_badge_test_data_generation() {
        let badge = TestDataGenerator::badge("Test Badge");
        assert_eq!(badge.name, "Test Badge");
        assert_eq!(badge.status, "ACTIVE");
        assert!(badge.max_supply.is_none());
    }

    #[test]
    fn test_limited_badge_generation() {
        let badge = TestDataGenerator::limited_badge("Limited Badge", 100);
        assert_eq!(badge.max_supply, Some(100));
        assert_eq!(badge.badge_type, "LIMITED");
    }

    #[test]
    fn test_purchase_event_generation() {
        let event = TestDataGenerator::purchase_event("user-123", 500);
        assert_eq!(event["event"]["type"], "PURCHASE");
        assert_eq!(event["order"]["amount"], 500);
        assert_eq!(event["user"]["id"], "user-123");
    }

    #[test]
    fn test_simple_rule_generation() {
        let rule = TestDataGenerator::simple_rule("test-rule", "event.type", "eq", json!("PURCHASE"));
        assert_eq!(rule["id"], "test-rule");
        assert_eq!(rule["root"]["type"], "condition");
        assert_eq!(rule["root"]["field"], "event.type");
    }

    #[test]
    fn test_and_rule_generation() {
        let conditions = vec![
            TestDataGenerator::condition("a", "eq", json!(1)),
            TestDataGenerator::condition("b", "gt", json!(10)),
        ];
        let rule = TestDataGenerator::and_rule("and-rule", conditions);
        assert_eq!(rule["root"]["type"], "group");
        assert_eq!(rule["root"]["operator"], "AND");
        assert_eq!(rule["root"]["children"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_mock_rule_engine_always_match() {
        let engine = MockRuleEngine::always_match();
        let context = json!({"test": true});
        let result = engine.evaluate("any-rule", &context).await;
        assert!(result.matched);
    }

    #[tokio::test]
    async fn test_mock_rule_engine_never_match() {
        let engine = MockRuleEngine::never_match();
        let context = json!({"test": true});
        let result = engine.evaluate("any-rule", &context).await;
        assert!(!result.matched);
    }

    #[tokio::test]
    async fn test_mock_rule_engine_preset_result() {
        let engine = MockRuleEngine::new();
        let custom_result = MockEvaluationResult {
            matched: true,
            rule_id: "custom-rule".to_string(),
            rule_name: "Custom Rule".to_string(),
            matched_conditions: vec!["cond-a".to_string(), "cond-b".to_string()],
            evaluation_time_ms: 5,
        };
        engine.set_rule_result("custom-rule", custom_result.clone()).await;

        let context = json!({});
        let result = engine.evaluate("custom-rule", &context).await;
        assert_eq!(result.rule_id, "custom-rule");
        assert_eq!(result.matched_conditions.len(), 2);
    }

    #[test]
    fn test_fixture_vip_purchase_scenario() {
        let fixture = TestFixture::vip_purchase_scenario();
        assert_eq!(fixture.users.len(), 3);
        assert_eq!(fixture.badges.len(), 1);
        assert_eq!(fixture.rules.len(), 1);
    }

    #[test]
    fn test_json_assertions() {
        let json1 = json!({"name": "test", "value": 42});
        let json2 = json!({"name": "test", "value": 100});

        TestAssertions::assert_json_field_eq(&json1, &json2, "name");
        TestAssertions::assert_json_has_field(&json1, "value");
    }

    #[test]
    fn test_time_assertions() {
        let now = Utc::now();
        let close_time = now + Duration::milliseconds(100);
        TestAssertions::assert_time_within(now, close_time, Duration::seconds(1));
    }
}
