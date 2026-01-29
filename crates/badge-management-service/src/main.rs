//! 徽章管理服务（C端）
//!
//! 提供徽章查询、兑换、展示等 C 端功能的 gRPC 服务入口。

use anyhow::Result;
use badge_proto::badge::badge_management_service_server::BadgeManagementServiceServer;
use badge_shared::{cache::Cache, config::AppConfig, database::Database};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tonic::transport::Server;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use badge_management::{
    grpc::BadgeManagementServiceImpl,
    repository::{
        BadgeLedgerRepository, BadgeRepository, RedemptionRepository, UserBadgeRepository,
    },
    service::{BadgeQueryService, GrantService, RedemptionService, RevokeService},
};

/// 服务配置
///
/// 从 AppConfig 中提取服务启动所需的地址配置
struct ServiceConfig {
    grpc_addr: SocketAddr,
    health_addr: SocketAddr,
}

impl ServiceConfig {
    fn from_app_config(config: &AppConfig) -> Self {
        let grpc_port = config.server.port;
        // 健康检查端点使用 metrics_port（与规则引擎保持一致）
        let health_port = config.observability.metrics_port;

        Self {
            grpc_addr: format!("{}:{}", config.server.host, grpc_port)
                .parse()
                .expect("Invalid gRPC address"),
            health_addr: format!("{}:{}", config.server.host, health_port)
                .parse()
                .expect("Invalid health address"),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 1. 初始化日志
    init_tracing();

    info!("Starting badge-management-service...");

    // 2. 加载配置
    let config = AppConfig::load("badge-management-service").unwrap_or_else(|e| {
        tracing::warn!("Failed to load config, using defaults: {}", e);
        AppConfig::default()
    });
    info!(
        environment = %config.environment,
        "Configuration loaded"
    );

    let service_config = ServiceConfig::from_app_config(&config);

    // 3. 初始化数据库连接
    let db = Database::connect(&config.database).await?;
    let pool = db.pool().clone();
    info!("Database connection established");

    // 4. 初始化 Redis 缓存
    let cache = Arc::new(Cache::new(&config.redis)?);
    // 验证 Redis 连接
    cache.health_check().await?;
    info!("Redis connection established");

    // 5. 创建仓储
    let badge_repo = Arc::new(BadgeRepository::new(pool.clone()));
    let user_badge_repo = Arc::new(UserBadgeRepository::new(pool.clone()));
    let ledger_repo = Arc::new(BadgeLedgerRepository::new(pool.clone()));
    let redemption_repo = Arc::new(RedemptionRepository::new(pool.clone()));
    info!("Repositories initialized");

    // 6. 创建服务
    let query_service = Arc::new(BadgeQueryService::new(
        badge_repo.clone(),
        user_badge_repo.clone(),
        redemption_repo.clone(),
        ledger_repo.clone(),
        cache.clone(),
    ));

    let grant_service = Arc::new(GrantService::new(
        badge_repo.clone(),
        cache.clone(),
        pool.clone(),
    ));

    let revoke_service = Arc::new(RevokeService::new(cache.clone(), pool.clone()));

    let redemption_service = Arc::new(RedemptionService::new(
        redemption_repo.clone(),
        cache.clone(),
        pool.clone(),
    ));
    info!("Services initialized");

    // 7. 创建 gRPC 服务
    let grpc_service = BadgeManagementServiceImpl::new(
        query_service,
        grant_service,
        revoke_service,
        redemption_service,
        pool.clone(),
    );

    // 8. 启动健康检查端点（HTTP）
    let health_addr = service_config.health_addr;
    let health_db = db.clone();
    let health_cache = cache.clone();
    tokio::spawn(async move {
        run_health_server(health_addr, health_db, health_cache).await;
    });
    info!("Health server listening on {}", health_addr);

    // 9. 启动 gRPC 服务
    info!("gRPC server listening on {}", service_config.grpc_addr);

    Server::builder()
        .add_service(BadgeManagementServiceServer::new(grpc_service))
        .serve_with_shutdown(service_config.grpc_addr, shutdown_signal())
        .await?;

    info!("Service shutdown complete");
    Ok(())
}

/// 初始化 tracing 日志
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,badge_management=debug"));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).with_level(true))
        .with(filter)
        .init();
}

/// 健康检查服务器
///
/// 提供 /health 和 /ready 端点，用于 Kubernetes 健康探针
async fn run_health_server(addr: SocketAddr, db: Database, cache: Arc<Cache>) {
    use axum::{Json, Router, routing::get};
    use serde::Serialize;

    #[derive(Serialize)]
    struct HealthResponse {
        status: String,
        service: String,
    }

    #[derive(Serialize)]
    struct ReadyResponse {
        ready: bool,
        database: String,
        redis: String,
    }

    let db_ready = db.clone();
    let cache_ready = cache.clone();

    let app = Router::new()
        .route(
            "/health",
            get(|| async {
                Json(HealthResponse {
                    status: "healthy".to_string(),
                    service: "badge-management-service".to_string(),
                })
            }),
        )
        .route(
            "/ready",
            get(move || {
                let db = db_ready.clone();
                let cache = cache_ready.clone();
                async move {
                    let db_status = match db.health_check().await {
                        Ok(_) => "connected",
                        Err(_) => "disconnected",
                    };
                    let redis_status = match cache.health_check().await {
                        Ok(_) => "connected",
                        Err(_) => "disconnected",
                    };

                    let ready = db_status == "connected" && redis_status == "connected";

                    Json(ReadyResponse {
                        ready,
                        database: db_status.to_string(),
                        redis: redis_status.to_string(),
                    })
                }
            }),
        );

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// 优雅关闭信号处理
///
/// 监听 Ctrl+C 和 SIGTERM 信号，用于 Kubernetes 优雅关闭
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, starting graceful shutdown...");
        }
        _ = terminate => {
            info!("Received SIGTERM, starting graceful shutdown...");
        }
    }
}
