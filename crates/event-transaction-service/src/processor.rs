//! 交易事件处理器
//!
//! 实现 `EventProcessor` trait，负责交易类事件的完整处理流程。
//! 与行为事件处理器不同，本处理器需要根据事件类型区分两条处理路径：
//! - **Purchase**: 评估规则 -> 匹配则发放徽章（与行为服务一致）
//! - **Refund / OrderCancel**: 查找原订单关联的徽章 -> 执行撤销
//!
//! 退款撤销是交易事件服务的核心差异点：购买发放的徽章在退款时需要回收，
//! 避免用户通过"购买 -> 获取徽章 -> 退款"的方式白嫖徽章。

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use badge_shared::cache::Cache;
use badge_shared::error::BadgeError;
use badge_shared::events::{
    EventPayload, EventProcessor, EventResult, EventType, GrantedBadge, MatchedRule,
};
use tracing::{debug, info, warn};

use crate::rule_client::{RevokeResult, TransactionRuleService};
use crate::rule_mapping::RuleBadgeMapping;

/// 幂等键前缀，标记事件是否已处理
const PROCESSED_KEY_PREFIX: &str = "event:txn:processed:";
/// 幂等记录保留 24 小时，超过此窗口的重复消费不再拦截
const PROCESSED_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// 交易事件处理器
///
/// 组合三个依赖完成事件处理：
/// - `cache`: Redis 幂等校验
/// - `rule_client`: gRPC 调用（规则引擎 + 徽章管理 + 徽章撤销）
/// - `rule_mapping`: 规则到徽章的映射配置
///
/// 使用 trait object 而非泛型参数，因为处理器会被存储到 Consumer 中，
/// trait object 避免了泛型传播到整个调用链。
pub struct TransactionEventProcessor {
    cache: Cache,
    rule_client: Arc<dyn TransactionRuleService>,
    rule_mapping: Arc<RuleBadgeMapping>,
}

impl TransactionEventProcessor {
    pub fn new(
        cache: Cache,
        rule_client: Arc<dyn TransactionRuleService>,
        rule_mapping: Arc<RuleBadgeMapping>,
    ) -> Self {
        Self {
            cache,
            rule_client,
            rule_mapping,
        }
    }

    /// 构造 Redis 幂等键，使用 txn 前缀区分行为事件的幂等键
    fn processed_key(event_id: &str) -> String {
        format!("{PROCESSED_KEY_PREFIX}{event_id}")
    }

