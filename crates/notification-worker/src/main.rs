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
use notification_worker::consumer::NotificationConsumer;
use notification_worker::sender::{
    AppPushSender, EmailSender, NotificationSender, SmsSender, WeChatSender,
};
use tokio::io::AsyncWriteExt;
use tokio::sync::watch;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Starting notification-worker...");

    let config = AppConfig::load("notification-worker")?;

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

    let health_port = config.server.port;
    let health_handle = tokio::spawn(start_health_server(health_port));

    let shutdown_handle = tokio::spawn(async move {
        shutdown_signal().await;
        info!("收到关闭信号，开始优雅关闭...");
        let _ = shutdown_tx.send(true);
    });

    consumer.run(shutdown_rx).await?;

    let _ = health_handle.await;
    let _ = shutdown_handle.await;

    info!("notification-worker 已关闭");
    Ok(())
}

/// 健康检查 HTTP 服务器
///
/// 提供 /health 和 /ready 端点供 Kubernetes 探测服务状态。
/// 使用原生 TCP 实现避免额外依赖，对于仅返回固定 JSON 的探针已足够。
async fn start_health_server(port: u16) {
    let listener = match tokio::net::TcpListener::bind(("0.0.0.0", port)).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(error = %e, port, "健康检查服务器绑定端口失败");
            return;
        }
    };

    info!(port, "健康检查 HTTP 服务器已启动");

    loop {
        let (mut stream, _) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                tracing::warn!(error = %e, "接受健康检查连接失败");
                continue;
            }
        };

        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let n = match tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await {
                Ok(n) => n,
                Err(_) => return,
            };

            let request = String::from_utf8_lossy(&buf[..n]);
            let is_health = request.contains("GET /health") || request.contains("GET /ready");

            let response = if is_health {
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"status\":\"ok\"}"
            } else {
                "HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\n\r\nNot Found"
            };

            let _ = stream.write_all(response.as_bytes()).await;
        });
    }
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
