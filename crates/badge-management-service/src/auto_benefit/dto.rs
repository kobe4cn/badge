//! 自动权益发放相关的数据传输对象
//!
//! 定义自动权益发放功能所需的配置、上下文和结果结构。

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// 自动权益发放配置
#[derive(Debug, Clone, Deserialize)]
pub struct AutoBenefitConfig {
    /// 是否启用自动权益发放
    pub enabled: bool,
    /// 单次评估最大规则数量
    pub max_rules_per_evaluation: usize,
    /// 评估超时时间（毫秒）
    pub evaluation_timeout_ms: u64,
    /// 是否异步执行（不阻塞徽章发放主流程）
    pub async_execution: bool,
}

impl Default for AutoBenefitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_rules_per_evaluation: 100,
            evaluation_timeout_ms: 5000,
            async_execution: true,
        }
    }
}

/// 自动权益评估上下文
///
/// 包含评估自动权益规则所需的所有信息
pub struct AutoBenefitContext {
    /// 用户 ID
    pub user_id: String,
    /// 触发评估的徽章 ID
    pub trigger_badge_id: i64,
    /// 触发评估的用户徽章记录 ID
    pub trigger_user_badge_id: i64,
    /// 用户当前持有的所有有效徽章 ID
    pub user_badges: Vec<i64>,
    /// 评估开始时间（用于超时检测）
    pub start_time: Instant,
}

impl AutoBenefitContext {
    /// 创建新的评估上下文
    pub fn new(
        user_id: String,
        trigger_badge_id: i64,
        trigger_user_badge_id: i64,
        user_badges: Vec<i64>,
    ) -> Self {
        Self {
            user_id,
            trigger_badge_id,
            trigger_user_badge_id,
            user_badges,
            start_time: Instant::now(),
        }
    }

    /// 获取已用时间（毫秒）
    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    /// 检查是否超时
    pub fn is_timeout(&self, timeout_ms: u64) -> bool {
        self.elapsed_ms() >= timeout_ms
    }
}

/// 自动权益评估结果
#[derive(Debug, Default)]
pub struct AutoBenefitResult {
    /// 评估的规则数量
    pub rules_evaluated: usize,
    /// 匹配成功的规则数量
    pub rules_matched: usize,
    /// 成功创建的权益发放记录
    pub grants_created: Vec<AutoBenefitGrant>,
    /// 被跳过的规则列表
    pub skipped_rules: Vec<SkippedRule>,
}

impl AutoBenefitResult {
    /// 创建空结果
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加成功发放的记录
    pub fn add_grant(&mut self, grant: AutoBenefitGrant) {
        self.rules_matched += 1;
        self.grants_created.push(grant);
    }

    /// 添加被跳过的规则
    pub fn add_skipped(&mut self, rule_id: i64, reason: SkipReason) {
        self.skipped_rules.push(SkippedRule { rule_id, reason });
    }
}

/// 自动权益发放记录
#[derive(Debug, Clone, Serialize)]
pub struct AutoBenefitGrant {
    /// 自动发放记录 ID
    pub id: i64,
    /// 关联的规则 ID
    pub rule_id: i64,
    /// 关联的权益发放记录 ID（可能尚未生成）
    pub benefit_grant_id: Option<i64>,
    /// 发放状态
    pub status: AutoBenefitStatus,
}

/// 跳过的规则记录
#[derive(Debug, Clone, Serialize)]
pub struct SkippedRule {
    /// 规则 ID
    pub rule_id: i64,
    /// 跳过原因
    pub reason: SkipReason,
}

/// 跳过原因枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SkipReason {
    /// 徽章条件不满足
    BadgeRequirementNotMet,
    /// 频率限制已达上限
    FrequencyLimitReached,
    /// 库存不足
    StockExhausted,
    /// 时间窗口已关闭
    TimeWindowClosed,
    /// 已经发放过（幂等保护）
    AlreadyGranted,
    /// 规则未发布
    RuleNotPublished,
}

impl std::fmt::Display for SkipReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadgeRequirementNotMet => write!(f, "BADGE_REQUIREMENT_NOT_MET"),
            Self::FrequencyLimitReached => write!(f, "FREQUENCY_LIMIT_REACHED"),
            Self::StockExhausted => write!(f, "STOCK_EXHAUSTED"),
            Self::TimeWindowClosed => write!(f, "TIME_WINDOW_CLOSED"),
            Self::AlreadyGranted => write!(f, "ALREADY_GRANTED"),
            Self::RuleNotPublished => write!(f, "RULE_NOT_PUBLISHED"),
        }
    }
}

/// 自动发放状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AutoBenefitStatus {
    /// 待处理
    #[default]
    Pending,
    /// 处理中
    Processing,
    /// 发放成功
    Success,
    /// 发放失败
    Failed,
    /// 已跳过
    Skipped,
}

impl std::fmt::Display for AutoBenefitStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "PENDING"),
            Self::Processing => write!(f, "PROCESSING"),
            Self::Success => write!(f, "SUCCESS"),
            Self::Failed => write!(f, "FAILED"),
            Self::Skipped => write!(f, "SKIPPED"),
        }
    }
}


