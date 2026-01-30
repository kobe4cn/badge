//! B端管理后台错误类型定义
//!
//! 包含所有 admin service 特有的错误类型

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// B端管理后台错误类型
#[derive(Debug, thiserror::Error)]
pub enum AdminError {
    // 验证错误
    #[error("参数验证失败: {0}")]
    Validation(String),

    // 资源不存在
    #[error("分类不存在: {0}")]
    CategoryNotFound(i64),
    #[error("系列不存在: {0}")]
    SeriesNotFound(i64),
    #[error("徽章不存在: {0}")]
    BadgeNotFound(i64),
    #[error("规则不存在: {0}")]
    RuleNotFound(i64),
    #[error("任务不存在: {0}")]
    TaskNotFound(i64),
    #[error("依赖关系不存在: {0}")]
    DependencyNotFound(uuid::Uuid),

    // 业务错误
    #[error("徽章已发布，无法删除")]
    BadgeAlreadyPublished,
    #[error("规则 JSON 格式无效: {0}")]
    InvalidRuleJson(String),
    #[error("文件处理失败: {0}")]
    FileProcessingError(String),
    #[error("库存不足")]
    InsufficientStock,
    #[error("用户徽章数量不足")]
    InsufficientUserBadge,

    // 系统错误
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Redis错误: {0}")]
    Redis(String),
    #[error("内部错误: {0}")]
    Internal(String),
}

impl AdminError {
    /// 返回对应的 HTTP 状态码
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::Validation(_) | Self::InvalidRuleJson(_) => StatusCode::BAD_REQUEST,

            Self::CategoryNotFound(_)
            | Self::SeriesNotFound(_)
            | Self::BadgeNotFound(_)
            | Self::RuleNotFound(_)
            | Self::TaskNotFound(_)
            | Self::DependencyNotFound(_) => StatusCode::NOT_FOUND,

            Self::BadgeAlreadyPublished | Self::InsufficientStock | Self::InsufficientUserBadge => {
                StatusCode::CONFLICT
            }

            Self::FileProcessingError(_) => StatusCode::UNPROCESSABLE_ENTITY,

            Self::Database(_) | Self::Redis(_) | Self::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    /// 返回错误码（用于 API 响应）
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Validation(_) => "VALIDATION_ERROR",
            Self::CategoryNotFound(_) => "CATEGORY_NOT_FOUND",
            Self::SeriesNotFound(_) => "SERIES_NOT_FOUND",
            Self::BadgeNotFound(_) => "BADGE_NOT_FOUND",
            Self::RuleNotFound(_) => "RULE_NOT_FOUND",
            Self::TaskNotFound(_) => "TASK_NOT_FOUND",
            Self::DependencyNotFound(_) => "DEPENDENCY_NOT_FOUND",
            Self::BadgeAlreadyPublished => "BADGE_ALREADY_PUBLISHED",
            Self::InvalidRuleJson(_) => "INVALID_RULE_JSON",
            Self::FileProcessingError(_) => "FILE_PROCESSING_ERROR",
            Self::InsufficientStock => "INSUFFICIENT_STOCK",
            Self::InsufficientUserBadge => "INSUFFICIENT_USER_BADGE",
            Self::Database(_) => "DATABASE_ERROR",
            Self::Redis(_) => "REDIS_ERROR",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }
}

impl IntoResponse for AdminError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = json!({
            "success": false,
            "code": self.error_code(),
            "message": self.to_string(),
            "data": serde_json::Value::Null
        });

        (status, axum::Json(body)).into_response()
    }
}

/// 从 validator 错误转换
impl From<validator::ValidationErrors> for AdminError {
    fn from(errors: validator::ValidationErrors) -> Self {
        Self::Validation(errors.to_string())
    }
}

/// 从 badge-management-service 的错误转换
impl From<badge_management::BadgeError> for AdminError {
    fn from(err: badge_management::BadgeError) -> Self {
        match err {
            badge_management::BadgeError::Database(e) => Self::Database(e),
            badge_management::BadgeError::BadgeNotFound(id) => Self::BadgeNotFound(id),
            badge_management::BadgeError::SeriesNotFound(id) => Self::SeriesNotFound(id),
            badge_management::BadgeError::CategoryNotFound(id) => Self::CategoryNotFound(id),
            badge_management::BadgeError::Validation(msg) => Self::Validation(msg),
            other => Self::Internal(other.to_string()),
        }
    }
}

/// 服务层 Result 类型别名
pub type Result<T> = std::result::Result<T, AdminError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(
            AdminError::Validation("test".into()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AdminError::BadgeNotFound(1).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            AdminError::BadgeAlreadyPublished.status_code(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            AdminError::Internal("test".into()).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(
            AdminError::Validation("test".into()).error_code(),
            "VALIDATION_ERROR"
        );
        assert_eq!(AdminError::BadgeNotFound(1).error_code(), "BADGE_NOT_FOUND");
    }
}
