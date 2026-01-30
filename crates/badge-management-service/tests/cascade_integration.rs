//! 级联触发与竞争兑换集成测试
//!
//! 测试完整的业务流程，包括依赖配置、发放、级联触发、竞争兑换等

use badge_management::cascade::{
    BlockReason, CascadeConfig, CascadeContext, CascadeResult, DependencyGraph, DependencyType,
};
use badge_management::lock::LockConfig;
use badge_management::repository::BadgeDependencyRow;
use badge_management::service::{
    CompetitiveRedeemRequest, CompetitiveRedeemResponse, ConsumedBadge,
};
use chrono::Utc;
use std::time::Duration;
use uuid::Uuid;

/// 创建测试用的依赖关系行
fn create_dependency_row(
    badge_id: Uuid,
    depends_on: Uuid,
    dep_type: &str,
    auto_trigger: bool,
    group_id: &str,
    exclusive_group: Option<&str>,
) -> BadgeDependencyRow {
    BadgeDependencyRow {
        id: Uuid::new_v4(),
        badge_id,
        depends_on_badge_id: depends_on,
        dependency_type: dep_type.to_string(),
        required_quantity: 1,
        exclusive_group_id: exclusive_group.map(String::from),
        auto_trigger,
        priority: 0,
        dependency_group_id: group_id.to_string(),
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

mod cascade_integration {
    use super::*;

    /// 测试场景：用户完成注册获得徽章A，绑定手机获得徽章B，同时持有A+B自动点亮C
    ///
    /// 该场景验证多前置条件的级联触发机制：
    /// - 获得 A 时会检查 C 是否可触发（此时缺少 B，不满足条件）
    /// - 获得 B 时再次检查 C，此时 A+B 都满足，C 被自动发放
    #[test]
    fn test_cascade_chain_registration_binding_achievement() {
        let badge_a = Uuid::new_v4(); // 注册徽章
        let badge_b = Uuid::new_v4(); // 绑定徽章
        let badge_c = Uuid::new_v4(); // 成就徽章（需要A+B）

        let dependencies = vec![
            // C 依赖 A（前置条件，自动触发）
            create_dependency_row(badge_c, badge_a, "prerequisite", true, "group1", None),
            // C 依赖 B（前置条件，自动触发）
            create_dependency_row(badge_c, badge_b, "prerequisite", true, "group1", None),
        ];

        let graph = DependencyGraph::from_rows(dependencies);

        // 当用户获得 A 时，触发检查 C
        let triggered_by_a = graph.get_triggered_by(badge_a);
        assert_eq!(triggered_by_a.len(), 1);
        assert_eq!(triggered_by_a[0].badge_id, badge_c);

        // 当用户获得 B 时，也触发检查 C
        let triggered_by_b = graph.get_triggered_by(badge_b);
        assert_eq!(triggered_by_b.len(), 1);
        assert_eq!(triggered_by_b[0].badge_id, badge_c);

        // C 的前置条件是 A 和 B（同组 AND 关系）
        let prereqs_c = graph.get_prerequisites(badge_c);
        assert_eq!(prereqs_c.len(), 2);
    }

    /// 测试场景：多级级联 A -> B -> C -> D
    ///
    /// 验证级联触发可以形成链式反应：
    /// 用户获得 A 后，如果满足条件，依次触发 B、C、D 的检查与发放
    #[test]
    fn test_multi_level_cascade() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        let d = Uuid::new_v4();

        let dependencies = vec![
            create_dependency_row(b, a, "prerequisite", true, "g1", None),
            create_dependency_row(c, b, "prerequisite", true, "g1", None),
            create_dependency_row(d, c, "prerequisite", true, "g1", None),
        ];

        let graph = DependencyGraph::from_rows(dependencies);

        // 验证触发链
        assert_eq!(graph.get_triggered_by(a).len(), 1);
        assert_eq!(graph.get_triggered_by(a)[0].badge_id, b);

        assert_eq!(graph.get_triggered_by(b).len(), 1);
        assert_eq!(graph.get_triggered_by(b)[0].badge_id, c);

        assert_eq!(graph.get_triggered_by(c).len(), 1);
        assert_eq!(graph.get_triggered_by(c)[0].badge_id, d);

        // D 不触发任何徽章（链条末端）
        assert!(graph.get_triggered_by(d).is_empty());
    }

    /// 测试场景：循环依赖检测
    ///
    /// CascadeContext 通过 visited 集合追踪已访问的徽章，
    /// 防止 A -> B -> C -> A 这样的循环依赖导致无限递归
    #[test]
    fn test_cycle_detection_in_cascade_context() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        let mut context = CascadeContext::new();

        // 模拟访问路径 A -> B -> C
        context.enter(a);
        assert!(!context.has_cycle(b));

        context.enter(b);
        assert!(!context.has_cycle(c));

        context.enter(c);
        // 尝试再次访问 A，应检测到循环
        assert!(context.has_cycle(a));

        // 验证路径
        assert_eq!(context.path.len(), 3);
        assert_eq!(context.path[0], a);
        assert_eq!(context.path[1], b);
        assert_eq!(context.path[2], c);
    }

    /// 测试场景：深度限制
    ///
    /// 即使没有循环，也要限制级联深度，防止过深的依赖链消耗过多资源
    #[test]
    fn test_depth_limit_enforcement() {
        let config = CascadeConfig {
            max_depth: 5,
            timeout_ms: 5000,
            graph_cache_seconds: 300,
        };

        let mut context = CascadeContext::new();

        // 模拟 6 层深度
        for i in 0..6 {
            context.enter(Uuid::new_v4());
            if context.depth > config.max_depth {
                // 应该在第 6 层被阻止
                assert_eq!(i, 5);
                break;
            }
        }

        assert!(context.depth > config.max_depth);
    }

    /// 测试场景：CascadeContext 的 leave 操作
    ///
    /// 验证离开某层后深度正确递减，但 visited 集合保持不变（防止同一评估中重复访问）
    #[test]
    fn test_cascade_context_enter_leave() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();

        let mut context = CascadeContext::new();
        assert_eq!(context.depth, 0);
        assert!(context.path.is_empty());

        context.enter(a);
        assert_eq!(context.depth, 1);
        assert_eq!(context.path.len(), 1);

        context.enter(b);
        assert_eq!(context.depth, 2);
        assert_eq!(context.path.len(), 2);

        context.leave();
        assert_eq!(context.depth, 1);
        assert_eq!(context.path.len(), 1);

        // visited 集合仍然包含已访问的徽章
        assert!(context.visited.contains(&a));
        assert!(context.visited.contains(&b));
    }

    /// 测试场景：CascadeContext 计时功能
    #[test]
    fn test_cascade_context_elapsed_time() {
        let context = CascadeContext::new();

        // 等待一小段时间
        std::thread::sleep(Duration::from_millis(10));

        // elapsed_ms 应该大于 0
        let elapsed = context.elapsed_ms();
        assert!(elapsed >= 10, "elapsed_ms should be at least 10ms");
    }

    /// 测试场景：多个徽章触发同一目标徽章
    ///
    /// 如果 C 依赖 A 和 B，那么获得 A 或 B 都应该触发对 C 的检查
    #[test]
    fn test_multiple_triggers_for_same_target() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        let d = Uuid::new_v4();
        let target = Uuid::new_v4();

        // target 依赖 a, b, c, d
        let dependencies = vec![
            create_dependency_row(target, a, "prerequisite", true, "g1", None),
            create_dependency_row(target, b, "prerequisite", true, "g1", None),
            create_dependency_row(target, c, "prerequisite", true, "g1", None),
            create_dependency_row(target, d, "prerequisite", true, "g1", None),
        ];

        let graph = DependencyGraph::from_rows(dependencies);

        // 每个依赖徽章都应该触发 target 的检查
        assert_eq!(graph.get_triggered_by(a)[0].badge_id, target);
        assert_eq!(graph.get_triggered_by(b)[0].badge_id, target);
        assert_eq!(graph.get_triggered_by(c)[0].badge_id, target);
        assert_eq!(graph.get_triggered_by(d)[0].badge_id, target);

        // target 有 4 个前置条件
        assert_eq!(graph.get_prerequisites(target).len(), 4);
    }

    /// 测试场景：非自动触发的依赖不会出现在 triggered_by 中
    #[test]
    fn test_non_auto_trigger_dependencies() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        let dependencies = vec![
            // B 依赖 A，自动触发
            create_dependency_row(b, a, "prerequisite", true, "g1", None),
            // C 依赖 A，但不自动触发（需要手动兑换）
            create_dependency_row(c, a, "prerequisite", false, "g1", None),
        ];

        let graph = DependencyGraph::from_rows(dependencies);

        // A 只触发 B，不触发 C
        let triggered = graph.get_triggered_by(a);
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].badge_id, b);

        // 但 C 的前置条件仍然包含 A
        let prereqs_c = graph.get_prerequisites(c);
        assert_eq!(prereqs_c.len(), 1);
        assert_eq!(prereqs_c[0].depends_on_badge_id, a);
    }
}

