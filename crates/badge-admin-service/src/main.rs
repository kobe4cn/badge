//! 徽章管理后台服务（B端）
//!
//! 提供徽章配置、发放管理、统计报表等 REST API。

use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting badge-admin-service...");
    Ok(())
}
