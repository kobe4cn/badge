//! Kafka 辅助工具
//!
//! 提供事件发送和消费功能，用于测试事件驱动流程。

use anyhow::Result;
use rdkafka::Message;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio_stream::StreamExt;
use uuid::Uuid;

/// Kafka Topics
pub mod topics {
    pub const ENGAGEMENT_EVENTS: &str = "badge.engagement.events";
    pub const TRANSACTION_EVENTS: &str = "badge.transaction.events";
    pub const NOTIFICATIONS: &str = "badge.notifications";
    pub const RULE_RELOAD: &str = "badge.rule.reload";
    pub const DLQ: &str = "badge.dlq";
}

/// Kafka 辅助工具
pub struct KafkaHelper {
    producer: FutureProducer,
    consumer: StreamConsumer,
    brokers: String,
}

impl KafkaHelper {
    pub async fn new(brokers: &str) -> Result<Self> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .create()?;

        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("group.id", format!("test-consumer-{}", Uuid::new_v4()))
            .set("enable.partition.eof", "false")
            .set("auto.offset.reset", "latest")
            .create()?;

        Ok(Self {
            producer,
            consumer,
            brokers: brokers.to_string(),
        })
    }

    // ========== 事件发送 ==========

    /// 发送行为事件
    pub async fn send_engagement_event(&self, event: EngagementEvent) -> Result<()> {
        self.send_event(topics::ENGAGEMENT_EVENTS, &event.event_id, &event)
            .await
    }

    /// 发送交易事件
    pub async fn send_transaction_event(&self, event: TransactionEvent) -> Result<()> {
        self.send_event(topics::TRANSACTION_EVENTS, &event.event_id, &event)
            .await
    }

    /// 发送规则刷新消息
    ///
    /// 使用 RuleReloadEvent 格式，与 badge_shared::rules::RuleReloadEvent 一致。
    pub async fn send_rule_reload(&self) -> Result<()> {
        let msg = serde_json::json!({
            "service_group": null,
            "event_type": null,
            "trigger_source": "test-harness",
            "triggered_at": chrono::Utc::now().to_rfc3339()
        });
        self.send_event(topics::RULE_RELOAD, "reload", &msg).await
    }

    /// 发送针对特定服务组的规则刷新消息
    pub async fn send_rule_reload_for_group(&self, service_group: &str) -> Result<()> {
        let msg = serde_json::json!({
            "service_group": service_group,
            "event_type": null,
            "trigger_source": "test-harness",
            "triggered_at": chrono::Utc::now().to_rfc3339()
        });
        self.send_event(topics::RULE_RELOAD, "reload", &msg).await
    }

    /// 通用事件发送
    async fn send_event<T: Serialize>(&self, topic: &str, key: &str, event: &T) -> Result<()> {
        let payload = serde_json::to_string(event)?;

        self.producer
            .send(
                FutureRecord::to(topic).key(key).payload(&payload),
                Duration::from_secs(5),
            )
            .await
            .map_err(|(e, _)| anyhow::anyhow!("发送消息失败: {}", e))?;

        Ok(())
    }

    // ========== 事件消费 ==========

    /// 消费通知消息
    pub async fn consume_notifications(&self) -> Result<Vec<NotificationMessage>> {
        self.consume_messages(topics::NOTIFICATIONS, Duration::from_secs(5))
            .await
    }

    /// 消费死信队列
    pub async fn consume_dlq(&self) -> Result<Vec<serde_json::Value>> {
        self.consume_messages(topics::DLQ, Duration::from_secs(2))
            .await
    }

    /// 通用消息消费
    async fn consume_messages<T: for<'de> Deserialize<'de>>(
        &self,
        topic: &str,
        timeout: Duration,
    ) -> Result<Vec<T>> {
        // 创建新的消费者订阅指定 topic
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", &self.brokers)
            .set("group.id", format!("test-{}-{}", topic, Uuid::new_v4()))
            .set("enable.partition.eof", "false")
            .set("auto.offset.reset", "earliest")
            .create()?;

        consumer.subscribe(&[topic])?;

        let mut messages = Vec::new();
        let mut stream = consumer.stream();

        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            tokio::select! {
                msg = stream.next() => {
                    match msg {
                        Some(Ok(m)) => {
                            if let Some(payload) = m.payload() {
                                if let Ok(parsed) = serde_json::from_slice(payload) {
                                    messages.push(parsed);
                                }
                            }
                        }
                        _ => break,
                    }
                }
                _ = tokio::time::sleep_until(deadline) => {
                    break;
                }
            }
        }

        Ok(messages)
    }

    /// 清空 topic 中的消息（通过消费掉）
    pub async fn drain_topic(&self, topic: &str) -> Result<()> {
        let _: Vec<serde_json::Value> = self
            .consume_messages(topic, Duration::from_millis(500))
            .await?;
        Ok(())
    }
}

// ========== 事件类型定义 ==========

/// 行为事件
///
/// 采用 camelCase 序列化格式，与 badge_shared::events::EventPayload 保持一致。
/// event_type 使用 SCREAMING_SNAKE_CASE 格式（如 CHECK_IN）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngagementEvent {
    pub event_id: String,
    pub event_type: String,
    pub user_id: String,
    pub timestamp: String,
    pub data: serde_json::Value,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

