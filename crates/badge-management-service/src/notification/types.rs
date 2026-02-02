//! 通知类型定义
//!
//! 定义通知相关的数据结构和枚举类型。

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use badge_shared::events::{NotificationChannel as Channel, NotificationType};

/// 通知请求
///
/// 包含发送通知所需的所有信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    /// 通知唯一标识
    pub notification_id: String,
    /// 目标用户 ID
    pub user_id: String,
    /// 通知类型
    pub notification_type: NotificationType,
    /// 通知标题（模板或已渲染）
    pub title: String,
    /// 通知正文（模板或已渲染）
    pub body: String,
    /// 要发送的渠道列表
    pub channels: Vec<Channel>,
    /// 通知携带的业务数据
    pub data: HashMap<String, serde_json::Value>,
    /// 模板变量（用于渲染 title 和 body）
    pub variables: HashMap<String, String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 优先级（1-10，10 最高）
    pub priority: u8,
}

impl Notification {
    /// 创建新通知
    pub fn new(
        user_id: impl Into<String>,
        notification_type: NotificationType,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            notification_id: Uuid::now_v7().to_string(),
            user_id: user_id.into(),
            notification_type,
            title: title.into(),
            body: body.into(),
            channels: vec![Channel::AppPush], // 默认使用 App Push
            data: HashMap::new(),
            variables: HashMap::new(),
            created_at: Utc::now(),
            priority: 5,
        }
    }

    /// 添加发送渠道
    pub fn with_channels(mut self, channels: Vec<Channel>) -> Self {
        self.channels = channels;
        self
    }

    /// 添加单个渠道
    pub fn add_channel(mut self, channel: Channel) -> Self {
        if !self.channels.contains(&channel) {
            self.channels.push(channel);
        }
        self
    }

    /// 添加业务数据
    pub fn with_data(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.data.insert(key.into(), value);
        self
    }

    /// 批量添加业务数据
    pub fn with_data_map(mut self, data: HashMap<String, serde_json::Value>) -> Self {
        self.data.extend(data);
        self
    }

    /// 添加模板变量
    pub fn with_variable(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.insert(key.into(), value.into());
        self
    }

    /// 批量添加模板变量
    pub fn with_variables(mut self, variables: HashMap<String, String>) -> Self {
        self.variables.extend(variables);
        self
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority.min(10);
        self
    }
}

/// 通知构建器
///
/// 提供便捷的通知创建方法
pub struct NotificationBuilder;

impl NotificationBuilder {
    /// 创建徽章获取通知
    pub fn badge_granted(
        user_id: impl Into<String>,
        badge_id: i64,
        badge_name: impl Into<String>,
    ) -> Notification {
        let badge_name = badge_name.into();
        Notification::new(
            user_id,
            NotificationType::BadgeGranted,
            "恭喜获得新徽章！",
            format!("您已获得「{}」徽章，快去看看吧！", badge_name),
        )
        .with_data("badge_id", serde_json::json!(badge_id))
        .with_data("badge_name", serde_json::json!(&badge_name))
        .with_variable("badge_name", &badge_name)
        .with_channels(vec![Channel::AppPush, Channel::WeChat])
    }

    /// 创建徽章即将过期通知
    pub fn badge_expiring(
        user_id: impl Into<String>,
        badge_id: i64,
        badge_name: impl Into<String>,
        expires_at: DateTime<Utc>,
    ) -> Notification {
        let badge_name = badge_name.into();
        let days_left = (expires_at - Utc::now()).num_days();

        Notification::new(
            user_id,
            NotificationType::BadgeExpiring,
            "徽章即将过期提醒",
            format!(
                "您的「{}」徽章将在 {} 天后过期，请及时使用！",
                badge_name, days_left
            ),
        )
        .with_data("badge_id", serde_json::json!(badge_id))
        .with_data("badge_name", serde_json::json!(&badge_name))
        .with_data("expires_at", serde_json::json!(expires_at.to_rfc3339()))
        .with_data("days_left", serde_json::json!(days_left))
        .with_variable("badge_name", &badge_name)
        .with_variable("days_left", days_left.to_string())
        .with_channels(vec![Channel::AppPush, Channel::Sms])
        .with_priority(7)
    }

    /// 创建徽章撤销通知
    pub fn badge_revoked(
        user_id: impl Into<String>,
        badge_id: i64,
        badge_name: impl Into<String>,
        reason: impl Into<String>,
    ) -> Notification {
        let badge_name = badge_name.into();
        let reason = reason.into();

        Notification::new(
            user_id,
            NotificationType::BadgeRevoked,
            "徽章已被撤销",
            format!("您的「{}」徽章已被撤销，原因：{}", badge_name, reason),
        )
        .with_data("badge_id", serde_json::json!(badge_id))
        .with_data("badge_name", serde_json::json!(&badge_name))
        .with_data("reason", serde_json::json!(&reason))
        .with_variable("badge_name", &badge_name)
        .with_variable("reason", &reason)
        .with_channels(vec![Channel::AppPush])
    }

