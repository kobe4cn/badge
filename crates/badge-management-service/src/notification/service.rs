//! 通知服务
//!
//! 提供统一的通知发送接口，支持多渠道并行发送。
//!
//! ## 设计说明
//!
//! - **异步发送**：通知发送不阻塞主业务流程
//! - **多渠道并行**：各渠道独立发送，互不影响
//! - **部分失败容忍**：单渠道失败不影响其他渠道
//! - **Kafka 集成**：发送结果通过 Kafka 传递给下游消费者

use std::sync::Arc;
use std::time::Instant;

use badge_shared::events::{NotificationChannel as ChannelType, NotificationEvent};
use badge_shared::kafka::{topics, KafkaProducer};
use chrono::Utc;
use futures::future::join_all;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

use super::channels::{
    AppPushChannel, EmailChannel, NotificationChannel, SmsChannel, WeChatChannel,
};
use super::template::TemplateEngine;
use super::types::{ChannelResult, Notification, NotificationContext, NotificationResult};
use crate::error::{BadgeError, Result};

/// 通知服务
///
/// 管理通知渠道和模板引擎，提供统一的发送接口。
pub struct NotificationService {
    /// 已注册的通知渠道
    channels: Vec<Arc<dyn NotificationChannel>>,
    /// 模板引擎
    template_engine: Arc<TemplateEngine>,
    /// Kafka 生产者（用于发送通知事件）
    kafka_producer: Option<Arc<KafkaProducer>>,
    /// 是否启用异步发送
    async_enabled: RwLock<bool>,
}

impl NotificationService {
    /// 创建通知服务
    pub fn new(template_engine: Arc<TemplateEngine>) -> Self {
        Self {
            channels: Vec::new(),
            template_engine,
            kafka_producer: None,
            async_enabled: RwLock::new(true),
        }
    }

    /// 使用默认配置创建
    ///
    /// 包含所有默认渠道和模板
    pub fn with_defaults() -> Self {
        let mut service = Self::new(Arc::new(TemplateEngine::with_defaults()));
        service.register_default_channels();
        service
    }

    /// 设置 Kafka 生产者
    pub fn with_kafka_producer(mut self, producer: Arc<KafkaProducer>) -> Self {
        self.kafka_producer = Some(producer);
        self
    }

    /// 注册默认渠道
    fn register_default_channels(&mut self) {
        self.register_channel(Arc::new(AppPushChannel::with_defaults()));
        self.register_channel(Arc::new(SmsChannel::with_defaults()));
        self.register_channel(Arc::new(EmailChannel::with_defaults()));
        self.register_channel(Arc::new(WeChatChannel::with_defaults()));
    }

    /// 注册通知渠道
    pub fn register_channel(&mut self, channel: Arc<dyn NotificationChannel>) {
        info!(
            channel_type = ?channel.channel_type(),
            channel_name = channel.name(),
            "注册通知渠道"
        );
        self.channels.push(channel);
    }

    /// 获取已注册的渠道
    pub fn get_channel(&self, channel_type: ChannelType) -> Option<&Arc<dyn NotificationChannel>> {
        self.channels
            .iter()
            .find(|c| c.channel_type() == channel_type)
    }

    /// 获取所有已注册的渠道类型
    pub fn registered_channel_types(&self) -> Vec<ChannelType> {
        self.channels.iter().map(|c| c.channel_type()).collect()
    }

