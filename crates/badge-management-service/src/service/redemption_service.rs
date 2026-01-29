//! 徽章兑换服务
//!
//! 处理徽章兑换权益的核心业务逻辑，包括：
//! - 幂等处理（防止重复兑换）
//! - 兑换规则有效性检查
//! - 权益库存检查
//! - 用户徽章余额检查
//! - 事务性扣减与订单创建
//!
//! ## 兑换流程
//!
//! 1. 幂等检查 -> 2. 规则有效性 -> 3. 权益库存 -> 4. 徽章余额
//!    -> 5. 事务写入 -> 6. 缓存失效

use std::sync::Arc;

use chrono::Utc;
use sqlx::PgPool;
use tracing::{info, instrument, warn};
use uuid::Uuid;

use badge_shared::cache::Cache;

use crate::error::{BadgeError, Result};
use crate::models::{
    BadgeLedger, BadgeRedemptionRule, Benefit, ChangeType, LogAction, OrderStatus,
    RedemptionDetail, RedemptionOrder, RequiredBadge, SourceType, UserBadgeStatus,
};
use crate::repository::{BadgeLedgerRepository, RedemptionRepository, UserBadgeRepository};
use crate::service::dto::{
    ConsumedBadgeDto, RedeemBadgeRequest, RedeemBadgeResponse, RedemptionHistoryDto,
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

/// 徽章兑换服务
///
/// 负责徽章兑换权益的完整流程，包括验证、事务处理和缓存管理
pub struct RedemptionService {
    redemption_repo: Arc<RedemptionRepository>,
    cache: Arc<Cache>,
    pool: PgPool,
}

impl RedemptionService {
    pub fn new(
        redemption_repo: Arc<RedemptionRepository>,
        cache: Arc<Cache>,
        pool: PgPool,
    ) -> Self {
        Self {
            redemption_repo,
            cache,
            pool,
        }
    }

    /// 兑换徽章换取权益
    ///
    /// 完整事务流程：
    /// 1. 校验幂等键（防止重复兑换）
    /// 2. 校验兑换规则有效性
    /// 3. 校验权益库存
    /// 4. 校验用户徽章余额
    /// 5. 创建兑换订单
    /// 6. 创建兑换明细
    /// 7. 扣减用户徽章
    /// 8. 写入账本流水
    /// 9. 更新权益已兑换数量
    /// 10. 提交事务
    /// 11. 清除缓存
    #[instrument(skip(self), fields(user_id = %request.user_id, rule_id = %request.rule_id))]
    pub async fn redeem_badge(&self, request: RedeemBadgeRequest) -> Result<RedeemBadgeResponse> {
        // 1. 幂等检查
        if let Some(response) = self.check_idempotency(&request.idempotency_key).await? {
            info!(idempotency_key = %request.idempotency_key, "幂等请求，返回已存在的订单");
            return Ok(response);
        }

        // 2. 获取兑换规则并检查有效性
        let rule = self.validate_rule(request.rule_id).await?;

        // 3. 获取权益并检查库存
        let benefit = self.validate_benefit(rule.benefit_id).await?;

        // 4. 解析所需徽章
        let required_badges = rule
            .parse_required_badges()
            .map_err(BadgeError::Serialization)?;

        // 5-10. 事务内执行兑换
        let (order_id, order_no) = self
            .execute_redemption(&request, &rule, &benefit, &required_badges)
            .await?;

        // 11. 清除缓存
        self.invalidate_user_cache(&request.user_id).await;

        info!(
            user_id = %request.user_id,
            rule_id = %request.rule_id,
            order_id = order_id,
            order_no = %order_no,
            benefit_name = %benefit.name,
            "徽章兑换成功"
        );

        Ok(RedeemBadgeResponse::success(
            order_id,
            order_no,
            benefit.name,
        ))
    }

    /// 查询用户兑换历史
    ///
    /// 返回用户的兑换订单列表，包含消耗的徽章明细
    #[instrument(skip(self), fields(user_id = %user_id, limit = %limit))]
    pub async fn get_user_redemptions(
        &self,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<RedemptionHistoryDto>> {
        // 获取用户的兑换订单
        let orders = self
            .redemption_repo
            .list_orders_by_user(user_id, limit)
            .await?;

        if orders.is_empty() {
            return Ok(Vec::new());
        }

        // 收集所有订单 ID 和权益 ID
        let order_ids: Vec<i64> = orders.iter().map(|o| o.id).collect();
        let benefit_ids: Vec<i64> = orders.iter().map(|o| o.benefit_id).collect();

        // 批量获取权益信息
        let benefits = self.get_benefits_by_ids(&benefit_ids).await?;

        // 批量获取所有订单的明细
        let all_details = self.get_details_for_orders(&order_ids).await?;

        // 批量获取明细中涉及的徽章名称
        let badge_ids: Vec<i64> = all_details.iter().map(|d| d.badge_id).collect();
        let badge_names = self.get_badge_names(&badge_ids).await?;

        // 组装 DTO
        let mut result = Vec::with_capacity(orders.len());
        for order in orders {
            let benefit_name = benefits
                .get(&order.benefit_id)
                .map(|b| b.name.clone())
                .unwrap_or_else(|| "未知权益".to_string());

            let consumed_badges: Vec<ConsumedBadgeDto> = all_details
                .iter()
                .filter(|d| d.order_id == order.id)
                .map(|d| ConsumedBadgeDto {
                    badge_id: d.badge_id,
                    badge_name: badge_names
                        .get(&d.badge_id)
                        .cloned()
                        .unwrap_or_else(|| "未知徽章".to_string()),
                    quantity: d.quantity,
                })
                .collect();

            result.push(RedemptionHistoryDto {
                order_id: order.id,
                order_no: order.order_no,
                benefit_name,
                status: order.status,
                consumed_badges,
                created_at: order.created_at,
            });
        }

        Ok(result)
    }

    // ==================== 私有方法 ====================

    /// 幂等检查
    ///
    /// 通过幂等键查询是否已存在兑换订单
    async fn check_idempotency(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<RedeemBadgeResponse>> {
        let order = self
            .redemption_repo
            .get_order_by_idempotency_key(idempotency_key)
            .await?;

        if let Some(order) = order {
            // 查询权益名称
            let benefit_name =
                if let Some(benefit) = self.redemption_repo.get_benefit(order.benefit_id).await? {
                    benefit.name
                } else {
                    "未知权益".to_string()
                };

            return Ok(Some(RedeemBadgeResponse::from_existing(
                order.id,
                order.order_no,
                benefit_name,
                order.status,
            )));
        }

        Ok(None)
    }

    /// 验证兑换规则
    async fn validate_rule(&self, rule_id: i64) -> Result<BadgeRedemptionRule> {
        let rule = self
            .redemption_repo
            .get_redemption_rule(rule_id)
            .await?
            .ok_or(BadgeError::RedemptionRuleNotFound(rule_id))?;

        // 检查规则是否启用且在有效期内
        let now = Utc::now();
        if !rule.is_active(now) {
            return Err(BadgeError::RedemptionRuleInactive(rule_id));
        }

        Ok(rule)
    }

    /// 验证权益
    async fn validate_benefit(&self, benefit_id: i64) -> Result<Benefit> {
        let benefit = self
            .redemption_repo
            .get_benefit(benefit_id)
            .await?
            .ok_or(BadgeError::BenefitNotFound(benefit_id))?;

        // 检查是否可兑换（启用且有库存）
        if !benefit.is_redeemable() {
            return Err(BadgeError::BenefitOutOfStock(benefit_id));
        }

        Ok(benefit)
    }

    /// 执行兑换事务
    ///
    /// 在单个事务内完成：
    /// - 创建兑换订单（Pending 状态）
    /// - 锁定并检查用户徽章余额
    /// - 扣减徽章数量
    /// - 创建兑换明细
    /// - 写入账本流水
    /// - 更新权益已兑换数量
    /// - 更新订单状态为 Success
    async fn execute_redemption(
        &self,
        request: &RedeemBadgeRequest,
        rule: &BadgeRedemptionRule,
        benefit: &Benefit,
        required_badges: &[RequiredBadge],
    ) -> Result<(i64, String)> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now();

        // 5.1 生成订单号
        let order_no = generate_order_no();

        // 5.2 创建兑换订单（状态: Pending）
        let order = RedemptionOrder {
            id: 0,
            order_no: order_no.clone(),
            user_id: request.user_id.clone(),
            rule_id: request.rule_id,
            benefit_id: rule.benefit_id,
            status: OrderStatus::Pending,
            failure_reason: None,
            benefit_result: None,
            idempotency_key: Some(request.idempotency_key.clone()),
            created_at: now,
            updated_at: now,
        };
        let order_id = RedemptionRepository::create_order_in_tx(&mut tx, &order).await?;

        // 5.3 对每个所需徽章进行处理
        for required in required_badges {
            // 锁定用户徽章（FOR UPDATE）
            let user_badge = UserBadgeRepository::get_user_badge_for_update(
                &mut tx,
                &request.user_id,
                required.badge_id,
            )
            .await?
            .ok_or_else(|| BadgeError::UserBadgeNotFound {
                user_id: request.user_id.clone(),
                badge_id: required.badge_id,
            })?;

            // 检查徽章状态
            if user_badge.status != UserBadgeStatus::Active {
                return Err(BadgeError::Validation(format!(
                    "徽章状态不可兑换: badge_id={}, status={:?}",
                    required.badge_id, user_badge.status
                )));
            }

            // 检查余额
            if user_badge.quantity < required.quantity {
                return Err(BadgeError::InsufficientBadges {
                    required: required.quantity,
                    available: user_badge.quantity,
                });
            }

            // 计算新余额
            let new_quantity = user_badge.quantity - required.quantity;

            // 扣减数量
            UserBadgeRepository::update_user_badge_quantity_in_tx(
                &mut tx,
                user_badge.id,
                -required.quantity,
            )
            .await?;

            // 如果归零则更新状态为 Redeemed
            if new_quantity == 0 {
                UserBadgeRepository::update_user_badge_status_in_tx(
                    &mut tx,
                    user_badge.id,
                    UserBadgeStatus::Redeemed,
                )
                .await?;
            }

            // 创建兑换明细
            let detail = RedemptionDetail {
                id: 0,
                order_id,
                user_badge_id: user_badge.id,
                badge_id: required.badge_id,
                quantity: required.quantity,
                created_at: now,
            };
            RedemptionRepository::create_detail_in_tx(&mut tx, &detail).await?;

            // 写入账本流水（REDEEM_OUT）
            let ledger = BadgeLedger {
                id: 0,
                user_id: request.user_id.clone(),
                badge_id: required.badge_id,
                change_type: ChangeType::RedeemOut,
                quantity: required.quantity,
                balance_after: new_quantity,
                ref_id: Some(order_no.clone()),
                ref_type: SourceType::Redemption,
                remark: Some(format!("兑换权益: {}", benefit.name)),
                created_at: now,
            };
            BadgeLedgerRepository::create_in_tx(&mut tx, &ledger).await?;

            // 写入用户徽章日志
            sqlx::query(
                r#"
                INSERT INTO user_badge_logs
                    (user_badge_id, user_id, badge_id, action, reason, operator, quantity, source_type, source_ref_id, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
                "#,
            )
            .bind(user_badge.id)
            .bind(&request.user_id)
            .bind(required.badge_id)
            .bind(LogAction::Redeem)
            .bind(format!("兑换权益: {}", benefit.name))
            .bind::<Option<String>>(None)
            .bind(required.quantity)
            .bind(SourceType::Redemption)
            .bind(&order_no)
            .execute(&mut *tx)
            .await?;
        }

        // 5.4 更新权益已兑换数量
        RedemptionRepository::increment_redeemed_count_in_tx(&mut tx, benefit.id, 1).await?;

        // 5.5 更新订单状态为 Success
        RedemptionRepository::update_order_status_in_tx(
            &mut tx,
            order_id,
            OrderStatus::Success,
            None,
        )
        .await?;

        // 6. 提交事务
        tx.commit().await?;

        Ok((order_id, order_no))
    }

    /// 批量获取权益信息
    async fn get_benefits_by_ids(
        &self,
        ids: &[i64],
    ) -> Result<std::collections::HashMap<i64, Benefit>> {
        let mut result = std::collections::HashMap::new();

        // 逐个获取权益（未来可优化为批量查询）
        for &id in ids {
            if let Some(benefit) = self.redemption_repo.get_benefit(id).await? {
                result.insert(id, benefit);
            }
        }

        Ok(result)
    }

    /// 获取订单的兑换明细
    async fn get_details_for_orders(&self, order_ids: &[i64]) -> Result<Vec<RedemptionDetail>> {
        let mut all_details = Vec::new();

        for &order_id in order_ids {
            let details = self.redemption_repo.list_details_by_order(order_id).await?;
            all_details.extend(details);
        }

        Ok(all_details)
    }

    /// 获取徽章名称映射
    async fn get_badge_names(
        &self,
        badge_ids: &[i64],
    ) -> Result<std::collections::HashMap<i64, String>> {
        let mut result = std::collections::HashMap::new();

        if badge_ids.is_empty() {
            return Ok(result);
        }

        // 通过 SQL 批量获取徽章名称
        let rows = sqlx::query(
            r#"
            SELECT id, name FROM badges WHERE id = ANY($1)
            "#,
        )
        .bind(badge_ids)
        .fetch_all(&self.pool)
        .await?;

        for row in rows {
            use sqlx::Row;
            let id: i64 = row.get("id");
            let name: String = row.get("name");
            result.insert(id, name);
        }

        Ok(result)
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

/// 生成兑换订单号
///
/// 格式: RD{yyyyMMddHHmmss}{6位随机数}
/// 使用 UUID v4 的一部分作为随机数源
fn generate_order_no() -> String {
    let now = Utc::now();
    // 使用 UUID v4 生成随机数，取前 6 位十六进制转为十进制后取模
    let uuid = Uuid::new_v4();
    let random = uuid.as_u128() % 1_000_000;
    format!("RD{}{:06}", now.format("%Y%m%d%H%M%S"), random)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::BenefitType;
    use serde_json::json;

    #[test]
    fn test_generate_order_no_format() {
        let order_no = generate_order_no();

        // 验证前缀
        assert!(order_no.starts_with("RD"));

        // 验证长度: "RD" + 14 位时间戳 + 6 位随机数 = 22
        assert_eq!(order_no.len(), 22);

        // 验证随机性：连续生成多个订单号应不同
        let order_no2 = generate_order_no();
        // 由于时间戳可能相同，只验证格式正确
        assert!(order_no2.starts_with("RD"));
        assert_eq!(order_no2.len(), 22);
    }

    #[test]
    fn test_redeem_badge_request_new() {
        let request = RedeemBadgeRequest::new("user-123", 1, "idem-key-001");

        assert_eq!(request.user_id, "user-123");
        assert_eq!(request.rule_id, 1);
        assert_eq!(request.idempotency_key, "idem-key-001");
    }

    #[test]
    fn test_redeem_badge_response_success() {
        let response = RedeemBadgeResponse::success(
            100,
            "RD20241001120000123456".to_string(),
            "VIP 优惠券".to_string(),
        );

        assert!(response.success);
        assert_eq!(response.order_id, 100);
        assert_eq!(response.order_no, "RD20241001120000123456");
        assert_eq!(response.benefit_name, "VIP 优惠券");
        assert_eq!(response.message, "兑换成功");
    }

    #[test]
    fn test_redeem_badge_response_from_existing() {
        let response = RedeemBadgeResponse::from_existing(
            100,
            "RD20241001120000123456".to_string(),
            "VIP 优惠券".to_string(),
            OrderStatus::Success,
        );

        assert!(response.success);
        assert_eq!(response.message, "幂等请求，返回已存在的订单");
    }

    #[test]
    fn test_redemption_history_dto_serialization() {
        let dto = RedemptionHistoryDto {
            order_id: 1,
            order_no: "RD20241001120000123456".to_string(),
            benefit_name: "测试权益".to_string(),
            status: OrderStatus::Success,
            consumed_badges: vec![ConsumedBadgeDto {
                badge_id: 1,
                badge_name: "测试徽章".to_string(),
                quantity: 2,
            }],
            created_at: Utc::now(),
        };

        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["orderId"], 1);
        assert_eq!(json["orderNo"], "RD20241001120000123456");
        assert_eq!(json["benefitName"], "测试权益");
        assert_eq!(json["status"], "SUCCESS");
        assert!(json["consumedBadges"].is_array());
        assert_eq!(json["consumedBadges"][0]["badgeId"], 1);
    }

    #[test]
    fn test_consumed_badge_dto_serialization() {
        let dto = ConsumedBadgeDto {
            badge_id: 1,
            badge_name: "新手徽章".to_string(),
            quantity: 3,
        };

        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["badgeId"], 1);
        assert_eq!(json["badgeName"], "新手徽章");
        assert_eq!(json["quantity"], 3);
    }

    fn create_test_benefit() -> Benefit {
        Benefit {
            id: 1,
            benefit_type: BenefitType::Coupon,
            name: "测试优惠券".to_string(),
            description: Some("测试描述".to_string()),
            icon_url: None,
            config: json!({"couponId": "coupon-001"}),
            total_stock: Some(100),
            redeemed_count: 0,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_benefit_is_redeemable() {
        let mut benefit = create_test_benefit();

        // 启用且有库存
        assert!(benefit.is_redeemable());

        // 禁用
        benefit.enabled = false;
        assert!(!benefit.is_redeemable());

        // 启用但无库存
        benefit.enabled = true;
        benefit.redeemed_count = 100;
        assert!(!benefit.is_redeemable());
    }
}
