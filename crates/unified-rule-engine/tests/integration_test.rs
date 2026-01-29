//! 规则引擎集成测试
//!
//! 测试完整的规则加载、编译、执行工作流。

use rule_engine::{
    Condition, EvaluationContext, LogicalGroup, Operator, Rule, RuleCompiler, RuleExecutor,
    RuleNode, RuleStore,
};
use serde_json::json;

/// 创建测试上下文：模拟一个购买事件
fn create_purchase_context() -> EvaluationContext {
    EvaluationContext::new(json!({
        "event": {
            "type": "PURCHASE",
            "timestamp": "2024-01-15T10:00:00Z",
            "source": "mobile_app"
        },
        "order": {
            "id": "order-12345",
            "amount": 1500,
            "currency": "CNY",
            "items": [
                {"sku": "TICKET-001", "name": "门票", "price": 500, "quantity": 2},
                {"sku": "FOOD-001", "name": "餐饮", "price": 500, "quantity": 1}
            ],
            "category": "park_visit"
        },
        "user": {
            "id": "user-67890",
            "level": "gold",
            "is_vip": true,
            "tags": ["frequent_visitor", "annual_pass"],
            "registered_at": "2023-01-01T00:00:00Z",
            "total_purchases": 15000
        },
        "location": {
            "park": "shanghai",
            "zone": "adventure_isle"
        }
    }))
}

/// 创建测试上下文：模拟一个退款事件
fn create_refund_context() -> EvaluationContext {
    EvaluationContext::new(json!({
        "event": {
            "type": "REFUND",
            "timestamp": "2024-01-16T14:30:00Z",
            "source": "customer_service"
        },
        "order": {
            "id": "order-12345",
            "amount": 500,
            "currency": "CNY",
            "original_order_id": "order-12340"
        },
        "user": {
            "id": "user-67890",
            "level": "silver",
            "is_vip": false,
            "tags": [],
            "total_purchases": 3000
        }
    }))
}

// ==================== 完整工作流测试 ====================

#[test]
fn test_full_workflow_with_store() {
    // 1. 创建存储
    let store = RuleStore::new();

    // 2. 加载规则
    let rule_json = r#"
    {
        "id": "vip-purchase-badge",
        "name": "VIP高额消费徽章",
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
                    "field": "user.is_vip",
                    "operator": "eq",
                    "value": true
                },
                {
                    "type": "condition",
                    "field": "order.amount",
                    "operator": "gte",
                    "value": 1000
                }
            ]
        }
    }
    "#;

    store.load_from_json(rule_json).unwrap();
    assert_eq!(store.len(), 1);

    // 3. 获取编译后的规则
    let compiled = store.get("vip-purchase-badge").unwrap();
    assert_eq!(compiled.required_fields.len(), 3);

    // 4. 执行评估
    let executor = RuleExecutor::new();
    let context = create_purchase_context();
    let result = executor.execute(&compiled, &context).unwrap();

    // 5. 验证结果
    assert!(result.matched);
    assert_eq!(result.rule_id, "vip-purchase-badge");
    assert_eq!(result.matched_conditions.len(), 3);
}

// ==================== 复杂规则测试 ====================

#[test]
fn test_nested_logical_groups() {
    let mut compiler = RuleCompiler::new();

    // (event.type == PURCHASE) AND ((amount >= 2000) OR (is_vip AND level == gold))
    let rule_json = r#"
    {
        "id": "complex-rule",
        "name": "复杂嵌套规则",
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
                            "type": "group",
                            "operator": "AND",
                            "children": [
                                {
                                    "type": "condition",
                                    "field": "user.is_vip",
                                    "operator": "eq",
                                    "value": true
                                },
                                {
                                    "type": "condition",
                                    "field": "user.level",
                                    "operator": "eq",
                                    "value": "gold"
                                }
                            ]
                        }
                    ]
                }
            ]
        }
    }
    "#;

    let compiled = compiler.compile_from_json(rule_json).unwrap();
    let executor = RuleExecutor::new().with_trace();
    let context = create_purchase_context();

    let result = executor.execute(&compiled, &context).unwrap();

    // amount=1500 < 2000, 但 is_vip=true AND level=gold 满足
    assert!(result.matched);
}

