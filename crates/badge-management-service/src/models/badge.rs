//! 徽章相关实体定义
//!
//! 包含徽章三层结构：Category（大类）-> Series（系列）-> Badge（徽章）

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::enums::{BadgeStatus, BadgeType, CategoryStatus, ValidityType};

/// 徽章大类（一级分类）
///
/// 用于统计和顶层分类，如"交易徽章"、"互动徽章"等
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BadgeCategory {
    pub id: i64,
    /// 分类名称
    pub name: String,
    /// 分类图标 URL
    #[sqlx(default)]
    pub icon_url: Option<String>,
    /// 排序权重，数值越小越靠前
    pub sort_order: i32,
    /// 分类状态
    pub status: CategoryStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 徽章系列（二级分类）
///
/// 用于分组展示，如"2024春节系列"、"周年庆系列"等
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BadgeSeries {
    pub id: i64,
    /// 所属大类 ID
    pub category_id: i64,
    /// 系列名称
    pub name: String,
    /// 系列描述
    #[sqlx(default)]
    pub description: Option<String>,
    /// 系列封面图 URL
    #[sqlx(default)]
    pub cover_url: Option<String>,
    /// 排序权重
    pub sort_order: i32,
    /// 系列状态
    pub status: CategoryStatus,
    /// 系列开始时间（可选，用于限时系列）
    #[sqlx(default)]
    pub start_time: Option<DateTime<Utc>>,
    /// 系列结束时间
    #[sqlx(default)]
    pub end_time: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 有效期配置
///
/// 定义徽章的过期规则，嵌入在 Badge 的 validity_config 字段中
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidityConfig {
    /// 有效期类型
    pub validity_type: ValidityType,
    /// 固定过期日期（当 validity_type = FixedDate 时使用）
    #[serde(default)]
    pub fixed_date: Option<DateTime<Utc>>,
    /// 相对有效天数（当 validity_type = RelativeDays 时使用）
    #[serde(default)]
    pub relative_days: Option<i32>,
}

impl Default for ValidityConfig {
    fn default() -> Self {
        Self {
            validity_type: ValidityType::Permanent,
            fixed_date: None,
            relative_days: None,
        }
    }
}

/// 徽章资源配置
///
/// 存储徽章的各种展示资源
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgeAssets {
    /// 徽章图标（小图）
    pub icon_url: String,
    /// 徽章大图
    #[serde(default)]
    pub image_url: Option<String>,
    /// 动效资源（Lottie 或视频）
    #[serde(default)]
    pub animation_url: Option<String>,
    /// 灰态图标（未获取时展示）
    #[serde(default)]
    pub disabled_icon_url: Option<String>,
}

/// 徽章定义
///
/// 实际发放给用户的徽章实体，包含完整的配置信息
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Badge {
    pub id: i64,
    /// 所属系列 ID
    pub series_id: i64,
    /// 业务唯一编码，用于外部系统对接
    #[sqlx(default)]
    pub code: Option<String>,
    /// 徽章类型
    pub badge_type: BadgeType,
    /// 徽章名称
    pub name: String,
    /// 徽章描述
    #[sqlx(default)]
    pub description: Option<String>,
    /// 获取条件描述（展示给用户）
    #[sqlx(default)]
    pub obtain_description: Option<String>,
    /// 排序权重
    pub sort_order: i32,
    /// 徽章状态
    pub status: BadgeStatus,
    /// 资源配置（JSON）
    /// 存储 BadgeAssets 结构
    pub assets: Value,
    /// 有效期配置（JSON）
    /// 存储 ValidityConfig 结构
    pub validity_config: Value,
    /// 最大发放总量（null 表示不限量）
    #[sqlx(default)]
    pub max_supply: Option<i64>,
    /// 已发放数量
    #[serde(default)]
    pub issued_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Badge {
    /// 解析资源配置
    pub fn parse_assets(&self) -> Result<BadgeAssets, serde_json::Error> {
        serde_json::from_value(self.assets.clone())
    }

    /// 解析有效期配置
    pub fn parse_validity_config(&self) -> Result<ValidityConfig, serde_json::Error> {
        serde_json::from_value(self.validity_config.clone())
    }

    /// 检查是否还有库存
    pub fn has_stock(&self) -> bool {
        match self.max_supply {
            Some(max) => self.issued_count < max,
            None => true,
        }
    }

    /// 检查徽章是否可发放
    pub fn is_issuable(&self) -> bool {
        self.status == BadgeStatus::Active && self.has_stock()
    }
}

/// 徽章获取规则
///
/// 定义用户获取徽章的条件，与规则引擎配合使用
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BadgeRule {
    pub id: i64,
    /// 关联的徽章 ID
    pub badge_id: i64,
    /// 规则定义（JSON，传给规则引擎）
    pub rule_json: Value,
    /// 关联的事件类型编码，决定由哪个事件服务处理
    #[sqlx(default)]
    pub event_type: Option<String>,
    /// 规则唯一编码，用于日志追踪和管理后台展示
    #[sqlx(default)]
    pub rule_code: Option<String>,
    /// 全局配额，限制该规则可发放的徽章总数（NULL 表示不限制）
    #[sqlx(default)]
    pub global_quota: Option<i32>,
    /// 已发放数量，用于配额校验
    #[serde(default)]
    pub global_granted: i32,
    /// 规则显示名称
    #[sqlx(default)]
    pub name: Option<String>,
    /// 规则描述
    #[sqlx(default)]
    pub description: Option<String>,
    /// 规则生效开始时间
    #[sqlx(default)]
    pub start_time: Option<DateTime<Utc>>,
    /// 规则生效结束时间
    #[sqlx(default)]
    pub end_time: Option<DateTime<Utc>>,
    /// 单用户最大获取数量
    #[sqlx(default)]
    pub max_count_per_user: Option<i32>,
    /// 规则是否启用
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl BadgeRule {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_badge_assets_serialization() {
        let assets = BadgeAssets {
            icon_url: "https://example.com/icon.png".to_string(),
            image_url: Some("https://example.com/image.png".to_string()),
            animation_url: None,
            disabled_icon_url: None,
        };

        let json = serde_json::to_value(&assets).unwrap();
        assert_eq!(json["iconUrl"], "https://example.com/icon.png");

        let parsed: BadgeAssets = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.icon_url, assets.icon_url);
    }

    #[test]
    fn test_validity_config_default() {
        let config = ValidityConfig::default();
        assert_eq!(config.validity_type, ValidityType::Permanent);
        assert!(config.fixed_date.is_none());
    }

    #[test]
    fn test_badge_has_stock() {
        let mut badge = create_test_badge();

        // 无限量
        badge.max_supply = None;
        assert!(badge.has_stock());

        // 有库存
        badge.max_supply = Some(100);
        badge.issued_count = 50;
        assert!(badge.has_stock());

        // 无库存
        badge.issued_count = 100;
        assert!(!badge.has_stock());
    }

    #[test]
    fn test_badge_rule_is_active() {
        let now = Utc::now();
        let mut rule = create_test_rule();

        // 启用且无时间限制
        rule.enabled = true;
        rule.start_time = None;
        rule.end_time = None;
        assert!(rule.is_active(now));

        // 禁用
        rule.enabled = false;
        assert!(!rule.is_active(now));

        // 启用但未到开始时间
        rule.enabled = true;
        rule.start_time = Some(now + chrono::Duration::hours(1));
        assert!(!rule.is_active(now));
    }

    fn create_test_badge() -> Badge {
        Badge {
            id: 1,
            series_id: 1,
            code: None,
            badge_type: BadgeType::Normal,
            name: "Test Badge".to_string(),
            description: None,
            obtain_description: None,
            sort_order: 0,
            status: BadgeStatus::Active,
            assets: json!({"iconUrl": "https://example.com/icon.png"}),
            validity_config: json!({"validityType": "PERMANENT"}),
            max_supply: None,
            issued_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_rule() -> BadgeRule {
        BadgeRule {
            id: 1,
            badge_id: 1,
            rule_json: json!({}),
            event_type: None,
            rule_code: None,
            global_quota: None,
            global_granted: 0,
            name: None,
            description: None,
            start_time: None,
            end_time: None,
            max_count_per_user: None,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}
