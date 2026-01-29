//! 死信队列处理
//!
//! 当事件处理失败且重试耗尽后，消息会被发送到死信队列（DLQ）。
//! DLQ 消费者会按退避策略尝试重新投递，超过上限后记录日志等待人工介入。
//! 这一机制确保消息不会因瞬时故障而永久丢失。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use tracing::{error, info, warn};

use crate::config::AppConfig;
use crate::error::BadgeError;
use crate::events::EventPayload;
use crate::kafka::{ConsumerMessage, KafkaConsumer, KafkaProducer, topics};
use crate::retry::RetryPolicy;

// ---------------------------------------------------------------------------
// DeadLetterMessage — 死信消息信封
// ---------------------------------------------------------------------------

/// 死信消息信封
///
/// 包装原始消息，附加失败原因、重试次数等元数据，
/// 便于在死信队列消费时决定是否重试或永久归档。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeadLetterMessage {
    /// 原始消息 ID（如 event_id）
    pub message_id: String,
    /// 原始 topic
    pub source_topic: String,
    /// 原始消息内容（JSON 序列化的字符串）
    pub payload: String,
    /// 失败原因
    pub error: String,
    /// 已重试次数
    pub retry_count: u32,
    /// 最大重试次数
    pub max_retries: u32,
    /// 首次失败时间
    pub first_failed_at: DateTime<Utc>,
    /// 最近失败时间
    pub last_failed_at: DateTime<Utc>,
    /// 下次重试时间（None 表示不再重试）
    pub next_retry_at: Option<DateTime<Utc>>,
    /// 来源服务
    pub source_service: String,
}

impl DeadLetterMessage {
    /// 创建新的死信消息
    ///
    /// 首次进入 DLQ 时 retry_count 为 0，next_retry_at 立即设置为当前时间，
    /// 让 DLQ 消费者在首轮扫描时即可尝试重新投递。
    pub fn new(
        message_id: impl Into<String>,
        source_topic: impl Into<String>,
        payload: impl Into<String>,
        error: impl Into<String>,
        max_retries: u32,
        source_service: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            message_id: message_id.into(),
            source_topic: source_topic.into(),
            payload: payload.into(),
            error: error.into(),
            retry_count: 0,
            max_retries,
            first_failed_at: now,
            last_failed_at: now,
            next_retry_at: Some(now),
            source_service: source_service.into(),
        }
    }

    /// 是否应继续重试
    ///
    /// 只要已重试次数尚未达到上限，就允许继续尝试
    pub fn should_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    /// 增加重试计数并更新元数据
    ///
    /// 每次重试失败后调用，更新错误信息和时间戳，
    /// 并根据退避策略计算下一次重试时间。
    /// 如果已达上限则 next_retry_at 置为 None，表示不再重试。
    pub fn increment_retry(&mut self, error: &str, retry_policy: &RetryPolicy) {
        self.retry_count += 1;
        self.error = error.to_string();
        self.last_failed_at = Utc::now();

        if self.should_retry() {
            let delay = retry_policy.delay_for_attempt(self.retry_count);
            self.next_retry_at =
                Some(self.last_failed_at + chrono::Duration::from_std(delay).unwrap_or_default());
        } else {
            // 已耗尽重试机会，不再安排重试
            self.next_retry_at = None;
        }
    }
}

// ---------------------------------------------------------------------------
// DlqProducer — 将失败消息发送到死信队列
// ---------------------------------------------------------------------------

/// DLQ 生产者
///
/// 各服务在事件处理失败后调用此组件将消息写入死信队列，
/// 而非直接丢弃。保证消息最终会被重试或人工处理。
pub struct DlqProducer {
    producer: KafkaProducer,
    source_service: String,
    retry_policy: RetryPolicy,
}

impl DlqProducer {
    pub fn new(producer: KafkaProducer, source_service: &str, retry_policy: RetryPolicy) -> Self {
        Self {
            producer,
            source_service: source_service.to_string(),
            retry_policy,
        }
    }