    /// 发送通知
    ///
    /// 根据通知配置的渠道列表，并行发送到各个渠道。
    /// 会先使用模板引擎渲染通知内容。
    #[instrument(
        skip(self, notification),
        fields(
            notification_id = %notification.notification_id,
            user_id = %notification.user_id,
            notification_type = ?notification.notification_type,
            channels = ?notification.channels
        )
    )]
    pub async fn send(&self, notification: Notification) -> Result<NotificationResult> {
        let start = Instant::now();

        info!("开始发送通知");

        // 渲染通知内容
        let rendered = self.render_notification(&notification);

        // 筛选出需要发送的渠道
        let target_channels: Vec<_> = self
            .channels
            .iter()
            .filter(|c| notification.channels.contains(&c.channel_type()))
            .cloned()
            .collect();

        if target_channels.is_empty() {
            warn!("没有匹配的渠道可用");
            return Ok(NotificationResult::success(
                notification.notification_id.clone(),
                vec![],
                start.elapsed().as_millis() as u64,
            ));
        }

        debug!(
            target_channel_count = target_channels.len(),
            "找到匹配的渠道"
        );

        // 并行发送到所有渠道
        let send_futures: Vec<_> = target_channels
            .iter()
            .map(|channel| {
                let channel = channel.clone();
                let notification = rendered.clone();
                async move {
                    let result = channel.send(&notification).await;
                    (channel.channel_type(), result)
                }
            })
            .collect();

        let results = join_all(send_futures).await;

        // 收集发送结果
        let channel_results: Vec<ChannelResult> = results
            .into_iter()
            .map(|(channel_type, result)| match result {
                Ok(r) => r,
                Err(e) => {
                    error!(
                        channel = ?channel_type,
                        error = %e,
                        "渠道发送异常"
                    );
                    ChannelResult::failed(channel_type, e.to_string(), 0)
                }
            })
            .collect();

        let duration_ms = start.elapsed().as_millis() as u64;
        let notification_result = NotificationResult::success(
            notification.notification_id.clone(),
            channel_results,
            duration_ms,
        );

        // 记录发送结果
        self.log_result(&notification, &notification_result);

        // 发送到 Kafka（如果配置了）
        if let Err(e) = self
            .publish_notification_event(&rendered, &notification_result)
            .await
        {
            warn!(error = %e, "发布通知事件到 Kafka 失败");
        }

        Ok(notification_result)
    }

    /// 批量发送通知
    ///
    /// 对每个通知独立发送，单个失败不影响其他通知
    #[instrument(skip(self, notifications), fields(count = notifications.len()))]
    pub async fn send_batch(
        &self,
        notifications: Vec<Notification>,
    ) -> Vec<Result<NotificationResult>> {
        info!("开始批量发送通知");

        let send_futures: Vec<_> = notifications
            .into_iter()
            .map(|notification| self.send(notification))
            .collect();

        join_all(send_futures).await
    }

    /// 异步发送通知（fire-and-forget）
    ///
    /// 在后台任务中发送通知，不阻塞调用者。
    /// 适用于主业务流程中的通知发送。
    pub fn send_async(&self, notification: Notification) {
        let service = self.clone_for_async();

        tokio::spawn(async move {
            if let Err(e) = service.send(notification).await {
                error!(error = %e, "异步发送通知失败");
            }
        });
    }

    /// 克隆服务用于异步发送
    fn clone_for_async(&self) -> NotificationServiceAsync {
        NotificationServiceAsync {
            channels: self.channels.clone(),
            template_engine: self.template_engine.clone(),
            kafka_producer: self.kafka_producer.clone(),
        }
    }

    /// 渲染通知内容
    fn render_notification(&self, notification: &Notification) -> Notification {
        let context = NotificationContext::from(notification.variables.clone());

        let (rendered_title, rendered_body) = self.template_engine.render_notification(
            &notification.notification_type,
            &notification.title,
            &notification.body,
            &context,
        );

        let mut rendered = notification.clone();
        rendered.title = rendered_title;
        rendered.body = rendered_body;
        rendered
    }

    /// 记录发送结果
    fn log_result(&self, notification: &Notification, result: &NotificationResult) {
        let success_count = result.success_count();
        let failure_count = result.failure_count();
        let total = result.channel_results.len();

        if result.success {
            info!(
                notification_id = %notification.notification_id,
                success_count,
                total,
                duration_ms = result.duration_ms,
                "通知发送完成（全部成功）"
            );
        } else if result.is_partial_success() {
            warn!(
                notification_id = %notification.notification_id,
                success_count,
                failure_count,
                total,
                duration_ms = result.duration_ms,
                "通知发送完成（部分成功）"
            );
        } else {
            error!(
                notification_id = %notification.notification_id,
                failure_count,
                total,
                duration_ms = result.duration_ms,
                "通知发送完成（全部失败）"
            );
        }
    }

    /// 发布通知事件到 Kafka
    async fn publish_notification_event(
        &self,
        notification: &Notification,
        _result: &NotificationResult,
    ) -> Result<()> {
        let Some(ref producer) = self.kafka_producer else {
            debug!("未配置 Kafka 生产者，跳过发布通知事件");
            return Ok(());
        };

        let event = NotificationEvent {
            notification_id: notification.notification_id.clone(),
            user_id: notification.user_id.clone(),
            notification_type: notification.notification_type.clone(),
            title: notification.title.clone(),
            body: notification.body.clone(),
            data: serde_json::Value::Object(
                notification
                    .data
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            ),
            channels: notification.channels.clone(),
            created_at: Utc::now(),
        };

        producer
            .send_json(
                topics::BADGE_NOTIFICATIONS,
                &notification.notification_id,
                &event,
            )
            .await
            .map_err(|e| BadgeError::Internal(format!("发送 Kafka 消息失败: {e}")))?;

        debug!(
            notification_id = %notification.notification_id,
            "通知事件已发布到 Kafka"
        );

        Ok(())
    }

    /// 设置是否启用异步发送
    pub async fn set_async_enabled(&self, enabled: bool) {
        let mut guard = self.async_enabled.write().await;
        *guard = enabled;
    }

    /// 检查是否启用异步发送
    pub async fn is_async_enabled(&self) -> bool {
        *self.async_enabled.read().await
    }
}

