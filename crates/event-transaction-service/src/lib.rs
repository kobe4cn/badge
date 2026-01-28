//! 订单事件处理服务
//!
//! 消费 Kafka 中的交易类事件（Purchase、Refund、OrderCancel），
//! 经过幂等校验后触发规则引擎评估与徽章发放。
//! 与行为事件服务不同，本服务额外处理退款/取消场景下的徽章撤销逻辑。

pub mod consumer;
pub mod error;
pub mod processor;
pub mod rule_client;
pub mod rule_mapping;
