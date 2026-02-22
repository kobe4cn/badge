//! 徽章服务领域模型
//!
//! 包含徽章系统的所有核心实体定义

pub mod badge;
pub mod enums;
pub mod redemption;
pub mod user_badge;

// 重新导出常用类型
pub use badge::{Badge, BadgeAssets, BadgeCategory, BadgeRule, BadgeSeries, ValidityConfig};
pub use enums::{
    BadgeStatus, BadgeType, BenefitType, CategoryStatus, ChangeType, GrantStatus, LogAction,
    OrderStatus, RecipientType, RedemptionValidityType, RevokeReason, SourceType,
    UserBadgeStatus, ValidityType,
};
pub use redemption::{
    BadgeRedemptionRule, Benefit, BenefitInfo, BenefitStatus, FrequencyConfig, RedemptionDetail,
    RedemptionOrder, RedemptionRequest, RedemptionResult, RequiredBadge,
};
pub use user_badge::{BadgeLedger, UserBadge, UserBadgeLog, UserBadgeSummary};
