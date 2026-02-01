//! 负载测试
//!
//! 模拟高并发场景下的规则引擎性能。
//! 此测试可独立运行，也可通过 criterion 框架运行。

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rule_engine::{
    Condition, EvaluationContext, LogicalGroup, Operator, Rule, RuleExecutor, RuleNode, RuleStore,
};
use std::hint::black_box;
use serde_json::json;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// 并发评估测试配置
struct ConcurrencyConfig {
    thread_count: usize,
    iterations_per_thread: usize,
    rule_count: usize,
}

/// 并发测试结果
#[derive(Debug)]
#[allow(dead_code)]
struct ConcurrencyResult {
    total_evaluations: usize,
    total_duration: Duration,
    throughput_per_sec: f64,
    avg_latency_us: f64,
    min_latency_us: u64,
    max_latency_us: u64,
    p99_latency_us: u64,
}

/// 创建测试规则
fn create_test_rules(count: usize) -> Vec<Rule> {
    (0..count)
        .map(|i| {
            let mut rule = Rule::new(
                format!("rule_{}", i),
                RuleNode::Group(LogicalGroup::and(vec![
                    RuleNode::Condition(Condition::new("event.type", Operator::Eq, "PURCHASE")),
                    RuleNode::Condition(Condition::new("order.amount", Operator::Gte, 100 + i)),
                    RuleNode::Condition(Condition::new("user.is_active", Operator::Eq, true)),
                ])),
            );
            rule.id = format!("rule_{}", i);
            rule
        })
        .collect()
}

/// 创建测试上下文
fn create_test_context(variant: usize) -> EvaluationContext {
    EvaluationContext::new(json!({
        "event": {
            "type": if variant % 2 == 0 { "PURCHASE" } else { "VIEW" },
            "timestamp": "2024-01-15T10:00:00Z"
        },
        "order": {
            "amount": 500 + (variant % 1000),
            "items": variant % 10 + 1
        },
        "user": {
            "id": format!("user-{}", variant),
            "is_active": variant % 3 != 0,
            "level": variant % 5
        }
    }))
}

/// 运行并发评估测试
fn run_concurrent_evaluation(config: ConcurrencyConfig) -> ConcurrencyResult {
    let store = Arc::new(RuleStore::new());

    // 加载规则
    for rule in create_test_rules(config.rule_count) {
        store.load(rule).unwrap();
    }

    let rules: Vec<_> = store.list_all();
    let rules = Arc::new(rules);

    let mut handles = Vec::with_capacity(config.thread_count);
    let start = Instant::now();

    for thread_id in 0..config.thread_count {
        let rules = Arc::clone(&rules);
        let iterations = config.iterations_per_thread;

        let handle = thread::spawn(move || {
            let executor = RuleExecutor::new();
            let mut latencies = Vec::with_capacity(iterations);

            for i in 0..iterations {
                let context = create_test_context(thread_id * 1000 + i);

                // 评估所有规则
                for rule in rules.iter() {
                    let iter_start = Instant::now();
                    let result = executor.execute(rule, &context);
                    let latency = iter_start.elapsed().as_micros() as u64;
                    latencies.push(latency);
                    let _ = black_box(result);
                }
            }

            latencies
        });

        handles.push(handle);
    }

    // 收集所有线程的延迟数据
    let mut all_latencies: Vec<u64> = Vec::new();
    for handle in handles {
        let latencies = handle.join().unwrap();
        all_latencies.extend(latencies);
    }

    let total_duration = start.elapsed();
    let total_evaluations = all_latencies.len();

    // 计算统计数据
    all_latencies.sort_unstable();
    let sum: u64 = all_latencies.iter().sum();
    let avg_latency_us = sum as f64 / total_evaluations as f64;
    let min_latency_us = *all_latencies.first().unwrap_or(&0);
    let max_latency_us = *all_latencies.last().unwrap_or(&0);
    let p99_index = (total_evaluations as f64 * 0.99) as usize;
    let p99_latency_us = all_latencies.get(p99_index).copied().unwrap_or(0);

    let throughput_per_sec = total_evaluations as f64 / total_duration.as_secs_f64();

    ConcurrencyResult {
        total_evaluations,
        total_duration,
        throughput_per_sec,
        avg_latency_us,
        min_latency_us,
        max_latency_us,
        p99_latency_us,
    }
}

