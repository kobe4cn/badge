//! 批量事件发送器
//!
//! 提供将事件发送到 Kafka 的能力，支持单条和批量发送。
//! 自动根据事件类型路由到正确的 topic。

use badge_shared::error::BadgeError;
use badge_shared::events::{EventPayload, EventType};
use badge_shared::kafka::{KafkaProducer, topics};
use tracing::{debug, error};

/// 批量事件发送器
///
/// 封装 KafkaProducer，提供事件级别的发送接口。
/// 根据事件类型自动选择目标 topic，简化调用方逻辑。
pub struct BatchEventSender {
    producer: KafkaProducer,
}

impl BatchEventSender {
    /// 创建批量事件发送器
    pub fn new(producer: KafkaProducer) -> Self {
        Self { producer }
    }

    /// 发送单个事件到 Kafka
    ///
    /// 根据事件类型自动路由到对应的 topic：
    /// - 交易类事件（Purchase、Refund、OrderCancel）发送到 transaction topic
    /// - 其他行为类事件发送到 engagement topic
    pub async fn send(&self, event: &EventPayload) -> Result<(), BadgeError> {
        let topic = Self::select_topic(&event.event_type);

        self.producer
            .send_json(topic, &event.event_id, event)
            .await?;

        debug!(
            event_id = %event.event_id,
            event_type = ?event.event_type,
            topic,
            "事件已发送到 Kafka"
        );

        Ok(())
    }

    /// 批量发送事件
    ///
    /// 逐条发送所有事件，记录成功/失败数量。
    /// 单条失败不影响其他事件的发送，确保最大化投递成功率。
    pub async fn send_batch(&self, events: &[EventPayload]) -> BatchSendResult {
        let total = events.len();
        let mut success = 0;
        let mut failed = 0;
        let mut errors = Vec::new();

        for event in events {
            match self.send(event).await {
                Ok(()) => success += 1,
                Err(e) => {
                    failed += 1;
                    let error_msg = format!(
                        "事件 {} ({:?}) 发送失败: {}",
                        event.event_id, event.event_type, e
                    );
                    error!("{}", error_msg);
                    errors.push(error_msg);
                }
            }
        }

        BatchSendResult {
            total,
            success,
            failed,
            errors,
        }
    }

    /// 根据事件类型选择目标 topic
    ///
    /// 交易类事件需要更严格的处理（如幂等性、回滚），因此独立 topic。
    /// 行为类事件量大但处理相对简单，统一到一个 topic。
    fn select_topic(event_type: &EventType) -> &'static str {
        match event_type {
            EventType::Purchase | EventType::Refund | EventType::OrderCancel => {
                topics::TRANSACTION_EVENTS
            }
            _ => topics::ENGAGEMENT_EVENTS,
        }
    }
}

/// 批量发送结果
///
/// 记录批量发送操作的详细结果，便于调用方了解发送情况。
#[derive(Debug, Clone)]
pub struct BatchSendResult {
    /// 总事件数
    pub total: usize,
    /// 成功发送数
    pub success: usize,
    /// 发送失败数
    pub failed: usize,
    /// 错误信息列表
    pub errors: Vec<String>,
}

impl BatchSendResult {
    /// 是否全部成功
    pub fn is_all_success(&self) -> bool {
        self.failed == 0
    }

    /// 成功率（百分比）
    pub fn success_rate(&self) -> f64 {
        if self.total == 0 {
            100.0
        } else {
            (self.success as f64 / self.total as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_topic_transaction() {
        assert_eq!(
            BatchEventSender::select_topic(&EventType::Purchase),
            topics::TRANSACTION_EVENTS
        );
        assert_eq!(
            BatchEventSender::select_topic(&EventType::Refund),
            topics::TRANSACTION_EVENTS
        );
        assert_eq!(
            BatchEventSender::select_topic(&EventType::OrderCancel),
            topics::TRANSACTION_EVENTS
        );
    }

    #[test]
    fn test_select_topic_engagement() {
        assert_eq!(
            BatchEventSender::select_topic(&EventType::CheckIn),
            topics::ENGAGEMENT_EVENTS
        );
        assert_eq!(
            BatchEventSender::select_topic(&EventType::PageView),
            topics::ENGAGEMENT_EVENTS
        );
        assert_eq!(
            BatchEventSender::select_topic(&EventType::Share),
            topics::ENGAGEMENT_EVENTS
        );
        assert_eq!(
            BatchEventSender::select_topic(&EventType::Review),
            topics::ENGAGEMENT_EVENTS
        );
        assert_eq!(
            BatchEventSender::select_topic(&EventType::Registration),
            topics::ENGAGEMENT_EVENTS
        );
    }

    #[test]
    fn test_batch_send_result_all_success() {
        let result = BatchSendResult {
            total: 10,
            success: 10,
            failed: 0,
            errors: vec![],
        };

        assert!(result.is_all_success());
        assert_eq!(result.success_rate(), 100.0);
    }

    #[test]
    fn test_batch_send_result_partial_success() {
        let result = BatchSendResult {
            total: 10,
            success: 8,
            failed: 2,
            errors: vec!["error1".to_string(), "error2".to_string()],
        };

        assert!(!result.is_all_success());
        assert_eq!(result.success_rate(), 80.0);
    }

    #[test]
    fn test_batch_send_result_empty() {
        let result = BatchSendResult {
            total: 0,
            success: 0,
            failed: 0,
            errors: vec![],
        };

        assert!(result.is_all_success());
        assert_eq!(result.success_rate(), 100.0);
    }
}
