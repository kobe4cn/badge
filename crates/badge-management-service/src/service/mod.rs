//! 服务层
//!
//! 实现徽章业务逻辑，协调仓储层和缓存层。
//!
//! ## 模块结构
//!
//! - `dto`: 数据传输对象定义
//! - `query_service`: 徽章查询服务（只读操作）

pub mod dto;
pub mod query_service;

pub use dto::*;
pub use query_service::BadgeQueryService;
