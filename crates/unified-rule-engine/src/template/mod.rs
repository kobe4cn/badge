//! 规则模板模块
//!
//! 提供规则模板的参数化支持，包括模板定义、参数验证和规则编译
//!
//! # 主要功能
//!
//! - `ParameterDef`: 模板参数定义，支持多种类型和验证规则
//! - `RuleTemplate`: 规则模板，包含可参数化的规则定义
//! - `TemplateCategory`: 模板分类，用于组织和过滤模板

pub mod models;

pub use models::*;
