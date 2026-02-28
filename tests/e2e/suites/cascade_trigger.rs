//! 级联触发测试套件
//!
//! 测试徽章依赖关系和级联发放逻辑。

use crate::data::*;
use crate::helpers::*;
use crate::setup::TestEnvironment;
use serde_json::json;
use std::time::Duration;

#[cfg(test)]
mod cascade_chain_tests {
    use super::*;

    /// 简单级联测试 A -> B
    ///
    /// 验证当用户获得徽章 A 后，自动触发级联评估并发放徽章 B。
    /// 此场景用于验证最基本的级联触发功能。
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_simple_cascade_a_to_b() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        let scenario = ScenarioBuilder::new(&env.api)
            .cascade_chain()
            .await
            .unwrap();

        // 配置 B 依赖 A 的级联关系，auto_trigger 为 true 确保 A 获得时自动评估 B
        env.api
            .create_dependency(
                scenario.badge_b.id,
                &CreateDependencyRequest {
                    depends_on_badge_id: scenario.badge_a.id,
                    dependency_type: "prerequisite".to_string(),
                    required_quantity: 1,
                    exclusive_group_id: None,
                    auto_trigger: true,
                    priority: 0,
                    dependency_group_id: "default".to_string(),
                },
            )
            .await
            .unwrap();

        // 级联评估器会自动检查前置条件并发放，不需要为 B 创建规则

        // 刷新依赖缓存和规则
        env.api.refresh_dependency_cache().await.unwrap();
        env.kafka.send_rule_reload().await.unwrap();
        env.wait_for_rule_reload().await.unwrap();

        let user_id = UserGenerator::user_id();

