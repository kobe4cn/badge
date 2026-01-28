//! 服务层数据传输对象
//!
//! 定义服务层与外部交互使用的 DTO，与内部领域模型解耦

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::{BadgeAssets, BadgeType, BenefitType, UserBadgeStatus, ValidityConfig};

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
}
