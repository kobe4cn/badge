//! 模拟订单模型
//!
//! 用于测试和开发环境的订单数据结构，支持随机生成以模拟真实业务场景。

use chrono::{DateTime, Utc};
use fake::Fake;
use fake::faker::company::en::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 模拟订单
///
/// 包含完整的订单信息，用于模拟外部订单系统的数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockOrder {
    pub order_id: String,
    pub user_id: String,
    pub order_status: OrderStatus,
    pub total_amount: f64,
    pub currency: String,
    pub items: Vec<OrderItem>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 订单项
///
/// 表示订单中的单个商品
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub product_id: String,
    pub product_name: String,
    pub quantity: i32,
    pub unit_price: f64,
    pub category: String,
}

/// 订单状态
///
/// 模拟真实电商系统的订单生命周期
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    Paid,
    Shipped,
    Completed,
    Cancelled,
    Refunded,
}

impl MockOrder {
    /// 为指定用户生成随机订单
    ///
    /// 使用 fake crate 生成逼真的测试数据，包含 1-5 个随机商品
    pub fn random(user_id: &str) -> Self {
        let mut rng = rand::thread_rng();

        // 生成 1-5 个订单项
        let item_count = rng.gen_range(1..=5);
        let items: Vec<OrderItem> = (0..item_count).map(|_| OrderItem::random()).collect();

        // 计算订单总金额
        let total_amount: f64 = items
            .iter()
            .map(|item| item.unit_price * item.quantity as f64)
            .sum();

        let now = Utc::now();
        // 订单创建时间在过去 30 天内随机
        let days_ago = rng.gen_range(0..30);
        let created_at = now - chrono::Duration::days(days_ago);

        Self {
            order_id: format!("ORD-{}", Uuid::new_v4()),
            user_id: user_id.to_string(),
            order_status: OrderStatus::random(),
            total_amount,
            currency: "CNY".to_string(),
            items,
            created_at,
            updated_at: now,
        }
    }
}

impl OrderItem {
    /// 生成随机订单项
    fn random() -> Self {
        let mut rng = rand::thread_rng();

        // 预定义的商品类别，模拟真实电商场景
        let categories = [
            "Electronics",
            "Clothing",
            "Books",
            "Home & Garden",
            "Sports",
            "Beauty",
            "Food",
            "Toys",
        ];
        let category = categories[rng.gen_range(0..categories.len())];

        Self {
            product_id: format!("PROD-{}", Uuid::new_v4()),
            product_name: CatchPhrase().fake(),
            quantity: rng.gen_range(1..=5),
            unit_price: rng.gen_range(10.0..1000.0),
            category: category.to_string(),
        }
    }
}

impl OrderStatus {
    /// 随机生成订单状态
    ///
    /// 状态分布模拟真实业务场景：已完成订单占比最高
    fn random() -> Self {
        let mut rng = rand::thread_rng();
        // 权重分布: Completed 占 50%，其他状态各占约 10%
        match rng.gen_range(0..10) {
            0 => Self::Pending,
            1 => Self::Paid,
            2 => Self::Shipped,
            3..=7 => Self::Completed,
            8 => Self::Cancelled,
            _ => Self::Refunded,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_random() {
        let user_id = "user-123";
        let order = MockOrder::random(user_id);

        assert_eq!(order.user_id, user_id);
        assert!(order.order_id.starts_with("ORD-"));
        assert!(!order.items.is_empty());
        assert!(order.total_amount > 0.0);
        assert_eq!(order.currency, "CNY");
    }

    #[test]
    fn test_order_item_random() {
        let item = OrderItem::random();

        assert!(item.product_id.starts_with("PROD-"));
        assert!(!item.product_name.is_empty());
        assert!(item.quantity >= 1);
        assert!(item.unit_price > 0.0);
        assert!(!item.category.is_empty());
    }

    #[test]
    fn test_order_status_serialization() {
        let status = OrderStatus::Completed;
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: OrderStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, deserialized);
    }
}