        // 触发获得 A 徽章（消费 200 元满足 A 的规则）
        let event = TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 200);
        env.kafka.send_transaction_event(event).await.unwrap();

        // 等待 A 发放（延长超时以适应 CI 环境）
        env.wait_for_badge(&user_id, scenario.badge_a.id, Duration::from_secs(15))
            .await
            .unwrap();

        // 验证 A 获得
        assert!(
            env.db
                .user_has_badge(&user_id, scenario.badge_a.id)
                .await
                .unwrap(),
            "用户应该获得徽章 A"
        );

        // 验证 B 也自动获得（级联触发，给予更长等待时间）
        let has_badge_b = env
            .wait_for_badge(&user_id, scenario.badge_b.id, Duration::from_secs(15))
            .await;
        assert!(has_badge_b.is_ok(), "用户应该通过级联自动获得徽章 B");

        // 验证级联日志
        let cascade_logs = env.db.get_cascade_logs(&user_id).await.unwrap();
        assert!(!cascade_logs.is_empty(), "应该有级联评估记录");

        // 验证级联日志中包含 A 触发 B 的记录
        let trigger_log = cascade_logs.iter().find(|log| {
            log.trigger_badge_id == scenario.badge_a.id
                && log.has_granted_badge(scenario.badge_b.id)
        });
        assert!(trigger_log.is_some(), "应该记录 A 触发 B 的级联评估");
        assert!(
            trigger_log.unwrap().is_success(),
            "级联评估结果应该是成功"
        );

        env.cleanup().await.unwrap();
    }

    /// 多级级联测试 A -> B -> C
    ///
    /// 验证三级级联传递：获得 A 触发 B，获得 B 再触发 C。
    /// 用于验证级联的递归评估能力。
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_multi_level_cascade() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        let scenario = ScenarioBuilder::new(&env.api)
            .cascade_chain()
            .await
            .unwrap();

        // 配置 B 依赖 A
        env.api
            .create_dependency(
                scenario.badge_b.id,
                &CreateDependencyRequest {
                    depends_on_badge_id: scenario.badge_a.id,
                    dependency_type: "prerequisite".to_string(),
                    required_quantity: 1,
                    exclusive_group_id: None,
                    auto_trigger: true,
                    priority: 0,
                    dependency_group_id: "default".to_string(),
                },
            )
            .await
            .unwrap();

        // 配置 C 依赖 B
        env.api
            .create_dependency(
                scenario.badge_c.id,
                &CreateDependencyRequest {
                    depends_on_badge_id: scenario.badge_b.id,
                    dependency_type: "prerequisite".to_string(),
                    required_quantity: 1,
                    exclusive_group_id: None,
                    auto_trigger: true,
                    priority: 0,
                    dependency_group_id: "default".to_string(),
                },
            )
            .await
            .unwrap();

        // 级联评估器会自动检查前置条件并发放 B 和 C，不需要创建规则

        env.api.refresh_dependency_cache().await.unwrap();
        env.kafka.send_rule_reload().await.unwrap();
        env.wait_for_rule_reload().await.unwrap();

        let user_id = UserGenerator::user_id();

        // 触发获得 A 徽章
        let event = TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 200);
        env.kafka.send_transaction_event(event).await.unwrap();

        // 等待 A 发放
        env.wait_for_badge(&user_id, scenario.badge_a.id, Duration::from_secs(15))
            .await
            .unwrap();

        // 验证 A 获得
        assert!(
            env.db
                .user_has_badge(&user_id, scenario.badge_a.id)
                .await
                .unwrap(),
            "用户应该获得徽章 A"
        );

        // 等待三级级联完成（B 和 C 需要更长时间）
        let has_badge_b = env
            .wait_for_badge(&user_id, scenario.badge_b.id, Duration::from_secs(15))
            .await;
        assert!(has_badge_b.is_ok(), "用户应该通过级联获得徽章 B");

        let has_badge_c = env
            .wait_for_badge(&user_id, scenario.badge_c.id, Duration::from_secs(15))
            .await;
        assert!(has_badge_c.is_ok(), "用户应该通过级联获得徽章 C");

        // 验证级联日志：A 触发一次评估，同时发放 B 和 C
        // 当前实现为每个 evaluate() 调用创建一条日志，包含所有级联发放结果
        let cascade_logs = env.db.get_cascade_logs(&user_id).await.unwrap();
        assert!(!cascade_logs.is_empty(), "应该有级联评估记录");

        // 验证 A 触发的评估记录包含了 B 和 C 的发放（A->B->C 的完整级联）
        let a_triggered = cascade_logs.iter().find(|log| {
            log.trigger_badge_id == scenario.badge_a.id
        });
        assert!(a_triggered.is_some(), "应该有 A 触发的级联记录");

        let log = a_triggered.unwrap();
        assert!(
            log.has_granted_badge(scenario.badge_b.id),
            "A 的级联评估应该发放 B"
        );
        assert!(
            log.has_granted_badge(scenario.badge_c.id),
            "A 的级联评估应该发放 C（通过 B 的级联）"
        );

        env.cleanup().await.unwrap();
    }

    /// 扇出级联测试 A -> [B, C, D]
    ///
    /// 验证一对多级联：获得 A 后同时触发 B、C、D 的评估。
    /// 用于验证并行级联评估能力。
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_fan_out_cascade() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 创建一个包含 4 个徽章的场景
        let category = env
            .api
            .create_category(&TestCategories::achievement())
            .await
            .unwrap();
        let series = env
            .api
            .create_series(&CreateSeriesRequest {
                category_id: category.id,
                name: "Test扇出级联系列".to_string(),
                description: Some("扇出级联测试".to_string()),
                cover_url: None,
                theme: Some("rainbow".to_string()),
            })
            .await
            .unwrap();

        // 创建徽章 A（触发源）
        let badge_a = env
            .api
            .create_badge(
                &CreateBadgeRequest::new(series.id, "Test徽章A-扇出", "NORMAL")
                    .with_description("扇出触发源"),
            )
            .await
            .unwrap();

        // 创建徽章 B、C、D（被触发目标）
        let badge_b = env
            .api
            .create_badge(
                &CreateBadgeRequest::new(series.id, "Test徽章B-扇出", "NORMAL")
                    .with_description("扇出目标B"),
            )
            .await
            .unwrap();

        let badge_c = env
            .api
            .create_badge(
                &CreateBadgeRequest::new(series.id, "Test徽章C-扇出", "NORMAL")
                    .with_description("扇出目标C"),
            )
            .await
            .unwrap();

        let badge_d = env
            .api
            .create_badge(
                &CreateBadgeRequest::new(series.id, "Test徽章D-扇出", "NORMAL")
                    .with_description("扇出目标D"),
            )
            .await
            .unwrap();

        // 为 A 创建触发规则并发布
        let rule_a = env
            .api
            .create_rule(&CreateRuleRequest {
                badge_id: badge_a.id,
                rule_code: format!("test_fanout_a_{}", badge_a.id),
                name: "Test扇出徽章A规则".to_string(),
                event_type: "purchase".to_string(),
                rule_json: json!({
                    "type": "condition",
                    "field": "amount",
                    "operator": "gte",
                    "value": 100
                }),
                start_time: None,
                end_time: None,
                max_count_per_user: Some(1),
                global_quota: None,
            })
            .await
            .unwrap();
        env.api.publish_rule(rule_a.id).await.unwrap();

        // 配置 B、C、D 都依赖 A（级联评估器会自动检查前置条件并发放）
        for target_badge in [&badge_b, &badge_c, &badge_d] {
            env.api
                .create_dependency(
                    target_badge.id,
                    &CreateDependencyRequest {
                        depends_on_badge_id: badge_a.id,
                        dependency_type: "prerequisite".to_string(),
                        required_quantity: 1,
                        exclusive_group_id: None,
                        auto_trigger: true,
                        priority: 0,
                        dependency_group_id: "default".to_string(),
                    },
                )
                .await
                .unwrap();
        }

        // 上线所有徽章
        env.api
            .update_badge_status(badge_a.id, "active")
            .await
            .unwrap();
        env.api
            .update_badge_status(badge_b.id, "active")
            .await
            .unwrap();
        env.api
            .update_badge_status(badge_c.id, "active")
            .await
            .unwrap();
        env.api
            .update_badge_status(badge_d.id, "active")
            .await
            .unwrap();

        env.api.refresh_dependency_cache().await.unwrap();
        env.kafka.send_rule_reload().await.unwrap();
        env.wait_for_rule_reload().await.unwrap();

        let user_id = UserGenerator::user_id();

        // 触发获得 A 徽章
        let event = TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 200);
        env.kafka.send_transaction_event(event).await.unwrap();

        // 等待 A 发放
        env.wait_for_badge(&user_id, badge_a.id, Duration::from_secs(15))
            .await
            .unwrap();

        // 验证 A 获得
        assert!(
            env.db.user_has_badge(&user_id, badge_a.id).await.unwrap(),
            "用户应该获得徽章 A"
        );

        // 验证 B、C、D 都通过级联获得（给予更长等待时间）
        for (badge, name) in [(badge_b.id, "B"), (badge_c.id, "C"), (badge_d.id, "D")] {
            let has_badge = env
                .wait_for_badge(&user_id, badge, Duration::from_secs(15))
                .await;
            assert!(has_badge.is_ok(), "用户应该通过级联获得徽章 {}", name);
        }

        // 验证级联日志：A 触发一次评估，发放 B、C、D
        let cascade_logs = env.db.get_cascade_logs(&user_id).await.unwrap();
        let a_triggered_logs: Vec<_> = cascade_logs
            .iter()
            .filter(|log| log.trigger_badge_id == badge_a.id)
            .collect();
        assert!(!a_triggered_logs.is_empty(), "应该有 A 触发的级联评估记录");

        // 验证 A 触发的评估记录包含了 B、C、D 的发放
        let log = &a_triggered_logs[0];
        assert!(
            log.has_granted_badge(badge_b.id),
            "级联日志应包含 B 的发放记录"
        );
        assert!(
            log.has_granted_badge(badge_c.id),
            "级联日志应包含 C 的发放记录"
        );
        assert!(
            log.has_granted_badge(badge_d.id),
            "级联日志应包含 D 的发放记录"
        );

        env.cleanup().await.unwrap();
    }
}

