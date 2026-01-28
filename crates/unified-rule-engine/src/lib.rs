//! 统一规则引擎
//!
//! 提供可复用的规则评估能力，支持：
//! - JSON 规则定义和解析
//! - 规则编译和缓存
//! - 短路求值执行
//! - gRPC 服务接口

pub mod error;
pub mod models;
pub mod operators;

pub use error::{Result, RuleError};
pub use models::{Condition, EvaluationContext, EvaluationResult, LogicalGroup, Rule, RuleNode};
pub use operators::{LogicalOperator, Operator};
