//! 行为事件处理服务
//!
//! 消费 Kafka 行为事件（签到、浏览、分享等），触发规则引擎评估与徽章发放。

use std::sync::Arc;

use anyhow::Result;
use tokio::io::AsyncWriteExt;
use tokio::sync::watch;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // 使用环境变量控制日志级别，默认 info
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Starting event-engagement-service...");

    // 配置加载失败应立即终止，而非带着错误配置运行
    let config = badge_shared::config::AppConfig::load("event-engagement")?;

    let cache = badge_shared::cache::Cache::new(&config.redis)?;

    let producer = badge_shared::kafka::KafkaProducer::new(&config.kafka)?;

    // gRPC 客户端在启动阶段即建立连接，快速暴露网络配置问题
    let rule_engine_url =
        std::env::var("RULE_ENGINE_URL").unwrap_or_else(|_| "http://localhost:50051".to_string());
    let badge_service_url =
        std::env::var("BADGE_SERVICE_URL").unwrap_or_else(|_| "http://localhost:50052".to_string());

    let rule_client = event_engagement_service::rule_client::BadgeRuleClient::new(
        &rule_engine_url,
        &badge_service_url,
    )
    .await?;

    // 启动时为空映射，生产环境应从管理服务动态加载
    let rule_mapping = Arc::new(event_engagement_service::rule_mapping::RuleBadgeMapping::new());

    let processor = event_engagement_service::processor::EngagementEventProcessor::new(
        cache,
        Arc::new(rule_client),
        rule_mapping,
    );

    let consumer =
        event_engagement_service::consumer::EngagementConsumer::new(&config, processor, producer)?;

    // watch channel 实现优雅关闭：发送端置 true 后消费循环自行退出
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

    info!("event-engagement-service 已关闭");
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
            // 读取请求（仅需判断路径，不做完整解析）
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
