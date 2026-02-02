//! 通知渠道实现
//!
//! 定义通知渠道 trait 并提供各种渠道的具体实现。
//!
//! ## 支持的渠道
//!
//! - **AppPush**: App 推送通知（如 FCM、APNs）
//! - **SMS**: 短信通知
//! - **Email**: 邮件通知
//! - **WeChat**: 微信模板消息

mod app_push;
mod email;
mod sms;
mod wechat;

pub use app_push::AppPushChannel;
pub use email::EmailChannel;
pub use sms::SmsChannel;
pub use wechat::WeChatChannel;

use async_trait::async_trait;
use badge_shared::events::NotificationChannel as ChannelType;

use crate::error::Result;
use super::types::{ChannelResult, Notification};

/// 通知渠道 trait
///
/// 所有通知渠道都需要实现此 trait，提供统一的发送接口。
/// 渠道实现应当是无状态的，便于并发调用。
#[async_trait]
pub trait NotificationChannel: Send + Sync {
    /// 渠道类型标识
    fn channel_type(&self) -> ChannelType;

    /// 渠道名称（用于日志）
    fn name(&self) -> &str;

    /// 检查渠道是否可用
    ///
    /// 在发送前调用，用于判断是否应该跳过此渠道。
    /// 例如用户未绑定手机号时，SMS 渠道应返回 false。
    async fn is_available(&self, notification: &Notification) -> bool;

    /// 发送通知
    ///
    /// 实现具体的发送逻辑，返回发送结果。
    /// 发送失败应返回 ChannelResult::failed 而非 Err，
    /// 以便调用方区分"可重试的错误"和"永久失败"。
    async fn send(&self, notification: &Notification) -> Result<ChannelResult>;

    /// 批量发送通知
    ///
    /// 默认实现逐个发送，子类可覆盖以优化批量发送性能。
    async fn send_batch(&self, notifications: &[Notification]) -> Vec<Result<ChannelResult>> {
        let mut results = Vec::with_capacity(notifications.len());
        for notification in notifications {
            results.push(self.send(notification).await);
        }
        results
    }
}

/// 渠道配置
#[derive(Debug, Clone, Default)]
pub struct ChannelConfig {
    /// 是否启用
    pub enabled: bool,
    /// 请求超时（毫秒）
    pub timeout_ms: u64,
    /// 重试次数
    pub max_retries: u32,
    /// API 端点（如有）
    pub endpoint: Option<String>,
    /// API 密钥（如有）
    pub api_key: Option<String>,
}

impl ChannelConfig {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            timeout_ms: 5000,
            max_retries: 3,
            endpoint: None,
            api_key: None,
        }
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_config() {
        let config = ChannelConfig::new(true)
            .with_endpoint("https://api.example.com")
            .with_api_key("secret-key")
            .with_timeout(3000);

        assert!(config.enabled);
        assert_eq!(config.endpoint, Some("https://api.example.com".to_string()));
        assert_eq!(config.api_key, Some("secret-key".to_string()));
        assert_eq!(config.timeout_ms, 3000);
    }
}
