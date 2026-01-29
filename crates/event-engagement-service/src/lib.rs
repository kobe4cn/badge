//! 行为事件处理服务
//!
//! 消费 Kafka 中的用户行为事件（签到、浏览、分享等），
//! 经过幂等校验后触发规则引擎评估与徽章发放。

pub mod consumer;
pub mod error;
pub mod processor;
pub mod rule_client;
pub mod rule_mapping;
