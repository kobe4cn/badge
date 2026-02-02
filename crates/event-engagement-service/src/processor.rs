//! 行为事件处理器
//!
//! 实现 `EventProcessor` trait，负责行为类事件的完整处理流程：
//! 幂等校验 -> 规则校验 -> 规则引擎评估 -> 徽章发放 -> 结果汇总。

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use badge_shared::cache::Cache;
use badge_shared::error::BadgeError;
use badge_shared::events::{
    EventPayload, EventProcessor, EventResult, EventType, GrantedBadge, MatchedRule,
};
use badge_shared::rules::{BadgeGrant, RuleBadgeMapping, RuleValidator, SkippedRule};
use tracing::{debug, info, warn};

use crate::rule_client::BadgeRuleService;

/// 本地评估 rule_json 条件
///
/// 当规则引擎没有加载规则时，使用本地 rule_json 进行简化评估。
/// 支持简单条件（eq, neq, gt, gte, lt, lte）和组合条件（AND, OR）。
fn evaluate_rule_json(data: &serde_json::Value, rule_json: Option<&serde_json::Value>) -> bool {
    let rule = match rule_json {
        Some(r) => r,
        // 无 rule_json 时默认匹配（兼容仅事件类型匹配的规则）
        None => return true,
    };

    evaluate_node(data, rule)
}

/// 评估单个规则节点
fn evaluate_node(data: &serde_json::Value, node: &serde_json::Value) -> bool {
    let node_type = node.get("type").and_then(|t| t.as_str()).unwrap_or("");

    match node_type {
        "condition" => evaluate_condition(data, node),
        "group" => evaluate_group(data, node),
        _ => {
            // 未知类型，默认不匹配
            warn!(node_type, "未知的规则节点类型");
            false
        }
    }
}

/// 评估条件节点
fn evaluate_condition(data: &serde_json::Value, condition: &serde_json::Value) -> bool {
    let field = match condition.get("field").and_then(|f| f.as_str()) {
        Some(f) => f,
        None => return false,
    };

    let operator = condition
        .get("operator")
        .and_then(|o| o.as_str())
        .unwrap_or("");

    let expected = match condition.get("value") {
        Some(v) => v,
        None => return false,
    };

    // 从数据中获取字段值（支持点号分隔的路径）
    let actual = get_field_value(data, field);

    match operator.to_lowercase().as_str() {
        "eq" => values_equal(actual, expected),
        "neq" => !values_equal(actual, expected),
        "gt" => compare_numbers(actual, expected, |a, b| a > b),
        "gte" => compare_numbers(actual, expected, |a, b| a >= b),
        "lt" => compare_numbers(actual, expected, |a, b| a < b),
        "lte" => compare_numbers(actual, expected, |a, b| a <= b),
        _ => {
            // 不支持的操作符，默认不匹配
            debug!(operator, "不支持的操作符");
            false
        }
    }
}

/// 评估组节点
fn evaluate_group(data: &serde_json::Value, group: &serde_json::Value) -> bool {
    let operator = group
        .get("operator")
        .and_then(|o| o.as_str())
        .unwrap_or("AND");

    let children = match group.get("children").and_then(|c| c.as_array()) {
        Some(c) => c,
        None => return false,
    };

    match operator.to_uppercase().as_str() {
        "AND" => children.iter().all(|child| evaluate_node(data, child)),
        "OR" => children.iter().any(|child| evaluate_node(data, child)),
        _ => false,
    }
}

/// 从数据中获取字段值（支持点号分隔的路径）
fn get_field_value<'a>(data: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = data;

    for part in parts {
        match current {
            serde_json::Value::Object(map) => {
                current = map.get(part)?;
            }
            serde_json::Value::Array(arr) => {
                let index: usize = part.parse().ok()?;
                current = arr.get(index)?;
            }
            _ => return None,
        }
    }

    Some(current)
}

/// 比较两个值是否相等
fn values_equal(actual: Option<&serde_json::Value>, expected: &serde_json::Value) -> bool {
    match actual {
        Some(a) => a == expected,
        None => false,
    }
}