#[cfg(test)]
mod prerequisite_tests {
    use super::*;

    /// 前置条件未满足时阻止发放测试
    ///
    /// 验证当用户未持有前置徽章时，即使满足其他条件也不能获得目标徽章。
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_prerequisite_not_met_blocks_grant() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        let scenario = ScenarioBuilder::new(&env.api)
            .cascade_chain()
            .await
            .unwrap();

        // 配置 B 依赖 A（前置条件）
        env.api
            .create_dependency(
                scenario.badge_b.id,
                &CreateDependencyRequest {
                    depends_on_badge_id: scenario.badge_a.id,
                    dependency_type: "prerequisite".to_string(),
                    required_quantity: 1,
                    exclusive_group_id: None,
                    auto_trigger: false,
                    priority: 0,
                    dependency_group_id: "default".to_string(),
                },
            )
            .await
            .unwrap();

        // 为 B 创建一个基于消费金额的规则（不依赖级联事件）
        let rule_b = env
            .api
            .create_rule(&CreateRuleRequest {
                badge_id: scenario.badge_b.id,
                rule_code: format!("test_prereq_b_{}", scenario.badge_b.id),
                name: "Test徽章B消费规则".to_string(),
                event_type: "purchase".to_string(),
                rule_json: json!({
                    "type": "condition",
                    "field": "amount",
                    "operator": "gte",
                    "value": 50
                }),
                start_time: None,
                end_time: None,
                max_count_per_user: Some(1),
                global_quota: None,
            })
            .await
            .unwrap();

