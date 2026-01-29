//! 统一规则引擎服务
//!
//! 提供 gRPC 接口的规则评估服务。

use anyhow::Result;
use badge_proto::rule_engine::rule_engine_service_server::RuleEngineServiceServer;
use badge_shared::config::AppConfig;
use rule_engine::{RuleEngineServiceImpl, RuleStore};
use std::net::SocketAddr;
use tokio::signal;
use tonic::transport::Server;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

/// 服务配置
struct ServiceConfig {
    grpc_addr: SocketAddr,
    health_addr: SocketAddr,
}

impl ServiceConfig {
    fn from_app_config(config: &AppConfig) -> Self {
        let grpc_port = config.server.port;
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
    // 初始化日志
    init_tracing();

    info!("Starting unified-rule-engine service...");

    // 加载配置
    let config = AppConfig::load("unified-rule-engine").unwrap_or_else(|e| {
        tracing::warn!("Failed to load config, using defaults: {}", e);
        AppConfig::default()
    });

    let service_config = ServiceConfig::from_app_config(&config);

    // 创建规则存储
    let store = RuleStore::new();
    info!("Rule store initialized");

    // 创建 gRPC 服务
    let rule_service = RuleEngineServiceImpl::new(store.clone());

    // 启动健康检查服务
    let health_addr = service_config.health_addr;
    let health_store = store.clone();
    tokio::spawn(async move {
        run_health_server(health_addr, health_store).await;
    });

    // 启动 gRPC 服务
    info!("gRPC server listening on {}", service_config.grpc_addr);

    Server::builder()
        .add_service(RuleEngineServiceServer::new(rule_service))
        .serve_with_shutdown(service_config.grpc_addr, shutdown_signal())
        .await?;

    info!("Service shutdown complete");
    Ok(())
}

/// 初始化 tracing
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,unified_rule_engine=debug"));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).with_level(true))
        .with(filter)
        .init();
}

/// 健康检查服务器
async fn run_health_server(addr: SocketAddr, store: RuleStore) {
    use axum::{Json, Router, routing::get};
    use serde::Serialize;

    #[derive(Serialize)]
    struct HealthResponse {
        status: String,
        service: String,
        rules_count: usize,
    }

    #[derive(Serialize)]
    struct ReadyResponse {
        ready: bool,
        rules_count: usize,
    }

    let store_health = store.clone();
    let store_ready = store.clone();

    let app = Router::new()
        .route(
            "/health",
            get(move || async move {
                Json(HealthResponse {
                    status: "healthy".to_string(),
                    service: "unified-rule-engine".to_string(),
                    rules_count: store_health.len(),
                })
            }),
        )
        .route(
            "/ready",
            get(move || async move {
                Json(ReadyResponse {
                    ready: true,
                    rules_count: store_ready.len(),
                })
            }),
        );

    info!("Health server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// 优雅关闭信号处理
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
