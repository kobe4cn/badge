//! 徽章服务错误类型
//!
//! 定义服务层的业务错误和系统错误

use thiserror::Error;

/// 徽章服务错误类型
#[derive(Debug, Error)]
pub enum BadgeError {
    // === 徽章相关错误 ===
    #[error("徽章不存在: {0}")]
    BadgeNotFound(i64),

    #[error("徽章已下线: {0}")]
    BadgeInactive(i64),

    #[error("徽章库存不足: badge_id={0}")]
    BadgeOutOfStock(i64),

    #[error("徽章系列不存在: {0}")]
    SeriesNotFound(i64),

    #[error("徽章分类不存在: {0}")]
    CategoryNotFound(i64),

    // === 用户徽章相关错误 ===
    #[error("用户徽章不存在: user_id={user_id}, badge_id={badge_id}")]
    UserBadgeNotFound { user_id: String, badge_id: i64 },

    #[error("用户徽章已过期: user_badge_id={0}")]
    UserBadgeExpired(i64),

    #[error("用户徽章数量不足: 需要 {required}, 可用 {available}")]
    InsufficientBadges { required: i32, available: i32 },

    #[error("用户已达到徽章获取上限: badge_id={badge_id}, limit={limit}")]
    BadgeAcquisitionLimitReached { badge_id: i64, limit: i32 },

    // === 兑换相关错误 ===
    #[error("兑换规则不存在: {0}")]
    RedemptionRuleNotFound(i64),

    #[error("兑换规则未生效: rule_id={0}")]
    RedemptionRuleInactive(i64),

    #[error("权益不存在: {0}")]
    BenefitNotFound(i64),

    #[error("权益库存不足: benefit_id={0}")]
    BenefitOutOfStock(i64),

    #[error("已达到兑换频率限制: rule_id={rule_id}, 限制类型={limit_type}")]
    RedemptionFrequencyLimitReached { rule_id: i64, limit_type: String },

    #[error("兑换订单不存在: {0}")]
    OrderNotFound(i64),

    #[error("订单状态不允许此操作: order_id={order_id}, current_status={current_status}")]
    InvalidOrderStatus {
        order_id: i64,
        current_status: String,
    },

    #[error("重复的兑换请求: idempotency_key={0}")]
    DuplicateRedemption(String),

    // === 系统错误 ===
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),

    #[error("JSON 序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Redis 错误: {0}")]
    Redis(String),

    #[error("内部错误: {0}")]
    Internal(String),

    #[error("参数校验失败: {0}")]
    Validation(String),

    #[error("并发冲突，请重试")]
    ConcurrencyConflict,
}

/// 徽章服务 Result 类型别名
pub type Result<T> = std::result::Result<T, BadgeError>;

impl BadgeError {
    /// 检查是否为可重试的错误
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Database(_) | Self::Redis(_) | Self::ConcurrencyConflict
        )
    }

    /// 检查是否为业务错误（非系统错误）
    pub fn is_business_error(&self) -> bool {
        !matches!(
            self,
            Self::Database(_)
                | Self::Serialization(_)
                | Self::Redis(_)
                | Self::Internal(_)
                | Self::ConcurrencyConflict
        )
    }

    /// 获取错误码（用于 API 响应）
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::BadgeNotFound(_) => "BADGE_NOT_FOUND",
            Self::BadgeInactive(_) => "BADGE_INACTIVE",
            Self::BadgeOutOfStock(_) => "BADGE_OUT_OF_STOCK",
            Self::SeriesNotFound(_) => "SERIES_NOT_FOUND",
            Self::CategoryNotFound(_) => "CATEGORY_NOT_FOUND",
            Self::UserBadgeNotFound { .. } => "USER_BADGE_NOT_FOUND",
            Self::UserBadgeExpired(_) => "USER_BADGE_EXPIRED",
            Self::InsufficientBadges { .. } => "INSUFFICIENT_BADGES",
            Self::BadgeAcquisitionLimitReached { .. } => "ACQUISITION_LIMIT_REACHED",
            Self::RedemptionRuleNotFound(_) => "REDEMPTION_RULE_NOT_FOUND",
            Self::RedemptionRuleInactive(_) => "REDEMPTION_RULE_INACTIVE",
            Self::BenefitNotFound(_) => "BENEFIT_NOT_FOUND",
            Self::BenefitOutOfStock(_) => "BENEFIT_OUT_OF_STOCK",
            Self::RedemptionFrequencyLimitReached { .. } => "FREQUENCY_LIMIT_REACHED",
            Self::OrderNotFound(_) => "ORDER_NOT_FOUND",
            Self::InvalidOrderStatus { .. } => "INVALID_ORDER_STATUS",
            Self::DuplicateRedemption(_) => "DUPLICATE_REDEMPTION",
            Self::Database(_) => "DATABASE_ERROR",
            Self::Serialization(_) => "SERIALIZATION_ERROR",
            Self::Redis(_) => "REDIS_ERROR",
            Self::Internal(_) => "INTERNAL_ERROR",
            Self::Validation(_) => "VALIDATION_ERROR",
            Self::ConcurrencyConflict => "CONCURRENCY_CONFLICT",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_is_retryable() {
        assert!(BadgeError::ConcurrencyConflict.is_retryable());
        assert!(BadgeError::Redis("connection failed".to_string()).is_retryable());
        assert!(!BadgeError::BadgeNotFound(1).is_retryable());
        assert!(
            !BadgeError::InsufficientBadges {
                required: 5,
                available: 3
            }
            .is_retryable()
        );
    }

    #[test]
    fn test_error_is_business_error() {
        assert!(BadgeError::BadgeNotFound(1).is_business_error());
        assert!(
            BadgeError::InsufficientBadges {
                required: 5,
                available: 3
            }
            .is_business_error()
        );
        assert!(!BadgeError::Internal("panic".to_string()).is_business_error());
        assert!(!BadgeError::ConcurrencyConflict.is_business_error());
    }

    #[test]
    fn test_error_code() {
        assert_eq!(BadgeError::BadgeNotFound(1).error_code(), "BADGE_NOT_FOUND");
        assert_eq!(
            BadgeError::InsufficientBadges {
                required: 5,
                available: 3
            }
            .error_code(),
            "INSUFFICIENT_BADGES"
        );
        assert_eq!(
            BadgeError::ConcurrencyConflict.error_code(),
            "CONCURRENCY_CONFLICT"
        );
    }

    #[test]
    fn test_error_display() {
        let err = BadgeError::UserBadgeNotFound {
            user_id: "user-123".to_string(),
            badge_id: 1,
        };
        assert!(err.to_string().contains("user-123"));
        assert!(err.to_string().contains("1"));

        let err = BadgeError::InsufficientBadges {
            required: 5,
            available: 3,
        };
        assert!(err.to_string().contains("5"));
        assert!(err.to_string().contains("3"));
    }
}
