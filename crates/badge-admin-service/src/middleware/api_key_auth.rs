//! API Key 认证中间件
//!
//! 用于验证外部系统通过 X-API-Key 头部传递的 API Key。
//! 与 JWT 认证互斥使用，适用于服务端到服务端的调用场景。

use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use badge_shared::cache::Cache;
use serde_json::json;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use chrono::Utc;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tracing::{debug, warn};

/// API Key Header 名称
const API_KEY_HEADER: &str = "X-API-Key";

/// 外部 API 路由的共享 State
///
/// 认证需要 PgPool 查库，限流需要 Cache 操作 Redis，
/// 将两者封装为独立类型避免污染全局 AppState。
#[derive(Clone)]
pub struct ExternalApiState {
    pub pool: PgPool,
    pub cache: Arc<Cache>,
}

/// API Key 验证结果
#[derive(Debug, Clone)]
pub struct ApiKeyContext {
    pub key_id: i64,
    pub name: String,
    pub permissions: Vec<String>,
}

/// 计算 API Key 的 SHA256 哈希
fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// API Key 认证中间件
///
/// 从 X-API-Key Header 提取 key，验证后注入 ApiKeyContext 到请求扩展。
/// 认证通过后会对每个 API Key 执行基于 Redis 滑动窗口的限流检查。
pub async fn api_key_auth_middleware(
    State(state): State<ExternalApiState>,
    headers: HeaderMap,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let pool = &state.pool;
    let cache = &state.cache;

    // 从 Header 提取 API Key
    let api_key = match headers.get(API_KEY_HEADER) {
        Some(value) => match value.to_str() {
            Ok(key) => key.to_string(),
            Err(_) => {
                warn!("Invalid API Key header encoding");
                return Err((StatusCode::UNAUTHORIZED, "Invalid API Key header").into_response());
            }
        },
        None => {
            return Err((StatusCode::UNAUTHORIZED, "Missing API Key").into_response());
        }
    };

    let key_hash = hash_api_key(&api_key);

    // 查询时一并取出 rate_limit，避免限流逻辑需要额外一次数据库查询
    #[allow(clippy::type_complexity)]
    let row: Option<(i64, String, sqlx::types::JsonValue, bool, Option<chrono::DateTime<Utc>>, Option<i32>)> =
        sqlx::query_as(
            r#"
            SELECT id, name, permissions, enabled, expires_at, rate_limit
            FROM api_key
            WHERE key_hash = $1
            "#,
        )
        .bind(&key_hash)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            warn!(error = %e, "Database error during API Key validation");
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
        })?;

    let row = match row {
        Some(r) => r,
        None => {
            warn!(key_prefix = &api_key[..std::cmp::min(6, api_key.len())], "Invalid API Key");
            return Err((StatusCode::UNAUTHORIZED, "Invalid API Key").into_response());
        }
    };

    let (key_id, name, permissions_json, enabled, expires_at, rate_limit) = row;

    if !enabled {
        warn!(key_id = key_id, "API Key is disabled");
        return Err((StatusCode::UNAUTHORIZED, "API Key is disabled").into_response());
    }

    if let Some(exp) = expires_at
        && exp < Utc::now()
    {
        warn!(key_id = key_id, "API Key has expired");
        return Err((StatusCode::UNAUTHORIZED, "API Key has expired").into_response());
    }

    // 限流检查：只在配置了 rate_limit 且值大于 0 时生效，
    // NULL 或 0 表示不限流，避免对内部高信任 Key 产生不必要的 Redis 开销
    if let Some(limit) = rate_limit {
        if limit > 0 {
            if let Err(resp) = check_rate_limit(cache, key_id, limit).await {
                return Err(resp);
            }
        }
    }

    // 更新最后使用时间（异步，不阻塞请求）
    let pool_clone = pool.clone();
    tokio::spawn(async move {
        let _ = sqlx::query("UPDATE api_key SET last_used_at = NOW() WHERE id = $1")
            .bind(key_id)
            .execute(&pool_clone)
            .await;
    });

    let permissions: Vec<String> = serde_json::from_value(permissions_json).unwrap_or_default();

    let context = ApiKeyContext {
        key_id,
        name,
        permissions,
    };

    request.extensions_mut().insert(context);

    debug!(key_id = key_id, "API Key authenticated successfully");

    Ok(next.run(request).await)
}

