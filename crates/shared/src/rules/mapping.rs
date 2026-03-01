//! 规则到徽章的内存映射
//!
//! 使用 ArcSwap 实现原子替换，保证并发读取时不会看到中间状态（空映射）。

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use arc_swap::ArcSwap;
use chrono::{DateTime, Utc};
use crossbeam_utils::atomic::AtomicCell;

use super::models::{BadgeGrant, LoadStatus};

/// 规则到徽章的映射
///
/// 按 event_type 分组存储规则，支持高并发读取。
/// 使用 ArcSwap 原子指针交换，避免 clear()+insert() 导致的空读窗口。
pub struct RuleBadgeMapping {
    /// event_type -> Vec<BadgeGrant>，通过 ArcSwap 实现原子替换
    mappings: ArcSwap<HashMap<String, Vec<BadgeGrant>>>,
    /// 最后加载时间
    last_loaded_at: AtomicCell<Option<DateTime<Utc>>>,
    /// 规则总数
    rule_count: AtomicUsize,
}

impl RuleBadgeMapping {
    pub fn new() -> Self {
        Self {
            mappings: ArcSwap::from_pointee(HashMap::new()),
            last_loaded_at: AtomicCell::new(None),
            rule_count: AtomicUsize::new(0),
        }
    }

    /// 根据事件类型获取所有适用规则
    pub fn get_rules_by_event_type(&self, event_type: &str) -> Vec<BadgeGrant> {
        let snapshot = self.mappings.load();
        snapshot
            .get(event_type)
            .cloned()
            .unwrap_or_default()
    }

    /// 全量替换规则（刷新时调用）
    ///
    /// 先构建完整的新映射，再通过 ArcSwap::store 原子替换指针。
    /// 并发读取者始终看到完整的旧映射或完整的新映射，不会看到空状态。
    pub fn replace_all(&self, rules: Vec<BadgeGrant>) {
        let mut grouped: HashMap<String, Vec<BadgeGrant>> = HashMap::new();

        for rule in &rules {
            grouped
                .entry(rule.event_type.clone())
                .or_default()
                .push(rule.clone());
        }

        // 原子替换：读取者不会看到中间状态
        self.mappings.store(Arc::new(grouped));

        self.rule_count.store(rules.len(), Ordering::SeqCst);
        self.last_loaded_at.store(Some(Utc::now()));
    }

    /// 获取加载状态
    pub fn load_status(&self) -> LoadStatus {
        let snapshot = self.mappings.load();
        let event_types: Vec<String> = snapshot.keys().cloned().collect();

        LoadStatus {
            loaded: self.last_loaded_at.load().is_some(),
            rule_count: self.rule_count.load(Ordering::SeqCst),
            last_loaded_at: self.last_loaded_at.load(),
            event_types,
        }
    }

    /// 获取规则总数
    pub fn rule_count(&self) -> usize {
        self.rule_count.load(Ordering::SeqCst)
    }

    /// 是否已加载
    pub fn is_loaded(&self) -> bool {
        self.last_loaded_at.load().is_some()
    }

    /// 获取所有事件类型
    pub fn event_types(&self) -> Vec<String> {
        let snapshot = self.mappings.load();
        snapshot.keys().cloned().collect()
    }
}

impl Default for RuleBadgeMapping {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_rule(rule_id: i64, event_type: &str) -> BadgeGrant {
        BadgeGrant {
            rule_id,
            rule_code: format!("RULE_{}", rule_id),
            badge_id: rule_id * 10,
            badge_name: format!("Badge {}", rule_id),
            quantity: 1,
            event_type: event_type.to_string(),
            start_time: None,
            end_time: None,
            max_count_per_user: None,
            global_quota: None,
            global_granted: 0,
            rule_json: None,
        }
    }

    #[test]
    fn test_empty_mapping() {
        let mapping = RuleBadgeMapping::new();

        assert!(!mapping.is_loaded());
        assert_eq!(mapping.rule_count(), 0);
        assert!(mapping.event_types().is_empty());
        assert!(mapping.get_rules_by_event_type("login").is_empty());

        let status = mapping.load_status();
        assert!(!status.loaded);
        assert_eq!(status.rule_count, 0);
        assert!(status.last_loaded_at.is_none());
        assert!(status.event_types.is_empty());
    }

    #[test]
    fn test_replace_all() {
        let mapping = RuleBadgeMapping::new();

        let rules = vec![
            create_test_rule(1, "login"),
            create_test_rule(2, "login"),
            create_test_rule(3, "purchase"),
        ];

        mapping.replace_all(rules);

        assert!(mapping.is_loaded());
        assert_eq!(mapping.rule_count(), 3);

        let login_rules = mapping.get_rules_by_event_type("login");
        assert_eq!(login_rules.len(), 2);

        let purchase_rules = mapping.get_rules_by_event_type("purchase");
        assert_eq!(purchase_rules.len(), 1);
        assert_eq!(purchase_rules[0].rule_id, 3);

        let nonexistent = mapping.get_rules_by_event_type("nonexistent");
        assert!(nonexistent.is_empty());

        let mut event_types = mapping.event_types();
        event_types.sort();
        assert_eq!(event_types, vec!["login", "purchase"]);
    }

    #[test]
    fn test_replace_clears_old_rules() {
        let mapping = RuleBadgeMapping::new();

        // 首次加载
        let rules_v1 = vec![
            create_test_rule(1, "login"),
            create_test_rule(2, "purchase"),
        ];
        mapping.replace_all(rules_v1);

        assert_eq!(mapping.rule_count(), 2);
        assert_eq!(mapping.get_rules_by_event_type("login").len(), 1);
        assert_eq!(mapping.get_rules_by_event_type("purchase").len(), 1);

        // 替换为新规则（不包含 purchase 类型）
        let rules_v2 = vec![create_test_rule(3, "login"), create_test_rule(4, "signup")];
        mapping.replace_all(rules_v2);

        assert_eq!(mapping.rule_count(), 2);
        assert_eq!(mapping.get_rules_by_event_type("login").len(), 1);
        assert_eq!(mapping.get_rules_by_event_type("login")[0].rule_id, 3);
        // 旧的 purchase 规则应该被清除
        assert!(mapping.get_rules_by_event_type("purchase").is_empty());
        assert_eq!(mapping.get_rules_by_event_type("signup").len(), 1);
    }

    #[test]
    fn test_load_status() {
        let mapping = RuleBadgeMapping::new();

        // 初始状态
        let status = mapping.load_status();
        assert!(!status.loaded);
        assert_eq!(status.rule_count, 0);
        assert!(status.last_loaded_at.is_none());

        // 加载规则后
        let rules = vec![
            create_test_rule(1, "login"),
            create_test_rule(2, "purchase"),
        ];
        mapping.replace_all(rules);

        let status = mapping.load_status();
        assert!(status.loaded);
        assert_eq!(status.rule_count, 2);
        assert!(status.last_loaded_at.is_some());

        let mut event_types = status.event_types;
        event_types.sort();
        assert_eq!(event_types, vec!["login", "purchase"]);
    }
}
