//! 徽章取消服务
//!
//! 处理徽章取消/撤销的核心业务逻辑，包括：
//! - 用户徽章存在性检查
//! - 余额充足性检查
//! - 事务性扣减（用户徽章、账本流水）
//! - 状态变更（数量归零时标记为 Revoked）
//! - 发送撤销通知
//!
//! ## 取消流程
//!
//! 1. 参数校验 -> 2. 查询用户徽章 -> 3. 余额检查 -> 4. 事务内扣减 -> 5. 缓存失效 -> 6. 发送通知

use std::sync::Arc;

use chrono::Utc;
use sqlx::PgPool;
use tokio::sync::RwLock;
use tracing::{info, instrument, warn};

use badge_shared::cache::Cache;

use crate::error::{BadgeError, Result};
use crate::models::{BadgeLedger, ChangeType, LogAction, UserBadgeStatus};
use crate::notification::NotificationSender;
use crate::repository::{BadgeLedgerRepository, BadgeRepositoryTrait, UserBadgeRepository};
use crate::service::dto::{
    BadgeGrantCondition, BatchRevokeResponse, RefundEvent, RefundProcessResult, RetainedBadgeInfo,
    RevokeBadgeRequest, RevokeBadgeResponse, RevokeResult, RevokedBadgeInfo,
};

/// 缓存键生成
mod cache_keys {
    pub fn user_badges(user_id: &str) -> String {
        format!("user:badge:{}", user_id)
    }

    pub fn badge_wall(user_id: &str) -> String {
        format!("user:badge:wall:{}", user_id)
    }
}

/// 徽章取消服务
///
/// 负责徽章取消的完整流程，包括验证、事务处理、缓存管理和通知发送
pub struct RevokeService<BR = crate::repository::BadgeRepository>
where
    BR: BadgeRepositoryTrait,
{
    cache: Arc<Cache>,
    pool: PgPool,
    /// 徽章仓储（用于获取徽章名称发送通知）
    badge_repo: Arc<BR>,
    /// 通知发送器（可选，用于发送徽章撤销通知）
    notification_sender: RwLock<Option<Arc<NotificationSender>>>,
}