    /// 创建兑换成功通知
    pub fn redemption_success(
        user_id: impl Into<String>,
        order_id: i64,
        order_no: impl Into<String>,
        benefit_name: impl Into<String>,
    ) -> Notification {
        let benefit_name = benefit_name.into();
        let order_no = order_no.into();

        Notification::new(
            user_id,
            NotificationType::RedemptionSuccess,
            "兑换成功！",
            format!("您已成功兑换「{}」，请查收！", benefit_name),
        )
        .with_data("order_id", serde_json::json!(order_id))
        .with_data("order_no", serde_json::json!(&order_no))
        .with_data("benefit_name", serde_json::json!(&benefit_name))
        .with_variable("benefit_name", &benefit_name)
        .with_variable("order_no", &order_no)
        .with_channels(vec![Channel::AppPush, Channel::Sms])
    }

    /// 创建兑换失败通知
    pub fn redemption_failed(
        user_id: impl Into<String>,
        order_id: i64,
        reason: impl Into<String>,
    ) -> Notification {
        let reason = reason.into();

        Notification::new(
            user_id,
            NotificationType::RedemptionFailed,
            "兑换失败",
            format!("兑换失败，原因：{}。如有疑问请联系客服。", reason),
        )
        .with_data("order_id", serde_json::json!(order_id))
        .with_data("reason", serde_json::json!(&reason))
        .with_variable("reason", &reason)
        .with_channels(vec![Channel::AppPush])
    }

    /// 创建权益发放成功通知
    pub fn benefit_granted(
        user_id: impl Into<String>,
        benefit_id: i64,
        benefit_name: impl Into<String>,
        benefit_type: impl Into<String>,
    ) -> Notification {
        let benefit_name = benefit_name.into();
        let benefit_type = benefit_type.into();

        // 复用 RedemptionSuccess 类型，因为权益发放本质上也是兑换成功的一种
        Notification::new(
            user_id,
            NotificationType::RedemptionSuccess,
            "权益已发放！",
            format!("您已获得「{}」，快去使用吧！", benefit_name),
        )
        .with_data("benefit_id", serde_json::json!(benefit_id))
        .with_data("benefit_name", serde_json::json!(&benefit_name))
        .with_data("benefit_type", serde_json::json!(&benefit_type))
        .with_variable("benefit_name", &benefit_name)
        .with_variable("benefit_type", &benefit_type)
        .with_channels(vec![Channel::AppPush, Channel::WeChat])
    }
}

/// 通知上下文
///
/// 用于模板渲染的上下文数据
#[derive(Debug, Clone, Default)]
pub struct NotificationContext {
    /// 变量映射
    pub variables: HashMap<String, String>,
}

impl NotificationContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.variables.insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.variables.get(key).map(|s| s.as_str())
    }
}

impl From<HashMap<String, String>> for NotificationContext {
    fn from(variables: HashMap<String, String>) -> Self {
        Self { variables }
    }
}

/// 通知发送结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationResult {
    /// 通知 ID
    pub notification_id: String,
    /// 是否所有渠道都成功
    pub success: bool,
    /// 各渠道发送结果
    pub channel_results: Vec<ChannelResult>,
    /// 发送耗时（毫秒）
    pub duration_ms: u64,
    /// 发送时间
    pub sent_at: DateTime<Utc>,
}

impl NotificationResult {
    /// 创建成功结果
    pub fn success(notification_id: String, channel_results: Vec<ChannelResult>, duration_ms: u64) -> Self {
        let all_success = channel_results.iter().all(|r| r.status == SendStatus::Success);
        Self {
            notification_id,
            success: all_success,
            channel_results,
            duration_ms,
            sent_at: Utc::now(),
        }
    }

    /// 获取成功的渠道数量
    pub fn success_count(&self) -> usize {
        self.channel_results
            .iter()
            .filter(|r| r.status == SendStatus::Success)
            .count()
    }

    /// 获取失败的渠道数量
    pub fn failure_count(&self) -> usize {
        self.channel_results
            .iter()
            .filter(|r| r.status == SendStatus::Failed)
            .count()
    }

    /// 是否有部分成功
    pub fn is_partial_success(&self) -> bool {
        let success_count = self.success_count();
        success_count > 0 && success_count < self.channel_results.len()
    }
}

/// 单渠道发送结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelResult {
    /// 渠道类型
    pub channel: Channel,
    /// 发送状态
    pub status: SendStatus,
    /// 错误信息（失败时）
    pub error: Option<String>,
    /// 外部系统消息 ID（成功时）
    pub external_message_id: Option<String>,
    /// 发送耗时（毫秒）
    pub duration_ms: u64,
}