/// 基于 Redis 的滑动窗口限流
///
/// 使用「当前分钟」作为窗口 key 的一部分，配合 INCR + EXPIRE 实现固定窗口计数。
/// 选择固定窗口而非精确滑动窗口是因为：
/// 1. 实现简单，单次 INCR 即可完成，无需 Lua 脚本或 ZSET
/// 2. 对 API Key 级别的限流精度已经足够
/// 3. 最坏情况下窗口边界处可能短暂出现 2x 突发，但 API Key 场景可接受
async fn check_rate_limit(
    cache: &Cache,
    key_id: i64,
    limit: i32,
) -> std::result::Result<(), Response> {
    // 用当前分钟数作为窗口标识，同一分钟内的请求共享同一个计数器
    let now = Utc::now();
    let window = now.format("%Y%m%d%H%M").to_string();
    let rate_key = format!("rate_limit:{}:{}", key_id, window);

    let current = cache.incr(&rate_key, 1).await.map_err(|e| {
        // Redis 故障时放行请求，避免缓存不可用导致所有外部 API 全部 503
        warn!(error = %e, key_id = key_id, "Redis rate limit check failed, allowing request");
        return;
    });

    let current = match current {
        Ok(count) => count,
        // Redis 异常时降级放行
        Err(()) => return Ok(()),
    };

    // 首次写入时设置 TTL，确保计数器在窗口结束后自动清理
    if current == 1 {
        if let Err(e) = cache.expire(&rate_key, Duration::from_secs(60)).await {
            warn!(error = %e, "Failed to set rate limit key TTL");
        }
    }

    if current > limit as i64 {
        warn!(
            key_id = key_id,
            current = current,
            limit = limit,
            "API Key rate limit exceeded"
        );
        let body = json!({
            "success": false,
            "code": "RATE_LIMITED",
            "message": format!("Rate limit exceeded: {} requests per minute", limit),
            "data": null
        });
        return Err((StatusCode::TOO_MANY_REQUESTS, axum::Json(body)).into_response());
    }

    Ok(())
}

/// 从请求扩展中提取 API Key 上下文
pub fn extract_api_key_context(request: &Request<Body>) -> Option<&ApiKeyContext> {
    request.extensions().get::<ApiKeyContext>()
}

/// 检查 API Key 是否有指定权限
pub fn check_api_key_permission(context: &ApiKeyContext, required_permission: &str) -> bool {
    // 通配符权限表示拥有所有权限
    if context.permissions.contains(&"*".to_string()) {
        return true;
    }

    context.permissions.contains(&required_permission.to_string())
}

/// API Key 权限校验中间件工厂
///
/// 与 API Key 认证中间件配合使用，检查 ApiKeyContext 中是否包含所需权限。
/// 支持通配符 "*" 表示拥有全部权限。
/// 必须在 api_key_auth_middleware 之后（即内层）使用，
/// 因为它依赖 ApiKeyContext 已被注入到请求扩展中。
pub fn require_api_key_permission(
    required: &'static str,
) -> impl Fn(
    Request<Body>,
    Next,
) -> Pin<Box<dyn Future<Output = Response> + Send>>
       + Clone
       + Send {
    move |request: Request<Body>, next: Next| {
        let required = required;
        Box::pin(async move {
            let context = request.extensions().get::<ApiKeyContext>().cloned();

            match context {
                Some(ctx) => {
                    if !check_api_key_permission(&ctx, required) {
                        warn!(
                            key_id = ctx.key_id,
                            required_permission = required,
                            "API Key permission denied"
                        );
                        let body = json!({
                            "success": false,
                            "code": "FORBIDDEN",
                            "message": format!("API Key lacks permission: {}", required),
                            "data": null
                        });
                        return (StatusCode::FORBIDDEN, axum::Json(body)).into_response();
                    }
                    next.run(request).await
                }
                None => {
                    let body = json!({
                        "success": false,
                        "code": "UNAUTHORIZED",
                        "message": "Missing API Key context",
                        "data": null
                    });
                    (StatusCode::UNAUTHORIZED, axum::Json(body)).into_response()
                }
            }
        })
    }
}