/// 规则存储并发访问测试
fn run_concurrent_store_access(thread_count: usize, operations_per_thread: usize) -> Duration {
    let store = Arc::new(RuleStore::new());

    // 预加载一些规则
    for i in 0..100 {
        let mut rule = Rule::new(
            format!("preload_rule_{}", i),
            RuleNode::Condition(Condition::new("field", Operator::Eq, "value")),
        );
        rule.id = format!("preload_rule_{}", i);
        store.load(rule).unwrap();
    }

    let mut handles = Vec::with_capacity(thread_count);
    let start = Instant::now();

    for thread_id in 0..thread_count {
        let store = Arc::clone(&store);
        let ops = operations_per_thread;

        let handle = thread::spawn(move || {
            for i in 0..ops {
                match i % 4 {
                    0 => {
                        // 读取操作
                        let rule_id = format!("preload_rule_{}", i % 100);
                        black_box(store.get(&rule_id));
                    }
                    1 => {
                        // 写入操作
                        let mut rule = Rule::new(
                            format!("dynamic_rule_{}_{}", thread_id, i),
                            RuleNode::Condition(Condition::new("field", Operator::Eq, "value")),
                        );
                        rule.id = format!("dynamic_rule_{}_{}", thread_id, i);
                        let _ = store.load(rule);
                    }
                    2 => {
                        // 检查存在
                        let rule_id = format!("preload_rule_{}", i % 100);
                        black_box(store.contains(&rule_id));
                    }
                    3 => {
                        // 列出规则
                        black_box(store.len());
                    }
                    _ => unreachable!(),
                }
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    start.elapsed()
}

// ============================================================================
// Criterion 基准测试
// ============================================================================

/// 并发评估基准测试
fn bench_concurrent_evaluation(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_evaluation");

    // 不同并发级别测试
    for threads in [1, 2, 4, 8].iter() {
        let config = ConcurrencyConfig {
            thread_count: *threads,
            iterations_per_thread: 100,
            rule_count: 10,
        };

        group.throughput(Throughput::Elements(
            (config.iterations_per_thread * config.rule_count) as u64,
        ));
        group.bench_with_input(
            BenchmarkId::new("threads", threads),
            threads,
            |b, _threads| {
                b.iter(|| {
                    let result = run_concurrent_evaluation(ConcurrencyConfig {
                        thread_count: *_threads,
                        iterations_per_thread: 50,
                        rule_count: 5,
                    });
                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

/// 规则存储并发访问基准测试
fn bench_concurrent_store(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_store_access");

    for threads in [1, 2, 4, 8].iter() {
        group.bench_with_input(BenchmarkId::new("threads", threads), threads, |b, threads| {
            b.iter(|| {
                let duration = run_concurrent_store_access(*threads, 100);
                black_box(duration)
            })
        });
    }

    group.finish();
}

/// 缓存命中率测试
fn bench_cache_hit_ratio(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_performance");

    let store = RuleStore::new();

    // 加载规则
    for i in 0..100 {
        let mut rule = Rule::new(
            format!("cache_rule_{}", i),
            RuleNode::Condition(Condition::new("field", Operator::Eq, "value")),
        );
        rule.id = format!("cache_rule_{}", i);
        store.load(rule).unwrap();
    }

    // 热点数据访问（高缓存命中率场景）
    group.bench_function("hot_data_access", |b| {
        let mut i = 0;
        b.iter(|| {
            // 只访问前 10 条规则（模拟热点数据）
            let rule_id = format!("cache_rule_{}", i % 10);
            let result = store.get(black_box(&rule_id));
            i += 1;
            black_box(result)
        })
    });

    // 随机数据访问（低缓存命中率场景）
    group.bench_function("random_data_access", |b| {
        let mut i = 0;
        b.iter(|| {
            // 访问所有 100 条规则
            let rule_id = format!("cache_rule_{}", i % 100);
            let result = store.get(black_box(&rule_id));
            i += 1;
            black_box(result)
        })
    });

    // 不存在的数据访问（缓存未命中场景）
    group.bench_function("cache_miss", |b| {
        let mut i = 0;
        b.iter(|| {
            let rule_id = format!("nonexistent_rule_{}", i);
            let result = store.get(black_box(&rule_id));
            i += 1;
            black_box(result)
        })
    });

    group.finish();
}

/// 高负载场景测试
fn bench_high_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("high_load");
    group.sample_size(10); // 减少采样数以加快测试

    // 大量规则场景
    for rule_count in [100, 500, 1000].iter() {
        let store = RuleStore::new();
        let executor = RuleExecutor::new();

        // 加载规则
        for i in 0..*rule_count {
            let mut rule = Rule::new(
                format!("load_rule_{}", i),
                RuleNode::Group(LogicalGroup::and(vec![
                    RuleNode::Condition(Condition::new("event.type", Operator::Eq, "PURCHASE")),
                    RuleNode::Condition(Condition::new("order.amount", Operator::Gte, i)),
                ])),
            );
            rule.id = format!("load_rule_{}", i);
            store.load(rule).unwrap();
        }

        let rules: Vec<_> = store.list_all();
        let context = create_test_context(0);

        group.throughput(Throughput::Elements(*rule_count as u64));
        group.bench_with_input(
            BenchmarkId::new("rule_count", rule_count),
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

criterion_group!(
    benches,
    bench_concurrent_evaluation,
    bench_concurrent_store,
    bench_cache_hit_ratio,
    bench_high_load,
);

criterion_main!(benches);

// ============================================================================
// 独立运行的负载测试（可通过 cargo run --example load_test 运行）
// ============================================================================

#[cfg(feature = "standalone")]
fn main() {
    println!("========================================");
    println!("         规则引擎负载测试报告           ");
    println!("========================================\n");

    // 测试配置
    let configs = vec![
        ConcurrencyConfig {
            thread_count: 1,
            iterations_per_thread: 1000,
            rule_count: 10,
        },
        ConcurrencyConfig {
            thread_count: 4,
            iterations_per_thread: 1000,
            rule_count: 10,
        },
        ConcurrencyConfig {
            thread_count: 8,
            iterations_per_thread: 1000,
            rule_count: 10,
        },
        ConcurrencyConfig {
            thread_count: 8,
            iterations_per_thread: 1000,
            rule_count: 100,
        },
    ];

    for config in configs {
        println!(
            "配置: {} 线程, {} 迭代/线程, {} 规则",
            config.thread_count, config.iterations_per_thread, config.rule_count
        );
        println!("-----------------------------------------");

        let result = run_concurrent_evaluation(config);

        println!("  总评估次数:    {:>10}", result.total_evaluations);
        println!("  总耗时:        {:>10.2?}", result.total_duration);
        println!("  吞吐量:        {:>10.0} ops/sec", result.throughput_per_sec);
        println!("  平均延迟:      {:>10.2} us", result.avg_latency_us);
        println!("  最小延迟:      {:>10} us", result.min_latency_us);
        println!("  最大延迟:      {:>10} us", result.max_latency_us);
        println!("  P99 延迟:      {:>10} us", result.p99_latency_us);
        println!();
    }

    // 规则存储并发测试
    println!("========================================");
    println!("         规则存储并发访问测试           ");
    println!("========================================\n");

    for threads in [1, 4, 8] {
        let duration = run_concurrent_store_access(threads, 10000);
        let ops_per_sec = (threads * 10000) as f64 / duration.as_secs_f64();
        println!(
            "{} 线程: {:?} ({:.0} ops/sec)",
            threads, duration, ops_per_sec
        );
    }
}
