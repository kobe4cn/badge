//! 共享库
//!
//! 包含所有服务共用的配置、错误处理、数据库连接、缓存、Kafka 等基础设施代码。

pub mod cache;
pub mod config;
pub mod database;
pub mod error;
pub mod kafka;
pub mod telemetry;
