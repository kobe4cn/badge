//! 兑换相关实体定义
//!
//! 包含兑换规则、兑换订单、权益定义等

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::enums::{BenefitType, OrderStatus};

/// 权益状态
///
/// 控制权益的可见性和可兑换性
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum BenefitStatus {
    /// 启用 - 正常可兑换
    #[default]
    Active,
    /// 禁用 - 不可兑换
    Inactive,
}

/// 权益定义
///
/// 定义可被徽章兑换的权益，如优惠券、数字资产、预约资格等
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Benefit {
    pub id: i64,
    /// 权益编码（唯一标识）
    pub code: String,
    /// 权益名称
    pub name: String,
    /// 权益描述
    #[sqlx(default)]
    pub description: Option<String>,
    /// 权益类型
    pub benefit_type: BenefitType,
    /// 外部系统权益 ID
    #[sqlx(default)]
    pub external_id: Option<String>,
    /// 外部系统标识
    #[sqlx(default)]
    pub external_system: Option<String>,
    /// 总库存（null 表示不限量）
    #[sqlx(default)]
    pub total_stock: Option<i64>,
    /// 剩余库存
    #[sqlx(default)]
    pub remaining_stock: Option<i64>,
    /// 权益状态
    pub status: BenefitStatus,
    /// 权益配置（JSON，根据类型不同有不同结构）
    #[sqlx(default)]
    pub config: Option<Value>,
    /// 权益图标
    #[sqlx(default)]
    pub icon_url: Option<String>,
    /// 已兑换数量
    #[serde(default)]
    pub redeemed_count: i64,
    /// 是否启用（兼容性字段，与 status 同步）
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Benefit {
    /// 检查是否有库存
    pub fn has_stock(&self) -> bool {
        match self.remaining_stock {
            Some(remaining) => remaining > 0,
            None => true, // 不限量
        }
    }

    /// 获取剩余库存数量
    pub fn get_remaining_stock(&self) -> Option<i64> {
        self.remaining_stock
    }

    /// 检查是否可兑换
    pub fn is_redeemable(&self) -> bool {
        self.enabled && self.status == BenefitStatus::Active && self.has_stock()
    }
}

/// 频率限制配置
///
/// 定义用户兑换的频率限制
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrequencyConfig {
    /// 单用户总次数限制
    #[serde(default)]
    pub max_per_user: Option<i32>,
    /// 每日限制次数
    #[serde(default)]
    pub max_per_day: Option<i32>,
    /// 每周限制次数
    #[serde(default)]
    pub max_per_week: Option<i32>,
    /// 每月限制次数
    #[serde(default)]
    pub max_per_month: Option<i32>,
}

/// 徽章兑换规则
///
/// 定义如何用徽章兑换权益
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BadgeRedemptionRule {
    pub id: i64,
    /// 规则名称
    pub name: String,
    /// 规则描述
    #[sqlx(default)]
    pub description: Option<String>,
    /// 关联的权益 ID
    pub benefit_id: i64,
    /// 需要的徽章配置（JSON）
    /// 格式：[{ "badgeId": 1, "quantity": 2 }, { "badgeId": 2, "quantity": 1 }]
    pub required_badges: Value,
    /// 频率限制配置（JSON）
    pub frequency_config: Value,
    /// 有效期类型：FIXED-固定时间段，RELATIVE-相对徽章获取时间
    #[sqlx(default)]
    pub validity_type: Option<String>,
    /// 相对有效天数（validity_type=RELATIVE 时使用）
    #[sqlx(default)]
    pub relative_days: Option<i32>,
    /// 规则生效开始时间
    #[sqlx(default)]
    pub start_time: Option<DateTime<Utc>>,
    /// 规则生效结束时间
    #[sqlx(default)]
    pub end_time: Option<DateTime<Utc>>,
    /// 是否启用
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 需要的徽章配置项
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequiredBadge {
    pub badge_id: i64,
    pub quantity: i32,
}

impl BadgeRedemptionRule {
    /// 解析需要的徽章列表
    pub fn parse_required_badges(&self) -> Result<Vec<RequiredBadge>, serde_json::Error> {
        serde_json::from_value(self.required_badges.clone())
    }

    /// 解析频率限制配置
    pub fn parse_frequency_config(&self) -> Result<FrequencyConfig, serde_json::Error> {
        serde_json::from_value(self.frequency_config.clone())
    }

    /// 检查规则是否在有效期内
    pub fn is_active(&self, now: DateTime<Utc>) -> bool {
        if !self.enabled {
            return false;
        }

        let after_start = self.start_time.is_none_or(|t| now >= t);
        let before_end = self.end_time.is_none_or(|t| now <= t);

        after_start && before_end
    }
}

/// 兑换订单
///
/// 记录用户的兑换请求和处理状态
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct RedemptionOrder {
    pub id: i64,
    /// 订单号（业务唯一标识）
    pub order_no: String,
    /// 用户 ID
    pub user_id: String,
    /// 兑换规则 ID
    pub rule_id: i64,
    /// 权益 ID（冗余存储）
    pub benefit_id: i64,
    /// 订单状态
    pub status: OrderStatus,
    /// 失败原因
    #[sqlx(default)]
    pub failure_reason: Option<String>,
    /// 权益发放结果（JSON，由权益系统返回）
    #[sqlx(default)]
    pub benefit_result: Option<Value>,
    /// 幂等键（防止重复提交）
    #[sqlx(default)]
    pub idempotency_key: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RedemptionOrder {
    /// 检查订单是否可以重试
    pub fn can_retry(&self) -> bool {
        self.status == OrderStatus::Failed
    }

    /// 检查订单是否已完成（成功或失败）
    pub fn is_finished(&self) -> bool {
        matches!(self.status, OrderStatus::Success | OrderStatus::Cancelled)
    }
}

/// 兑换明细
///
/// 记录单次兑换消耗的具体徽章
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct RedemptionDetail {
    pub id: i64,
    /// 关联的订单 ID
    pub order_id: i64,
    /// 用户徽章 ID
    pub user_badge_id: i64,
    /// 徽章定义 ID（冗余存储）
    pub badge_id: i64,
    /// 消耗数量
    pub quantity: i32,
    pub created_at: DateTime<Utc>,
}

/// 兑换请求（用于服务层接口）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedemptionRequest {
    /// 用户 ID
    pub user_id: String,
    /// 兑换规则 ID
    pub rule_id: i64,
    /// 幂等键
    pub idempotency_key: Option<String>,
}

/// 兑换结果（服务层返回）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedemptionResult {
    /// 订单 ID
    pub order_id: i64,
    /// 订单号
    pub order_no: String,
    /// 是否成功
    pub success: bool,
    /// 权益信息
    pub benefit: Option<BenefitInfo>,
    /// 错误信息
    pub error_message: Option<String>,
}

/// 权益简要信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenefitInfo {
    pub benefit_id: i64,
    pub benefit_type: BenefitType,
    pub name: String,
    /// 权益发放结果
    pub result: Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_benefit_has_stock() {
        let mut benefit = create_test_benefit();

        // 不限量
        benefit.remaining_stock = None;
        assert!(benefit.has_stock());

        // 有库存
        benefit.remaining_stock = Some(50);
        assert!(benefit.has_stock());

        // 无库存
        benefit.remaining_stock = Some(0);
        assert!(!benefit.has_stock());
    }

    #[test]
    fn test_benefit_remaining_stock() {
        let mut benefit = create_test_benefit();

        benefit.remaining_stock = None;
        assert_eq!(benefit.get_remaining_stock(), None);

        benefit.remaining_stock = Some(70);
        assert_eq!(benefit.get_remaining_stock(), Some(70));
    }

    #[test]
    fn test_redemption_rule_parse_required_badges() {
        let rule = create_test_redemption_rule();
        let badges = rule.parse_required_badges().unwrap();

        assert_eq!(badges.len(), 2);
        assert_eq!(badges[0].badge_id, 1);
        assert_eq!(badges[0].quantity, 2);
        assert_eq!(badges[1].badge_id, 2);
        assert_eq!(badges[1].quantity, 1);
    }

    #[test]
    fn test_redemption_rule_is_active() {
        let now = Utc::now();
        let mut rule = create_test_redemption_rule();

        // 启用且无时间限制
        rule.enabled = true;
        rule.start_time = None;
        rule.end_time = None;
        assert!(rule.is_active(now));

        // 禁用
        rule.enabled = false;
        assert!(!rule.is_active(now));

        // 启用但已过期
        rule.enabled = true;
        rule.end_time = Some(now - chrono::Duration::days(1));
        assert!(!rule.is_active(now));
    }

    #[test]
    fn test_order_status() {
        let mut order = create_test_order();

        order.status = OrderStatus::Failed;
        assert!(order.can_retry());
        assert!(!order.is_finished());

        order.status = OrderStatus::Success;
        assert!(!order.can_retry());
        assert!(order.is_finished());

        order.status = OrderStatus::Pending;
        assert!(!order.can_retry());
        assert!(!order.is_finished());
    }

    fn create_test_benefit() -> Benefit {
        Benefit {
            id: 1,
            code: "TEST_COUPON_001".to_string(),
            name: "Test Coupon".to_string(),
            description: None,
            benefit_type: BenefitType::Coupon,
            external_id: None,
            external_system: None,
            total_stock: Some(100),
            remaining_stock: Some(100),
            status: BenefitStatus::Active,
            config: Some(json!({"couponId": "coupon-001", "value": 100})),
            icon_url: None,
            redeemed_count: 0,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_redemption_rule() -> BadgeRedemptionRule {
        BadgeRedemptionRule {
            id: 1,
            name: "Test Rule".to_string(),
            description: None,
            benefit_id: 1,
            required_badges: json!([
                {"badgeId": 1, "quantity": 2},
                {"badgeId": 2, "quantity": 1}
            ]),
            frequency_config: json!({"maxPerUser": 5, "maxPerDay": 1}),
            validity_type: None,
            relative_days: None,
            start_time: None,
            end_time: None,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_order() -> RedemptionOrder {
        RedemptionOrder {
            id: 1,
            order_no: "ORD-001".to_string(),
            user_id: "user-123".to_string(),
            rule_id: 1,
            benefit_id: 1,
            status: OrderStatus::Pending,
            failure_reason: None,
            benefit_result: None,
            idempotency_key: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}
