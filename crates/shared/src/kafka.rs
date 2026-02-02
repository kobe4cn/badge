//! Kafka 基础设施封装
//!
//! 将 rdkafka 的底层 API 封装为业务友好的 Producer/Consumer 抽象，
//! 统一消息序列化、错误映射和优雅关闭语义，避免各服务重复编写样板代码。

use std::collections::HashMap;
use std::time::Duration;

use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::{BorrowedMessage, Headers, Message};
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

use crate::config::KafkaConfig;
use crate::error::BadgeError;

// ---------------------------------------------------------------------------
// Topic 常量
// ---------------------------------------------------------------------------

/// 集中管理所有 Kafka topic 名称，防止字符串散落在各服务中导致拼写不一致
pub mod topics {
    pub const ENGAGEMENT_EVENTS: &str = "badge.engagement.events";
    pub const TRANSACTION_EVENTS: &str = "badge.transaction.events";
    pub const BADGE_NOTIFICATIONS: &str = "badge.notifications";
    pub const DEAD_LETTER_QUEUE: &str = "badge.dlq";
    pub const RULE_RELOAD: &str = "badge.rule.reload";
}

// ---------------------------------------------------------------------------
// ConsumerMessage
// ---------------------------------------------------------------------------

/// 消费到的 Kafka 消息的统一表示
///
/// 将 rdkafka 的 `BorrowedMessage`（带生命周期约束）转换为拥有所有权的结构体，
/// 使消息可以安全地跨 await 点传递给异步处理函数。
#[derive(Debug, Clone)]
pub struct ConsumerMessage {
    pub topic: String,
    pub partition: i32,
    pub offset: i64,
    pub key: Option<String>,
    pub payload: Vec<u8>,
    pub timestamp: Option<i64>,
    pub headers: HashMap<String, String>,
}

impl ConsumerMessage {
    /// 从 rdkafka 的借用消息构造，提取并拥有所有字段
    fn from_borrowed(msg: &BorrowedMessage<'_>) -> Self {
        let key = msg
            .key()
            .and_then(|k| std::str::from_utf8(k).ok())
            .map(String::from);

        let payload = msg.payload().map(|p| p.to_vec()).unwrap_or_default();

        let timestamp = msg.timestamp().to_millis();

        let mut headers = HashMap::new();
        if let Some(h) = msg.headers() {
            for idx in 0..h.count() {
                let header = h.get(idx);
                if let Some(raw) = header.value
                    && let Ok(value) = std::str::from_utf8(raw)
                {
                    headers.insert(header.key.to_string(), value.to_string());
                }
            }
        }

        Self {
            topic: msg.topic().to_string(),
            partition: msg.partition(),
            offset: msg.offset(),
            key,
            payload,
            timestamp,
            headers,
        }
    }

    /// 将负载视为 UTF-8 字符串返回
    pub fn payload_str(&self) -> Result<&str, BadgeError> {
        std::str::from_utf8(&self.payload)
            .map_err(|e| BadgeError::Kafka(format!("负载非 UTF-8 编码: {e}")))
    }

    /// 将 JSON 格式负载反序列化为目标类型
    pub fn deserialize_payload<T: DeserializeOwned>(&self) -> Result<T, BadgeError> {
        serde_json::from_slice(&self.payload)
            .map_err(|e| BadgeError::Kafka(format!("负载反序列化失败: {e}")))
    }
}

// ---------------------------------------------------------------------------
// KafkaProducer
// ---------------------------------------------------------------------------

/// 面向业务的 Kafka 生产者
///
/// 封装 `FutureProducer` 并提供类型安全的 JSON 发送方法，
/// 内部已派生 Clone（`FutureProducer` 本身是 Arc 包装的）。
#[derive(Clone)]
pub struct KafkaProducer {
    producer: FutureProducer,
}

