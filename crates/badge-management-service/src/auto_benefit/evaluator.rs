//! 自动权益评估器
//!
//! 核心组件，在用户获得徽章后评估是否应自动发放权益。
//!
//! ## 设计说明
//!
//! AutoBenefitEvaluator 是自动权益发放的核心，负责：
//! 1. 获取用户当前所有有效徽章
//! 2. 获取可能触发的自动权益规则
//! 3. 逐一评估每条规则的条件是否满足
//! 4. 对满足条件的规则执行权益发放
//!
//! ## 循环依赖处理
//!
//! BenefitService 依赖此评估器进行自动权益评估，而评估器需要调用
//! BenefitService 执行实际发放。通过延迟注入 (`set_benefit_service`) 打破循环。

use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;
use tracing::{error, info, instrument, warn};

use super::dto::{
    AutoBenefitConfig, AutoBenefitContext, AutoBenefitEvaluationLog, AutoBenefitGrant,
    AutoBenefitResult, AutoBenefitStatus, NewAutoBenefitGrant,
};
use super::rule_cache::{AutoBenefitRuleCache, CachedRule};
use crate::benefit::BenefitService;
use crate::error::Result;
use crate::repository::{AutoBenefitRepository, UserBadgeRepository};

/// 自动权益评估器
///
/// 徽章发放后调用，评估用户是否满足自动权益规则，满足则触发发放。
/// 使用 RwLock 延迟注入 BenefitService 以解决与 BenefitService 的循环依赖。
pub struct AutoBenefitEvaluator {
    config: AutoBenefitConfig,
    rule_cache: Arc<AutoBenefitRuleCache>,
    auto_benefit_repo: Arc<AutoBenefitRepository>,
    user_badge_repo: Arc<UserBadgeRepository>,
    /// 延迟注入的 BenefitService
    benefit_service: RwLock<Option<Arc<BenefitService>>>,
}

impl AutoBenefitEvaluator {
    /// 创建新的评估器
    pub fn new(
        config: AutoBenefitConfig,
        rule_cache: Arc<AutoBenefitRuleCache>,
        auto_benefit_repo: Arc<AutoBenefitRepository>,
        user_badge_repo: Arc<UserBadgeRepository>,
    ) -> Self {
        Self {
            config,
            rule_cache,
            auto_benefit_repo,
            user_badge_repo,
            benefit_service: RwLock::new(None),
        }
    }

    /// 延迟注入 BenefitService
    ///
    /// 服务启动后调用，打破与 BenefitService 的循环依赖
    pub async fn set_benefit_service(&self, service: Arc<BenefitService>) {
        let mut guard = self.benefit_service.write().await;
        *guard = Some(service);
        info!("自动权益评估器: BenefitService 已注入");
    }

    /// 检查 BenefitService 是否已注入
    pub async fn has_benefit_service(&self) -> bool {
        self.benefit_service.read().await.is_some()
    }

    /// 获取配置引用
    pub fn config(&self) -> &AutoBenefitConfig {
        &self.config
    }

    /// 主入口：徽章发放后调用
    ///
    /// 评估所有以 `trigger_badge_id` 为触发条件的自动权益规则，
    /// 对满足条件的规则执行权益发放。
    ///
    /// # Arguments
    /// * `context` - 评估上下文，包含用户 ID、触发徽章等信息
    ///
    /// # Returns
    /// 返回评估结果，包含成功发放和跳过的规则信息
    #[instrument(skip(self, context), fields(user_id = %context.user_id, trigger_badge_id = context.trigger_badge_id))]
    pub async fn evaluate(&self, mut context: AutoBenefitContext) -> Result<AutoBenefitResult> {
        // 功能开关检查
        if !self.config.enabled {
            info!("自动权益发放功能已禁用");
            return Ok(AutoBenefitResult::default());
        }

        // 1. 获取用户当前所有有效徽章
        let user_badges = self
            .user_badge_repo
            .list_active_badge_ids(&context.user_id)
            .await?;
        context.user_badges = user_badges;

        // 2. 获取可能触发的规则
        let rules = self
            .rule_cache
            .get_rules_by_trigger(context.trigger_badge_id)
            .await;

        if rules.is_empty() {
            info!(
                "徽章 {} 没有关联的自动权益规则",
                context.trigger_badge_id
            );
            return Ok(AutoBenefitResult::default());
        }

        info!(
            "开始评估自动权益: 候选规则数={}",
            rules.len()
        );

        // 3. 评估每条规则
        let mut result = AutoBenefitResult::default();
        for rule in rules.iter().take(self.config.max_rules_per_evaluation) {
            // 超时检查
            if context.is_timeout(self.config.evaluation_timeout_ms) {
                warn!(
                    "评估超时，已处理 {} 条规则，跳过剩余规则",
                    result.rules_evaluated
                );
                break;
            }

            match self.evaluate_rule(&context, rule).await {
                Ok(Some(grant)) => {
                    result.grants_created.push(grant);
                    result.rules_matched += 1;
                }
                Ok(None) => {
                    // 条件不满足，skip_reason 已在 evaluate_rule 中处理
                }
                Err(e) => {
                    warn!("规则 {} 评估失败: {}", rule.rule_id, e);
                }
            }
            result.rules_evaluated += 1;
        }

        // 4. 记录评估日志
        self.log_evaluation(&context, &result).await;

        info!(
            "评估完成: 规则数={}, 匹配数={}, 发放数={}",
            result.rules_evaluated,
            result.rules_matched,
            result.grants_created.len()
        );

        Ok(result)
    }

