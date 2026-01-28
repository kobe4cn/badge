//! 行为事件处理器
//!
//! 实现 `EventProcessor` trait，负责行为类事件的完整处理流程：
//! 幂等校验 -> 规则引擎评估 -> 徽章发放 -> 结果汇总。

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use badge_shared::cache::Cache;
use badge_shared::error::BadgeError;
use badge_shared::events::{
    EventPayload, EventProcessor, EventResult, EventType, GrantedBadge, MatchedRule,
};
use tracing::{debug, info, warn};

use crate::rule_client::BadgeRuleService;
use crate::rule_mapping::RuleBadgeMapping;

/// 幂等键前缀，标记事件是否已处理
const PROCESSED_KEY_PREFIX: &str = "event:processed:";
/// 幂等记录保留 24 小时，超过此窗口的重复消费不再拦截，
/// 因为 Kafka 消费偏移量提交周期远短于此窗口
const PROCESSED_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// 行为事件处理器
///
/// 组合三个依赖完成事件处理：
/// - `cache`: Redis 幂等校验
/// - `rule_client`: gRPC 调用（规则引擎 + 徽章管理）
/// - `rule_mapping`: 规则到徽章的映射配置
///
/// 使用 trait object 而非泛型参数，因为处理器会被存储到 Consumer 中，
/// trait object 避免了泛型传播到整个调用链。
pub struct EngagementEventProcessor {
    cache: Cache,
    rule_client: Arc<dyn BadgeRuleService>,
    rule_mapping: Arc<RuleBadgeMapping>,
}

