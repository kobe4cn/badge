//! 通知工作者服务
//!
//! 从 Kafka 消费通知事件，通过多渠道发送器（APP Push、SMS、微信、邮件）
//! 并行推送到用户端。

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use badge_shared::config::AppConfig;
use badge_shared::events::NotificationChannel;
use badge_shared::kafka::KafkaProducer;
use badge_shared::observability;
use notification_worker::consumer::NotificationConsumer;
use notification_worker::sender::{
    AppPushSender, EmailSender, NotificationSender, SmsSender, WeChatSender,
};
use tokio::sync::watch;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // 统一加载配置：从 config/{service_name}.toml 加载，包含可观测性配置
    let config = AppConfig::load("notification-worker")?;

    // 从 AppConfig 中提取可观测性配置并注入服务名
    let obs_config = config.observability.clone().with_service_name(&config.service_name);
    let _guard = observability::init(&obs_config).await?;

    info!("Starting notification-worker...");

    let producer = KafkaProducer::new(&config.kafka)?;

    // 注册所有渠道发送器，每个渠道独立实现 NotificationSender trait
    let senders: HashMap<NotificationChannel, Arc<dyn NotificationSender>> = HashMap::from([
        (
            NotificationChannel::AppPush,
            Arc::new(AppPushSender) as Arc<dyn NotificationSender>,
        ),
        (
            NotificationChannel::Sms,
            Arc::new(SmsSender) as Arc<dyn NotificationSender>,
        ),
        (
            NotificationChannel::WeChat,
            Arc::new(WeChatSender) as Arc<dyn NotificationSender>,
        ),
        (
            NotificationChannel::Email,
            Arc::new(EmailSender) as Arc<dyn NotificationSender>,
        ),
    ]);

    let consumer = NotificationConsumer::new(&config, senders, producer)?;

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // 健康检查端点已由 observability 模块在 metrics_port 上提供
    let shutdown_handle = tokio::spawn(async move {
        shutdown_signal().await;
        info!("收到关闭信号，开始优雅关闭...");
        let _ = shutdown_tx.send(true);
    });

    consumer.run(shutdown_rx).await?;

    let _ = shutdown_handle.await;

    info!("notification-worker 已关闭");
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
