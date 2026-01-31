//! B端服务响应 DTO 定义
//!
//! 所有 REST API 的响应体结构

use badge_management::{
    BadgeAssets, BadgeStatus, BadgeType, CategoryStatus, SourceType, ValidityConfig,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 分页响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

impl<T> PageResponse<T> {
    /// 创建分页响应
    pub fn new(items: Vec<T>, total: i64, page: i64, page_size: i64) -> Self {
        let total_pages = if page_size > 0 {
            (total + page_size - 1) / page_size
        } else {
            0
        };

        Self {
            items,
            total,
            page,
            page_size,
            total_pages,
        }
    }

    /// 创建空分页响应
    pub fn empty(page: i64, page_size: i64) -> Self {
        Self {
            items: Vec::new(),
            total: 0,
            page,
            page_size,
            total_pages: 0,
        }
    }
}

/// API 统一响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse<T> {
    pub success: bool,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    /// 创建成功响应
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            code: "SUCCESS".to_string(),
            message: "操作成功".to_string(),
            data: Some(data),
        }
    }

    /// 创建成功响应（无数据）
    pub fn success_empty() -> ApiResponse<()> {
        ApiResponse {
            success: true,
            code: "SUCCESS".to_string(),
            message: "操作成功".to_string(),
            data: None,
        }
    }

    /// 创建成功响应（自定义消息）
    pub fn success_with_message(data: T, message: impl Into<String>) -> Self {
        Self {
            success: true,
            code: "SUCCESS".to_string(),
            message: message.into(),
            data: Some(data),
        }
    }

    /// 创建错误响应
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            code: code.into(),
            message: message.into(),
            data: None,
        }
    }
}

