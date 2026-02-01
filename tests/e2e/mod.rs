//! Badge 系统端到端测试
//!
//! 测试覆盖完整的业务流程，包括：
//! - 基础配置（分类/系列/徽章）
//! - 规则配置（画布编辑器）
//! - 权益配置和发放
//! - 事件触发全链路
//! - 级联触发
//! - 兑换流程
//! - 通知系统
//! - 逆向场景
//! - 数据一致性

pub mod data;
pub mod helpers;
pub mod setup;
pub mod suites;

pub use setup::TestEnvironment;
