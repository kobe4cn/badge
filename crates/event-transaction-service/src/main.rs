//! 订单事件处理服务
//!
//! 消费 Kafka 订单事件（购买、退款、取消），处理徽章发放与退款撤销逻辑。

use std::sync::Arc;

use anyhow::Result;
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

    info!("Starting event-transaction-service...");

    let config = badge_shared::config::AppConfig::load("event-transaction")?;

    let cache = badge_shared::cache::Cache::new(&config.redis)?;

    let producer = badge_shared::kafka::KafkaProducer::new(&config.kafka)?;

    let rule_engine_url =
        std::env::var("RULE_ENGINE_URL").unwrap_or_else(|_| "http://localhost:50051".to_string());
    let badge_service_url =
        std::env::var("BADGE_SERVICE_URL").unwrap_or_else(|_| "http://localhost:50052".to_string());

    let rule_client = event_transaction_service::rule_client::TransactionRuleClient::new(
        &rule_engine_url,
        &badge_service_url,
    )
    .await?;

    let rule_mapping = Arc::new(event_transaction_service::rule_mapping::RuleBadgeMapping::new());

    let processor = event_transaction_service::processor::TransactionEventProcessor::new(
        cache,
        Arc::new(rule_client),
        rule_mapping,
    );

    let consumer = event_transaction_service::consumer::TransactionConsumer::new(
        &config, processor, producer,
    )?;

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

    info!("event-transaction-service 已关闭");
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