impl ChannelResult {
    /// 创建成功结果
    pub fn success(channel: Channel, external_message_id: Option<String>, duration_ms: u64) -> Self {
        Self {
            channel,
            status: SendStatus::Success,
            error: None,
            external_message_id,
            duration_ms,
        }
    }

    /// 创建失败结果
    pub fn failed(channel: Channel, error: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            channel,
            status: SendStatus::Failed,
            error: Some(error.into()),
            external_message_id: None,
            duration_ms,
        }
    }

    /// 创建跳过结果（渠道未配置或不可用）
    pub fn skipped(channel: Channel, reason: impl Into<String>) -> Self {
        Self {
            channel,
            status: SendStatus::Skipped,
            error: Some(reason.into()),
            external_message_id: None,
            duration_ms: 0,
        }
    }
}

/// 发送状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SendStatus {
    /// 发送成功
    Success,
    /// 发送失败
    Failed,
    /// 已跳过（渠道不可用）
    Skipped,
    /// 处理中（异步发送）
    Pending,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_creation() {
        let notification = Notification::new(
            "user-123",
            NotificationType::BadgeGranted,
            "测试标题",
            "测试内容",
        );

        assert_eq!(notification.user_id, "user-123");
        assert_eq!(notification.notification_type, NotificationType::BadgeGranted);
        assert_eq!(notification.title, "测试标题");
        assert_eq!(notification.body, "测试内容");
        assert_eq!(notification.channels, vec![Channel::AppPush]);
        assert_eq!(notification.priority, 5);
    }

    #[test]
    fn test_notification_builder_methods() {
        let notification = Notification::new(
            "user-123",
            NotificationType::BadgeGranted,
            "标题",
            "内容",
        )
        .with_channels(vec![Channel::AppPush, Channel::Sms])
        .with_data("badge_id", serde_json::json!(42))
        .with_variable("badge_name", "测试徽章")
        .with_priority(8);

        assert_eq!(notification.channels.len(), 2);
        assert_eq!(notification.data.get("badge_id").unwrap(), &serde_json::json!(42));
        assert_eq!(notification.variables.get("badge_name").unwrap(), "测试徽章");
        assert_eq!(notification.priority, 8);
    }

    #[test]
    fn test_notification_builder_badge_granted() {
        let notification = NotificationBuilder::badge_granted("user-123", 42, "首次购物");

        assert_eq!(notification.user_id, "user-123");
        assert_eq!(notification.notification_type, NotificationType::BadgeGranted);
        assert!(notification.title.contains("恭喜"));
        assert!(notification.body.contains("首次购物"));
        assert_eq!(notification.data.get("badge_id").unwrap(), &serde_json::json!(42));
    }

    #[test]
    fn test_notification_builder_redemption_success() {
        let notification = NotificationBuilder::redemption_success(
            "user-123",
            100,
            "RD20250101000000123456",
            "VIP 优惠券",
        );

        assert_eq!(notification.notification_type, NotificationType::RedemptionSuccess);
        assert!(notification.body.contains("VIP 优惠券"));
        assert_eq!(notification.data.get("order_id").unwrap(), &serde_json::json!(100));
    }

    #[test]
    fn test_channel_result_success() {
        let result = ChannelResult::success(Channel::AppPush, Some("msg-123".to_string()), 50);

        assert_eq!(result.channel, Channel::AppPush);
        assert_eq!(result.status, SendStatus::Success);
        assert!(result.error.is_none());
        assert_eq!(result.external_message_id, Some("msg-123".to_string()));
        assert_eq!(result.duration_ms, 50);
    }

    #[test]
    fn test_channel_result_failed() {
        let result = ChannelResult::failed(Channel::Sms, "短信服务不可用", 100);

        assert_eq!(result.channel, Channel::Sms);
        assert_eq!(result.status, SendStatus::Failed);
        assert_eq!(result.error, Some("短信服务不可用".to_string()));
        assert!(result.external_message_id.is_none());
    }

    #[test]
    fn test_notification_result() {
        let channel_results = vec![
            ChannelResult::success(Channel::AppPush, None, 30),
            ChannelResult::failed(Channel::Sms, "发送失败", 50),
        ];

        let result = NotificationResult::success("notif-001".to_string(), channel_results, 80);

        assert!(!result.success); // 有失败的渠道
        assert!(result.is_partial_success());
        assert_eq!(result.success_count(), 1);
        assert_eq!(result.failure_count(), 1);
    }

    #[test]
    fn test_notification_context() {
        let mut context = NotificationContext::new();
        context.set("badge_name", "测试徽章");
        context.set("user_name", "张三");

        assert_eq!(context.get("badge_name"), Some("测试徽章"));
        assert_eq!(context.get("user_name"), Some("张三"));
        assert_eq!(context.get("not_exists"), None);
    }
}
