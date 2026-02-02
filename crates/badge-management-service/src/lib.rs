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
//! - **级联触发**：当用户获得某徽章后，自动检查并触发依赖此徽章的其他徽章
//! - **自动权益**：获得徽章时自动发放关联权益
//! - **通知发送**：徽章发放/兑换后的多渠道通知
//!
//! ## 模块结构
//!
//! - `models`: 领域模型定义
//! - `error`: 错误类型定义
//! - `repository`: 数据库仓储层
//! - `service`: 业务服务层
//! - `grpc`: gRPC 服务端实现
//! - `cascade`: 级联触发模块
//! - `lock`: 分布式锁模块
//! - `benefit`: 权益处理模块
//! - `auto_benefit`: 自动权益发放模块
//! - `notification`: 通知服务模块

pub mod auto_benefit;
pub mod benefit;
pub mod cascade;
pub mod error;
pub mod grpc;
pub mod lock;
pub mod models;
pub mod notification;
pub mod repository;
pub mod service;

pub use auto_benefit::{
    AutoBenefitConfig, AutoBenefitContext, AutoBenefitGrant, AutoBenefitResult, AutoBenefitStatus,
    NewAutoBenefitGrant, SkipReason, SkippedRule,
};
pub use benefit::{
    BenefitGrantRequest, BenefitGrantResult, BenefitHandler, BenefitRevokeResult, BenefitService,
    GrantBenefitRequest, GrantBenefitResponse, HandlerRegistry,
};
pub use error::{BadgeError, Result};
pub use grpc::BadgeManagementServiceImpl;
pub use lock::{LockConfig, LockGuard, LockManager};
pub use models::*;
pub use notification::{
    Notification, NotificationBuilder, NotificationChannel, NotificationResult,
    NotificationSender, NotificationService, TemplateEngine,
};
pub use repository::{
    AutoBenefitRepository, BadgeLedgerRepository, BadgeRepository, RedemptionRepository,
    UserBadgeRepository,
};
pub use service::{BadgeQueryService, GrantService, RedemptionService, RevokeService, dto};
