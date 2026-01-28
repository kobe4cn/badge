//! 规则存储管理
//!
//! 使用 DashMap 提供线程安全的规则缓存，支持规则的加载、更新、删除和批量操作。

use crate::compiler::{CompiledRule, RuleCompiler};
use crate::error::{Result, RuleError};
use crate::models::Rule;
use dashmap::DashMap;
use std::sync::Arc;
use tracing::{info, instrument, warn};

/// 规则存储
#[derive(Clone)]
pub struct RuleStore {
    /// 编译后的规则缓存
    rules: Arc<DashMap<String, CompiledRule>>,
    /// 规则编译器
    compiler: Arc<parking_lot::Mutex<RuleCompiler>>,
}

impl RuleStore {
    /// 创建新的规则存储
    pub fn new() -> Self {
        Self {
            rules: Arc::new(DashMap::new()),
            compiler: Arc::new(parking_lot::Mutex::new(RuleCompiler::new())),
        }
    }

    /// 获取当前存储的规则数量
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// 检查存储是否为空
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// 加载规则（从 Rule 对象）
    #[instrument(skip(self, rule), fields(rule_id = %rule.id, rule_name = %rule.name))]
    pub fn load(&self, rule: Rule) -> Result<()> {
        let compiled = {
            let mut compiler = self.compiler.lock();
            compiler.compile(rule)?
        };

        let rule_id = compiled.id().to_string();
        self.rules.insert(rule_id.clone(), compiled);

        info!("规则已加载: {}", rule_id);
        Ok(())
    }

    /// 加载规则（从 JSON 字符串）
    #[instrument(skip(self, json))]
    pub fn load_from_json(&self, json: &str) -> Result<String> {
        let compiled = {
            let mut compiler = self.compiler.lock();
            compiler.compile_from_json(json)?
        };

        let rule_id = compiled.id().to_string();
        self.rules.insert(rule_id.clone(), compiled);

        info!("规则已加载: {}", rule_id);
        Ok(rule_id)
    }

    /// 更新规则
    #[instrument(skip(self, rule), fields(rule_id = %rule.id))]
    pub fn update(&self, rule: Rule) -> Result<()> {
        let rule_id = rule.id.clone();

        if !self.rules.contains_key(&rule_id) {
            warn!("更新不存在的规则: {}", rule_id);
            return Err(RuleError::RuleNotFound(rule_id));
        }

        self.load(rule)
    }

    /// 删除规则
    #[instrument(skip(self))]
    pub fn delete(&self, rule_id: &str) -> Result<()> {
        if self.rules.remove(rule_id).is_some() {
            info!("规则已删除: {}", rule_id);
            Ok(())
        } else {
            warn!("删除不存在的规则: {}", rule_id);
            Err(RuleError::RuleNotFound(rule_id.to_string()))
        }
    }

    /// 获取规则
    pub fn get(&self, rule_id: &str) -> Option<CompiledRule> {
        self.rules.get(rule_id).map(|r| r.clone())
    }

    /// 检查规则是否存在
    pub fn contains(&self, rule_id: &str) -> bool {
        self.rules.contains_key(rule_id)
    }

    /// 获取所有规则 ID
    pub fn list_ids(&self) -> Vec<String> {
        self.rules.iter().map(|r| r.key().clone()).collect()
    }

    /// 获取所有规则
    pub fn list_all(&self) -> Vec<CompiledRule> {
        self.rules.iter().map(|r| r.value().clone()).collect()
    }

    /// 批量加载规则
    #[instrument(skip(self, rules))]
    pub fn load_batch(&self, rules: Vec<Rule>) -> Result<Vec<String>> {
        let mut loaded_ids = Vec::with_capacity(rules.len());
        let mut errors = Vec::new();

        for rule in rules {
            let rule_id = rule.id.clone();
            match self.load(rule) {
                Ok(()) => loaded_ids.push(rule_id),
                Err(e) => errors.push((rule_id, e)),
            }
        }

        if !errors.is_empty() {
            warn!("批量加载部分失败: {:?}", errors);
        }

        info!("批量加载完成: {} 成功, {} 失败", loaded_ids.len(), errors.len());
        Ok(loaded_ids)
    }

    /// 清空所有规则
    #[instrument(skip(self))]
    pub fn clear(&self) {
        let count = self.rules.len();
        self.rules.clear();
        info!("已清空 {} 条规则", count);
    }

    /// 获取规则统计信息
    pub fn stats(&self) -> RuleStoreStats {
        let rules_count = self.rules.len();
        let total_fields: usize = self
            .rules
            .iter()
            .map(|r| r.required_fields.len())
            .sum();

        RuleStoreStats {
            rules_count,
            total_fields,
            avg_fields_per_rule: if rules_count > 0 {
                total_fields as f64 / rules_count as f64
            } else {
                0.0
            },
        }
    }
}

impl Default for RuleStore {
    fn default() -> Self {
        Self::new()
    }
}

