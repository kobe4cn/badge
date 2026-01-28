//! 通知消费者
//!
//! 从 Kafka 消费通知事件，分发到对应渠道的发送器执行推送。
//! 多渠道发送并行执行，单个渠道失败不影响其他渠道。

use std::collections::HashMap;
use std::sync::Arc;

use badge_shared::config::AppConfig;
use badge_shared::events::{NotificationChannel, NotificationEvent};
use badge_shared::kafka::{KafkaConsumer, KafkaProducer, topics};
use tokio::sync::watch;
use tracing::{error, info, warn};

use crate::error::NotificationError;
use crate::sender::{NotificationSender, SendResult};
use crate::templates::NotificationTemplateEngine;

/// 通知消费者
///
/// 从 Kafka 消费通知事件，分发到对应渠道的发送器执行推送。
/// 多渠道发送并行执行，单个渠道失败不影响其他渠道。
pub struct NotificationConsumer {
    consumer: KafkaConsumer,
    senders: HashMap<NotificationChannel, Arc<dyn NotificationSender>>,
    template_engine: NotificationTemplateEngine,
    /// 发送失败的通知投递到死信队列，供后续排查或重试
    producer: KafkaProducer,
}

impl NotificationConsumer {
    pub fn new(
        config: &AppConfig,
        senders: HashMap<NotificationChannel, Arc<dyn NotificationSender>>,
        producer: KafkaProducer,
    ) -> Result<Self, NotificationError> {
        let consumer = KafkaConsumer::new(&config.kafka, Some("notifications"))?;
        let template_engine = NotificationTemplateEngine::new();
        Ok(Self {
            consumer,
            senders,
            template_engine,
            producer,
        })
    }

    /// 启动消费循环，直到收到 shutdown 信号
    pub async fn run(self, shutdown: watch::Receiver<bool>) -> Result<(), NotificationError> {
        self.consumer.subscribe(&[topics::BADGE_NOTIFICATIONS])?;

        info!(topic = topics::BADGE_NOTIFICATIONS, "通知消费者已启动");

        let senders = self.senders;
        let _template_engine = self.template_engine;
        let producer = self.producer;

        self.consumer
            .start(shutdown, |msg| {
                let senders = &senders;
                let producer = &producer;
                async move {
                    if let Err(e) = handle_message(senders, producer, &msg).await {
                        error!(
                            error = %e,
                            topic = %msg.topic,
                            partition = msg.partition,
                            offset = msg.offset,
                            "处理通知事件失败"
                        );
                    }
                    Ok(())
                }
            })
            .await;

        info!("通知消费者已停止");
        Ok(())
    }
}

/// 处理单条 Kafka 通知消息
///
/// 拆分为独立函数而非方法，便于在测试中直接调用而无需构造完整的 Consumer。
async fn handle_message(
    senders: &HashMap<NotificationChannel, Arc<dyn NotificationSender>>,
    producer: &KafkaProducer,
    msg: &badge_shared::kafka::ConsumerMessage,
) -> Result<(), NotificationError> {
    let notification: NotificationEvent = serde_json::from_slice(&msg.payload)
        .map_err(|e| NotificationError::DeserializationFailed(e.to_string()))?;

    info!(
        notification_id = %notification.notification_id,
        user_id = %notification.user_id,
        notification_type = ?notification.notification_type,
        channels = ?notification.channels,
        "收到通知事件"
    );

    let results = handle_notification(senders, &notification).await;

    // 检查发送结果，对失败的渠道记录日志
    let mut all_success = true;
    for result in &results {
        if !result.success {
            all_success = false;
            warn!(
                notification_id = %notification.notification_id,
                channel = ?result.channel,
                error = ?result.error,
                "渠道发送失败"
            );
        }
    }

    // 所有渠道都失败时投递到死信队列，便于后续重试
    if !all_success && results.iter().all(|r| !r.success) {
        send_to_dlq(producer, &notification).await;
    }

    info!(
        notification_id = %notification.notification_id,
        total_channels = results.len(),
        success_count = results.iter().filter(|r| r.success).count(),
        "通知事件处理完成"
    );

    Ok(())
}