/// 用于异步发送的轻量级服务结构
struct NotificationServiceAsync {
    channels: Vec<Arc<dyn NotificationChannel>>,
    template_engine: Arc<TemplateEngine>,
    kafka_producer: Option<Arc<KafkaProducer>>,
}

impl NotificationServiceAsync {
    async fn send(&self, notification: Notification) -> Result<NotificationResult> {
        let start = Instant::now();

        // 渲染通知内容
        let context = NotificationContext::from(notification.variables.clone());
        let (rendered_title, rendered_body) = self.template_engine.render_notification(
            &notification.notification_type,
            &notification.title,
            &notification.body,
            &context,
        );

        let mut rendered = notification.clone();
        rendered.title = rendered_title;
        rendered.body = rendered_body;

        // 筛选渠道
        let target_channels: Vec<_> = self
            .channels
            .iter()
            .filter(|c| notification.channels.contains(&c.channel_type()))
            .cloned()
            .collect();

        // 并行发送
        let send_futures: Vec<_> = target_channels
            .iter()
            .map(|channel| {
                let channel = channel.clone();
                let notification = rendered.clone();
                async move {
                    let result = channel.send(&notification).await;
                    (channel.channel_type(), result)
                }
            })
            .collect();

        let results = join_all(send_futures).await;

        let channel_results: Vec<ChannelResult> = results
            .into_iter()
            .map(|(channel_type, result)| match result {
                Ok(r) => r,
                Err(e) => ChannelResult::failed(channel_type, e.to_string(), 0),
            })
            .collect();

        let duration_ms = start.elapsed().as_millis() as u64;
        let notification_result = NotificationResult::success(
            notification.notification_id.clone(),
            channel_results,
            duration_ms,
        );

        // 发布到 Kafka
        if let Some(ref producer) = self.kafka_producer {
            let event = NotificationEvent {
                notification_id: rendered.notification_id.clone(),
                user_id: rendered.user_id.clone(),
                notification_type: rendered.notification_type.clone(),
                title: rendered.title.clone(),
                body: rendered.body.clone(),
                data: serde_json::Value::Object(
                    rendered
                        .data
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                ),
                channels: rendered.channels.clone(),
                created_at: Utc::now(),
            };

            let _ = producer
                .send_json(
                    topics::BADGE_NOTIFICATIONS,
                    &rendered.notification_id,
                    &event,
                )
                .await;
        }

        Ok(notification_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use badge_shared::events::NotificationType;

    fn create_test_notification(user_id: &str) -> Notification {
        Notification::new(
            user_id,
            NotificationType::BadgeGranted,
            "测试标题",
            "测试内容",
        )
        .with_channels(vec![ChannelType::AppPush])
    }

    #[tokio::test]
    async fn test_notification_service_creation() {
        let service = NotificationService::with_defaults();

        let channel_types = service.registered_channel_types();
        assert!(channel_types.contains(&ChannelType::AppPush));
        assert!(channel_types.contains(&ChannelType::Sms));
        assert!(channel_types.contains(&ChannelType::Email));
        assert!(channel_types.contains(&ChannelType::WeChat));
    }

    #[tokio::test]
    async fn test_send_single_channel() {
        let service = NotificationService::with_defaults();
        let notification = create_test_notification("user-123");

        let result = service.send(notification).await.unwrap();

        assert_eq!(result.channel_results.len(), 1);
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_send_multiple_channels() {
        let service = NotificationService::with_defaults();
        let notification = Notification::new(
            "user-123",
            NotificationType::BadgeGranted,
            "测试标题",
            "测试内容",
        )
        .with_channels(vec![ChannelType::AppPush, ChannelType::Sms, ChannelType::Email]);

        let result = service.send(notification).await.unwrap();

        assert_eq!(result.channel_results.len(), 3);
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_send_partial_failure() {
        let service = NotificationService::with_defaults();

        // 使用会触发某些渠道失败的用户 ID
        let notification = Notification::new(
            "fail_sms_user",
            NotificationType::BadgeGranted,
            "测试标题",
            "测试内容",
        )
        .with_channels(vec![ChannelType::AppPush, ChannelType::Sms]);

        let result = service.send(notification).await.unwrap();

        assert_eq!(result.channel_results.len(), 2);
        assert!(result.is_partial_success());
        assert_eq!(result.success_count(), 1);
        assert_eq!(result.failure_count(), 1);
    }

    #[tokio::test]
    async fn test_send_with_template_rendering() {
        let service = NotificationService::with_defaults();
        let notification = Notification::new(
            "user-123",
            NotificationType::BadgeGranted,
            "恭喜获得新徽章！",
            "您已获得「{{badge_name}}」徽章！",
        )
        .with_variable("badge_name", "首次购物")
        .with_channels(vec![ChannelType::AppPush]);

        let result = service.send(notification).await.unwrap();

        assert!(result.success);
    }

    #[tokio::test]
    async fn test_send_batch() {
        let service = NotificationService::with_defaults();

        let notifications: Vec<Notification> = (1..=3)
            .map(|i| create_test_notification(&format!("user-{}", i)))
            .collect();

        let results = service.send_batch(notifications).await;

        assert_eq!(results.len(), 3);
        for result in results {
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_send_no_matching_channels() {
        let service = NotificationService::new(Arc::new(TemplateEngine::with_defaults()));
        // 没有注册任何渠道

        let notification = create_test_notification("user-123");
        let result = service.send(notification).await.unwrap();

        assert!(result.channel_results.is_empty());
    }

    #[tokio::test]
    async fn test_get_channel() {
        let service = NotificationService::with_defaults();

        assert!(service.get_channel(ChannelType::AppPush).is_some());
        assert!(service.get_channel(ChannelType::Sms).is_some());
    }

    #[tokio::test]
    async fn test_async_enabled_setting() {
        let service = NotificationService::with_defaults();

        assert!(service.is_async_enabled().await);

        service.set_async_enabled(false).await;
        assert!(!service.is_async_enabled().await);

        service.set_async_enabled(true).await;
        assert!(service.is_async_enabled().await);
    }
}
