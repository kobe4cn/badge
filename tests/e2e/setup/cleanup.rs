//! 测试数据清理
//!
//! 在测试前后清理数据库中的测试数据，确保测试隔离性。

use anyhow::Result;
use sqlx::PgPool;

/// 测试清理器
pub struct TestCleanup {
    pool: PgPool,
}

impl TestCleanup {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 清理所有测试数据
    ///
    /// 按照外键依赖顺序删除数据，避免约束冲突。
    /// 使用级联删除策略：先找出所有测试相关的 ID，然后按依赖顺序清理。
    pub async fn clean_all(&self) -> Result<()> {
        // 第一步：收集所有测试相关的 ID
        let test_badge_ids = self.get_test_badge_ids().await?;
        let test_series_ids = self.get_test_series_ids().await?;
        let test_category_ids = self.get_test_category_ids().await?;
        let test_benefit_ids = self.get_test_benefit_ids().await?;

        // 第二步：按外键依赖顺序清理（从叶子节点到根节点）
        // 1. 最底层：用户相关数据
        self.clean_auto_benefit_grants().await?;
        self.clean_auto_benefit_evaluation_logs().await?;
        self.clean_benefit_grants().await?;
        self.clean_redemption_orders().await?;

        // 2. 兑换规则（引用 badges 和 benefits）
        self.clean_redemption_rules_by_ids(&test_badge_ids, &test_benefit_ids)
            .await?;

        // 3. 用户徽章相关（引用 badges）
        self.clean_user_badge_logs().await?;
        self.clean_user_badges_by_badge_ids(&test_badge_ids).await?;
        self.clean_badge_ledger().await?;
        self.clean_cascade_logs().await?;

        // 4. 徽章规则和依赖（引用 badges）
        self.clean_badge_rules_by_badge_ids(&test_badge_ids).await?;
        self.clean_badge_dependencies_by_badge_ids(&test_badge_ids)
            .await?;

        // 5. 徽章本身（引用 badge_series）
        self.clean_badges_by_ids(&test_badge_ids).await?;
        self.clean_badges_by_series_ids(&test_series_ids).await?;

        // 6. 徽章系列（引用 badge_categories）
        self.clean_badge_series_by_ids(&test_series_ids).await?;
        self.clean_badge_series_by_category_ids(&test_category_ids)
            .await?;

        // 7. 徽章分类
        self.clean_badge_categories_by_ids(&test_category_ids).await?;

        // 8. 权益定义
        self.clean_benefits_by_ids(&test_benefit_ids).await?;

        tracing::info!("测试数据已清理");
        Ok(())
    }

