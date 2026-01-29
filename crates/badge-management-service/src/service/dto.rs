//! 服务层数据传输对象
//!
//! 定义服务层与外部交互使用的 DTO，与内部领域模型解耦

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::{
    BadgeAssets, BadgeType, BenefitType, SourceType, UserBadgeStatus, ValidityConfig,
};

/// 用户徽章 DTO
///
/// 展示用户持有的单个徽章信息，聚合了徽章定义和用户持有数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserBadgeDto {
    pub badge_id: i64,
    pub badge_name: String,
    pub badge_type: BadgeType,
    pub quantity: i32,
    pub status: UserBadgeStatus,
    pub acquired_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    pub assets: BadgeAssets,
}

/// 徽章详情 DTO
///
/// 徽章的完整信息，包含所属系列、分类以及可兑换权益
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgeDetailDto {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub badge_type: BadgeType,
    pub series_id: i64,
    pub series_name: String,
    pub category_id: i64,
    pub category_name: String,
    pub assets: BadgeAssets,
    pub obtain_description: String,
    pub validity_config: ValidityConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_supply: Option<i32>,
    pub issued_count: i32,
    pub redeemable_benefits: Vec<BenefitSummaryDto>,
}

/// 权益摘要 DTO
///
/// 用于在徽章详情中展示可兑换的权益列表
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenefitSummaryDto {
    pub benefit_id: i64,
    pub benefit_type: BenefitType,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    /// 兑换所需的徽章数量
    pub required_quantity: i32,
}

/// 徽章墙 DTO
///
/// 按分类组织的用户徽章展示视图
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgeWallDto {
    pub total_count: i32,
    pub categories: Vec<BadgeWallCategoryDto>,
}

/// 徽章墙分类 DTO
///
/// 徽章墙中的单个分类及其包含的徽章
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgeWallCategoryDto {
    pub category_id: i64,
    pub category_name: String,
    pub badges: Vec<UserBadgeDto>,
}

/// 分类 DTO
///
/// 徽章分类的基本信息，包含该分类下的徽章数量
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryDto {
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    pub badge_count: i32,
}

/// 系列详情 DTO
///
/// 徽章系列的完整信息，包含系列内所有徽章
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesDetailDto {
    pub id: i64,
    pub category_id: i64,
    pub category_name: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<DateTime<Utc>>,
    pub badges: Vec<BadgeSummaryDto>,
}

/// 徽章摘要 DTO
///
/// 徽章的简要信息，用于系列详情等场景
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgeSummaryDto {
    pub id: i64,
    pub name: String,
    pub badge_type: BadgeType,
    pub assets: BadgeAssets,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_supply: Option<i32>,
    pub issued_count: i32,
}

/// 用户徽章统计 DTO
///
/// 用户徽章的汇总统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserBadgeStatsDto {
    pub total_badges: i32,
    pub active_badges: i32,
    pub expired_badges: i32,
    pub redeemed_badges: i32,
    pub by_category: Vec<CategoryStatsDto>,
}

/// 分类统计 DTO
///
/// 单个分类下的徽章统计
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryStatsDto {
    pub category_id: i64,
    pub category_name: String,
    pub count: i32,
}

// ==================== 发放服务 DTO ====================

/// 徽章发放请求
///
/// 用于发放徽章给用户的请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantBadgeRequest {
    /// 用户 ID
    pub user_id: String,
    /// 徽章 ID
    pub badge_id: i64,
    /// 发放数量
    pub quantity: i32,
    /// 来源类型
    pub source_type: SourceType,
    /// 来源关联 ID（如事件 ID、活动 ID）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_ref_id: Option<String>,
    /// 幂等键，用于防止重复发放
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    /// 发放原因/备注
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// 操作人（手动发放时使用）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<String>,
}

impl GrantBadgeRequest {
    /// 创建简单的发放请求（用于规则引擎触发的自动发放）
    pub fn new(user_id: impl Into<String>, badge_id: i64, quantity: i32) -> Self {
        Self {
            user_id: user_id.into(),
            badge_id,
            quantity,
            source_type: SourceType::Event,
            source_ref_id: None,
            idempotency_key: None,
            reason: None,
            operator: None,
        }
    }

    /// 创建带幂等键的发放请求
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// 设置来源信息
    pub fn with_source(mut self, source_type: SourceType, ref_id: Option<String>) -> Self {
        self.source_type = source_type;
        self.source_ref_id = ref_id;
        self
    }
}

/// 徽章发放响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantBadgeResponse {
    /// 是否成功
    pub success: bool,
    /// 用户徽章记录 ID
    pub user_badge_id: i64,
    /// 发放后的新数量
    pub new_quantity: i32,
    /// 响应消息
    pub message: String,
}

impl GrantBadgeResponse {
    pub fn success(user_badge_id: i64, new_quantity: i32) -> Self {
        Self {
            success: true,
            user_badge_id,
            new_quantity,
            message: "徽章发放成功".to_string(),
        }
    }

