//! 规则校验器
//!
//! 在发放徽章前校验规则的各项限制条件。

use std::time::Instant;

use chrono::Utc;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::cache::Cache;
use crate::error::BadgeError;

use super::models::{BadgeGrant, ValidationContext, ValidationReason, ValidationResult};

/// 规则校验器
///
/// 在发放徽章前进行综合校验，确保满足时间窗口、用户限额、全局配额等条件。
/// Cache 参数保留用于后续优化，可缓存用户发放次数以减少数据库查询。
pub struct RuleValidator {
    #[allow(dead_code)]
    cache: Cache,
    db_pool: PgPool,
}

impl RuleValidator {
    pub fn new(cache: Cache, db_pool: PgPool) -> Self {
        Self { cache, db_pool }
    }

    /// 综合校验规则是否允许发放
    ///
    /// 校验顺序：
    /// 1. 时间有效性（start_time, end_time）
    /// 2. 用户发放次数限制（max_count_per_user）
    /// 3. 全局配额限制（global_quota）
    pub async fn can_grant(
        &self,
        rule: &BadgeGrant,
        user_id: &str,
    ) -> Result<ValidationResult, BadgeError> {
        let start = Instant::now();
        let now = Utc::now();
        let mut context = ValidationContext {
            checked_at: now,
            ..Default::default()
        };

        // 先检查过期，过期的规则无论如何都不应该发放
        if let Some(end_time) = rule.end_time {
            if now > end_time {
                let result =
                    self.build_result(rule, user_id, ValidationReason::RuleExpired { end_time }, context);
                self.log_validation(&result, start.elapsed().as_millis() as u64);
                return Ok(result);
            }
        }

        // 检查是否在生效时间之前
        if let Some(start_time) = rule.start_time {
            if now < start_time {
                let result = self.build_result(
                    rule,
                    user_id,
                    ValidationReason::RuleNotStarted { start_time },
                    context,
                );
                self.log_validation(&result, start.elapsed().as_millis() as u64);
                return Ok(result);
            }
        }

        // 检查用户发放次数限制
        if let Some(max_count) = rule.max_count_per_user {
            let user_count = self.get_user_grant_count(user_id, rule.badge_id).await?;
            context.user_granted_count = Some(user_count);

            if user_count >= max_count {
                let result = self.build_result(
                    rule,
                    user_id,
                    ValidationReason::UserLimitExceeded {
                        current: user_count,
                        max: max_count,
                    },
                    context,
                );
                self.log_validation(&result, start.elapsed().as_millis() as u64);
                return Ok(result);
            }
        }

        // 检查全局配额
        if let Some(quota) = rule.global_quota {
            context.global_granted_count = Some(rule.global_granted);

            if rule.global_granted >= quota {
                let result = self.build_result(
                    rule,
                    user_id,
                    ValidationReason::GlobalQuotaExhausted {
                        granted: rule.global_granted,
                        quota,
                    },
                    context,
                );
                self.log_validation(&result, start.elapsed().as_millis() as u64);
                return Ok(result);
            }
        }

        // 所有校验通过
        let result = self.build_result(rule, user_id, ValidationReason::Allowed, context);
        self.log_validation(&result, start.elapsed().as_millis() as u64);
        Ok(result)
    }

    /// 查询用户对某徽章的已发放次数
    ///
    /// 从 user_badge_logs 表统计 action='grant' 的记录数
    async fn get_user_grant_count(&self, user_id: &str, badge_id: i64) -> Result<i32, BadgeError> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM user_badge_logs
            WHERE user_id = $1 AND badge_id = $2 AND action = 'grant'
            "#,
        )
        .bind(user_id)
        .bind(badge_id)
        .fetch_one(&self.db_pool)
        .await?;

        Ok(count.0 as i32)
    }

    fn build_result(
        &self,
        rule: &BadgeGrant,
        user_id: &str,
        reason: ValidationReason,
        context: ValidationContext,
    ) -> ValidationResult {
        ValidationResult {
            allowed: reason.is_allowed(),
            rule_id: rule.rule_id,
            rule_code: rule.rule_code.clone(),
            user_id: user_id.to_string(),
            reason,
            context,
        }
    }

    fn log_validation(&self, result: &ValidationResult, elapsed_ms: u64) {
        if result.allowed {
            info!(
                rule_id = result.rule_id,
                rule_code = %result.rule_code,
                user_id = %result.user_id,
                validation_ms = elapsed_ms,
                "规则校验通过"
            );
        } else {
            warn!(
                rule_id = result.rule_id,
                rule_code = %result.rule_code,
                user_id = %result.user_id,
                deny_code = result.reason.deny_code(),
                deny_message = %result.reason.message(),
                user_granted_count = ?result.context.user_granted_count,
                global_granted_count = ?result.context.global_granted_count,
                validation_ms = elapsed_ms,
                "规则校验未通过"
            );
        }
    }
}
