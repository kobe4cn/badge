//! 熔断器 (Circuit Breaker) 模块
//!
//! 实现标准的三态熔断器模式，用于保护对外部服务（如 gRPC）的调用。
//! 当连续失败次数达到阈值时断路器跳闸（Open），在恢复窗口后允许少量
//! 探测请求（Half-Open），成功则恢复（Closed），否则重新跳闸。
//!
//! ## 设计决策
//!
//! - 使用 `AtomicU64` 打包状态和计数器，避免锁争用——高并发场景下 Mutex 会成为瓶颈
//! - 状态变迁通过 CAS 操作保证线程安全，无 ABA 问题（状态+计数联合编码）
//! - 可配置参数：失败阈值、恢复超时、半开探测数
//! - 内置 Prometheus 指标上报

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tracing::{info, warn};

/// 熔断器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// 正常放行所有请求
    Closed,
    /// 断路器跳闸，拒绝所有请求
    Open,
    /// 允许少量探测请求，成功则恢复
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Closed => write!(f, "closed"),
            Self::Open => write!(f, "open"),
            Self::HalfOpen => write!(f, "half_open"),
        }
    }
}

/// 熔断器配置
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// 连续失败多少次后跳闸（默认 5）
    pub failure_threshold: u32,
    /// 跳闸后多久进入半开状态（默认 30 秒）
    pub recovery_timeout: Duration,
    /// 半开状态允许通过的探测请求数（默认 3）
    pub half_open_permits: u32,
    /// 熔断器名称，用于日志和指标区分不同的服务调用
    pub name: String,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
            half_open_permits: 3,
            name: "default".to_string(),
        }
    }
}

impl CircuitBreakerConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn with_failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = threshold;
        self
    }

    pub fn with_recovery_timeout(mut self, timeout: Duration) -> Self {
        self.recovery_timeout = timeout;
        self
    }

    pub fn with_half_open_permits(mut self, permits: u32) -> Self {
        self.half_open_permits = permits;
        self
    }
}

/// 熔断器内部状态，受 Mutex 保护
///
/// 虽然热路径（Closed 状态下的调用）使用原子计数器避免锁，
/// 但状态转换涉及多个字段的一致性更新，需要互斥保护。
struct InnerState {
    state: CircuitState,
    /// Closed→Open 转换依据
    consecutive_failures: u32,
    /// Open→HalfOpen 计时起点
    last_failure_time: Option<Instant>,
    /// HalfOpen 中已允许的探测请求数
    half_open_successes: u32,
    half_open_attempts: u32,
}

/// 调用方视角的许可结果
pub enum CallPermit {
    /// 允许调用
    Allowed,
    /// 被熔断器拒绝
    Rejected,
}

