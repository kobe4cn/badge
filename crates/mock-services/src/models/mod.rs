//! 模拟数据模型
//!
//! 包含订单、用户、优惠券等模拟数据结构，用于测试和开发环境。

pub mod coupon;
pub mod order;
pub mod user;

pub use coupon::{CouponStatus, CouponType, MockCoupon};
pub use order::{MockOrder, OrderItem, OrderStatus};
pub use user::{MembershipLevel, MockUser};
