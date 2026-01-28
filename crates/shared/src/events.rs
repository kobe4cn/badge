//! 事件模型与处理管道抽象
//!
//! 定义徽章系统中所有事件的统一信封格式、事件类型分类、处理结果以及
//! 通知事件模型。同时提供 `EventProcessor` trait 作为事件处理管道的
//! 核心抽象，供各服务实现具体的事件处理逻辑。

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::BadgeError;

// ---------------------------------------------------------------------------
// EventType — 事件类型枚举
// ---------------------------------------------------------------------------

/// 事件类型枚举
///
/// 按业务域划分为四大类：交易、行为、身份、季节。
/// 分类信息用于路由事件到不同的处理管道，以及在规则引擎中按类别批量启用/禁用规则。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventType {
    // 交易类事件 — 涉及金额流转，需要与订单系统核对
    Purchase,
    Refund,
    OrderCancel,

    // 行为类事件 — 用户主动互动行为，用于衡量活跃度
    CheckIn,
    ProfileUpdate,
    PageView,
    Share,
    Review,

    // 身份类事件 — 用户生命周期里程碑，通常只触发一次或按年触发
    Registration,
    MembershipUpgrade,
    Anniversary,

    // 季节类事件 — 运营活动驱动，有时间窗口限制
    SeasonalActivity,
    CampaignParticipation,
}

impl EventType {
    /// 交易类事件涉及资金流转，后续可能需要退款回滚等补偿逻辑
    pub fn is_transaction(&self) -> bool {
        matches!(self, Self::Purchase | Self::Refund | Self::OrderCancel)
    }

    /// 行为类事件反映用户活跃度，是徽章发放最常见的触发源
    pub fn is_engagement(&self) -> bool {
        matches!(
            self,
            Self::CheckIn | Self::ProfileUpdate | Self::PageView | Self::Share | Self::Review
        )
    }

    /// 身份类事件对应用户生命周期关键节点，一般触发频率低但价值高
    pub fn is_identity(&self) -> bool {
        matches!(
            self,
            Self::Registration | Self::MembershipUpgrade | Self::Anniversary
        )
    }

    /// 季节类事件受运营活动时间窗口约束，过期后不再触发
    pub fn is_seasonal(&self) -> bool {
        matches!(self, Self::SeasonalActivity | Self::CampaignParticipation)
    }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 序列化为 SCREAMING_SNAKE_CASE 保持与 serde 的一致性，
        // 便于在日志、Kafka header 和规则引擎中统一引用
        let s = match self {
            Self::Purchase => "PURCHASE",
            Self::Refund => "REFUND",
            Self::OrderCancel => "ORDER_CANCEL",
            Self::CheckIn => "CHECK_IN",
            Self::ProfileUpdate => "PROFILE_UPDATE",
            Self::PageView => "PAGE_VIEW",
            Self::Share => "SHARE",
            Self::Review => "REVIEW",
            Self::Registration => "REGISTRATION",
            Self::MembershipUpgrade => "MEMBERSHIP_UPGRADE",
            Self::Anniversary => "ANNIVERSARY",
            Self::SeasonalActivity => "SEASONAL_ACTIVITY",
            Self::CampaignParticipation => "CAMPAIGN_PARTICIPATION",
        };
        write!(f, "{s}")
    }
}

// ---------------------------------------------------------------------------
// EventPayload — 通用事件信封
// ---------------------------------------------------------------------------

/// 通用事件信封
///
/// 所有进入徽章系统的事件都包装在此信封中，确保：
/// - 通过 `event_id`（UUID v7）实现幂等性校验
/// - 通过 `trace_id` 串联分布式追踪上下文
/// - 通过 `data` 字段以 JSON 承载不同事件类型的业务数据，避免为每种事件定义独立消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventPayload {
    /// 事件唯一标识（UUID v7），时间有序便于索引，同时用于幂等性校验
    pub event_id: String,
    /// 事件类型
    pub event_type: EventType,
    /// 触发事件的用户 ID
    pub user_id: String,
    /// 事件发生时间
    pub timestamp: DateTime<Utc>,
    /// 事件业务数据（JSON 对象，不同事件类型携带不同字段）
    pub data: serde_json::Value,
    /// 事件来源系统
    pub source: String,
    /// 追踪 ID（用于分布式追踪串联）
    pub trace_id: Option<String>,
}

