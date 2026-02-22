//! 统一规则引擎服务
//!
//! 提供 gRPC 接口的规则评估服务。

use anyhow::Result;
use badge_proto::rule_engine::rule_engine_service_server::RuleEngineServiceServer;
use badge_shared::config::AppConfig;
use badge_shared::observability;
use rule_engine::{Rule, RuleEngineServiceImpl, RuleNode, RuleStore};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::signal;
use tonic::transport::Server;
use tracing::{info, warn};

/// 服务配置
struct ServiceConfig {
    grpc_addr: SocketAddr,
}

impl ServiceConfig {
    fn from_app_config(config: &AppConfig) -> Self {
        let grpc_port = config.server.port;

        Self {
            grpc_addr: format!("{}:{}", config.server.host, grpc_port)
                .parse()
                .expect("Invalid gRPC address"),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 统一加载配置：从 config/{service_name}.toml 加载，包含可观测性配置
    let config = AppConfig::load("unified-rule-engine").unwrap_or_else(|e| {
        eprintln!("Failed to load config, using defaults: {}", e);
        AppConfig::default()
    });

    // 从 AppConfig 中提取可观测性配置并注入服务名
    let obs_config = config.observability.clone().with_service_name(&config.service_name);
    let _guard = observability::init(&obs_config).await?;

    info!("Starting unified-rule-engine service...");

    let service_config = ServiceConfig::from_app_config(&config);

    // 创建规则存储
    let store = RuleStore::new();
    info!("Rule store initialized");

    // 连接数据库并加载规则
    match load_rules_from_database(&config, &store).await {
        Ok(count) => info!("Loaded {} rules from database", count),
        Err(e) => warn!("Failed to load rules from database: {}, starting with empty store", e),
    }

    // 创建 gRPC 服务
    let rule_service = RuleEngineServiceImpl::new(store);

    // 启动 gRPC 服务
    // 健康检查端点已由 observability 模块在 metrics_port 上提供
    let tls_config = badge_shared::grpc_tls::build_server_tls_config(&config.tls)
        .await
        .expect("TLS 配置加载失败");

    let mut server_builder = Server::builder();
    if let Some(tls) = tls_config {
        server_builder = server_builder
            .tls_config(tls)
            .expect("gRPC TLS 配置应用失败");
        info!("gRPC server listening on {} (TLS enabled)", service_config.grpc_addr);
    } else {
        info!("gRPC server listening on {} (plaintext)", service_config.grpc_addr);
    }

    server_builder
        .add_service(RuleEngineServiceServer::new(rule_service))
        .serve_with_shutdown(service_config.grpc_addr, shutdown_signal())
        .await?;

    info!("Service shutdown complete");
    Ok(())
}

/// 从数据库加载所有启用的规则
///
/// 查询 badge_rules 表，将 rule_json 转换为规则引擎的 Rule 结构并加载。
async fn load_rules_from_database(config: &AppConfig, store: &RuleStore) -> Result<usize> {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(config.database.connect_timeout_seconds))
        .connect(&config.database.url)
        .await?;

    // 查询所有启用的规则
    let rows = sqlx::query_as::<_, RuleRow>(
        r#"
        SELECT r.id, r.rule_code, r.rule_json
        FROM badge_rules r
        WHERE r.enabled = TRUE
          AND (r.start_time IS NULL OR r.start_time <= NOW())
          AND (r.end_time IS NULL OR r.end_time > NOW())
        "#,
    )
    .fetch_all(&pool)
    .await?;

    let mut loaded_count = 0;
    for row in rows {
        // 将 rule_json 包装成完整的 Rule 结构
        let rule_id = row.id.to_string();
        let rule_name = row.rule_code.unwrap_or_else(|| format!("rule_{}", row.id));

        // 尝试解析 rule_json 为 RuleNode
        match serde_json::from_value::<RuleNode>(row.rule_json.clone()) {
            Ok(root) => {
                let rule = Rule {
                    id: rule_id.clone(),
                    name: rule_name,
                    version: "1.0".to_string(),
                    root,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                };

                if let Err(e) = store.load(rule) {
                    warn!(rule_id = %rule_id, error = %e, "Failed to load rule");
                } else {
                    loaded_count += 1;
                }
            }
            Err(e) => {
                warn!(
                    rule_id = %rule_id,
                    error = %e,
                    rule_json = %row.rule_json,
                    "Failed to parse rule_json"
                );
            }
        }
    }

    pool.close().await;
    Ok(loaded_count)
}

/// 数据库规则行
#[derive(sqlx::FromRow)]
struct RuleRow {
    id: i64,
    rule_code: Option<String>,
    rule_json: serde_json::Value,
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
