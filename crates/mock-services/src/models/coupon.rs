//! 模拟优惠券模型
//!
//! 用于测试和开发环境的优惠券数据结构，支持随机生成以模拟真实业务场景。

use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 模拟优惠券
///
/// 包含优惠券的完整信息，支持百分比折扣、固定金额和免运费三种类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockCoupon {
    pub coupon_id: String,
    pub user_id: String,
    pub coupon_type: CouponType,
    pub discount_value: f64,
    pub min_order_amount: f64,
    pub status: CouponStatus,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// 优惠券类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CouponType {
    /// 折扣百分比（如 10% off）
    Percentage,
    /// 固定金额（如减 50 元）
    FixedAmount,
    /// 免运费
    FreeShipping,
}

/// 优惠券状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CouponStatus {
    /// 可用
    Active,
    /// 已使用
    Used,
    /// 已过期
    Expired,
}

impl MockCoupon {
    /// 为指定用户生成随机优惠券
    ///
    /// 根据优惠券类型自动设置合理的折扣值和最低消费门槛
    pub fn random(user_id: &str) -> Self {
        let mut rng = rand::thread_rng();

        let coupon_type = CouponType::random();
        let (discount_value, min_order_amount) = match coupon_type {
            // 百分比折扣：5%-30%，门槛 100-500 元
            CouponType::Percentage => {
                let discount = rng.gen_range(5.0..=30.0);
                let min_amount = rng.gen_range(100.0..=500.0);
                (discount, min_amount)
            }
            // 固定金额：10-100 元，门槛为折扣金额的 3-10 倍
            CouponType::FixedAmount => {
                let discount = rng.gen_range(10.0..=100.0);
                let min_amount = discount * rng.gen_range(3.0..=10.0);
                (discount, min_amount)
            }
            // 免运费：折扣值为典型运费，门槛 50-200 元
            CouponType::FreeShipping => {
                let shipping_cost = rng.gen_range(8.0..=25.0);
                let min_amount = rng.gen_range(50.0..=200.0);
                (shipping_cost, min_amount)
            }
        };

        let now = Utc::now();
        // 发放时间在过去 0-60 天内
        let days_ago = rng.gen_range(0..60);
        let issued_at = now - chrono::Duration::days(days_ago);

        // 有效期 30-90 天
        let validity_days = rng.gen_range(30..=90);
        let expires_at = issued_at + chrono::Duration::days(validity_days);

        // 根据过期时间和随机因素决定状态
        let status = if expires_at < now {
            CouponStatus::Expired
        } else {
            CouponStatus::random_active_status()
        };

        Self {
            coupon_id: format!("CPN-{}", Uuid::new_v4()),
            user_id: user_id.to_string(),
            coupon_type,
            discount_value,
            min_order_amount,
            status,
            issued_at,
            expires_at,
        }
    }

    /// 判断优惠券是否可用
    pub fn is_usable(&self) -> bool {
        self.status == CouponStatus::Active && self.expires_at > Utc::now()
    }

    /// 计算折扣金额
    ///
    /// 根据订单金额和优惠券类型计算实际折扣
    pub fn calculate_discount(&self, order_amount: f64) -> f64 {
        if order_amount < self.min_order_amount {
            return 0.0;
        }

        match self.coupon_type {
            CouponType::Percentage => order_amount * (self.discount_value / 100.0),
            CouponType::FixedAmount => self.discount_value,
            CouponType::FreeShipping => self.discount_value,
        }
    }
}

impl CouponType {
    /// 随机生成优惠券类型
    ///
    /// 百分比折扣最常见，免运费相对较少
    fn random() -> Self {
        let mut rng = rand::thread_rng();
        match rng.gen_range(0..10) {
            0..=4 => Self::Percentage,
            5..=7 => Self::FixedAmount,
            _ => Self::FreeShipping,
        }
    }
}

impl CouponStatus {
    /// 随机生成未过期优惠券的状态
    ///
    /// 大部分优惠券处于可用状态，少部分已使用
    fn random_active_status() -> Self {
        let mut rng = rand::thread_rng();
        if rng.gen_bool(0.7) {
            Self::Active
        } else {
            Self::Used
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coupon_random() {
        let user_id = "user-456";
        let coupon = MockCoupon::random(user_id);

        assert_eq!(coupon.user_id, user_id);
        assert!(coupon.coupon_id.starts_with("CPN-"));
        assert!(coupon.discount_value > 0.0);
        assert!(coupon.min_order_amount > 0.0);
        assert!(coupon.expires_at > coupon.issued_at);
    }

    #[test]
    fn test_coupon_calculate_discount_percentage() {
        let coupon = MockCoupon {
            coupon_id: "test".to_string(),
            user_id: "user".to_string(),
            coupon_type: CouponType::Percentage,
            discount_value: 10.0, // 10%
            min_order_amount: 100.0,
            status: CouponStatus::Active,
            issued_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::days(30),
        };

        // 未达到门槛
        assert_eq!(coupon.calculate_discount(50.0), 0.0);
        // 达到门槛，10% 折扣
        assert!((coupon.calculate_discount(200.0) - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_coupon_calculate_discount_fixed() {
        let coupon = MockCoupon {
            coupon_id: "test".to_string(),
            user_id: "user".to_string(),
            coupon_type: CouponType::FixedAmount,
            discount_value: 50.0, // 减 50 元
            min_order_amount: 200.0,
            status: CouponStatus::Active,
            issued_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::days(30),
        };

        assert_eq!(coupon.calculate_discount(100.0), 0.0);
        assert!((coupon.calculate_discount(300.0) - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_coupon_status_serialization() {
        let status = CouponStatus::Active;
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: CouponStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, deserialized);
    }

    #[test]
    fn test_coupon_type_serialization() {
        let coupon_type = CouponType::Percentage;
        let json = serde_json::to_string(&coupon_type).unwrap();
        let deserialized: CouponType = serde_json::from_str(&json).unwrap();
        assert_eq!(coupon_type, deserialized);
    }
}