// ==================== 各种操作符测试 ====================

#[test]
fn test_between_operator() {
    let mut compiler = RuleCompiler::new();
    let rule_json = r#"
    {
        "id": "amount-range",
        "name": "金额范围规则",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "order.amount",
            "operator": "between",
            "value": [1000, 2000]
        }
    }
    "#;

    let compiled = compiler.compile_from_json(rule_json).unwrap();
    let executor = RuleExecutor::new();

    // amount=1500，在范围内
    let context = create_purchase_context();
    let result = executor.execute(&compiled, &context).unwrap();
    assert!(result.matched);
}

#[test]
fn test_in_operator() {
    let mut compiler = RuleCompiler::new();
    let rule_json = r#"
    {
        "id": "category-check",
        "name": "类目检查规则",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "order.category",
            "operator": "in",
            "value": ["park_visit", "hotel_booking", "dining"]
        }
    }
    "#;

    let compiled = compiler.compile_from_json(rule_json).unwrap();
    let executor = RuleExecutor::new();
    let context = create_purchase_context();

    let result = executor.execute(&compiled, &context).unwrap();
    assert!(result.matched); // category=park_visit
}

#[test]
fn test_contains_operator_array() {
    let mut compiler = RuleCompiler::new();
    let rule_json = r#"
    {
        "id": "tag-check",
        "name": "用户标签检查",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "user.tags",
            "operator": "contains",
            "value": "annual_pass"
        }
    }
    "#;

    let compiled = compiler.compile_from_json(rule_json).unwrap();
    let executor = RuleExecutor::new();
    let context = create_purchase_context();

    let result = executor.execute(&compiled, &context).unwrap();
    assert!(result.matched);
}

#[test]
fn test_contains_any_operator() {
    let mut compiler = RuleCompiler::new();
    let rule_json = r#"
    {
        "id": "any-tag-check",
        "name": "任意标签检查",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "user.tags",
            "operator": "contains_any",
            "value": ["premium_member", "annual_pass", "club33"]
        }
    }
    "#;

    let compiled = compiler.compile_from_json(rule_json).unwrap();
    let executor = RuleExecutor::new();
    let context = create_purchase_context();

    let result = executor.execute(&compiled, &context).unwrap();
    assert!(result.matched); // 包含 annual_pass
}

#[test]
fn test_time_comparison() {
    let mut compiler = RuleCompiler::new();
    let rule_json = r#"
    {
        "id": "time-check",
        "name": "时间检查规则",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "event.timestamp",
            "operator": "after",
            "value": "2024-01-01T00:00:00Z"
        }
    }
    "#;

    let compiled = compiler.compile_from_json(rule_json).unwrap();
    let executor = RuleExecutor::new();
    let context = create_purchase_context();

    let result = executor.execute(&compiled, &context).unwrap();
    assert!(result.matched); // 2024-01-15 > 2024-01-01
}

#[test]
fn test_regex_operator() {
    let mut compiler = RuleCompiler::new();
    let rule_json = r#"
    {
        "id": "order-id-format",
        "name": "订单ID格式检查",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "order.id",
            "operator": "regex",
            "value": "^order-\\d+$"
        }
    }
    "#;

    let compiled = compiler.compile_from_json(rule_json).unwrap();
    let executor = RuleExecutor::new();
    let context = create_purchase_context();

    let result = executor.execute(&compiled, &context).unwrap();
    assert!(result.matched); // order-12345 匹配
}

// ==================== 短路求值测试 ====================

