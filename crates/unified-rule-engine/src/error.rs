//! 规则引擎错误类型

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuleError {
    #[error("规则解析失败: {0}")]
    ParseError(String),

    #[error("规则编译失败: {0}")]
    CompileError(String),

    #[error("规则执行失败: {0}")]
    ExecutionError(String),

    #[error("无效的操作符: {operator} 不支持类型 {value_type}")]
    InvalidOperator {
        operator: String,
        value_type: String,
    },

    #[error("字段不存在: {0}")]
    FieldNotFound(String),

    #[error("类型不匹配: 期望 {expected}, 实际 {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("规则未找到: {0}")]
    RuleNotFound(String),

    #[error("JSON 序列化错误: {0}")]
    JsonError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, RuleError>;