impl<BR> RevokeService<BR>
where
    BR: BadgeRepositoryTrait,
{
    pub fn new(cache: Arc<Cache>, pool: PgPool, badge_repo: Arc<BR>) -> Self {
        Self {
            cache,
            pool,
            badge_repo,
            notification_sender: RwLock::new(None),
        }
    }

    /// 设置通知发送器
    ///
    /// 在服务初始化后注入通知发送器，用于发送徽章撤销通知。
    pub async fn set_notification_sender(&self, sender: Arc<NotificationSender>) {
        let mut guard = self.notification_sender.write().await;
        *guard = Some(sender);
        info!("RevokeService 通知发送器已设置");
    }

    /// 取消/撤销徽章
    ///
    /// 完整的取消流程：
    /// 1. 参数校验（quantity > 0, reason 非空）
    /// 2. 查询用户徽章记录
    /// 3. 检查徽章状态和余额
    /// 4. 事务内执行扣减
    /// 5. 清除缓存
    /// 6. 发送撤销通知
    #[instrument(skip(self), fields(user_id = %request.user_id, badge_id = %request.badge_id, quantity = %request.quantity))]
    pub async fn revoke_badge(&self, request: RevokeBadgeRequest) -> Result<RevokeBadgeResponse> {
        // 1. 参数校验
        self.validate_request(&request)?;

        // 2-4. 事务内执行取消操作
        let remaining_quantity = self.execute_revoke(&request).await?;

        // 5. 清除缓存
        self.invalidate_user_cache(&request.user_id).await;

        // 6. 发送撤销通知（异步，不阻塞主流程）
        self.send_revoke_notification(&request.user_id, request.badge_id, &request.reason)
            .await;

        info!(
            user_id = %request.user_id,
            badge_id = %request.badge_id,
            quantity = request.quantity,
            remaining = remaining_quantity,
            "徽章取消成功"
        );

        Ok(RevokeBadgeResponse::success(remaining_quantity))
    }

    /// 发送徽章撤销通知
    ///
    /// 异步发送通知，失败不影响主撤销流程
    async fn send_revoke_notification(&self, user_id: &str, badge_id: i64, reason: &str) {
        let sender = {
            let guard = self.notification_sender.read().await;
            guard.clone()
        };

        if let Some(sender) = sender {
            // 获取徽章名称用于通知
            if let Ok(Some(badge)) = self.badge_repo.get_badge(badge_id).await {
                sender.send_badge_revoked(user_id, badge_id, &badge.name, reason);
            }
        }
    }

    /// 批量取消徽章
    ///
    /// 对每个请求独立处理，单个失败不影响其他请求
    #[instrument(skip(self), fields(request_count = requests.len()))]
    pub async fn batch_revoke_badges(
        &self,
        requests: Vec<RevokeBadgeRequest>,
    ) -> Result<BatchRevokeResponse> {
        let total = requests.len() as i32;
        let mut results = Vec::with_capacity(requests.len());
        let mut success_count = 0;
        let mut failed_count = 0;

        for request in requests {
            let user_id = request.user_id.clone();
            let badge_id = request.badge_id;

            match self.revoke_badge(request).await {
                Ok(response) => {
                    success_count += 1;
                    results.push(RevokeResult::success(
                        user_id,
                        badge_id,
                        response.remaining_quantity,
                    ));
                }
                Err(e) => {
                    failed_count += 1;
                    warn!(user_id = %user_id, badge_id = %badge_id, error = %e, "批量取消单条失败");
                    results.push(RevokeResult::failure(user_id, badge_id, e.to_string()));
                }
            }
        }

        info!(
            total = total,
            success = success_count,
            failed = failed_count,
            "批量取消完成"
        );

        Ok(BatchRevokeResponse {
            total,
            success_count,
            failed_count,
            results,
        })
    }

    /// 处理退款事件
    ///
    /// 根据退款金额判断是否需要撤销已发放的徽章：
    /// - 全额退款：撤销所有由该订单触发发放的徽章
    /// - 部分退款（金额仍满足条件）：保留徽章
    /// - 部分退款（金额不满足条件）：撤销徽章
    #[instrument(skip(self, conditions), fields(
        event_id = %event.event_id,
        user_id = %event.user_id,
        original_order_id = %event.original_order_id,
        refund_amount = %event.refund_amount
    ))]
    pub async fn handle_refund(
        &self,
        event: &RefundEvent,
        conditions: &[BadgeGrantCondition],
    ) -> Result<RefundProcessResult> {
        let start = std::time::Instant::now();

        info!(
            original_amount = event.original_amount,
            refund_amount = event.refund_amount,
            remaining_amount = event.remaining_amount,
            is_full_refund = event.is_full_refund(),
            "开始处理退款事件"
        );

        let mut revoked_badges = Vec::new();
        let mut retained_badges = Vec::new();

        // 确定需要检查的徽章列表
        let badges_to_check: Vec<i64> = if let Some(ref badge_ids) = event.badge_ids_to_revoke {
            // 事件中显式指定了需要撤销的徽章
            badge_ids.clone()
        } else {
            // 从规则条件中提取所有相关的徽章 ID
            conditions.iter().map(|c| c.badge_id).collect()
        };

        if badges_to_check.is_empty() {
            info!("无需检查的徽章，跳过退款处理");
            return Ok(RefundProcessResult::success(
                event.event_id.clone(),
                vec![],
                vec![],
                start.elapsed().as_millis() as i64,
            ));
        }

        // 获取退款后的有效消费金额
        let effective_amount = event.effective_amount();

        for badge_id in badges_to_check {
            // 查找该徽章的发放条件
            let condition = conditions.iter().find(|c| c.badge_id == badge_id);

            // 获取徽章名称
            let badge_name = if let Ok(Some(badge)) = self.badge_repo.get_badge(badge_id).await {
                badge.name.clone()
            } else {
                format!("Badge {}", badge_id)
            };

            // 判断是否需要撤销
            let should_revoke = if event.is_full_refund() {
                // 全额退款：直接撤销
                true
            } else if let Some(cond) = condition {
                // 部分退款：检查是否仍满足金额阈值
                if let Some(threshold) = cond.amount_threshold {
                    effective_amount < threshold
                } else {
                    // 无金额阈值条件，不撤销
                    false
                }
            } else {
                // 无规则条件，全额退款撤销，部分退款不撤销
                event.is_full_refund()
            };

            if should_revoke {
                // 执行撤销
                let reason = format!(
                    "退款撤销: refund_order={}, original_order={}, refund_amount={}",
                    event.refund_order_id, event.original_order_id, event.refund_amount
                );

                let revoke_request = RevokeBadgeRequest::system(&event.user_id, badge_id, 1, &reason)
                    .with_source_ref(&event.refund_order_id);

                match self.revoke_badge(revoke_request).await {
                    Ok(_) => {
                        revoked_badges.push(RevokedBadgeInfo {
                            badge_id,
                            badge_name,
                            quantity: 1,
                            reason,
                        });
                    }
                    Err(e) => {
                        // 撤销失败（可能用户已经没有该徽章）
                        warn!(
                            badge_id,
                            error = %e,
                            "徽章撤销失败，可能用户已不持有该徽章"
                        );
                    }
                }
            } else {
                // 保留徽章
                let reason = if let Some(cond) = condition {
                    if let Some(threshold) = cond.amount_threshold {
                        format!(
                            "部分退款后金额 {} 仍满足阈值 {}",
                            effective_amount, threshold
                        )
                    } else {
                        "规则无金额阈值要求".to_string()
                    }
                } else {
                    "未找到发放规则条件".to_string()
                };

                retained_badges.push(RetainedBadgeInfo {
                    badge_id,
                    badge_name,
                    reason,
                });
            }
        }

        let processing_time_ms = start.elapsed().as_millis() as i64;

        info!(
            revoked_count = revoked_badges.len(),
            retained_count = retained_badges.len(),
            processing_time_ms,
            "退款事件处理完成"
        );

        Ok(RefundProcessResult::success(
            event.event_id.clone(),
            revoked_badges,
            retained_badges,
            processing_time_ms,
        ))
    }

    /// 检查是否已处理过该退款事件（幂等检查）
    ///
    /// 通过 Redis 检查事件 ID 是否已处理，防止重复处理
    pub async fn is_refund_processed(&self, event_id: &str) -> Result<bool> {
        let key = format!("refund:processed:{}", event_id);
        let exists = self
            .cache
            .exists(&key)
            .await
            .map_err(|e| BadgeError::Redis(e.to_string()))?;
        Ok(exists)
    }

    /// 标记退款事件为已处理
    ///
    /// 在 Redis 中设置幂等标记，24 小时后自动过期
    pub async fn mark_refund_processed(&self, event_id: &str) -> Result<()> {
        let key = format!("refund:processed:{}", event_id);
        self.cache
            .set(&key, &"1", std::time::Duration::from_secs(24 * 60 * 60))
            .await
            .map_err(|e| BadgeError::Redis(e.to_string()))?;
        Ok(())
    }

    // ==================== 私有方法 ====================

    /// 参数校验
    fn validate_request(&self, request: &RevokeBadgeRequest) -> Result<()> {
        if request.quantity <= 0 {
            return Err(BadgeError::Validation("取消数量必须大于0".to_string()));
        }

        if request.reason.trim().is_empty() {
            return Err(BadgeError::Validation("取消原因不能为空".to_string()));
        }

        Ok(())
    }

    /// 执行取消事务
    ///
    /// 在单个事务内完成：
    /// - 查询用户徽章（带行级锁）
    /// - 检查状态和余额
    /// - 扣减数量
    /// - 更新状态（如归零）
    /// - 写入账本流水
    /// - 写入操作日志
    async fn execute_revoke(&self, request: &RevokeBadgeRequest) -> Result<i32> {
        let mut tx = self.pool.begin().await?;

        // 2. 查询用户徽章（带行级锁防止并发）
        let user_badge = UserBadgeRepository::get_user_badge_for_update(
            &mut tx,
            &request.user_id,
            request.badge_id,
        )
        .await?
        .ok_or_else(|| BadgeError::UserBadgeNotFound {
            user_id: request.user_id.clone(),
            badge_id: request.badge_id,
        })?;

        // 检查徽章状态：只允许取消 Active 状态的徽章
        if user_badge.status != UserBadgeStatus::Active {
            return Err(BadgeError::Validation(format!(
                "徽章状态不允许取消: 当前状态={:?}",
                user_badge.status
            )));
        }

        // 3. 余额检查
        if user_badge.quantity < request.quantity {
            return Err(BadgeError::InsufficientBadges {
                required: request.quantity,
                available: user_badge.quantity,
            });
        }

        // 4.1 计算新余额
        let new_quantity = user_badge.quantity - request.quantity;

        // 4.2 更新用户徽章数量（使用负数增量）
        UserBadgeRepository::update_user_badge_quantity_in_tx(
            &mut tx,
            user_badge.id,
            -request.quantity,
        )
        .await?;

        // 4.3 如果数量归零，更新状态为 Revoked
        if new_quantity == 0 {
            UserBadgeRepository::update_user_badge_status_in_tx(
                &mut tx,
                user_badge.id,
                UserBadgeStatus::Revoked,
            )
            .await?;
        }

        // 4.4 写入账本流水（使用负数记录取消数量，表示减少）
        let ledger = BadgeLedger {
            id: 0,
            user_id: request.user_id.clone(),
            badge_id: request.badge_id,
            change_type: ChangeType::Cancel,
            quantity: -request.quantity, // 负数表示减少
            balance_after: new_quantity,
            ref_id: request.source_ref_id.clone(),
            ref_type: request.source_type,
            remark: Some(request.reason.clone()),
            created_at: Utc::now(),
        };
        BadgeLedgerRepository::create_in_tx(&mut tx, &ledger).await?;

        // 4.5 写入用户徽章日志
        sqlx::query(
            r#"
            INSERT INTO user_badge_logs
                (user_badge_id, user_id, badge_id, action, reason, operator, quantity, source_type, source_ref_id, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            "#,
        )
        .bind(user_badge.id)
        .bind(&request.user_id)
        .bind(request.badge_id)
        .bind(LogAction::Revoke)
        .bind(&request.reason)
        .bind(&request.operator)
        .bind(request.quantity)
        .bind(request.source_type)
        .bind(&request.source_ref_id)
        .execute(&mut *tx)
        .await?;

        // 5. 提交事务
        tx.commit().await?;

        Ok(new_quantity)
    }

    /// 使用户徽章相关缓存失效
    async fn invalidate_user_cache(&self, user_id: &str) {
        let keys = [
            cache_keys::user_badges(user_id),
            cache_keys::badge_wall(user_id),
        ];

        for key in keys {
            if let Err(e) = self.cache.delete(&key).await {
                warn!(key = %key, error = %e, "缓存失效失败");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SourceType;

    // ==================== 退款事件测试 ====================

    #[test]
    fn test_refund_event_full_refund() {
        let event = RefundEvent {
            event_id: "evt-001".to_string(),
            user_id: "user-123".to_string(),
            original_order_id: "order-001".to_string(),
            refund_order_id: "refund-001".to_string(),
            original_amount: 50000, // 500 元
            refund_amount: 50000,   // 全额退款
            remaining_amount: 0,
            reason: Some("商品不满意".to_string()),
            badge_ids_to_revoke: None,
            timestamp: Utc::now(),
        };

        assert!(event.is_full_refund());
        assert_eq!(event.effective_amount(), 0);
    }

    #[test]
    fn test_refund_event_partial_refund() {
        let event = RefundEvent {
            event_id: "evt-002".to_string(),
            user_id: "user-123".to_string(),
            original_order_id: "order-002".to_string(),
            refund_order_id: "refund-002".to_string(),
            original_amount: 80000, // 800 元
            refund_amount: 20000,   // 退款 200 元
            remaining_amount: 60000, // 剩余 600 元
            reason: Some("部分商品退货".to_string()),
            badge_ids_to_revoke: None,
            timestamp: Utc::now(),
        };

        assert!(!event.is_full_refund());
        assert_eq!(event.effective_amount(), 60000);
    }

    #[test]
    fn test_refund_process_result_success() {
        let result = RefundProcessResult::success(
            "evt-001".to_string(),
            vec![RevokedBadgeInfo {
                badge_id: 1,
                badge_name: "500元徽章".to_string(),
                quantity: 1,
                reason: "全额退款撤销".to_string(),
            }],
            vec![],
            100,
        );

        assert!(result.success);
        assert_eq!(result.revoked_badges.len(), 1);
        assert_eq!(result.retained_badges.len(), 0);
        assert_eq!(result.processing_time_ms, 100);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_refund_process_result_with_retained() {
        let result = RefundProcessResult::success(
            "evt-002".to_string(),
            vec![],
            vec![RetainedBadgeInfo {
                badge_id: 1,
                badge_name: "500元徽章".to_string(),
                reason: "部分退款后金额 60000 仍满足阈值 50000".to_string(),
            }],
            50,
        );

        assert!(result.success);
        assert_eq!(result.revoked_badges.len(), 0);
        assert_eq!(result.retained_badges.len(), 1);
    }

    #[test]
    fn test_refund_process_result_failure() {
        let result = RefundProcessResult::failure("evt-003".to_string(), "处理超时");

        assert!(!result.success);
        assert_eq!(result.error, Some("处理超时".to_string()));
        assert!(result.revoked_badges.is_empty());
        assert!(result.retained_badges.is_empty());
    }

    #[test]
    fn test_badge_grant_condition() {
        let condition = BadgeGrantCondition {
            rule_id: 1,
            badge_id: 100,
            badge_name: "消费满500徽章".to_string(),
            amount_threshold: Some(50000), // 500 元阈值
            event_type: "purchase".to_string(),
        };

        assert_eq!(condition.amount_threshold, Some(50000));
    }

    // ==================== 撤销请求测试 ====================

    #[test]
    fn test_revoke_request_manual() {
        let request = RevokeBadgeRequest::manual("user-123", 1, 5, "违规行为", "admin");
        assert_eq!(request.user_id, "user-123");
        assert_eq!(request.badge_id, 1);
        assert_eq!(request.quantity, 5);
        assert_eq!(request.reason, "违规行为");
        assert_eq!(request.operator, Some("admin".to_string()));
        assert_eq!(request.source_type, SourceType::Manual);
    }

    #[test]
    fn test_revoke_request_system() {
        let request = RevokeBadgeRequest::system("user-123", 1, 3, "系统自动回收")
            .with_source_ref("task-001");
        assert_eq!(request.source_type, SourceType::System);
        assert_eq!(request.source_ref_id, Some("task-001".to_string()));
        assert!(request.operator.is_none());
    }

    #[test]
    fn test_revoke_response_success() {
        let response = RevokeBadgeResponse::success(5);
        assert!(response.success);
        assert_eq!(response.remaining_quantity, 5);
    }

    #[test]
    fn test_revoke_response_failure() {
        let response = RevokeBadgeResponse::failure("余额不足");
        assert!(!response.success);
        assert_eq!(response.message, "余额不足");
    }

    #[test]
    fn test_revoke_result_success() {
        let result = RevokeResult::success("user-1".to_string(), 1, 3);
        assert!(result.success);
        assert_eq!(result.remaining_quantity, Some(3));
        assert!(result.error.is_none());
    }

    #[test]
    fn test_revoke_result_failure() {
        let result = RevokeResult::failure("user-1".to_string(), 1, "用户未持有该徽章");
        assert!(!result.success);
        assert!(result.remaining_quantity.is_none());
        assert_eq!(result.error, Some("用户未持有该徽章".to_string()));
    }

    #[test]
    fn test_batch_revoke_response_serialization() {
        let response = BatchRevokeResponse {
            total: 3,
            success_count: 2,
            failed_count: 1,
            results: vec![
                RevokeResult::success("user-1".to_string(), 1, 5),
                RevokeResult::success("user-2".to_string(), 1, 0),
                RevokeResult::failure("user-3".to_string(), 1, "余额不足"),
            ],
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["total"], 3);
        assert_eq!(json["successCount"], 2);
        assert_eq!(json["failedCount"], 1);
        assert!(json["results"].is_array());
        assert_eq!(json["results"][0]["success"], true);
        assert_eq!(json["results"][2]["success"], false);
    }
}
