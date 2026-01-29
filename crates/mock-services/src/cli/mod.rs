//! CLI 模块
//!
//! 提供命令行接口，支持以下功能：
//!
//! - `server` - 启动 Mock HTTP 服务
//! - `generate` - 生成并发送事件到 Kafka
//! - `scenario` - 运行预定义或自定义场景
//! - `populate` - 批量生成测试数据
//!
//! # 使用示例
//!
//! ```bash
//! # 启动服务器
//! mock-server server --port 8090 --populate
//!
//! # 生成事件
//! mock-server generate -e purchase -u user-001 -c 5 --amount 99.99
//!
//! # 运行场景
//! mock-server scenario -n first_purchase
//!
//! # 批量生成数据
//! mock-server populate -u 50 --orders 1-10 -o data.json
//! ```

pub mod commands;
pub mod runner;

pub use commands::{Cli, Commands};
pub use runner::CommandRunner;
