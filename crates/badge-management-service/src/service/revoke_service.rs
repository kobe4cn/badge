//! 徽章取消服务
//!
//! 处理徽章取消/撤销的核心业务逻辑，包括：
//! - 用户徽章存在性检查
//! - 余额充足性检查
//! - 事务性扣减（用户徽章、账本流水）
//! - 状态变更（数量归零时标记为 Revoked）
//!
//! ## 取消流程
//!
//! 1. 参数校验 -> 2. 查询用户徽章 -> 3. 余额检查 -> 4. 事务内扣减 -> 5. 缓存失效

use std::sync::Arc;

use chrono::Utc;
use sqlx::PgPool;
use tracing::{info, instrument, warn};

use badge_shared::cache::Cache;

use crate::error::{BadgeError, Result};
use crate::models::{BadgeLedger, ChangeType, LogAction, UserBadgeStatus};
use crate::repository::{BadgeLedgerRepository, UserBadgeRepository};
use crate::service::dto::{
    BatchRevokeResponse, RevokeBadgeRequest, RevokeBadgeResponse, RevokeResult,
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
/// 负责徽章取消的完整流程，包括验证、事务处理和缓存管理
pub struct RevokeService {
    cache: Arc<Cache>,
    pool: PgPool,
}

impl RevokeService {
    pub fn new(cache: Arc<Cache>, pool: PgPool) -> Self {
        Self { cache, pool }
    }

    /// 取消/撤销徽章
    ///
    /// 完整的取消流程：
    /// 1. 参数校验（quantity > 0, reason 非空）
    /// 2. 查询用户徽章记录
    /// 3. 检查徽章状态和余额
    /// 4. 事务内执行扣减
    /// 5. 清除缓存
    #[instrument(skip(self), fields(user_id = %request.user_id, badge_id = %request.badge_id, quantity = %request.quantity))]
    pub async fn revoke_badge(&self, request: RevokeBadgeRequest) -> Result<RevokeBadgeResponse> {
        // 1. 参数校验
        self.validate_request(&request)?;

        // 2-4. 事务内执行取消操作
        let remaining_quantity = self.execute_revoke(&request).await?;

        // 5. 清除缓存
        self.invalidate_user_cache(&request.user_id).await;

        info!(
            user_id = %request.user_id,
            badge_id = %request.badge_id,
            quantity = request.quantity,
            remaining = remaining_quantity,
            "徽章取消成功"
        );

        Ok(RevokeBadgeResponse::success(remaining_quantity))
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

        // 4.4 写入账本流水（使用正数记录取消数量，通过 change_type 区分方向）
        let ledger = BadgeLedger {
            id: 0,
            user_id: request.user_id.clone(),
            badge_id: request.badge_id,
            change_type: ChangeType::Cancel,
            quantity: request.quantity,
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
        let request =
            RevokeBadgeRequest::system("user-123", 1, 3, "系统自动回收").with_source_ref("task-001");
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
