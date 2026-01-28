//! 订单事件处理服务
//!
//! 消费 Kafka 订单事件，处理复杂业务逻辑。

use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting event-transaction-service...");
    Ok(())
}