/// 比较两个数值
fn compare_numbers(
    actual: Option<&serde_json::Value>,
    expected: &serde_json::Value,
    cmp: fn(f64, f64) -> bool,
) -> bool {
    let actual_num = actual.and_then(|v| v.as_f64());
    let expected_num = expected.as_f64();

    match (actual_num, expected_num) {
        (Some(a), Some(b)) => cmp(a, b),
        _ => false,
    }
}

/// 幂等键前缀，标记事件是否已处理
const PROCESSED_KEY_PREFIX: &str = "event:processed:";
/// 幂等记录保留 24 小时，超过此窗口的重复消费不再拦截，
/// 因为 Kafka 消费偏移量提交周期远短于此窗口
const PROCESSED_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// 行为事件处理器
///
/// 组合四个依赖完成事件处理：
/// - `cache`: Redis 幂等校验
/// - `rule_client`: gRPC 调用（规则引擎 + 徽章管理）
/// - `rule_mapping`: 规则到徽章的映射配置（从数据库动态加载）
/// - `rule_validator`: 规则校验器（时间窗口、配额等校验）
///
/// 使用 trait object 而非泛型参数，因为处理器会被存储到 Consumer 中，
/// trait object 避免了泛型传播到整个调用链。
pub struct EngagementEventProcessor {
    cache: Cache,
    rule_client: Arc<dyn BadgeRuleService>,
    rule_mapping: Arc<RuleBadgeMapping>,
    rule_validator: Arc<RuleValidator>,
}

impl EngagementEventProcessor {
    pub fn new(
        cache: Cache,
        rule_client: Arc<dyn BadgeRuleService>,
        rule_mapping: Arc<RuleBadgeMapping>,
        rule_validator: Arc<RuleValidator>,
    ) -> Self {
        Self {
            cache,
            rule_client,
            rule_mapping,
            rule_validator,
        }
    }

    /// 构造 Redis 幂等键
    fn processed_key(event_id: &str) -> String {
        format!("{PROCESSED_KEY_PREFIX}{event_id}")
    }
}