mod competitive_redemption_integration {
    use super::*;

    /// 测试场景：D需要A+B+F(消耗), E需要A+C+F(消耗), D和E互斥
    ///
    /// 这是一个典型的互斥兑换场景：
    /// - F 是消耗型徽章，只能用于兑换 D 或 E 中的一个
    /// - D 和 E 在同一互斥组中，用户只能拥有其中一个
    #[test]
    fn test_exclusive_redemption_scenario() {
        let a = Uuid::new_v4(); // 普通徽章
        let b = Uuid::new_v4(); // 普通徽章
        let c = Uuid::new_v4(); // 普通徽章
        let f = Uuid::new_v4(); // 消耗型徽章
        let d = Uuid::new_v4(); // 目标徽章 D
        let e = Uuid::new_v4(); // 目标徽章 E

        let dependencies = vec![
            // D 的依赖
            create_dependency_row(d, a, "prerequisite", false, "d_group", None),
            create_dependency_row(d, b, "prerequisite", false, "d_group", None),
            create_dependency_row(d, f, "consume", false, "d_group", Some("exclusive_df")),
            // E 的依赖
            create_dependency_row(e, a, "prerequisite", false, "e_group", None),
            create_dependency_row(e, c, "prerequisite", false, "e_group", None),
            create_dependency_row(e, f, "consume", false, "e_group", Some("exclusive_df")),
        ];

        let graph = DependencyGraph::from_rows(dependencies);

        // 验证 D 的前置条件
        let d_prereqs = graph.get_prerequisites(d);
        assert_eq!(d_prereqs.len(), 3);

        // 验证 E 的前置条件
        let e_prereqs = graph.get_prerequisites(e);
        assert_eq!(e_prereqs.len(), 3);

        // 验证互斥组包含 D 和 E
        let exclusive_group = graph.get_exclusive_group("exclusive_df");
        assert_eq!(exclusive_group.len(), 2);
        assert!(exclusive_group.contains(&d));
        assert!(exclusive_group.contains(&e));
    }

