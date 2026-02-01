//! 数据生成器
//!
//! 用于生成随机或批量测试数据。

use rand::Rng;
use uuid::Uuid;

/// 用户数据生成器
pub struct UserGenerator;

impl UserGenerator {
    /// 生成测试用户 ID
    pub fn user_id() -> String {
        format!("test_user_{}", Uuid::now_v7())
    }

    /// 生成指定前缀的用户 ID
    pub fn user_id_with_prefix(prefix: &str) -> String {
        format!("test_{}_{}", prefix, Uuid::now_v7())
    }

    /// 批量生成用户 ID
    pub fn batch_user_ids(count: usize) -> Vec<String> {
        (0..count).map(|_| Self::user_id()).collect()
    }

    /// 生成 SWID 格式的用户 ID
    pub fn swid() -> String {
        format!("SWID-{}", Uuid::now_v7().to_string().to_uppercase())
    }
}

/// 订单数据生成器
pub struct OrderGenerator;

impl OrderGenerator {
    /// 生成订单 ID
    pub fn order_id() -> String {
        format!("test_order_{}", Uuid::now_v7())
    }

    /// 生成随机金额 (1-10000)
    pub fn random_amount() -> i64 {
        rand::rng().random_range(1..=10000)
    }

    /// 生成指定范围的金额
    pub fn amount_in_range(min: i64, max: i64) -> i64 {
        rand::rng().random_range(min..=max)
    }
}

/// 事件数据生成器
pub struct EventGenerator;

impl EventGenerator {
    /// 生成事件 ID
    pub fn event_id() -> String {
        Uuid::now_v7().to_string()
    }

    /// 生成购买事件数据
    pub fn purchase_event_data(amount: i64) -> serde_json::Value {
        serde_json::json!({
            "event_id": Self::event_id(),
            "event_type": "purchase",
            "user_id": UserGenerator::user_id(),
            "order_id": OrderGenerator::order_id(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "amount": amount,
            "currency": "CNY",
            "items": []
        })
    }

    /// 生成签到事件数据
    pub fn checkin_event_data(consecutive_days: i32) -> serde_json::Value {
        serde_json::json!({
            "event_id": Self::event_id(),
            "event_type": "checkin",
            "user_id": UserGenerator::user_id(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "consecutive_days": consecutive_days
        })
    }

    /// 生成分享事件数据
    pub fn share_event_data() -> serde_json::Value {
        serde_json::json!({
            "event_id": Self::event_id(),
            "event_type": "share",
            "user_id": UserGenerator::user_id(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "content_id": format!("content_{}", Uuid::new_v4()),
            "platform": "wechat"
        })
    }
}

/// 规则 JSON 生成器
pub struct RuleJsonGenerator;

impl RuleJsonGenerator {
    /// 简单条件
    pub fn simple_condition(
        field: &str,
        operator: &str,
        value: impl Into<serde_json::Value>,
    ) -> serde_json::Value {
        serde_json::json!({
            "type": "condition",
            "field": field,
            "operator": operator,
            "value": value.into()
        })
    }

    /// AND 组合
    pub fn and_group(conditions: Vec<serde_json::Value>) -> serde_json::Value {
        serde_json::json!({
            "type": "group",
            "operator": "AND",
            "children": conditions
        })
    }

    /// OR 组合
    pub fn or_group(conditions: Vec<serde_json::Value>) -> serde_json::Value {
        serde_json::json!({
            "type": "group",
            "operator": "OR",
            "children": conditions
        })
    }

    /// 嵌套规则
    pub fn nested(depth: usize, breadth: usize) -> serde_json::Value {
        Self::build_nested(depth, breadth, 0)
    }

    fn build_nested(depth: usize, breadth: usize, level: usize) -> serde_json::Value {
        if depth == 0 {
            Self::simple_condition(
                &format!("field_{}_{}", level, depth),
                "eq",
                format!("value_{}_{}", level, depth),
            )
        } else {
            let operator = if depth % 2 == 0 { "AND" } else { "OR" };
            let children: Vec<_> = (0..breadth)
                .map(|i| Self::build_nested(depth - 1, breadth, i))
                .collect();

            serde_json::json!({
                "type": "group",
                "operator": operator,
                "children": children
            })
        }
    }
}

/// 批量数据生成器
pub struct BatchGenerator;

impl BatchGenerator {
    /// 生成批量购买事件
    pub fn purchase_events(count: usize, amount_range: (i64, i64)) -> Vec<serde_json::Value> {
        let mut rng = rand::rng();
        (0..count)
            .map(|_| {
                let amount = rng.random_range(amount_range.0..=amount_range.1);
                EventGenerator::purchase_event_data(amount)
            })
            .collect()
    }

    /// 生成批量用户徽章场景
    pub fn user_badge_scenarios(user_count: usize) -> Vec<(String, Vec<serde_json::Value>)> {
        (0..user_count)
            .map(|_| {
                let user_id = UserGenerator::user_id();
                let events = vec![
                    EventGenerator::purchase_event_data(OrderGenerator::random_amount()),
                    EventGenerator::checkin_event_data(1),
                    EventGenerator::share_event_data(),
                ];
                (user_id, events)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_generator() {
        let user_id = UserGenerator::user_id();
        assert!(user_id.starts_with("test_user_"));

        let batch = UserGenerator::batch_user_ids(10);
        assert_eq!(batch.len(), 10);
    }

    #[test]
    fn test_rule_json_generator() {
        let condition = RuleJsonGenerator::simple_condition("amount", "gte", 1000);
        assert_eq!(condition["type"], "condition");
        assert_eq!(condition["field"], "amount");

        let and_group = RuleJsonGenerator::and_group(vec![
            RuleJsonGenerator::simple_condition("a", "eq", 1),
            RuleJsonGenerator::simple_condition("b", "eq", 2),
        ]);
        assert_eq!(and_group["operator"], "AND");
        assert_eq!(and_group["children"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_nested_rule() {
        let nested = RuleJsonGenerator::nested(2, 2);
        assert_eq!(nested["type"], "group");
    }
}
