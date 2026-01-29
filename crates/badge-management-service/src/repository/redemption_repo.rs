//! 兑换仓储
//!
//! 提供权益、兑换规则、兑换订单的数据访问

use async_trait::async_trait;
use sqlx::{PgConnection, PgPool, Row};

use super::traits::RedemptionRepositoryTrait;
use crate::error::Result;
use crate::models::{BadgeRedemptionRule, Benefit, OrderStatus, RedemptionDetail, RedemptionOrder};

/// 兑换仓储
///
/// 负责兑换相关实体的数据访问，包括权益定义、兑换规则、订单和明细
pub struct RedemptionRepository {
    pool: PgPool,
}

impl RedemptionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ==================== 权益 ====================

    /// 获取单个权益
    pub async fn get_benefit(&self, id: i64) -> Result<Option<Benefit>> {
        let benefit = sqlx::query_as::<_, Benefit>(
            r#"
            SELECT id, benefit_type, name, description, icon_url, config,
                   total_stock, redeemed_count, enabled, created_at, updated_at
            FROM benefits
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(benefit)
    }

    /// 列出所有启用的权益
    pub async fn list_benefits(&self) -> Result<Vec<Benefit>> {
        let benefits = sqlx::query_as::<_, Benefit>(
            r#"
            SELECT id, benefit_type, name, description, icon_url, config,
                   total_stock, redeemed_count, enabled, created_at, updated_at
            FROM benefits
            WHERE enabled = true
            ORDER BY id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(benefits)
    }

    /// 更新权益已兑换数量
    pub async fn increment_redeemed_count(&self, benefit_id: i64, delta: i64) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE benefits
            SET redeemed_count = redeemed_count + $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(benefit_id)
        .bind(delta)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 在事务中更新权益已兑换数量
    pub async fn increment_redeemed_count_in_tx(
        tx: &mut PgConnection,
        benefit_id: i64,
        delta: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE benefits
            SET redeemed_count = redeemed_count + $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(benefit_id)
        .bind(delta)
        .execute(tx)
        .await?;

        Ok(())
    }

    // ==================== 兑换规则 ====================

    /// 获取单个兑换规则
    pub async fn get_redemption_rule(&self, id: i64) -> Result<Option<BadgeRedemptionRule>> {
        let rule = sqlx::query_as::<_, BadgeRedemptionRule>(
            r#"
            SELECT id, name, description, benefit_id, required_badges,
                   frequency_config, start_time, end_time, enabled,
                   created_at, updated_at
            FROM badge_redemption_rules
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(rule)
    }

    /// 列出某徽章关联的兑换规则
    ///
    /// 通过 required_badges JSON 字段进行匹配
    pub async fn list_rules_by_badge(&self, badge_id: i64) -> Result<Vec<BadgeRedemptionRule>> {
        // 使用 PostgreSQL JSON 查询功能匹配包含指定徽章的规则
        let rules = sqlx::query_as::<_, BadgeRedemptionRule>(
            r#"
            SELECT id, name, description, benefit_id, required_badges,
                   frequency_config, start_time, end_time, enabled,
                   created_at, updated_at
            FROM badge_redemption_rules
            WHERE enabled = true
              AND EXISTS (
                  SELECT 1 FROM jsonb_array_elements(required_badges) AS elem
                  WHERE (elem->>'badgeId')::bigint = $1
              )
            ORDER BY id ASC
            "#,
        )
        .bind(badge_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rules)
    }

    /// 列出所有启用的兑换规则
    pub async fn list_active_rules(&self) -> Result<Vec<BadgeRedemptionRule>> {
        let rules = sqlx::query_as::<_, BadgeRedemptionRule>(
            r#"
            SELECT id, name, description, benefit_id, required_badges,
                   frequency_config, start_time, end_time, enabled,
                   created_at, updated_at
            FROM badge_redemption_rules
            WHERE enabled = true
            ORDER BY id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rules)
    }

    // ==================== 兑换订单 ====================

    /// 创建兑换订单
    ///
    /// 返回新订单的 ID
    pub async fn create_order(&self, order: &RedemptionOrder) -> Result<i64> {
        let row = sqlx::query(
            r#"
            INSERT INTO redemption_orders (order_no, user_id, rule_id, benefit_id, status,
                                          failure_reason, benefit_result, idempotency_key,
                                          created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id
            "#,
        )
        .bind(&order.order_no)
        .bind(&order.user_id)
        .bind(order.rule_id)
        .bind(order.benefit_id)
        .bind(order.status)
        .bind(&order.failure_reason)
        .bind(&order.benefit_result)
        .bind(&order.idempotency_key)
        .bind(order.created_at)
        .bind(order.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// 在事务中创建兑换订单
    pub async fn create_order_in_tx(tx: &mut PgConnection, order: &RedemptionOrder) -> Result<i64> {
        let row = sqlx::query(
            r#"
            INSERT INTO redemption_orders (order_no, user_id, rule_id, benefit_id, status,
                                          failure_reason, benefit_result, idempotency_key,
                                          created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id
            "#,
        )
        .bind(&order.order_no)
        .bind(&order.user_id)
        .bind(order.rule_id)
        .bind(order.benefit_id)
        .bind(order.status)
        .bind(&order.failure_reason)
        .bind(&order.benefit_result)
        .bind(&order.idempotency_key)
        .bind(order.created_at)
        .bind(order.updated_at)
        .fetch_one(tx)
        .await?;

        Ok(row.get("id"))
    }

    /// 获取订单
    pub async fn get_order(&self, id: i64) -> Result<Option<RedemptionOrder>> {
        let order = sqlx::query_as::<_, RedemptionOrder>(
            r#"
            SELECT id, order_no, user_id, rule_id, benefit_id, status,
                   failure_reason, benefit_result, idempotency_key,
                   created_at, updated_at
            FROM redemption_orders
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(order)
    }

    /// 根据订单号获取订单
    pub async fn get_order_by_no(&self, order_no: &str) -> Result<Option<RedemptionOrder>> {
        let order = sqlx::query_as::<_, RedemptionOrder>(
            r#"
            SELECT id, order_no, user_id, rule_id, benefit_id, status,
                   failure_reason, benefit_result, idempotency_key,
                   created_at, updated_at
            FROM redemption_orders
            WHERE order_no = $1
            "#,
        )
        .bind(order_no)
        .fetch_optional(&self.pool)
        .await?;

        Ok(order)
    }

    /// 根据幂等键获取订单
    ///
    /// 用于防止重复提交
    pub async fn get_order_by_idempotency_key(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<RedemptionOrder>> {
        let order = sqlx::query_as::<_, RedemptionOrder>(
            r#"
            SELECT id, order_no, user_id, rule_id, benefit_id, status,
                   failure_reason, benefit_result, idempotency_key,
                   created_at, updated_at
            FROM redemption_orders
            WHERE idempotency_key = $1
            "#,
        )
        .bind(idempotency_key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(order)
    }

    /// 更新订单状态
    pub async fn update_order_status(
        &self,
        id: i64,
        status: OrderStatus,
        failure_reason: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE redemption_orders
            SET status = $2, failure_reason = $3, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(failure_reason)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 在事务中更新订单状态
    pub async fn update_order_status_in_tx(
        tx: &mut PgConnection,
        id: i64,
        status: OrderStatus,
        failure_reason: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE redemption_orders
            SET status = $2, failure_reason = $3, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(failure_reason)
        .execute(tx)
        .await?;

        Ok(())
    }

    /// 更新订单权益结果
    pub async fn update_order_benefit_result(
        &self,
        id: i64,
        benefit_result: &serde_json::Value,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE redemption_orders
            SET benefit_result = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(benefit_result)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 列出用户的兑换订单
    pub async fn list_orders_by_user(
        &self,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<RedemptionOrder>> {
        let orders = sqlx::query_as::<_, RedemptionOrder>(
            r#"
            SELECT id, order_no, user_id, rule_id, benefit_id, status,
                   failure_reason, benefit_result, idempotency_key,
                   created_at, updated_at
            FROM redemption_orders
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(orders)
    }

    /// 统计用户对某规则的兑换次数
    ///
    /// 用于频率限制检查
    pub async fn count_user_redemptions(
        &self,
        user_id: &str,
        rule_id: i64,
        since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<i64> {
        let count: i64 = if let Some(since_time) = since {
            sqlx::query_scalar(
                r#"
                SELECT COUNT(*) as count
                FROM redemption_orders
                WHERE user_id = $1 AND rule_id = $2 AND status = $3 AND created_at >= $4
                "#,
            )
            .bind(user_id)
            .bind(rule_id)
            .bind(OrderStatus::Success)
            .bind(since_time)
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_scalar(
                r#"
                SELECT COUNT(*) as count
                FROM redemption_orders
                WHERE user_id = $1 AND rule_id = $2 AND status = $3
                "#,
            )
            .bind(user_id)
            .bind(rule_id)
            .bind(OrderStatus::Success)
            .fetch_one(&self.pool)
            .await?
        };

        Ok(count)
    }

    // ==================== 兑换明细 ====================

    /// 创建兑换明细
    pub async fn create_detail(&self, detail: &RedemptionDetail) -> Result<i64> {
        let row = sqlx::query(
            r#"
            INSERT INTO redemption_details (order_id, user_badge_id, badge_id, quantity, created_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
        )
        .bind(detail.order_id)
        .bind(detail.user_badge_id)
        .bind(detail.badge_id)
        .bind(detail.quantity)
        .bind(detail.created_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// 在事务中创建兑换明细
    pub async fn create_detail_in_tx(
        tx: &mut PgConnection,
        detail: &RedemptionDetail,
    ) -> Result<i64> {
        let row = sqlx::query(
            r#"
            INSERT INTO redemption_details (order_id, user_badge_id, badge_id, quantity, created_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
        )
        .bind(detail.order_id)
        .bind(detail.user_badge_id)
        .bind(detail.badge_id)
        .bind(detail.quantity)
        .bind(detail.created_at)
        .fetch_one(tx)
        .await?;

        Ok(row.get("id"))
    }

    /// 列出订单的兑换明细
    pub async fn list_details_by_order(&self, order_id: i64) -> Result<Vec<RedemptionDetail>> {
        let details = sqlx::query_as::<_, RedemptionDetail>(
            r#"
            SELECT id, order_id, user_badge_id, badge_id, quantity, created_at
            FROM redemption_details
            WHERE order_id = $1
            ORDER BY id ASC
            "#,
        )
        .bind(order_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(details)
    }
}

#[async_trait]
impl RedemptionRepositoryTrait for RedemptionRepository {
    async fn get_benefit(&self, id: i64) -> Result<Option<Benefit>> {
        self.get_benefit(id).await
    }

    async fn list_benefits(&self) -> Result<Vec<Benefit>> {
        self.list_benefits().await
    }

    async fn increment_redeemed_count(&self, benefit_id: i64, delta: i64) -> Result<()> {
        self.increment_redeemed_count(benefit_id, delta).await
    }

    async fn get_redemption_rule(&self, id: i64) -> Result<Option<BadgeRedemptionRule>> {
        self.get_redemption_rule(id).await
    }

    async fn list_rules_by_badge(&self, badge_id: i64) -> Result<Vec<BadgeRedemptionRule>> {
        self.list_rules_by_badge(badge_id).await
    }

    async fn list_active_rules(&self) -> Result<Vec<BadgeRedemptionRule>> {
        self.list_active_rules().await
    }

    async fn create_order(&self, order: &RedemptionOrder) -> Result<i64> {
        self.create_order(order).await
    }

    async fn get_order(&self, id: i64) -> Result<Option<RedemptionOrder>> {
        self.get_order(id).await
    }

    async fn get_order_by_no(&self, order_no: &str) -> Result<Option<RedemptionOrder>> {
        self.get_order_by_no(order_no).await
    }

    async fn get_order_by_idempotency_key(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<RedemptionOrder>> {
        self.get_order_by_idempotency_key(idempotency_key).await
    }

    async fn update_order_status(
        &self,
        id: i64,
        status: OrderStatus,
        failure_reason: Option<String>,
    ) -> Result<()> {
        self.update_order_status(id, status, failure_reason.as_deref())
            .await
    }

    async fn update_order_benefit_result(
        &self,
        id: i64,
        benefit_result: &serde_json::Value,
    ) -> Result<()> {
        self.update_order_benefit_result(id, benefit_result).await
    }

    async fn list_orders_by_user(&self, user_id: &str, limit: i64) -> Result<Vec<RedemptionOrder>> {
        self.list_orders_by_user(user_id, limit).await
    }

    async fn count_user_redemptions(
        &self,
        user_id: &str,
        rule_id: i64,
        since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<i64> {
        self.count_user_redemptions(user_id, rule_id, since).await
    }

    async fn create_detail(&self, detail: &RedemptionDetail) -> Result<i64> {
        self.create_detail(detail).await
    }

    async fn list_details_by_order(&self, order_id: i64) -> Result<Vec<RedemptionDetail>> {
        self.list_details_by_order(order_id).await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_repository_methods_exist() {
        // 类型检查：确保方法签名正确
    }
}
