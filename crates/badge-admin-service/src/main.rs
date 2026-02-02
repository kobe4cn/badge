//! 徽章管理后台服务（B端）
//!
//! 提供徽章配置、发放管理、统计报表等 REST API。

use std::sync::Arc;

use axum::{Json, Router, middleware, routing::get};
use badge_admin_service::{middleware::auth_middleware, routes, state::AppState};
use badge_proto::badge::badge_management_service_client::BadgeManagementServiceClient;
use badge_shared::{
    cache::Cache,
    config::AppConfig,
    database::Database,
    observability::{self, middleware as obs_middleware},
};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 统一加载配置：从 config/{service_name}.toml 加载，包含可观测性配置
    let config = AppConfig::load("badge-admin-service").unwrap_or_default();

    // 从 AppConfig 中提取可观测性配置并注入服务名
    let obs_config = config.observability.clone().with_service_name(&config.service_name);
    let _guard = observability::init(&obs_config).await?;

    info!("Starting badge-admin-service on {}", config.server_addr());

    // 初始化基础设施
    let db = Database::connect(&config.database).await?;
    let cache = Arc::new(Cache::new(&config.redis)?);

    let state = AppState::new(db.pool().clone(), cache.clone());

    // 尝试连接 badge-management-service 的 gRPC 端点（用于跨服务刷新缓存）
    // 默认地址为 http://127.0.0.1:50052，可通过环境变量 BADGE_MANAGEMENT_GRPC_ADDR 覆盖
    let grpc_addr = std::env::var("BADGE_MANAGEMENT_GRPC_ADDR")
        .unwrap_or_else(|_| "http://127.0.0.1:50052".to_string());

    match BadgeManagementServiceClient::connect(grpc_addr.clone()).await {
        Ok(client) => {
            state.set_badge_management_client(client).await;
            info!("Connected to badge-management-service gRPC at {}", grpc_addr);
        }
        Err(e) => {
            // 连接失败不阻止服务启动，只记录警告
            warn!(
                "Failed to connect to badge-management-service gRPC at {}: {}. \
                Dependency cache refresh will be local-only.",
                grpc_addr, e
            );
        }
    }

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
        // 认证中间件：验证 JWT Token
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
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
