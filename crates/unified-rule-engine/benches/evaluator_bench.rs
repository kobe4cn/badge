//! 条件评估器性能基准测试
//!
//! 针对 ConditionEvaluator 的各种操作进行细粒度的性能测试。

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rule_engine::{ConditionEvaluator, Operator};
use serde_json::{json, Value};
use std::hint::black_box;

/// 创建测试数据
fn create_test_values() -> (Value, Value) {
    let field = json!(1000);
    let expected = json!(500);
    (field, expected)
}

fn create_string_values() -> (Value, Value) {
    let field = json!("hello world");
    let expected = json!("world");
    (field, expected)
}

fn create_array_values() -> (Value, Value) {
    let field = json!(["a", "b", "c", "d", "e"]);
    let expected = json!(["b", "d"]);
    (field, expected)
}

/// 数值比较操作基准
fn bench_numeric_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("numeric_operations");

    let (field, expected) = create_test_values();

    group.bench_function("eq", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(Some(&field)),
                black_box(Operator::Eq),
                black_box(&expected),
            )
        })
    });

    group.bench_function("neq", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(Some(&field)),
                black_box(Operator::Neq),
                black_box(&expected),
            )
        })
    });

    group.bench_function("gt", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(Some(&field)),
                black_box(Operator::Gt),
                black_box(&expected),
            )
        })
    });

    group.bench_function("gte", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(Some(&field)),
                black_box(Operator::Gte),
                black_box(&expected),
            )
        })
    });

    group.bench_function("lt", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(Some(&field)),
                black_box(Operator::Lt),
                black_box(&expected),
            )
        })
    });

    group.bench_function("lte", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(Some(&field)),
                black_box(Operator::Lte),
                black_box(&expected),
            )
        })
    });

    let between_value = json!([100, 2000]);
    group.bench_function("between", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(Some(&field)),
                black_box(Operator::Between),
                black_box(&between_value),
            )
        })
    });

    group.finish();
}

/// 字符串操作基准
fn bench_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_operations");

    let (field, expected) = create_string_values();

    group.bench_function("contains", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(Some(&field)),
                black_box(Operator::Contains),
                black_box(&expected),
            )
        })
    });

    let prefix = json!("hello");
    group.bench_function("starts_with", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(Some(&field)),
                black_box(Operator::StartsWith),
                black_box(&prefix),
            )
        })
    });

    let suffix = json!("world");
    group.bench_function("ends_with", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(Some(&field)),
                black_box(Operator::EndsWith),
                black_box(&suffix),
            )
        })
    });

    group.finish();
}

/// 正则表达式操作基准
fn bench_regex_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("regex_operations");

    let email = json!("user@example.com");

    // 简单正则
    let simple_pattern = json!(r"^user");
    group.bench_function("simple_regex", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(Some(&email)),
                black_box(Operator::Regex),
                black_box(&simple_pattern),
            )
        })
    });

    // 复杂正则
    let complex_pattern = json!(r"^[\w.-]+@[\w.-]+\.\w+$");
    group.bench_function("complex_regex", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(Some(&email)),
                black_box(Operator::Regex),
                black_box(&complex_pattern),
            )
        })
    });

    group.finish();
}

/// 数组操作基准
fn bench_array_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("array_operations");

    let (field, expected) = create_array_values();

    group.bench_function("contains_any", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(Some(&field)),
                black_box(Operator::ContainsAny),
                black_box(&expected),
            )
        })
    });

    group.bench_function("contains_all", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(Some(&field)),
                black_box(Operator::ContainsAll),
                black_box(&expected),
            )
        })
    });

    let single = json!("c");
    group.bench_function("contains_single", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(Some(&field)),
                black_box(Operator::Contains),
                black_box(&single),
            )
        })
    });

    group.finish();
}

/// In 操作符不同列表大小的性能
fn bench_in_operator_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("in_operator_scaling");

    let field = json!("target");

    for size in [5, 10, 50, 100, 500].iter() {
        let list: Vec<Value> = (0..*size)
            .map(|i| {
                if i == size - 1 {
                    json!("target")
                } else {
                    json!(format!("item_{}", i))
                }
            })
            .collect();
        let list_value = Value::Array(list);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                ConditionEvaluator::evaluate(
                    black_box(Some(&field)),
                    black_box(Operator::In),
                    black_box(&list_value),
                )
            })
        });
    }

    group.finish();
}

/// 空值检查操作基准
fn bench_empty_checks(c: &mut Criterion) {
    let mut group = c.benchmark_group("empty_checks");

    let null_value = json!(null);
    let empty_string = json!("");
    let empty_array: Value = json!([]);
    let non_empty = json!("hello");

    group.bench_function("is_empty_null", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(Some(&null_value)),
                black_box(Operator::IsEmpty),
                black_box(&null_value),
            )
        })
    });

    group.bench_function("is_empty_string", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(Some(&empty_string)),
                black_box(Operator::IsEmpty),
                black_box(&null_value),
            )
        })
    });

    group.bench_function("is_empty_array", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(Some(&empty_array)),
                black_box(Operator::IsEmpty),
                black_box(&null_value),
            )
        })
    });

    group.bench_function("is_not_empty", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(Some(&non_empty)),
                black_box(Operator::IsNotEmpty),
                black_box(&null_value),
            )
        })
    });

    group.bench_function("is_empty_none", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(None),
                black_box(Operator::IsEmpty),
                black_box(&null_value),
            )
        })
    });

    group.finish();
}

/// 时间比较操作基准
fn bench_time_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("time_operations");

    let field_time = json!("2024-01-15T10:00:00Z");
    let expected_time = json!("2024-01-20T10:00:00Z");

    group.bench_function("before", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(Some(&field_time)),
                black_box(Operator::Before),
                black_box(&expected_time),
            )
        })
    });

    group.bench_function("after", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(Some(&field_time)),
                black_box(Operator::After),
                black_box(&expected_time),
            )
        })
    });

    // 纯日期格式
    let date_field = json!("2024-01-15");
    let date_expected = json!("2024-01-20");

    group.bench_function("before_date_only", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(Some(&date_field)),
                black_box(Operator::Before),
                black_box(&date_expected),
            )
        })
    });

    group.finish();
}

/// 缺失字段处理基准
fn bench_missing_field(c: &mut Criterion) {
    let mut group = c.benchmark_group("missing_field");

    let expected = json!("test");

    group.bench_function("eq_missing", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(None),
                black_box(Operator::Eq),
                black_box(&expected),
            )
        })
    });

    group.bench_function("gt_missing", |b| {
        b.iter(|| {
            ConditionEvaluator::evaluate(
                black_box(None),
                black_box(Operator::Gt),
                black_box(&json!(100)),
            )
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_numeric_operations,
    bench_string_operations,
    bench_regex_operations,
    bench_array_operations,
    bench_in_operator_scaling,
    bench_empty_checks,
    bench_time_operations,
    bench_missing_field,
);

criterion_main!(benches);
