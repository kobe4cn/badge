//! 规则与徽章映射
//!
//! 管理规则 ID 到徽章发放配置的映射关系。事件处理流程中，规则引擎返回匹配的
//! 规则 ID 后，需要通过此映射查找对应的徽章 ID 和发放数量。
//!
//! 当前使用内存映射实现，生产环境应从数据库或配置中心加载。

use dashmap::DashMap;

/// 规则 ID -> 徽章发放配置的映射
///
/// 使用 DashMap 而非 HashMap + RwLock，在高并发读场景下
/// 分段锁的性能优于全局读写锁。
pub struct RuleBadgeMapping {
    mappings: DashMap<String, BadgeGrant>,
}

/// 单条规则匹配后需要发放的徽章配置
#[derive(Debug, Clone)]
pub struct BadgeGrant {
    /// 徽章 ID（与徽章管理服务中的 badge_id 对应）
    pub badge_id: i64,
    /// 徽章名称（用于日志和通知展示，避免反查徽章服务）
    pub badge_name: String,
    /// 单次发放数量（大部分徽章为 1，活动类可能多发）
    pub quantity: i32,
}

impl RuleBadgeMapping {
    pub fn new() -> Self {
        Self {
            mappings: DashMap::new(),
        }
    }

    /// 注册一条规则到徽章的映射
    pub fn add_mapping(&self, rule_id: impl Into<String>, badge_grant: BadgeGrant) {
        self.mappings.insert(rule_id.into(), badge_grant);
    }

    /// 查找规则对应的徽章发放配置
    pub fn get_grant(&self, rule_id: &str) -> Option<BadgeGrant> {
        self.mappings.get(rule_id).map(|entry| entry.clone())
    }

    /// 返回所有已注册的规则 ID，用于批量评估请求
    pub fn get_all_rule_ids(&self) -> Vec<String> {
        self.mappings
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
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

    #[test]
    fn test_add_and_get_mapping() {
        let mapping = RuleBadgeMapping::new();
        mapping.add_mapping(
            "rule-001",
            BadgeGrant {
                badge_id: 42,
                badge_name: "首次购物".to_string(),
                quantity: 1,
            },
        );
        mapping.add_mapping(
            "rule-002",
            BadgeGrant {
                badge_id: 43,
                badge_name: "累计消费满1000".to_string(),
                quantity: 1,
            },
        );

        let grant = mapping.get_grant("rule-001").expect("应找到映射");
        assert_eq!(grant.badge_id, 42);
        assert_eq!(grant.badge_name, "首次购物");
        assert_eq!(grant.quantity, 1);

        let grant = mapping.get_grant("rule-002").expect("应找到映射");
        assert_eq!(grant.badge_id, 43);
        assert_eq!(grant.badge_name, "累计消费满1000");
    }

    #[test]
    fn test_get_nonexistent_mapping() {
        let mapping = RuleBadgeMapping::new();
        assert!(mapping.get_grant("nonexistent-rule").is_none());
    }

    #[test]
    fn test_get_all_rule_ids() {
        let mapping = RuleBadgeMapping::new();
        mapping.add_mapping(
            "rule-001",
            BadgeGrant {
                badge_id: 42,
                badge_name: "首次购物".to_string(),
                quantity: 1,
            },
        );
        mapping.add_mapping(
            "rule-002",
            BadgeGrant {
                badge_id: 43,
                badge_name: "累计消费满1000".to_string(),
                quantity: 1,
            },
        );

        let mut rule_ids = mapping.get_all_rule_ids();
        rule_ids.sort();

        assert_eq!(rule_ids.len(), 2);
        assert_eq!(rule_ids[0], "rule-001");
        assert_eq!(rule_ids[1], "rule-002");
    }
}