/// 新建自动发放记录请求
pub struct NewAutoBenefitGrant {
    /// 用户 ID
    pub user_id: String,
    /// 匹配的规则 ID
    pub rule_id: i64,
    /// 触发的徽章 ID
    pub trigger_badge_id: i64,
    /// 触发的用户徽章记录 ID
    pub trigger_user_badge_id: i64,
    /// 幂等键（防止重复发放）
    pub idempotency_key: String,
}

impl NewAutoBenefitGrant {
    /// 生成幂等键
    ///
    /// 基于用户ID、规则ID和用户徽章ID生成，确保同一徽章获得事件不会重复触发同一规则
    pub fn generate_idempotency_key(
        user_id: &str,
        rule_id: i64,
        trigger_user_badge_id: i64,
    ) -> String {
        format!(
            "auto_benefit:{}:{}:{}",
            user_id, rule_id, trigger_user_badge_id
        )
    }

    /// 创建新的发放请求
    pub fn new(
        user_id: String,
        rule_id: i64,
        trigger_badge_id: i64,
        trigger_user_badge_id: i64,
    ) -> Self {
        let idempotency_key =
            Self::generate_idempotency_key(&user_id, rule_id, trigger_user_badge_id);
        Self {
            user_id,
            rule_id,
            trigger_badge_id,
            trigger_user_badge_id,
            idempotency_key,
        }
    }
}

/// 评估日志记录
///
/// 用于记录每次自动权益评估的执行情况，便于问题排查和性能分析
#[derive(Debug, Serialize)]
pub struct AutoBenefitEvaluationLog {
    /// 用户 ID
    pub user_id: String,
    /// 触发的徽章 ID
    pub trigger_badge_id: i64,
    /// 评估上下文信息（JSON 格式）
    pub evaluation_context: serde_json::Value,
    /// 评估的规则数量
    pub rules_evaluated: i32,
    /// 匹配的规则数量
    pub rules_matched: i32,
    /// 创建的发放记录数量
    pub grants_created: i32,
    /// 评估耗时（毫秒）
    pub duration_ms: i64,
}

impl AutoBenefitEvaluationLog {
    /// 从评估结果创建日志记录
    pub fn from_result(
        user_id: String,
        trigger_badge_id: i64,
        result: &AutoBenefitResult,
        duration_ms: i64,
    ) -> Self {
        Self {
            user_id,
            trigger_badge_id,
            evaluation_context: serde_json::json!({}),
            rules_evaluated: result.rules_evaluated as i32,
            rules_matched: result.rules_matched as i32,
            grants_created: result.grants_created.len() as i32,
            duration_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_benefit_config_default() {
        let config = AutoBenefitConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_rules_per_evaluation, 100);
        assert_eq!(config.evaluation_timeout_ms, 5000);
        assert!(config.async_execution);
    }

    #[test]
    fn test_auto_benefit_context_timeout() {
        let ctx = AutoBenefitContext::new("user-1".to_string(), 1, 1, vec![1, 2, 3]);

        // 刚创建时不应超时
        assert!(!ctx.is_timeout(5000));
    }

    #[test]
    fn test_auto_benefit_result_operations() {
        let mut result = AutoBenefitResult::new();
        assert_eq!(result.rules_matched, 0);
        assert!(result.grants_created.is_empty());

        result.add_grant(AutoBenefitGrant {
            id: 1,
            rule_id: 100,
            benefit_grant_id: Some(200),
            status: AutoBenefitStatus::Success,
        });

        assert_eq!(result.rules_matched, 1);
        assert_eq!(result.grants_created.len(), 1);

        result.add_skipped(101, SkipReason::FrequencyLimitReached);
        assert_eq!(result.skipped_rules.len(), 1);
        assert_eq!(result.skipped_rules[0].rule_id, 101);
    }

    #[test]
    fn test_idempotency_key_generation() {
        let key = NewAutoBenefitGrant::generate_idempotency_key("user-123", 100, 500);
        assert_eq!(key, "auto_benefit:user-123:100:500");
    }

    #[test]
    fn test_auto_benefit_status_display() {
        assert_eq!(AutoBenefitStatus::Pending.to_string(), "PENDING");
        assert_eq!(AutoBenefitStatus::Processing.to_string(), "PROCESSING");
        assert_eq!(AutoBenefitStatus::Success.to_string(), "SUCCESS");
        assert_eq!(AutoBenefitStatus::Failed.to_string(), "FAILED");
        assert_eq!(AutoBenefitStatus::Skipped.to_string(), "SKIPPED");
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
        assert_eq!(SkipReason::StockExhausted.to_string(), "STOCK_EXHAUSTED");
        assert_eq!(SkipReason::TimeWindowClosed.to_string(), "TIME_WINDOW_CLOSED");
        assert_eq!(SkipReason::AlreadyGranted.to_string(), "ALREADY_GRANTED");
        assert_eq!(SkipReason::RuleNotPublished.to_string(), "RULE_NOT_PUBLISHED");
    }
}
