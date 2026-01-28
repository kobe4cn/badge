//! 多渠道通知发送器
//!
//! 通过 `NotificationSender` trait 抽象发送行为，各渠道（APP Push、SMS、微信、邮件）
//! 提供独立实现。当前版本为模拟发送（仅记录日志），便于在无外部依赖的情况下
//! 验证消费管道的完整性。未来替换为真实 SDK 调用时只需实现同一 trait。

use async_trait::async_trait;
use badge_shared::events::{NotificationChannel, NotificationEvent};
use tracing::info;
use uuid::Uuid;

use crate::error::NotificationError;

/// 发送结果
///
/// 统一记录各渠道的发送状态，consumer 汇总后决定是否需要重试。
pub struct SendResult {
    pub success: bool,
    pub channel: NotificationChannel,
    /// 外部渠道返回的消息标识，用于追踪投递状态
    pub message_id: Option<String>,
    pub error: Option<String>,
}

/// 通知发送器 trait，各渠道实现具体的推送逻辑
#[async_trait]
pub trait NotificationSender: Send + Sync {
    /// 发送通知到指定渠道
    async fn send(&self, notification: &NotificationEvent)
    -> Result<SendResult, NotificationError>;

    /// 该发送器支持的渠道
    fn channel(&self) -> NotificationChannel;
}

// ---------------------------------------------------------------------------
// APP 推送发送器
// ---------------------------------------------------------------------------

/// 模拟 APP 推送发送器
///
/// 生产环境中替换为 APNs / FCM 等推送服务的 SDK 调用
pub struct AppPushSender;

#[async_trait]
impl NotificationSender for AppPushSender {
    async fn send(
        &self,
        notification: &NotificationEvent,
    ) -> Result<SendResult, NotificationError> {
        let message_id = Uuid::now_v7().to_string();

        info!(
            channel = "APP_PUSH",
            notification_id = %notification.notification_id,
            user_id = %notification.user_id,
            message_id = %message_id,
            title = %notification.title,
            "模拟发送 APP 推送通知"
        );

        Ok(SendResult {
            success: true,
            channel: NotificationChannel::AppPush,
            message_id: Some(message_id),
            error: None,
        })
    }

    fn channel(&self) -> NotificationChannel {
        NotificationChannel::AppPush
    }
}

// ---------------------------------------------------------------------------
// 短信发送器
// ---------------------------------------------------------------------------

/// 模拟短信发送器
///
/// 生产环境中替换为短信服务商（如阿里云 SMS）的 API 调用
pub struct SmsSender;

#[async_trait]
impl NotificationSender for SmsSender {
    async fn send(
        &self,
        notification: &NotificationEvent,
    ) -> Result<SendResult, NotificationError> {
        let message_id = Uuid::now_v7().to_string();

        info!(
            channel = "SMS",
            notification_id = %notification.notification_id,
            user_id = %notification.user_id,
            message_id = %message_id,
            body = %notification.body,
            "模拟发送短信通知"
        );

        Ok(SendResult {
            success: true,
            channel: NotificationChannel::Sms,
            message_id: Some(message_id),
            error: None,
        })
    }

    fn channel(&self) -> NotificationChannel {
        NotificationChannel::Sms
    }
}

// ---------------------------------------------------------------------------
// 微信发送器
// ---------------------------------------------------------------------------

/// 模拟微信推送发送器
///
/// 生产环境中替换为微信模板消息 / 订阅消息的 API 调用
pub struct WeChatSender;

#[async_trait]
impl NotificationSender for WeChatSender {
    async fn send(
        &self,
        notification: &NotificationEvent,
    ) -> Result<SendResult, NotificationError> {
        let message_id = Uuid::now_v7().to_string();

        info!(
            channel = "WECHAT",
            notification_id = %notification.notification_id,
            user_id = %notification.user_id,
            message_id = %message_id,
            title = %notification.title,
            "模拟发送微信推送通知"
        );

        Ok(SendResult {
            success: true,
            channel: NotificationChannel::WeChat,
            message_id: Some(message_id),
            error: None,
        })
    }

    fn channel(&self) -> NotificationChannel {
        NotificationChannel::WeChat
    }
}

// ---------------------------------------------------------------------------
// 邮件发送器
// ---------------------------------------------------------------------------

/// 模拟邮件发送器
///
/// 生产环境中替换为 SMTP 或邮件服务商（如 SendGrid）的 API 调用
pub struct EmailSender;

#[async_trait]
impl NotificationSender for EmailSender {
    async fn send(
        &self,
        notification: &NotificationEvent,
    ) -> Result<SendResult, NotificationError> {
        let message_id = Uuid::now_v7().to_string();

        info!(
            channel = "EMAIL",
            notification_id = %notification.notification_id,
            user_id = %notification.user_id,
            message_id = %message_id,
            title = %notification.title,
            body = %notification.body,
            "模拟发送邮件通知"
        );

        Ok(SendResult {
            success: true,
            channel: NotificationChannel::Email,
            message_id: Some(message_id),
            error: None,
        })
    }

    fn channel(&self) -> NotificationChannel {
        NotificationChannel::Email
    }
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use badge_shared::events::NotificationType;
    use chrono::Utc;

    /// 构造通用的测试通知事件
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
            channels: vec![NotificationChannel::AppPush],
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_app_push_send() {
        let sender = AppPushSender;
        let notification = make_test_notification();

        let result = sender.send(&notification).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(result.success);
        assert_eq!(result.channel, NotificationChannel::AppPush);
        assert!(result.message_id.is_some());
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_sms_send() {
        let sender = SmsSender;
        let notification = make_test_notification();

        let result = sender.send(&notification).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(result.success);
        assert_eq!(result.channel, NotificationChannel::Sms);
        assert!(result.message_id.is_some());
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_wechat_send() {
        let sender = WeChatSender;
        let notification = make_test_notification();

        let result = sender.send(&notification).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(result.success);
        assert_eq!(result.channel, NotificationChannel::WeChat);
        assert!(result.message_id.is_some());
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_email_send() {
        let sender = EmailSender;
        let notification = make_test_notification();

        let result = sender.send(&notification).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(result.success);
        assert_eq!(result.channel, NotificationChannel::Email);
        assert!(result.message_id.is_some());
        assert!(result.error.is_none());
    }

    #[test]
    fn test_sender_channel_type() {
        assert_eq!(AppPushSender.channel(), NotificationChannel::AppPush);
        assert_eq!(SmsSender.channel(), NotificationChannel::Sms);
        assert_eq!(WeChatSender.channel(), NotificationChannel::WeChat);
        assert_eq!(EmailSender.channel(), NotificationChannel::Email);
    }
}
