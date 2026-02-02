//! 自动权益发放模块
//!
//! 实现"获得徽章时自动发放权益"的核心功能。
//!
//! ## 功能概述
//!
//! 当用户获得新徽章时，自动评估配置的权益规则，对满足条件的规则
//! 自动发放对应权益（如优惠券、积分等）。
//!
//! ## 模块结构
//!
//! - `dto`: 数据传输对象（配置、上下文、结果等）
//! - `rule_cache`: 规则缓存层，按触发徽章索引自动兑换规则
//!
//! ## 后续扩展
//!
//! - `evaluator`: 规则评估器
//! - `executor`: 权益发放执行器

pub mod dto;
pub mod rule_cache;

// 后续任务会添加更多模块
// pub mod evaluator;
// pub mod executor;
// #[cfg(test)]
// mod tests;

// Re-export commonly used types
pub use dto::{
    AutoBenefitConfig, AutoBenefitContext, AutoBenefitEvaluationLog, AutoBenefitGrant,
    AutoBenefitResult, AutoBenefitStatus, NewAutoBenefitGrant, SkipReason, SkippedRule,
};
pub use rule_cache::{create_shared_cache, AutoBenefitRuleCache, CachedRule};
