//! SMS 短信通知渠道
//!
//! 通过短信服务商发送短信通知。
//! 当前为模拟实现，生产环境需要接入真实的短信服务。

use std::time::Instant;

use async_trait::async_trait;
use tracing::{debug, info, warn};
use uuid::Uuid;

use badge_shared::events::NotificationChannel as ChannelType;

use super::{ChannelConfig, ChannelResult, NotificationChannel};
use crate::error::Result;
use crate::notification::types::Notification;

/// SMS 短信通知渠道
///
/// 负责向用户发送短信通知。
/// 由于短信有字数限制，会自动截断过长的内容。
pub struct SmsChannel {
    config: ChannelConfig,
    /// 短信内容最大长度（中文字符数）
    max_content_length: usize,
}

impl SmsChannel {
    pub fn new(config: ChannelConfig) -> Self {
        Self {
            config,
            max_content_length: 70, // 标准短信长度
        }
    }

    /// 使用默认配置创建
    pub fn with_defaults() -> Self {
        Self::new(ChannelConfig::new(true).with_timeout(5000))
    }

    /// 截断过长的内容
    fn truncate_content(&self, content: &str) -> String {
        let chars: Vec<char> = content.chars().collect();
        if chars.len() <= self.max_content_length {
            content.to_string()
        } else {
            // 截断并添加省略号
            let truncated: String = chars[..self.max_content_length - 3].iter().collect();
            format!("{}...", truncated)
        }
    }

    /// 模拟发送短信（生产环境应接入真实短信服务）
    async fn send_sms(&self, notification: &Notification) -> Result<String> {
        // 模拟网络延迟
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

        let content = self.truncate_content(&notification.body);

        debug!(
            notification_id = %notification.notification_id,
            user_id = %notification.user_id,
            content_length = content.len(),
            "SMS 发送中..."
        );

        // 模拟随机失败
        #[cfg(test)]
        if notification.user_id.contains("fail_sms") {
            return Err(crate::error::BadgeError::Internal(
                "模拟 SMS 发送失败".to_string(),
            ));
        }

        // 模拟无效手机号
        #[cfg(test)]
        if notification.user_id.contains("invalid_phone") {
            return Err(crate::error::BadgeError::Validation(
                "用户手机号无效".to_string(),
            ));
        }

        // 生成模拟的消息 ID
        let message_id = format!("sms_{}", Uuid::new_v4());

        info!(
            notification_id = %notification.notification_id,
            message_id = %message_id,
            "SMS 发送成功"
        );

        Ok(message_id)
    }
}

#[async_trait]
impl NotificationChannel for SmsChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::Sms
    }

    fn name(&self) -> &str {
        "SMS"
    }

    async fn is_available(&self, notification: &Notification) -> bool {
        if !self.config.enabled {
            warn!(
                notification_id = %notification.notification_id,
                "SMS 渠道已禁用"
            );
            return false;
        }

        // 实际实现应检查用户是否有有效的手机号
        // 当前模拟实现：用户 ID 包含 "no_phone" 则视为无手机号
        if notification.user_id.contains("no_phone") {
            warn!(
                notification_id = %notification.notification_id,
                user_id = %notification.user_id,
                "用户未绑定手机号，跳过 SMS"
            );
            return false;
        }

        true
    }

    async fn send(&self, notification: &Notification) -> Result<ChannelResult> {
        let start = Instant::now();

        if !self.is_available(notification).await {
            return Ok(ChannelResult::skipped(
                self.channel_type(),
                "用户未绑定手机号或渠道已禁用",
            ));
        }

        match self.send_sms(notification).await {
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
    async fn test_sms_channel_creation() {
        let channel = SmsChannel::with_defaults();
        assert_eq!(channel.channel_type(), ChannelType::Sms);
        assert_eq!(channel.name(), "SMS");
    }

    #[tokio::test]
    async fn test_sms_send_success() {
        let channel = SmsChannel::with_defaults();
        let notification = create_test_notification("user-123");

        let result = channel.send(&notification).await.unwrap();

        assert_eq!(result.channel, ChannelType::Sms);
        assert_eq!(result.status, crate::notification::types::SendStatus::Success);
        assert!(result.external_message_id.is_some());
        assert!(result.external_message_id.unwrap().starts_with("sms_"));
    }

    #[tokio::test]
    async fn test_sms_user_no_phone() {
        let channel = SmsChannel::with_defaults();
        let notification = create_test_notification("user_no_phone_123");

        assert!(!channel.is_available(&notification).await);

        let result = channel.send(&notification).await.unwrap();
        assert_eq!(result.status, crate::notification::types::SendStatus::Skipped);
    }

    #[tokio::test]
    async fn test_sms_send_failure() {
        let channel = SmsChannel::with_defaults();
        let notification = create_test_notification("fail_sms_user");

        let result = channel.send(&notification).await.unwrap();

        assert_eq!(result.status, crate::notification::types::SendStatus::Failed);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_sms_truncate_content() {
        let channel = SmsChannel::with_defaults();

        // 短内容不截断
        let short = "这是一条短消息";
        assert_eq!(channel.truncate_content(short), short);

        // 长内容截断
        let long = "这".repeat(100);
        let truncated = channel.truncate_content(&long);
        assert!(truncated.ends_with("..."));
        assert!(truncated.chars().count() <= 70);
    }

    #[tokio::test]
    async fn test_sms_disabled() {
        let config = ChannelConfig::new(false);
        let channel = SmsChannel::new(config);
        let notification = create_test_notification("user-123");

        assert!(!channel.is_available(&notification).await);
    }
}
