//! 徽章管理服务（C端）
//!
//! 提供徽章查询、兑换、展示等 C 端功能。

use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting badge-management-service...");
    Ok(())
}
