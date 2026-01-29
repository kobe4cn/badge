//! 数据库仓储层
//!
//! 提供所有实体的数据访问接口，封装 SQL 操作细节。
//!
//! ## 设计原则
//!
//! - 仓储只负责数据持久化，不包含业务逻辑
//! - 使用 SQLx 进行类型安全的数据库操作
//! - 事务控制由调用方（服务层）决定
//! - 定义 trait 接口以支持 mock 测试

mod badge_repo;
mod dependency_repo;
mod ledger_repo;
mod redemption_repo;
mod traits;
mod user_badge_repo;

pub use badge_repo::BadgeRepository;
pub use dependency_repo::{
    BadgeDependencyRow, CascadeEvaluationLog, CreateDependencyRequest, DependencyRepository,
};
pub use ledger_repo::BadgeLedgerRepository;
pub use redemption_repo::RedemptionRepository;
pub use traits::*;
pub use user_badge_repo::UserBadgeRepository;