    /// 测试场景：竞争兑换请求和响应结构
    #[test]
    fn test_redemption_request_response_flow() {
        let user_id = "user_12345".to_string();
        let target_badge = Uuid::new_v4();
        let consumed_badge_1 = Uuid::new_v4();
        let consumed_badge_2 = Uuid::new_v4();

        // 创建请求
        let request = CompetitiveRedeemRequest::new(user_id.clone(), target_badge);
        assert_eq!(request.user_id, user_id);
        assert_eq!(request.target_badge_id, target_badge);

        // 模拟成功响应
        let success_response = CompetitiveRedeemResponse {
            success: true,
            target_badge_id: target_badge,
            consumed_badges: vec![
                ConsumedBadge {
                    badge_id: consumed_badge_1,
                    quantity: 1,
                },
                ConsumedBadge {
                    badge_id: consumed_badge_2,
                    quantity: 2,
                },
            ],
            failure_reason: None,
        };

        assert!(success_response.success);
        assert_eq!(success_response.consumed_badges.len(), 2);
        assert_eq!(success_response.consumed_badges[0].quantity, 1);
        assert_eq!(success_response.consumed_badges[1].quantity, 2);

        // 模拟失败响应
        let failure_response = CompetitiveRedeemResponse {
            success: false,
            target_badge_id: target_badge,
            consumed_badges: vec![],
            failure_reason: Some("互斥冲突：用户已持有徽章 E".to_string()),
        };

        assert!(!failure_response.success);
        assert!(failure_response.consumed_badges.is_empty());
        assert!(failure_response.failure_reason.is_some());
    }

