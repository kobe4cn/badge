use std::collections::HashMap;

use super::dto::BadgeDependency;
use crate::repository::BadgeDependencyRow;

/// 依赖图
///
/// 用于高效查询徽章之间的依赖关系，在服务启动时从数据库加载并缓存
#[derive(Debug, Default, Clone)]
pub struct DependencyGraph {
    /// badge_id -> 依赖此徽章的徽章列表 (auto_trigger=true)
    triggered_by: HashMap<i64, Vec<BadgeDependency>>,
    /// badge_id -> 此徽章的前置条件
    prerequisites: HashMap<i64, Vec<BadgeDependency>>,
    /// exclusive_group_id -> 组内徽章列表
    exclusive_groups: HashMap<String, Vec<i64>>,
}

impl DependencyGraph {
    /// 从数据库行构建依赖图
    pub fn from_rows(rows: Vec<BadgeDependencyRow>) -> Self {
        let mut graph = Self::default();

        for row in rows {
            let dependency = BadgeDependency::from_row(row.clone());

            // 构建 triggered_by 映射（仅自动触发的依赖）
            if dependency.auto_trigger {
                graph
                    .triggered_by
                    .entry(dependency.depends_on_badge_id)
                    .or_default()
                    .push(dependency.clone());
            }

            // 构建 prerequisites 映射
            graph
                .prerequisites
                .entry(dependency.badge_id)
                .or_default()
                .push(dependency.clone());

            // 构建 exclusive_groups 映射
            if let Some(ref group_id) = dependency.exclusive_group_id {
                let badges = graph.exclusive_groups.entry(group_id.clone()).or_default();
                if !badges.contains(&dependency.badge_id) {
                    badges.push(dependency.badge_id);
                }
            }
        }

        // 按优先级排序，确保高优先级的依赖先被处理
        for deps in graph.triggered_by.values_mut() {
            deps.sort_by_key(|d| d.priority);
        }
        for deps in graph.prerequisites.values_mut() {
            deps.sort_by_key(|d| d.priority);
        }

        graph
    }

    /// 获取依赖某徽章的所有徽章（用于级联触发）
    pub fn get_triggered_by(&self, badge_id: i64) -> &[BadgeDependency] {
        self.triggered_by
            .get(&badge_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// 获取某徽章的所有前置条件
    pub fn get_prerequisites(&self, badge_id: i64) -> &[BadgeDependency] {
        self.prerequisites
            .get(&badge_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// 获取互斥组中的所有徽章
    pub fn get_exclusive_group(&self, group_id: &str) -> &[i64] {
        self.exclusive_groups
            .get(group_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// 检查依赖图是否为空
    pub fn is_empty(&self) -> bool {
        self.triggered_by.is_empty() && self.prerequisites.is_empty()
    }
}

impl BadgeDependency {
    /// 从数据库行转换
    pub fn from_row(row: BadgeDependencyRow) -> Self {
        Self {
            id: row.id,
            badge_id: row.badge_id,
            depends_on_badge_id: row.depends_on_badge_id,
            dependency_type: row.dependency_type.parse()
                .unwrap_or(super::dto::DependencyType::Prerequisite),
            required_quantity: row.required_quantity,
            exclusive_group_id: row.exclusive_group_id,
            auto_trigger: row.auto_trigger,
            priority: row.priority,
            dependency_group_id: row.dependency_group_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::sync::atomic::{AtomicI64, Ordering};

    static TEST_ID: AtomicI64 = AtomicI64::new(1000);

    fn next_test_id() -> i64 {
        TEST_ID.fetch_add(1, Ordering::Relaxed)
    }

    fn create_test_row(
        badge_id: i64,
        depends_on: i64,
        dep_type: &str,
        auto_trigger: bool,
        group_id: &str,
        exclusive_group: Option<&str>,
    ) -> BadgeDependencyRow {
        BadgeDependencyRow {
            id: next_test_id(),
            badge_id,
            depends_on_badge_id: depends_on,
            dependency_type: dep_type.to_string(),
            required_quantity: 1,
            exclusive_group_id: exclusive_group.map(|s| s.to_string()),
            auto_trigger,
            priority: 0,
            dependency_group_id: group_id.to_string(),
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_empty_graph() {
        let graph = DependencyGraph::from_rows(vec![]);
        assert!(graph.is_empty());
    }

    #[test]
    fn test_triggered_by_mapping() {
        let badge_a = next_test_id();
        let badge_b = next_test_id();

        let rows = vec![create_test_row(
            badge_b,
            badge_a,
            "prerequisite",
            true,
            "default",
            None,
        )];

        let graph = DependencyGraph::from_rows(rows);

        // 获得 badge_a 后应触发对 badge_b 的检查
        let triggered = graph.get_triggered_by(badge_a);
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].badge_id, badge_b);
    }

    #[test]
    fn test_prerequisites_mapping() {
        let badge_a = next_test_id();
        let badge_b = next_test_id();

        let rows = vec![create_test_row(
            badge_b,
            badge_a,
            "prerequisite",
            true,
            "default",
            None,
        )];

        let graph = DependencyGraph::from_rows(rows);

        // badge_b 的前置条件是 badge_a
        let prereqs = graph.get_prerequisites(badge_b);
        assert_eq!(prereqs.len(), 1);
        assert_eq!(prereqs[0].depends_on_badge_id, badge_a);
    }

    #[test]
    fn test_exclusive_groups() {
        let badge_a = next_test_id();
        let badge_b = next_test_id();
        let badge_c = next_test_id();

        let rows = vec![
            create_test_row(
                badge_a,
                next_test_id(),
                "exclusive",
                false,
                "default",
                Some("vip_tier"),
            ),
            create_test_row(
                badge_b,
                next_test_id(),
                "exclusive",
                false,
                "default",
                Some("vip_tier"),
            ),
        ];

        let graph = DependencyGraph::from_rows(rows);

        let group = graph.get_exclusive_group("vip_tier");
        assert_eq!(group.len(), 2);
        assert!(group.contains(&badge_a));
        assert!(group.contains(&badge_b));
        assert!(!group.contains(&badge_c));
    }

    #[test]
    fn test_non_auto_trigger_not_in_triggered_by() {
        let badge_a = next_test_id();
        let badge_b = next_test_id();

        let rows = vec![create_test_row(
            badge_b,
            badge_a,
            "prerequisite",
            false, // auto_trigger = false
            "default",
            None,
        )];

        let graph = DependencyGraph::from_rows(rows);

        // auto_trigger=false 的依赖不应出现在 triggered_by 中
        let triggered = graph.get_triggered_by(badge_a);
        assert!(triggered.is_empty());

        // 但仍应出现在 prerequisites 中
        let prereqs = graph.get_prerequisites(badge_b);
        assert_eq!(prereqs.len(), 1);
    }
}
