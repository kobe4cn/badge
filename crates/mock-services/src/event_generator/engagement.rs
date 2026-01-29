//! 行为事件生成器
//!
//! 生成用户行为类事件，包括签到、页面浏览、分享、评价等。
//! 不同行为类型会生成不同结构的 data 字段。

use badge_shared::events::{EventPayload, EventType};
use fake::Fake;
use fake::faker::lorem::en::{Paragraph, Sentence};
use rand::Rng;
use serde_json::json;
use uuid::Uuid;

use super::traits::EventGenerator;

/// 行为事件生成器
///
/// 根据配置的事件类型生成对应的行为事件数据。
/// 支持 CheckIn、PageView、Share、Review 四种行为类型。
pub struct EngagementEventGenerator {
    /// 要生成的行为事件类型
    pub event_type: EventType,
}

impl EngagementEventGenerator {
    /// 创建签到事件生成器
    pub fn checkin() -> Self {
        Self {
            event_type: EventType::CheckIn,
        }
    }

    /// 创建页面浏览事件生成器
    pub fn page_view() -> Self {
        Self {
            event_type: EventType::PageView,
        }
    }

    /// 创建分享事件生成器
    pub fn share() -> Self {
        Self {
            event_type: EventType::Share,
        }
    }

    /// 创建评价事件生成器
    pub fn review() -> Self {
        Self {
            event_type: EventType::Review,
        }
    }

    /// 生成签到事件数据
    fn generate_checkin_data(rng: &mut impl Rng) -> serde_json::Value {
        const LOCATIONS: [&str; 4] = ["app", "mini_program", "h5", "store"];

        json!({
            "location": LOCATIONS[rng.gen_range(0..LOCATIONS.len())],
            "consecutive_days": rng.gen_range(1..=365)
        })
    }

    /// 生成页面浏览事件数据
    fn generate_page_view_data(rng: &mut impl Rng) -> serde_json::Value {
        const PAGES: [&str; 6] = [
            "/home",
            "/product/detail",
            "/cart",
            "/order/list",
            "/user/profile",
            "/promotion/activity",
        ];

        const REFERRERS: [&str; 4] = ["direct", "search", "social", "email"];

        json!({
            "page_url": format!("https://shop.example.com{}", PAGES[rng.gen_range(0..PAGES.len())]),
            "duration_seconds": rng.gen_range(1..=600),
            "referrer": REFERRERS[rng.gen_range(0..REFERRERS.len())]
        })
    }

    /// 生成分享事件数据
    fn generate_share_data(rng: &mut impl Rng) -> serde_json::Value {
        const PLATFORMS: [&str; 5] = ["wechat", "weibo", "qq", "douyin", "xiaohongshu"];
        const CONTENT_TYPES: [&str; 3] = ["product", "article", "promotion"];

        json!({
            "platform": PLATFORMS[rng.gen_range(0..PLATFORMS.len())],
            "content_type": CONTENT_TYPES[rng.gen_range(0..CONTENT_TYPES.len())],
            "content_id": format!("CNT-{}", Uuid::new_v4())
        })
    }

    /// 生成评价事件数据
    fn generate_review_data(rng: &mut impl Rng) -> serde_json::Value {
        // 评价内容使用 fake crate 生成
        let content: String = if rng.gen_bool(0.7) {
            Paragraph(1..3).fake()
        } else {
            Sentence(1..2).fake()
        };

        json!({
            "product_id": format!("PROD-{}", Uuid::new_v4()),
            "rating": rng.gen_range(1..=5),
            "content": content
        })
    }
}

impl EventGenerator for EngagementEventGenerator {
    fn generate(&self, user_id: &str) -> EventPayload {
        let mut rng = rand::thread_rng();

        let data = match self.event_type {
            EventType::CheckIn => Self::generate_checkin_data(&mut rng),
            EventType::PageView => Self::generate_page_view_data(&mut rng),
            EventType::Share => Self::generate_share_data(&mut rng),
            EventType::Review => Self::generate_review_data(&mut rng),
            // 对于不支持的类型，生成空数据
            _ => json!({}),
        };

        EventPayload::new(self.event_type.clone(), user_id, data, "mock-services")
    }

    fn event_type(&self) -> EventType {
        self.event_type.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engagement_event_generator_checkin() {
        let generator = EngagementEventGenerator::checkin();
        let event = generator.generate("user-001");

        assert_eq!(event.event_type, EventType::CheckIn);
        assert_eq!(event.user_id, "user-001");

        let data = &event.data;
        assert!(data["location"].as_str().is_some());
        assert!(data["consecutive_days"].as_i64().is_some());

        let days = data["consecutive_days"].as_i64().unwrap();
        assert!(days >= 1 && days <= 365);
    }

    #[test]
    fn test_engagement_event_generator_page_view() {
        let generator = EngagementEventGenerator::page_view();
        let event = generator.generate("user-002");

        assert_eq!(event.event_type, EventType::PageView);

        let data = &event.data;
        assert!(
            data["page_url"]
                .as_str()
                .unwrap()
                .starts_with("https://shop.example.com")
        );
        assert!(data["duration_seconds"].as_i64().is_some());
        assert!(data["referrer"].as_str().is_some());

        let duration = data["duration_seconds"].as_i64().unwrap();
        assert!(duration >= 1 && duration <= 600);
    }

    #[test]
    fn test_engagement_event_generator_share() {
        let generator = EngagementEventGenerator::share();
        let event = generator.generate("user-003");

        assert_eq!(event.event_type, EventType::Share);

        let data = &event.data;
        let valid_platforms = ["wechat", "weibo", "qq", "douyin", "xiaohongshu"];
        let platform = data["platform"].as_str().unwrap();
        assert!(valid_platforms.contains(&platform));

        let valid_types = ["product", "article", "promotion"];
        let content_type = data["content_type"].as_str().unwrap();
        assert!(valid_types.contains(&content_type));

        assert!(data["content_id"].as_str().unwrap().starts_with("CNT-"));
    }

    #[test]
    fn test_engagement_event_generator_review() {
        let generator = EngagementEventGenerator::review();
        let event = generator.generate("user-004");

        assert_eq!(event.event_type, EventType::Review);

        let data = &event.data;
        assert!(data["product_id"].as_str().unwrap().starts_with("PROD-"));

        let rating = data["rating"].as_i64().unwrap();
        assert!(rating >= 1 && rating <= 5);

        assert!(!data["content"].as_str().unwrap().is_empty());
    }

    #[test]
    fn test_engagement_event_type() {
        assert_eq!(
            EngagementEventGenerator::checkin().event_type(),
            EventType::CheckIn
        );
        assert_eq!(
            EngagementEventGenerator::page_view().event_type(),
            EventType::PageView
        );
        assert_eq!(
            EngagementEventGenerator::share().event_type(),
            EventType::Share
        );
        assert_eq!(
            EngagementEventGenerator::review().event_type(),
            EventType::Review
        );
    }

    #[test]
    fn test_batch_generate() {
        let generator = EngagementEventGenerator::checkin();
        let events = generator.generate_batch("user-batch", 5);

        assert_eq!(events.len(), 5);
        for event in events {
            assert_eq!(event.event_type, EventType::CheckIn);
            assert_eq!(event.user_id, "user-batch");
        }
    }
}
