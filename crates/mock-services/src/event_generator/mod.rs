//! 事件生成器模块
//!
//! 提供用于模拟各类业务事件的生成器，支持购买、签到、页面浏览等事件类型。
//! 生成的事件符合 badge_shared::events 定义的格式，可直接用于测试和模拟场景。

mod batch;
mod engagement;
mod purchase;
mod traits;

pub use batch::{BatchEventSender, BatchSendResult};
pub use engagement::EngagementEventGenerator;
pub use purchase::PurchaseEventGenerator;
pub use traits::EventGenerator;

use badge_shared::events::{EventPayload, EventType};
use serde_json::json;
use uuid::Uuid;

/// 快速生成购买事件
///
/// 使用简化参数创建购买事件，适用于快速测试场景。
/// 默认使用 CNY 货币，包含单个商品项。
pub fn quick_purchase_event(user_id: &str, amount: f64) -> EventPayload {
    let order_id = format!("ORD-{}", Uuid::new_v4());
    let product_id = format!("PROD-{}", Uuid::new_v4());

    EventPayload::new(
        EventType::Purchase,
        user_id,
        json!({
            "order_id": order_id,
            "amount": amount,
            "currency": "CNY",
            "items": [{
                "product_id": product_id,
                "name": "测试商品",
                "quantity": 1,
                "price": amount,
                "category": "Electronics"
            }],
            "payment_method": "ALIPAY"
        }),
        "mock-services",
    )
}

/// 快速生成签到事件
///
/// 创建用户签到事件，记录连续签到天数。
/// 连续签到是常见的徽章触发条件，如「连续签到 7 天」。
pub fn quick_checkin_event(user_id: &str, consecutive_days: i32) -> EventPayload {
    EventPayload::new(
        EventType::CheckIn,
        user_id,
        json!({
            "location": "app",
            "consecutive_days": consecutive_days
        }),
        "mock-services",
    )
}

/// 快速生成退款事件
///
/// 创建退款事件，用于测试徽章回收逻辑。
/// 退款可能导致之前因购买获得的徽章被撤销。
pub fn quick_refund_event(
    user_id: &str,
    order_id: &str,
    amount: f64,
    badge_ids: &[i64],
) -> EventPayload {
    EventPayload::new(
        EventType::Refund,
        user_id,
        json!({
            "order_id": order_id,
            "refund_amount": amount,
            "currency": "CNY",
            "reason": "用户申请退款",
            "badge_ids_to_revoke": badge_ids
        }),
        "mock-services",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quick_purchase_event() {
        let event = quick_purchase_event("user-001", 199.99);

        assert_eq!(event.event_type, EventType::Purchase);
        assert_eq!(event.user_id, "user-001");
        assert_eq!(event.source, "mock-services");

        // 验证 data 字段
        let data = &event.data;
        assert!(data["order_id"].as_str().unwrap().starts_with("ORD-"));
        assert_eq!(data["amount"].as_f64().unwrap(), 199.99);
        assert_eq!(data["currency"].as_str().unwrap(), "CNY");
        assert!(data["items"].as_array().is_some());
        assert_eq!(data["payment_method"].as_str().unwrap(), "ALIPAY");
    }

    #[test]
    fn test_quick_checkin_event() {
        let event = quick_checkin_event("user-002", 7);

        assert_eq!(event.event_type, EventType::CheckIn);
        assert_eq!(event.user_id, "user-002");

        let data = &event.data;
        assert_eq!(data["consecutive_days"].as_i64().unwrap(), 7);
        assert_eq!(data["location"].as_str().unwrap(), "app");
    }

    #[test]
    fn test_quick_refund_event() {
        let badge_ids = vec![1, 2, 3];
        let event = quick_refund_event("user-003", "ORD-12345", 99.99, &badge_ids);

        assert_eq!(event.event_type, EventType::Refund);
        assert_eq!(event.user_id, "user-003");

        let data = &event.data;
        assert_eq!(data["order_id"].as_str().unwrap(), "ORD-12345");
        assert_eq!(data["refund_amount"].as_f64().unwrap(), 99.99);

        let revoke_ids: Vec<i64> = data["badge_ids_to_revoke"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_i64().unwrap())
            .collect();
        assert_eq!(revoke_ids, vec![1, 2, 3]);
    }
}
