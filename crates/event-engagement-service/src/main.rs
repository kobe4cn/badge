//! 行为事件处理服务
//!
//! 消费 Kafka 行为事件，触发徽章发放。

use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting event-engagement-service...");
    Ok(())
}
