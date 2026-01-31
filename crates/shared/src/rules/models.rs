//! 规则模块的数据模型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 规则对应的徽章发放配置
///
/// 表示一条规则与徽章之间的映射关系，包含发放数量、时间窗口、配额限制等信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadgeGrant {
    pub rule_id: i64,
    pub rule_code: String,
    pub badge_id: i64,
    pub badge_name: String,
    pub quantity: i32,
    pub event_type: String,
    /// 规则生效时间，None 表示立即生效
    pub start_time: Option<DateTime<Utc>>,
    /// 规则失效时间，None 表示永久有效
    pub end_time: Option<DateTime<Utc>>,
    /// 单用户最大获得次数，None 表示不限制
    pub max_count_per_user: Option<i32>,
    /// 全局配额上限，None 表示不限制
    pub global_quota: Option<i32>,
    /// 当前已发放数量
    pub global_granted: i32,
}

/// 规则校验结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub allowed: bool,
    pub rule_id: i64,
    pub rule_code: String,
    pub user_id: String,
    pub reason: ValidationReason,
    pub context: ValidationContext,
}

/// 校验结果原因
///
/// 标识规则校验通过或被拒绝的具体原因，便于上层业务进行针对性处理。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidationReason {
    /// 校验通过
    Allowed,
    /// 规则已过期
    RuleExpired { end_time: DateTime<Utc> },
    /// 规则尚未生效
    RuleNotStarted { start_time: DateTime<Utc> },
    /// 用户已达领取上限
    UserLimitExceeded { current: i32, max: i32 },
    /// 全局配额已耗尽
    GlobalQuotaExhausted { granted: i32, quota: i32 },
}

impl ValidationReason {
    /// 返回拒绝原因的错误码，用于 API 响应
    pub fn deny_code(&self) -> Option<&'static str> {
        match self {
            ValidationReason::Allowed => None,
            ValidationReason::RuleExpired { .. } => Some("RULE_EXPIRED"),
            ValidationReason::RuleNotStarted { .. } => Some("RULE_NOT_STARTED"),
            ValidationReason::UserLimitExceeded { .. } => Some("USER_LIMIT_EXCEEDED"),
            ValidationReason::GlobalQuotaExhausted { .. } => Some("GLOBAL_QUOTA_EXHAUSTED"),
        }
    }

    /// 返回人类可读的描述信息
    pub fn message(&self) -> String {
        match self {
            ValidationReason::Allowed => "Validation passed".to_string(),
            ValidationReason::RuleExpired { end_time } => {
                format!("Rule expired at {}", end_time.format("%Y-%m-%d %H:%M:%S UTC"))
            }
            ValidationReason::RuleNotStarted { start_time } => {
                format!(
                    "Rule not started yet, starts at {}",
                    start_time.format("%Y-%m-%d %H:%M:%S UTC")
                )
            }
            ValidationReason::UserLimitExceeded { current, max } => {
                format!(
                    "User limit exceeded: already granted {} times, max is {}",
                    current, max
                )
            }
            ValidationReason::GlobalQuotaExhausted { granted, quota } => {
                format!(
                    "Global quota exhausted: {} granted out of {} quota",
                    granted, quota
                )
            }
        }
    }

    /// 判断是否为允许状态
    pub fn is_allowed(&self) -> bool {
        matches!(self, ValidationReason::Allowed)
    }
}

/// 校验上下文
///
/// 记录校验时的状态快照，用于审计和调试。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationContext {
    pub checked_at: DateTime<Utc>,
    /// 用户当前已获得次数
    pub user_granted_count: Option<i32>,
    /// 全局当前已发放数量
    pub global_granted_count: Option<i32>,
}

/// 全局配额更新结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuotaUpdateResult {
    /// 更新成功
    Success {
        rule_id: i64,
        previous: i32,
        current: i32,
        remaining: i32,
    },
    /// 配额已耗尽
    QuotaExhausted {
        rule_id: i64,
        quota: i32,
        granted: i32,
        requested: i32,
    },
    /// 规则不存在
    RuleNotFound { rule_id: i64 },
}

