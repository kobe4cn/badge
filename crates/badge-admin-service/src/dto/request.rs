//! B端服务请求 DTO 定义
//!
//! 所有 REST API 的请求参数和请求体结构

use badge_management::{BadgeAssets, BadgeStatus, BadgeType, SourceType, ValidityConfig};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use validator::Validate;

/// 创建徽章分类请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateCategoryRequest {
    #[validate(length(min = 1, max = 50, message = "分类名称长度必须在1-50个字符之间"))]
    pub name: String,
    pub icon_url: Option<String>,
    pub sort_order: Option<i32>,
}

/// 更新徽章分类请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCategoryRequest {
    #[validate(length(min = 1, max = 50, message = "分类名称长度必须在1-50个字符之间"))]
    pub name: Option<String>,
    pub icon_url: Option<String>,
    pub sort_order: Option<i32>,
}

/// 创建徽章系列请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateSeriesRequest {
    pub category_id: i64,
    #[validate(length(min = 1, max = 100, message = "系列名称长度必须在1-100个字符之间"))]
    pub name: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

/// 更新徽章系列请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSeriesRequest {
    pub category_id: Option<i64>,
    #[validate(length(min = 1, max = 100, message = "系列名称长度必须在1-100个字符之间"))]
    pub name: Option<String>,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

/// 创建徽章请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateBadgeRequest {
    pub series_id: i64,
    pub badge_type: BadgeType,
    #[validate(length(min = 1, max = 100, message = "徽章名称长度必须在1-100个字符之间"))]
    pub name: String,
    pub description: Option<String>,
    pub obtain_description: Option<String>,
    pub assets: BadgeAssets,
    pub validity_config: ValidityConfig,
    pub max_supply: Option<i32>,
}

/// 更新徽章请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBadgeRequest {
    #[validate(length(min = 1, max = 100, message = "徽章名称长度必须在1-100个字符之间"))]
    pub name: Option<String>,
    pub description: Option<String>,
    pub obtain_description: Option<String>,
    pub assets: Option<BadgeAssets>,
    pub validity_config: Option<ValidityConfig>,
    pub max_supply: Option<i32>,
    pub status: Option<BadgeStatus>,
}

/// 创建规则请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateRuleRequest {
    pub badge_id: i64,
    pub rule_json: serde_json::Value,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub max_count_per_user: Option<i32>,
}

/// 更新规则请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRuleRequest {
    pub rule_json: Option<serde_json::Value>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub max_count_per_user: Option<i32>,
    pub enabled: Option<bool>,
}

/// 手动发放请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ManualGrantRequest {
    pub user_id: String,
    pub badge_id: i64,
    #[validate(range(min = 1, max = 100, message = "单次发放数量必须在1-100之间"))]
    pub quantity: i32,
    #[validate(length(min = 1, max = 500, message = "发放原因不能为空且不超过500字符"))]
    pub reason: String,
}

/// 批量发放请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct BatchGrantRequest {
    pub badge_id: i64,
    /// OSS 文件地址，包含用户列表
    #[validate(url(message = "文件地址必须是有效的URL"))]
    pub file_url: String,
    #[validate(length(min = 1, max = 500, message = "发放原因不能为空且不超过500字符"))]
    pub reason: String,
}

/// 手动取消请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ManualRevokeRequest {
    pub user_id: String,
    pub badge_id: i64,
    #[validate(range(min = 1, max = 100, message = "单次取消数量必须在1-100之间"))]
    pub quantity: i32,
    #[validate(length(min = 1, max = 500, message = "取消原因不能为空且不超过500字符"))]
    pub reason: String,
}

/// 批量取消请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct BatchRevokeRequest {
    pub badge_id: i64,
    /// OSS 文件地址，包含用户列表
    #[validate(url(message = "文件地址必须是有效的URL"))]
    pub file_url: String,
    #[validate(length(min = 1, max = 500, message = "取消原因不能为空且不超过500字符"))]
    pub reason: String,
}

/// 分页查询参数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    20
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: default_page(),
            page_size: default_page_size(),
        }
    }
}

impl PaginationParams {
    /// 计算数据库查询的 offset
    pub fn offset(&self) -> i64 {
        (self.page - 1).max(0) * self.page_size
    }

    /// 获取限制条数（最大100）
    pub fn limit(&self) -> i64 {
        self.page_size.clamp(1, 100)
    }
}

/// 徽章查询过滤
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgeQueryFilter {
    pub category_id: Option<i64>,
    pub series_id: Option<i64>,
    pub badge_type: Option<BadgeType>,
    pub status: Option<BadgeStatus>,
    pub keyword: Option<String>,
}

/// 发放记录查询过滤
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantLogFilter {
    pub user_id: Option<String>,
    pub badge_id: Option<i64>,
    pub source_type: Option<SourceType>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

/// 操作日志查询过滤
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationLogFilter {
    pub operator_id: Option<String>,
    pub module: Option<String>,
    pub action: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

/// 批量任务查询过滤
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchTaskFilter {
    pub task_type: Option<String>,
    pub status: Option<String>,
    pub created_by: Option<String>,
}

/// 统计时间范围参数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeRangeParams {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_params_default() {
        let params = PaginationParams::default();
        assert_eq!(params.page, 1);
        assert_eq!(params.page_size, 20);
    }

    #[test]
    fn test_pagination_offset() {
        let params = PaginationParams {
            page: 3,
            page_size: 10,
        };
        assert_eq!(params.offset(), 20);
        assert_eq!(params.limit(), 10);
    }

    #[test]
    fn test_pagination_offset_edge_cases() {
        let params = PaginationParams {
            page: 0,
            page_size: 10,
        };
        // page 为 0 时，offset 应该为 0
        assert_eq!(params.offset(), 0);

        let params = PaginationParams {
            page: 1,
            page_size: 200,
        };
        // page_size 超过100时应被限制
        assert_eq!(params.limit(), 100);
    }

    #[test]
    fn test_create_badge_request_validation() {
        use validator::Validate;

        let request = CreateBadgeRequest {
            series_id: 1,
            badge_type: BadgeType::Normal,
            name: "".to_string(), // 空名称应该失败
            description: None,
            obtain_description: None,
            assets: BadgeAssets {
                icon_url: "https://example.com/icon.png".to_string(),
                image_url: None,
                animation_url: None,
                disabled_icon_url: None,
            },
            validity_config: ValidityConfig::default(),
            max_supply: None,
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn test_manual_grant_request_validation() {
        use validator::Validate;

        let valid_request = ManualGrantRequest {
            user_id: "user123".to_string(),
            badge_id: 1,
            quantity: 5,
            reason: "测试发放".to_string(),
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = ManualGrantRequest {
            user_id: "user123".to_string(),
            badge_id: 1,
            quantity: 0, // 数量为0应该失败
            reason: "测试发放".to_string(),
        };
        assert!(invalid_request.validate().is_err());
    }
}
