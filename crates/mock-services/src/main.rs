//! Mock Services CLI
//!
//! 模拟服务的命令行入口点。
//! 提供服务启动、事件生成、场景测试、数据填充等功能。

use clap::Parser;
use mock_services::cli::{Cli, CommandRunner, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // 初始化 tracing 日志
    // 优先使用环境变量 RUST_LOG，否则使用命令行参数指定的级别
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| cli.log_level.clone().into()),
        )
        .init();

    let runner = CommandRunner::new(cli.kafka_brokers);

    match cli.command {
        Commands::Server {
            port,
            populate,
            user_count,
        } => {
            runner.run_server(port, populate, user_count).await?;
        }
        Commands::Generate {
            event_type,
            user_id,
            count,
            amount,
        } => {
            runner
                .run_generate(&event_type, &user_id, count, amount)
                .await?;
        }
        Commands::Scenario {
            name,
            user_id,
            file,
        } => {
            runner.run_scenario(&name, user_id, file).await?;
        }
        Commands::Populate {
            users,
            orders,
            output,
        } => {
            runner.run_populate(users, &orders, output).await?;
        }
    }

    Ok(())
}