impl EventPayload {
    /// 构建新事件，自动生成 UUID v7 作为 event_id 并记录当前时间
    ///
    /// UUID v7 包含时间戳前缀，使得按 event_id 排序即可获得时间顺序，
    /// 适合作为 Kafka 消息的 key 或数据库主键。
    pub fn new(
        event_type: EventType,
        user_id: impl Into<String>,
        data: serde_json::Value,
        source: impl Into<String>,
    ) -> Self {
        Self {
            event_id: Uuid::now_v7().to_string(),
            event_type,
            user_id: user_id.into(),
            timestamp: Utc::now(),
            data,
            source: source.into(),
            trace_id: None,
        }
    }

    /// 将事件转换为规则引擎的评估上下文 JSON
    ///
    /// 规则引擎需要一个扁平化的 JSON 对象来评估条件表达式。此方法将事件信封的
    /// 元数据（event_type、user_id、timestamp）与业务 data 合并到同一层级，
    /// 使规则表达式可以直接引用 `$.user_id` 或 `$.amount` 等字段。
    pub fn to_evaluation_context(&self) -> serde_json::Value {
        let mut context = serde_json::json!({
            "event_id": self.event_id,
            "event_type": self.event_type,
            "user_id": self.user_id,
            "timestamp": self.timestamp.to_rfc3339(),
            "source": self.source,
        });

        // 将 data 中的字段展开到顶层，便于规则引擎直接访问
        if let serde_json::Value::Object(data_map) = &self.data
            && let serde_json::Value::Object(ref mut ctx_map) = context
        {
            for (key, value) in data_map {
                ctx_map.insert(key.clone(), value.clone());
            }
        }

        context
    }
}

// ---------------------------------------------------------------------------
// EventResult — 事件处理结果
// ---------------------------------------------------------------------------

/// 事件处理结果
///
/// 记录单个事件经过规则引擎评估后的完整处理结果，
/// 包括匹配到的规则、实际发放的徽章、以及处理过程中遇到的错误。
/// `errors` 字段采用字符串数组而非立即失败，因为一个事件可能匹配多条规则，
/// 部分规则失败不应阻止其他规则的正常执行。
#[derive(Debug, Clone, Serialize)]
pub struct EventResult {
    pub event_id: String,
    pub processed: bool,
    /// 匹配到的规则及对应要发放的徽章
    pub matched_rules: Vec<MatchedRule>,
    /// 已成功发放的徽章
    pub granted_badges: Vec<GrantedBadge>,
    /// 处理耗时（毫秒）
    pub processing_time_ms: i64,
    /// 部分规则执行失败时收集错误信息，不中断整体流程
    pub errors: Vec<String>,
}

/// 匹配到的规则及其关联的徽章信息
#[derive(Debug, Clone, Serialize)]
pub struct MatchedRule {
    pub rule_id: String,
    pub rule_name: String,
    pub badge_id: i64,
    pub badge_name: String,
    pub quantity: i32,
}

/// 已成功发放的徽章记录
#[derive(Debug, Clone, Serialize)]
pub struct GrantedBadge {
    pub badge_id: i64,
    pub badge_name: String,
    /// 用户徽章表中的记录 ID，用于后续查询和兑换
    pub user_badge_id: i64,
    pub quantity: i32,
}

// ---------------------------------------------------------------------------
// NotificationEvent — 通知事件
// ---------------------------------------------------------------------------

/// 通知事件
///
/// 徽章发放/过期/撤销等操作完成后，通过 Kafka 发送通知事件，
/// 由通知服务异步推送到用户端。解耦徽章处理与消息推送，
/// 避免推送失败影响核心业务流程。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationEvent {
    pub notification_id: String,
    pub user_id: String,
    pub notification_type: NotificationType,
    pub title: String,
    pub body: String,
    pub data: serde_json::Value,
    /// 通知需要投递的渠道列表，支持多渠道同时推送
    pub channels: Vec<NotificationChannel>,
    pub created_at: DateTime<Utc>,
}