#[test]
fn test_and_short_circuit() {
    let mut compiler = RuleCompiler::new();

    // 第一个条件就不满足，后续条件不应该被评估
    let rule_json = r#"
    {
        "id": "and-short-circuit",
        "name": "AND短路测试",
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
                    "field": "nonexistent.field",
                    "operator": "eq",
                    "value": "should_not_be_evaluated"
                }
            ]
        }
    }
    "#;

    let compiled = compiler.compile_from_json(rule_json).unwrap();
    let executor = RuleExecutor::new().with_trace();
    let context = create_purchase_context(); // event.type = PURCHASE

    let result = executor.execute(&compiled, &context).unwrap();
    assert!(!result.matched);
    // 检查追踪信息中有短路
    assert!(result.evaluation_trace.iter().any(|t| t.contains("短路")));
}

#[test]
fn test_or_short_circuit() {
    let mut compiler = RuleCompiler::new();

    // 第一个条件就满足，后续条件不应该被评估
    let rule_json = r#"
    {
        "id": "or-short-circuit",
        "name": "OR短路测试",
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
                    "field": "nonexistent.field",
                    "operator": "eq",
                    "value": "should_not_be_evaluated"
                }
            ]
        }
    }
    "#;

    let compiled = compiler.compile_from_json(rule_json).unwrap();
    let executor = RuleExecutor::new().with_trace();
    let context = create_purchase_context();

    let result = executor.execute(&compiled, &context).unwrap();
    assert!(result.matched);
    assert!(result.evaluation_trace.iter().any(|t| t.contains("短路")));
}

// ==================== 边界情况测试 ====================

#[test]
fn test_missing_field() {
    let mut compiler = RuleCompiler::new();
    let rule_json = r#"
    {
        "id": "missing-field",
        "name": "缺失字段测试",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "nonexistent.path.to.field",
            "operator": "eq",
            "value": "something"
        }
    }
    "#;

    let compiled = compiler.compile_from_json(rule_json).unwrap();
    let executor = RuleExecutor::new();
    let context = create_purchase_context();

    let result = executor.execute(&compiled, &context).unwrap();
    assert!(!result.matched); // 字段不存在，返回 false
}

#[test]
fn test_is_empty_on_missing_field() {
    let mut compiler = RuleCompiler::new();
    let rule_json = r#"
    {
        "id": "empty-check",
        "name": "空值检查",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "nonexistent.field",
            "operator": "is_empty",
            "value": null
        }
    }
    "#;

    let compiled = compiler.compile_from_json(rule_json).unwrap();
    let executor = RuleExecutor::new();
    let context = create_purchase_context();

    let result = executor.execute(&compiled, &context).unwrap();
    assert!(result.matched); // 不存在的字段被认为是空的
}

#[test]
fn test_array_index_access() {
    let mut compiler = RuleCompiler::new();
    let rule_json = r#"
    {
        "id": "array-access",
        "name": "数组索引访问",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "order.items.0.sku",
            "operator": "eq",
            "value": "TICKET-001"
        }
    }
    "#;

    let compiled = compiler.compile_from_json(rule_json).unwrap();
    let executor = RuleExecutor::new();
    let context = create_purchase_context();

    let result = executor.execute(&compiled, &context).unwrap();
    assert!(result.matched);
}

// ==================== 不同事件类型测试 ====================

#[test]
fn test_refund_event_not_match_purchase_rule() {
    let mut compiler = RuleCompiler::new();
    let rule_json = r#"
    {
        "id": "purchase-only",
        "name": "仅购买规则",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "event.type",
            "operator": "eq",
            "value": "PURCHASE"
        }
    }
    "#;

    let compiled = compiler.compile_from_json(rule_json).unwrap();
    let executor = RuleExecutor::new();
    let context = create_refund_context();

    let result = executor.execute(&compiled, &context).unwrap();
    assert!(!result.matched);
}

// ==================== 批量操作测试 ====================

