//! 规则引擎性能基准测试
//!
//! 测试覆盖：
//! - 简单条件评估性能
//! - 复杂嵌套规则评估性能
//! - 批量规则评估性能
//! - 不同数据量下的性能曲线

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rule_engine::{
    Condition, EvaluationContext, LogicalGroup, LogicalOperator, Operator, Rule, RuleCompiler,
    RuleExecutor, RuleNode, RuleStore,
};
use std::hint::black_box;
use serde_json::json;

/// 创建简单条件规则
fn create_simple_rule() -> Rule {
    Rule::new(
        "simple_rule",
        RuleNode::Condition(Condition::new("event.type", Operator::Eq, "PURCHASE")),
    )
}

/// 创建 AND 组合规则
fn create_and_rule(conditions_count: usize) -> Rule {
    let conditions: Vec<RuleNode> = (0..conditions_count)
        .map(|i| {
            RuleNode::Condition(Condition::new(
                format!("field_{}", i),
                Operator::Eq,
                format!("value_{}", i),
            ))
        })
        .collect();

    Rule::new(
        "and_rule",
        RuleNode::Group(LogicalGroup::new(LogicalOperator::And, conditions)),
    )
}

/// 创建嵌套规则（AND 包含多个 OR 组）
fn create_nested_rule(depth: usize, breadth: usize) -> Rule {
    fn build_nested(depth: usize, breadth: usize, level: usize) -> RuleNode {
        if depth == 0 {
            RuleNode::Condition(Condition::new(
                format!("field_{}_{}", level, depth),
                Operator::Eq,
                format!("value_{}_{}", level, depth),
            ))
        } else {
            let operator = if depth % 2 == 0 {
                LogicalOperator::And
            } else {
                LogicalOperator::Or
            };

            let children: Vec<RuleNode> = (0..breadth)
                .map(|i| build_nested(depth - 1, breadth, i))
                .collect();

            RuleNode::Group(LogicalGroup::new(operator, children))
        }
    }

    Rule::new("nested_rule", build_nested(depth, breadth, 0))
}

/// 创建包含多种操作符的复杂规则
fn create_complex_rule() -> Rule {
    Rule::new(
        "complex_rule",
        RuleNode::Group(LogicalGroup::and(vec![
            RuleNode::Condition(Condition::new("event.type", Operator::Eq, "PURCHASE")),
            RuleNode::Condition(Condition::new("order.amount", Operator::Gte, 1000)),
            RuleNode::Condition(Condition::new("order.amount", Operator::Lte, 10000)),
            RuleNode::Group(LogicalGroup::or(vec![
                RuleNode::Condition(Condition::new("user.is_vip", Operator::Eq, true)),
                RuleNode::Condition(Condition::new(
                    "user.membership_years",
                    Operator::Gte,
                    2,
                )),
            ])),
            RuleNode::Condition(Condition::new(
                "user.tags",
                Operator::ContainsAny,
                json!(["premium", "gold"]),
            )),
            RuleNode::Condition(Condition::new(
                "order.items",
                Operator::Between,
                json!([1, 10]),
            )),
        ])),
    )
}

/// 创建测试上下文（匹配场景）
fn create_matching_context() -> EvaluationContext {
    EvaluationContext::new(json!({
        "event": {
            "type": "PURCHASE",
            "timestamp": "2024-01-15T10:00:00Z"
        },
        "order": {
            "amount": 5000,
            "items": 5,
            "product_ids": ["p001", "p002", "p003"]
        },
        "user": {
            "id": "user-123",
            "is_vip": true,
            "membership_years": 3,
            "tags": ["premium", "frequent", "gold"],
            "profile": {
                "age": 30,
                "country": "US"
            }
        }
    }))
}

/// 创建测试上下文（不匹配场景）
fn create_non_matching_context() -> EvaluationContext {
    EvaluationContext::new(json!({
        "event": {
            "type": "REFUND",
            "timestamp": "2024-01-15T10:00:00Z"
        },
        "order": {
            "amount": 100,
            "items": 1
        },
        "user": {
            "id": "user-456",
            "is_vip": false,
            "membership_years": 0,
            "tags": ["new"]
        }
    }))
}

