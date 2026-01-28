//! 统一规则引擎服务
//!
//! 提供规则解析、编译、执行能力，支持复杂条件组合和嵌套逻辑。

use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting unified-rule-engine...");
    Ok(())
}
