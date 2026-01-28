//! 行为事件处理器
//!
//! 实现 `EventProcessor` trait，负责行为类事件的完整处理流程。
//! 当前版本仅完成幂等校验和基本处理框架，规则匹配与 gRPC 调用
//! 将在 Task 6.4 中补充。

use std::time::Duration;

use async_trait::async_trait;
use badge_shared::cache::Cache;
use badge_shared::error::BadgeError;
use badge_shared::events::{EventPayload, EventProcessor, EventResult, EventType};
use tracing::{debug, info};

/// 幂等键前缀，标记事件是否已处理
const PROCESSED_KEY_PREFIX: &str = "event:processed:";
/// 幂等记录保留 24 小时，超过此时间窗口的重复消费不再拦截，
/// 因为 Kafka 消费偏移量提交周期远小于此窗口
const PROCESSED_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// 行为事件处理器
///
/// 通过 Redis 实现幂等性校验，确保 Kafka 重复投递不会导致重复处理。
/// gRPC 客户端（规则引擎 + 徽章管理）将在 Task 6.4 中注入。
pub struct EngagementEventProcessor {
    cache: Cache,
}

impl EngagementEventProcessor {
    pub fn new(cache: Cache) -> Self {
        Self { cache }
    }

    /// 构造 Redis 幂等键
    fn processed_key(event_id: &str) -> String {
        format!("{PROCESSED_KEY_PREFIX}{event_id}")
    }
}

#[async_trait]
impl EventProcessor for EngagementEventProcessor {
    /// 处理行为事件
    ///
    /// 当前返回空结果占位，Task 6.4 将在此调用规则引擎 BatchEvaluate
    /// 并对匹配的规则调用 GrantBadge 发放徽章。
    async fn process(&self, event: &EventPayload) -> Result<EventResult, BadgeError> {
        let start = std::time::Instant::now();

        info!(
            event_id = %event.event_id,
            event_type = %event.event_type,
            user_id = %event.user_id,
            "开始处理行为事件"
        );

        // TODO(Task 6.4): 调用规则引擎 BatchEvaluate + 徽章发放 GrantBadge
        let result = EventResult {
            event_id: event.event_id.clone(),
            processed: true,
            matched_rules: vec![],
            granted_badges: vec![],
            processing_time_ms: start.elapsed().as_millis() as i64,
            errors: vec![],
        };

        info!(
            event_id = %event.event_id,
            processing_time_ms = result.processing_time_ms,
            "行为事件处理完成"
        );

        Ok(result)
    }

    /// 本处理器负责的事件类型：所有行为类事件
    fn supported_event_types(&self) -> Vec<EventType> {
        vec![
            EventType::CheckIn,
            EventType::ProfileUpdate,
            EventType::PageView,
            EventType::Share,
            EventType::Review,
        ]
    }

    /// 通过 Redis EXISTS 检查事件是否已处理过
    async fn is_processed(&self, event_id: &str) -> Result<bool, BadgeError> {
        let key = Self::processed_key(event_id);
        let exists = self.cache.exists(&key).await?;

        if exists {
            debug!(event_id, "事件已处理，跳过");
        }

        Ok(exists)
    }

    /// 在 Redis 中设置幂等标记，24 小时后自动过期
    async fn mark_processed(&self, event_id: &str) -> Result<(), BadgeError> {
        let key = Self::processed_key(event_id);
        // 值为 "1" 即可，只需要键的存在性
        self.cache.set(&key, &"1", PROCESSED_TTL).await?;

        debug!(event_id, "事件已标记为已处理");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证支持的事件类型覆盖所有行为类事件
    #[test]
    fn test_supported_event_types() {
        // 使用 Default trait 不可用，直接构造一个无效 Cache 不合适，
        // 但 supported_event_types 是纯函数不需要 Redis 连接，
        // 所以用一个不会实际连接的 URL 构造
        let config = badge_shared::config::RedisConfig {
            url: "redis://localhost:6379".to_string(),
            pool_size: 1,
        };
        let cache = Cache::new(&config).expect("Redis client 创建失败");
        let processor = EngagementEventProcessor::new(cache);

        let types = processor.supported_event_types();

        assert_eq!(types.len(), 5);
        assert!(types.contains(&EventType::CheckIn));
        assert!(types.contains(&EventType::ProfileUpdate));
        assert!(types.contains(&EventType::PageView));
        assert!(types.contains(&EventType::Share));
        assert!(types.contains(&EventType::Review));

        // 确认不包含非行为类事件
        assert!(!types.contains(&EventType::Purchase));
        assert!(!types.contains(&EventType::Registration));
        assert!(!types.contains(&EventType::SeasonalActivity));
    }

    /// 验证 process 返回正确的 EventResult 结构
    #[tokio::test]
    async fn test_process_returns_result() {
        let config = badge_shared::config::RedisConfig {
            url: "redis://localhost:6379".to_string(),
            pool_size: 1,
        };
        let cache = Cache::new(&config).expect("Redis client 创建失败");
        let processor = EngagementEventProcessor::new(cache);

        let event = EventPayload::new(
            EventType::CheckIn,
            "user-001",
            serde_json::json!({"location": "北京"}),
            "test",
        );

        // process 内部不依赖 Redis 连接（当前版本），可以直接调用
        let result = processor.process(&event).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.event_id, event.event_id);
        assert!(result.processed);
        assert!(result.matched_rules.is_empty());
        assert!(result.granted_badges.is_empty());
        assert!(result.errors.is_empty());
        assert!(result.processing_time_ms >= 0);
    }
}