    /// 将失败消息发送到死信队列
    pub async fn send_to_dlq(
        &self,
        message_id: &str,
        source_topic: &str,
        payload: &str,
        error: &str,
    ) -> Result<(), BadgeError> {
        let dlq_msg = DeadLetterMessage::new(
            message_id,
            source_topic,
            payload,
            error,
            self.retry_policy.max_retries,
            &self.source_service,
        );

        self.producer
            .send_json(topics::DEAD_LETTER_QUEUE, message_id, &dlq_msg)
            .await?;

        warn!(message_id, source_topic, error, "消息已发送到死信队列");

        Ok(())
    }

    /// 从 EventPayload 构造死信消息并发送
    ///
    /// 便捷方法：自动提取 event_id 作为 message_id，
    /// 根据事件类型推断 source_topic，并将整个事件序列化为 payload。
    pub async fn send_event_to_dlq(
        &self,
        event: &EventPayload,
        error: &str,
    ) -> Result<(), BadgeError> {
        let payload = serde_json::to_string(event)
            .map_err(|e| BadgeError::Kafka(format!("序列化事件失败: {e}")))?;

        // 根据事件类型推断原始 topic
        let source_topic = if event.event_type.is_transaction() {
            topics::TRANSACTION_EVENTS
        } else if event.event_type.is_engagement() {
            topics::ENGAGEMENT_EVENTS
        } else {
            // 身份类和季节类事件暂归入行为事件 topic
            topics::ENGAGEMENT_EVENTS
        };

        self.send_to_dlq(&event.event_id, source_topic, &payload, error)
            .await
    }
}

// ---------------------------------------------------------------------------
// DlqConsumer — 处理死信队列消息
// ---------------------------------------------------------------------------

/// DLQ 消费者
///
/// 持续消费死信队列，对尚有重试机会且已到达重试时间的消息重新投递到原始 topic。
/// 超过重试上限的消息记录日志以便人工介入。
pub struct DlqConsumer {
    consumer: KafkaConsumer,
    /// 将待重试的消息发回原始 topic
    retry_producer: KafkaProducer,
}

impl DlqConsumer {
    /// 创建 DLQ 消费者
    ///
    /// 使用 `.dlq` 后缀作为独立消费组，与业务消费者互不干扰
    pub fn new(config: &AppConfig, retry_producer: KafkaProducer) -> Result<Self, BadgeError> {
        let consumer = KafkaConsumer::new(&config.kafka, Some("dlq"))?;
        consumer.subscribe(&[topics::DEAD_LETTER_QUEUE])?;

        info!(
            "DLQ 消费者已创建，订阅 topic: {}",
            topics::DEAD_LETTER_QUEUE
        );

        Ok(Self {
            consumer,
            retry_producer,
        })
    }

    /// 启动 DLQ 消费循环
    pub async fn run(self, shutdown: watch::Receiver<bool>) {
        let retry_producer = self.retry_producer.clone();

        self.consumer
            .start(shutdown, move |msg| {
                let producer = retry_producer.clone();
                async move { handle_dlq_message(&msg, &producer).await }
            })
            .await;

        info!("DLQ 消费循环已退出");
    }
}

