//! 性能测试模块
//!
//! 包含负载测试、压力测试和基准测试场景。

pub mod scenarios;

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

/// 性能测试配置
#[derive(Debug, Clone)]
pub struct LoadTestConfig {
    /// 并发用户数
    pub concurrent_users: usize,
    /// 测试持续时间
    pub duration: Duration,
    /// 每秒请求数限制
    pub requests_per_second: Option<u32>,
    /// 预热时间
    pub warmup_duration: Duration,
    /// 请求超时
    pub request_timeout: Duration,
}

impl Default for LoadTestConfig {
    fn default() -> Self {
        Self {
            concurrent_users: 100,
            duration: Duration::from_secs(60),
            requests_per_second: None,
            warmup_duration: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
        }
    }
}

/// 性能测试结果统计
#[derive(Debug, Clone, Default)]
pub struct LoadTestMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub latencies_ms: Vec<f64>,
    pub errors: Vec<String>,
}

impl LoadTestMetrics {
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        self.successful_requests as f64 / self.total_requests as f64 * 100.0
    }

    pub fn avg_latency_ms(&self) -> f64 {
        if self.latencies_ms.is_empty() {
            return 0.0;
        }
        self.latencies_ms.iter().sum::<f64>() / self.latencies_ms.len() as f64
    }

    pub fn p50_latency_ms(&self) -> f64 {
        self.percentile(50.0)
    }

    pub fn p95_latency_ms(&self) -> f64 {
        self.percentile(95.0)
    }

    pub fn p99_latency_ms(&self) -> f64 {
        self.percentile(99.0)
    }

    fn percentile(&self, p: f64) -> f64 {
        if self.latencies_ms.is_empty() {
            return 0.0;
        }
        let mut sorted = self.latencies_ms.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let index = ((p / 100.0) * (sorted.len() - 1) as f64) as usize;
        sorted[index]
    }

    pub fn throughput(&self, duration: Duration) -> f64 {
        if duration.as_secs_f64() == 0.0 {
            return 0.0;
        }
        self.successful_requests as f64 / duration.as_secs_f64()
    }

    pub fn print_summary(&self, duration: Duration) {
        println!("\n========== 性能测试结果 ==========");
        println!("总请求数: {}", self.total_requests);
        println!("成功请求: {}", self.successful_requests);
        println!("失败请求: {}", self.failed_requests);
        println!("成功率: {:.2}%", self.success_rate());
        println!("吞吐量: {:.2} req/s", self.throughput(duration));
        println!("平均延迟: {:.2}ms", self.avg_latency_ms());
        println!("P50 延迟: {:.2}ms", self.p50_latency_ms());
        println!("P95 延迟: {:.2}ms", self.p95_latency_ms());
        println!("P99 延迟: {:.2}ms", self.p99_latency_ms());
        println!("==================================\n");
    }
}

/// 负载测试执行器
pub struct LoadTestRunner {
    config: LoadTestConfig,
    semaphore: Arc<Semaphore>,
}

impl LoadTestRunner {
    pub fn new(config: LoadTestConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.concurrent_users));
        Self { config, semaphore }
    }

    /// 执行负载测试
    pub async fn run<F, Fut>(&self, task: F) -> LoadTestMetrics
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Duration, String>> + Send,
    {
        let task = Arc::new(task);
        let mut metrics = LoadTestMetrics::default();
        let start = Instant::now();

        // 预热阶段
        if self.config.warmup_duration > Duration::ZERO {
            println!("预热中...");
            tokio::time::sleep(self.config.warmup_duration).await;
        }

        println!("开始负载测试...");
        let test_start = Instant::now();

        while test_start.elapsed() < self.config.duration {
            let permit = self.semaphore.clone().acquire_owned().await.unwrap();
            let task = task.clone();

            let result = tokio::spawn(async move {
                let _permit = permit;
                task().await
            })
            .await;

            metrics.total_requests += 1;

            match result {
                Ok(Ok(latency)) => {
                    metrics.successful_requests += 1;
                    metrics.latencies_ms.push(latency.as_secs_f64() * 1000.0);
                }
                Ok(Err(e)) => {
                    metrics.failed_requests += 1;
                    if metrics.errors.len() < 100 {
                        metrics.errors.push(e);
                    }
                }
                Err(e) => {
                    metrics.failed_requests += 1;
                    if metrics.errors.len() < 100 {
                        metrics.errors.push(format!("Task panic: {}", e));
                    }
                }
            }

            // 速率限制
            if let Some(rps) = self.config.requests_per_second {
                let expected_requests = (test_start.elapsed().as_secs_f64() * rps as f64) as u64;
                if metrics.total_requests > expected_requests {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
        }

        let total_duration = start.elapsed();
        metrics.print_summary(total_duration);
        metrics
    }
}

/// 性能测试断言
pub struct PerformanceAssertions;

impl PerformanceAssertions {
    /// 断言成功率达标
    pub fn assert_success_rate(metrics: &LoadTestMetrics, min_rate: f64) {
        let rate = metrics.success_rate();
        assert!(
            rate >= min_rate,
            "成功率 {:.2}% 低于目标 {:.2}%",
            rate,
            min_rate
        );
    }

    /// 断言 P99 延迟在范围内
    pub fn assert_p99_latency(metrics: &LoadTestMetrics, max_ms: f64) {
        let p99 = metrics.p99_latency_ms();
        assert!(
            p99 <= max_ms,
            "P99 延迟 {:.2}ms 超过目标 {:.2}ms",
            p99,
            max_ms
        );
    }

    /// 断言吞吐量达标
    pub fn assert_throughput(metrics: &LoadTestMetrics, duration: Duration, min_rps: f64) {
        let throughput = metrics.throughput(duration);
        assert!(
            throughput >= min_rps,
            "吞吐量 {:.2} req/s 低于目标 {:.2} req/s",
            throughput,
            min_rps
        );
    }
}