    pub fn from_existing(user_badge_id: i64, new_quantity: i32) -> Self {
        Self {
            success: true,
            user_badge_id,
            new_quantity,
            message: "幂等请求，返回已存在的记录".to_string(),
        }
    }
}

/// 批量发放响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchGrantResponse {
    /// 总请求数
    pub total: i32,
    /// 成功数量
    pub success_count: i32,
    /// 失败数量
    pub failed_count: i32,
    /// 各请求的处理结果
    pub results: Vec<GrantResult>,
}

/// 单个发放结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantResult {
    /// 用户 ID
    pub user_id: String,
    /// 徽章 ID
    pub badge_id: i64,
    /// 是否成功
    pub success: bool,
    /// 用户徽章 ID（成功时返回）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_badge_id: Option<i64>,
    /// 发放后数量（成功时返回）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_quantity: Option<i32>,
    /// 错误信息（失败时返回）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl GrantResult {
    pub fn success(user_id: String, badge_id: i64, user_badge_id: i64, new_quantity: i32) -> Self {
        Self {
            user_id,
            badge_id,
            success: true,
            user_badge_id: Some(user_badge_id),
            new_quantity: Some(new_quantity),
            error: None,
        }
    }

    pub fn failure(user_id: String, badge_id: i64, error: impl Into<String>) -> Self {
        Self {
            user_id,
            badge_id,
            success: false,
            user_badge_id: None,
            new_quantity: None,
            error: Some(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_badge_dto_serialization() {
        let dto = UserBadgeDto {
            badge_id: 1,
            badge_name: "Test Badge".to_string(),
            badge_type: BadgeType::Normal,
            quantity: 1,
            status: UserBadgeStatus::Active,
            acquired_at: Utc::now(),
            expires_at: None,
            assets: BadgeAssets {
                icon_url: "https://example.com/icon.png".to_string(),
                image_url: None,
                animation_url: None,
                disabled_icon_url: None,
            },
        };

        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["badgeId"], 1);
        assert_eq!(json["badgeName"], "Test Badge");
        assert_eq!(json["badgeType"], "NORMAL");
        // expires_at 为 None 时不应出现在 JSON 中
        assert!(!json.as_object().unwrap().contains_key("expiresAt"));
    }

    #[test]
    fn test_badge_wall_dto_structure() {
        let wall = BadgeWallDto {
            total_count: 5,
            categories: vec![BadgeWallCategoryDto {
                category_id: 1,
                category_name: "Trading".to_string(),
                badges: vec![],
            }],
        };

        let json = serde_json::to_value(&wall).unwrap();
        assert_eq!(json["totalCount"], 5);
        assert!(json["categories"].is_array());
    }

    #[test]
    fn test_user_badge_stats_dto() {
        let stats = UserBadgeStatsDto {
            total_badges: 10,
            active_badges: 8,
            expired_badges: 1,
            redeemed_badges: 1,
            by_category: vec![CategoryStatsDto {
                category_id: 1,
                category_name: "Achievement".to_string(),
                count: 5,
            }],
        };

        let json = serde_json::to_value(&stats).unwrap();
        assert_eq!(json["totalBadges"], 10);
        assert_eq!(json["activeBadges"], 8);
    }

    #[test]
    fn test_grant_badge_request_serialization() {
        let request = GrantBadgeRequest::new("user-123", 1, 2)
            .with_idempotency_key("req-001")
            .with_source(SourceType::Event, Some("event-123".to_string()));

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["userId"], "user-123");
        assert_eq!(json["badgeId"], 1);
        assert_eq!(json["quantity"], 2);
        assert_eq!(json["sourceType"], "EVENT");
        assert_eq!(json["idempotencyKey"], "req-001");
    }

    #[test]
    fn test_grant_badge_response_success() {
        let response = GrantBadgeResponse::success(100, 5);
        assert!(response.success);
        assert_eq!(response.user_badge_id, 100);
        assert_eq!(response.new_quantity, 5);
    }

    #[test]
    fn test_batch_grant_response() {
        let response = BatchGrantResponse {
            total: 3,
            success_count: 2,
            failed_count: 1,
            results: vec![
                GrantResult::success("user-1".to_string(), 1, 100, 1),
                GrantResult::success("user-2".to_string(), 1, 101, 1),
                GrantResult::failure("user-3".to_string(), 1, "库存不足"),
            ],
        };

        assert_eq!(response.total, 3);
        assert_eq!(response.success_count, 2);
        assert_eq!(response.failed_count, 1);
        assert!(response.results[0].success);
        assert!(!response.results[2].success);
        assert_eq!(response.results[2].error, Some("库存不足".to_string()));
    }
}

// ==================== 取消服务 DTO ====================

/// 徽章取消请求
///
/// 用于撤销/取消用户已持有的徽章
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevokeBadgeRequest {
    /// 用户 ID
    pub user_id: String,
    /// 徽章 ID
    pub badge_id: i64,
    /// 取消数量
    pub quantity: i32,
    /// 取消原因（必填）
    pub reason: String,
    /// 操作人（手动取消时使用）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<String>,
    /// 来源类型
    pub source_type: SourceType,
    /// 来源关联 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_ref_id: Option<String>,
}