    /// 测试场景：带规则 ID 的兑换请求
    #[test]
    fn test_redemption_request_with_rule_id() {
        let user_id = "user_abc";
        let target_badge = Uuid::new_v4();
        let rule_id = "promotion_2024_spring";

        let request =
            CompetitiveRedeemRequest::new(user_id, target_badge).with_rule_id(rule_id);

        assert_eq!(request.user_id, user_id);
        assert_eq!(request.target_badge_id, target_badge);
        assert_eq!(request.rule_id, Some(rule_id.to_string()));
    }

    /// 测试场景：消耗型依赖的数量要求
    #[test]
    fn test_consume_dependency_quantity() {
        let source = Uuid::new_v4();
        let target = Uuid::new_v4();

        let mut row = create_dependency_row(target, source, "consume", false, "g1", None);
        row.required_quantity = 5; // 需要消耗 5 个

        let graph = DependencyGraph::from_rows(vec![row]);

        let prereqs = graph.get_prerequisites(target);
        assert_eq!(prereqs.len(), 1);
        assert_eq!(prereqs[0].required_quantity, 5);
        assert_eq!(prereqs[0].dependency_type, DependencyType::Consume);
    }

    /// 测试场景：混合依赖类型
    ///
    /// 目标徽章同时需要：前置条件 + 消耗型依赖
    #[test]
    fn test_mixed_dependency_types() {
        let prereq_badge = Uuid::new_v4();
        let consume_badge = Uuid::new_v4();
        let target = Uuid::new_v4();

        let dependencies = vec![
            create_dependency_row(target, prereq_badge, "prerequisite", false, "g1", None),
            create_dependency_row(target, consume_badge, "consume", false, "g1", None),
        ];

        let graph = DependencyGraph::from_rows(dependencies);

        let prereqs = graph.get_prerequisites(target);
        assert_eq!(prereqs.len(), 2);

        // 验证依赖类型
        let has_prerequisite = prereqs
            .iter()
            .any(|p| p.dependency_type == DependencyType::Prerequisite);
        let has_consume = prereqs
            .iter()
            .any(|p| p.dependency_type == DependencyType::Consume);

        assert!(has_prerequisite);
        assert!(has_consume);
    }
}

mod lock_integration {
    use super::*;

    /// 测试场景：锁配置在竞争兑换中的应用
    #[test]
    fn test_lock_config_for_redemption() {
        let config = LockConfig {
            default_ttl: Duration::from_secs(10), // 兑换操作 10 秒超时
            retry_count: 2,                       // 最多重试 2 次
            retry_delay: Duration::from_millis(50), // 50ms 重试间隔
        };

        assert_eq!(config.default_ttl.as_secs(), 10);
        assert_eq!(config.retry_count, 2);
        assert_eq!(config.retry_delay.as_millis(), 50);
    }

    /// 测试场景：锁 key 格式化
    ///
    /// 锁 key 应该唯一标识被保护的资源
    #[test]
    fn test_lock_key_formatting() {
        let user_id = "user_123";
        let badge_id = Uuid::new_v4();
        let rule_id = "rule_456";

        // 竞争兑换锁 key
        let redeem_lock = format!("redeem:{}:{}", user_id, badge_id);
        assert!(redeem_lock.starts_with("redeem:"));

        // 带规则的锁 key
        let rule_lock = format!("redeem:{}:{}:{}", user_id, badge_id, rule_id);
        assert!(rule_lock.contains(rule_id));
    }