    /// 评估单条规则
    ///
    /// 按顺序检查：时间窗口 -> 徽章条件 -> 幂等 -> 频率限制，
    /// 全部通过后执行发放。
    async fn evaluate_rule(
        &self,
        context: &AutoBenefitContext,
        rule: &CachedRule,
    ) -> Result<Option<AutoBenefitGrant>> {
        let now = Utc::now();

        // 1. 时间窗口检查
        if !rule.is_within_time_window(now) {
            return Ok(None); // TimeWindowClosed
        }

        // 2. 徽章条件检查
        if !self.check_badge_requirements(context, rule) {
            return Ok(None); // BadgeRequirementNotMet
        }

        // 3. 幂等检查
        let idempotency_key = NewAutoBenefitGrant::generate_idempotency_key(
            &context.user_id,
            rule.rule_id,
            context.trigger_user_badge_id,
        );
        if self
            .auto_benefit_repo
            .exists_by_idempotency_key(&idempotency_key)
            .await?
        {
            return Ok(None); // AlreadyGranted
        }

        // 4. 频率限制检查
        if !self.check_frequency_limit(context, rule).await? {
            return Ok(None); // FrequencyLimitReached
        }

        // 5. 创建发放记录并执行
        self.execute_grant(context, rule, &idempotency_key).await
    }

    /// 检查徽章条件是否满足
    ///
    /// 规则要求的所有徽章，用户必须全部持有
    fn check_badge_requirements(&self, context: &AutoBenefitContext, rule: &CachedRule) -> bool {
        let required_ids = rule.get_required_badge_ids();

        // 所有必需的徽章都要在用户持有列表中
        required_ids
            .iter()
            .all(|id| context.user_badges.contains(id))
    }

