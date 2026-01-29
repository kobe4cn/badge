//! 用户徽章仓储
//!
//! 提供用户持有徽章的数据访问，支持事务和行级锁

use async_trait::async_trait;
use sqlx::{PgConnection, PgPool, Row};

use super::traits::UserBadgeRepositoryTrait;
use crate::error::Result;
use crate::models::{UserBadge, UserBadgeStatus};

/// 用户徽章仓储
///
/// 负责用户徽章的 CRUD 操作，支持事务场景下的行级锁定
pub struct UserBadgeRepository {
    pool: PgPool,
}

impl UserBadgeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ==================== 查询操作 ====================

    /// 获取用户的某个徽章记录
    pub async fn get_user_badge(&self, user_id: &str, badge_id: i64) -> Result<Option<UserBadge>> {
        let user_badge = sqlx::query_as::<_, UserBadge>(
            r#"
            SELECT id, user_id, badge_id, status, quantity, acquired_at,
                   expires_at, created_at, updated_at
            FROM user_badges
            WHERE user_id = $1 AND badge_id = $2
            "#,
        )
        .bind(user_id)
        .bind(badge_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user_badge)
    }

    /// 根据 ID 获取用户徽章
    pub async fn get_user_badge_by_id(&self, id: i64) -> Result<Option<UserBadge>> {
        let user_badge = sqlx::query_as::<_, UserBadge>(
            r#"
            SELECT id, user_id, badge_id, status, quantity, acquired_at,
                   expires_at, created_at, updated_at
            FROM user_badges
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user_badge)
    }

    /// 列出用户的所有徽章
    pub async fn list_user_badges(&self, user_id: &str) -> Result<Vec<UserBadge>> {
        let badges = sqlx::query_as::<_, UserBadge>(
            r#"
            SELECT id, user_id, badge_id, status, quantity, acquired_at,
                   expires_at, created_at, updated_at
            FROM user_badges
            WHERE user_id = $1
            ORDER BY acquired_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(badges)
    }

    /// 按状态列出用户徽章
    pub async fn list_user_badges_by_status(
        &self,
        user_id: &str,
        status: UserBadgeStatus,
    ) -> Result<Vec<UserBadge>> {
        let badges = sqlx::query_as::<_, UserBadge>(
            r#"
            SELECT id, user_id, badge_id, status, quantity, acquired_at,
                   expires_at, created_at, updated_at
            FROM user_badges
            WHERE user_id = $1 AND status = $2
            ORDER BY acquired_at DESC
            "#,
        )
        .bind(user_id)
        .bind(status)
        .fetch_all(&self.pool)
        .await?;

        Ok(badges)
    }

    // ==================== 写入操作 ====================

    /// 创建用户徽章记录
    ///
    /// 返回新记录的 ID
    pub async fn create_user_badge(&self, badge: &UserBadge) -> Result<i64> {
        let row = sqlx::query(
            r#"
            INSERT INTO user_badges (user_id, badge_id, status, quantity, acquired_at, expires_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id
            "#,
        )
        .bind(&badge.user_id)
        .bind(badge.badge_id)
        .bind(badge.status)
        .bind(badge.quantity)
        .bind(badge.acquired_at)
        .bind(badge.expires_at)
        .bind(badge.created_at)
        .bind(badge.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// 更新用户徽章记录
    pub async fn update_user_badge(&self, badge: &UserBadge) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE user_badges
            SET status = $2, quantity = $3, expires_at = $4, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(badge.id)
        .bind(badge.status)
        .bind(badge.quantity)
        .bind(badge.expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 更新用户徽章数量
    ///
    /// 使用增量更新而非覆盖，避免并发问题
    pub async fn update_user_badge_quantity(&self, id: i64, delta: i32) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE user_badges
            SET quantity = quantity + $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(delta)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ==================== 事务操作 ====================

    /// 在事务中获取用户徽章（带行级锁）
    ///
    /// 使用 FOR UPDATE 锁定行，防止兑换时的并发问题
    pub async fn get_user_badge_for_update(
        tx: &mut PgConnection,
        user_id: &str,
        badge_id: i64,
    ) -> Result<Option<UserBadge>> {
        let user_badge = sqlx::query_as::<_, UserBadge>(
            r#"
            SELECT id, user_id, badge_id, status, quantity, acquired_at,
                   expires_at, created_at, updated_at
            FROM user_badges
            WHERE user_id = $1 AND badge_id = $2
            FOR UPDATE
            "#,
        )
        .bind(user_id)
        .bind(badge_id)
        .fetch_optional(tx)
        .await?;

        Ok(user_badge)
    }

    /// 在事务中创建用户徽章
    pub async fn create_user_badge_in_tx(tx: &mut PgConnection, badge: &UserBadge) -> Result<i64> {
        let row = sqlx::query(
            r#"
            INSERT INTO user_badges (user_id, badge_id, status, quantity, acquired_at, expires_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id
            "#,
        )
        .bind(&badge.user_id)
        .bind(badge.badge_id)
        .bind(badge.status)
        .bind(badge.quantity)
        .bind(badge.acquired_at)
        .bind(badge.expires_at)
        .bind(badge.created_at)
        .bind(badge.updated_at)
        .fetch_one(tx)
        .await?;

        Ok(row.get("id"))
    }

    /// 在事务中更新用户徽章数量
    pub async fn update_user_badge_quantity_in_tx(
        tx: &mut PgConnection,
        id: i64,
        delta: i32,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE user_badges
            SET quantity = quantity + $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(delta)
        .execute(tx)
        .await?;

        Ok(())
    }

    /// 在事务中更新用户徽章状态
    pub async fn update_user_badge_status_in_tx(
        tx: &mut PgConnection,
        id: i64,
        status: UserBadgeStatus,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE user_badges
            SET status = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(status)
        .execute(tx)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl UserBadgeRepositoryTrait for UserBadgeRepository {
    async fn get_user_badge(&self, user_id: &str, badge_id: i64) -> Result<Option<UserBadge>> {
        self.get_user_badge(user_id, badge_id).await
    }

    async fn get_user_badge_by_id(&self, id: i64) -> Result<Option<UserBadge>> {
        self.get_user_badge_by_id(id).await
    }

    async fn list_user_badges(&self, user_id: &str) -> Result<Vec<UserBadge>> {
        self.list_user_badges(user_id).await
    }

    async fn list_user_badges_by_status(
        &self,
        user_id: &str,
        status: UserBadgeStatus,
    ) -> Result<Vec<UserBadge>> {
        self.list_user_badges_by_status(user_id, status).await
    }

    async fn create_user_badge(&self, badge: &UserBadge) -> Result<i64> {
        self.create_user_badge(badge).await
    }

    async fn update_user_badge(&self, badge: &UserBadge) -> Result<()> {
        self.update_user_badge(badge).await
    }

    async fn update_user_badge_quantity(&self, id: i64, delta: i32) -> Result<()> {
        self.update_user_badge_quantity(id, delta).await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_repository_methods_exist() {
        // 类型检查：确保方法签名正确
        // 实际测试需要配合测试数据库
    }
}