/// 创建包含多字段的大型上下文
fn create_large_context(field_count: usize) -> EvaluationContext {
    let mut data = serde_json::Map::new();

    for i in 0..field_count {
        data.insert(format!("field_{}", i), json!(format!("value_{}", i)));
    }

    // 添加基础字段供规则匹配
    data.insert("event".to_string(), json!({"type": "PURCHASE"}));
    data.insert("order".to_string(), json!({"amount": 5000, "items": 5}));
    data.insert(
        "user".to_string(),
        json!({
            "is_vip": true,
            "membership_years": 3,
            "tags": ["premium", "gold"]
        }),
    );

    EvaluationContext::new(serde_json::Value::Object(data))
}

// ============================================================================
// 基准测试函数
// ============================================================================

/// 简单条件评估基准
fn bench_simple_condition(c: &mut Criterion) {
    let mut compiler = RuleCompiler::new();
    let rule = create_simple_rule();
    let compiled = compiler.compile(rule).unwrap();
    let executor = RuleExecutor::new();
    let context = create_matching_context();

    c.bench_function("simple_condition_evaluation", |b| {
        b.iter(|| {
            let result = executor.execute(black_box(&compiled), black_box(&context));
            black_box(result)
        })
    });
}

/// AND 组合条件评估基准（不同条件数量）
fn bench_and_conditions(c: &mut Criterion) {
    let mut group = c.benchmark_group("and_conditions");

    for conditions_count in [2, 5, 10, 20, 50].iter() {
        let mut compiler = RuleCompiler::new();
        let rule = create_and_rule(*conditions_count);
        let compiled = compiler.compile(rule).unwrap();
        let executor = RuleExecutor::new();
        let context = create_large_context(*conditions_count);

        group.throughput(Throughput::Elements(*conditions_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(conditions_count),
            conditions_count,
            |b, _| {
                b.iter(|| {
                    let result = executor.execute(black_box(&compiled), black_box(&context));
                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

/// 嵌套规则评估基准（不同嵌套深度）
fn bench_nested_rules(c: &mut Criterion) {
    let mut group = c.benchmark_group("nested_rules");

    // (depth, breadth) 组合
    let configs = [(1, 2), (2, 2), (3, 2), (4, 2), (2, 4), (3, 3)];

    for (depth, breadth) in configs.iter() {
        let mut compiler = RuleCompiler::new();
        let rule = create_nested_rule(*depth, *breadth);
        let compiled = compiler.compile(rule).unwrap();
        let executor = RuleExecutor::new();
        let context = create_large_context(100);

        // 计算总节点数作为吞吐量度量
        let total_nodes = (breadth.pow(*depth as u32 + 1) - 1) / (breadth - 1);

        group.throughput(Throughput::Elements(total_nodes as u64));
        group.bench_with_input(
            BenchmarkId::new("depth_breadth", format!("{}x{}", depth, breadth)),
            &(depth, breadth),
            |b, _| {
                b.iter(|| {
                    let result = executor.execute(black_box(&compiled), black_box(&context));
                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

/// 复杂规则评估基准
fn bench_complex_rule(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_rule");

    let mut compiler = RuleCompiler::new();
    let rule = create_complex_rule();
    let compiled = compiler.compile(rule).unwrap();
    let executor = RuleExecutor::new();

    // 匹配场景
    let matching_ctx = create_matching_context();
    group.bench_function("matching", |b| {
        b.iter(|| {
            let result = executor.execute(black_box(&compiled), black_box(&matching_ctx));
            black_box(result)
        })
    });

    // 不匹配场景（测试短路求值效果）
    let non_matching_ctx = create_non_matching_context();
    group.bench_function("non_matching_short_circuit", |b| {
        b.iter(|| {
            let result = executor.execute(black_box(&compiled), black_box(&non_matching_ctx));
            black_box(result)
        })
    });

    group.finish();
}

/// 规则编译基准
fn bench_rule_compilation(c: &mut Criterion) {
    let mut group = c.benchmark_group("rule_compilation");

    // 简单规则编译
    let simple_json = r#"
    {
        "id": "rule-001",
        "name": "simple",
        "version": "1.0",
        "root": {
            "type": "condition",
            "field": "event.type",
            "operator": "eq",
            "value": "PURCHASE"
        }
    }
    "#;

    group.bench_function("simple_rule", |b| {
        b.iter(|| {
            let mut compiler = RuleCompiler::new();
            let result = compiler.compile_from_json(black_box(simple_json));
            black_box(result)
        })
    });

    // 复杂规则编译
    let complex_json = r#"
    {
        "id": "rule-002",
        "name": "complex",
        "version": "1.0",
        "root": {
            "type": "group",
            "operator": "AND",
            "children": [
                {"type": "condition", "field": "event.type", "operator": "eq", "value": "PURCHASE"},
                {"type": "condition", "field": "order.amount", "operator": "gte", "value": 1000},
                {
                    "type": "group",
                    "operator": "OR",
                    "children": [
                        {"type": "condition", "field": "user.is_vip", "operator": "eq", "value": true},
                        {"type": "condition", "field": "user.level", "operator": "gte", "value": 5}
                    ]
                }
            ]
        }
    }
    "#;

    group.bench_function("complex_rule", |b| {
        b.iter(|| {
            let mut compiler = RuleCompiler::new();
            let result = compiler.compile_from_json(black_box(complex_json));
            black_box(result)
        })
    });

    group.finish();
}

/// 规则存储操作基准
fn bench_rule_store(c: &mut Criterion) {
    let mut group = c.benchmark_group("rule_store");

    // 规则加载
    group.bench_function("load_rule", |b| {
        let store = RuleStore::new();
        let mut counter = 0;
        b.iter(|| {
            let rule = Rule::new(
                format!("rule_{}", counter),
                RuleNode::Condition(Condition::new("field", Operator::Eq, "value")),
            );
            // 需要修改 rule.id 以避免重复
            let mut rule = rule;
            rule.id = format!("rule_{}", counter);
            let result = store.load(black_box(rule));
            counter += 1;
            black_box(result)
        })
    });

    // 规则查询
    let store = RuleStore::new();
    for i in 0..1000 {
        let mut rule = Rule::new(
            format!("rule_{}", i),
            RuleNode::Condition(Condition::new("field", Operator::Eq, "value")),
        );
        rule.id = format!("rule_{}", i);
        store.load(rule).unwrap();
    }

    group.bench_function("get_rule", |b| {
        let mut i = 0;
        b.iter(|| {
            let rule_id = format!("rule_{}", i % 1000);
            let result = store.get(black_box(&rule_id));
            i += 1;
            black_box(result)
        })
    });

    group.finish();
}

/// 批量规则评估基准
fn bench_batch_evaluation(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_evaluation");

    for rule_count in [10, 50, 100, 500].iter() {
        let store = RuleStore::new();
        let executor = RuleExecutor::new();

        // 加载规则
        for i in 0..*rule_count {
            let mut rule = create_simple_rule();
            rule.id = format!("rule_{}", i);
            rule.name = format!("rule_{}", i);
            store.load(rule).unwrap();
        }

        let rules: Vec<_> = store.list_all();
        let context = create_matching_context();

        group.throughput(Throughput::Elements(*rule_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(rule_count),
            rule_count,
            |b, _| {
                b.iter(|| {
                    let results: Vec<_> = rules
                        .iter()
                        .map(|rule| executor.execute(rule, black_box(&context)))
                        .collect();
                    black_box(results)
                })
            },
        );
    }

    group.finish();
}

/// 上下文字段访问基准
fn bench_context_field_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_field_access");

    let context = create_matching_context();

    // 浅层字段访问
    group.bench_function("shallow_field", |b| {
        b.iter(|| {
            let value = context.get_field(black_box("event"));
            black_box(value)
        })
    });

    // 深层字段访问
    group.bench_function("deep_field", |b| {
        b.iter(|| {
            let value = context.get_field(black_box("user.profile.age"));
            black_box(value)
        })
    });

    // 数组索引访问
    group.bench_function("array_index_field", |b| {
        b.iter(|| {
            let value = context.get_field(black_box("order.product_ids.0"));
            black_box(value)
        })
    });

    // 不存在的字段
    group.bench_function("missing_field", |b| {
        b.iter(|| {
            let value = context.get_field(black_box("nonexistent.deep.field"));
            black_box(value)
        })
    });

    group.finish();
}

/// 各种操作符性能对比
fn bench_operators(c: &mut Criterion) {
    let mut group = c.benchmark_group("operators");

    let executor = RuleExecutor::new();
    let context = create_matching_context();

    // 相等比较
    let mut compiler = RuleCompiler::new();
    let eq_rule = compiler
        .compile(Rule::new(
            "eq_rule",
            RuleNode::Condition(Condition::new("event.type", Operator::Eq, "PURCHASE")),
        ))
        .unwrap();

    group.bench_function("eq", |b| {
        b.iter(|| executor.execute(black_box(&eq_rule), black_box(&context)))
    });

    // 数值比较
    let gte_rule = compiler
        .compile(Rule::new(
            "gte_rule",
            RuleNode::Condition(Condition::new("order.amount", Operator::Gte, 1000)),
        ))
        .unwrap();

    group.bench_function("gte", |b| {
        b.iter(|| executor.execute(black_box(&gte_rule), black_box(&context)))
    });

    // 范围比较
    let between_rule = compiler
        .compile(Rule::new(
            "between_rule",
            RuleNode::Condition(Condition::new(
                "order.amount",
                Operator::Between,
                json!([1000, 10000]),
            )),
        ))
        .unwrap();

    group.bench_function("between", |b| {
        b.iter(|| executor.execute(black_box(&between_rule), black_box(&context)))
    });

    // 列表包含
    let in_rule = compiler
        .compile(Rule::new(
            "in_rule",
            RuleNode::Condition(Condition::new(
                "event.type",
                Operator::In,
                json!(["PURCHASE", "REFUND", "EXCHANGE"]),
            )),
        ))
        .unwrap();

    group.bench_function("in", |b| {
        b.iter(|| executor.execute(black_box(&in_rule), black_box(&context)))
    });

    // 数组包含任意
    let contains_any_rule = compiler
        .compile(Rule::new(
            "contains_any_rule",
            RuleNode::Condition(Condition::new(
                "user.tags",
                Operator::ContainsAny,
                json!(["premium", "gold", "platinum"]),
            )),
        ))
        .unwrap();

    group.bench_function("contains_any", |b| {
        b.iter(|| executor.execute(black_box(&contains_any_rule), black_box(&context)))
    });

    // 字符串前缀
    let context_with_string = EvaluationContext::new(json!({
        "email": "user@example.com"
    }));

    let starts_with_rule = compiler
        .compile(Rule::new(
            "starts_with_rule",
            RuleNode::Condition(Condition::new("email", Operator::StartsWith, "user")),
        ))
        .unwrap();

    group.bench_function("starts_with", |b| {
        b.iter(|| executor.execute(black_box(&starts_with_rule), black_box(&context_with_string)))
    });

    // 正则表达式
    let regex_rule = compiler
        .compile(Rule::new(
            "regex_rule",
            RuleNode::Condition(Condition::new(
                "email",
                Operator::Regex,
                r"^[\w.-]+@[\w.-]+\.\w+$",
            )),
        ))
        .unwrap();

    group.bench_function("regex", |b| {
        b.iter(|| executor.execute(black_box(&regex_rule), black_box(&context_with_string)))
    });

    group.finish();
}

// 配置 criterion
criterion_group!(
    benches,
    bench_simple_condition,
    bench_and_conditions,
    bench_nested_rules,
    bench_complex_rule,
    bench_rule_compilation,
    bench_rule_store,
    bench_batch_evaluation,
    bench_context_field_access,
    bench_operators,
);

criterion_main!(benches);