/// 分类响应 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryDto {
    pub id: i64,
    pub name: String,
    pub icon_url: Option<String>,
    pub sort_order: i32,
    pub status: CategoryStatus,
    pub badge_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 系列响应 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesDto {
    pub id: i64,
    pub category_id: i64,
    pub category_name: String,
    pub name: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub sort_order: i32,
    pub status: CategoryStatus,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub badge_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 徽章详情响应（B端完整信息）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgeAdminDto {
    pub id: i64,
    pub series_id: i64,
    pub series_name: String,
    pub category_id: i64,
    pub category_name: String,
    pub badge_type: BadgeType,
    pub name: String,
    pub description: Option<String>,
    pub obtain_description: Option<String>,
    pub assets: BadgeAssets,
    pub validity_config: ValidityConfig,
    pub max_supply: Option<i32>,
    pub issued_count: i32,
    pub status: BadgeStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 徽章列表项 DTO（精简版）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgeListItemDto {
    pub id: i64,
    pub name: String,
    pub badge_type: BadgeType,
    pub status: BadgeStatus,
    pub icon_url: String,
    pub series_name: String,
    pub category_name: String,
    pub issued_count: i32,
    pub max_supply: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// 规则响应 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleDto {
    pub id: i64,
    pub badge_id: i64,
    pub badge_name: String,
    pub rule_json: serde_json::Value,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub max_count_per_user: Option<i32>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 发放记录响应 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantLogDto {
    pub id: i64,
    pub user_id: String,
    pub badge_id: i64,
    pub badge_name: String,
    pub quantity: i32,
    pub source_type: SourceType,
    pub source_id: Option<String>,
    pub reason: Option<String>,
    pub operator_id: Option<String>,
    pub operator_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 用户徽章视图 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserBadgeViewDto {
    pub user_id: String,
    pub badge_id: i64,
    pub badge_name: String,
    pub badge_type: BadgeType,
    pub icon_url: String,
    pub quantity: i32,
    pub first_acquired_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 统计概览
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsOverview {
    pub total_badges: i64,
    pub active_badges: i64,
    pub total_issued: i64,
    pub total_redeemed: i64,
    pub today_issued: i64,
    pub today_redeemed: i64,
}

/// 趋势数据点
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendDataPoint {
    pub date: String,
    pub issued_count: i64,
    pub redeemed_count: i64,
}

/// 徽章统计详情
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgeStatsDto {
    pub badge_id: i64,
    pub badge_name: String,
    pub total_issued: i64,
    pub total_redeemed: i64,
    pub unique_holders: i64,
    pub today_issued: i64,
    pub today_redeemed: i64,
    /// 近期趋势数据
    pub daily_trends: Vec<TrendDataPoint>,
}

/// 徽章排行 DTO
///
/// 按发放量排名，包含发放/兑换/持有人数统计
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgeRankingDto {
    pub badge_id: i64,
    pub badge_name: String,
    pub badge_type: String,
    pub total_issued: i64,
    pub total_redeemed: i64,
    pub active_holders: i64,
}

/// 用户徽章管理视图 DTO
///
/// B端查看用户持有徽章时使用，包含有效期信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserBadgeAdminDto {
    pub badge_id: i64,
    pub badge_name: String,
    pub badge_type: String,
    pub quantity: i32,
    pub status: String,
    pub acquired_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// 用户兑换记录 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRedemptionDto {
    pub order_id: i64,
    pub order_no: String,
    pub benefit_name: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// 用户统计 DTO
///
/// 汇总用户的徽章持有和兑换情况
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserStatsDto {
    pub user_id: String,
    pub total_badges: i64,
    pub active_badges: i64,
    pub expired_badges: i64,
    pub total_redeemed: i64,
}

/// 用户账本流水 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserLedgerDto {
    pub id: i64,
    pub badge_id: i64,
    pub badge_name: String,
    pub change_type: String,
    pub source_type: String,
    pub quantity: i32,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 操作日志响应 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationLogDto {
    pub id: i64,
    pub operator_id: String,
    pub operator_name: Option<String>,
    pub module: String,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub before_data: Option<serde_json::Value>,
    pub after_data: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 批量任务状态响应 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchTaskDto {
    pub id: i64,
    pub task_type: String,
    pub status: String,
    pub total_count: i32,
    pub success_count: i32,
    pub failure_count: i32,
    pub progress: i32,
    pub file_url: Option<String>,
    pub result_file_url: Option<String>,
    pub error_message: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建资源成功响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedResponse {
    pub id: i64,
}

impl CreatedResponse {
    pub fn new(id: i64) -> Self {
        Self { id }
    }
}

/// 删除成功响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletedResponse {
    pub deleted: bool,
}

impl DeletedResponse {
    pub fn success() -> Self {
        Self { deleted: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_response_new() {
        let items = vec![1, 2, 3];
        let response = PageResponse::new(items, 100, 2, 10);

        assert_eq!(response.total, 100);
        assert_eq!(response.page, 2);
        assert_eq!(response.page_size, 10);
        assert_eq!(response.total_pages, 10);
        assert_eq!(response.items.len(), 3);
    }

    #[test]
    fn test_page_response_total_pages_calculation() {
        // 恰好整除
        let response = PageResponse::<i32>::new(vec![], 100, 1, 10);
        assert_eq!(response.total_pages, 10);

        // 有余数
        let response = PageResponse::<i32>::new(vec![], 101, 1, 10);
        assert_eq!(response.total_pages, 11);

        // 空数据
        let response = PageResponse::<i32>::empty(1, 10);
        assert_eq!(response.total_pages, 0);
    }

    #[test]
    fn test_api_response_success() {
        let response = ApiResponse::success("test data");
        assert!(response.success);
        assert_eq!(response.code, "SUCCESS");
        assert_eq!(response.data, Some("test data"));
    }

    #[test]
    fn test_api_response_error() {
        let response = ApiResponse::<()>::error("TEST_ERROR", "测试错误");
        assert!(!response.success);
        assert_eq!(response.code, "TEST_ERROR");
        assert_eq!(response.message, "测试错误");
        assert!(response.data.is_none());
    }

    #[test]
    fn test_api_response_serialization() {
        let response = ApiResponse::success(CreatedResponse::new(123));
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"id\":123"));
    }
}
