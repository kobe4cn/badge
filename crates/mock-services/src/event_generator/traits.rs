//! 事件生成器 trait 定义
//!
//! 定义统一的事件生成器接口，使不同类型的生成器可以互换使用。

use badge_shared::events::{EventPayload, EventType};

/// 事件生成器 trait
///
/// 所有事件生成器必须实现此 trait，提供生成单个和批量事件的能力。
/// 设计为同步接口是因为事件生成本身是纯计算操作，不涉及 I/O。
pub trait EventGenerator: Send + Sync {
    /// 生成单个事件
    ///
    /// 返回一个完整的 EventPayload，可直接发送到 Kafka。
    fn generate(&self, user_id: &str) -> EventPayload;

    /// 生成多个事件
    ///
    /// 批量生成同类型事件，用于压力测试或批量模拟场景。
    /// 默认实现通过循环调用 generate，子类可覆盖以优化性能。
    fn generate_batch(&self, user_id: &str, count: usize) -> Vec<EventPayload> {
        (0..count).map(|_| self.generate(user_id)).collect()
    }

    /// 该生成器产生的事件类型
    ///
    /// 用于事件路由和统计，确保生成器与其声明的类型一致。
    fn event_type(&self) -> EventType;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 简单的测试用生成器实现
    struct TestGenerator;

    impl EventGenerator for TestGenerator {
        fn generate(&self, user_id: &str) -> EventPayload {
            EventPayload::new(EventType::CheckIn, user_id, json!({"test": true}), "test")
        }

        fn event_type(&self) -> EventType {
            EventType::CheckIn
        }
    }

    #[test]
    fn test_trait_generate() {
        let generator = TestGenerator;
        let event = generator.generate("user-001");

        assert_eq!(event.user_id, "user-001");
        assert_eq!(event.event_type, EventType::CheckIn);
    }

    #[test]
    fn test_trait_generate_batch() {
        let generator = TestGenerator;
        let events = generator.generate_batch("user-001", 5);

        assert_eq!(events.len(), 5);
        for event in events {
            assert_eq!(event.user_id, "user-001");
            assert_eq!(event.event_type, EventType::CheckIn);
        }
    }

    #[test]
    fn test_trait_event_type() {
        let generator = TestGenerator;
        assert_eq!(generator.event_type(), EventType::CheckIn);
    }
}