    /// 测试场景：不同用户/徽章组合产生不同的锁 key
    #[test]
    fn test_lock_key_uniqueness() {
        let user_a = "user_a";
        let user_b = "user_b";
        let badge_1 = Uuid::new_v4();
        let badge_2 = Uuid::new_v4();

        let key_1 = format!("redeem:{}:{}", user_a, badge_1);
        let key_2 = format!("redeem:{}:{}", user_a, badge_2);
        let key_3 = format!("redeem:{}:{}", user_b, badge_1);

        // 所有 key 都应该不同
        assert_ne!(key_1, key_2);
        assert_ne!(key_1, key_3);
        assert_ne!(key_2, key_3);
    }

    /// 测试场景：默认锁配置
    #[test]
    fn test_default_lock_config() {
        let config = LockConfig::default();

        // 默认配置应该是合理的生产环境值
        assert!(config.default_ttl.as_secs() >= 10);
        assert!(config.retry_count >= 1);
        assert!(config.retry_delay.as_millis() >= 10);
    }
}

mod cascade_result_integration {
    use super::*;

    /// 测试场景：CascadeResult 记录发放和阻止的徽章
    #[test]
    fn test_cascade_result_structure() {
        let result = CascadeResult::default();

        // 初始状态应为空
        assert!(result.granted_badges.is_empty());
        assert!(result.blocked_badges.is_empty());
    }

    /// 测试场景：BlockReason 枚举覆盖各种阻止原因
    #[test]
    fn test_block_reasons() {
        let missing_badges = vec![Uuid::new_v4(), Uuid::new_v4()];
        let conflicting_badge = Uuid::new_v4();

        // 前置条件不满足
        let reason1 = BlockReason::PrerequisiteNotMet {
            missing: missing_badges.clone(),
        };
        if let BlockReason::PrerequisiteNotMet { missing } = reason1 {
            assert_eq!(missing.len(), 2);
        } else {
            panic!("Expected PrerequisiteNotMet");
        }

        // 互斥冲突
        let reason2 = BlockReason::ExclusiveConflict {
            conflicting: conflicting_badge,
        };
        if let BlockReason::ExclusiveConflict { conflicting } = reason2 {
            assert_eq!(conflicting, conflicting_badge);
        } else {
            panic!("Expected ExclusiveConflict");
        }

        // 循环检测
        let reason3 = BlockReason::CycleDetected;
        assert!(matches!(reason3, BlockReason::CycleDetected));

        // 深度超限
        let reason4 = BlockReason::DepthExceeded;
        assert!(matches!(reason4, BlockReason::DepthExceeded));

        // 超时
        let reason5 = BlockReason::Timeout;
        assert!(matches!(reason5, BlockReason::Timeout));
    }
}

mod dependency_type_integration {
    use super::*;

    /// 测试场景：DependencyType 从字符串解析
    #[test]
    fn test_dependency_type_from_str() {
        assert_eq!(
            DependencyType::from_str("prerequisite"),
            Some(DependencyType::Prerequisite)
        );
        assert_eq!(
            DependencyType::from_str("PREREQUISITE"),
            Some(DependencyType::Prerequisite)
        );
        assert_eq!(
            DependencyType::from_str("consume"),
            Some(DependencyType::Consume)
        );
        assert_eq!(
            DependencyType::from_str("CONSUME"),
            Some(DependencyType::Consume)
        );
        assert_eq!(
            DependencyType::from_str("exclusive"),
            Some(DependencyType::Exclusive)
        );
        assert_eq!(
            DependencyType::from_str("invalid"),
            None
        );
    }

    /// 测试场景：DependencyType 的业务语义
    #[test]
    fn test_dependency_type_semantics() {
        // Prerequisite: 用户必须持有依赖徽章，但不会被消耗
        let prereq = DependencyType::Prerequisite;
        assert_eq!(prereq, DependencyType::Prerequisite);

        // Consume: 用户必须持有依赖徽章，且会被消耗（数量减少）
        let consume = DependencyType::Consume;
        assert_eq!(consume, DependencyType::Consume);

        // Exclusive: 互斥关系，持有此徽章则不能获得目标徽章
        let exclusive = DependencyType::Exclusive;
        assert_eq!(exclusive, DependencyType::Exclusive);
    }
}

