//! 服务层
//!
//! 实现徽章业务逻辑，协调仓储层和缓存层。
//!
//! ## 模块结构
//!
//! - `dto`: 数据传输对象定义
//! - `query_service`: 徽章查询服务（只读操作）
//! - `grant_service`: 徽章发放服务（写入操作）
//! - `revoke_service`: 徽章取消服务（写入操作）
//! - `redemption_service`: 徽章兑换服务（写入操作）
//! - `competitive_redemption`: 竞争兑换服务（需要消耗徽章的兑换）

pub mod competitive_redemption;
pub mod dto;
pub mod grant_service;
pub mod query_service;
pub mod redemption_service;
pub mod revoke_service;

pub use competitive_redemption::{
    CompetitiveRedeemRequest, CompetitiveRedeemResponse, CompetitiveRedemptionService,
    ConsumedBadge,
};
pub use dto::*;
pub use grant_service::GrantService;
pub use query_service::BadgeQueryService;
pub use redemption_service::RedemptionService;
pub use revoke_service::RevokeService;