        // 发布规则使其生效，确保测试验证的是前置条件阻止发放而非规则未启用
        env.api.publish_rule(rule_b.id).await.unwrap();

        env.api.refresh_dependency_cache().await.unwrap();
        env.kafka.send_rule_reload().await.unwrap();
        env.wait_for_rule_reload().await.unwrap();

        let user_id = UserGenerator::user_id();

        // 发送消费事件，金额满足 B 的规则但不满足 A 的规则
        let event = TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 80);
        env.kafka.send_transaction_event(event).await.unwrap();
        env.wait_for_processing(Duration::from_secs(5))
            .await
            .unwrap();

        // 验证用户没有获得 A（金额不足）
        assert!(
            !env.db
                .user_has_badge(&user_id, scenario.badge_a.id)
                .await
                .unwrap(),
            "用户不应该获得徽章 A"
        );

        // 验证用户也没有获得 B（前置条件不满足）
        // 注：前置条件检查现在在 grant_service 中进行，规则引擎评估通过后
        // grant_badge 会检查前置条件并返回错误，因此 B 不会被发放
        assert!(
            !env.db
                .user_has_badge(&user_id, scenario.badge_b.id)
                .await
                .unwrap(),
            "前置条件不满足时，用户不应该获得徽章 B"
        );

        env.cleanup().await.unwrap();
    }

    /// 前置条件满足时允许发放测试
    ///
    /// 验证当用户已持有前置徽章时，满足规则条件后能正确获得目标徽章。
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_prerequisite_met_allows_grant() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        let scenario = ScenarioBuilder::new(&env.api)
            .cascade_chain()
            .await
            .unwrap();

        // 配置 B 依赖 A（前置条件），并设置 auto_trigger
        env.api
            .create_dependency(
                scenario.badge_b.id,
                &CreateDependencyRequest {
                    depends_on_badge_id: scenario.badge_a.id,
                    dependency_type: "prerequisite".to_string(),
                    required_quantity: 1,
                    exclusive_group_id: None,
                    auto_trigger: true,
                    priority: 0,
                    dependency_group_id: "default".to_string(),
                },
            )
            .await
            .unwrap();

        // 级联评估器会自动检查前置条件并发放 B，不需要创建规则

        env.api.refresh_dependency_cache().await.unwrap();
        env.kafka.send_rule_reload().await.unwrap();
        env.wait_for_rule_reload().await.unwrap();

        let user_id = UserGenerator::user_id();

        // 发送消费事件，金额满足 A 的规则（>=100）
        let event = TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 200);
        env.kafka.send_transaction_event(event).await.unwrap();

        // 等待 A 发放
        env.wait_for_badge(&user_id, scenario.badge_a.id, Duration::from_secs(15))
            .await
            .unwrap();

        // 验证用户获得 A
        assert!(
            env.db
                .user_has_badge(&user_id, scenario.badge_a.id)
                .await
                .unwrap(),
            "用户应该获得徽章 A"
        );

        // 验证用户也获得 B（前置条件满足 + 级联触发，给予更长等待时间）
        let has_badge_b = env
            .wait_for_badge(&user_id, scenario.badge_b.id, Duration::from_secs(15))
            .await;
        assert!(
            has_badge_b.is_ok(),
            "前置条件满足后，用户应该通过级联获得徽章 B"
        );

        // 验证级联日志中有成功的记录
        let cascade_logs = env.db.get_cascade_logs(&user_id).await.unwrap();
        let granted_log = cascade_logs.iter().find(|log| {
            log.trigger_badge_id == scenario.badge_a.id
                && log.has_granted_badge(scenario.badge_b.id)
                && log.is_success()
        });
        assert!(granted_log.is_some(), "应该记录级联发放成功");

        env.cleanup().await.unwrap();
    }
}

#[cfg(test)]
mod mutual_exclusion_tests {
    use super::*;