/// 通知类型
///
/// 不同通知类型对应不同的消息模板和优先级策略
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NotificationType {
    BadgeGranted,
    BadgeExpiring,
    BadgeRevoked,
    RedemptionSuccess,
    RedemptionFailed,
}

/// 通知投递渠道
///
/// 各渠道有不同的消息长度限制和格式要求，通知服务会按渠道适配内容
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NotificationChannel {
    AppPush,
    Sms,
    WeChat,
    Email,
}

// ---------------------------------------------------------------------------
// EventProcessor trait — 事件处理管道抽象
// ---------------------------------------------------------------------------

/// 事件处理管道的核心抽象
///
/// 各服务实现此 trait 来处理特定类型的事件。设计要点：
/// - `process` 负责完整的事件处理流程（规则评估 -> 徽章发放 -> 结果记录）
/// - `supported_event_types` 用于路由层将事件分发到正确的处理器
/// - `is_processed` / `mark_processed` 配合实现幂等性，防止 Kafka 重复消费导致重复发放
#[async_trait]
pub trait EventProcessor: Send + Sync {
    /// 处理单个事件，返回处理结果
    async fn process(&self, event: &EventPayload) -> Result<EventResult, BadgeError>;

    /// 该处理器支持的事件类型，用于事件路由
    fn supported_event_types(&self) -> Vec<EventType>;

    /// 检查事件是否已处理（基于 event_id 的幂等性校验）
    async fn is_processed(&self, event_id: &str) -> Result<bool, BadgeError>;