#[test]
fn test_batch_rule_evaluation() {
    let store = RuleStore::new();

    // 加载多条规则
    let rules = vec![
        r#"{"id": "rule-1", "name": "购买规则", "version": "1.0", "root": {"type": "condition", "field": "event.type", "operator": "eq", "value": "PURCHASE"}}"#,
        r#"{"id": "rule-2", "name": "VIP规则", "version": "1.0", "root": {"type": "condition", "field": "user.is_vip", "operator": "eq", "value": true}}"#,
        r#"{"id": "rule-3", "name": "高额规则", "version": "1.0", "root": {"type": "condition", "field": "order.amount", "operator": "gte", "value": 5000}}"#,
    ];

    for rule_json in rules {
        store.load_from_json(rule_json).unwrap();
    }

    assert_eq!(store.len(), 3);

    let executor = RuleExecutor::new();
    let context = create_purchase_context();

    // 评估所有规则
    let mut matched_count = 0;
    for id in store.list_ids() {
        let rule = store.get(&id).unwrap();
        let result = executor.execute(&rule, &context).unwrap();
        if result.matched {
            matched_count += 1;
        }
    }

    // PURCHASE 和 is_vip 匹配，amount=1500 < 5000 不匹配
    assert_eq!(matched_count, 2);
}

// ==================== 规则更新测试 ====================

#[test]
fn test_rule_update() {
    let store = RuleStore::new();
    let executor = RuleExecutor::new();
    let context = create_purchase_context();

    // 初始规则：amount >= 2000（不匹配）
    let rule_v1 = r#"
    {
        "id": "amount-rule",
        "name": "金额规则",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "order.amount",
            "operator": "gte",
            "value": 2000
        }
    }
    "#;

    store.load_from_json(rule_v1).unwrap();
    let compiled = store.get("amount-rule").unwrap();
    let result = executor.execute(&compiled, &context).unwrap();
    assert!(!result.matched); // 1500 < 2000

    // 更新规则：amount >= 1000（匹配）
    let rule_v2: Rule = serde_json::from_str(
        r#"
        {
            "id": "amount-rule",
            "name": "金额规则（已更新）",
            "version": "2.0",
            "root": {
                "type": "condition",
                "field": "order.amount",
                "operator": "gte",
                "value": 1000
            }
        }
        "#,
    )
    .unwrap();

    store.update(rule_v2).unwrap();
    let compiled = store.get("amount-rule").unwrap();
    let result = executor.execute(&compiled, &context).unwrap();
    assert!(result.matched); // 1500 >= 1000
    assert_eq!(compiled.rule.name, "金额规则（已更新）");
}

// ==================== 额外的操作符测试 ====================

#[test]
fn test_not_in_operator() {
    let mut compiler = RuleCompiler::new();
    let rule_json = r#"
    {
        "id": "not-in-test",
        "name": "不在列表中测试",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "user.level",
            "operator": "not_in",
            "value": ["bronze", "silver"]
        }
    }
    "#;

    let compiled = compiler.compile_from_json(rule_json).unwrap();
    let executor = RuleExecutor::new();
    let context = create_purchase_context();

    let result = executor.execute(&compiled, &context).unwrap();
    assert!(result.matched); // level=gold 不在 [bronze, silver] 中
}

#[test]
fn test_neq_operator() {
    let mut compiler = RuleCompiler::new();
    let rule_json = r#"
    {
        "id": "neq-test",
        "name": "不等于测试",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "event.type",
            "operator": "neq",
            "value": "REFUND"
        }
    }
    "#;

    let compiled = compiler.compile_from_json(rule_json).unwrap();
    let executor = RuleExecutor::new();
    let context = create_purchase_context();

    let result = executor.execute(&compiled, &context).unwrap();
    assert!(result.matched); // event.type=PURCHASE != REFUND
}

#[test]
fn test_starts_with_operator() {
    let mut compiler = RuleCompiler::new();
    let rule_json = r#"
    {
        "id": "starts-with-test",
        "name": "前缀测试",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "order.id",
            "operator": "starts_with",
            "value": "order-"
        }
    }
    "#;

    let compiled = compiler.compile_from_json(rule_json).unwrap();
    let executor = RuleExecutor::new();
    let context = create_purchase_context();

    let result = executor.execute(&compiled, &context).unwrap();
    assert!(result.matched); // order-12345 以 order- 开头
}