    /// 获取所有测试徽章的 ID
    async fn get_test_badge_ids(&self) -> Result<Vec<i64>> {
        let rows: Vec<(i64,)> =
            sqlx::query_as("SELECT id FROM badges WHERE name LIKE 'Test%'")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 获取所有测试系列的 ID
    async fn get_test_series_ids(&self) -> Result<Vec<i64>> {
        let rows: Vec<(i64,)> =
            sqlx::query_as("SELECT id FROM badge_series WHERE name LIKE 'Test%'")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 获取所有测试分类的 ID
    async fn get_test_category_ids(&self) -> Result<Vec<i64>> {
        let rows: Vec<(i64,)> =
            sqlx::query_as("SELECT id FROM badge_categories WHERE name LIKE 'Test%'")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 获取所有测试权益的 ID
    async fn get_test_benefit_ids(&self) -> Result<Vec<i64>> {
        let rows: Vec<(i64,)> =
            sqlx::query_as("SELECT id FROM benefits WHERE name LIKE 'Test%'")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 清理自动权益发放记录
    async fn clean_auto_benefit_grants(&self) -> Result<()> {
        sqlx::query("DELETE FROM auto_benefit_grants WHERE user_id LIKE 'test_%'")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 清理自动权益评估日志
    async fn clean_auto_benefit_evaluation_logs(&self) -> Result<()> {
        sqlx::query("DELETE FROM auto_benefit_evaluation_logs WHERE user_id LIKE 'test_%'")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 清理权益发放记录
    async fn clean_benefit_grants(&self) -> Result<()> {
        // 同时清理 grant_no 以 test_ 开头的记录和 user_id 以 test_ 开头的记录
        // 后者用于清理自动权益发放记录（grant_no 格式为 auto_benefit:test_user_xxx）
        sqlx::query("DELETE FROM benefit_grants WHERE grant_no LIKE 'test_%' OR user_id LIKE 'test_%'")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 清理兑换订单
    async fn clean_redemption_orders(&self) -> Result<()> {
        sqlx::query("DELETE FROM redemption_orders WHERE order_no LIKE 'test_%'")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 根据徽章和权益 ID 清理兑换规则
    async fn clean_redemption_rules_by_ids(
        &self,
        badge_ids: &[i64],
        benefit_ids: &[i64],
    ) -> Result<()> {
        // 先清理名称匹配的规则
        sqlx::query("DELETE FROM badge_redemption_rules WHERE name LIKE 'Test%'")
            .execute(&self.pool)
            .await?;

        // 清理引用测试权益的规则
        if !benefit_ids.is_empty() {
            sqlx::query(
                "DELETE FROM badge_redemption_rules WHERE benefit_id = ANY($1::bigint[])",
            )
            .bind(benefit_ids)
            .execute(&self.pool)
            .await?;
        }

        // 清理引用测试徽章的规则（通过 required_badges JSONB 字段）
        if !badge_ids.is_empty() {
            for badge_id in badge_ids {
                sqlx::query(
                    "DELETE FROM badge_redemption_rules WHERE required_badges @> $1::jsonb",
                )
                .bind(serde_json::json!([badge_id]))
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }

    /// 清理用户徽章日志
    async fn clean_user_badge_logs(&self) -> Result<()> {
        sqlx::query("DELETE FROM user_badge_logs WHERE user_id LIKE 'test_%'")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 清理用户徽章（按 user_id 匹配）
    async fn clean_user_badges(&self) -> Result<()> {
        sqlx::query("DELETE FROM user_badges WHERE user_id LIKE 'test_%'")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 根据徽章 ID 清理用户徽章
    async fn clean_user_badges_by_badge_ids(&self, badge_ids: &[i64]) -> Result<()> {
        if !badge_ids.is_empty() {
            sqlx::query("DELETE FROM user_badges WHERE badge_id = ANY($1::bigint[])")
                .bind(badge_ids)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    /// 清理账本
    async fn clean_badge_ledger(&self) -> Result<()> {
        sqlx::query("DELETE FROM badge_ledger WHERE user_id LIKE 'test_%'")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 清理级联日志
    async fn clean_cascade_logs(&self) -> Result<()> {
        sqlx::query("DELETE FROM cascade_evaluation_logs WHERE user_id LIKE 'test_%'")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 清理徽章规则（按 rule_code 匹配）
    #[allow(dead_code)]
    async fn clean_badge_rules(&self) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM badge_rules
            WHERE rule_code LIKE 'test_%'
               OR badge_id IN (SELECT id FROM badges WHERE name LIKE 'Test%')
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// 根据徽章 ID 清理徽章规则
    async fn clean_badge_rules_by_badge_ids(&self, badge_ids: &[i64]) -> Result<()> {
        // 先清理 rule_code 匹配的
        sqlx::query("DELETE FROM badge_rules WHERE rule_code LIKE 'test_%'")
            .execute(&self.pool)
            .await?;

        // 再清理引用测试徽章的规则
        if !badge_ids.is_empty() {
            sqlx::query("DELETE FROM badge_rules WHERE badge_id = ANY($1::bigint[])")
                .bind(badge_ids)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    /// 清理徽章依赖（按名称匹配）
    #[allow(dead_code)]
    async fn clean_badge_dependencies(&self) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM badge_dependencies
            WHERE badge_id IN (SELECT id FROM badges WHERE name LIKE 'Test%')
               OR depends_on_badge_id IN (SELECT id FROM badges WHERE name LIKE 'Test%')
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// 根据徽章 ID 清理徽章依赖
    async fn clean_badge_dependencies_by_badge_ids(&self, badge_ids: &[i64]) -> Result<()> {
        if !badge_ids.is_empty() {
            sqlx::query(
                "DELETE FROM badge_dependencies WHERE badge_id = ANY($1::bigint[]) OR depends_on_badge_id = ANY($1::bigint[])",
            )
            .bind(badge_ids)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    /// 清理徽章（按名称匹配）
    #[allow(dead_code)]
    async fn clean_badges(&self) -> Result<()> {
        sqlx::query("DELETE FROM badges WHERE name LIKE 'Test%'")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 根据 ID 清理徽章
    async fn clean_badges_by_ids(&self, badge_ids: &[i64]) -> Result<()> {
        if !badge_ids.is_empty() {
            sqlx::query("DELETE FROM badges WHERE id = ANY($1::bigint[])")
                .bind(badge_ids)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    /// 根据系列 ID 清理徽章
    async fn clean_badges_by_series_ids(&self, series_ids: &[i64]) -> Result<()> {
        if !series_ids.is_empty() {
            sqlx::query("DELETE FROM badges WHERE series_id = ANY($1::bigint[])")
                .bind(series_ids)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    /// 清理徽章系列（按名称匹配）
    #[allow(dead_code)]
    async fn clean_badge_series(&self) -> Result<()> {
        sqlx::query("DELETE FROM badge_series WHERE name LIKE 'Test%'")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 根据 ID 清理徽章系列
    async fn clean_badge_series_by_ids(&self, series_ids: &[i64]) -> Result<()> {
        if !series_ids.is_empty() {
            sqlx::query("DELETE FROM badge_series WHERE id = ANY($1::bigint[])")
                .bind(series_ids)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    /// 根据分类 ID 清理徽章系列
    async fn clean_badge_series_by_category_ids(&self, category_ids: &[i64]) -> Result<()> {
        if !category_ids.is_empty() {
            sqlx::query("DELETE FROM badge_series WHERE category_id = ANY($1::bigint[])")
                .bind(category_ids)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    /// 清理徽章分类（按名称匹配）
    #[allow(dead_code)]
    async fn clean_badge_categories(&self) -> Result<()> {
        sqlx::query("DELETE FROM badge_categories WHERE name LIKE 'Test%'")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 根据 ID 清理徽章分类
    async fn clean_badge_categories_by_ids(&self, category_ids: &[i64]) -> Result<()> {
        if !category_ids.is_empty() {
            sqlx::query("DELETE FROM badge_categories WHERE id = ANY($1::bigint[])")
                .bind(category_ids)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    /// 清理权益定义（按名称匹配）
    #[allow(dead_code)]
    async fn clean_benefits(&self) -> Result<()> {
        sqlx::query("DELETE FROM benefits WHERE name LIKE 'Test%'")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 根据 ID 清理权益定义
    async fn clean_benefits_by_ids(&self, benefit_ids: &[i64]) -> Result<()> {
        if !benefit_ids.is_empty() {
            sqlx::query("DELETE FROM benefits WHERE id = ANY($1::bigint[])")
                .bind(benefit_ids)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    /// 清理 Redis 测试数据
    pub async fn clean_redis(&self, redis_url: &str) -> Result<()> {
        let client = redis::Client::open(redis_url)?;
        let mut conn = client.get_multiplexed_async_connection().await?;

        // 清理测试相关的 key
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg("test_*")
            .query_async(&mut conn)
            .await?;

        if !keys.is_empty() {
            let _: i32 = redis::cmd("DEL").arg(&keys).query_async(&mut conn).await?;
        }

        Ok(())
    }
}
