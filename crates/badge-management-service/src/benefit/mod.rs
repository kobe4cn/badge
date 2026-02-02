//! 权益处理模块
//!
//! 提供权益发放的统一抽象和多态实现支持。
//!
//! ## 模块结构
//!
//! - `dto`: 权益发放相关的数据传输对象
//! - `handler`: BenefitHandler trait 定义
//! - `handlers`: 各权益类型的具体 Handler 实现
//! - `registry`: Handler 注册表，按权益类型索引
//! - `service`: 权益服务层，封装发放、撤销、查询的业务逻辑
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
//! use badge_management::benefit::{BenefitService, GrantBenefitRequest};
//! use badge_management::benefit::registry::HandlerRegistry;
//! use badge_management::models::BenefitType;
//! use std::sync::Arc;
//!
//! // 创建服务（使用默认 Handler）
//! let service = BenefitService::with_defaults();
//!
//! // 发放权益
//! let request = GrantBenefitRequest::new(
//!     "user-123",
//!     BenefitType::Coupon,
//!     1,
//!     serde_json::json!({"coupon_template_id": "tpl-001"}),
//! );
//! let result = service.grant_benefit(request).await?;
//!
//! // 或者使用自定义注册表
//! let registry = Arc::new(HandlerRegistry::with_defaults());
//! let service = BenefitService::new(registry);
//! ```

pub mod dto;
pub mod handler;
pub mod handlers;
pub mod registry;
pub mod service;

// Re-export commonly used types from dto
pub use dto::{BenefitGrantRequest, BenefitGrantResult, BenefitRevokeResult};

// Re-export handler trait
pub use handler::BenefitHandler;

// Re-export handlers for convenience
pub use handlers::{CouponHandler, PhysicalHandler, PointsHandler};

// Re-export registry types
pub use registry::{HandlerRegistry, RegistryConfig};

// Re-export service types
pub use service::{BenefitService, GrantBenefitRequest, GrantBenefitResponse, RevokeResult};
