//! 规则编译器
//!
//! 将 JSON 规则解析并编译成内存中的执行树，支持字段索引预提取优化。

use crate::error::{Result, RuleError};
use crate::models::{Condition, Rule, RuleNode};
use crate::operators::Operator;
use serde_json::Value;
use std::collections::HashSet;

/// 编译后的规则
#[derive(Debug, Clone)]
pub struct CompiledRule {
    /// 原始规则
    pub rule: Rule,
    /// 规则中使用的所有字段路径（用于优化字段提取）
    pub required_fields: HashSet<String>,
    /// 编译版本号（用于缓存失效）
    pub compile_version: u64,
}

impl CompiledRule {
    /// 获取规则 ID
    pub fn id(&self) -> &str {
        &self.rule.id
    }

    /// 获取规则名称
    pub fn name(&self) -> &str {
        &self.rule.name
    }

    /// 获取根节点
    pub fn root(&self) -> &RuleNode {
        &self.rule.root
    }
}

/// 规则编译器
pub struct RuleCompiler {
    compile_version: u64,
}

impl RuleCompiler {
    pub fn new() -> Self {
        Self { compile_version: 0 }
    }

    /// 从 JSON 字符串编译规则
    pub fn compile_from_json(&mut self, json: &str) -> Result<CompiledRule> {
        let rule: Rule = serde_json::from_str(json)?;
        self.compile(rule)
    }

    /// 编译规则
    pub fn compile(&mut self, rule: Rule) -> Result<CompiledRule> {
        // 验证规则结构
        self.validate_rule(&rule)?;

        // 提取所有使用的字段
        let required_fields = self.extract_fields(&rule.root);

        self.compile_version += 1;

        Ok(CompiledRule {
            rule,
            required_fields,
            compile_version: self.compile_version,
        })
    }

    /// 验证规则结构
    fn validate_rule(&self, rule: &Rule) -> Result<()> {
        if rule.id.is_empty() {
            return Err(RuleError::ParseError("规则 ID 不能为空".to_string()));
        }

        if rule.name.is_empty() {
            return Err(RuleError::ParseError("规则名称不能为空".to_string()));
        }

        self.validate_node(&rule.root, "root")?;

        Ok(())
    }

    /// 验证规则节点
    fn validate_node(&self, node: &RuleNode, path: &str) -> Result<()> {
        match node {
            RuleNode::Condition(cond) => {
                self.validate_condition(cond, path)?;
            }
            RuleNode::Group(group) => {
                if group.children.is_empty() {
                    return Err(RuleError::ParseError(format!(
                        "逻辑组 '{}' 不能为空",
                        path
                    )));
                }

                for (i, child) in group.children.iter().enumerate() {
                    let child_path = format!("{}.children[{}]", path, i);
                    self.validate_node(child, &child_path)?;
                }
            }
        }

        Ok(())
    }

    /// 验证条件
    fn validate_condition(&self, cond: &Condition, path: &str) -> Result<()> {
        if cond.field.is_empty() {
            return Err(RuleError::ParseError(format!(
                "条件 '{}' 的字段不能为空",
                path
            )));
        }

        // 验证操作符和值的兼容性
        self.validate_operator_value(cond, path)?;

        Ok(())
    }

    /// 验证操作符和值的兼容性
    fn validate_operator_value(&self, cond: &Condition, path: &str) -> Result<()> {
        match cond.operator {
            Operator::Between => {
                if let Value::Array(arr) = &cond.value {
                    if arr.len() != 2 {
                        return Err(RuleError::ParseError(format!(
                            "条件 '{}' 的 between 操作符需要 [min, max] 数组，当前有 {} 个元素",
                            path,
                            arr.len()
                        )));
                    }
                } else {
                    return Err(RuleError::ParseError(format!(
                        "条件 '{}' 的 between 操作符需要 [min, max] 数组",
                        path
                    )));
                }
            }
            Operator::In | Operator::NotIn | Operator::ContainsAny | Operator::ContainsAll => {
                if !cond.value.is_array() {
                    return Err(RuleError::ParseError(format!(
                        "条件 '{}' 的 {} 操作符需要数组值",
                        path, cond.operator
                    )));
                }
            }
            Operator::Regex => {
                if let Some(pattern) = cond.value.as_str() {
                    // 预验证正则表达式
                    regex::Regex::new(pattern).map_err(|e| {
                        RuleError::ParseError(format!(
                            "条件 '{}' 的正则表达式无效: {}",
                            path, e
                        ))
                    })?;
                } else {
                    return Err(RuleError::ParseError(format!(
                        "条件 '{}' 的 regex 操作符需要字符串值",
                        path
                    )));
                }
            }
            Operator::IsEmpty | Operator::IsNotEmpty => {
                // 这些操作符不需要值
            }
            _ => {
                // 其他操作符不做特殊验证
            }
        }

        Ok(())
    }