impl EngagementEvent {
    pub fn checkin(user_id: &str) -> Self {
        Self {
            event_id: Uuid::now_v7().to_string(),
            event_type: "CHECK_IN".to_string(),
            user_id: user_id.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            data: serde_json::json!({
                "consecutive_days": 1
            }),
            source: "test-harness".to_string(),
            trace_id: None,
        }
    }

    /// 创建带指定连续签到天数的签到事件
    pub fn checkin_with_days(user_id: &str, consecutive_days: i32) -> Self {
        Self {
            event_id: Uuid::now_v7().to_string(),
            event_type: "CHECK_IN".to_string(),
            user_id: user_id.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            data: serde_json::json!({
                "consecutive_days": consecutive_days
            }),
            source: "test-harness".to_string(),
            trace_id: None,
        }
    }

    pub fn share(user_id: &str, content_id: &str) -> Self {
        Self {
            event_id: Uuid::now_v7().to_string(),
            event_type: "SHARE".to_string(),
            user_id: user_id.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            data: serde_json::json!({
                "content_id": content_id,
                "platform": "wechat"
            }),
            source: "test-harness".to_string(),
            trace_id: None,
        }
    }

    pub fn page_view(user_id: &str, page_id: &str) -> Self {
        Self {
            event_id: Uuid::now_v7().to_string(),
            event_type: "PAGE_VIEW".to_string(),
            user_id: user_id.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            data: serde_json::json!({
                "page_id": page_id,
                "duration_ms": 5000
            }),
            source: "test-harness".to_string(),
            trace_id: None,
        }
    }
}

/// 交易事件
///
/// 采用 camelCase 序列化格式，与 badge_shared::events::EventPayload 保持一致。
/// event_type 使用 SCREAMING_SNAKE_CASE 格式（如 PURCHASE）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionEvent {
    pub event_id: String,
    pub event_type: String,
    pub user_id: String,
    pub timestamp: String,
    pub data: serde_json::Value,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

impl TransactionEvent {
    /// 创建购买事件
    ///
    /// `total_amount` 默认等于 `amount`，表示单笔购买场景下累计消费等于订单金额。
    /// 如需指定不同的累计金额，使用 `purchase_with_total`。
    pub fn purchase(user_id: &str, order_id: &str, amount: i64) -> Self {
        Self {
            event_id: Uuid::now_v7().to_string(),
            event_type: "PURCHASE".to_string(),
            user_id: user_id.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            data: serde_json::json!({
                "amount": amount,
                "order_id": order_id,
                "total_amount": amount,
                "currency": "CNY",
                "items": []
            }),
            source: "test-harness".to_string(),
            trace_id: None,
        }
    }

    /// 创建带累计金额的购买事件
    ///
    /// 用于测试累计消费规则场景，`total_amount` 表示用户的历史累计消费金额。
    pub fn purchase_with_total(
        user_id: &str,
        order_id: &str,
        amount: i64,
        total_amount: i64,
    ) -> Self {
        Self {
            event_id: Uuid::now_v7().to_string(),
            event_type: "PURCHASE".to_string(),
            user_id: user_id.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            data: serde_json::json!({
                "amount": amount,
                "order_id": order_id,
                "total_amount": total_amount,
                "currency": "CNY",
                "items": []
            }),
            source: "test-harness".to_string(),
            trace_id: None,
        }
    }

    pub fn refund(user_id: &str, order_id: &str, original_order_id: &str, amount: i64) -> Self {
        Self {
            event_id: Uuid::now_v7().to_string(),
            event_type: "REFUND".to_string(),
            user_id: user_id.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            data: serde_json::json!({
                "order_id": order_id,
                "original_order_id": original_order_id,
                "refund_amount": amount,
                "refund_reason": "test refund"
            }),
            source: "test-harness".to_string(),
            trace_id: None,
        }
    }

    /// 带有徽章撤销列表的退款事件
    pub fn refund_with_badges(
        user_id: &str,
        order_id: &str,
        original_order_id: &str,
        amount: i64,
        badge_ids: &[i64],
    ) -> Self {
        Self {
            event_id: Uuid::now_v7().to_string(),
            event_type: "REFUND".to_string(),
            user_id: user_id.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            data: serde_json::json!({
                "order_id": order_id,
                "original_order_id": original_order_id,
                "refund_amount": amount,
                "refund_reason": "test refund",
                "badge_ids_to_revoke": badge_ids
            }),
            source: "test-harness".to_string(),
            trace_id: None,
        }
    }
}

/// 通知消息
///
/// 使用 camelCase 序列化格式，与 badge_shared::events::NotificationEvent 保持一致。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationMessage {
    pub notification_id: String,
    pub notification_type: String,
    pub user_id: String,
    pub title: String,
    pub body: String,
    pub channels: Vec<String>,
    pub data: serde_json::Value,
}

impl Default for TransactionEvent {
    fn default() -> Self {
        Self::purchase("test_user", "test_order", 100)
    }
}

impl TransactionEvent {
    /// 创建带自定义 order_id 的订单取消事件
    pub fn order_cancel(user_id: &str, order_id: &str, original_order_id: &str) -> Self {
        Self {
            event_id: Uuid::now_v7().to_string(),
            event_type: "ORDER_CANCEL".to_string(),
            user_id: user_id.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            data: serde_json::json!({
                "order_id": order_id,
                "original_order_id": original_order_id,
                "cancel_reason": "test cancel"
            }),
            source: "test-harness".to_string(),
            trace_id: None,
        }
    }
}