/// 按通知指定的渠道列表并行发送
///
/// 使用 futures::future::join_all 并行执行所有渠道的发送操作。
/// 单个渠道的失败不会阻塞其他渠道，确保通知尽可能到达用户。
pub async fn handle_notification(
    senders: &HashMap<NotificationChannel, Arc<dyn NotificationSender>>,
    notification: &NotificationEvent,
) -> Vec<SendResult> {
    let futures: Vec<_> = notification
        .channels
        .iter()
        .map(|channel| {
            let senders = senders.clone();
            let notification = notification.clone();
            let channel = channel.clone();
            async move {
                if let Some(sender) = senders.get(&channel) {
                    match sender.send(&notification).await {
                        Ok(result) => result,
                        Err(e) => {
                            error!(
                                channel = ?channel,
                                error = %e,
                                "发送器执行异常"
                            );
                            SendResult {
                                success: false,
                                channel,
                                message_id: None,
                                error: Some(e.to_string()),
                            }
                        }
                    }
                } else {
                    // 没有注册对应渠道的发送器
                    warn!(
                        channel = ?channel,
                        "未找到该渠道的发送器，跳过"
                    );
                    SendResult {
                        success: false,
                        channel,
                        message_id: None,
                        error: Some("发送器未注册".to_string()),
                    }
                }
            }
        })
        .collect();

    futures::future::join_all(futures).await
}

/// 将处理失败的通知发送到死信队列
async fn send_to_dlq(producer: &KafkaProducer, notification: &NotificationEvent) {
    if let Err(e) = producer
        .send_json(
            topics::DEAD_LETTER_QUEUE,
            &notification.notification_id,
            notification,
        )
        .await
    {
        error!(
            notification_id = %notification.notification_id,
            error = %e,
            "发送到死信队列失败，通知可能丢失"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use badge_shared::events::NotificationType;
    use badge_shared::kafka::ConsumerMessage;
    use chrono::Utc;

    /// 构造测试用的通知事件
    fn make_test_notification() -> NotificationEvent {
        NotificationEvent {
            notification_id: "notif-test-001".to_string(),
            user_id: "user-001".to_string(),
            notification_type: NotificationType::BadgeGranted,
            title: "恭喜获得新徽章".to_string(),
            body: "您已获得「首次购物」徽章！".to_string(),
            data: serde_json::json!({
                "badge_id": 42,
                "badge_name": "首次购物"
            }),
            channels: vec![NotificationChannel::AppPush, NotificationChannel::WeChat],
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_notification_deserialize() {
        let notification = make_test_notification();
        let payload = serde_json::to_vec(&notification).expect("序列化测试通知失败");

        let msg = ConsumerMessage {
            topic: topics::BADGE_NOTIFICATIONS.to_string(),
            partition: 0,
            offset: 1,
            key: Some(notification.notification_id.clone()),
            payload,
            timestamp: Some(Utc::now().timestamp_millis()),
            headers: HashMap::new(),
        };

        let deserialized: NotificationEvent =
            serde_json::from_slice(&msg.payload).expect("反序列化通知失败");

        assert_eq!(deserialized.notification_id, "notif-test-001");
        assert_eq!(deserialized.user_id, "user-001");
        assert_eq!(
            deserialized.notification_type,
            NotificationType::BadgeGranted
        );
        assert_eq!(deserialized.channels.len(), 2);
        assert_eq!(deserialized.channels[0], NotificationChannel::AppPush);
        assert_eq!(deserialized.channels[1], NotificationChannel::WeChat);
    }

    #[test]
    fn test_notification_deserialize_invalid_json() {
        let invalid_payload = b"not valid json";
        let result: Result<NotificationEvent, _> = serde_json::from_slice(invalid_payload);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_notification_parallel_send() {
        // 注册两个渠道的发送器
        let mut senders: HashMap<NotificationChannel, Arc<dyn NotificationSender>> = HashMap::new();
        senders.insert(
            NotificationChannel::AppPush,
            Arc::new(crate::sender::AppPushSender),
        );
        senders.insert(
            NotificationChannel::WeChat,
            Arc::new(crate::sender::WeChatSender),
        );

        let notification = make_test_notification();
        let results = handle_notification(&senders, &notification).await;

        // 两个渠道都应成功
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.success));
    }

    #[tokio::test]
    async fn test_handle_notification_missing_sender() {
        // 只注册 AppPush 发送器，但通知需要 AppPush + WeChat
        let mut senders: HashMap<NotificationChannel, Arc<dyn NotificationSender>> = HashMap::new();
        senders.insert(
            NotificationChannel::AppPush,
            Arc::new(crate::sender::AppPushSender),
        );

        let notification = make_test_notification();
        let results = handle_notification(&senders, &notification).await;

        assert_eq!(results.len(), 2);

        // AppPush 应成功
        let app_push = results
            .iter()
            .find(|r| r.channel == NotificationChannel::AppPush)
            .expect("应有 AppPush 结果");
        assert!(app_push.success);

        // WeChat 因发送器未注册应失败
        let wechat = results
            .iter()
            .find(|r| r.channel == NotificationChannel::WeChat)
            .expect("应有 WeChat 结果");
        assert!(!wechat.success);
        assert!(wechat.error.is_some());
    }
}