    /// 互斥徽章测试
    ///
    /// 验证当用户已持有互斥组中的一个徽章时，不能再获得同组的其他徽章。
    /// 适用于「铂金会员」与「钻石会员」等互斥等级场景。
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_mutual_exclusion_blocks_second_badge() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        // 创建互斥徽章场景
        let category = env
            .api
            .create_category(&TestCategories::achievement())
            .await
            .unwrap();
        let series = env
            .api
            .create_series(&CreateSeriesRequest {
                category_id: category.id,
                name: "Test会员等级".to_string(),
                description: Some("互斥会员等级测试".to_string()),
                cover_url: None,
                theme: Some("gold".to_string()),
            })
            .await
            .unwrap();

        // 创建两个互斥徽章
        let badge_platinum = env
            .api
            .create_badge(
                &CreateBadgeRequest::new(series.id, "Test铂金会员", "NORMAL")
                    .with_description("铂金会员徽章"),
            )
            .await
            .unwrap();

        let badge_diamond = env
            .api
            .create_badge(
                &CreateBadgeRequest::new(series.id, "Test钻石会员", "NORMAL")
                    .with_description("钻石会员徽章"),
            )
            .await
            .unwrap();

        // 为徽章创建规则并发布
        let rule_platinum = env
            .api
            .create_rule(&CreateRuleRequest {
                badge_id: badge_platinum.id,
                rule_code: format!("test_platinum_{}", badge_platinum.id),
                name: "Test铂金会员规则".to_string(),
                event_type: "purchase".to_string(),
                rule_json: json!({
                    "type": "condition",
                    "field": "amount",
                    "operator": "gte",
                    "value": 100
                }),
                start_time: None,
                end_time: None,
                max_count_per_user: Some(1),
                global_quota: None,
            })
            .await
            .unwrap();
        env.api.publish_rule(rule_platinum.id).await.unwrap();

        let rule_diamond = env
            .api
            .create_rule(&CreateRuleRequest {
                badge_id: badge_diamond.id,
                rule_code: format!("test_diamond_{}", badge_diamond.id),
                name: "Test钻石会员规则".to_string(),
                event_type: "purchase".to_string(),
                rule_json: json!({
                    "type": "condition",
                    "field": "amount",
                    "operator": "gte",
                    "value": 200
                }),
                start_time: None,
                end_time: None,
                max_count_per_user: Some(1),
                global_quota: None,
            })
            .await
            .unwrap();
        env.api.publish_rule(rule_diamond.id).await.unwrap();

        // 配置互斥关系：铂金和钻石互斥
        let exclusive_group = "membership_level";
        env.api
            .create_dependency(
                badge_platinum.id,
                &CreateDependencyRequest {
                    depends_on_badge_id: badge_diamond.id,
                    dependency_type: "exclusive".to_string(),
                    required_quantity: 1,
                    exclusive_group_id: Some(exclusive_group.to_string()),
                    auto_trigger: false,
                    priority: 0,
                    dependency_group_id: "default".to_string(),
                },
            )
            .await
            .unwrap();

        env.api
            .create_dependency(
                badge_diamond.id,
                &CreateDependencyRequest {
                    depends_on_badge_id: badge_platinum.id,
                    dependency_type: "exclusive".to_string(),
                    required_quantity: 1,
                    exclusive_group_id: Some(exclusive_group.to_string()),
                    auto_trigger: false,
                    priority: 0,
                    dependency_group_id: "default".to_string(),
                },
            )
            .await
            .unwrap();

        env.api
            .update_badge_status(badge_platinum.id, "active")
            .await
            .unwrap();
        env.api
            .update_badge_status(badge_diamond.id, "active")
            .await
            .unwrap();

        env.api.refresh_dependency_cache().await.unwrap();
        env.kafka.send_rule_reload().await.unwrap();
        env.wait_for_rule_reload().await.unwrap();

        let user_id = UserGenerator::user_id();

