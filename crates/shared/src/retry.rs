//! 重试策略与执行器
//!
//! 提供指数退避重试机制，用于瞬时故障（网络抖动、数据库连接池满等）的自动恢复。
//! 业务逻辑错误（如参数无效）不应被重试——由调用方通过 `is_retryable` 闭包控制。

use std::future::Future;
use std::time::Duration;

use tracing::{info, warn};

use crate::error::BadgeError;

// ---------------------------------------------------------------------------
// RetryPolicy — 重试策略配置
// ---------------------------------------------------------------------------

/// 重试策略配置
///
/// 使用指数退避避免重试风暴：首次失败等 1 秒，第 2 次等 2 秒，
/// 第 3 次等 4 秒...直到达到最大间隔或最大重试次数。
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// 最大重试次数（不含首次执行）
    pub max_retries: u32,
    /// 首次重试前的等待时间
    pub initial_delay: Duration,
    /// 退避时间上限，防止等待过长
    pub max_delay: Duration,
    /// 每次重试的退避倍数
    pub multiplier: f64,
}

impl Default for RetryPolicy {
    /// 默认策略：最多重试 3 次，初始等待 1 秒，最大等待 30 秒，倍数 2.0
    ///
    /// 该默认值适用于大多数基础设施层的瞬时故障场景。
    /// 如需更激进或保守的策略，可在构造时覆盖各字段。
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
        }
    }
}

impl RetryPolicy {
    /// 计算第 N 次重试的等待时间（attempt 从 0 开始）
    ///
    /// 公式: initial_delay * multiplier^attempt，结果不超过 max_delay。
    /// 使用 f64 运算后再转回 Duration，接受微秒级精度损失——
    /// 对秒级退避场景而言完全可接受。
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base_ms = self.initial_delay.as_millis() as f64;
        let delay_ms = base_ms * self.multiplier.powi(attempt as i32);
        let capped_ms = delay_ms.min(self.max_delay.as_millis() as f64);
        Duration::from_millis(capped_ms as u64)
    }

    /// 是否应继续重试
    ///
    /// attempt 表示已经失败的次数（从 0 开始计数的重试轮次），
    /// 当 attempt < max_retries 时返回 true。
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_retries
    }
}

// ---------------------------------------------------------------------------
// retry_with_policy — 带重试的异步执行器
// ---------------------------------------------------------------------------