mod cascade_config_integration {
    use super::*;

    /// 测试场景：级联配置的默认值
    #[test]
    fn test_cascade_config_default() {
        let config = CascadeConfig::default();

        // 默认最大深度应该合理（防止无限递归，但允许适度的级联）
        assert!(config.max_depth >= 5);
        assert!(config.max_depth <= 20);

        // 默认超时应该合理
        assert!(config.timeout_ms >= 1000);
        assert!(config.timeout_ms <= 30000);

        // 缓存时间应该合理
        assert!(config.graph_cache_seconds >= 60);
    }

    /// 测试场景：自定义级联配置
    #[test]
    fn test_cascade_config_custom() {
        let config = CascadeConfig {
            max_depth: 3,
            timeout_ms: 2000,
            graph_cache_seconds: 120,
        };

        assert_eq!(config.max_depth, 3);
        assert_eq!(config.timeout_ms, 2000);
        assert_eq!(config.graph_cache_seconds, 120);
    }
}

mod dependency_graph_advanced {
    use super::*;

    /// 测试场景：空依赖图
    #[test]
    fn test_empty_dependency_graph() {
        let graph = DependencyGraph::from_rows(vec![]);

        assert!(graph.is_empty());

        let random_badge = Uuid::new_v4();
        assert!(graph.get_triggered_by(random_badge).is_empty());
        assert!(graph.get_prerequisites(random_badge).is_empty());
        assert!(graph.get_exclusive_group("any_group").is_empty());
    }

    /// 测试场景：优先级排序
    ///
    /// 依赖应该按 priority 字段排序，低优先级值的依赖先被处理
    #[test]
    fn test_dependency_priority_ordering() {
        let source = Uuid::new_v4();
        let target1 = Uuid::new_v4();
        let target2 = Uuid::new_v4();
        let target3 = Uuid::new_v4();

        let mut dep1 = create_dependency_row(target1, source, "prerequisite", true, "g1", None);
        dep1.priority = 10;

        let mut dep2 = create_dependency_row(target2, source, "prerequisite", true, "g1", None);
        dep2.priority = 1;

        let mut dep3 = create_dependency_row(target3, source, "prerequisite", true, "g1", None);
        dep3.priority = 5;

        let graph = DependencyGraph::from_rows(vec![dep1, dep2, dep3]);

        let triggered = graph.get_triggered_by(source);
        assert_eq!(triggered.len(), 3);

        // 验证按优先级排序：1, 5, 10
        assert_eq!(triggered[0].priority, 1);
        assert_eq!(triggered[1].priority, 5);
        assert_eq!(triggered[2].priority, 10);
    }

    /// 测试场景：同一徽章的多个互斥组
    #[test]
    fn test_multiple_exclusive_groups() {
        let badge_a = Uuid::new_v4();
        let badge_b = Uuid::new_v4();
        let badge_c = Uuid::new_v4();
        let badge_d = Uuid::new_v4();

        let dependencies = vec![
            // A 和 B 在 group1 中互斥
            create_dependency_row(badge_a, Uuid::new_v4(), "exclusive", false, "g1", Some("group1")),
            create_dependency_row(badge_b, Uuid::new_v4(), "exclusive", false, "g1", Some("group1")),
            // C 和 D 在 group2 中互斥
            create_dependency_row(badge_c, Uuid::new_v4(), "exclusive", false, "g1", Some("group2")),
            create_dependency_row(badge_d, Uuid::new_v4(), "exclusive", false, "g1", Some("group2")),
        ];

        let graph = DependencyGraph::from_rows(dependencies);

        let group1 = graph.get_exclusive_group("group1");
        assert_eq!(group1.len(), 2);
        assert!(group1.contains(&badge_a));
        assert!(group1.contains(&badge_b));

        let group2 = graph.get_exclusive_group("group2");
        assert_eq!(group2.len(), 2);
        assert!(group2.contains(&badge_c));
        assert!(group2.contains(&badge_d));

        // 不同组不会混淆
        assert!(!group1.contains(&badge_c));
        assert!(!group2.contains(&badge_a));
    }
}
