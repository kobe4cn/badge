//! 规则执行器
//!
//! 实现规则的短路求值执行，返回匹配结果和评估追踪信息。

use crate::compiler::CompiledRule;
use crate::error::Result;
use crate::evaluator::ConditionEvaluator;
use crate::models::{Condition, EvaluationContext, EvaluationResult, LogicalGroup, RuleNode};
use crate::operators::LogicalOperator;
use std::time::Instant;

/// 规则执行器
pub struct RuleExecutor {
    /// 是否记录详细评估追踪
    trace_enabled: bool,
}

impl RuleExecutor {
    pub fn new() -> Self {
        Self {
            trace_enabled: false,
        }
    }

    /// 启用评估追踪
    pub fn with_trace(mut self) -> Self {
        self.trace_enabled = true;
        self
    }

    /// 执行规则评估
    pub fn execute(
        &self,
        rule: &CompiledRule,
        context: &EvaluationContext,
    ) -> Result<EvaluationResult> {
        let start = Instant::now();

        let mut result = EvaluationResult::new(rule.id().to_string(), rule.name().to_string());

        // 执行规则评估
        let matched = self.evaluate_node(rule.root(), context, &mut result, "root")?;

        result.matched = matched;
        result.evaluation_time_ms = start.elapsed().as_millis() as i64;

        Ok(result)
    }

    /// 递归评估规则节点
    fn evaluate_node(
        &self,
        node: &RuleNode,
        context: &EvaluationContext,
        result: &mut EvaluationResult,
        path: &str,
    ) -> Result<bool> {
        match node {
            RuleNode::Condition(cond) => self.evaluate_condition(cond, context, result, path),
            RuleNode::Group(group) => self.evaluate_group(group, context, result, path),
        }
    }

    /// 评估条件节点
    fn evaluate_condition(
        &self,
        cond: &Condition,
        context: &EvaluationContext,
        result: &mut EvaluationResult,
        path: &str,
    ) -> Result<bool> {
        let field_value = context.get_field(&cond.field);

        let matched = ConditionEvaluator::evaluate(field_value, cond.operator, &cond.value)?;

        if self.trace_enabled {
            result.evaluation_trace.push(format!(
                "{}: {} {} {} => {}",
                path,
                cond.field,
                cond.operator,
                cond.value,
                if matched { "MATCHED" } else { "NOT_MATCHED" }
            ));
        }

        if matched {
            result.matched_conditions.push(format!(
                "{}.{} {} {}",
                path, cond.field, cond.operator, cond.value
            ));
        }

        Ok(matched)
    }

    /// 评估逻辑组节点（短路求值）
    fn evaluate_group(
        &self,
        group: &LogicalGroup,
        context: &EvaluationContext,
        result: &mut EvaluationResult,
        path: &str,
    ) -> Result<bool> {
        if self.trace_enabled {
            result.evaluation_trace.push(format!(
                "{}: 开始评估 {} 组 (共 {} 个子节点)",
                path,
                group.operator,
                group.children.len()
            ));
        }

        match group.operator {
            LogicalOperator::And => {
                // AND: 所有条件都必须满足，遇到 false 立即返回
                for (i, child) in group.children.iter().enumerate() {
                    let child_path = format!("{}.children[{}]", path, i);
                    let child_matched = self.evaluate_node(child, context, result, &child_path)?;

                    if !child_matched {
                        if self.trace_enabled {
                            result
                                .evaluation_trace
                                .push(format!("{}: AND 短路 - 子节点 {} 不匹配", path, i));
                        }
                        return Ok(false);
                    }
                }

                if self.trace_enabled {
                    result
                        .evaluation_trace
                        .push(format!("{}: AND 组全部匹配", path));
                }
                Ok(true)
            }
            LogicalOperator::Or => {
                // OR: 任一条件满足即可，遇到 true 立即返回
                for (i, child) in group.children.iter().enumerate() {
                    let child_path = format!("{}.children[{}]", path, i);
                    let child_matched = self.evaluate_node(child, context, result, &child_path)?;

                    if child_matched {
                        if self.trace_enabled {
                            result
                                .evaluation_trace
                                .push(format!("{}: OR 短路 - 子节点 {} 匹配", path, i));
                        }
                        return Ok(true);
                    }
                }

                if self.trace_enabled {
                    result
                        .evaluation_trace
                        .push(format!("{}: OR 组无匹配", path));
                }
                Ok(false)
            }
        }
    }
}

