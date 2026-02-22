//! 分级限流中间件
//!
//! 基于 Redis 滑动窗口计数器实现请求限流，按 API 类型分级限制：
//! - 批量操作（/tasks, /batch）: 最严格，默认 10 req/min
//! - 写操作（POST/PUT/DELETE）: 中等，默认 100 req/min
//! - 读操作（GET）: 最宽松，默认 500 req/min
//!
//! 支持两个维度的限流：
//! - 用户级：按 JWT sub（用户 ID）限流，防止单个用户滥用
//! - 全局级：所有请求共享配额，防止系统整体过载
//!
//! 使用 Redis INCR + EXPIRE 实现分布式计数，支持多实例部署。

use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;
use tracing::warn;

use badge_shared::cache::Cache;

use crate::auth::Claims;
use crate::state::AppState;

/// 限流级别配置
///
/// 每个级别对应不同的请求配额和时间窗口
#[derive(Debug, Clone, Copy)]
struct RateLimit {
    /// 时间窗口内允许的最大请求数
    max_requests: i64,
    /// 时间窗口（秒）
    window_secs: u64,
}

/// 限流配置
///
/// 预设三个级别的限流参数，通过 API 路径和 HTTP 方法自动分级
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// 批量操作限制（最严格）
    batch: RateLimit,
    /// 写操作限制
    write: RateLimit,
    /// 读操作限制（最宽松）
    read: RateLimit,
    /// 全局限流倍数：全局配额 = 用户配额 * 此倍数
    global_multiplier: i64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            batch: RateLimit {
                max_requests: 10,
                window_secs: 60,
            },
            write: RateLimit {
                max_requests: 100,
                window_secs: 60,
            },
            read: RateLimit {
                max_requests: 500,
                window_secs: 60,
            },
            global_multiplier: 50,
        }
    }
}

/// 限流中间件
///
/// 放置在 auth 中间件之后（需要从 Claims 中提取用户 ID），
/// 在 audit 中间件之前（被限流的请求不应记录审计日志）。
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();

    // 健康检查和监控端点跳过限流
    if is_exempt_path(&path) {
        return next.run(request).await;
    }

    let method = request.method().clone();
    let config = RateLimitConfig::default();
    let limit = classify_rate_limit(&path, &method, &config);

    // 从 auth 中间件注入的 Claims 中提取用户 ID
    let user_id = request.extensions().get::<Claims>().map(|c| c.sub.clone());

    // 1. 用户级限流（仅已认证请求）
    if let Some(ref uid) = user_id {
        let user_key = format!(
            "rl:user:{}:{}:{}",
            uid,
            rate_tier_name(&path, &method),
            window_key(limit.window_secs)
        );
        match check_rate_limit(
            &state.cache,
            &user_key,
            limit.max_requests,
            limit.window_secs,
        )
        .await
        {
            Ok(remaining) if remaining < 0 => {
                warn!(
                    user_id = uid,
                    path = %path,
                    tier = rate_tier_name(&path, &method),
                    "用户限流触发"
                );
                return too_many_requests_response(limit.window_secs);
            }
            Err(e) => {
                // Redis 不可用时放行，避免限流服务故障导致业务不可用
                warn!(error = %e, "Redis 限流检查失败，跳过限流");
            }
            _ => {}
        }
    }

    // 2. 全局级限流
    let global_limit = limit.max_requests * config.global_multiplier;
    let global_key = format!(
        "rl:global:{}:{}",
        rate_tier_name(&path, &method),
        window_key(limit.window_secs)
    );
    match check_rate_limit(&state.cache, &global_key, global_limit, limit.window_secs).await {
        Ok(remaining) if remaining < 0 => {
            warn!(
                path = %path,
                tier = rate_tier_name(&path, &method),
                "全局限流触发"
            );
            return too_many_requests_response(limit.window_secs);
        }
        Err(e) => {
            warn!(error = %e, "Redis 全局限流检查失败，跳过限流");
        }
        _ => {}
    }

    next.run(request).await
}

/// 使用 Redis INCR + EXPIRE 实现固定窗口计数器
///
/// 返回剩余配额（负数表示已超限）。
/// 利用 INCR 的原子性保证多实例部署时计数准确。
async fn check_rate_limit(
    cache: &Arc<Cache>,
    key: &str,
    max_requests: i64,
    window_secs: u64,
) -> Result<i64, String> {
    // INCR 在 key 不存在时自动创建并设为 1
    let count = cache
        .incr(key, 1)
        .await
        .map_err(|e| format!("Redis INCR 失败: {}", e))?;

    // 首次创建时设置过期时间，确保窗口到期后自动清理
    if count == 1 {
        let _ = cache
            .expire(key, std::time::Duration::from_secs(window_secs))
            .await;
    }

    Ok(max_requests - count)
}