#[test]
fn test_ends_with_operator() {
    let mut compiler = RuleCompiler::new();
    let rule_json = r#"
    {
        "id": "ends-with-test",
        "name": "后缀测试",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "order.id",
            "operator": "ends_with",
            "value": "12345"
        }
    }
    "#;

    let compiled = compiler.compile_from_json(rule_json).unwrap();
    let executor = RuleExecutor::new();
    let context = create_purchase_context();

    let result = executor.execute(&compiled, &context).unwrap();
    assert!(result.matched); // order-12345 以 12345 结尾
}

#[test]
fn test_contains_all_operator() {
    let mut compiler = RuleCompiler::new();
    let rule_json = r#"
    {
        "id": "contains-all-test",
        "name": "全部包含测试",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "user.tags",
            "operator": "contains_all",
            "value": ["frequent_visitor", "annual_pass"]
        }
    }
    "#;

    let compiled = compiler.compile_from_json(rule_json).unwrap();
    let executor = RuleExecutor::new();
    let context = create_purchase_context();

    let result = executor.execute(&compiled, &context).unwrap();
    assert!(result.matched); // tags 包含两个值
}

#[test]
fn test_is_not_empty_operator() {
    let mut compiler = RuleCompiler::new();
    let rule_json = r#"
    {
        "id": "not-empty-test",
        "name": "非空测试",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "user.tags",
            "operator": "is_not_empty",
            "value": null
        }
    }
    "#;

    let compiled = compiler.compile_from_json(rule_json).unwrap();
    let executor = RuleExecutor::new();
    let context = create_purchase_context();

    let result = executor.execute(&compiled, &context).unwrap();
    assert!(result.matched); // tags 不为空
}

#[test]
fn test_before_time_operator() {
    let mut compiler = RuleCompiler::new();
    let rule_json = r#"
    {
        "id": "before-time-test",
        "name": "时间之前测试",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "event.timestamp",
            "operator": "before",
            "value": "2024-12-31T23:59:59Z"
        }
    }
    "#;

    let compiled = compiler.compile_from_json(rule_json).unwrap();
    let executor = RuleExecutor::new();
    let context = create_purchase_context();

    let result = executor.execute(&compiled, &context).unwrap();
    assert!(result.matched); // 2024-01-15 < 2024-12-31
}

#[test]
fn test_lt_and_gt_operators() {
    let mut compiler = RuleCompiler::new();

    // 测试 lt (小于)
    let lt_rule = r#"
    {
        "id": "lt-test",
        "name": "小于测试",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "order.amount",
            "operator": "lt",
            "value": 2000
        }
    }
    "#;

    let compiled = compiler.compile_from_json(lt_rule).unwrap();
    let executor = RuleExecutor::new();
    let context = create_purchase_context();

    let result = executor.execute(&compiled, &context).unwrap();
    assert!(result.matched); // 1500 < 2000

    // 测试 gt (大于)
    let gt_rule = r#"
    {
        "id": "gt-test",
        "name": "大于测试",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "order.amount",
            "operator": "gt",
            "value": 1000
        }
    }
    "#;

    let compiled = compiler.compile_from_json(gt_rule).unwrap();
    let result = executor.execute(&compiled, &context).unwrap();
    assert!(result.matched); // 1500 > 1000
}

#[test]
fn test_lte_operator() {
    let mut compiler = RuleCompiler::new();
    let rule_json = r#"
    {
        "id": "lte-test",
        "name": "小于等于测试",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "order.amount",
            "operator": "lte",
            "value": 1500
        }
    }
    "#;

    let compiled = compiler.compile_from_json(rule_json).unwrap();
    let executor = RuleExecutor::new();
    let context = create_purchase_context();

    let result = executor.execute(&compiled, &context).unwrap();
    assert!(result.matched); // 1500 <= 1500
}

// ==================== 程序化构建规则测试 ====================