        // 第一次购买，获得铂金会员
        let event1 = TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 150);
        env.kafka.send_transaction_event(event1).await.unwrap();

        // 等待铂金徽章发放
        env.wait_for_badge(&user_id, badge_platinum.id, Duration::from_secs(15))
            .await
            .unwrap();

        assert!(
            env.db
                .user_has_badge(&user_id, badge_platinum.id)
                .await
                .unwrap(),
            "用户应该获得铂金会员徽章"
        );

        // 第二次购买，金额满足钻石会员条件，但由于互斥应该被阻止
        let event2 = TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 300);
        env.kafka.send_transaction_event(event2).await.unwrap();
        // 等待事件处理完成
        env.wait_for_processing(Duration::from_secs(8))
            .await
            .unwrap();

        assert!(
            !env.db
                .user_has_badge(&user_id, badge_diamond.id)
                .await
                .unwrap(),
            "由于互斥关系，用户不应该获得钻石会员徽章"
        );

        // 验证仍然持有铂金
        assert!(
            env.db
                .user_has_badge(&user_id, badge_platinum.id)
                .await
                .unwrap(),
            "用户应该继续持有铂金会员徽章"
        );

        // 注：互斥检查现在在 grant_service 中进行，规则引擎评估通过后
        // grant_badge 会检查互斥组并返回错误，因此钻石会员不会被发放

        env.cleanup().await.unwrap();
    }
}

#[cfg(test)]
mod cycle_detection_tests {
    use super::*;

    /// 循环依赖检测测试
    ///
    /// 验证系统能正确检测并阻止循环依赖（A -> B -> C -> A）。
    /// 循环依赖会导致无限递归，必须在配置时或运行时检测并阻止。
    #[tokio::test]
    #[ignore = "需要运行服务"]
    async fn test_cycle_detected_and_blocked() {
        let env = TestEnvironment::setup().await.unwrap();
        env.prepare_test_data().await.unwrap();

        let scenario = ScenarioBuilder::new(&env.api)
            .cascade_chain()
            .await
            .unwrap();

        // 配置 B 依赖 A
        env.api
            .create_dependency(
                scenario.badge_b.id,
                &CreateDependencyRequest {
                    depends_on_badge_id: scenario.badge_a.id,
                    dependency_type: "prerequisite".to_string(),
                    required_quantity: 1,
                    exclusive_group_id: None,
                    auto_trigger: true,
                    priority: 0,
                    dependency_group_id: "default".to_string(),
                },
            )
            .await
            .unwrap();

        // 配置 C 依赖 B
        env.api
            .create_dependency(
                scenario.badge_c.id,
                &CreateDependencyRequest {
                    depends_on_badge_id: scenario.badge_b.id,
                    dependency_type: "prerequisite".to_string(),
                    required_quantity: 1,
                    exclusive_group_id: None,
                    auto_trigger: true,
                    priority: 0,
                    dependency_group_id: "default".to_string(),
                },
            )
            .await
            .unwrap();

        // 尝试配置 A 依赖 C（形成循环 A -> B -> C -> A）
        let cycle_result = env
            .api
            .create_dependency(
                scenario.badge_a.id,
                &CreateDependencyRequest {
                    depends_on_badge_id: scenario.badge_c.id,
                    dependency_type: "prerequisite".to_string(),
                    required_quantity: 1,
                    exclusive_group_id: None,
                    auto_trigger: true,
                    priority: 0,
                    dependency_group_id: "default".to_string(),
                },
            )
            .await;

        // 验证循环依赖被阻止（API 应该返回错误）
        assert!(cycle_result.is_err(), "创建循环依赖应该失败");

        // 如果 API 层没有阻止，验证运行时能正确处理
        if cycle_result.is_ok() {
            env.api.refresh_dependency_cache().await.unwrap();
            env.kafka.send_rule_reload().await.unwrap();
            env.wait_for_rule_reload().await.unwrap();

            let user_id = UserGenerator::user_id();

            // 触发获得 A 徽章
            let event = TransactionEvent::purchase(&user_id, &OrderGenerator::order_id(), 200);
            env.kafka.send_transaction_event(event).await.unwrap();
            env.wait_for_processing(Duration::from_secs(5))
                .await
                .unwrap();

            // 即使有循环依赖配置，A 本身应该能被发放
            assert!(
                env.db
                    .user_has_badge(&user_id, scenario.badge_a.id)
                    .await
                    .unwrap(),
                "A 徽章应该被发放（循环不影响起始点）"
            );

            // 验证级联日志中有循环检测记录
            let cascade_logs = env.db.get_cascade_logs(&user_id).await.unwrap();
            let cycle_log = cascade_logs
                .iter()
                .find(|log| log.is_cycle_detected());

            // 如果存在循环日志，验证其内容
            if cycle_log.is_some() {
                assert!(
                    cascade_logs
                        .iter()
                        .any(|log| log.is_cycle_detected()),
                    "应该记录循环检测"
                );
            }
        }

        env.cleanup().await.unwrap();
    }
}