impl EngagementEventProcessor {
    pub fn new(
        cache: Cache,
        rule_client: Arc<dyn BadgeRuleService>,
        rule_mapping: Arc<RuleBadgeMapping>,
    ) -> Self {
        Self {
            cache,
            rule_client,
            rule_mapping,
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
    /// 1. 将事件转为规则引擎评估上下文
    /// 2. 获取所有已注册的规则 ID 批量评估
    /// 3. 对每条匹配规则查找徽章配置并发放
    /// 4. 收集所有结果（含部分失败），不因单条规则失败中断整体流程
    async fn process(&self, event: &EventPayload) -> Result<EventResult, BadgeError> {
        let start = std::time::Instant::now();

        info!(
            event_id = %event.event_id,
            event_type = %event.event_type,
            user_id = %event.user_id,
            "开始处理行为事件"
        );

        // 1. 将事件转为规则引擎评估上下文
        let context = event.to_evaluation_context();

        // 2. 获取所有规则 ID 进行批量评估
        let rule_ids = self.rule_mapping.get_all_rule_ids();
        if rule_ids.is_empty() {
            debug!(event_id = %event.event_id, "无已注册规则，跳过评估");
            return Ok(EventResult {
                event_id: event.event_id.clone(),
                processed: true,
                matched_rules: vec![],
                granted_badges: vec![],
                processing_time_ms: start.elapsed().as_millis() as i64,
                errors: vec![],
            });
        }

        // 3. 调用规则引擎批量评估
        let matches = self
            .rule_client
            .evaluate_rules(&rule_ids, context)
            .await
            .map_err(|e| BadgeError::Internal(format!("规则评估失败: {e}")))?;

        let mut matched_rules = Vec::new();
        let mut granted_badges = Vec::new();
        let mut errors = Vec::new();

        // 4. 对每条匹配的规则查找徽章配置并发放
        for rule_match in &matches {
            let Some(badge_grant) = self.rule_mapping.get_grant(&rule_match.rule_id) else {
                // 规则匹配但无对应徽章配置，说明映射数据不完整
                warn!(
                    rule_id = %rule_match.rule_id,
                    rule_name = %rule_match.rule_name,
                    "规则匹配但未找到对应的徽章映射，跳过发放"
                );
                continue;
            };

            // 记录匹配的规则
            matched_rules.push(MatchedRule {
                rule_id: rule_match.rule_id.clone(),
                rule_name: rule_match.rule_name.clone(),
                badge_id: badge_grant.badge_id,
                badge_name: badge_grant.badge_name.clone(),
                quantity: badge_grant.quantity,
            });

            // 5. 调用徽章发放，失败只记录不中断
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
                        rule_id = %rule_match.rule_id,
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
                        rule_id = %rule_match.rule_id,
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
    use crate::rule_client::{GrantResult, RuleMatch};
    use crate::rule_mapping::BadgeGrant;

    /// Mock 实现：模拟 gRPC 客户端行为，无需真实网络连接
    struct MockBadgeRuleService {
        /// 预设的规则评估结果
        evaluate_result: Vec<RuleMatch>,
        /// 预设的徽章发放结果
        grant_result: GrantResult,
    }

    impl MockBadgeRuleService {
        fn new(evaluate_result: Vec<RuleMatch>, grant_result: GrantResult) -> Self {
            Self {
                evaluate_result,
                grant_result,
            }
        }
    }

    #[async_trait]
    impl BadgeRuleService for MockBadgeRuleService {
        async fn evaluate_rules(
            &self,
            _rule_ids: &[String],
            _context: serde_json::Value,
        ) -> Result<Vec<RuleMatch>, crate::error::EngagementError> {
            Ok(self.evaluate_result.clone())
        }

        async fn grant_badge(
            &self,
            _user_id: &str,
            _badge_id: &str,
            _quantity: i32,
            _source_ref: &str,
        ) -> Result<GrantResult, crate::error::EngagementError> {
            Ok(self.grant_result.clone())
        }
    }

    /// 构造测试用的 processor，注入 mock 客户端
    fn make_test_processor(
        mock_service: MockBadgeRuleService,
        rule_mapping: RuleBadgeMapping,
    ) -> EngagementEventProcessor {
        let config = badge_shared::config::RedisConfig {
            url: "redis://localhost:6379".to_string(),
            pool_size: 1,
        };
        let cache = Cache::new(&config).expect("Redis client 创建失败");

        EngagementEventProcessor::new(cache, Arc::new(mock_service), Arc::new(rule_mapping))
    }

    /// 验证支持的事件类型覆盖所有行为类事件
    #[test]
    fn test_supported_event_types() {
        let mock = MockBadgeRuleService::new(
            vec![],
            GrantResult {
                success: true,
                user_badge_id: "0".to_string(),
                message: String::new(),
            },
        );
        let processor = make_test_processor(mock, RuleBadgeMapping::new());

        let types = processor.supported_event_types();

        assert_eq!(types.len(), 5);
        assert!(types.contains(&EventType::CheckIn));
        assert!(types.contains(&EventType::ProfileUpdate));
        assert!(types.contains(&EventType::PageView));
        assert!(types.contains(&EventType::Share));
        assert!(types.contains(&EventType::Review));

        // 确认不包含非行为类事件
        assert!(!types.contains(&EventType::Purchase));
        assert!(!types.contains(&EventType::Registration));
        assert!(!types.contains(&EventType::SeasonalActivity));
    }

    /// 无注册规则时，直接返回空结果
    #[tokio::test]
    async fn test_process_no_rules() {
        let mock = MockBadgeRuleService::new(
            vec![],
            GrantResult {
                success: true,
                user_badge_id: "0".to_string(),
                message: String::new(),
            },
        );
        let processor = make_test_processor(mock, RuleBadgeMapping::new());

        let event = EventPayload::new(
            EventType::CheckIn,
            "user-001",
            serde_json::json!({"location": "北京"}),
            "test",
        );

        let result = processor.process(&event).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.event_id, event.event_id);
        assert!(result.processed);
        assert!(result.matched_rules.is_empty());
        assert!(result.granted_badges.is_empty());
        assert!(result.errors.is_empty());
        assert!(result.processing_time_ms >= 0);
    }

    /// 规则匹配并成功发放徽章
    #[tokio::test]
    async fn test_process_with_matched_rules() {
        let mock = MockBadgeRuleService::new(
            vec![RuleMatch {
                rule_id: "rule-001".to_string(),
                rule_name: "每日签到奖励".to_string(),
                matched_conditions: vec!["event_type == CHECK_IN".to_string()],
            }],
            GrantResult {
                success: true,
                user_badge_id: "1001".to_string(),
                message: "发放成功".to_string(),
            },
        );

        let mapping = RuleBadgeMapping::new();
        mapping.add_mapping(
            "rule-001",
            BadgeGrant {
                badge_id: 42,
                badge_name: "每日签到".to_string(),
                quantity: 1,
            },
        );

        let processor = make_test_processor(mock, mapping);

        let event = EventPayload::new(
            EventType::CheckIn,
            "user-001",
            serde_json::json!({"location": "北京"}),
            "test",
        );

        let result = processor.process(&event).await.unwrap();

        assert!(result.processed);
        assert_eq!(result.matched_rules.len(), 1);
        assert_eq!(result.matched_rules[0].rule_id, "rule-001");
        assert_eq!(result.matched_rules[0].badge_id, 42);
        assert_eq!(result.matched_rules[0].badge_name, "每日签到");
        assert_eq!(result.granted_badges.len(), 1);
        assert_eq!(result.granted_badges[0].badge_id, 42);
        assert_eq!(result.granted_badges[0].user_badge_id, 1001);
        assert!(result.errors.is_empty());
    }

    /// 规则匹配但徽章发放失败时，错误被收集而非中断流程
    #[tokio::test]
    async fn test_process_grant_failure_collected_as_error() {
        let mock = MockBadgeRuleService::new(
            vec![RuleMatch {
                rule_id: "rule-001".to_string(),
                rule_name: "每日签到奖励".to_string(),
                matched_conditions: vec![],
            }],
            GrantResult {
                success: false,
                user_badge_id: String::new(),
                message: "库存不足".to_string(),
            },
        );

        let mapping = RuleBadgeMapping::new();
        mapping.add_mapping(
            "rule-001",
            BadgeGrant {
                badge_id: 42,
                badge_name: "每日签到".to_string(),
                quantity: 1,
            },
        );

        let processor = make_test_processor(mock, mapping);

        let event = EventPayload::new(
            EventType::CheckIn,
            "user-001",
            serde_json::json!({}),
            "test",
        );

        let result = processor.process(&event).await.unwrap();

        // 规则匹配了但发放失败
        assert_eq!(result.matched_rules.len(), 1);
        assert!(result.granted_badges.is_empty());
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("库存不足"));
    }

    /// 规则匹配但无对应徽章映射时，跳过发放
    #[tokio::test]
    async fn test_process_matched_rule_without_badge_mapping() {
        let mock = MockBadgeRuleService::new(
            vec![RuleMatch {
                rule_id: "rule-orphan".to_string(),
                rule_name: "孤立规则".to_string(),
                matched_conditions: vec![],
            }],
            GrantResult {
                success: true,
                user_badge_id: "1".to_string(),
                message: String::new(),
            },
        );

        // 映射中注册了不同的规则 ID
        let mapping = RuleBadgeMapping::new();
        mapping.add_mapping(
            "rule-other",
            BadgeGrant {
                badge_id: 99,
                badge_name: "其他徽章".to_string(),
                quantity: 1,
            },
        );

        let processor = make_test_processor(mock, mapping);

        let event = EventPayload::new(EventType::Share, "user-002", serde_json::json!({}), "test");

        let result = processor.process(&event).await.unwrap();

        // 匹配了规则但映射中找不到，不应有匹配规则记录
        assert!(result.matched_rules.is_empty());
        assert!(result.granted_badges.is_empty());
        assert!(result.errors.is_empty());
    }
}