/// 带重试的异步执行器
///
/// 对任意异步操作应用重试策略。仅在操作返回可重试错误时才重试，
/// 业务逻辑错误（如参数无效）不会被重试，直接向上传播。
pub async fn retry_with_policy<F, Fut, T>(
    policy: &RetryPolicy,
    operation_name: &str,
    is_retryable: impl Fn(&BadgeError) -> bool,
    mut operation: F,
) -> Result<T, BadgeError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, BadgeError>>,
{
    let mut attempt: u32 = 0;

    loop {
        match operation().await {
            Ok(value) => {
                if attempt > 0 {
                    info!(operation = operation_name, attempt, "操作在重试后成功");
                }
                return Ok(value);
            }
            Err(err) => {
                // 非瞬时错误不重试，直接返回
                if !is_retryable(&err) {
                    warn!(
                        operation = operation_name,
                        error = %err,
                        "操作失败且不可重试，直接返回错误"
                    );
                    return Err(err);
                }

                // 已用尽重试次数
                if !policy.should_retry(attempt) {
                    warn!(
                        operation = operation_name,
                        attempt,
                        max_retries = policy.max_retries,
                        error = %err,
                        "已达最大重试次数，放弃重试"
                    );
                    return Err(err);
                }

                let delay = policy.delay_for_attempt(attempt);
                warn!(
                    operation = operation_name,
                    attempt,
                    delay_ms = delay.as_millis() as u64,
                    error = %err,
                    "操作失败，将在退避后重试"
                );

                tokio::time::sleep(delay).await;
                attempt += 1;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn test_default_retry_policy() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 3);
        assert_eq!(policy.initial_delay, Duration::from_secs(1));
        assert_eq!(policy.max_delay, Duration::from_secs(30));
        assert!((policy.multiplier - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_delay_for_attempt_exponential_backoff() {
        let policy = RetryPolicy::default();

        // attempt 0: 1s * 2^0 = 1s
        assert_eq!(policy.delay_for_attempt(0), Duration::from_secs(1));
        // attempt 1: 1s * 2^1 = 2s
        assert_eq!(policy.delay_for_attempt(1), Duration::from_secs(2));
        // attempt 2: 1s * 2^2 = 4s
        assert_eq!(policy.delay_for_attempt(2), Duration::from_secs(4));
        // attempt 3: 1s * 2^3 = 8s
        assert_eq!(policy.delay_for_attempt(3), Duration::from_secs(8));
    }

    #[test]
    fn test_delay_capped_at_max() {
        let policy = RetryPolicy {
            max_retries: 10,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(5),
            multiplier: 2.0,
        };

        // attempt 0: 1s
        assert_eq!(policy.delay_for_attempt(0), Duration::from_secs(1));
        // attempt 1: 2s
        assert_eq!(policy.delay_for_attempt(1), Duration::from_secs(2));
        // attempt 2: 4s
        assert_eq!(policy.delay_for_attempt(2), Duration::from_secs(4));
        // attempt 3: 8s -> 受限于 max_delay -> 5s
        assert_eq!(policy.delay_for_attempt(3), Duration::from_secs(5));
        // attempt 10: 仍受限于 max_delay -> 5s
        assert_eq!(policy.delay_for_attempt(10), Duration::from_secs(5));
    }

    #[test]
    fn test_should_retry() {
        let policy = RetryPolicy {
            max_retries: 3,
            ..RetryPolicy::default()
        };

        assert!(policy.should_retry(0));
        assert!(policy.should_retry(1));
        assert!(policy.should_retry(2));
        // 第 3 次（已重试 3 次）不再重试
        assert!(!policy.should_retry(3));
        assert!(!policy.should_retry(4));
    }

    #[tokio::test]
    async fn test_retry_with_policy_succeeds_first_try() {
        let policy = RetryPolicy::default();
        let call_count = Arc::new(AtomicU32::new(0));
        let counter = call_count.clone();

        let result = retry_with_policy(
            &policy,
            "test_op",
            |_| true,
            || {
                let counter = counter.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, BadgeError>(42)
                }
            },
        )
        .await;

        assert_eq!(result.unwrap(), 42);
        // 首次即成功，只调用 1 次
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_with_policy_succeeds_after_retries() {
        // 使用极短的退避时间，避免测试等待过久
        let policy = RetryPolicy {
            max_retries: 3,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            multiplier: 2.0,
        };
        let call_count = Arc::new(AtomicU32::new(0));
        let counter = call_count.clone();

        let result = retry_with_policy(
            &policy,
            "test_op",
            |_| true,
            || {
                let counter = counter.clone();
                async move {
                    let n = counter.fetch_add(1, Ordering::SeqCst);
                    if n < 2 {
                        // 前两次失败
                        Err(BadgeError::Kafka("模拟瞬时故障".to_string()))
                    } else {
                        // 第三次成功
                        Ok(99)
                    }
                }
            },
        )
        .await;

        assert_eq!(result.unwrap(), 99);
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_with_policy_exhausts_retries() {
        let policy = RetryPolicy {
            max_retries: 2,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            multiplier: 2.0,
        };
        let call_count = Arc::new(AtomicU32::new(0));
        let counter = call_count.clone();

        let result: Result<i32, _> = retry_with_policy(
            &policy,
            "test_op",
            |_| true,
            || {
                let counter = counter.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err(BadgeError::Kafka("持续故障".to_string()))
                }
            },
        )
        .await;

        assert!(result.is_err());
        // 首次执行 + 2 次重试 = 3 次调用
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }
}