impl QuotaUpdateResult {
    pub fn is_success(&self) -> bool {
        matches!(self, QuotaUpdateResult::Success { .. })
    }
}

/// 规则加载状态
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LoadStatus {
    pub loaded: bool,
    pub rule_count: usize,
    pub last_loaded_at: Option<DateTime<Utc>>,
    pub event_types: Vec<String>,
}

/// 规则刷新事件
///
/// 通过 Kafka 消息触发规则的重新加载，支持按服务组或事件类型筛选。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleReloadEvent {
    /// 目标服务组，None 表示所有服务
    pub service_group: Option<String>,
    /// 目标事件类型，None 表示所有事件类型
    pub event_type: Option<String>,
    /// 触发来源标识
    pub trigger_source: String,
    pub triggered_at: DateTime<Utc>,
}

/// 跳过的规则信息
///
/// 记录因校验失败而被跳过的规则，用于批量处理时的结果汇总。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkippedRule {
    pub rule_id: i64,
    pub rule_code: String,
    pub skip_reason: ValidationReason,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_validation_reason_deny_code() {
        assert_eq!(ValidationReason::Allowed.deny_code(), None);
        assert_eq!(
            ValidationReason::RuleExpired {
                end_time: Utc::now()
            }
            .deny_code(),
            Some("RULE_EXPIRED")
        );
        assert_eq!(
            ValidationReason::RuleNotStarted {
                start_time: Utc::now()
            }
            .deny_code(),
            Some("RULE_NOT_STARTED")
        );
        assert_eq!(
            ValidationReason::UserLimitExceeded {
                current: 5,
                max: 3
            }
            .deny_code(),
            Some("USER_LIMIT_EXCEEDED")
        );
        assert_eq!(
            ValidationReason::GlobalQuotaExhausted {
                granted: 100,
                quota: 100
            }
            .deny_code(),
            Some("GLOBAL_QUOTA_EXHAUSTED")
        );
    }

    #[test]
    fn test_validation_reason_message() {
        assert_eq!(ValidationReason::Allowed.message(), "Validation passed");

        let end_time = Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap();
        let expired = ValidationReason::RuleExpired { end_time };
        assert!(expired.message().contains("2024-12-31 23:59:59 UTC"));

        let start_time = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let not_started = ValidationReason::RuleNotStarted { start_time };
        assert!(not_started.message().contains("2025-01-01 00:00:00 UTC"));

        let user_limit = ValidationReason::UserLimitExceeded {
            current: 5,
            max: 3,
        };
        assert!(user_limit.message().contains("5"));
        assert!(user_limit.message().contains("3"));

        let quota_exhausted = ValidationReason::GlobalQuotaExhausted {
            granted: 100,
            quota: 100,
        };
        assert!(quota_exhausted.message().contains("100"));
    }

    #[test]
    fn test_validation_reason_is_allowed() {
        assert!(ValidationReason::Allowed.is_allowed());
        assert!(!ValidationReason::RuleExpired {
            end_time: Utc::now()
        }
        .is_allowed());
        assert!(!ValidationReason::RuleNotStarted {
            start_time: Utc::now()
        }
        .is_allowed());
        assert!(!ValidationReason::UserLimitExceeded {
            current: 5,
            max: 3
        }
        .is_allowed());
        assert!(!ValidationReason::GlobalQuotaExhausted {
            granted: 100,
            quota: 100
        }
        .is_allowed());
    }

    #[test]
    fn test_quota_update_result_is_success() {
        let success = QuotaUpdateResult::Success {
            rule_id: 1,
            previous: 10,
            current: 11,
            remaining: 89,
        };
        assert!(success.is_success());

        let exhausted = QuotaUpdateResult::QuotaExhausted {
            rule_id: 1,
            quota: 100,
            granted: 100,
            requested: 1,
        };
        assert!(!exhausted.is_success());

        let not_found = QuotaUpdateResult::RuleNotFound { rule_id: 999 };
        assert!(!not_found.is_success());
    }
}