impl KafkaProducer {
    /// 根据配置创建生产者
    ///
    /// 设置 `message.timeout.ms` 为 5 秒——在徽章系统中如果 5 秒内仍无法投递，
    /// 应由上层重试或写入死信队列，而非无限等待。
    pub fn new(config: &KafkaConfig) -> Result<Self, BadgeError> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &config.brokers)
            .set("message.timeout.ms", "5000")
            .create()
            .map_err(|e| BadgeError::Kafka(format!("创建生产者失败: {e}")))?;

        info!(brokers = %config.brokers, "Kafka 生产者已初始化");
        Ok(Self { producer })
    }

    /// 发送原始字节消息
    pub async fn send(
        &self,
        topic: &str,
        key: &str,
        payload: &[u8],
    ) -> Result<(i32, i64), BadgeError> {
        let record = FutureRecord::to(topic).key(key).payload(payload);

        // rdkafka 0.39+ 返回 Delivery 结构体而非元组
        let delivery = self
            .producer
            .send(record, Duration::from_secs(5))
            .await
            .map_err(|(e, _)| BadgeError::Kafka(format!("发送消息失败: {e}")))?;

        debug!(
            topic,
            key,
            partition = delivery.partition,
            offset = delivery.offset,
            "消息已发送"
        );
        Ok((delivery.partition, delivery.offset))
    }

    /// 将值序列化为 JSON 后发送
    ///
    /// 序列化与网络发送拆分为两步，便于独立定位故障原因。
    pub async fn send_json<T: Serialize>(
        &self,
        topic: &str,
        key: &str,
        value: &T,
    ) -> Result<(i32, i64), BadgeError> {
        let payload =
            serde_json::to_vec(value).map_err(|e| BadgeError::Kafka(format!("序列化失败: {e}")))?;

        self.send(topic, key, &payload).await
    }
}

// ---------------------------------------------------------------------------
// KafkaConsumer
// ---------------------------------------------------------------------------

/// 面向业务的 Kafka 消费者
///
/// 封装 `StreamConsumer` 并提供基于 `watch` channel 的优雅关闭语义，
/// 确保进程退出时不会丢失正在处理的消息。
pub struct KafkaConsumer {
    consumer: StreamConsumer,
}

impl KafkaConsumer {
    /// 创建消费者
    ///
    /// `group_id_suffix` 允许同一服务内不同消费逻辑使用独立的消费组，
    /// 例如 "badge-service.notifications" 和 "badge-service.dlq"。
    pub fn new(config: &KafkaConfig, group_id_suffix: Option<&str>) -> Result<Self, BadgeError> {
        let group_id = match group_id_suffix {
            Some(suffix) => format!("{}.{}", config.consumer_group, suffix),
            None => config.consumer_group.clone(),
        };

        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", &config.brokers)
            .set("group.id", &group_id)
            .set("auto.offset.reset", &config.auto_offset_reset)
            .set("enable.auto.commit", "true")
            .create()
            .map_err(|e| BadgeError::Kafka(format!("创建消费者失败: {e}")))?;

        info!(brokers = %config.brokers, group_id, "Kafka 消费者已初始化");
        Ok(Self { consumer })
    }

    /// 订阅指定的 topic 列表
    pub fn subscribe(&self, topics: &[&str]) -> Result<(), BadgeError> {
        self.consumer
            .subscribe(topics)
            .map_err(|e| BadgeError::Kafka(format!("订阅 topic 失败: {e}")))?;

        info!(?topics, "已订阅 Kafka topics");
        Ok(())
    }