#[async_trait]
impl EventProcessor for EngagementEventProcessor {
    /// 处理行为事件的完整流程
    ///
    /// 1. 根据事件类型获取适用的规则
    /// 2. 对每条规则进行校验（时间窗口、用户限额、全局配额）
    /// 3. 校验通过的规则交给规则引擎评估
    /// 4. 对匹配的规则发放徽章
    /// 5. 收集所有结果（含部分失败），不因单条规则失败中断整体流程
    async fn process(&self, event: &EventPayload) -> Result<EventResult, BadgeError> {
        let start = std::time::Instant::now();

        info!(
            event_id = %event.event_id,
            event_type = %event.event_type,
            user_id = %event.user_id,
            "开始处理行为事件"
        );

        // 1. 根据事件类型获取所有适用的规则
        // 使用 to_db_key() 获取数据库中的事件类型键名（小写下划线格式）
        let rules = self
            .rule_mapping
            .get_rules_by_event_type(event.event_type.to_db_key());
        if rules.is_empty() {
            debug!(
                event_id = %event.event_id,
                event_type = %event.event_type,
                "无适用规则，跳过评估"
            );
            return Ok(EventResult {
                event_id: event.event_id.clone(),
                processed: true,
                matched_rules: vec![],
                granted_badges: vec![],
                processing_time_ms: start.elapsed().as_millis() as i64,
                errors: vec![],
            });
        }

        info!(
            event_id = %event.event_id,
            event_type = %event.event_type,
            rule_count = rules.len(),
            "找到适用规则"
        );

        // 2. 对每条规则进行校验
        let mut valid_rules: Vec<BadgeGrant> = Vec::new();
        let mut skipped_rules: Vec<SkippedRule> = Vec::new();

        for rule in rules {
            match self.rule_validator.can_grant(&rule, &event.user_id).await {
                Ok(result) if result.allowed => {
                    valid_rules.push(rule);
                }
                Ok(result) => {
                    skipped_rules.push(SkippedRule {
                        rule_id: rule.rule_id,
                        rule_code: rule.rule_code.clone(),
                        skip_reason: result.reason,
                    });
                }
                Err(e) => {
                    warn!(
                        rule_id = rule.rule_id,
                        rule_code = %rule.rule_code,
                        error = %e,
                        "规则校验出错，跳过该规则"
                    );
                }
            }
        }

        if !skipped_rules.is_empty() {
            info!(
                event_id = %event.event_id,
                skipped_count = skipped_rules.len(),
                "部分规则因校验未通过被跳过"
            );
        }

        if valid_rules.is_empty() {
            debug!(
                event_id = %event.event_id,
                "所有规则校验未通过，跳过评估"
            );
            return Ok(EventResult {
                event_id: event.event_id.clone(),
                processed: true,
                matched_rules: vec![],
                granted_badges: vec![],
                processing_time_ms: start.elapsed().as_millis() as i64,
                errors: vec![],
            });
        }

        // 3. 将事件转为规则引擎评估上下文
        let context = event.to_evaluation_context();

        // 收集有效规则的 ID 用于批量评估
        let rule_ids: Vec<String> = valid_rules.iter().map(|r| r.rule_id.to_string()).collect();

        // 4. 调用规则引擎批量评估
        let matches = self
            .rule_client
            .evaluate_rules(&rule_ids, context)
            .await
            .map_err(|e| BadgeError::Internal(format!("规则评估失败: {e}")))?;

        let mut matched_rules = Vec::new();
        let mut granted_badges = Vec::new();
        let mut errors = Vec::new();

        // 5. 对每条规则发放徽章
        // 当规则引擎返回空（规则未加载到引擎）时，使用本地 rule_json 进行条件评估
        let rules_to_grant: Vec<&BadgeGrant> = if matches.is_empty() {
            // 规则引擎无匹配，使用本地 rule_json 进行条件评估（简化模式）
            info!(
                event_id = %event.event_id,
                valid_rules_count = valid_rules.len(),
                "规则引擎未返回匹配，使用本地 rule_json 评估"
            );
            valid_rules
                .iter()
                .filter(|r| evaluate_rule_json(&event.data, r.rule_json.as_ref()))
                .collect()
        } else {
            // 使用规则引擎匹配结果
            matches
                .iter()
                .filter_map(|rule_match| {
                    valid_rules
                        .iter()
                        .find(|r| r.rule_id.to_string() == rule_match.rule_id)
                })
                .collect()
        };

        for badge_grant in rules_to_grant {
            // 记录匹配的规则
            matched_rules.push(MatchedRule {
                rule_id: badge_grant.rule_id.to_string(),
                rule_name: badge_grant.rule_code.clone(),
                badge_id: badge_grant.badge_id,
                badge_name: badge_grant.badge_name.clone(),
                quantity: badge_grant.quantity,
            });

            // 6. 调用徽章发放，失败只记录不中断
            match self
                .rule_client
                .grant_badge(
                    &event.user_id,
                    &badge_grant.badge_id.to_string(),
                    badge_grant.quantity,
                    &event.event_id,
                )
                .await
            {
                Ok(grant_result) if grant_result.success => {
                    // user_badge_id 从 gRPC 返回的是 String，转为 i64
                    let user_badge_id = grant_result.user_badge_id.parse::<i64>().unwrap_or(0);

                    granted_badges.push(GrantedBadge {
                        badge_id: badge_grant.badge_id,
                        badge_name: badge_grant.badge_name.clone(),
                        user_badge_id,
                        quantity: badge_grant.quantity,
                    });
                }
                Ok(grant_result) => {
                    // 发放接口返回 success=false，业务层面的拒绝（如库存不足）
                    let err_msg = format!(
                        "徽章发放被拒绝: badge_id={}, 原因={}",
                        badge_grant.badge_id, grant_result.message
                    );
                    warn!(
                        rule_id = badge_grant.rule_id,
                        badge_id = badge_grant.badge_id,
                        message = %grant_result.message,
                        "徽章发放未成功"
                    );
                    errors.push(err_msg);
                }
                Err(e) => {
                    // gRPC 调用失败（网络、超时等），收集错误继续处理其他规则
                    let err_msg = format!(
                        "徽章发放调用失败: badge_id={}, 错误={}",
                        badge_grant.badge_id, e
                    );
                    warn!(
                        rule_id = badge_grant.rule_id,
                        badge_id = badge_grant.badge_id,
                        error = %e,
                        "徽章发放调用异常"
                    );
                    errors.push(err_msg);
                }
            }
        }

        let result = EventResult {
            event_id: event.event_id.clone(),
            processed: true,
            matched_rules,
            granted_badges,
            processing_time_ms: start.elapsed().as_millis() as i64,
            errors,
        };

        info!(
            event_id = %event.event_id,
            matched_count = result.matched_rules.len(),
            granted_count = result.granted_badges.len(),
            error_count = result.errors.len(),
            processing_time_ms = result.processing_time_ms,
            "行为事件处理完成"
        );

        Ok(result)
    }

