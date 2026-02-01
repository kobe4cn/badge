//! Mock 服务模块
//!
//! 提供模拟的 REST API 服务实现，用于开发和测试环境。

pub mod benefit_service;
pub mod coupon_service;
pub mod notification_service;
pub mod order_service;
pub mod profile_service;

#[cfg(test)]
mod coupon_service_tests;

pub use benefit_service::{BenefitServiceState, benefit_routes};
pub use coupon_service::{CouponServiceState, coupon_routes};
pub use notification_service::{NotificationServiceState, notification_routes};
pub use order_service::{OrderServiceState, order_routes};
pub use profile_service::{ProfileServiceState, profile_routes};
