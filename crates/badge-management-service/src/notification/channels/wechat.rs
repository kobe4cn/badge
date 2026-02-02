//! 微信通知渠道
//!
//! 通过微信公众号模板消息发送通知。
//! 当前为模拟实现，生产环境需要接入微信开放平台。

use std::time::Instant;

use async_trait::async_trait;
use tracing::{debug, info, warn};
use uuid::Uuid;

use badge_shared::events::NotificationChannel as ChannelType;

use super::{ChannelConfig, ChannelResult, NotificationChannel};
use crate::error::Result;
use crate::notification::types::Notification;

/// 微信通知渠道
///
/// 负责通过微信公众号向用户发送模板消息。
/// 需要用户已关注公众号并授权。
pub struct WeChatChannel {
    config: ChannelConfig,
    /// 公众号 App ID
    #[allow(dead_code)]
    app_id: String,
    /// 模板 ID 映射（通知类型 -> 微信模板 ID）
    #[allow(dead_code)]
    template_ids: std::collections::HashMap<String, String>,
}

impl WeChatChannel {
    pub fn new(config: ChannelConfig, app_id: String) -> Self {
        Self {
            config,
            app_id,
            template_ids: std::collections::HashMap::new(),
        }
    }

    /// 使用默认配置创建
    pub fn with_defaults() -> Self {
        Self::new(
            ChannelConfig::new(true).with_timeout(5000),
            "wx_default_app_id".to_string(),
        )
    }

    /// 注册模板 ID
    #[allow(dead_code)]
    pub fn register_template(&mut self, notification_type: &str, template_id: &str) {
        self.template_ids
            .insert(notification_type.to_string(), template_id.to_string());
    }

    /// 模拟发送微信模板消息（生产环境应接入微信 API）
    async fn send_template_message(&self, notification: &Notification) -> Result<String> {
        // 模拟网络延迟
        tokio::time::sleep(tokio::time::Duration::from_millis(25)).await;

        debug!(
            notification_id = %notification.notification_id,
            user_id = %notification.user_id,
            notification_type = ?notification.notification_type,
            "微信模板消息发送中..."
        );

        // 模拟随机失败
        #[cfg(test)]
        if notification.user_id.contains("fail_wechat") {
            return Err(crate::error::BadgeError::Internal(
                "模拟微信发送失败".to_string(),
            ));
        }

        // 模拟用户未关注公众号
        #[cfg(test)]
        if notification.user_id.contains("not_follow") {
            return Err(crate::error::BadgeError::Validation(
                "用户未关注公众号".to_string(),
            ));
        }

        // 生成模拟的消息 ID
        let message_id = format!("wx_{}", Uuid::new_v4());

        info!(
            notification_id = %notification.notification_id,
            message_id = %message_id,
            "微信模板消息发送成功"
        );

        Ok(message_id)
    }
}

#[async_trait]
impl NotificationChannel for WeChatChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::WeChat
    }

    fn name(&self) -> &str {
        "WeChat"
    }

    async fn is_available(&self, notification: &Notification) -> bool {
        if !self.config.enabled {
            warn!(
                notification_id = %notification.notification_id,
                "微信渠道已禁用"
            );
            return false;
        }

        // 实际实现应检查用户是否有有效的微信 OpenID
        // 当前模拟实现：用户 ID 包含 "no_wechat" 则视为未绑定微信
        if notification.user_id.contains("no_wechat") {
            warn!(
                notification_id = %notification.notification_id,
                user_id = %notification.user_id,
                "用户未绑定微信，跳过微信通知"
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
                "用户未绑定微信或渠道已禁用",
            ));
        }

        match self.send_template_message(notification).await {
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
    async fn test_wechat_channel_creation() {
        let channel = WeChatChannel::with_defaults();
        assert_eq!(channel.channel_type(), ChannelType::WeChat);
        assert_eq!(channel.name(), "WeChat");
    }

    #[tokio::test]
    async fn test_wechat_send_success() {
        let channel = WeChatChannel::with_defaults();
        let notification = create_test_notification("user-123");

        let result = channel.send(&notification).await.unwrap();

        assert_eq!(result.channel, ChannelType::WeChat);
        assert_eq!(result.status, crate::notification::types::SendStatus::Success);
        assert!(result.external_message_id.is_some());
        assert!(result.external_message_id.unwrap().starts_with("wx_"));
    }

    #[tokio::test]
    async fn test_wechat_user_not_bound() {
        let channel = WeChatChannel::with_defaults();
        let notification = create_test_notification("user_no_wechat_123");

        assert!(!channel.is_available(&notification).await);

        let result = channel.send(&notification).await.unwrap();
        assert_eq!(result.status, crate::notification::types::SendStatus::Skipped);
    }

    #[tokio::test]
    async fn test_wechat_send_failure() {
        let channel = WeChatChannel::with_defaults();
        let notification = create_test_notification("fail_wechat_user");

        let result = channel.send(&notification).await.unwrap();

        assert_eq!(result.status, crate::notification::types::SendStatus::Failed);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_wechat_disabled() {
        let config = ChannelConfig::new(false);
        let channel = WeChatChannel::new(config, "test_app_id".to_string());
        let notification = create_test_notification("user-123");

        assert!(!channel.is_available(&notification).await);
    }

    #[tokio::test]
    async fn test_wechat_register_template() {
        let mut channel = WeChatChannel::with_defaults();
        channel.register_template("BADGE_GRANTED", "template_id_001");

        assert_eq!(
            channel.template_ids.get("BADGE_GRANTED"),
            Some(&"template_id_001".to_string())
        );
    }
}
