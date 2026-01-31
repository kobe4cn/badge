//! 权益处理模块
//!
//! 提供权益发放的统一抽象和多态实现支持。
//!
//! ## 模块结构
//!
//! - `dto`: 权益发放相关的数据传输对象
//! - `handler`: BenefitHandler trait 定义
//! - `handlers`: 各权益类型的具体 Handler 实现
//!
//! ## 设计说明
//!
//! 权益发放采用策略模式，通过 `BenefitHandler` trait 定义统一接口，
//! 不同权益类型（优惠券、积分、数字资产等）实现各自的 Handler。
//! Handler 注册表负责管理所有 Handler 实例，根据权益类型路由到对应实现。
//!
//! ## 使用示例
//!
//! ```ignore
//! use badge_management::benefit::{BenefitHandler, BenefitGrantRequest, BenefitGrantResult};
//! use badge_management::benefit::handlers::CouponHandler;
//!
//! // 创建处理器
//! let handler = CouponHandler::new("http://coupon-service:8080");
//!
//! // 构造发放请求
//! let request = BenefitGrantRequest::new(
//!     "grant-001",
//!     "user-123",
//!     benefit_id,
//!     config,
//! );
//!
//! // 执行发放
//! let result = handler.grant(request).await?;
//! ```

pub mod dto;
pub mod handler;
pub mod handlers;

// Re-export commonly used types
pub use dto::{BenefitGrantRequest, BenefitGrantResult, BenefitRevokeResult};
pub use handler::BenefitHandler;

// Re-export handlers for convenience
pub use handlers::{CouponHandler, PhysicalHandler, PointsHandler};