/// 处理单条死信消息
///
/// 判断消息是否仍可重试且重试时间已到达：
/// - 是 → 将原始 payload 发回 source_topic，由业务消费者重新处理
/// - 否 → 记录错误日志，需要人工介入处理
async fn handle_dlq_message(
    msg: &ConsumerMessage,
    retry_producer: &KafkaProducer,
) -> Result<(), BadgeError> {
    let dlq_msg: DeadLetterMessage = msg.deserialize_payload()?;

    if dlq_msg.should_retry() {
        // 检查是否已到达下次重试时间
        let now = Utc::now();
        if let Some(next_retry) = dlq_msg.next_retry_at
            && now >= next_retry
        {
            info!(
                message_id = %dlq_msg.message_id,
                source_topic = %dlq_msg.source_topic,
                retry_count = dlq_msg.retry_count,
                max_retries = dlq_msg.max_retries,
                "重试死信消息，发回原始 topic"
            );

            retry_producer
                .send(
                    &dlq_msg.source_topic,
                    &dlq_msg.message_id,
                    dlq_msg.payload.as_bytes(),
                )
                .await?;

            return Ok(());
        }

        // 重试时间未到，记录调试信息（消息会在下次消费时再次检查）
        info!(
            message_id = %dlq_msg.message_id,
            next_retry_at = ?dlq_msg.next_retry_at,
            "死信消息重试时间未到，跳过"
        );
    } else {
        // 已耗尽重试次数，需人工介入
        error!(
            message_id = %dlq_msg.message_id,
            source_topic = %dlq_msg.source_topic,
            source_service = %dlq_msg.source_service,
            retry_count = dlq_msg.retry_count,
            max_retries = dlq_msg.max_retries,
            first_failed_at = %dlq_msg.first_failed_at,
            last_failed_at = %dlq_msg.last_failed_at,
            error = %dlq_msg.error,
            "死信消息已耗尽重试次数，需人工介入"
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::retry::RetryPolicy;
    use std::time::Duration;

    #[test]
    fn test_dead_letter_message_creation() {
        let msg = DeadLetterMessage::new(
            "evt-001",
            "badge.engagement.events",
            r#"{"eventId":"evt-001"}"#,
            "处理超时",
            3,
            "engagement-service",
        );

        assert_eq!(msg.message_id, "evt-001");
        assert_eq!(msg.source_topic, "badge.engagement.events");
        assert_eq!(msg.payload, r#"{"eventId":"evt-001"}"#);
        assert_eq!(msg.error, "处理超时");
        assert_eq!(msg.retry_count, 0);
        assert_eq!(msg.max_retries, 3);
        assert_eq!(msg.source_service, "engagement-service");
        assert!(msg.next_retry_at.is_some());
        // 首次失败和最近失败时间应相同
        assert_eq!(msg.first_failed_at, msg.last_failed_at);
    }

    #[test]
    fn test_should_retry_when_under_limit() {
        let msg = DeadLetterMessage::new("evt-001", "topic", "payload", "error", 3, "svc");
        // retry_count=0 < max_retries=3
        assert!(msg.should_retry());
    }

    #[test]
    fn test_should_not_retry_when_at_limit() {
        let mut msg = DeadLetterMessage::new("evt-001", "topic", "payload", "error", 2, "svc");
        msg.retry_count = 2;
        // retry_count=2 == max_retries=2
        assert!(!msg.should_retry());

        msg.retry_count = 3;
        assert!(!msg.should_retry());
    }

    #[test]
    fn test_increment_retry() {
        let mut msg = DeadLetterMessage::new("evt-001", "topic", "payload", "初始错误", 3, "svc");
        let policy = RetryPolicy {
            max_retries: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
        };

        let original_first_failed = msg.first_failed_at;

        // 第一次重试失败
        msg.increment_retry("第二次错误", &policy);
        assert_eq!(msg.retry_count, 1);
        assert_eq!(msg.error, "第二次错误");
        assert!(msg.next_retry_at.is_some());
        // first_failed_at 不应改变
        assert_eq!(msg.first_failed_at, original_first_failed);

        // 第二次重试失败
        msg.increment_retry("第三次错误", &policy);
        assert_eq!(msg.retry_count, 2);
        assert_eq!(msg.error, "第三次错误");
        assert!(msg.next_retry_at.is_some());

        // 第三次重试失败——已达上限
        msg.increment_retry("最终错误", &policy);
        assert_eq!(msg.retry_count, 3);
        assert_eq!(msg.error, "最终错误");
        // 达到上限后不再安排重试
        assert!(msg.next_retry_at.is_none());
        assert!(!msg.should_retry());
    }

    #[test]
    fn test_dead_letter_serialization() {
        let msg = DeadLetterMessage::new(
            "evt-002",
            "badge.transaction.events",
            r#"{"amount":100}"#,
            "数据库连接失败",
            5,
            "transaction-service",
        );

        let json = serde_json::to_string(&msg).unwrap();

        // 验证 camelCase 序列化
        assert!(json.contains("messageId"));
        assert!(json.contains("sourceTopic"));
        assert!(json.contains("retryCount"));
        assert!(json.contains("maxRetries"));
        assert!(json.contains("firstFailedAt"));
        assert!(json.contains("lastFailedAt"));
        assert!(json.contains("nextRetryAt"));
        assert!(json.contains("sourceService"));

        // 验证能反序列化回来
        let deserialized: DeadLetterMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.message_id, "evt-002");
        assert_eq!(deserialized.source_topic, "badge.transaction.events");
        assert_eq!(deserialized.retry_count, 0);
        assert_eq!(deserialized.max_retries, 5);
        assert_eq!(deserialized.source_service, "transaction-service");
    }
}
