//! 规则操作符定义

use serde::{Deserialize, Serialize};
use std::fmt;

/// 条件操作符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    // 通用比较
    Eq,
    Neq,

    // 数值比较
    Gt,
    Gte,
    Lt,
    Lte,
    Between,

    // 包含检查
    In,
    NotIn,
    Contains,
    ContainsAny,
    ContainsAll,

    // 字符串操作
    StartsWith,
    EndsWith,
    Regex,

    // 时间操作
    Before,
    After,

    // 空值检查
    IsEmpty,
    IsNotEmpty,
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Eq => "eq",
            Self::Neq => "neq",
            Self::Gt => "gt",
            Self::Gte => "gte",
            Self::Lt => "lt",
            Self::Lte => "lte",
            Self::Between => "between",
            Self::In => "in",
            Self::NotIn => "not_in",
            Self::Contains => "contains",
            Self::ContainsAny => "contains_any",
            Self::ContainsAll => "contains_all",
            Self::StartsWith => "starts_with",
            Self::EndsWith => "ends_with",
            Self::Regex => "regex",
            Self::Before => "before",
            Self::After => "after",
            Self::IsEmpty => "is_empty",
            Self::IsNotEmpty => "is_not_empty",
        };
        write!(f, "{}", s)
    }
}

/// 逻辑操作符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogicalOperator {
    And,
    Or,
}

impl fmt::Display for LogicalOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::And => write!(f, "AND"),
            Self::Or => write!(f, "OR"),
        }
    }
}
