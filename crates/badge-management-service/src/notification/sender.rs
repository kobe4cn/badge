//! 通知发送器
//!
//! 提供业务服务发送通知的便捷接口。
//!
//! ## 设计说明
//!
//! `NotificationSender` 是一个轻量级的通知发送封装，可以被注入到各个业务服务中。
//! 它负责：
//! - 根据业务事件类型创建通知
//! - 异步发送通知（不阻塞主业务流程）
//! - 处理发送失败（记录日志但不影响业务）

use std::sync::Arc;

use badge_shared::events::NotificationChannel as ChannelType;
use chrono::{DateTime, Utc};
use tracing::{error, info, warn};

use super::service::NotificationService;
use super::types::{Notification, NotificationBuilder};

/// 通知发送器
///
/// 封装 NotificationService，提供业务友好的发送接口
#[derive(Clone)]
pub struct NotificationSender {
    service: Arc<NotificationService>,
}

impl NotificationSender {
    pub fn new(service: Arc<NotificationService>) -> Self {
        Self { service }
    }

    /// 发送徽章获取通知
    ///
    /// 在徽章发放成功后调用，异步发送通知到用户
    pub fn send_badge_granted(&self, user_id: &str, badge_id: i64, badge_name: &str) {
        let notification = NotificationBuilder::badge_granted(user_id, badge_id, badge_name);
        self.send_async(notification);
    }

    /// 发送徽章即将过期通知
    ///
    /// 由定时任务调用，提醒用户徽章即将过期
    pub fn send_badge_expiring(
        &self,
        user_id: &str,
        badge_id: i64,
        badge_name: &str,
        expires_at: DateTime<Utc>,
    ) {
        let notification =
            NotificationBuilder::badge_expiring(user_id, badge_id, badge_name, expires_at);
        self.send_async(notification);
    }

    /// 发送徽章撤销通知
    pub fn send_badge_revoked(&self, user_id: &str, badge_id: i64, badge_name: &str, reason: &str) {
        let notification =
            NotificationBuilder::badge_revoked(user_id, badge_id, badge_name, reason);
        self.send_async(notification);
    }

    /// 发送兑换成功通知
    ///
    /// 在兑换成功后调用
    pub fn send_redemption_success(
        &self,
        user_id: &str,
        order_id: i64,
        order_no: &str,
        benefit_name: &str,
    ) {
        let notification =
            NotificationBuilder::redemption_success(user_id, order_id, order_no, benefit_name);
        self.send_async(notification);
    }

    /// 发送兑换失败通知
    pub fn send_redemption_failed(&self, user_id: &str, order_id: i64, reason: &str) {
        let notification = NotificationBuilder::redemption_failed(user_id, order_id, reason);
        self.send_async(notification);
    }

    /// 发送权益发放成功通知
    pub fn send_benefit_granted(
        &self,
        user_id: &str,
        benefit_id: i64,
        benefit_name: &str,
        benefit_type: &str,
    ) {
        let notification =
            NotificationBuilder::benefit_granted(user_id, benefit_id, benefit_name, benefit_type);
        self.send_async(notification);
    }

    /// 发送自定义通知
    pub fn send_custom(&self, notification: Notification) {
        self.send_async(notification);
    }

    /// 发送自定义通知（同步等待结果）
    pub async fn send_custom_sync(
        &self,
        notification: Notification,
    ) -> crate::error::Result<super::types::NotificationResult> {
        self.service.send(notification).await
    }

    /// 异步发送通知（fire-and-forget）
    fn send_async(&self, notification: Notification) {
        let service = self.service.clone();
        let notification_id = notification.notification_id.clone();
        let user_id = notification.user_id.clone();
        let notification_type = notification.notification_type.clone();

        tokio::spawn(async move {
            match service.send(notification).await {
                Ok(result) => {
                    if result.success {
                        info!(
                            notification_id = %notification_id,
                            user_id = %user_id,
                            notification_type = ?notification_type,
                            "通知发送成功"
                        );
                    } else if result.is_partial_success() {
                        warn!(
                            notification_id = %notification_id,
                            user_id = %user_id,
                            success_count = result.success_count(),
                            failure_count = result.failure_count(),
                            "通知部分发送成功"
                        );
                    } else {
                        error!(
                            notification_id = %notification_id,
                            user_id = %user_id,
                            "通知发送失败"
                        );
                    }
                }
                Err(e) => {
                    error!(
                        notification_id = %notification_id,
                        user_id = %user_id,
                        error = %e,
                        "通知发送异常"
                    );
                }
            }
        });
    }
}

/// 通知发送器配置
#[derive(Debug, Clone)]
pub struct NotificationSenderConfig {
    /// 默认发送渠道
    pub default_channels: Vec<ChannelType>,
    /// 是否启用异步发送
    pub async_enabled: bool,
}

impl Default for NotificationSenderConfig {
    fn default() -> Self {
        Self {
            default_channels: vec![ChannelType::AppPush, ChannelType::WeChat],
            async_enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use badge_shared::events::NotificationType;

    #[tokio::test]
    async fn test_notification_sender_creation() {
        let service = Arc::new(NotificationService::with_defaults());
        let sender = NotificationSender::new(service);

        // 测试发送不会 panic
        sender.send_badge_granted("user-123", 42, "测试徽章");

        // 等待异步任务完成
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_send_custom_sync() {
        let service = Arc::new(NotificationService::with_defaults());
        let sender = NotificationSender::new(service);

        let notification = Notification::new(
            "user-123",
            NotificationType::BadgeGranted,
            "测试标题",
            "测试内容",
        )
        .with_channels(vec![ChannelType::AppPush]);

        let result = sender.send_custom_sync(notification).await.unwrap();
        assert!(result.success);
    }
}
