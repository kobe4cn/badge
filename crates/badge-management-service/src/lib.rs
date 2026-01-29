//! 徽章管理服务（C端）
//!
//! 提供徽章查询、兑换、展示等 C 端功能。
//!
//! ## 核心功能
//!
//! - **徽章查询**：按分类、系列查询徽章，查询用户持有徽章
//! - **徽章发放**：根据规则引擎结果发放徽章给用户
//! - **徽章兑换**：用徽章兑换权益（优惠券、数字资产等）
//! - **账本记录**：记录徽章的每一次变动，支持审计追溯
//!
//! ## 模块结构
//!
//! - `models`: 领域模型定义
//! - `error`: 错误类型定义
//! - `repository`: 数据库仓储层
//! - `service`: 业务服务层
//! - `grpc`: gRPC 服务端实现

pub mod error;
pub mod grpc;
pub mod models;
pub mod repository;
pub mod service;

pub use error::{BadgeError, Result};
pub use grpc::BadgeManagementServiceImpl;
pub use models::*;
pub use repository::{
    BadgeLedgerRepository, BadgeRepository, RedemptionRepository, UserBadgeRepository,
};
pub use service::{BadgeQueryService, GrantService, RedemptionService, RevokeService, dto};
