//! 共享库
//!
//! 包含所有服务共用的配置、错误处理、数据库连接、缓存、Kafka 等基础设施代码。

pub mod cache;
pub mod circuit_breaker;
pub mod config;
pub mod config_watcher;
pub mod database;
pub mod dlq;
pub mod error;
pub mod events;
pub mod grpc_tls;
pub mod kafka;
pub mod observability;
pub mod retry;
pub mod rules;
pub mod test_utils;
