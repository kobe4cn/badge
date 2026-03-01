//! 测试套件模块
//!
//! 按业务功能组织的测试用例集合。
//! `_pipeline_warmup` 模块通过下划线前缀确保最先执行，预热 Kafka 消费管道。

pub mod _pipeline_warmup;
pub mod basic_config;
pub mod benefit_config;
pub mod cascade_trigger;
pub mod data_consistency;
pub mod deep_nesting;
pub mod event_trigger;
pub mod notification;
pub mod redemption;
pub mod reverse_flow;
pub mod rule_config;