impl RevokeBadgeRequest {
    /// 创建手动取消请求
    pub fn manual(
        user_id: impl Into<String>,
        badge_id: i64,
        quantity: i32,
        reason: impl Into<String>,
        operator: impl Into<String>,
    ) -> Self {
        Self {
            user_id: user_id.into(),
            badge_id,
            quantity,
            reason: reason.into(),
            operator: Some(operator.into()),
            source_type: SourceType::Manual,
            source_ref_id: None,
        }
    }

    /// 创建系统取消请求
    pub fn system(
        user_id: impl Into<String>,
        badge_id: i64,
        quantity: i32,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            user_id: user_id.into(),
            badge_id,
            quantity,
            reason: reason.into(),
            operator: None,
            source_type: SourceType::System,
            source_ref_id: None,
        }
    }

    /// 设置来源引用 ID
    pub fn with_source_ref(mut self, ref_id: impl Into<String>) -> Self {
        self.source_ref_id = Some(ref_id.into());
        self
    }
}

/// 徽章取消响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevokeBadgeResponse {
    /// 是否成功
    pub success: bool,
    /// 取消后剩余数量
    pub remaining_quantity: i32,
    /// 响应消息
    pub message: String,
}

impl RevokeBadgeResponse {
    pub fn success(remaining_quantity: i32) -> Self {
        Self {
            success: true,
            remaining_quantity,
            message: "徽章取消成功".to_string(),
        }
    }

    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            remaining_quantity: 0,
            message: message.into(),
        }
    }
}

/// 批量取消响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRevokeResponse {
    /// 总请求数
    pub total: i32,
    /// 成功数量
    pub success_count: i32,
    /// 失败数量
    pub failed_count: i32,
    /// 各请求的处理结果
    pub results: Vec<RevokeResult>,
}

/// 单个取消结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevokeResult {
    /// 用户 ID
    pub user_id: String,
    /// 徽章 ID
    pub badge_id: i64,
    /// 是否成功
    pub success: bool,
    /// 剩余数量（成功时返回）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_quantity: Option<i32>,
    /// 错误信息（失败时返回）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl RevokeResult {
    pub fn success(user_id: String, badge_id: i64, remaining_quantity: i32) -> Self {
        Self {
            user_id,
            badge_id,
            success: true,
            remaining_quantity: Some(remaining_quantity),
            error: None,
        }
    }

    pub fn failure(user_id: String, badge_id: i64, error: impl Into<String>) -> Self {
        Self {
            user_id,
            badge_id,
            success: false,
            remaining_quantity: None,
            error: Some(error.into()),
        }
    }
}

// ==================== 兑换服务 DTO ====================

/// 徽章兑换请求
///
/// 用于兑换徽章换取权益的请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedeemBadgeRequest {
    /// 用户 ID
    pub user_id: String,
    /// 兑换规则 ID
    pub rule_id: i64,
    /// 幂等键（防止重复提交）
    pub idempotency_key: String,
}

impl RedeemBadgeRequest {
    /// 创建兑换请求
    pub fn new(
        user_id: impl Into<String>,
        rule_id: i64,
        idempotency_key: impl Into<String>,
    ) -> Self {
        Self {
            user_id: user_id.into(),
            rule_id,
            idempotency_key: idempotency_key.into(),
        }
    }
}

/// 徽章兑换响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedeemBadgeResponse {
    /// 是否成功
    pub success: bool,
    /// 订单 ID
    pub order_id: i64,
    /// 订单号
    pub order_no: String,
    /// 权益名称
    pub benefit_name: String,
    /// 响应消息
    pub message: String,
}

impl RedeemBadgeResponse {
    pub fn success(order_id: i64, order_no: String, benefit_name: String) -> Self {
        Self {
            success: true,
            order_id,
            order_no,
            benefit_name,
            message: "兑换成功".to_string(),
        }
    }

    pub fn from_existing(
        order_id: i64,
        order_no: String,
        benefit_name: String,
        status: crate::models::OrderStatus,
    ) -> Self {
        Self {
            success: status == crate::models::OrderStatus::Success,
            order_id,
            order_no,
            benefit_name,
            message: "幂等请求，返回已存在的订单".to_string(),
        }
    }

    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            order_id: 0,
            order_no: String::new(),
            benefit_name: String::new(),
            message: message.into(),
        }
    }
}

/// 兑换历史 DTO
///
/// 用于展示用户的兑换记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedemptionHistoryDto {
    /// 订单 ID
    pub order_id: i64,
    /// 订单号
    pub order_no: String,
    /// 权益名称
    pub benefit_name: String,
    /// 订单状态
    pub status: crate::models::OrderStatus,
    /// 消耗的徽章列表
    pub consumed_badges: Vec<ConsumedBadgeDto>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 消耗的徽章 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsumedBadgeDto {
    /// 徽章 ID
    pub badge_id: i64,
    /// 徽章名称
    pub badge_name: String,
    /// 消耗数量
    pub quantity: i32,
}