#[test]
fn test_programmatic_rule_building() {
    let rule = Rule::new(
        "programmatic-rule",
        RuleNode::Group(LogicalGroup::and(vec![
            RuleNode::Condition(Condition::new("event.type", Operator::Eq, "PURCHASE")),
            RuleNode::Group(LogicalGroup::or(vec![
                RuleNode::Condition(Condition::new("order.amount", Operator::Gte, 1000)),
                RuleNode::Condition(Condition::new("user.is_vip", Operator::Eq, true)),
            ])),
        ])),
    );

    let store = RuleStore::new();
    store.load(rule).unwrap();

    let compiled = store.get("programmatic-rule");
    assert!(compiled.is_none()); // 使用 Rule::new 生成的 id 是 UUID，不是 "programmatic-rule"

    // 通过 list_ids 获取实际的 ID
    let ids = store.list_ids();
    assert_eq!(ids.len(), 1);

    let compiled = store.get(&ids[0]).unwrap();
    let executor = RuleExecutor::new();
    let context = create_purchase_context();

    let result = executor.execute(&compiled, &context).unwrap();
    assert!(result.matched);
}

#[test]
fn test_programmatic_rule_with_explicit_id() {
    let mut rule = Rule::new(
        "explicit-id-rule",
        RuleNode::Condition(Condition::new("event.type", Operator::Eq, "PURCHASE")),
    );
    rule.id = "my-explicit-id".to_string();

    let store = RuleStore::new();
    store.load(rule).unwrap();

    let compiled = store.get("my-explicit-id").unwrap();
    let executor = RuleExecutor::new();
    let context = create_purchase_context();

    let result = executor.execute(&compiled, &context).unwrap();
    assert!(result.matched);
    assert_eq!(result.rule_name, "explicit-id-rule");
}

// ==================== 错误处理测试 ====================

#[test]
fn test_invalid_json_rule() {
    let store = RuleStore::new();
    let result = store.load_from_json("invalid json");
    assert!(result.is_err());
}

#[test]
fn test_empty_rule_id() {
    let mut compiler = RuleCompiler::new();
    let rule_json = r#"
    {
        "id": "",
        "name": "empty id rule",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "a",
            "operator": "eq",
            "value": 1
        }
    }
    "#;

    let result = compiler.compile_from_json(rule_json);
    assert!(result.is_err());
}

#[test]
fn test_update_nonexistent_rule() {
    let store = RuleStore::new();
    let rule: Rule = serde_json::from_str(
        r#"
        {
            "id": "nonexistent",
            "name": "test",
            "version": "1.0",
            "root": {
                "type": "condition",
                "field": "a",
                "operator": "eq",
                "value": 1
            }
        }
        "#,
    )
    .unwrap();

    let result = store.update(rule);
    assert!(result.is_err());
}

#[test]
fn test_delete_rule() {
    let store = RuleStore::new();
    let rule_json = r#"
    {
        "id": "to-delete",
        "name": "删除测试",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "a",
            "operator": "eq",
            "value": 1
        }
    }
    "#;

    store.load_from_json(rule_json).unwrap();
    assert!(store.contains("to-delete"));

    store.delete("to-delete").unwrap();
    assert!(!store.contains("to-delete"));
}

#[test]
fn test_rule_store_stats() {
    let store = RuleStore::new();

    // 加载多条规则
    store
        .load_from_json(
            r#"{"id": "r1", "name": "rule1", "version": "1.0", "root": {"type": "condition", "field": "a", "operator": "eq", "value": 1}}"#,
        )
        .unwrap();
    store
        .load_from_json(
            r#"{"id": "r2", "name": "rule2", "version": "1.0", "root": {"type": "group", "operator": "AND", "children": [{"type": "condition", "field": "b", "operator": "eq", "value": 2}, {"type": "condition", "field": "c", "operator": "eq", "value": 3}]}}"#,
        )
        .unwrap();

    let stats = store.stats();
    assert_eq!(stats.rules_count, 2);
    assert_eq!(stats.total_fields, 3); // r1: 1 field, r2: 2 fields
}