    /// 检查频率限制
    ///
    /// 支持每用户总数限制和每日限制两种模式
    async fn check_frequency_limit(
        &self,
        context: &AutoBenefitContext,
        rule: &CachedRule,
    ) -> Result<bool> {
        let Some(ref freq) = rule.frequency_config else {
            return Ok(true); // 无频率限制配置
        };

        // 检查每用户总数限制
        if let Some(max) = freq.max_per_user {
            let count = self
                .auto_benefit_repo
                .count_user_grants(&context.user_id, rule.rule_id, None)
                .await?;
            if count >= max as i64 {
                return Ok(false);
            }
        }

        // 检查每日限制
        if let Some(max) = freq.max_per_day {
            let since = Utc::now() - chrono::Duration::days(1);
            let count = self
                .auto_benefit_repo
                .count_user_grants(&context.user_id, rule.rule_id, Some(since))
                .await?;
            if count >= max as i64 {
                return Ok(false);
            }
        }

        // 检查每周限制
        if let Some(max) = freq.max_per_week {
            let since = Utc::now() - chrono::Duration::weeks(1);
            let count = self
                .auto_benefit_repo
                .count_user_grants(&context.user_id, rule.rule_id, Some(since))
                .await?;
            if count >= max as i64 {
                return Ok(false);
            }
        }

        // 检查每月限制
        if let Some(max) = freq.max_per_month {
            let since = Utc::now() - chrono::Duration::days(30);
            let count = self
                .auto_benefit_repo
                .count_user_grants(&context.user_id, rule.rule_id, Some(since))
                .await?;
            if count >= max as i64 {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// 执行权益发放
    ///
    /// 创建发放记录，调用 BenefitService 执行实际发放，更新记录状态
    async fn execute_grant(
        &self,
        context: &AutoBenefitContext,
        rule: &CachedRule,
        idempotency_key: &str,
    ) -> Result<Option<AutoBenefitGrant>> {
        // 创建发放记录
        let new_grant = NewAutoBenefitGrant::new(
            context.user_id.clone(),
            rule.rule_id,
            context.trigger_badge_id,
            context.trigger_user_badge_id,
        );

        let grant = match self.auto_benefit_repo.create_grant(&new_grant).await? {
            Some(g) => g,
            None => {
                // 幂等保护：记录已存在
                return Ok(None);
            }
        };

        // 更新状态为处理中
        self.auto_benefit_repo
            .update_status(grant.id, AutoBenefitStatus::Processing, None, None)
            .await?;

        // 调用 BenefitService 执行实际发放
        let benefit_service = self.benefit_service.read().await;
        if let Some(ref service) = *benefit_service {
            // 调用权益发放
            match service
                .grant_benefit_for_auto_rule(
                    &context.user_id,
                    rule.rule_id,
                    rule.benefit_id,
                    idempotency_key,
                )
                .await
            {
                Ok(benefit_grant_id) => {
                    self.auto_benefit_repo
                        .update_status(
                            grant.id,
                            AutoBenefitStatus::Success,
                            Some(benefit_grant_id),
                            None,
                        )
                        .await?;

                    info!(
                        "自动权益发放成功: rule_id={}, benefit_grant_id={}",
                        rule.rule_id, benefit_grant_id
                    );

                    Ok(Some(AutoBenefitGrant {
                        id: grant.id,
                        rule_id: rule.rule_id,
                        benefit_grant_id: Some(benefit_grant_id),
                        status: AutoBenefitStatus::Success,
                    }))
                }
                Err(e) => {
                    self.auto_benefit_repo
                        .update_status(
                            grant.id,
                            AutoBenefitStatus::Failed,
                            None,
                            Some(&e.to_string()),
                        )
                        .await?;

                    warn!("自动权益发放失败: rule_id={}, error={}", rule.rule_id, e);
                    Err(e)
                }
            }
        } else {
            // BenefitService 未注入
            self.auto_benefit_repo
                .update_status(
                    grant.id,
                    AutoBenefitStatus::Skipped,
                    None,
                    Some("BenefitService not available"),
                )
                .await?;

            warn!("BenefitService 未注入，跳过自动权益发放");
            Ok(None)
        }
    }

    /// 记录评估日志
    async fn log_evaluation(&self, context: &AutoBenefitContext, result: &AutoBenefitResult) {
        let log = AutoBenefitEvaluationLog::from_result(
            context.user_id.clone(),
            context.trigger_badge_id,
            result,
            context.elapsed_ms() as i64,
        );

        if let Err(e) = self.auto_benefit_repo.log_evaluation(&log).await {
            error!("记录评估日志失败: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auto_benefit::{AutoBenefitConfig, SkipReason};

    #[test]
    fn test_auto_benefit_config_default() {
        let config = AutoBenefitConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_rules_per_evaluation, 100);
        assert_eq!(config.evaluation_timeout_ms, 5000);
    }

    #[test]
    fn test_auto_benefit_context_timeout() {
        let context = AutoBenefitContext::new("user-1".to_string(), 1, 1, vec![1, 2, 3]);

        // 刚创建时不应超时
        assert!(!context.is_timeout(5000));
        // 超时时间为 0 应该超时
        assert!(context.is_timeout(0));
    }

    #[test]
    fn test_auto_benefit_result_operations() {
        let mut result = AutoBenefitResult::default();
        assert_eq!(result.rules_evaluated, 0);
        assert_eq!(result.rules_matched, 0);
        assert!(result.grants_created.is_empty());

        result.rules_evaluated = 5;
        result.rules_matched = 2;
        result.add_skipped(100, SkipReason::FrequencyLimitReached);

        assert_eq!(result.rules_evaluated, 5);
        assert_eq!(result.rules_matched, 2);
        assert_eq!(result.skipped_rules.len(), 1);
    }

    #[test]
    fn test_skip_reason_display() {
        assert_eq!(
            SkipReason::BadgeRequirementNotMet.to_string(),
            "BADGE_REQUIREMENT_NOT_MET"
        );
        assert_eq!(
            SkipReason::FrequencyLimitReached.to_string(),
            "FREQUENCY_LIMIT_REACHED"
        );
        assert_eq!(SkipReason::AlreadyGranted.to_string(), "ALREADY_GRANTED");
    }
}
