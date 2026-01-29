//! 统一错误处理模块
//!
//! 定义系统中所有共享的错误类型，使用 thiserror 提供良好的错误信息。

use thiserror::Error;

/// 系统错误类型
#[derive(Debug, Error)]
pub enum BadgeError {
    // ==================== 数据库错误 ====================
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),

    #[error("记录未找到: {entity} id={id}")]
    NotFound { entity: String, id: String },

    #[error("记录已存在: {entity} {field}={value}")]
    AlreadyExists {
        entity: String,
        field: String,
        value: String,
    },

    // ==================== 缓存错误 ====================
    #[error("Redis 错误: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("缓存未命中: {key}")]
    CacheMiss { key: String },

    // ==================== Kafka 错误 ====================
    #[error("Kafka 错误: {0}")]
    Kafka(String),

    // ==================== 业务逻辑错误 ====================
    #[error("徽章余额不足: 需要 {required}, 实际 {actual}")]
    InsufficientBalance { required: i32, actual: i32 },

    #[error("徽章已过期: badge_id={badge_id}")]
    BadgeExpired { badge_id: String },

    #[error("兑换条件不满足: {reason}")]
    RedemptionConditionNotMet { reason: String },

    #[error("操作频率超限: {operation}")]
    RateLimitExceeded { operation: String },

    #[error("徽章不可用: {reason}")]
    BadgeUnavailable { reason: String },

    // ==================== 规则引擎错误 ====================
    #[error("规则解析失败: {0}")]
    RuleParseFailed(String),

    #[error("规则执行失败: {0}")]
    RuleExecutionFailed(String),

    #[error("规则未找到: rule_id={rule_id}")]
    RuleNotFound { rule_id: String },

    // ==================== 验证错误 ====================
    #[error("参数验证失败: {0}")]
    Validation(String),

    #[error("无效的参数: {field} - {message}")]
    InvalidArgument { field: String, message: String },

    // ==================== 权限错误 ====================
    #[error("未授权访问")]
    Unauthorized,

    #[error("权限不足: {operation}")]
    Forbidden { operation: String },

    // ==================== 外部服务错误 ====================
    #[error("外部服务错误: {service} - {message}")]
    ExternalService { service: String, message: String },

    #[error("外部服务超时: {service}")]
    ExternalServiceTimeout { service: String },

    // ==================== 通用错误 ====================
    #[error("内部错误: {0}")]
    Internal(String),

    #[error("{0}")]
    Custom(String),
}

/// 错误结果类型别名
pub type Result<T> = std::result::Result<T, BadgeError>;

impl BadgeError {
    /// 获取错误码
    pub fn code(&self) -> &'static str {
        match self {
            Self::Database(_) => "DATABASE_ERROR",
            Self::NotFound { .. } => "NOT_FOUND",
            Self::AlreadyExists { .. } => "ALREADY_EXISTS",
            Self::Redis(_) => "REDIS_ERROR",
            Self::CacheMiss { .. } => "CACHE_MISS",
            Self::Kafka(_) => "KAFKA_ERROR",
            Self::InsufficientBalance { .. } => "INSUFFICIENT_BALANCE",
            Self::BadgeExpired { .. } => "BADGE_EXPIRED",
            Self::RedemptionConditionNotMet { .. } => "REDEMPTION_CONDITION_NOT_MET",
            Self::RateLimitExceeded { .. } => "RATE_LIMIT_EXCEEDED",
            Self::BadgeUnavailable { .. } => "BADGE_UNAVAILABLE",
            Self::RuleParseFailed(_) => "RULE_PARSE_FAILED",
            Self::RuleExecutionFailed(_) => "RULE_EXECUTION_FAILED",
            Self::RuleNotFound { .. } => "RULE_NOT_FOUND",
            Self::Validation(_) => "VALIDATION_ERROR",
            Self::InvalidArgument { .. } => "INVALID_ARGUMENT",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden { .. } => "FORBIDDEN",
            Self::ExternalService { .. } => "EXTERNAL_SERVICE_ERROR",
            Self::ExternalServiceTimeout { .. } => "EXTERNAL_SERVICE_TIMEOUT",
            Self::Internal(_) => "INTERNAL_ERROR",
            Self::Custom(_) => "CUSTOM_ERROR",
        }
    }

    /// 是否为可重试错误
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Database(_)
                | Self::Redis(_)
                | Self::Kafka(_)
                | Self::ExternalServiceTimeout { .. }
        )
    }

    /// 转换为 gRPC 状态码
    pub fn to_grpc_status(&self) -> tonic::Status {
        use tonic::{Code, Status};

        let (code, message) = match self {
            Self::NotFound { .. } => (Code::NotFound, self.to_string()),
            Self::AlreadyExists { .. } => (Code::AlreadyExists, self.to_string()),
            Self::Validation(_) | Self::InvalidArgument { .. } => {
                (Code::InvalidArgument, self.to_string())
            }
            Self::Unauthorized => (Code::Unauthenticated, self.to_string()),
            Self::Forbidden { .. } => (Code::PermissionDenied, self.to_string()),
            Self::RateLimitExceeded { .. } => (Code::ResourceExhausted, self.to_string()),
            Self::ExternalServiceTimeout { .. } => (Code::DeadlineExceeded, self.to_string()),
            _ => (Code::Internal, self.to_string()),
        };

        Status::new(code, message)
    }
}

impl From<BadgeError> for tonic::Status {
    fn from(err: BadgeError) -> Self {
        err.to_grpc_status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code() {
        let err = BadgeError::NotFound {
            entity: "Badge".to_string(),
            id: "123".to_string(),
        };
        assert_eq!(err.code(), "NOT_FOUND");
    }

    #[test]
    fn test_is_retryable() {
        let db_err = BadgeError::Database(sqlx::Error::PoolTimedOut);
        assert!(db_err.is_retryable());

        let not_found = BadgeError::NotFound {
            entity: "Badge".to_string(),
            id: "123".to_string(),
        };
        assert!(!not_found.is_retryable());
    }
}
