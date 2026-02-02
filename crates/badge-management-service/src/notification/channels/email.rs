//! Email 邮件通知渠道
//!
//! 通过邮件服务发送邮件通知。
//! 当前为模拟实现，生产环境需要接入真实的邮件服务（如 SendGrid、AWS SES）。

use std::time::Instant;

use async_trait::async_trait;
use tracing::{debug, info, warn};
use uuid::Uuid;

use badge_shared::events::NotificationChannel as ChannelType;

use super::{ChannelConfig, ChannelResult, NotificationChannel};
use crate::error::Result;
use crate::notification::types::Notification;

/// Email 邮件通知渠道
///
/// 负责向用户发送邮件通知。
/// 支持 HTML 格式的邮件内容。
pub struct EmailChannel {
    config: ChannelConfig,
    /// 发件人地址
    from_address: String,
    /// 发件人名称（用于构建邮件头 From 字段）
    #[allow(dead_code)]
    from_name: String,
}

impl EmailChannel {
    pub fn new(config: ChannelConfig, from_address: String, from_name: String) -> Self {
        Self {
            config,
            from_address,
            from_name,
        }
    }

    /// 使用默认配置创建
    pub fn with_defaults() -> Self {
        Self::new(
            ChannelConfig::new(true).with_timeout(10000),
            "noreply@badge-system.com".to_string(),
            "徽章系统".to_string(),
        )
    }

    /// 构建 HTML 邮件内容
    fn build_html_content(&self, notification: &Notification) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>{}</title>
    <style>
        body {{ font-family: Arial, sans-serif; line-height: 1.6; color: #333; }}
        .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
        .header {{ background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 20px; border-radius: 8px 8px 0 0; }}
        .content {{ background: #f9f9f9; padding: 20px; border-radius: 0 0 8px 8px; }}
        .footer {{ text-align: center; color: #888; font-size: 12px; margin-top: 20px; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>{}</h1>
        </div>
        <div class="content">
            <p>{}</p>
        </div>
        <div class="footer">
            <p>此邮件由徽章系统自动发送，请勿回复。</p>
        </div>
    </div>
</body>
</html>"#,
            notification.title, notification.title, notification.body
        )
    }

    /// 模拟发送邮件（生产环境应接入真实邮件服务）
    async fn send_email(&self, notification: &Notification) -> Result<String> {
        // 模拟网络延迟
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;

        let html_content = self.build_html_content(notification);

        debug!(
            notification_id = %notification.notification_id,
            user_id = %notification.user_id,
            from = %self.from_address,
            subject = %notification.title,
            content_length = html_content.len(),
            "Email 发送中..."
        );

        // 模拟随机失败
        #[cfg(test)]
        if notification.user_id.contains("fail_email") {
            return Err(crate::error::BadgeError::Internal(
                "模拟 Email 发送失败".to_string(),
            ));
        }

        // 模拟无效邮箱
        #[cfg(test)]
        if notification.user_id.contains("invalid_email") {
            return Err(crate::error::BadgeError::Validation(
                "用户邮箱地址无效".to_string(),
            ));
        }

        // 生成模拟的消息 ID
        let message_id = format!("email_{}", Uuid::new_v4());

        info!(
            notification_id = %notification.notification_id,
            message_id = %message_id,
            "Email 发送成功"
        );

        Ok(message_id)
    }
}

#[async_trait]
impl NotificationChannel for EmailChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::Email
    }

    fn name(&self) -> &str {
        "Email"
    }

    async fn is_available(&self, notification: &Notification) -> bool {
        if !self.config.enabled {
            warn!(
                notification_id = %notification.notification_id,
                "Email 渠道已禁用"
            );
            return false;
        }

        // 实际实现应检查用户是否有有效的邮箱地址
        // 当前模拟实现：用户 ID 包含 "no_email" 则视为无邮箱
        if notification.user_id.contains("no_email") {
            warn!(
                notification_id = %notification.notification_id,
                user_id = %notification.user_id,
                "用户未绑定邮箱，跳过 Email"
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
                "用户未绑定邮箱或渠道已禁用",
            ));
        }

        match self.send_email(notification).await {
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
    async fn test_email_channel_creation() {
        let channel = EmailChannel::with_defaults();
        assert_eq!(channel.channel_type(), ChannelType::Email);
        assert_eq!(channel.name(), "Email");
        assert_eq!(channel.from_address, "noreply@badge-system.com");
    }

    #[tokio::test]
    async fn test_email_send_success() {
        let channel = EmailChannel::with_defaults();
        let notification = create_test_notification("user-123");

        let result = channel.send(&notification).await.unwrap();

        assert_eq!(result.channel, ChannelType::Email);
        assert_eq!(result.status, crate::notification::types::SendStatus::Success);
        assert!(result.external_message_id.is_some());
        assert!(result.external_message_id.unwrap().starts_with("email_"));
    }

    #[tokio::test]
    async fn test_email_user_no_email() {
        let channel = EmailChannel::with_defaults();
        let notification = create_test_notification("user_no_email_123");

        assert!(!channel.is_available(&notification).await);

        let result = channel.send(&notification).await.unwrap();
        assert_eq!(result.status, crate::notification::types::SendStatus::Skipped);
    }

    #[tokio::test]
    async fn test_email_send_failure() {
        let channel = EmailChannel::with_defaults();
        let notification = create_test_notification("fail_email_user");

        let result = channel.send(&notification).await.unwrap();

        assert_eq!(result.status, crate::notification::types::SendStatus::Failed);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_email_build_html_content() {
        let channel = EmailChannel::with_defaults();
        let notification = create_test_notification("user-123");

        let html = channel.build_html_content(&notification);

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains(&notification.title));
        assert!(html.contains(&notification.body));
        assert!(html.contains("徽章系统"));
    }

    #[tokio::test]
    async fn test_email_disabled() {
        let config = ChannelConfig::new(false);
        let channel = EmailChannel::new(
            config,
            "test@example.com".to_string(),
            "Test".to_string(),
        );
        let notification = create_test_notification("user-123");

        assert!(!channel.is_available(&notification).await);
    }
}
