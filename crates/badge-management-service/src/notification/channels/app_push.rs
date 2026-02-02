//! App Push 通知渠道
//!
//! 通过 FCM/APNs 等推送服务发送 App 内通知。
//! 当前为模拟实现，生产环境需要接入真实的推送服务。

use std::time::Instant;

use async_trait::async_trait;
use tracing::{debug, info, warn};
use uuid::Uuid;

use badge_shared::events::NotificationChannel as ChannelType;

use super::{ChannelConfig, ChannelResult, NotificationChannel};
use crate::error::Result;
use crate::notification::types::Notification;

/// App Push 通知渠道
///
/// 负责向用户的移动设备发送推送通知。
/// 当前为模拟实现，直接返回成功结果。
pub struct AppPushChannel {
    config: ChannelConfig,
}

impl AppPushChannel {
    pub fn new(config: ChannelConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建
    pub fn with_defaults() -> Self {
        Self::new(ChannelConfig::new(true).with_timeout(3000))
    }

    /// 模拟发送推送（生产环境应接入真实推送服务）
    async fn send_push(&self, notification: &Notification) -> Result<String> {
        // 模拟网络延迟
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        debug!(
            notification_id = %notification.notification_id,
            user_id = %notification.user_id,
            title = %notification.title,
            "App Push 发送中..."
        );

        // 模拟随机失败（用于测试部分失败场景）
        // 生产环境应移除此逻辑
        #[cfg(test)]
        if notification.user_id.contains("fail_push") {
            return Err(crate::error::BadgeError::Internal(
                "模拟 App Push 发送失败".to_string(),
            ));
        }

        // 生成模拟的消息 ID
        let message_id = format!("push_{}", Uuid::new_v4());

        info!(
            notification_id = %notification.notification_id,
            message_id = %message_id,
            "App Push 发送成功"
        );

        Ok(message_id)
    }
}

#[async_trait]
impl NotificationChannel for AppPushChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::AppPush
    }

    fn name(&self) -> &str {
        "App Push"
    }

    async fn is_available(&self, notification: &Notification) -> bool {
        if !self.config.enabled {
            warn!(
                notification_id = %notification.notification_id,
                "App Push 渠道已禁用"
            );
            return false;
        }

        // 实际实现应检查用户是否有有效的推送 token
        // 当前模拟实现直接返回 true
        true
    }

    async fn send(&self, notification: &Notification) -> Result<ChannelResult> {
        let start = Instant::now();

        if !self.is_available(notification).await {
            return Ok(ChannelResult::skipped(
                self.channel_type(),
                "渠道不可用或已禁用",
            ));
        }

        match self.send_push(notification).await {
            Ok(message_id) => Ok(ChannelResult::success(
                self.channel_type(),
                Some(message_id),
                start.elapsed().as_millis() as u64,
            )),
            Err(e) => Ok(ChannelResult::failed(
                self.channel_type(),
                e.to_string(),
                start.elapsed().as_millis() as u64,
            )),
        }
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
    }

    #[tokio::test]
    async fn test_app_push_channel_creation() {
        let channel = AppPushChannel::with_defaults();
        assert_eq!(channel.channel_type(), ChannelType::AppPush);
        assert_eq!(channel.name(), "App Push");
    }

    #[tokio::test]
    async fn test_app_push_send_success() {
        let channel = AppPushChannel::with_defaults();
        let notification = create_test_notification("user-123");

        let result = channel.send(&notification).await.unwrap();

        assert_eq!(result.channel, ChannelType::AppPush);
        assert_eq!(result.status, crate::notification::types::SendStatus::Success);
        assert!(result.external_message_id.is_some());
        assert!(result.external_message_id.unwrap().starts_with("push_"));
    }

    #[tokio::test]
    async fn test_app_push_disabled() {
        let config = ChannelConfig::new(false);
        let channel = AppPushChannel::new(config);
        let notification = create_test_notification("user-123");

        assert!(!channel.is_available(&notification).await);

        let result = channel.send(&notification).await.unwrap();
        assert_eq!(result.status, crate::notification::types::SendStatus::Skipped);
    }

    #[tokio::test]
    async fn test_app_push_send_failure() {
        let channel = AppPushChannel::with_defaults();
        let notification = create_test_notification("fail_push_user");

        let result = channel.send(&notification).await.unwrap();

        assert_eq!(result.status, crate::notification::types::SendStatus::Failed);
        assert!(result.error.is_some());
    }
}