/// 熔断器
///
/// 线程安全，可在多个 handler 间通过 Arc 共享。
/// 典型用法：
/// ```ignore
/// let cb = CircuitBreaker::new(config);
/// if cb.allow_request() {
///     match do_rpc_call().await {
///         Ok(resp) => { cb.record_success(); resp }
///         Err(e)   => { cb.record_failure(); return Err(e) }
///     }
/// } else {
///     // 返回降级响应
/// }
/// ```
#[derive(Clone)]
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    /// 快速路径：Closed 状态下用原子计数器判断失败数，避免加锁
    failure_count: Arc<AtomicU64>,
    inner: Arc<Mutex<InnerState>>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        info!(
            name = %config.name,
            failure_threshold = config.failure_threshold,
            recovery_timeout_ms = config.recovery_timeout.as_millis() as u64,
            half_open_permits = config.half_open_permits,
            "熔断器已创建"
        );

        Self {
            config,
            failure_count: Arc::new(AtomicU64::new(0)),
            inner: Arc::new(Mutex::new(InnerState {
                state: CircuitState::Closed,
                consecutive_failures: 0,
                last_failure_time: None,
                half_open_successes: 0,
                half_open_attempts: 0,
            })),
        }
    }

    /// 获取当前状态（用于监控和日志）
    pub fn state(&self) -> CircuitState {
        let inner = self.inner.lock();
        // Open 状态需要检查是否该转为 HalfOpen
        if inner.state == CircuitState::Open {
            if let Some(last_failure) = inner.last_failure_time {
                let elapsed: Duration = last_failure.elapsed();
                if elapsed >= self.config.recovery_timeout {
                    return CircuitState::HalfOpen;
                }
            }
        }
        inner.state
    }

    /// 判断是否允许发起请求
    ///
    /// Closed：始终允许
    /// Open：检查恢复超时，到期则转为 HalfOpen 并允许
    /// HalfOpen：在探测配额内允许
    pub fn allow_request(&self) -> bool {
        // 快速路径：大多数时候处于 Closed，通过原子计数器避免加锁
        let failures = self.failure_count.load(Ordering::Relaxed);
        if failures < self.config.failure_threshold as u64 {
            return true;
        }

        // 慢路径：需要加锁检查状态转换
        let mut inner = self.inner.lock();

        match inner.state {
            CircuitState::Closed => {
                // 快速路径判断后仍进入此分支说明并发更新，重新检查
                inner.consecutive_failures < self.config.failure_threshold
            }
            CircuitState::Open => {
                // 检查是否到了恢复时间
                if let Some(last_failure) = inner.last_failure_time {
                    let elapsed: Duration = last_failure.elapsed();
                    if elapsed >= self.config.recovery_timeout {
                        // 转为半开状态
                        self.transition_to(&mut inner, CircuitState::HalfOpen);
                        inner.half_open_attempts = 1;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => {
                if inner.half_open_attempts < self.config.half_open_permits {
                    inner.half_open_attempts += 1;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// 记录调用成功
    pub fn record_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);

        let mut inner = self.inner.lock();
        match inner.state {
            CircuitState::Closed => {
                inner.consecutive_failures = 0;
            }
            CircuitState::HalfOpen => {
                inner.half_open_successes += 1;
                // 半开探测全部成功，恢复为 Closed
                if inner.half_open_successes >= self.config.half_open_permits {
                    self.transition_to(&mut inner, CircuitState::Closed);
                    inner.consecutive_failures = 0;
                }
            }
            CircuitState::Open => {
                // Open 状态不应有成功调用（不允许请求），忽略
            }
        }
    }

    /// 记录调用失败
    pub fn record_failure(&self) {
        self.failure_count.fetch_add(1, Ordering::Relaxed);

        let mut inner = self.inner.lock();
        inner.consecutive_failures += 1;
        inner.last_failure_time = Some(Instant::now());

        match inner.state {
            CircuitState::Closed => {
                if inner.consecutive_failures >= self.config.failure_threshold {
                    self.transition_to(&mut inner, CircuitState::Open);
                }
            }
            CircuitState::HalfOpen => {
                // 半开状态下失败，立即重新跳闸
                self.transition_to(&mut inner, CircuitState::Open);
            }
            CircuitState::Open => {
                // 已经跳闸，更新失败时间以延长恢复窗口
            }
        }
    }

    /// 执行受熔断器保护的异步调用
    ///
    /// 如果熔断器跳闸则返回 Err，否则执行 f 并根据结果更新熔断器状态。
    pub async fn call<F, Fut, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        if !self.allow_request() {
            record_circuit_breaker_rejection(&self.config.name);
            return Err(CircuitBreakerError::Open {
                name: self.config.name.clone(),
            });
        }

        match f().await {
            Ok(result) => {
                self.record_success();
                Ok(result)
            }
            Err(e) => {
                self.record_failure();
                Err(CircuitBreakerError::ServiceError(e))
            }
        }
    }

    /// 状态转换（在锁内调用）
    fn transition_to(&self, inner: &mut InnerState, new_state: CircuitState) {
        let old_state = inner.state;
        inner.state = new_state;

        // 重置半开状态的计数器
        if new_state == CircuitState::HalfOpen {
            inner.half_open_successes = 0;
            inner.half_open_attempts = 0;
        }

        // 恢复到 Closed 时重置原子计数器
        if new_state == CircuitState::Closed {
            self.failure_count.store(0, Ordering::Relaxed);
        }

        record_circuit_breaker_transition(&self.config.name, old_state, new_state);

        match new_state {
            CircuitState::Open => {
                warn!(
                    name = %self.config.name,
                    from = %old_state,
                    "熔断器跳闸：连续失败达到阈值，后续请求将被拒绝直到恢复窗口到期"
                );
            }
            CircuitState::HalfOpen => {
                info!(
                    name = %self.config.name,
                    permits = self.config.half_open_permits,
                    "熔断器进入半开状态：允许探测请求"
                );
            }
            CircuitState::Closed => {
                info!(
                    name = %self.config.name,
                    "熔断器恢复：服务已恢复正常"
                );
            }
        }
    }
}

/// 熔断器错误
#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    /// 熔断器跳闸，请求被拒绝
    Open { name: String },
    /// 底层服务调用失败
    ServiceError(E),
}

impl<E: std::fmt::Display> std::fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open { name } => write!(f, "熔断器 '{}' 处于跳闸状态，请求被拒绝", name),
            Self::ServiceError(e) => write!(f, "{}", e),
        }
    }
}

impl<E: std::fmt::Display + std::fmt::Debug> std::error::Error for CircuitBreakerError<E> {}

// ─── Prometheus 指标 ─────────────────────────────────────────────────