    /// 本处理器负责的事件类型：所有行为类事件
    fn supported_event_types(&self) -> Vec<EventType> {
        vec![
            EventType::CheckIn,
            EventType::ProfileUpdate,
            EventType::PageView,
            EventType::Share,
            EventType::Review,
        ]
    }

    /// 通过 Redis EXISTS 检查事件是否已处理过
    async fn is_processed(&self, event_id: &str) -> Result<bool, BadgeError> {
        let key = Self::processed_key(event_id);
        let exists = self.cache.exists(&key).await?;

        if exists {
            debug!(event_id, "事件已处理，跳过");
        }

        Ok(exists)
    }

    /// 在 Redis 中设置幂等标记，24 小时后自动过期
    async fn mark_processed(&self, event_id: &str) -> Result<(), BadgeError> {
        let key = Self::processed_key(event_id);
        // 值为 "1" 即可，只需要键的存在性
        self.cache.set(&key, &"1", PROCESSED_TTL).await?;

        debug!(event_id, "事件已标记为已处理");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use badge_shared::rules::ValidationReason;

    fn create_test_rule(rule_id: i64, event_type: &str) -> BadgeGrant {
        BadgeGrant {
            rule_id,
            rule_code: format!("RULE_{}", rule_id),
            badge_id: rule_id * 10,
            badge_name: format!("Badge {}", rule_id),
            quantity: 1,
            event_type: event_type.to_string(),
            start_time: None,
            end_time: None,
            max_count_per_user: None,
            global_quota: None,
            global_granted: 0,
            rule_json: None,
        }
    }

    /// 验证支持的事件类型覆盖所有行为类事件
    #[test]
    fn test_supported_event_types() {
        let expected_types = vec![
            EventType::CheckIn,
            EventType::ProfileUpdate,
            EventType::PageView,
            EventType::Share,
            EventType::Review,
        ];

        assert_eq!(expected_types.len(), 5);
        assert!(expected_types.contains(&EventType::CheckIn));
        assert!(expected_types.contains(&EventType::ProfileUpdate));
        assert!(expected_types.contains(&EventType::PageView));
        assert!(expected_types.contains(&EventType::Share));
        assert!(expected_types.contains(&EventType::Review));

        // 确认不包含非行为类事件
        assert!(!expected_types.contains(&EventType::Purchase));
        assert!(!expected_types.contains(&EventType::Registration));
        assert!(!expected_types.contains(&EventType::SeasonalActivity));
    }

    /// 测试规则映射的分组功能
    #[test]
    fn test_rule_mapping_by_event_type() {
        let mapping = RuleBadgeMapping::new();

        let rules = vec![
            create_test_rule(1, "check_in"),
            create_test_rule(2, "check_in"),
            create_test_rule(3, "share"),
        ];

        mapping.replace_all(rules);

        // 验证按事件类型获取规则
        let checkin_rules = mapping.get_rules_by_event_type("check_in");
        assert_eq!(checkin_rules.len(), 2);

        let share_rules = mapping.get_rules_by_event_type("share");
        assert_eq!(share_rules.len(), 1);
        assert_eq!(share_rules[0].rule_id, 3);

        // 不存在的事件类型应返回空列表
        let nonexistent = mapping.get_rules_by_event_type("nonexistent");
        assert!(nonexistent.is_empty());
    }

    /// 测试 SkippedRule 结构
    #[test]
    fn test_skipped_rule_structure() {
        let skipped = SkippedRule {
            rule_id: 1,
            rule_code: "RULE_001".to_string(),
            skip_reason: ValidationReason::UserLimitExceeded { current: 5, max: 3 },
        };

        assert_eq!(skipped.rule_id, 1);
        assert_eq!(skipped.rule_code, "RULE_001");
        assert!(!skipped.skip_reason.is_allowed());
    }
}