/// 根据路径和方法分级确定限流参数
///
/// 批量操作 > 写操作 > 读操作（限制从严到宽）
fn classify_rate_limit<'a>(path: &str, method: &Method, config: &'a RateLimitConfig) -> RateLimit {
    // 批量操作路径特征：任务接口和批量操作接口
    if path.contains("/tasks") || path.contains("/batch") || path.contains("/upload-csv") {
        return config.batch;
    }

    // 按 HTTP 方法区分读写
    match *method {
        Method::GET | Method::HEAD | Method::OPTIONS => config.read,
        _ => config.write,
    }
}

/// 返回当前窗口的时间标识
///
/// 以窗口大小对齐的 Unix 时间戳，相同窗口内的请求共享同一个计数器
fn window_key(window_secs: u64) -> u64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    now / window_secs
}

/// 获取限流层级名称（用于 Redis key 的命名空间隔离）
fn rate_tier_name<'a>(path: &str, method: &Method) -> &'a str {
    if path.contains("/tasks") || path.contains("/batch") || path.contains("/upload-csv") {
        "batch"
    } else {
        match *method {
            Method::GET | Method::HEAD | Method::OPTIONS => "read",
            _ => "write",
        }
    }
}

/// 免限流路径
fn is_exempt_path(path: &str) -> bool {
    matches!(path, "/health" | "/ready" | "/metrics") || path.starts_with("/api/admin/auth/")
}

/// 生成 429 Too Many Requests 响应
///
/// 包含 Retry-After 头，告知客户端何时可以重试
fn too_many_requests_response(window_secs: u64) -> Response {
    let body = json!({
        "success": false,
        "code": "RATE_LIMITED",
        "message": "请求过于频繁，请稍后再试",
        "data": null
    });

    let mut response = (StatusCode::TOO_MANY_REQUESTS, axum::Json(body)).into_response();
    // Retry-After 使用窗口剩余秒数的近似值
    if let Ok(val) = axum::http::HeaderValue::from_str(&window_secs.to_string()) {
        response.headers_mut().insert("Retry-After", val);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_batch_operations() {
        let config = RateLimitConfig::default();

        // 任务接口
        let limit = classify_rate_limit("/api/admin/tasks", &Method::POST, &config);
        assert_eq!(limit.max_requests, 10);

        // 批量操作
        let limit = classify_rate_limit("/api/admin/grants/batch", &Method::POST, &config);
        assert_eq!(limit.max_requests, 10);

        // CSV 上传
        let limit = classify_rate_limit("/api/admin/grants/upload-csv", &Method::POST, &config);
        assert_eq!(limit.max_requests, 10);
    }

    #[test]
    fn test_classify_write_operations() {
        let config = RateLimitConfig::default();

        let limit = classify_rate_limit("/api/admin/badges", &Method::POST, &config);
        assert_eq!(limit.max_requests, 100);

        let limit = classify_rate_limit("/api/admin/badges/1", &Method::PUT, &config);
        assert_eq!(limit.max_requests, 100);

        let limit = classify_rate_limit("/api/admin/badges/1", &Method::DELETE, &config);
        assert_eq!(limit.max_requests, 100);
    }

    #[test]
    fn test_classify_read_operations() {
        let config = RateLimitConfig::default();

        let limit = classify_rate_limit("/api/admin/badges", &Method::GET, &config);
        assert_eq!(limit.max_requests, 500);

        let limit = classify_rate_limit("/api/admin/users/search", &Method::GET, &config);
        assert_eq!(limit.max_requests, 500);
    }

    #[test]
    fn test_exempt_paths() {
        assert!(is_exempt_path("/health"));
        assert!(is_exempt_path("/ready"));
        assert!(is_exempt_path("/metrics"));
        assert!(is_exempt_path("/api/admin/auth/login"));
        assert!(!is_exempt_path("/api/admin/badges"));
        assert!(!is_exempt_path("/api/admin/tasks"));
    }

    #[test]
    fn test_rate_tier_name() {
        assert_eq!(rate_tier_name("/api/admin/tasks", &Method::POST), "batch");
        assert_eq!(
            rate_tier_name("/api/admin/grants/batch", &Method::POST),
            "batch"
        );
        assert_eq!(rate_tier_name("/api/admin/badges", &Method::POST), "write");
        assert_eq!(rate_tier_name("/api/admin/badges", &Method::GET), "read");
    }

    #[test]
    fn test_window_key_stability() {
        // 同一窗口内调用应返回相同的 key
        let key1 = window_key(60);
        let key2 = window_key(60);
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_default_config() {
        let config = RateLimitConfig::default();
        assert_eq!(config.batch.max_requests, 10);
        assert_eq!(config.write.max_requests, 100);
        assert_eq!(config.read.max_requests, 500);
        assert_eq!(config.global_multiplier, 50);
    }
}
