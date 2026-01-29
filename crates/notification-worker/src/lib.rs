//! 通知工作者服务
//!
//! 从 Kafka 消费通知事件，通过多渠道发送器并行推送到用户端。
//! 各渠道独立发送，单个渠道失败不影响其他渠道的投递。

pub mod consumer;
pub mod error;
pub mod sender;
pub mod templates;