    /// 启动消费循环
    ///
    /// 使用 `tokio::select!` 同时监听消息流和关闭信号：
    /// - 收到消息时调用 handler 处理；handler 返回错误只记录日志而不中断循环，
    ///   避免单条坏消息导致整个消费者停止。
    /// - 关闭信号变为 `true` 时退出循环，确保正在执行的 handler 能自然完成。
    pub async fn start<F, Fut>(self, mut shutdown: watch::Receiver<bool>, handler: F)
    where
        F: Fn(ConsumerMessage) -> Fut,
        Fut: std::future::Future<Output = Result<(), BadgeError>>,
    {
        use futures::StreamExt;

        let stream = self.consumer.stream();
        futures::pin_mut!(stream);

        info!("Kafka 消费循环已启动");

        loop {
            tokio::select! {
                // 偏向关闭信号，保证收到关闭时能尽快退出
                biased;

                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        info!("收到关闭信号，Kafka 消费循环退出");
                        break;
                    }
                }

                msg_result = stream.next() => {
                    let Some(msg_result) = msg_result else {
                        warn!("Kafka 消息流意外结束");
                        break;
                    };

                    match msg_result {
                        Ok(borrowed_msg) => {
                            let msg = ConsumerMessage::from_borrowed(&borrowed_msg);
                            debug!(
                                topic = %msg.topic,
                                partition = msg.partition,
                                offset = msg.offset,
                                "收到 Kafka 消息"
                            );

                            if let Err(e) = handler(msg).await {
                                error!(error = %e, "处理 Kafka 消息失败");
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "接收 Kafka 消息出错");
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_constants() {
        assert_eq!(topics::ENGAGEMENT_EVENTS, "badge.engagement.events");
        assert_eq!(topics::TRANSACTION_EVENTS, "badge.transaction.events");
        assert_eq!(topics::BADGE_NOTIFICATIONS, "badge.notifications");
        assert_eq!(topics::DEAD_LETTER_QUEUE, "badge.dlq");
    }

    #[test]
    fn test_consumer_message_creation() {
        let msg = ConsumerMessage {
            topic: "test-topic".to_string(),
            partition: 0,
            offset: 42,
            key: Some("key-1".to_string()),
            payload: b"hello".to_vec(),
            timestamp: Some(1_700_000_000_000),
            headers: HashMap::from([("trace-id".to_string(), "abc-123".to_string())]),
        };

        assert_eq!(msg.topic, "test-topic");
        assert_eq!(msg.partition, 0);
        assert_eq!(msg.offset, 42);
        assert_eq!(msg.key.as_deref(), Some("key-1"));
        assert_eq!(msg.payload, b"hello");
        assert_eq!(msg.timestamp, Some(1_700_000_000_000));
        assert_eq!(msg.headers.get("trace-id").unwrap(), "abc-123");
    }

    #[test]
    fn test_consumer_message_deserialize() {
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct Event {
            user_id: String,
            action: String,
        }

        let event_json = r#"{"user_id":"u-001","action":"login"}"#;
        let msg = ConsumerMessage {
            topic: "events".to_string(),
            partition: 1,
            offset: 100,
            key: None,
            payload: event_json.as_bytes().to_vec(),
            timestamp: None,
            headers: HashMap::new(),
        };

        let event: Event = msg.deserialize_payload().unwrap();
        assert_eq!(
            event,
            Event {
                user_id: "u-001".to_string(),
                action: "login".to_string(),
            }
        );
    }

    #[test]
    fn test_consumer_message_deserialize_invalid_json() {
        let msg = ConsumerMessage {
            topic: "events".to_string(),
            partition: 0,
            offset: 0,
            key: None,
            payload: b"not json".to_vec(),
            timestamp: None,
            headers: HashMap::new(),
        };

        let result: Result<serde_json::Value, _> = msg.deserialize_payload();
        assert!(result.is_err());
    }

    #[test]
    fn test_consumer_message_payload_str() {
        let msg = ConsumerMessage {
            topic: "test".to_string(),
            partition: 0,
            offset: 0,
            key: None,
            payload: b"hello world".to_vec(),
            timestamp: None,
            headers: HashMap::new(),
        };

        assert_eq!(msg.payload_str().unwrap(), "hello world");
    }

    #[test]
    fn test_consumer_message_payload_str_invalid_utf8() {
        let msg = ConsumerMessage {
            topic: "test".to_string(),
            partition: 0,
            offset: 0,
            key: None,
            payload: vec![0xFF, 0xFE],
            timestamp: None,
            headers: HashMap::new(),
        };

        assert!(msg.payload_str().is_err());
    }
}
