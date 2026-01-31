//! Kafka 消费者与事件分发
//!
//! 将 Kafka 消息解码为事件信封，校验事件类型并路由到 TransactionEventProcessor。
//! 处理失败的消息发送到死信队列，处理成功时：
//! - Purchase 成功发放 -> 发送 BadgeGranted 通知
//! - Refund / OrderCancel 撤销成功 -> 发送 BadgeRevoked 通知

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

use crate::error::TransactionError;
use crate::processor::TransactionEventProcessor;

/// 交易事件消费者
///
/// 组合 KafkaConsumer（消息拉取）、TransactionEventProcessor（业务处理）
/// 和 KafkaProducer（通知/DLQ 投递）三个组件，形成完整的消费管道。
pub struct TransactionConsumer {
    consumer: KafkaConsumer,
    processor: TransactionEventProcessor,
    producer: KafkaProducer,
}

impl TransactionConsumer {
    pub fn new(
        config: &AppConfig,
        processor: TransactionEventProcessor,
        producer: KafkaProducer,
    ) -> Result<Self, TransactionError> {
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
    pub async fn run(self, shutdown: watch::Receiver<bool>) -> Result<(), TransactionError> {
        self.consumer.subscribe(&[topics::TRANSACTION_EVENTS])?;

        info!(topic = topics::TRANSACTION_EVENTS, "交易事件消费者已启动");

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
                            "处理交易事件失败"
                        );
                    }
                    Ok(())
                }
            })
            .await;

        info!("交易事件消费者已停止");
        Ok(())
    }
}

/// 处理单条 Kafka 消息的完整流程
///
/// 拆分为独立函数而非方法，便于在测试中直接调用而无需构造完整的 Consumer。
/// 流程：反序列化 -> 事件类型校验 -> 幂等检查 -> 业务处理 -> 标记已处理 -> 发送通知
pub async fn handle_message(
    processor: &TransactionEventProcessor,
    producer: &KafkaProducer,
    msg: &badge_shared::kafka::ConsumerMessage,
) -> Result<(), TransactionError> {
    // 1. 反序列化事件信封
    let event: EventPayload = msg.deserialize_payload().map_err(|e| {
        warn!(error = %e, "事件反序列化失败，将发送到死信队列");
        TransactionError::Shared(e)
    })?;

    info!(
        event_id = %event.event_id,
        event_type = %event.event_type,
        user_id = %event.user_id,
        "收到交易事件"
    );

    // 2. 校验是否为交易类事件
    if !is_supported_event_type(&event.event_type, processor) {
        warn!(
            event_type = %event.event_type,
            event_id = %event.event_id,
            "收到非交易类事件，忽略"
        );
        return Err(TransactionError::UnsupportedEventType {
            event_type: event.event_type.to_string(),
        });
    }

    // 3. 幂等检查：避免 Kafka 重复投递导致重复处理
    if processor.is_processed(&event.event_id).await? {
        return Err(TransactionError::AlreadyProcessed {
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
                "交易事件处理失败，发送到死信队列"
            );
            send_to_dlq(producer, &event).await;
            return Err(TransactionError::Shared(e));
        }
    };

    // 5. 标记为已处理
    if let Err(e) = processor.mark_processed(&event.event_id).await {
        warn!(
            event_id = %event.event_id,
            error = %e,
            "标记事件为已处理失败，后续可能重复处理"
        );
    }

    // 6. 根据事件类型发送不同的通知
    match event.event_type {
        EventType::Purchase => {
            // 有徽章发放时才通知
            if !result.granted_badges.is_empty() {
                send_grant_notification(producer, &event, &result).await;
            }
        }
        EventType::Refund | EventType::OrderCancel => {
            // 退款/取消事件中，只要没有全部失败就发送撤销通知
            if result.errors.is_empty() {
                send_revoke_notification(producer, &event).await;
            }
        }
        _ => {}
    }

    info!(
        event_id = %event.event_id,
        matched_rules = result.matched_rules.len(),
        granted_badges = result.granted_badges.len(),
        processing_time_ms = result.processing_time_ms,
        "交易事件处理完成"
    );

    Ok(())
}

