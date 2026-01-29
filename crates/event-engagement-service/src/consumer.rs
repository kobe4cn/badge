//! Kafka 消费者与事件分发
//!
//! 将 Kafka 消息解码为事件信封，校验事件类型并路由到 EngagementEventProcessor，
//! 处理失败的消息发送到死信队列，处理成功的结果生成通知事件。

use badge_shared::config::AppConfig;
use badge_shared::events::{
    EventPayload, EventProcessor, EventType, NotificationChannel, NotificationEvent,
    NotificationType,
};
use badge_shared::kafka::{KafkaConsumer, KafkaProducer, topics};
use chrono::Utc;
use tokio::sync::watch;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::error::EngagementError;
use crate::processor::EngagementEventProcessor;

/// 行为事件消费者
///
/// 组合 KafkaConsumer（消息拉取）、EngagementEventProcessor（业务处理）
/// 和 KafkaProducer（通知/DLQ 投递）三个组件，形成完整的消费管道。
pub struct EngagementConsumer {
    consumer: KafkaConsumer,
    processor: EngagementEventProcessor,
    producer: KafkaProducer,
}

impl EngagementConsumer {
    pub fn new(
        config: &AppConfig,
        processor: EngagementEventProcessor,
        producer: KafkaProducer,
    ) -> Result<Self, EngagementError> {
        let consumer = KafkaConsumer::new(&config.kafka, None)?;
        Ok(Self {
            consumer,
            processor,
            producer,
        })
    }

    /// 启动消费循环，直到收到 shutdown 信号
    ///
    /// 将 processor 和 producer 移入闭包，通过 KafkaConsumer::start
    /// 驱动消费循环。单独抽取 handle_message 方法方便单元测试。
    pub async fn run(self, shutdown: watch::Receiver<bool>) -> Result<(), EngagementError> {
        self.consumer.subscribe(&[topics::ENGAGEMENT_EVENTS])?;

        info!(topic = topics::ENGAGEMENT_EVENTS, "行为事件消费者已启动");

        let processor = self.processor;
        let producer = self.producer;

        self.consumer
            .start(shutdown, |msg| {
                let processor = &processor;
                let producer = &producer;
                async move {
                    if let Err(e) = handle_message(processor, producer, &msg).await {
                        error!(
                            error = %e,
                            topic = %msg.topic,
                            partition = msg.partition,
                            offset = msg.offset,
                            "处理行为事件失败"
                        );
                    }
                    Ok(())
                }
            })
            .await;

        info!("行为事件消费者已停止");
        Ok(())
    }
}

/// 处理单条 Kafka 消息的完整流程
///
/// 拆分为独立函数而非方法，便于在测试中直接调用而无需构造完整的 Consumer。
/// 流程：反序列化 -> 事件类型校验 -> 幂等检查 -> 业务处理 -> 标记已处理 -> 发送通知
pub async fn handle_message(
    processor: &EngagementEventProcessor,
    producer: &KafkaProducer,
    msg: &badge_shared::kafka::ConsumerMessage,
) -> Result<(), EngagementError> {
    // 1. 反序列化事件信封
    let event: EventPayload = msg.deserialize_payload().map_err(|e| {
        warn!(error = %e, "事件反序列化失败，将发送到死信队列");
        EngagementError::Shared(e)
    })?;

    info!(
        event_id = %event.event_id,
        event_type = %event.event_type,
        user_id = %event.user_id,
        "收到行为事件"
    );

    // 2. 校验是否为行为类事件
    if !is_supported_event_type(&event.event_type, processor) {
        warn!(
            event_type = %event.event_type,
            event_id = %event.event_id,
            "收到非行为类事件，忽略"
        );
        return Err(EngagementError::UnsupportedEventType {
            event_type: event.event_type.to_string(),
        });
    }

    // 3. 幂等检查：避免 Kafka 重复投递导致重复处理
    if processor.is_processed(&event.event_id).await? {
        return Err(EngagementError::AlreadyProcessed {
            event_id: event.event_id,
        });
    }

    // 4. 执行业务处理
    let result = match processor.process(&event).await {
        Ok(r) => r,
        Err(e) => {
            error!(
                event_id = %event.event_id,
                error = %e,
                "行为事件处理失败，发送到死信队列"
            );
            send_to_dlq(producer, &event).await;
            return Err(EngagementError::Shared(e));
        }
    };

    // 5. 标记为已处理
    if let Err(e) = processor.mark_processed(&event.event_id).await {
        // 标记失败不影响本次处理结果，但可能导致重复处理
        warn!(
            event_id = %event.event_id,
            error = %e,
            "标记事件为已处理失败，后续可能重复处理"
        );
    }

    // 6. 发送通知事件（仅在有徽章发放时通知）
    if !result.granted_badges.is_empty() {
        send_notification(producer, &event, &result).await;
    }

    info!(
        event_id = %event.event_id,
        matched_rules = result.matched_rules.len(),
        granted_badges = result.granted_badges.len(),
        processing_time_ms = result.processing_time_ms,
        "行为事件处理完成"
    );

    Ok(())
}