impl Default for RuleExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::RuleCompiler;
    use serde_json::json;

    fn create_test_context() -> EvaluationContext {
        EvaluationContext::new(json!({
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
                "is_vip": true,
                "tags": ["vip", "frequent"]
            }
        }))
    }

    fn compile_rule(json: &str) -> CompiledRule {
        let mut compiler = RuleCompiler::new();
        compiler.compile_from_json(json).unwrap()
    }

    #[test]
    fn test_simple_condition_match() {
        let rule = compile_rule(
            r#"
            {
                "id": "rule-001",
                "name": "test",
                "version": "1.0",
                "root": {
                    "type": "condition",
                    "field": "event.type",
                    "operator": "eq",
                    "value": "PURCHASE"
                }
            }
            "#,
        );

        let context = create_test_context();
        let executor = RuleExecutor::new();
        let result = executor.execute(&rule, &context).unwrap();

        assert!(result.matched);
        assert_eq!(result.matched_conditions.len(), 1);
    }

    #[test]
    fn test_simple_condition_not_match() {
        let rule = compile_rule(
            r#"
            {
                "id": "rule-001",
                "name": "test",
                "version": "1.0",
                "root": {
                    "type": "condition",
                    "field": "event.type",
                    "operator": "eq",
                    "value": "REFUND"
                }
            }
            "#,
        );

        let context = create_test_context();
        let executor = RuleExecutor::new();
        let result = executor.execute(&rule, &context).unwrap();

        assert!(!result.matched);
    }

    #[test]
    fn test_and_group_all_match() {
        let rule = compile_rule(
            r#"
            {
                "id": "rule-001",
                "name": "test",
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
            "#,
        );

        let context = create_test_context();
        let executor = RuleExecutor::new();
        let result = executor.execute(&rule, &context).unwrap();

        assert!(result.matched);
        assert_eq!(result.matched_conditions.len(), 2);
    }

    #[test]
    fn test_and_group_short_circuit() {
        let rule = compile_rule(
            r#"
            {
                "id": "rule-001",
                "name": "test",
                "version": "1.0",
                "root": {
                    "type": "group",
                    "operator": "AND",
                    "children": [
                        {
                            "type": "condition",
                            "field": "event.type",
                            "operator": "eq",
                            "value": "REFUND"
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
            "#,
        );

        let context = create_test_context();
        let executor = RuleExecutor::new().with_trace();
        let result = executor.execute(&rule, &context).unwrap();

        assert!(!result.matched);
        // AND 短路：第一个条件不匹配，第二个不会评估
        assert!(result.evaluation_trace.iter().any(|t| t.contains("短路")));
    }

    #[test]
    fn test_or_group_first_match() {
        let rule = compile_rule(
            r#"
            {
                "id": "rule-001",
                "name": "test",
                "version": "1.0",
                "root": {
                    "type": "group",
                    "operator": "OR",
                    "children": [
                        {
                            "type": "condition",
                            "field": "event.type",
                            "operator": "eq",
                            "value": "PURCHASE"
                        },
                        {
                            "type": "condition",
                            "field": "event.type",
                            "operator": "eq",
                            "value": "REFUND"
                        }
                    ]
                }
            }
            "#,
        );

        let context = create_test_context();
        let executor = RuleExecutor::new().with_trace();
        let result = executor.execute(&rule, &context).unwrap();

        assert!(result.matched);
        // OR 短路：第一个条件匹配，第二个不会评估
        assert!(result.evaluation_trace.iter().any(|t| t.contains("短路")));
    }

    #[test]
    fn test_nested_groups() {
        let rule = compile_rule(
            r#"
            {
                "id": "rule-001",
                "name": "vip_purchase",
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
                            "type": "group",
                            "operator": "OR",
                            "children": [
                                {
                                    "type": "condition",
                                    "field": "order.amount",
                                    "operator": "gte",
                                    "value": 2000
                                },
                                {
                                    "type": "condition",
                                    "field": "user.is_vip",
                                    "operator": "eq",
                                    "value": true
                                }
                            ]
                        }
                    ]
                }
            }
            "#,
        );

        let context = create_test_context();
        let executor = RuleExecutor::new();
        let result = executor.execute(&rule, &context).unwrap();

        // PURCHASE + (amount >= 2000 OR is_vip)
        // amount=1000 < 2000, but is_vip=true
        assert!(result.matched);
    }

    #[test]
    fn test_trace_output() {
        let rule = compile_rule(
            r#"
            {
                "id": "rule-001",
                "name": "test",
                "version": "1.0",
                "root": {
                    "type": "condition",
                    "field": "event.type",
                    "operator": "eq",
                    "value": "PURCHASE"
                }
            }
            "#,
        );

        let context = create_test_context();
        let executor = RuleExecutor::new().with_trace();
        let result = executor.execute(&rule, &context).unwrap();

        assert!(!result.evaluation_trace.is_empty());
        assert!(result.evaluation_trace[0].contains("MATCHED"));
    }

    #[test]
    fn test_array_contains() {
        let rule = compile_rule(
            r#"
            {
                "id": "rule-001",
                "name": "test",
                "version": "1.0",
                "root": {
                    "type": "condition",
                    "field": "user.tags",
                    "operator": "contains",
                    "value": "vip"
                }
            }
            "#,
        );

        let context = create_test_context();
        let executor = RuleExecutor::new();
        let result = executor.execute(&rule, &context).unwrap();

        assert!(result.matched);
    }

    #[test]
    fn test_evaluation_time() {
        let rule = compile_rule(
            r#"
            {
                "id": "rule-001",
                "name": "test",
                "version": "1.0",
                "root": {
                    "type": "condition",
                    "field": "event.type",
                    "operator": "eq",
                    "value": "PURCHASE"
                }
            }
            "#,
        );

        let context = create_test_context();
        let executor = RuleExecutor::new();
        let result = executor.execute(&rule, &context).unwrap();

        // 应该记录评估时间
        assert!(result.evaluation_time_ms >= 0);
    }
}