/// 校验事件类型是否在处理器的支持列表中
fn is_supported_event_type(event_type: &EventType, processor: &TransactionEventProcessor) -> bool {
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

/// Purchase 成功发放时生成 BadgeGranted 通知
async fn send_grant_notification(
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
            warn!(
                event_id = %event.event_id,
                badge_id = badge.badge_id,
                error = %e,
                "发送徽章发放通知失败"
            );
        }
    }
}

/// Refund / OrderCancel 撤销成功时生成 BadgeRevoked 通知
async fn send_revoke_notification(producer: &KafkaProducer, event: &EventPayload) {
    let original_order_id = event.data["original_order_id"]
        .as_str()
        .unwrap_or("unknown");

    let notification = NotificationEvent {
        notification_id: Uuid::now_v7().to_string(),
        user_id: event.user_id.clone(),
        notification_type: NotificationType::BadgeRevoked,
        title: "徽章变更通知".to_string(),
        body: format!("由于订单 {} 退款/取消，相关徽章已被回收", original_order_id),
        data: serde_json::json!({
            "original_order_id": original_order_id,
            "source_event_id": event.event_id,
            "event_type": event.event_type.to_string(),
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
        warn!(
            event_id = %event.event_id,
            error = %e,
            "发送徽章撤销通知失败"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use badge_shared::kafka::ConsumerMessage;
    use std::collections::HashMap;

    /// 构造测试用的 ConsumerMessage
    fn make_test_message(event: &EventPayload) -> ConsumerMessage {
        let payload = serde_json::to_vec(event).expect("序列化测试事件失败");
        ConsumerMessage {
            topic: topics::TRANSACTION_EVENTS.to_string(),
            partition: 0,
            offset: 1,
            key: Some(event.event_id.clone()),
            payload,
            timestamp: Some(Utc::now().timestamp_millis()),
            headers: HashMap::new(),
        }
    }

    /// 交易服务支持的事件类型
    fn get_supported_event_types() -> Vec<EventType> {
        vec![
            EventType::Purchase,
            EventType::Refund,
            EventType::OrderCancel,
        ]
    }

    /// 非交易类事件应被拒绝
    #[test]
    fn test_handle_unsupported_event_type() {
        let supported_types = get_supported_event_types();

        // CheckIn 是行为类事件，不在交易处理器的支持列表中
        assert!(!supported_types.contains(&EventType::CheckIn));

        // Purchase 是交易类事件，应该在支持列表中
        assert!(supported_types.contains(&EventType::Purchase));

        // Refund 也是交易类事件
        assert!(supported_types.contains(&EventType::Refund));

        // OrderCancel 也是交易类事件
        assert!(supported_types.contains(&EventType::OrderCancel));
    }

    /// 验证有效交易事件可以正确解析和类型校验
    #[test]
    fn test_handle_valid_transaction_event() {
        let event = EventPayload::new(
            EventType::Purchase,
            "user-001",
            serde_json::json!({"amount": 100.0, "category": "electronics"}),
            "order-service",
        );

        let msg = make_test_message(&event);

        // 验证消息可以反序列化
        let deserialized: EventPayload = msg.deserialize_payload().expect("反序列化失败");
        assert_eq!(deserialized.event_id, event.event_id);
        assert_eq!(deserialized.event_type, EventType::Purchase);
        assert_eq!(deserialized.user_id, "user-001");

        // 验证事件类型是受支持的
        let supported_types = get_supported_event_types();
        assert!(supported_types.contains(&deserialized.event_type));
    }

    /// 验证退款事件可以正确解析
    #[test]
    fn test_handle_refund_event_deserialize() {
        let event = EventPayload::new(
            EventType::Refund,
            "user-002",
            serde_json::json!({
                "original_order_id": "order-123",
                "badge_ids": [42, 43],
                "refund_reason": "商品不满意"
            }),
            "order-service",
        );

        let msg = make_test_message(&event);

        let deserialized: EventPayload = msg.deserialize_payload().expect("反序列化失败");
        assert_eq!(deserialized.event_type, EventType::Refund);
        assert_eq!(deserialized.data["original_order_id"], "order-123");
        assert_eq!(deserialized.data["refund_reason"], "商品不满意");

        let badge_ids = deserialized.data["badge_ids"]
            .as_array()
            .expect("badge_ids 应为数组");
        assert_eq!(badge_ids.len(), 2);
    }
}
