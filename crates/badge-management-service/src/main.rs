//! 徽章管理服务（C端）
//!
//! 提供徽章查询、兑换、展示等 C 端功能的 gRPC 服务入口。

use anyhow::Result;
use badge_proto::badge::badge_management_service_server::BadgeManagementServiceServer;
use badge_shared::{
    cache::Cache,
    config::AppConfig,
    database::Database,
    observability,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tonic::transport::Server;
use tracing::info;

use badge_management::{
    auto_benefit::{AutoBenefitConfig, AutoBenefitEvaluator, AutoBenefitRuleCache},
    benefit::BenefitService,
    cascade::{CascadeConfig, CascadeEvaluator},
    grpc::BadgeManagementServiceImpl,
    notification::{NotificationSender, NotificationService},
    repository::{
        AutoBenefitRepository, BadgeLedgerRepository, BadgeRepository, DependencyRepository,
        RedemptionRepository, UserBadgeRepository,
    },
    service::{BadgeQueryService, GrantService, RedemptionService, RevokeService},
};

/// 服务配置
///
/// 从 AppConfig 中提取服务启动所需的地址配置
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
    // 1. 统一加载配置：从 config/{service_name}.toml 加载，包含可观测性配置
    let config = AppConfig::load("badge-management-service").unwrap_or_else(|e| {
        tracing::warn!("Failed to load config, using defaults: {}", e);
        AppConfig::default()
    });

    // 2. 从 AppConfig 中提取可观测性配置并注入服务名
    let obs_config = config.observability.clone().with_service_name(&config.service_name);
    let _guard = observability::init(&obs_config).await?;

    info!("Starting badge-management-service...");
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

    let revoke_service = Arc::new(RevokeService::new(
        cache.clone(),
        pool.clone(),
        badge_repo.clone(),
    ));

    // 6.1 初始化通知服务
    let notification_service = Arc::new(NotificationService::with_defaults());
    let notification_sender = Arc::new(NotificationSender::new(notification_service.clone()));
    info!("Notification service initialized");

    // 6.2 初始化权益服务（默认 Handler + 数据库持久化 + Redis 分布式幂等）
    let benefit_service = Arc::new(
        BenefitService::with_defaults()
            .with_pool(pool.clone())
            .with_cache(cache.clone()),
    );
    info!("Benefit service initialized with default handlers, database persistence and Redis idempotency");

    let redemption_service = Arc::new(RedemptionService::with_benefit_service(
        redemption_repo.clone(),
        cache.clone(),
        pool.clone(),
        benefit_service.clone(),
    ));

    // 6.3 初始化自动权益评估器
    let auto_benefit_rule_cache = Arc::new(AutoBenefitRuleCache::new(pool.clone()));
    let auto_benefit_repo = Arc::new(AutoBenefitRepository::new(pool.clone()));
    let auto_benefit_evaluator = Arc::new(AutoBenefitEvaluator::new(
        AutoBenefitConfig::default(),
        auto_benefit_rule_cache.clone(),
        auto_benefit_repo,
        user_badge_repo.clone(),
    ));
    // 预热规则缓存
    if let Err(e) = auto_benefit_rule_cache.warmup().await {
        tracing::warn!("Auto benefit rule cache warmup failed: {}", e);
    }
    info!("Auto benefit evaluator initialized");

    // 7. 初始化级联评估器（解决循环依赖：CascadeEvaluator 需要 GrantService，GrantService 需要 CascadeEvaluator）
    let dependency_repo = Arc::new(DependencyRepository::new(pool.clone()));
    let cascade_config = CascadeConfig::default();
    let cascade_evaluator = Arc::new(CascadeEvaluator::new(
        cascade_config,
        dependency_repo,
        user_badge_repo.clone(),
    ));

    // 互相注入：打破循环依赖
    cascade_evaluator
        .set_grant_service(grant_service.clone())
        .await;
    grant_service
        .set_cascade_evaluator(cascade_evaluator.clone())
        .await;
    info!("Cascade evaluator initialized");

    // 注入自动权益评估器的依赖
    auto_benefit_evaluator
        .set_benefit_service(benefit_service.clone())
        .await;
    grant_service
        .set_auto_benefit_evaluator(auto_benefit_evaluator.clone())
        .await;
    info!("Auto benefit evaluator dependencies injected");

    // 设置通知发送器
    grant_service
        .set_notification_sender(notification_sender.clone())
        .await;
    revoke_service
        .set_notification_sender(notification_sender.clone())
        .await;
    redemption_service
        .set_notification_sender(notification_sender.clone())
        .await;
    info!("Notification senders configured");

    info!("Services initialized");

    // 7. 创建 gRPC 服务
    let grpc_service = BadgeManagementServiceImpl::new(
        query_service,
        grant_service,
        revoke_service,
        redemption_service,
        pool.clone(),
        Some(cascade_evaluator),
    )
    .with_auto_benefit_rule_cache(auto_benefit_rule_cache);

    // 8. 启动 gRPC 服务
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
        .add_service(BadgeManagementServiceServer::new(grpc_service))
        .serve_with_shutdown(service_config.grpc_addr, shutdown_signal())
        .await?;

    info!("Service shutdown complete");
    Ok(())
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
