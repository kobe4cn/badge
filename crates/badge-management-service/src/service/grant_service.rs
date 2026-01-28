//! 徽章发放服务
//!
//! 处理徽章发放的核心业务逻辑，包括：
//! - 徽章有效性检查
//! - 库存检查与扣减
//! - 用户获取上限检查
//! - 事务性写入（用户徽章、账本流水）
//! - 幂等处理
//!
//! ## 发放流程
//!
//! 1. 幂等检查 -> 2. 徽章有效性 -> 3. 库存检查 -> 4. 用户限制检查
//!    -> 5. 事务写入 -> 6. 缓存失效

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use sqlx::{PgPool, Row};
use tracing::{info, instrument, warn};

use badge_shared::cache::Cache;

use crate::error::{BadgeError, Result};
use crate::models::{
    BadgeLedger, BadgeStatus, ChangeType, LogAction, UserBadge, UserBadgeStatus, ValidityConfig,
    ValidityType,
};
use crate::repository::{BadgeLedgerRepository, BadgeRepositoryTrait, UserBadgeRepository};
use crate::service::dto::{
    BatchGrantResponse, GrantBadgeRequest, GrantBadgeResponse, GrantResult,
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

/// 徽章发放服务
///
/// 负责徽章发放的完整流程，包括验证、事务处理和缓存管理
pub struct GrantService<BR>
where
    BR: BadgeRepositoryTrait,
{
    badge_repo: Arc<BR>,
    cache: Arc<Cache>,
    pool: PgPool,
}

impl<BR> GrantService<BR>
where
    BR: BadgeRepositoryTrait,
{
    pub fn new(badge_repo: Arc<BR>, cache: Arc<Cache>, pool: PgPool) -> Self {
        Self {
            badge_repo,
            cache,
            pool,
        }
    }

    /// 发放徽章给用户
    ///
    /// 完整的发放流程：
    /// 1. 幂等检查（如果有 idempotency_key）
    /// 2. 徽章有效性检查
    /// 3. 库存检查
    /// 4. 用户限制检查
    /// 5. 事务内写入
    /// 6. 清除缓存
    #[instrument(skip(self), fields(user_id = %request.user_id, badge_id = %request.badge_id))]
    pub async fn grant_badge(&self, request: GrantBadgeRequest) -> Result<GrantBadgeResponse> {
        // 参数校验
        if request.quantity <= 0 {
            return Err(BadgeError::Validation("发放数量必须大于0".to_string()));
        }

        // 1. 幂等检查
        if let Some(ref key) = request.idempotency_key
            && let Some(response) = self.check_idempotency(key).await?
        {
            info!(idempotency_key = %key, "幂等请求，返回已存在的记录");
            return Ok(response);
        }

        // 2. 徽章有效性检查
        let badge = self.validate_badge(request.badge_id).await?;

        // 3. 库存检查
        self.check_stock(&badge, request.quantity).await?;

        // 4. 用户限制检查
        self.check_user_limit(&request.user_id, request.badge_id, request.quantity)
            .await?;

        // 5. 事务内执行发放
        let (user_badge_id, new_quantity) = self.execute_grant(&request, &badge).await?;

        // 6. 清除缓存
        self.invalidate_user_cache(&request.user_id).await;

        info!(
            user_id = %request.user_id,
            badge_id = %request.badge_id,
            quantity = request.quantity,
            user_badge_id = user_badge_id,
            new_quantity = new_quantity,
            "徽章发放成功"
        );

        Ok(GrantBadgeResponse::success(user_badge_id, new_quantity))
    }

    /// 批量发放徽章
    ///
    /// 对每个请求独立处理，单个失败不影响其他请求
    #[instrument(skip(self), fields(request_count = requests.len()))]
    pub async fn batch_grant_badges(
        &self,
        requests: Vec<GrantBadgeRequest>,
    ) -> Result<BatchGrantResponse> {
        let total = requests.len() as i32;
        let mut results = Vec::with_capacity(requests.len());
        let mut success_count = 0;
        let mut failed_count = 0;

        for request in requests {
            let user_id = request.user_id.clone();
            let badge_id = request.badge_id;

            match self.grant_badge(request).await {
                Ok(response) => {
                    success_count += 1;
                    results.push(GrantResult::success(
                        user_id,
                        badge_id,
                        response.user_badge_id,
                        response.new_quantity,
                    ));
                }
                Err(e) => {
                    failed_count += 1;
                    warn!(user_id = %user_id, badge_id = %badge_id, error = %e, "批量发放单条失败");
                    results.push(GrantResult::failure(user_id, badge_id, e.to_string()));
                }
            }
        }

        info!(
            total = total,
            success = success_count,
            failed = failed_count,
            "批量发放完成"
        );

        Ok(BatchGrantResponse {
            total,
            success_count,
            failed_count,
            results,
        })
    }

    // ==================== 私有方法 ====================

    /// 幂等检查
    ///
    /// 查询是否已存在相同幂等键的发放记录
    async fn check_idempotency(&self, key: &str) -> Result<Option<GrantBadgeResponse>> {
        // 通过账本记录查询幂等键（ref_id 字段存储幂等键）
        let row = sqlx::query(
            r#"
            SELECT l.user_id, l.badge_id, l.balance_after,
                   ub.id as user_badge_id, ub.quantity
            FROM badge_ledgers l
            JOIN user_badges ub ON l.user_id = ub.user_id AND l.badge_id = ub.badge_id
            WHERE l.ref_id = $1 AND l.change_type = 'acquire'
            LIMIT 1
            "#,
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let user_badge_id: i64 = row.get("user_badge_id");
            let quantity: i32 = row.get("quantity");
            return Ok(Some(GrantBadgeResponse::from_existing(
                user_badge_id,
                quantity,
            )));
        }

        Ok(None)
    }

    /// 验证徽章有效性
    async fn validate_badge(
        &self,
        badge_id: i64,
    ) -> Result<crate::models::Badge> {
        let badge = self
            .badge_repo
            .get_badge(badge_id)
            .await?
            .ok_or(BadgeError::BadgeNotFound(badge_id))?;

        // 检查徽章状态
        if badge.status != BadgeStatus::Active {
            return Err(BadgeError::BadgeInactive(badge_id));
        }

        Ok(badge)
    }

    /// 检查库存是否充足
    async fn check_stock(
        &self,
        badge: &crate::models::Badge,
        quantity: i32,
    ) -> Result<()> {
        if let Some(max_supply) = badge.max_supply {
            let remaining = max_supply - badge.issued_count;
            if remaining < quantity as i64 {
                return Err(BadgeError::BadgeOutOfStock(badge.id));
            }
        }
        Ok(())
    }

    /// 检查用户获取限制
    async fn check_user_limit(
        &self,
        user_id: &str,
        badge_id: i64,
        quantity: i32,
    ) -> Result<()> {
        // 获取徽章规则
        let rules = self.badge_repo.get_badge_rules(badge_id).await?;
        let now = Utc::now();

        // 查找生效的规则中最严格的限制
        let max_limit = rules
            .iter()
            .filter(|r| r.is_active(now))
            .filter_map(|r| r.max_count_per_user)
            .min();

        if let Some(limit) = max_limit {
            // 查询用户当前持有数量
            let current = self.get_user_badge_quantity(user_id, badge_id).await?;
            if current + quantity > limit {
                return Err(BadgeError::BadgeAcquisitionLimitReached {
                    badge_id,
                    limit,
                });
            }
        }

        Ok(())
    }

    /// 获取用户某徽章的当前持有数量
    async fn get_user_badge_quantity(&self, user_id: &str, badge_id: i64) -> Result<i32> {
        let row = sqlx::query(
            r#"
            SELECT COALESCE(quantity, 0) as quantity
            FROM user_badges
            WHERE user_id = $1 AND badge_id = $2
            "#,
        )
        .bind(user_id)
        .bind(badge_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.get("quantity")).unwrap_or(0))
    }

    /// 执行发放事务
    ///
    /// 在单个事务内完成：
    /// - 创建/更新用户徽章记录
    /// - 写入账本流水
    /// - 更新徽章已发放数量
    /// - 写入操作日志
    async fn execute_grant(
        &self,
        request: &GrantBadgeRequest,
        badge: &crate::models::Badge,
    ) -> Result<(i64, i32)> {
        let mut tx = self.pool.begin().await?;

        // 5.1 查询/创建用户徽章记录（带锁）
        let existing = UserBadgeRepository::get_user_badge_for_update(
            &mut tx,
            &request.user_id,
            request.badge_id,
        )
        .await?;

        let (user_badge_id, new_quantity) = if let Some(ub) = existing {
            // 更新现有记录
            let new_qty = ub.quantity + request.quantity;
            UserBadgeRepository::update_user_badge_quantity_in_tx(&mut tx, ub.id, request.quantity)
                .await?;
            (ub.id, new_qty)
        } else {
            // 5.2 计算过期时间
            let validity_config = badge.parse_validity_config().unwrap_or_default();
            let expires_at = calculate_expires_at(&validity_config);

            // 创建新记录
            let now = Utc::now();
            let new_badge = UserBadge {
                id: 0,
                user_id: request.user_id.clone(),
                badge_id: request.badge_id,
                status: UserBadgeStatus::Active,
                quantity: request.quantity,
                acquired_at: now,
                expires_at,
                created_at: now,
                updated_at: now,
            };

            let id = UserBadgeRepository::create_user_badge_in_tx(&mut tx, &new_badge).await?;
            (id, request.quantity)
        };

        // 5.3 写入账本流水
        let ledger = BadgeLedger {
            id: 0,
            user_id: request.user_id.clone(),
            badge_id: request.badge_id,
            change_type: ChangeType::Acquire,
            quantity: request.quantity,
            balance_after: new_quantity,
            ref_id: request.idempotency_key.clone().or(request.source_ref_id.clone()),
            ref_type: request.source_type,
            remark: request.reason.clone(),
            created_at: Utc::now(),
        };
        BadgeLedgerRepository::create_in_tx(&mut tx, &ledger).await?;

        // 5.4 更新徽章已发放数量
        sqlx::query(
            r#"
            UPDATE badges
            SET issued_count = issued_count + $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(request.badge_id)
        .bind(request.quantity as i64)
        .execute(&mut *tx)
        .await?;

        // 5.5 写入用户徽章日志
        sqlx::query(
            r#"
            INSERT INTO user_badge_logs
                (user_badge_id, user_id, badge_id, action, reason, operator, quantity, source_type, source_ref_id, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            "#,
        )
        .bind(user_badge_id)
        .bind(&request.user_id)
        .bind(request.badge_id)
        .bind(LogAction::Grant)
        .bind(&request.reason)
        .bind(&request.operator)
        .bind(request.quantity)
        .bind(request.source_type)
        .bind(&request.source_ref_id)
        .execute(&mut *tx)
        .await?;

        // 6. 提交事务
        tx.commit().await?;

        Ok((user_badge_id, new_quantity))
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

/// 计算徽章过期时间
///
/// 根据有效期配置计算具体的过期时间点
fn calculate_expires_at(config: &ValidityConfig) -> Option<DateTime<Utc>> {
    match config.validity_type {
        ValidityType::Permanent => None,
        ValidityType::FixedDate => config.fixed_date,
        ValidityType::RelativeDays => config
            .relative_days
            .map(|days| Utc::now() + Duration::days(days as i64)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Badge, BadgeRule, BadgeType, SourceType};
    use serde_json::json;

    fn create_test_badge(id: i64) -> Badge {
        Badge {
            id,
            series_id: 1,
            badge_type: BadgeType::Normal,
            name: format!("Badge {}", id),
            description: None,
            obtain_description: None,
            sort_order: 0,
            status: BadgeStatus::Active,
            assets: json!({"iconUrl": "https://example.com/icon.png"}),
            validity_config: json!({"validityType": "PERMANENT"}),
            max_supply: Some(1000),
            issued_count: 100,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_badge_rule(badge_id: i64) -> BadgeRule {
        BadgeRule {
            id: 1,
            badge_id,
            rule_json: json!({}),
            start_time: None,
            end_time: None,
            max_count_per_user: Some(10),
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_calculate_expires_at_permanent() {
        let config = ValidityConfig {
            validity_type: ValidityType::Permanent,
            fixed_date: None,
            relative_days: None,
        };
        assert!(calculate_expires_at(&config).is_none());
    }

    #[test]
    fn test_calculate_expires_at_fixed_date() {
        let fixed = Utc::now() + Duration::days(30);
        let config = ValidityConfig {
            validity_type: ValidityType::FixedDate,
            fixed_date: Some(fixed),
            relative_days: None,
        };
        assert_eq!(calculate_expires_at(&config), Some(fixed));
    }

    #[test]
    fn test_calculate_expires_at_relative_days() {
        let config = ValidityConfig {
            validity_type: ValidityType::RelativeDays,
            fixed_date: None,
            relative_days: Some(7),
        };
        let result = calculate_expires_at(&config);
        assert!(result.is_some());

        let expires = result.unwrap();
        let expected = Utc::now() + Duration::days(7);
        // 允许 1 秒误差
        assert!((expires - expected).num_seconds().abs() < 2);
    }

    #[test]
    fn test_grant_request_builder() {
        let request = GrantBadgeRequest::new("user-123", 1, 5)
            .with_idempotency_key("key-001")
            .with_source(SourceType::Event, Some("event-001".to_string()));

        assert_eq!(request.user_id, "user-123");
        assert_eq!(request.badge_id, 1);
        assert_eq!(request.quantity, 5);
        assert_eq!(request.idempotency_key, Some("key-001".to_string()));
        assert_eq!(request.source_type, SourceType::Event);
        assert_eq!(request.source_ref_id, Some("event-001".to_string()));
    }

    #[test]
    fn test_grant_result_success() {
        let result = GrantResult::success("user-1".to_string(), 1, 100, 5);
        assert!(result.success);
        assert_eq!(result.user_badge_id, Some(100));
        assert_eq!(result.new_quantity, Some(5));
        assert!(result.error.is_none());
    }

    #[test]
    fn test_grant_result_failure() {
        let result = GrantResult::failure("user-1".to_string(), 1, "库存不足");
        assert!(!result.success);
        assert!(result.user_badge_id.is_none());
        assert_eq!(result.error, Some("库存不足".to_string()));
    }

    #[test]
    fn test_badge_stock_check() {
        let mut badge = create_test_badge(1);

        // 有库存
        badge.max_supply = Some(1000);
        badge.issued_count = 500;
        assert!(badge.has_stock());

        // 无库存
        badge.issued_count = 1000;
        assert!(!badge.has_stock());

        // 无限量
        badge.max_supply = None;
        assert!(badge.has_stock());
    }

    #[test]
    fn test_badge_rule_is_active() {
        let now = Utc::now();
        let mut rule = create_test_badge_rule(1);

        // 启用且无时间限制
        assert!(rule.is_active(now));

        // 禁用
        rule.enabled = false;
        assert!(!rule.is_active(now));

        // 未到开始时间
        rule.enabled = true;
        rule.start_time = Some(now + Duration::hours(1));
        assert!(!rule.is_active(now));

        // 已过结束时间
        rule.start_time = None;
        rule.end_time = Some(now - Duration::hours(1));
        assert!(!rule.is_active(now));
    }
}
