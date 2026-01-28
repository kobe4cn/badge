//! 通知服务
//!
//! 处理多渠道通知发送（APP Push、SMS、微信、邮件等）。

use std::collections::HashMap;
use std::sync::Arc;

use badge_shared::config::AppConfig;
use badge_shared::events::NotificationChannel;
use badge_shared::kafka::KafkaProducer;
use notification_worker::consumer::NotificationConsumer;
use notification_worker::sender::{
    AppPushSender, EmailSender, NotificationSender, SmsSender, WeChatSender,
};
use tokio::sync::watch;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting notification-worker...");

    let config = AppConfig::load("notification-worker")?;
    let producer = KafkaProducer::new(&config.kafka)?;

    // 注册所有渠道发送器
    let mut senders: HashMap<NotificationChannel, Arc<dyn NotificationSender>> = HashMap::new();
    senders.insert(NotificationChannel::AppPush, Arc::new(AppPushSender));
    senders.insert(NotificationChannel::Sms, Arc::new(SmsSender));
    senders.insert(NotificationChannel::WeChat, Arc::new(WeChatSender));
    senders.insert(NotificationChannel::Email, Arc::new(EmailSender));

    let consumer = NotificationConsumer::new(&config, senders, producer)?;

    // shutdown 信号用于优雅关闭消费循环
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    tokio::spawn(async move {
        if let Ok(()) = tokio::signal::ctrl_c().await {
            info!("收到 Ctrl+C 信号，开始优雅关闭...");
            let _ = shutdown_tx.send(true);
        }
    });

    consumer.run(shutdown_rx).await?;

    info!("notification-worker 已停止");
    Ok(())
}
