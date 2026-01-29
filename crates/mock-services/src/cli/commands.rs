//! CLI 命令定义
//!
//! 使用 clap derive 宏定义命令行接口结构。
//! 各子命令对应不同的功能模块：服务启动、事件生成、场景执行、数据填充。

use clap::{Parser, Subcommand};

/// Mock 服务命令行工具
///
/// 提供模拟服务的启动、事件生成、场景测试等功能。
/// 使用 `--help` 查看各子命令的详细说明。
#[derive(Parser, Debug)]
#[command(name = "mock-server")]
#[command(version, about = "徽章系统模拟服务工具")]
#[command(propagate_version = true)]
pub struct Cli {
    /// 日志级别 (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    pub log_level: String,

    /// Kafka brokers 地址
    #[arg(long, default_value = "localhost:9092")]
    pub kafka_brokers: String,

    #[command(subcommand)]
    pub command: Commands,
}

/// 子命令枚举
///
/// 每个变体对应一个独立的功能模块，通过子命令方式调用。
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// 启动 Mock 服务（HTTP REST API）
    ///
    /// 启动 Axum HTTP 服务器，提供订单、用户、优惠券等 REST API。
    /// 可选择预填充测试数据以便快速开始测试。
    Server {
        /// 服务端口
        #[arg(short, long, default_value = "8090")]
        port: u16,

        /// 是否预填充测试数据
        #[arg(long)]
        populate: bool,

        /// 预填充用户数量
        #[arg(long, default_value = "100")]
        user_count: usize,
    },

    /// 生成并发送事件到 Kafka
    ///
    /// 支持的事件类型：purchase, checkin, pageview, share, review, refund
    Generate {
        /// 事件类型
        #[arg(short, long)]
        event_type: String,

        /// 用户 ID
        #[arg(short, long)]
        user_id: String,

        /// 生成数量
        #[arg(short, long, default_value = "1")]
        count: usize,

        /// 购买金额（仅 purchase 事件）
        #[arg(long)]
        amount: Option<f64>,
    },

    /// 运行预定义场景
    ///
    /// 使用 `--name list` 列出所有可用场景。
    /// 场景可以从文件加载（JSON/YAML 格式）或使用预定义场景。
    Scenario {
        /// 场景名称（使用 "list" 列出所有场景）
        #[arg(short, long)]
        name: String,

        /// 用户 ID（覆盖场景中的默认值）
        #[arg(short, long)]
        user_id: Option<String>,

        /// 场景配置文件路径（JSON/YAML）
        #[arg(short, long)]
        file: Option<String>,
    },

    /// 批量生成测试数据
    ///
    /// 生成用户、订单、优惠券等测试数据。
    /// 可输出到文件供其他工具使用。
    Populate {
        /// 用户数量
        #[arg(short, long, default_value = "100")]
        users: usize,

        /// 每用户订单数量范围（格式：min-max）
        #[arg(long, default_value = "1-10")]
        orders: String,

        /// 输出到文件（JSON 格式）
        #[arg(short, long)]
        output: Option<String>,
    },
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_server() {
        // 测试默认参数
        let cli = Cli::parse_from(["mock-server", "server"]);
        match cli.command {
            Commands::Server {
                port,
                populate,
                user_count,
            } => {
                assert_eq!(port, 8090);
                assert!(!populate);
                assert_eq!(user_count, 100);
            }
            _ => panic!("预期 Server 命令"),
        }

        // 测试自定义参数
        let cli = Cli::parse_from([
            "mock-server",
            "server",
            "--port",
            "9000",
            "--populate",
            "--user-count",
            "50",
        ]);
        match cli.command {
            Commands::Server {
                port,
                populate,
                user_count,
            } => {
                assert_eq!(port, 9000);
                assert!(populate);
                assert_eq!(user_count, 50);
            }
            _ => panic!("预期 Server 命令"),
        }
    }

    #[test]
    fn test_cli_parse_generate() {
        // 测试基本参数
        let cli = Cli::parse_from([
            "mock-server",
            "generate",
            "--event-type",
            "purchase",
            "--user-id",
            "user-001",
        ]);
        match cli.command {
            Commands::Generate {
                event_type,
                user_id,
                count,
                amount,
            } => {
                assert_eq!(event_type, "purchase");
                assert_eq!(user_id, "user-001");
                assert_eq!(count, 1);
                assert!(amount.is_none());
            }
            _ => panic!("预期 Generate 命令"),
        }

        // 测试带金额参数
        let cli = Cli::parse_from([
            "mock-server",
            "generate",
            "-e",
            "purchase",
            "-u",
            "user-002",
            "-c",
            "5",
            "--amount",
            "99.99",
        ]);
        match cli.command {
            Commands::Generate {
                event_type,
                user_id,
                count,
                amount,
            } => {
                assert_eq!(event_type, "purchase");
                assert_eq!(user_id, "user-002");
                assert_eq!(count, 5);
                assert_eq!(amount, Some(99.99));
            }
            _ => panic!("预期 Generate 命令"),
        }
    }

    #[test]
    fn test_cli_parse_scenario() {
        // 测试列出场景
        let cli = Cli::parse_from(["mock-server", "scenario", "--name", "list"]);
        match cli.command {
            Commands::Scenario {
                name,
                user_id,
                file,
            } => {
                assert_eq!(name, "list");
                assert!(user_id.is_none());
                assert!(file.is_none());
            }
            _ => panic!("预期 Scenario 命令"),
        }

        // 测试带用户 ID 覆盖
        let cli = Cli::parse_from([
            "mock-server",
            "scenario",
            "-n",
            "first_purchase",
            "-u",
            "custom-user",
        ]);
        match cli.command {
            Commands::Scenario {
                name,
                user_id,
                file,
            } => {
                assert_eq!(name, "first_purchase");
                assert_eq!(user_id, Some("custom-user".to_string()));
                assert!(file.is_none());
            }
            _ => panic!("预期 Scenario 命令"),
        }

        // 测试从文件加载
        let cli = Cli::parse_from([
            "mock-server",
            "scenario",
            "-n",
            "custom",
            "-f",
            "/path/to/scenario.yaml",
        ]);
        match cli.command {
            Commands::Scenario { file, .. } => {
                assert_eq!(file, Some("/path/to/scenario.yaml".to_string()));
            }
            _ => panic!("预期 Scenario 命令"),
        }
    }

    #[test]
    fn test_cli_parse_populate() {
        // 测试默认参数
        let cli = Cli::parse_from(["mock-server", "populate"]);
        match cli.command {
            Commands::Populate {
                users,
                orders,
                output,
            } => {
                assert_eq!(users, 100);
                assert_eq!(orders, "1-10");
                assert!(output.is_none());
            }
            _ => panic!("预期 Populate 命令"),
        }

        // 测试自定义参数
        let cli = Cli::parse_from([
            "mock-server",
            "populate",
            "-u",
            "50",
            "--orders",
            "5-20",
            "-o",
            "data.json",
        ]);
        match cli.command {
            Commands::Populate {
                users,
                orders,
                output,
            } => {
                assert_eq!(users, 50);
                assert_eq!(orders, "5-20");
                assert_eq!(output, Some("data.json".to_string()));
            }
            _ => panic!("预期 Populate 命令"),
        }
    }

    #[test]
    fn test_cli_global_options() {
        // 测试全局选项
        let cli = Cli::parse_from([
            "mock-server",
            "--log-level",
            "debug",
            "--kafka-brokers",
            "kafka:9092",
            "server",
        ]);

        assert_eq!(cli.log_level, "debug");
        assert_eq!(cli.kafka_brokers, "kafka:9092");
    }
}
