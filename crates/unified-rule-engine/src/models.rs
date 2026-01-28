//! 规则引擎领域模型

use crate::operators::{LogicalOperator, Operator};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// 规则定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub version: String,
    pub root: RuleNode,
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
}

impl Rule {
    pub fn new(name: impl Into<String>, root: RuleNode) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            version: "1.0".to_string(),
            root,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

/// 规则节点（条件或逻辑组）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuleNode {
    Condition(Condition),
    Group(LogicalGroup),
}

/// 条件节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub field: String,
    pub operator: Operator,
    pub value: Value,
}

impl Condition {
    pub fn new(field: impl Into<String>, operator: Operator, value: impl Into<Value>) -> Self {
        Self {
            field: field.into(),
            operator,
            value: value.into(),
        }
    }
}

/// 逻辑组节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicalGroup {
    pub operator: LogicalOperator,
    pub children: Vec<RuleNode>,
}

impl LogicalGroup {
    pub fn new(operator: LogicalOperator, children: Vec<RuleNode>) -> Self {
        Self { operator, children }
    }

    pub fn and(children: Vec<RuleNode>) -> Self {
        Self::new(LogicalOperator::And, children)
    }

    pub fn or(children: Vec<RuleNode>) -> Self {
        Self::new(LogicalOperator::Or, children)
    }
}

/// 评估上下文 - 提供给规则引擎的数据
#[derive(Debug, Clone, Default)]
pub struct EvaluationContext {
    data: Value,
}

impl EvaluationContext {
    pub fn new(data: Value) -> Self {
        Self { data }
    }

    /// 从 JSON 对象创建
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let data: Value = serde_json::from_str(json)?;
        Ok(Self { data })
    }

    /// 获取字段值（支持点号分隔的路径，如 "event.type" 或 "user.profile.age"）
    pub fn get_field(&self, path: &str) -> Option<&Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = &self.data;

        for part in parts {
            match current {
                Value::Object(map) => {
                    current = map.get(part)?;
                }
                Value::Array(arr) => {
                    // 支持数组索引访问，如 "items.0.name"
                    let index: usize = part.parse().ok()?;
                    current = arr.get(index)?;
                }
                _ => return None,
            }
        }

        Some(current)
    }

    /// 获取底层数据
    pub fn data(&self) -> &Value {
        &self.data
    }
}

/// 评估结果
#[derive(Debug, Clone, Serialize)]
pub struct EvaluationResult {
    pub matched: bool,
    pub rule_id: String,
    pub rule_name: String,
    pub matched_conditions: Vec<String>,
    pub evaluation_trace: Vec<String>,
    pub evaluation_time_ms: i64,
}

impl EvaluationResult {
    pub fn new(rule_id: String, rule_name: String) -> Self {
        Self {
            matched: false,
            rule_id,
            rule_name,
            matched_conditions: Vec::new(),
            evaluation_trace: Vec::new(),
            evaluation_time_ms: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_rule_serialization() {
        let rule = Rule::new(
            "test_rule",
            RuleNode::Group(LogicalGroup::and(vec![
                RuleNode::Condition(Condition::new("event.type", Operator::Eq, "PURCHASE")),
                RuleNode::Condition(Condition::new("order.amount", Operator::Gte, 500)),
            ])),
        );

        let json = serde_json::to_string_pretty(&rule).unwrap();
        println!("{}", json);

        let parsed: Rule = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "test_rule");
    }

    #[test]
    fn test_rule_deserialization() {
        let json = r#"
        {
            "id": "rule-001",
            "name": "purchase_badge",
            "version": "1.0",
            "root": {
                "type": "group",
                "operator": "AND",
                "children": [
                    {
                        "type": "condition",
                        "field": "event.type",
                        "operator": "eq",
                        "value": "PURCHASE"
                    },
                    {
                        "type": "condition",
                        "field": "order.amount",
                        "operator": "gte",
                        "value": 500
                    }
                ]
            }
        }
        "#;

        let rule: Rule = serde_json::from_str(json).unwrap();
        assert_eq!(rule.id, "rule-001");
        assert_eq!(rule.name, "purchase_badge");
    }

    #[test]
    fn test_evaluation_context() {
        let ctx = EvaluationContext::new(json!({
            "event": {
                "type": "PURCHASE",
                "timestamp": "2024-01-15T10:00:00Z"
            },
            "order": {
                "amount": 1000,
                "items": [
                    {"name": "ticket", "price": 500},
                    {"name": "food", "price": 500}
                ]
            },
            "user": {
                "id": "user-123",
                "is_vip": true
            }
        }));

        assert_eq!(ctx.get_field("event.type"), Some(&json!("PURCHASE")));
        assert_eq!(ctx.get_field("order.amount"), Some(&json!(1000)));
        assert_eq!(ctx.get_field("user.is_vip"), Some(&json!(true)));
        assert_eq!(ctx.get_field("order.items.0.name"), Some(&json!("ticket")));
        assert_eq!(ctx.get_field("nonexistent"), None);
    }
}
