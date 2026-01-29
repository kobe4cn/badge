//! 级联触发模块
//!
//! 当用户获得某徽章后，自动检查并触发依赖此徽章的其他徽章
//!
//! ## 核心组件
//!
//! - `CascadeEvaluator` - 级联评估器，负责评估和触发级联发放
//! - `BadgeGranter` - 徽章发放接口 trait，用于解耦循环依赖
//! - `DependencyGraph` - 依赖图，缓存徽章之间的依赖关系
//! - `CascadeConfig` - 级联配置（深度、超时等）
//! - `CascadeContext` - 评估上下文（深度、已访问徽章等）
//! - `CascadeResult` - 评估结果（已发放、已阻止的徽章）

mod dependency_graph;
mod dto;
mod evaluator;

pub use dependency_graph::DependencyGraph;
pub use dto::*;
pub use evaluator::{BadgeGranter, CascadeEvaluator};
