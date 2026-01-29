//! 购买事件生成器
//!
//! 生成模拟购买事件，支持配置金额范围、商品类别等参数。

use badge_shared::events::{EventPayload, EventType};
use fake::Fake;
use fake::faker::company::en::CatchPhrase;
use rand::Rng;
use serde_json::json;
use uuid::Uuid;

use super::traits::EventGenerator;

/// 购买事件生成器
///
/// 生成符合业务规范的购买事件数据，包括订单信息、商品列表和支付方式。
/// 可通过配置调整金额范围和商品类别，以模拟不同业务场景。
pub struct PurchaseEventGenerator {
    /// 最小订单金额
    pub min_amount: f64,
    /// 最大订单金额
    pub max_amount: f64,
    /// 可选的商品类别列表
    pub categories: Vec<String>,
}

impl Default for PurchaseEventGenerator {
    /// 提供合理的默认配置
    ///
    /// 默认金额范围 10-5000 元，覆盖常见消费场景。
    /// 类别包含电子、服装、图书等常见电商品类。
    fn default() -> Self {
        Self {
            min_amount: 10.0,
            max_amount: 5000.0,
            categories: vec![
                "Electronics".to_string(),
                "Clothing".to_string(),
                "Books".to_string(),
                "Home & Garden".to_string(),
                "Sports".to_string(),
                "Beauty".to_string(),
                "Food".to_string(),
                "Toys".to_string(),
            ],
        }
    }
}

impl PurchaseEventGenerator {
    /// 创建自定义配置的生成器
    pub fn new(min_amount: f64, max_amount: f64, categories: Vec<String>) -> Self {
        Self {
            min_amount,
            max_amount,
            categories,
        }
    }

    /// 生成随机商品项
    fn generate_item(&self, rng: &mut impl Rng) -> serde_json::Value {
        let category = if self.categories.is_empty() {
            "General".to_string()
        } else {
            self.categories[rng.gen_range(0..self.categories.len())].clone()
        };

        let quantity = rng.gen_range(1..=5);
        let price = rng.gen_range(self.min_amount / 2.0..self.max_amount / 2.0);

        json!({
            "product_id": format!("PROD-{}", Uuid::new_v4()),
            "name": CatchPhrase().fake::<String>(),
            "quantity": quantity,
            "price": (price * 100.0).round() / 100.0,
            "category": category
        })
    }

    /// 随机选择支付方式
    fn random_payment_method(rng: &mut impl Rng) -> &'static str {
        const METHODS: [&str; 4] = ["ALIPAY", "WECHAT_PAY", "CREDIT_CARD", "DEBIT_CARD"];
        METHODS[rng.gen_range(0..METHODS.len())]
    }
}

impl EventGenerator for PurchaseEventGenerator {
    fn generate(&self, user_id: &str) -> EventPayload {
        let mut rng = rand::thread_rng();

        // 生成 1-5 个商品项
        let item_count = rng.gen_range(1..=5);
        let items: Vec<serde_json::Value> = (0..item_count)
            .map(|_| self.generate_item(&mut rng))
            .collect();

        // 计算总金额（各商品的 price * quantity 之和）
        let total_amount: f64 = items
            .iter()
            .map(|item| {
                let price = item["price"].as_f64().unwrap_or(0.0);
                let quantity = item["quantity"].as_i64().unwrap_or(1) as f64;
                price * quantity
            })
            .sum();

        // 保留两位小数
        let total_amount = (total_amount * 100.0).round() / 100.0;

        EventPayload::new(
            EventType::Purchase,
            user_id,
            json!({
                "order_id": format!("ORD-{}", Uuid::new_v4()),
                "amount": total_amount,
                "currency": "CNY",
                "items": items,
                "payment_method": Self::random_payment_method(&mut rng)
            }),
            "mock-services",
        )
    }

    fn event_type(&self) -> EventType {
        EventType::Purchase
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_purchase_event_generator() {
        let generator = PurchaseEventGenerator::default();
        let event = generator.generate("user-001");

        assert_eq!(event.event_type, EventType::Purchase);
        assert_eq!(event.user_id, "user-001");
        assert_eq!(event.source, "mock-services");

        // 验证必要字段存在
        let data = &event.data;
        assert!(data["order_id"].as_str().unwrap().starts_with("ORD-"));
        assert!(data["amount"].as_f64().is_some());
        assert_eq!(data["currency"].as_str().unwrap(), "CNY");
        assert!(data["items"].as_array().is_some());
        assert!(data["payment_method"].as_str().is_some());

        // 验证商品列表非空
        let items = data["items"].as_array().unwrap();
        assert!(!items.is_empty());
        assert!(items.len() <= 5);
    }

    #[test]
    fn test_purchase_event_generator_custom() {
        let generator =
            PurchaseEventGenerator::new(100.0, 200.0, vec!["CustomCategory".to_string()]);
        let event = generator.generate("user-002");

        // 验证使用了自定义类别
        let items = event.data["items"].as_array().unwrap();
        for item in items {
            assert_eq!(item["category"].as_str().unwrap(), "CustomCategory");
        }
    }

    #[test]
    fn test_purchase_event_generator_batch() {
        let generator = PurchaseEventGenerator::default();
        let events = generator.generate_batch("user-003", 10);

        assert_eq!(events.len(), 10);

        // 验证每个事件都有唯一的 order_id
        let order_ids: Vec<&str> = events
            .iter()
            .map(|e| e.data["order_id"].as_str().unwrap())
            .collect();

        let unique_ids: std::collections::HashSet<&str> = order_ids.iter().copied().collect();
        assert_eq!(unique_ids.len(), 10);
    }

    #[test]
    fn test_purchase_event_type() {
        let generator = PurchaseEventGenerator::default();
        assert_eq!(generator.event_type(), EventType::Purchase);
    }

    #[test]
    fn test_payment_methods() {
        let generator = PurchaseEventGenerator::default();
        let valid_methods = ["ALIPAY", "WECHAT_PAY", "CREDIT_CARD", "DEBIT_CARD"];

        // 多次生成以覆盖不同支付方式
        for _ in 0..20 {
            let event = generator.generate("user-test");
            let method = event.data["payment_method"].as_str().unwrap();
            assert!(
                valid_methods.contains(&method),
                "未知的支付方式: {}",
                method
            );
        }
    }
}
