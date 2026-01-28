//! 徽章仓储
//!
//! 提供徽章分类、系列、徽章定义的数据访问

use async_trait::async_trait;
use sqlx::PgPool;

use super::traits::BadgeRepositoryTrait;
use crate::error::Result;
use crate::models::{Badge, BadgeCategory, BadgeRule, BadgeSeries, BadgeStatus, CategoryStatus};

/// 徽章仓储
///
/// 负责徽章三层结构（分类 -> 系列 -> 徽章）的数据访问
pub struct BadgeRepository {
    pool: PgPool,
}

impl BadgeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ==================== 徽章分类 ====================

    /// 获取单个分类
    pub async fn get_category(&self, id: i64) -> Result<Option<BadgeCategory>> {
        let category = sqlx::query_as::<_, BadgeCategory>(
            r#"
            SELECT id, name, icon_url, sort_order, status, created_at, updated_at
            FROM badge_categories
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(category)
    }

    /// 列出所有启用的分类
    pub async fn list_categories(&self) -> Result<Vec<BadgeCategory>> {
        let categories = sqlx::query_as::<_, BadgeCategory>(
            r#"
            SELECT id, name, icon_url, sort_order, status, created_at, updated_at
            FROM badge_categories
            WHERE status = $1
            ORDER BY sort_order ASC, id ASC
            "#,
        )
        .bind(CategoryStatus::Active)
        .fetch_all(&self.pool)
        .await?;

        Ok(categories)
    }

    // ==================== 徽章系列 ====================

    /// 获取单个系列
    pub async fn get_series(&self, id: i64) -> Result<Option<BadgeSeries>> {
        let series = sqlx::query_as::<_, BadgeSeries>(
            r#"
            SELECT id, category_id, name, description, cover_url, sort_order,
                   status, start_time, end_time, created_at, updated_at
            FROM badge_series
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(series)
    }

    /// 按分类列出系列
    pub async fn list_series_by_category(&self, category_id: i64) -> Result<Vec<BadgeSeries>> {
        let series = sqlx::query_as::<_, BadgeSeries>(
            r#"
            SELECT id, category_id, name, description, cover_url, sort_order,
                   status, start_time, end_time, created_at, updated_at
            FROM badge_series
            WHERE category_id = $1 AND status = $2
            ORDER BY sort_order ASC, id ASC
            "#,
        )
        .bind(category_id)
        .bind(CategoryStatus::Active)
        .fetch_all(&self.pool)
        .await?;

        Ok(series)
    }

    // ==================== 徽章 ====================

    /// 获取单个徽章
    pub async fn get_badge(&self, id: i64) -> Result<Option<Badge>> {
        let badge = sqlx::query_as::<_, Badge>(
            r#"
            SELECT id, series_id, badge_type, name, description, obtain_description,
                   sort_order, status, assets, validity_config, max_supply,
                   issued_count, created_at, updated_at
            FROM badges
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(badge)
    }

    /// 批量获取徽章
    pub async fn get_badges_by_ids(&self, ids: &[i64]) -> Result<Vec<Badge>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let badges = sqlx::query_as::<_, Badge>(
            r#"
            SELECT id, series_id, badge_type, name, description, obtain_description,
                   sort_order, status, assets, validity_config, max_supply,
                   issued_count, created_at, updated_at
            FROM badges
            WHERE id = ANY($1)
            ORDER BY sort_order ASC, id ASC
            "#,
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await?;

        Ok(badges)
    }

    /// 按系列列出徽章
    pub async fn list_badges_by_series(&self, series_id: i64) -> Result<Vec<Badge>> {
        let badges = sqlx::query_as::<_, Badge>(
            r#"
            SELECT id, series_id, badge_type, name, description, obtain_description,
                   sort_order, status, assets, validity_config, max_supply,
                   issued_count, created_at, updated_at
            FROM badges
            WHERE series_id = $1 AND status = $2
            ORDER BY sort_order ASC, id ASC
            "#,
        )
        .bind(series_id)
        .bind(BadgeStatus::Active)
        .fetch_all(&self.pool)
        .await?;

        Ok(badges)
    }

    /// 列出所有已上线徽章
    pub async fn list_active_badges(&self) -> Result<Vec<Badge>> {
        let badges = sqlx::query_as::<_, Badge>(
            r#"
            SELECT id, series_id, badge_type, name, description, obtain_description,
                   sort_order, status, assets, validity_config, max_supply,
                   issued_count, created_at, updated_at
            FROM badges
            WHERE status = $1
            ORDER BY sort_order ASC, id ASC
            "#,
        )
        .bind(BadgeStatus::Active)
        .fetch_all(&self.pool)
        .await?;

        Ok(badges)
    }

    // ==================== 徽章规则 ====================

    /// 获取徽章的获取规则
    pub async fn get_badge_rules(&self, badge_id: i64) -> Result<Vec<BadgeRule>> {
        let rules = sqlx::query_as::<_, BadgeRule>(
            r#"
            SELECT id, badge_id, rule_json, start_time, end_time,
                   max_count_per_user, enabled, created_at, updated_at
            FROM badge_rules
            WHERE badge_id = $1 AND enabled = true
            ORDER BY id ASC
            "#,
        )
        .bind(badge_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rules)
    }

    // ==================== 库存操作 ====================

    /// 增加徽章已发放数量
    ///
    /// 用于发放徽章时更新库存计数
    pub async fn increment_issued_count(&self, badge_id: i64, delta: i64) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE badges
            SET issued_count = issued_count + $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(badge_id)
        .bind(delta)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl BadgeRepositoryTrait for BadgeRepository {
    async fn get_category(&self, id: i64) -> Result<Option<BadgeCategory>> {
        self.get_category(id).await
    }

    async fn list_categories(&self) -> Result<Vec<BadgeCategory>> {
        self.list_categories().await
    }

    async fn get_series(&self, id: i64) -> Result<Option<BadgeSeries>> {
        self.get_series(id).await
    }

    async fn list_series_by_category(&self, category_id: i64) -> Result<Vec<BadgeSeries>> {
        self.list_series_by_category(category_id).await
    }

    async fn get_badge(&self, id: i64) -> Result<Option<Badge>> {
        self.get_badge(id).await
    }

    async fn get_badges_by_ids(&self, ids: &[i64]) -> Result<Vec<Badge>> {
        self.get_badges_by_ids(ids).await
    }

    async fn list_badges_by_series(&self, series_id: i64) -> Result<Vec<Badge>> {
        self.list_badges_by_series(series_id).await
    }

    async fn list_active_badges(&self) -> Result<Vec<Badge>> {
        self.list_active_badges().await
    }

    async fn get_badge_rules(&self, badge_id: i64) -> Result<Vec<BadgeRule>> {
        self.get_badge_rules(badge_id).await
    }

    async fn increment_issued_count(&self, badge_id: i64, delta: i64) -> Result<()> {
        self.increment_issued_count(badge_id, delta).await
    }
}

#[cfg(test)]
mod tests {
    // 由于没有测试数据库，这里只测试仓储的构造
    // 实际集成测试需要配合 testcontainers 或测试数据库

    #[test]
    fn test_repository_creation() {
        // 仅验证类型定义正确，不实际连接数据库
        // let pool = PgPool::connect_lazy("postgres://test").unwrap();
        // let repo = BadgeRepository::new(pool);
        // 在集成测试中使用
    }
}
