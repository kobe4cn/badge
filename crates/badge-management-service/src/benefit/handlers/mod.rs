//! 权益处理器实现
//!
//! 提供各种权益类型的具体 Handler 实现：
//!
//! - `CouponHandler`: 优惠券发放（同步）
//! - `PointsHandler`: 积分发放（同步）
//! - `PhysicalHandler`: 实物奖品发放（异步，通过 Kafka）
//!
//! ## 设计说明
//!
//! 每个 Handler 专注于单一权益类型的发放逻辑，通过实现 `BenefitHandler` trait
//! 提供统一接口。外部系统调用目前为 stub 实现，实际对接时需替换为真实的 SDK 调用。

mod coupon;
mod physical;
mod points;

pub use coupon::CouponHandler;
pub use physical::PhysicalHandler;
pub use points::PointsHandler;
