//! 徽章管理后台服务（B端）
//!
//! 提供徽章配置、发放管理、统计报表等 REST API。

use std::sync::Arc;

use axum::{Json, Router, middleware, routing::get};
use badge_admin_service::{routes, state::AppState};
use badge_shared::{
    cache::Cache,
    config::AppConfig,
    database::Database,
    observability::{self, ObservabilityConfig, middleware as obs_middleware},
};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化可观测性（tracing + metrics）
    let obs_config = ObservabilityConfig::from_env("badge-admin-service");
    let _guard = observability::init(&obs_config).await?;

    let config = AppConfig::load("badge-admin-service").unwrap_or_default();
    info!("Starting badge-admin-service on {}", config.server_addr());

    // 初始化基础设施
    let db = Database::connect(&config.database).await?;
    let cache = Arc::new(Cache::new(&config.redis)?);

    let state = AppState::new(db.pool().clone(), cache.clone());

    // CORS 配置：开发阶段全放行，生产环境应收紧 allow_origin
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .nest("/api/admin", routes::api_routes())
        .route("/health", get(health_check))
        .route(
            "/ready",
            get({
                let db_for_ready = db;
                let cache_for_ready = cache;
                move || readiness_check(db_for_ready.clone(), cache_for_ready.clone())
            }),
        )
        .layer(cors)
        // 可观测性中间件：请求追踪和指标收集
        .layer(middleware::from_fn(obs_middleware::http_tracing))
        .layer(middleware::from_fn(obs_middleware::request_id))
        .with_state(state);

    let listener = TcpListener::bind(config.server_addr()).await?;
    info!("Listening on {}", config.server_addr());

    axum::serve(listener, app).await?;

    Ok(())
}

/// 存活探针：服务进程正常即返回 ok
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "badge-admin-service"
    }))
}

/// 就绪探针：检查数据库和 Redis 连接是否可用
///
/// K8s 就绪探针失败时会将 Pod 从 Service 端点移除，
/// 避免将流量路由到无法正常处理请求的实例。
async fn readiness_check(db: Database, cache: Arc<Cache>) -> Json<serde_json::Value> {
    let db_ok = db.health_check().await.is_ok();
    let cache_ok = cache.health_check().await.is_ok();
    let all_ok = db_ok && cache_ok;

    Json(serde_json::json!({
        "status": if all_ok { "ok" } else { "degraded" },
        "service": "badge-admin-service",
        "checks": {
            "database": if db_ok { "ok" } else { "fail" },
            "redis": if cache_ok { "ok" } else { "fail" }
        }
    }))
}