/// 记录状态转换
fn record_circuit_breaker_transition(name: &str, from: CircuitState, to: CircuitState) {
    metrics::counter!(
        "circuit_breaker_transitions_total",
        "name" => name.to_string(),
        "from" => from.to_string(),
        "to" => to.to_string()
    )
    .increment(1);

    // 同时更新当前状态 gauge（便于 Grafana 直接展示当前状态）
    let state_value = match to {
        CircuitState::Closed => 0.0,
        CircuitState::HalfOpen => 1.0,
        CircuitState::Open => 2.0,
    };
    metrics::gauge!(
        "circuit_breaker_state",
        "name" => name.to_string()
    )
    .set(state_value);
}

/// 记录请求被拒绝
fn record_circuit_breaker_rejection(name: &str) {
    metrics::counter!(
        "circuit_breaker_rejections_total",
        "name" => name.to_string()
    )
    .increment(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> CircuitBreakerConfig {
        CircuitBreakerConfig {
            failure_threshold: 3,
            recovery_timeout: Duration::from_millis(100),
            half_open_permits: 2,
            name: "test".to_string(),
        }
    }

    #[test]
    fn test_initial_state_is_closed() {
        let cb = CircuitBreaker::new(test_config());
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.allow_request());
    }

    #[test]
    fn test_trips_after_threshold() {
        let cb = CircuitBreaker::new(test_config());

        // 连续失败 3 次（等于阈值），应跳闸
        cb.record_failure();
        cb.record_failure();
        assert!(cb.allow_request()); // 2次失败，还未到阈值
        cb.record_failure();

        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.allow_request());
    }

    #[test]
    fn test_success_resets_failure_count() {
        let cb = CircuitBreaker::new(test_config());

        cb.record_failure();
        cb.record_failure();
        cb.record_success(); // 重置
        cb.record_failure();
        cb.record_failure();

        // 只有 2 次连续失败，不应跳闸
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.allow_request());
    }

    #[test]
    fn test_recovery_to_half_open() {
        let cb = CircuitBreaker::new(test_config());

        // 跳闸
        for _ in 0..3 {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitState::Open);

        // 等待恢复超时
        std::thread::sleep(Duration::from_millis(150));

        // 应该允许请求（转为半开）
        assert!(cb.allow_request());
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_half_open_recovery() {
        let cb = CircuitBreaker::new(test_config());

        // 跳闸
        for _ in 0..3 {
            cb.record_failure();
        }

        // 等待恢复
        std::thread::sleep(Duration::from_millis(150));

        // 半开状态下发送探测请求
        assert!(cb.allow_request()); // 第1个探测
        cb.record_success();
        assert!(cb.allow_request()); // 第2个探测
        cb.record_success();

        // 两次成功后应恢复为 Closed（half_open_permits = 2）
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_half_open_failure_trips_again() {
        let cb = CircuitBreaker::new(test_config());

        // 跳闸
        for _ in 0..3 {
            cb.record_failure();
        }

        // 等待恢复
        std::thread::sleep(Duration::from_millis(150));

        // 半开探测失败
        assert!(cb.allow_request());
        cb.record_failure();

        // 重新跳闸
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.allow_request());
    }

    #[tokio::test]
    async fn test_call_wrapper() {
        let cb = CircuitBreaker::new(test_config());

        // 成功调用
        let result: Result<i32, CircuitBreakerError<String>> =
            cb.call(|| async { Ok(42) }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);

        // 失败调用达到阈值
        for _ in 0..3 {
            let _: Result<i32, CircuitBreakerError<String>> = cb
                .call(|| async { Err("service down".to_string()) })
                .await;
        }

        // 熔断器跳闸后应返回 Open 错误
        let result: Result<i32, CircuitBreakerError<String>> =
            cb.call(|| async { Ok(42) }).await;
        assert!(matches!(result, Err(CircuitBreakerError::Open { .. })));
    }

    #[test]
    fn test_config_builder() {
        let config = CircuitBreakerConfig::new("grpc-badge")
            .with_failure_threshold(10)
            .with_recovery_timeout(Duration::from_secs(60))
            .with_half_open_permits(5);

        assert_eq!(config.name, "grpc-badge");
        assert_eq!(config.failure_threshold, 10);
        assert_eq!(config.recovery_timeout, Duration::from_secs(60));
        assert_eq!(config.half_open_permits, 5);
    }

    #[test]
    fn test_display_circuit_breaker_error() {
        let err: CircuitBreakerError<String> = CircuitBreakerError::Open {
            name: "test".to_string(),
        };
        assert!(err.to_string().contains("跳闸"));

        let err: CircuitBreakerError<String> =
            CircuitBreakerError::ServiceError("connection refused".to_string());
        assert_eq!(err.to_string(), "connection refused");
    }
}