    /// 处理购买事件：评估规则 -> 匹配则发放徽章
    ///
    /// 与行为事件服务的处理流程完全一致，仅事件类型不同
    async fn process_purchase(&self, event: &EventPayload) -> Result<EventResult, BadgeError> {
        let start = std::time::Instant::now();

        let context = event.to_evaluation_context();
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

        let matches = self
            .rule_client
            .evaluate_rules(&rule_ids, context)
            .await
            .map_err(|e| BadgeError::Internal(format!("规则评估失败: {e}")))?;

        let mut matched_rules = Vec::new();
        let mut granted_badges = Vec::new();
        let mut errors = Vec::new();

        for rule_match in &matches {
            let Some(badge_grant) = self.rule_mapping.get_grant(&rule_match.rule_id) else {
                warn!(
                    rule_id = %rule_match.rule_id,
                    rule_name = %rule_match.rule_name,
                    "规则匹配但未找到对应的徽章映射，跳过发放"
                );
                continue;
            };

            matched_rules.push(MatchedRule {
                rule_id: rule_match.rule_id.clone(),
                rule_name: rule_match.rule_name.clone(),
                badge_id: badge_grant.badge_id,
                badge_name: badge_grant.badge_name.clone(),
                quantity: badge_grant.quantity,
            });

            match self
                .rule_client
                .grant_badge(
                    &event.user_id,
                    badge_grant.badge_id,
                    badge_grant.quantity,
                    &event.event_id,
                )
                .await
            {
                Ok(grant_result) if grant_result.success => {
                    let user_badge_id = grant_result.user_badge_id.parse::<i64>().unwrap_or(0);
                    granted_badges.push(GrantedBadge {
                        badge_id: badge_grant.badge_id,
                        badge_name: badge_grant.badge_name.clone(),
                        user_badge_id,
                        quantity: badge_grant.quantity,
                    });
                }
                Ok(grant_result) => {
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

        Ok(EventResult {
            event_id: event.event_id.clone(),
            processed: true,
            matched_rules,
            granted_badges,
            processing_time_ms: start.elapsed().as_millis() as i64,
            errors,
        })
    }

    /// 处理退款/取消事件：查找原订单关联的徽章并撤销
    ///
    /// 退款撤销的业务逻辑：
    /// 1. 从事件 data 中读取 original_order_id 和退款原因
    /// 2. 尝试读取 badge_ids（显式指定要撤销的徽章列表）
    /// 3. 如果未指定 badge_ids，则从 rule_mapping 中查找所有可能的徽章
    /// 4. 逐个调用 revoke_badge，收集结果
    async fn process_refund(&self, event: &EventPayload) -> Result<EventResult, BadgeError> {
        let start = std::time::Instant::now();

        let original_order_id = event.data["original_order_id"]
            .as_str()
            .unwrap_or("unknown");
        let refund_reason = event.data["refund_reason"]
            .as_str()
            .unwrap_or("未指定退款原因");

        info!(
            event_id = %event.event_id,
            original_order_id,
            refund_reason,
            "开始处理退款/取消撤销"
        );

        // 确定需要撤销的徽章列表
        let badge_ids: Vec<i64> = if let Some(ids) = event.data["badge_ids"].as_array() {
            // 事件中显式指定了需要撤销的徽章
            ids.iter().filter_map(|v| v.as_i64()).collect()
        } else {
            // 未指定时，从映射中获取所有可能的徽章 ID 作为撤销目标
            self.rule_mapping
                .get_all_rule_ids()
                .iter()
                .filter_map(|rule_id| self.rule_mapping.get_grant(rule_id))
                .map(|grant| grant.badge_id)
                .collect()
        };

        if badge_ids.is_empty() {
            debug!(
                event_id = %event.event_id,
                "无需撤销的徽章，跳过"
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

        let mut errors = Vec::new();
        let mut revoke_results: Vec<RevokeResult> = Vec::new();

        let reason = format!(
            "退款撤销: original_order={}, reason={}",
            original_order_id, refund_reason
        );

        for badge_id in &badge_ids {
            match self
                .rule_client
                .revoke_badge(&event.user_id, *badge_id, 1, &reason)
                .await
            {
                Ok(result) if result.success => {
                    info!(
                        user_id = %event.user_id,
                        badge_id,
                        "徽章撤销成功"
                    );
                    revoke_results.push(result);
                }
                Ok(result) => {
                    let err_msg = format!(
                        "徽章撤销被拒绝: badge_id={}, 原因={}",
                        badge_id, result.message
                    );
                    warn!(
                        user_id = %event.user_id,
                        badge_id,
                        message = %result.message,
                        "徽章撤销未成功"
                    );
                    errors.push(err_msg);
                }
                Err(e) => {
                    let err_msg = format!("徽章撤销调用失败: badge_id={}, 错误={}", badge_id, e);
                    warn!(
                        user_id = %event.user_id,
                        badge_id,
                        error = %e,
                        "徽章撤销调用异常"
                    );
                    errors.push(err_msg);
                }
            }
        }

        Ok(EventResult {
            event_id: event.event_id.clone(),
            processed: true,
            // 退款撤销不涉及规则匹配和徽章发放
            matched_rules: vec![],
            granted_badges: vec![],
            processing_time_ms: start.elapsed().as_millis() as i64,
            errors,
        })
    }
}

#[async_trait]
impl EventProcessor for TransactionEventProcessor {
    /// 根据事件类型分发到不同的处理路径
    async fn process(&self, event: &EventPayload) -> Result<EventResult, BadgeError> {
        info!(
            event_id = %event.event_id,
            event_type = %event.event_type,
            user_id = %event.user_id,
            "开始处理交易事件"
        );

        let result = match event.event_type {
            EventType::Purchase => self.process_purchase(event).await?,
            EventType::Refund | EventType::OrderCancel => self.process_refund(event).await?,
            _ => {
                return Err(BadgeError::Internal(format!(
                    "不支持的交易事件类型: {}",
                    event.event_type
                )));
            }
        };

        info!(
            event_id = %event.event_id,
            event_type = %event.event_type,
            matched_count = result.matched_rules.len(),
            granted_count = result.granted_badges.len(),
            error_count = result.errors.len(),
            processing_time_ms = result.processing_time_ms,
            "交易事件处理完成"
        );

        Ok(result)
    }

    /// 本处理器负责的事件类型：所有交易类事件
    fn supported_event_types(&self) -> Vec<EventType> {
        vec![
            EventType::Purchase,
            EventType::Refund,
            EventType::OrderCancel,
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
        self.cache.set(&key, &"1", PROCESSED_TTL).await?;

        debug!(event_id, "事件已标记为已处理");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule_client::{GrantResult, RevokeResult, RuleMatch};
    use crate::rule_mapping::BadgeGrant;

    /// Mock 实现：模拟 gRPC 客户端行为，无需真实网络连接
    struct MockTransactionRuleService {
        evaluate_result: Vec<RuleMatch>,
        grant_result: GrantResult,
        revoke_result: RevokeResult,
    }

    impl MockTransactionRuleService {
        fn new(
            evaluate_result: Vec<RuleMatch>,
            grant_result: GrantResult,
            revoke_result: RevokeResult,
        ) -> Self {
            Self {
                evaluate_result,
                grant_result,
                revoke_result,
            }
        }
    }

    #[async_trait]
    impl TransactionRuleService for MockTransactionRuleService {
        async fn evaluate_rules(
            &self,
            _rule_ids: &[String],
            _context: serde_json::Value,
        ) -> Result<Vec<RuleMatch>, crate::error::TransactionError> {
            Ok(self.evaluate_result.clone())
        }

        async fn grant_badge(
            &self,
            _user_id: &str,
            _badge_id: i64,
            _quantity: i32,
            _source_ref: &str,
        ) -> Result<GrantResult, crate::error::TransactionError> {
            Ok(self.grant_result.clone())
        }

        async fn revoke_badge(
            &self,
            _user_id: &str,
            _badge_id: i64,
            _quantity: i32,
            _reason: &str,
        ) -> Result<RevokeResult, crate::error::TransactionError> {
            Ok(self.revoke_result.clone())
        }
    }

    /// 构造测试用的 processor，注入 mock 客户端
    fn make_test_processor(
        mock_service: MockTransactionRuleService,
        rule_mapping: RuleBadgeMapping,
    ) -> TransactionEventProcessor {
        let config = badge_shared::config::RedisConfig {
            url: "redis://localhost:6379".to_string(),
            pool_size: 1,
        };
        let cache = Cache::new(&config).expect("Redis client 创建失败");

        TransactionEventProcessor::new(cache, Arc::new(mock_service), Arc::new(rule_mapping))
    }

    /// 验证支持的事件类型覆盖所有交易类事件
    #[test]
    fn test_supported_event_types() {
        let mock = MockTransactionRuleService::new(
            vec![],
            GrantResult {
                success: true,
                user_badge_id: "0".to_string(),
                message: String::new(),
            },
            RevokeResult {
                success: true,
                message: String::new(),
            },
        );
        let processor = make_test_processor(mock, RuleBadgeMapping::new());

        let types = processor.supported_event_types();

        assert_eq!(types.len(), 3);
        assert!(types.contains(&EventType::Purchase));
        assert!(types.contains(&EventType::Refund));
        assert!(types.contains(&EventType::OrderCancel));

        // 确认不包含非交易类事件
        assert!(!types.contains(&EventType::CheckIn));
        assert!(!types.contains(&EventType::Registration));
        assert!(!types.contains(&EventType::SeasonalActivity));
    }

    /// 无注册规则时，Purchase 直接返回空结果
    #[tokio::test]
    async fn test_process_purchase_no_rules() {
        let mock = MockTransactionRuleService::new(
            vec![],
            GrantResult {
                success: true,
                user_badge_id: "0".to_string(),
                message: String::new(),
            },
            RevokeResult {
                success: true,
                message: String::new(),
            },
        );
        let processor = make_test_processor(mock, RuleBadgeMapping::new());

        let event = EventPayload::new(
            EventType::Purchase,
            "user-001",
            serde_json::json!({"amount": 100.0, "category": "electronics"}),
            "order-service",
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

    /// Purchase 事件匹配规则并成功发放徽章
    #[tokio::test]
    async fn test_process_purchase_with_matched_rules() {
        let mock = MockTransactionRuleService::new(
            vec![RuleMatch {
                rule_id: "rule-txn-001".to_string(),
                rule_name: "首次购物奖励".to_string(),
                matched_conditions: vec!["event_type == PURCHASE".to_string()],
            }],
            GrantResult {
                success: true,
                user_badge_id: "2001".to_string(),
                message: "发放成功".to_string(),
            },
            RevokeResult {
                success: true,
                message: String::new(),
            },
        );

        let mapping = RuleBadgeMapping::new();
        mapping.add_mapping(
            "rule-txn-001",
            BadgeGrant {
                badge_id: 42,
                badge_name: "首次购物".to_string(),
                quantity: 1,
            },
        );

        let processor = make_test_processor(mock, mapping);

        let event = EventPayload::new(
            EventType::Purchase,
            "user-001",
            serde_json::json!({"amount": 200.0}),
            "order-service",
        );

        let result = processor.process(&event).await.unwrap();

        assert!(result.processed);
        assert_eq!(result.matched_rules.len(), 1);
        assert_eq!(result.matched_rules[0].rule_id, "rule-txn-001");
        assert_eq!(result.matched_rules[0].badge_id, 42);
        assert_eq!(result.matched_rules[0].badge_name, "首次购物");
        assert_eq!(result.granted_badges.len(), 1);
        assert_eq!(result.granted_badges[0].badge_id, 42);
        assert_eq!(result.granted_badges[0].user_badge_id, 2001);
        assert!(result.errors.is_empty());
    }

    /// Refund 事件携带 badge_ids 时，按指定列表撤销
    #[tokio::test]
    async fn test_process_refund_with_explicit_badge_ids() {
        let mock = MockTransactionRuleService::new(
            vec![],
            GrantResult {
                success: true,
                user_badge_id: "0".to_string(),
                message: String::new(),
            },
            RevokeResult {
                success: true,
                message: "撤销成功".to_string(),
            },
        );
        let processor = make_test_processor(mock, RuleBadgeMapping::new());

        let event = EventPayload::new(
            EventType::Refund,
            "user-001",
            serde_json::json!({
                "original_order_id": "order-123",
                "badge_ids": [42, 43],
                "refund_reason": "商品质量问题"
            }),
            "order-service",
        );

        let result = processor.process(&event).await.unwrap();

        assert!(result.processed);
        // 退款撤销不涉及规则匹配和徽章发放
        assert!(result.matched_rules.is_empty());
        assert!(result.granted_badges.is_empty());
        // mock 返回 success=true，应无错误
        assert!(result.errors.is_empty());
    }

    /// OrderCancel 与 Refund 使用相同的撤销逻辑
    #[tokio::test]
    async fn test_process_order_cancel() {
        let mock = MockTransactionRuleService::new(
            vec![],
            GrantResult {
                success: true,
                user_badge_id: "0".to_string(),
                message: String::new(),
            },
            RevokeResult {
                success: true,
                message: "撤销成功".to_string(),
            },
        );
        let processor = make_test_processor(mock, RuleBadgeMapping::new());

        let event = EventPayload::new(
            EventType::OrderCancel,
            "user-002",
            serde_json::json!({
                "original_order_id": "order-456",
                "badge_ids": [42],
                "refund_reason": "用户主动取消"
            }),
            "order-service",
        );

        let result = processor.process(&event).await.unwrap();
        assert!(result.processed);
        assert!(result.errors.is_empty());
    }

    /// 退款撤销失败时，错误被收集而非中断流程
    #[tokio::test]
    async fn test_process_refund_revoke_failure_collected_as_error() {
        let mock = MockTransactionRuleService::new(
            vec![],
            GrantResult {
                success: true,
                user_badge_id: "0".to_string(),
                message: String::new(),
            },
            RevokeResult {
                success: false,
                message: "用户未持有该徽章".to_string(),
            },
        );
        let processor = make_test_processor(mock, RuleBadgeMapping::new());

        let event = EventPayload::new(
            EventType::Refund,
            "user-001",
            serde_json::json!({
                "original_order_id": "order-789",
                "badge_ids": [42],
                "refund_reason": "七天无理由"
            }),
            "order-service",
        );

        let result = processor.process(&event).await.unwrap();

        assert!(result.processed);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("用户未持有该徽章"));
    }

    /// 无 badge_ids 且无映射规则时，退款直接返回空结果
    #[tokio::test]
    async fn test_process_refund_no_badges_to_revoke() {
        let mock = MockTransactionRuleService::new(
            vec![],
            GrantResult {
                success: true,
                user_badge_id: "0".to_string(),
                message: String::new(),
            },
            RevokeResult {
                success: true,
                message: String::new(),
            },
        );
        // 空映射 + 事件中无 badge_ids
        let processor = make_test_processor(mock, RuleBadgeMapping::new());

        let event = EventPayload::new(
            EventType::Refund,
            "user-001",
            serde_json::json!({
                "original_order_id": "order-000",
                "refund_reason": "无相关徽章"
            }),
            "order-service",
        );

        let result = processor.process(&event).await.unwrap();

        assert!(result.processed);
        assert!(result.matched_rules.is_empty());
        assert!(result.granted_badges.is_empty());
        assert!(result.errors.is_empty());
    }
}
