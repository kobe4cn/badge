//! B端服务请求 DTO 定义
//!
//! 所有 REST API 的请求参数和请求体结构

use badge_management::{BadgeAssets, BadgeStatus, BadgeType, SourceType, ValidityConfig};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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
    /// 业务唯一编码（可选）
    #[validate(length(max = 50, message = "业务编码长度不能超过50个字符"))]
    pub code: Option<String>,
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
    /// 业务唯一编码
    #[validate(length(max = 50, message = "业务编码长度不能超过50个字符"))]
    pub code: Option<String>,
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
    /// 规则唯一标识码，用于规则引擎匹配
    #[validate(length(min = 1, max = 100, message = "规则编码长度必须在1-100个字符之间"))]
    pub rule_code: String,
    /// 规则名称（显示用）
    #[validate(length(min = 1, max = 200, message = "规则名称长度必须在1-200个字符之间"))]
    pub name: String,
    /// 规则描述（可选）
    pub description: Option<String>,
    /// 关联的事件类型，必须存在于 event_types 表中
    #[validate(length(min = 1, max = 50, message = "事件类型长度必须在1-50个字符之间"))]
    pub event_type: String,
    pub rule_json: serde_json::Value,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub max_count_per_user: Option<i32>,
    /// 全局发放配额限制
    pub global_quota: Option<i32>,
}

/// 更新规则请求
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRuleRequest {
    pub event_type: Option<String>,
    pub rule_code: Option<String>,
    /// 规则名称（显示用）
    #[validate(length(max = 200, message = "规则名称长度不能超过200个字符"))]
    pub name: Option<String>,
    /// 规则描述
    pub description: Option<String>,
    pub global_quota: Option<i32>,
    pub rule_json: Option<serde_json::Value>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub max_count_per_user: Option<i32>,
    pub enabled: Option<bool>,
}

/// 测试规则定义请求（无需持久化，仅做模拟评估）
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestRuleDefinitionRequest {
    pub rule_json: serde_json::Value,
    pub context: Option<serde_json::Value>,
}

/// 发放对象类型
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipientType {
    /// 账号注册人（默认）
    #[default]
    Owner,
    /// 实际使用人
    User,
}

impl RecipientType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecipientType::Owner => "OWNER",
            RecipientType::User => "USER",
        }
    }
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
    /// 发放对象类型：OWNER-账号注册人（默认），USER-实际使用人
    #[serde(default)]
    pub recipient_type: RecipientType,
    /// 实际使用人 ID（当 recipient_type = USER 时必填）
    pub actual_user_id: Option<String>,
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

/// 手动取消请求（兼容前端 userBadgeId 格式）
///
/// 前端发送 { userBadgeId, reason }，后端通过 user_badge_id 反查 user_id 和 badge_id
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ManualRevokeRequest {
    pub user_badge_id: i64,
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

/// 自动取消场景
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AutoRevokeScenario {
    /// 账号注销
    AccountDeletion,
    /// 身份变更（如会员降级、员工离职）
    IdentityChange,
    /// 条件不再满足（如订单退款、活动资格取消）
    ConditionUnmet,
    /// 违规处罚
    Violation,
    /// 其他系统触发
    SystemTriggered,
}

impl std::fmt::Display for AutoRevokeScenario {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AccountDeletion => write!(f, "account_deletion"),
            Self::IdentityChange => write!(f, "identity_change"),
            Self::ConditionUnmet => write!(f, "condition_unmet"),
            Self::Violation => write!(f, "violation"),
            Self::SystemTriggered => write!(f, "system_triggered"),
        }
    }
}

/// 自动取消请求
///
/// 用于账号注销、身份变更、条件不满足等自动触发的撤销场景
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AutoRevokeRequest {
    /// 用户 ID
    #[validate(length(min = 1, max = 100, message = "用户ID不能为空"))]
    pub user_id: String,
    /// 徽章 ID（可选，为空则撤销该用户所有徽章）
    pub badge_id: Option<i64>,
    /// 自动取消场景
    pub scenario: AutoRevokeScenario,
    /// 关联的业务 ID（如订单号、会员变更单号）
    pub ref_id: Option<String>,
    /// 取消原因说明
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
    /// 前端 ProTable 发送 `name` 参数，兼容两种参数名
    #[serde(alias = "name")]
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
    /// 开始日期（格式：YYYY-MM-DD）
    pub start_date: chrono::NaiveDate,
    /// 结束日期（格式：YYYY-MM-DD）
    pub end_date: chrono::NaiveDate,
}

impl TimeRangeParams {
    /// 转换为 UTC 时间戳（开始日期 00:00:00）
    pub fn start_time(&self) -> DateTime<Utc> {
        self.start_date.and_hms_opt(0, 0, 0).unwrap().and_utc()
    }

    /// 转换为 UTC 时间戳（结束日期 23:59:59）
    pub fn end_time(&self) -> DateTime<Utc> {
        self.end_date.and_hms_opt(23, 59, 59).unwrap().and_utc()
    }
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
            code: None,
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
            recipient_type: RecipientType::default(),
            actual_user_id: None,
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = ManualGrantRequest {
            user_id: "user123".to_string(),
            badge_id: 1,
            quantity: 0, // 数量为0应该失败
            reason: "测试发放".to_string(),
            recipient_type: RecipientType::default(),
            actual_user_id: None,
        };
        assert!(invalid_request.validate().is_err());
    }
}