/// 规则存储统计信息
#[derive(Debug, Clone)]
pub struct RuleStoreStats {
    /// 规则总数
    pub rules_count: usize,
    /// 所有规则使用的字段总数
    pub total_fields: usize,
    /// 平均每条规则使用的字段数
    pub avg_fields_per_rule: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Condition, LogicalGroup, RuleNode};
    use crate::operators::Operator;

    fn sample_rule(id: &str, name: &str) -> Rule {
        Rule {
            id: id.to_string(),
            name: name.to_string(),
            version: "1.0".to_string(),
            root: RuleNode::Group(LogicalGroup::and(vec![
                RuleNode::Condition(Condition::new("event.type", Operator::Eq, "PURCHASE")),
                RuleNode::Condition(Condition::new("order.amount", Operator::Gte, 500)),
            ])),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn sample_rule_json(id: &str, name: &str) -> String {
        format!(
            r#"
            {{
                "id": "{}",
                "name": "{}",
                "version": "1.0",
                "root": {{
                    "type": "group",
                    "operator": "AND",
                    "children": [
                        {{
                            "type": "condition",
                            "field": "event.type",
                            "operator": "eq",
                            "value": "PURCHASE"
                        }},
                        {{
                            "type": "condition",
                            "field": "order.amount",
                            "operator": "gte",
                            "value": 500
                        }}
                    ]
                }}
            }}
            "#,
            id, name
        )
    }

    #[test]
    fn test_load_rule() {
        let store = RuleStore::new();
        let rule = sample_rule("rule-001", "test");

        store.load(rule).unwrap();

        assert_eq!(store.len(), 1);
        assert!(store.contains("rule-001"));
    }

    #[test]
    fn test_load_from_json() {
        let store = RuleStore::new();
        let json = sample_rule_json("rule-001", "test");

        let rule_id = store.load_from_json(&json).unwrap();

        assert_eq!(rule_id, "rule-001");
        assert!(store.contains("rule-001"));
    }

    #[test]
    fn test_get_rule() {
        let store = RuleStore::new();
        store.load(sample_rule("rule-001", "test")).unwrap();

        let rule = store.get("rule-001").unwrap();
        assert_eq!(rule.id(), "rule-001");
        assert_eq!(rule.name(), "test");
    }

    #[test]
    fn test_get_nonexistent_rule() {
        let store = RuleStore::new();
        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn test_update_rule() {
        let store = RuleStore::new();
        store.load(sample_rule("rule-001", "test")).unwrap();

        let updated = sample_rule("rule-001", "updated");
        store.update(updated).unwrap();

        let rule = store.get("rule-001").unwrap();
        assert_eq!(rule.name(), "updated");
    }

    #[test]
    fn test_update_nonexistent_rule() {
        let store = RuleStore::new();
        let rule = sample_rule("rule-001", "test");

        let result = store.update(rule);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_rule() {
        let store = RuleStore::new();
        store.load(sample_rule("rule-001", "test")).unwrap();

        store.delete("rule-001").unwrap();

        assert!(!store.contains("rule-001"));
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_delete_nonexistent_rule() {
        let store = RuleStore::new();
        let result = store.delete("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_ids() {
        let store = RuleStore::new();
        store.load(sample_rule("rule-001", "test1")).unwrap();
        store.load(sample_rule("rule-002", "test2")).unwrap();

        let ids = store.list_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"rule-001".to_string()));
        assert!(ids.contains(&"rule-002".to_string()));
    }

    #[test]
    fn test_load_batch() {
        let store = RuleStore::new();
        let rules = vec![
            sample_rule("rule-001", "test1"),
            sample_rule("rule-002", "test2"),
            sample_rule("rule-003", "test3"),
        ];

        let loaded = store.load_batch(rules).unwrap();

        assert_eq!(loaded.len(), 3);
        assert_eq!(store.len(), 3);
    }

    #[test]
    fn test_clear() {
        let store = RuleStore::new();
        store.load(sample_rule("rule-001", "test1")).unwrap();
        store.load(sample_rule("rule-002", "test2")).unwrap();

        store.clear();

        assert!(store.is_empty());
    }

    #[test]
    fn test_stats() {
        let store = RuleStore::new();
        store.load(sample_rule("rule-001", "test1")).unwrap();
        store.load(sample_rule("rule-002", "test2")).unwrap();

        let stats = store.stats();

        assert_eq!(stats.rules_count, 2);
        assert_eq!(stats.total_fields, 4); // 每个规则有 2 个字段
        assert_eq!(stats.avg_fields_per_rule, 2.0);
    }

    #[test]
    fn test_concurrent_access() {
        use std::thread;

        let store = RuleStore::new();
        let store_clone = store.clone();

        let handle = thread::spawn(move || {
            for i in 0..100 {
                store_clone
                    .load(sample_rule(&format!("rule-{}", i), &format!("test-{}", i)))
                    .unwrap();
            }
        });

        for i in 100..200 {
            store
                .load(sample_rule(&format!("rule-{}", i), &format!("test-{}", i)))
                .unwrap();
        }

        handle.join().unwrap();

        assert_eq!(store.len(), 200);
    }
}
