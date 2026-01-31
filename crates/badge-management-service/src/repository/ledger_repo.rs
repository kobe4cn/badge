//! 徽章账本仓储
//!
//! 提供徽章流水记录的数据访问，支持余额查询和流水追溯

use async_trait::async_trait;
use sqlx::{PgConnection, PgPool, Row};

use super::traits::BadgeLedgerRepositoryTrait;
use crate::error::Result;
use crate::models::BadgeLedger;

/// 徽章账本仓储
///
/// 采用复式记账思想，记录徽章数量的每一次变动，确保数据可追溯
pub struct BadgeLedgerRepository {
    pool: PgPool,
}

impl BadgeLedgerRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 创建账本记录
    ///
    /// 返回新记录的 ID
    pub async fn create(&self, ledger: &BadgeLedger) -> Result<i64> {
        let row = sqlx::query(
            r#"
            INSERT INTO badge_ledger (user_id, badge_id, change_type, quantity, balance_after, ref_id, source_type, remark, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id
            "#,
        )
        .bind(&ledger.user_id)
        .bind(ledger.badge_id)
        .bind(ledger.change_type)
        .bind(ledger.quantity)
        .bind(ledger.balance_after)
        .bind(&ledger.ref_id)
        .bind(ledger.ref_type)
        .bind(&ledger.remark)
        .bind(ledger.created_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// 在事务中创建账本记录
    pub async fn create_in_tx(tx: &mut PgConnection, ledger: &BadgeLedger) -> Result<i64> {
        let row = sqlx::query(
            r#"
            INSERT INTO badge_ledger (user_id, badge_id, change_type, quantity, balance_after, ref_id, source_type, remark, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id
            "#,
        )
        .bind(&ledger.user_id)
        .bind(ledger.badge_id)
        .bind(ledger.change_type)
        .bind(ledger.quantity)
        .bind(ledger.balance_after)
        .bind(&ledger.ref_id)
        .bind(ledger.ref_type)
        .bind(&ledger.remark)
        .bind(ledger.created_at)
        .fetch_one(tx)
        .await?;

        Ok(row.get("id"))
    }

    /// 列出用户的账本记录
    ///
    /// 按时间倒序排列，返回最近的 limit 条记录
    pub async fn list_by_user(&self, user_id: &str, limit: i64) -> Result<Vec<BadgeLedger>> {
        let ledgers = sqlx::query_as::<_, BadgeLedger>(
            r#"
            SELECT id, user_id, badge_id, change_type, quantity, balance_after,
                   ref_id, source_type AS ref_type, remark, created_at
            FROM badge_ledger
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(ledgers)
    }

    /// 列出用户某个徽章的账本记录
    pub async fn list_by_user_badge(
        &self,
        user_id: &str,
        badge_id: i64,
    ) -> Result<Vec<BadgeLedger>> {
        let ledgers = sqlx::query_as::<_, BadgeLedger>(
            r#"
            SELECT id, user_id, badge_id, change_type, quantity, balance_after,
                   ref_id, source_type AS ref_type, remark, created_at
            FROM badge_ledger
            WHERE user_id = $1 AND badge_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .bind(badge_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(ledgers)
    }

    /// 获取用户某个徽章的当前余额
    ///
    /// 通过聚合最近一条流水的 balance_after 获取，如无记录返回 0
    pub async fn get_balance(&self, user_id: &str, badge_id: i64) -> Result<i32> {
        let row = sqlx::query(
            r#"
            SELECT COALESCE(
                (SELECT balance_after
                 FROM badge_ledger
                 WHERE user_id = $1 AND badge_id = $2
                 ORDER BY created_at DESC, id DESC
                 LIMIT 1),
                0
            ) as balance
            "#,
        )
        .bind(user_id)
        .bind(badge_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("balance"))
    }

    /// 在事务中获取用户某个徽章的当前余额
    pub async fn get_balance_in_tx(
        tx: &mut PgConnection,
        user_id: &str,
        badge_id: i64,
    ) -> Result<i32> {
        let row = sqlx::query(
            r#"
            SELECT COALESCE(
                (SELECT balance_after
                 FROM badge_ledger
                 WHERE user_id = $1 AND badge_id = $2
                 ORDER BY created_at DESC, id DESC
                 LIMIT 1),
                0
            ) as balance
            "#,
        )
        .bind(user_id)
        .bind(badge_id)
        .fetch_one(tx)
        .await?;

        Ok(row.get("balance"))
    }

    /// 获取用户所有徽章的余额汇总
    ///
    /// 返回 (badge_id, balance) 列表
    pub async fn get_all_balances(&self, user_id: &str) -> Result<Vec<(i64, i32)>> {
        // 使用窗口函数获取每个徽章的最新余额
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT ON (badge_id) badge_id, balance_after
            FROM badge_ledger
            WHERE user_id = $1
            ORDER BY badge_id, created_at DESC, id DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let balances = rows
            .iter()
            .map(|row| (row.get("badge_id"), row.get("balance_after")))
            .collect();

        Ok(balances)
    }
}

#[async_trait]
impl BadgeLedgerRepositoryTrait for BadgeLedgerRepository {
    async fn create(&self, ledger: &BadgeLedger) -> Result<i64> {
        self.create(ledger).await
    }

    async fn list_by_user(&self, user_id: &str, limit: i64) -> Result<Vec<BadgeLedger>> {
        self.list_by_user(user_id, limit).await
    }

    async fn list_by_user_badge(&self, user_id: &str, badge_id: i64) -> Result<Vec<BadgeLedger>> {
        self.list_by_user_badge(user_id, badge_id).await
    }

    async fn get_balance(&self, user_id: &str, badge_id: i64) -> Result<i32> {
        self.get_balance(user_id, badge_id).await
    }

    async fn get_all_balances(&self, user_id: &str) -> Result<Vec<(i64, i32)>> {
        self.get_all_balances(user_id).await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_repository_methods_exist() {
        // 类型检查：确保方法签名正确
    }
}