/// 校验事件类型是否在处理器的支持列表中
fn is_supported_event_type(event_type: &EventType, processor: &EngagementEventProcessor) -> bool {
    processor.supported_event_types().contains(event_type)
}

/// 将处理失败的事件发送到死信队列，供人工排查或延迟重试
async fn send_to_dlq(producer: &KafkaProducer, event: &EventPayload) {
    if let Err(e) = producer
        .send_json(topics::DEAD_LETTER_QUEUE, &event.event_id, event)
        .await
    {
        error!(
            event_id = %event.event_id,
            error = %e,
            "发送到死信队列失败，消息可能丢失"
        );
    }
}

/// 为成功发放的徽章生成通知事件并投递到通知 topic
async fn send_notification(
    producer: &KafkaProducer,
    event: &EventPayload,
    result: &badge_shared::events::EventResult,
) {
    for badge in &result.granted_badges {
        let notification = NotificationEvent {
            notification_id: Uuid::now_v7().to_string(),
            user_id: event.user_id.clone(),
            notification_type: NotificationType::BadgeGranted,
            title: "恭喜获得新徽章".to_string(),
            body: format!("您已获得「{}」徽章！", badge.badge_name),
            data: serde_json::json!({
                "badge_id": badge.badge_id,
                "badge_name": badge.badge_name,
                "quantity": badge.quantity,
                "source_event_id": event.event_id,
            }),
            channels: vec![NotificationChannel::AppPush],
            created_at: Utc::now(),
        };

        if let Err(e) = producer
            .send_json(
                topics::BADGE_NOTIFICATIONS,
                &notification.notification_id,
                &notification,
            )
            .await
        {
            // 通知发送失败不影响核心业务，仅记录警告
            warn!(
                event_id = %event.event_id,
                badge_id = badge.badge_id,
                error = %e,
                "发送徽章通知失败"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule_client::{BadgeRuleService, GrantResult, RuleMatch};
    use crate::rule_mapping::RuleBadgeMapping;
    use badge_shared::kafka::ConsumerMessage;
    use std::collections::HashMap;
    use std::sync::Arc;

    /// Mock 服务实现，用于 consumer 测试中构造 processor
    struct MockBadgeRuleService;

    #[async_trait::async_trait]
    impl BadgeRuleService for MockBadgeRuleService {
        async fn evaluate_rules(
            &self,
            _rule_ids: &[String],
            _context: serde_json::Value,
        ) -> Result<Vec<RuleMatch>, crate::error::EngagementError> {
            Ok(vec![])
        }

        async fn grant_badge(
            &self,
            _user_id: &str,
            _badge_id: &str,
            _quantity: i32,
            _source_ref: &str,
        ) -> Result<GrantResult, crate::error::EngagementError> {
            Ok(GrantResult {
                success: true,
                user_badge_id: "0".to_string(),
                message: String::new(),
            })
        }
    }

    /// 构造测试用 processor
    fn make_test_processor() -> EngagementEventProcessor {
        let config = badge_shared::config::RedisConfig {
            url: "redis://localhost:6379".to_string(),
            pool_size: 1,
        };
        let cache = badge_shared::cache::Cache::new(&config).expect("Redis client 创建失败");
        EngagementEventProcessor::new(
            cache,
            Arc::new(MockBadgeRuleService),
            Arc::new(RuleBadgeMapping::new()),
        )
    }

    /// 构造测试用的 ConsumerMessage
    fn make_test_message(event: &EventPayload) -> ConsumerMessage {
        let payload = serde_json::to_vec(event).expect("序列化测试事件失败");
        ConsumerMessage {
            topic: topics::ENGAGEMENT_EVENTS.to_string(),
            partition: 0,
            offset: 1,
            key: Some(event.event_id.clone()),
            payload,
            timestamp: Some(Utc::now().timestamp_millis()),
            headers: HashMap::new(),
        }
    }

    /// 非行为类事件应被拒绝
    #[test]
    fn test_handle_unsupported_event_type() {
        let processor = make_test_processor();

        // Purchase 是交易类事件，不在行为处理器的支持列表中
        let purchase_type = EventType::Purchase;
        assert!(!is_supported_event_type(&purchase_type, &processor));

        // CheckIn 是行为类事件，应该通过校验
        let checkin_type = EventType::CheckIn;
        assert!(is_supported_event_type(&checkin_type, &processor));
    }

    /// 验证有效行为事件可以正确解析和类型校验
    #[test]
    fn test_handle_valid_engagement_event() {
        let event = EventPayload::new(
            EventType::CheckIn,
            "user-001",
            serde_json::json!({"location": "上海"}),
            "test",
        );

        let msg = make_test_message(&event);

        // 验证消息可以反序列化
        let deserialized: EventPayload = msg.deserialize_payload().expect("反序列化失败");
        assert_eq!(deserialized.event_id, event.event_id);
        assert_eq!(deserialized.event_type, EventType::CheckIn);
        assert_eq!(deserialized.user_id, "user-001");

        // 验证事件类型是受支持的
        let processor = make_test_processor();

        assert!(is_supported_event_type(
            &deserialized.event_type,
            &processor
        ));
    }
}