    /// 标记事件为已处理，写入幂等性记录
    async fn mark_processed(&self, event_id: &str) -> Result<(), BadgeError>;
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_payload_serialization() {
        let event = EventPayload {
            event_id: "01912345-6789-7abc-8def-0123456789ab".to_string(),
            event_type: EventType::Purchase,
            user_id: "user-001".to_string(),
            timestamp: DateTime::parse_from_rfc3339("2025-01-15T10:30:00Z")
                .unwrap()
                .with_timezone(&Utc),
            data: serde_json::json!({"amount": 100.0, "currency": "CNY"}),
            source: "order-service".to_string(),
            trace_id: Some("trace-abc-123".to_string()),
        };

        let json = serde_json::to_string(&event).unwrap();

        // 验证 camelCase 序列化格式
        assert!(json.contains("eventId"));
        assert!(json.contains("eventType"));
        assert!(json.contains("userId"));
        assert!(json.contains("traceId"));

        // 验证反序列化能还原
        let deserialized: EventPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.event_id, event.event_id);
        assert_eq!(deserialized.event_type, EventType::Purchase);
        assert_eq!(deserialized.user_id, "user-001");
        assert_eq!(deserialized.source, "order-service");
        assert_eq!(deserialized.trace_id, Some("trace-abc-123".to_string()));
    }

    #[test]
    fn test_event_type_classification() {
        // 交易类
        assert!(EventType::Purchase.is_transaction());
        assert!(EventType::Refund.is_transaction());
        assert!(EventType::OrderCancel.is_transaction());
        assert!(!EventType::Purchase.is_engagement());

        // 行为类
        assert!(EventType::CheckIn.is_engagement());
        assert!(EventType::ProfileUpdate.is_engagement());
        assert!(EventType::PageView.is_engagement());
        assert!(EventType::Share.is_engagement());
        assert!(EventType::Review.is_engagement());
        assert!(!EventType::CheckIn.is_transaction());

        // 身份类
        assert!(EventType::Registration.is_identity());
        assert!(EventType::MembershipUpgrade.is_identity());
        assert!(EventType::Anniversary.is_identity());
        assert!(!EventType::Registration.is_seasonal());

        // 季节类
        assert!(EventType::SeasonalActivity.is_seasonal());
        assert!(EventType::CampaignParticipation.is_seasonal());
        assert!(!EventType::SeasonalActivity.is_identity());
    }

    #[test]
    fn test_event_type_display() {
        assert_eq!(EventType::Purchase.to_string(), "PURCHASE");
        assert_eq!(EventType::OrderCancel.to_string(), "ORDER_CANCEL");
        assert_eq!(EventType::CheckIn.to_string(), "CHECK_IN");
        assert_eq!(EventType::ProfileUpdate.to_string(), "PROFILE_UPDATE");
        assert_eq!(
            EventType::MembershipUpgrade.to_string(),
            "MEMBERSHIP_UPGRADE"
        );
        assert_eq!(EventType::Anniversary.to_string(), "ANNIVERSARY");
        assert_eq!(EventType::SeasonalActivity.to_string(), "SEASONAL_ACTIVITY");
        assert_eq!(
            EventType::CampaignParticipation.to_string(),
            "CAMPAIGN_PARTICIPATION"
        );
    }

    #[test]
    fn test_event_payload_to_context() {
        let event = EventPayload {
            event_id: "evt-001".to_string(),
            event_type: EventType::Purchase,
            user_id: "user-001".to_string(),
            timestamp: DateTime::parse_from_rfc3339("2025-01-15T10:30:00Z")
                .unwrap()
                .with_timezone(&Utc),
            data: serde_json::json!({"amount": 200.0, "category": "electronics"}),
            source: "order-service".to_string(),
            trace_id: None,
        };

        let ctx = event.to_evaluation_context();

        // 信封元数据应出现在顶层
        assert_eq!(ctx["event_id"], "evt-001");
        assert_eq!(ctx["user_id"], "user-001");
        assert_eq!(ctx["source"], "order-service");

        // data 中的业务字段应展开到顶层，规则引擎可直接引用
        assert_eq!(ctx["amount"], 200.0);
        assert_eq!(ctx["category"], "electronics");
    }

    #[test]
    fn test_notification_event_serialization() {
        let notification = NotificationEvent {
            notification_id: "notif-001".to_string(),
            user_id: "user-001".to_string(),
            notification_type: NotificationType::BadgeGranted,
            title: "恭喜获得新徽章".to_string(),
            body: "您已获得「首次购物」徽章！".to_string(),
            data: serde_json::json!({"badge_id": 42, "badge_name": "首次购物"}),
            channels: vec![NotificationChannel::AppPush, NotificationChannel::WeChat],
            created_at: DateTime::parse_from_rfc3339("2025-01-15T10:30:00Z")
                .unwrap()
                .with_timezone(&Utc),
        };

        let json = serde_json::to_string(&notification).unwrap();

        // 验证 camelCase 序列化
        assert!(json.contains("notificationId"));
        assert!(json.contains("notificationType"));
        assert!(json.contains("createdAt"));

        // 验证反序列化
        let deserialized: NotificationEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.notification_id, "notif-001");
        assert_eq!(
            deserialized.notification_type,
            NotificationType::BadgeGranted
        );
        assert_eq!(deserialized.channels.len(), 2);
        assert_eq!(deserialized.channels[0], NotificationChannel::AppPush);
        assert_eq!(deserialized.channels[1], NotificationChannel::WeChat);
    }

    #[test]
    fn test_event_result_creation() {
        let result = EventResult {
            event_id: "evt-001".to_string(),
            processed: true,
            matched_rules: vec![MatchedRule {
                rule_id: "rule-001".to_string(),
                rule_name: "首次购物奖励".to_string(),
                badge_id: 42,
                badge_name: "首次购物".to_string(),
                quantity: 1,
            }],
            granted_badges: vec![GrantedBadge {
                badge_id: 42,
                badge_name: "首次购物".to_string(),
                user_badge_id: 1001,
                quantity: 1,
            }],
            processing_time_ms: 15,
            errors: vec![],
        };

        assert!(result.processed);
        assert_eq!(result.matched_rules.len(), 1);
        assert_eq!(result.matched_rules[0].rule_id, "rule-001");
        assert_eq!(result.granted_badges.len(), 1);
        assert_eq!(result.granted_badges[0].user_badge_id, 1001);
        assert_eq!(result.processing_time_ms, 15);
        assert!(result.errors.is_empty());

        // 验证可序列化
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("evt-001"));
        assert!(json.contains("rule-001"));
    }
}