    /// 提取规则中使用的所有字段
    fn extract_fields(&self, node: &RuleNode) -> HashSet<String> {
        let mut fields = HashSet::new();
        self.collect_fields(node, &mut fields);
        fields
    }

    /// 递归收集字段
    fn collect_fields(&self, node: &RuleNode, fields: &mut HashSet<String>) {
        match node {
            RuleNode::Condition(cond) => {
                fields.insert(cond.field.clone());
            }
            RuleNode::Group(group) => {
                for child in &group.children {
                    self.collect_fields(child, fields);
                }
            }
        }
    }
}

impl Default for RuleCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_rule_json() -> &'static str {
        r#"
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
                        "type": "group",
                        "operator": "OR",
                        "children": [
                            {
                                "type": "condition",
                                "field": "order.amount",
                                "operator": "gte",
                                "value": 500
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
        "#
    }

    #[test]
    fn test_compile_from_json() {
        let mut compiler = RuleCompiler::new();
        let compiled = compiler.compile_from_json(sample_rule_json()).unwrap();

        assert_eq!(compiled.id(), "rule-001");
        assert_eq!(compiled.name(), "purchase_badge");
        assert_eq!(compiled.required_fields.len(), 3);
        assert!(compiled.required_fields.contains("event.type"));
        assert!(compiled.required_fields.contains("order.amount"));
        assert!(compiled.required_fields.contains("user.is_vip"));
    }

    #[test]
    fn test_compile_version() {
        let mut compiler = RuleCompiler::new();

        let compiled1 = compiler.compile_from_json(sample_rule_json()).unwrap();
        let compiled2 = compiler.compile_from_json(sample_rule_json()).unwrap();

        assert_eq!(compiled1.compile_version, 1);
        assert_eq!(compiled2.compile_version, 2);
    }

    #[test]
    fn test_validate_empty_id() {
        let mut compiler = RuleCompiler::new();
        let json = r#"
        {
            "id": "",
            "name": "test",
            "version": "1.0",
            "root": {
                "type": "condition",
                "field": "a",
                "operator": "eq",
                "value": 1
            }
        }
        "#;

        let result = compiler.compile_from_json(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("规则 ID 不能为空"));
    }

    #[test]
    fn test_validate_empty_group() {
        let mut compiler = RuleCompiler::new();
        let json = r#"
        {
            "id": "rule-001",
            "name": "test",
            "version": "1.0",
            "root": {
                "type": "group",
                "operator": "AND",
                "children": []
            }
        }
        "#;

        let result = compiler.compile_from_json(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("不能为空"));
    }

    #[test]
    fn test_validate_between_operator() {
        let mut compiler = RuleCompiler::new();
        let json = r#"
        {
            "id": "rule-001",
            "name": "test",
            "version": "1.0",
            "root": {
                "type": "condition",
                "field": "amount",
                "operator": "between",
                "value": [100, 500]
            }
        }
        "#;

        let result = compiler.compile_from_json(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_between_invalid() {
        let mut compiler = RuleCompiler::new();
        let json = r#"
        {
            "id": "rule-001",
            "name": "test",
            "version": "1.0",
            "root": {
                "type": "condition",
                "field": "amount",
                "operator": "between",
                "value": 100
            }
        }
        "#;

        let result = compiler.compile_from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_regex() {
        let mut compiler = RuleCompiler::new();
        let json = r#"
        {
            "id": "rule-001",
            "name": "test",
            "version": "1.0",
            "root": {
                "type": "condition",
                "field": "email",
                "operator": "regex",
                "value": "^[\\w.-]+@[\\w.-]+\\.\\w+$"
            }
        }
        "#;

        let result = compiler.compile_from_json(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_invalid_regex() {
        let mut compiler = RuleCompiler::new();
        let json = r#"
        {
            "id": "rule-001",
            "name": "test",
            "version": "1.0",
            "root": {
                "type": "condition",
                "field": "email",
                "operator": "regex",
                "value": "[invalid"
            }
        }
        "#;

        let result = compiler.compile_from_json(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("正则表达式无效"));
    }
}
