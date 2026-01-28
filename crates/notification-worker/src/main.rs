//! 通知服务
//!
//! 处理多渠道通知发送（APP Push、SMS、微信、邮件等）。

use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting notification-worker...");
    Ok(())
}
