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
    pub async fn clean_all(&self) -> Result<()> {
        // 按依赖顺序清理
        self.clean_benefit_grants().await?;
        self.clean_redemption_orders().await?;
        self.clean_user_badge_logs().await?;
        self.clean_user_badges().await?;
        self.clean_badge_ledger().await?;
        self.clean_cascade_logs().await?;
        self.clean_badge_rules().await?;
        self.clean_badge_dependencies().await?;
        self.clean_badges().await?;
        self.clean_badge_series().await?;
        self.clean_badge_categories().await?;
        self.clean_benefits().await?;

        tracing::info!("测试数据已清理");
        Ok(())
    }

    /// 清理权益发放记录
    async fn clean_benefit_grants(&self) -> Result<()> {
        sqlx::query("DELETE FROM benefit_grants WHERE grant_no LIKE 'test_%'")
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

    /// 清理用户徽章日志
    async fn clean_user_badge_logs(&self) -> Result<()> {
        sqlx::query("DELETE FROM user_badge_logs WHERE user_id LIKE 'test_%'")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 清理用户徽章
    async fn clean_user_badges(&self) -> Result<()> {
        sqlx::query("DELETE FROM user_badges WHERE user_id LIKE 'test_%'")
            .execute(&self.pool)
            .await?;
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
        sqlx::query("DELETE FROM cascade_logs WHERE user_id LIKE 'test_%'")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 清理徽章规则
    async fn clean_badge_rules(&self) -> Result<()> {
        sqlx::query("DELETE FROM badge_rules WHERE rule_code LIKE 'test_%'")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 清理徽章依赖
    async fn clean_badge_dependencies(&self) -> Result<()> {
        // 清理测试徽章的依赖关系
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

    /// 清理徽章
    async fn clean_badges(&self) -> Result<()> {
        sqlx::query("DELETE FROM badges WHERE name LIKE 'Test%'")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 清理徽章系列
    async fn clean_badge_series(&self) -> Result<()> {
        sqlx::query("DELETE FROM badge_series WHERE name LIKE 'Test%'")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 清理徽章分类
    async fn clean_badge_categories(&self) -> Result<()> {
        sqlx::query("DELETE FROM badge_categories WHERE name LIKE 'Test%'")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 清理权益定义
    async fn clean_benefits(&self) -> Result<()> {
        sqlx::query("DELETE FROM benefits WHERE name LIKE 'Test%'")
            .execute(&self.pool)
            .await?;
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
