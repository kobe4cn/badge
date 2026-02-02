//! 行为事件处理服务
//!
//! 消费 Kafka 行为事件（签到、浏览、分享等），触发规则引擎评估与徽章发放。

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::watch;
use tracing::info;

use badge_shared::config::AppConfig;
use badge_shared::database::Database;
use badge_shared::observability;
use badge_shared::rules::{RuleBadgeMapping, RuleLoader, RuleValidator};

#[tokio::main]
async fn main() -> Result<()> {
    // 统一加载配置：从 config/{service_name}.toml 加载，包含可观测性配置
    let config = AppConfig::load("event-engagement-service")?;

    // 从 AppConfig 中提取可观测性配置并注入服务名
    let obs_config = config.observability.clone().with_service_name(&config.service_name);
    let _guard = observability::init(&obs_config).await?;

    info!("Starting event-engagement-service...");

    // 初始化数据库连接
    let db = Database::connect(&config.database).await?;
    let db_pool = db.pool().clone();

    let cache = badge_shared::cache::Cache::new(&config.redis)?;

    let producer = badge_shared::kafka::KafkaProducer::new(&config.kafka)?;

    // gRPC 客户端使用懒连接模式，允许服务独立启动
    let rule_engine_url =
        std::env::var("RULE_ENGINE_URL").unwrap_or_else(|_| "http://localhost:50051".to_string());
    let badge_service_url =
        std::env::var("BADGE_SERVICE_URL").unwrap_or_else(|_| "http://localhost:50052".to_string());

    let rule_client = event_engagement_service::rule_client::BadgeRuleClient::new(
        &rule_engine_url,
        &badge_service_url,
    )?;

    // 初始化规则组件：RuleBadgeMapping 作为内存缓存存储从数据库加载的规则
    let rule_mapping = Arc::new(RuleBadgeMapping::new());

    // RuleLoader 负责从数据库加载规则并维护到 rule_mapping
    let rule_loader = Arc::new(RuleLoader::new(
        db_pool.clone(),
        "engagement",
        rule_mapping.clone(),
        config.rules.refresh_interval_secs,
        config.rules.initial_load_timeout_secs,
    ));

    // RuleValidator 在发放前校验规则的时间窗口、用户限额、全局配额等条件
    let rule_validator = Arc::new(RuleValidator::new(cache.clone(), db_pool.clone()));

    // watch channel 实现优雅关闭：发送端置 true 后消费循环自行退出
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // 初始加载规则（阻塞，失败则终止启动）
    rule_loader.initial_load().await?;

    // 启动后台刷新任务
    rule_loader
        .clone()
        .start_background_refresh(shutdown_rx.clone());

    let processor = event_engagement_service::processor::EngagementEventProcessor::new(
        cache,
        Arc::new(rule_client),
        rule_mapping,
        rule_validator,
    );

    let consumer = event_engagement_service::consumer::EngagementConsumer::new(
        &config,
        processor,
        producer,
        rule_loader.clone(),
    )?;

    // 健康检查端点已由 observability 模块在 metrics_port 上提供
    let shutdown_handle = tokio::spawn(async move {
        shutdown_signal().await;
        info!("收到关闭信号，开始优雅关闭...");
        let _ = shutdown_tx.send(true);
    });

    consumer.run(shutdown_rx).await?;

    let _ = shutdown_handle.await;

    info!("event-engagement-service 已关闭");
    Ok(())
}

/// 监听操作系统关闭信号
///
/// 同时监听 SIGINT（Ctrl+C）和 SIGTERM（容器编排发送），
/// 任一信号到达即触发优雅关闭流程。
async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("注册 SIGTERM 信号失败");
        tokio::select! {
            _ = ctrl_c => {}
            _ = sigterm.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        ctrl_c.await.ok();
    }
}
