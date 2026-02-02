//! 数据库验证工具
//!
//! 提供数据库状态断言功能，验证测试结果。

use anyhow::Result;
use sqlx::PgPool;

/// 数据库验证工具
pub struct DbVerifier {
    pool: PgPool,
}

impl DbVerifier {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ========== 用户徽章验证 ==========

    /// 获取用户所有徽章
    pub async fn get_user_badges(&self, user_id: &str) -> Result<Vec<UserBadgeRecord>> {
        let records = sqlx::query_as::<_, UserBadgeRecord>(
            r#"
            SELECT ub.id, ub.user_id, ub.badge_id, b.name as badge_name,
                   ub.status, ub.quantity, ub.first_acquired_at as acquired_at, ub.expires_at, ub.source_type
            FROM user_badges ub
            JOIN badges b ON ub.badge_id = b.id
            WHERE ub.user_id = $1
            ORDER BY ub.first_acquired_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// 检查用户是否拥有指定徽章
    ///
    /// 注意：badge-management-service 使用 sqlx 枚举写入 'ACTIVE'（大写），
    /// 而 badge-admin-service 直接写入 'active'（小写），故使用 ILIKE 不区分大小写比较。
    pub async fn user_has_badge(&self, user_id: &str, badge_id: i64) -> Result<bool> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM user_badges WHERE user_id = $1 AND badge_id = $2 AND UPPER(status) = 'ACTIVE'",
        )
        .bind(user_id)
        .bind(badge_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0 > 0)
    }

    /// 获取用户徽章数量
    pub async fn get_user_badge_count(&self, user_id: &str, badge_id: i64) -> Result<i32> {
        let result: Option<(i32,)> = sqlx::query_as(
            "SELECT quantity FROM user_badges WHERE user_id = $1 AND badge_id = $2 AND UPPER(status) = 'ACTIVE'",
        )
        .bind(user_id)
        .bind(badge_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|r| r.0).unwrap_or(0))
    }

    // ========== 账本验证 ==========

    /// 获取徽章账本记录
    pub async fn get_badge_ledger(
        &self,
        badge_id: i64,
        user_id: &str,
    ) -> Result<Vec<LedgerRecord>> {
        let records = sqlx::query_as::<_, LedgerRecord>(
            r#"
            SELECT id, user_id, badge_id, quantity as delta, balance_after as balance,
                   UPPER(change_type) as action, remark as reason, created_at
            FROM badge_ledger
            WHERE badge_id = $1 AND user_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(badge_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// 获取用户徽章余额
    pub async fn get_badge_balance(&self, user_id: &str, badge_id: i64) -> Result<i32> {
        let result: Option<(i32,)> = sqlx::query_as(
            "SELECT balance_after FROM badge_ledger WHERE user_id = $1 AND badge_id = $2 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(user_id)
        .bind(badge_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|r| r.0).unwrap_or(0))
    }

    // ========== 权益验证 ==========

    /// 获取权益发放记录
    pub async fn get_benefit_grants(&self, user_id: &str) -> Result<Vec<BenefitGrantRecord>> {
        let records = sqlx::query_as::<_, BenefitGrantRecord>(
            r#"
            SELECT bg.grant_no, bg.user_id, bg.benefit_id, b.benefit_type, bg.status,
                   bg.external_ref, bg.granted_at, bg.expires_at
            FROM benefit_grants bg
            JOIN benefits b ON bg.benefit_id = b.id
            WHERE bg.user_id = $1
            ORDER BY bg.granted_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// 检查权益是否已发放
    pub async fn benefit_granted(&self, user_id: &str, benefit_id: i64) -> Result<bool> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM benefit_grants WHERE user_id = $1 AND benefit_id = $2 AND status = 'success'",
        )
        .bind(user_id)
        .bind(benefit_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0 > 0)
    }

    // ========== 规则验证 ==========

    /// 获取规则信息
    pub async fn get_rule(&self, rule_id: i64) -> Result<Option<RuleRecord>> {
        let record = sqlx::query_as::<_, RuleRecord>(
            r#"
            SELECT id, badge_id, rule_code, event_type, rule_json,
                   enabled, global_quota, COALESCE(global_granted, 0) as global_granted
            FROM badge_rules
            WHERE id = $1
            "#,
        )
        .bind(rule_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// 获取规则已发放数量
    pub async fn get_rule_granted_count(&self, rule_id: i64) -> Result<i32> {
        let result: (i32,) =
            sqlx::query_as("SELECT COALESCE(global_granted, 0) FROM badge_rules WHERE id = $1")
                .bind(rule_id)
                .fetch_one(&self.pool)
                .await?;

        Ok(result.0)
    }

    // ========== 徽章验证 ==========

    /// 获取徽章已发放数量
    pub async fn get_badge_issued_count(&self, badge_id: i64) -> Result<i64> {
        let result: (i64,) =
            sqlx::query_as("SELECT COALESCE(issued_count, 0) FROM badges WHERE id = $1")
                .bind(badge_id)
                .fetch_one(&self.pool)
                .await?;

        Ok(result.0)
    }

    /// 获取徽章详情
    pub async fn get_badge(&self, badge_id: i64) -> Result<Option<BadgeRecord>> {
        let record = sqlx::query_as::<_, BadgeRecord>(
            r#"
            SELECT id, series_id, name, badge_type, status, max_supply, issued_count
            FROM badges
            WHERE id = $1
            "#,
        )
        .bind(badge_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// 获取徽章统计信息
    pub async fn get_badge_stats(&self, badge_id: i64) -> Result<BadgeStats> {
        let granted_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM user_badges WHERE badge_id = $1 AND UPPER(status) = 'ACTIVE'",
        )
        .bind(badge_id)
        .fetch_one(&self.pool)
        .await?;

        let badge = self
            .get_badge(badge_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("徽章不存在: {}", badge_id))?;

        Ok(BadgeStats {
            badge_id,
            granted_count: granted_count.0,
            max_supply: badge.max_supply,
            issued_count: badge.issued_count,
        })
    }

    // ========== 级联验证 ==========

    /// 获取级联日志
    pub async fn get_cascade_logs(&self, user_id: &str) -> Result<Vec<CascadeLogRecord>> {
        let records = sqlx::query_as::<_, CascadeLogRecord>(
            r#"
            SELECT id, user_id, trigger_badge_id, result_status,
                   granted_badges, blocked_badges, duration_ms, created_at
            FROM cascade_evaluation_logs
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    // ========== 通用查询 ==========

    /// 执行自定义 SQL 查询
    pub async fn query_one<T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin>(
        &self,
        sql: &str,
    ) -> Result<T> {
        let record = sqlx::query_as::<_, T>(sql).fetch_one(&self.pool).await?;
        Ok(record)
    }

    /// 执行计数查询
    pub async fn count(&self, table: &str, condition: &str) -> Result<i64> {
        let sql = format!("SELECT COUNT(*) FROM {} WHERE {}", table, condition);
        let result: (i64,) = sqlx::query_as(&sql).fetch_one(&self.pool).await?;
        Ok(result.0)
    }
}

// ========== 记录类型 ==========

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserBadgeRecord {
    pub id: i64,
    pub user_id: String,
    pub badge_id: i64,
    pub badge_name: String,
    pub status: String,
    pub quantity: i32,
    pub acquired_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub source_type: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LedgerRecord {
    pub id: i64,
    pub user_id: String,
    pub badge_id: i64,
    pub delta: i32,
    pub balance: i32,
    pub action: String,
    pub reason: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BenefitGrantRecord {
    pub grant_no: String,
    pub user_id: String,
    pub benefit_id: i64,
    pub benefit_type: String,
    pub status: String,
    pub external_ref: Option<String>,
    pub granted_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RuleRecord {
    pub id: i64,
    pub badge_id: i64,
    pub rule_code: Option<String>,
    pub event_type: Option<String>,
    pub rule_json: serde_json::Value,
    pub enabled: bool,
    pub global_quota: Option<i32>,
    pub global_granted: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BadgeRecord {
    pub id: i64,
    pub series_id: i64,
    pub name: String,
    pub badge_type: String,
    pub status: String,
    pub max_supply: Option<i64>,
    pub issued_count: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CascadeLogRecord {
    pub id: i64,
    pub user_id: String,
    pub trigger_badge_id: i64,
    pub result_status: String,
    pub granted_badges: Option<serde_json::Value>,
    pub blocked_badges: Option<serde_json::Value>,
    pub duration_ms: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl CascadeLogRecord {
    /// 检查某个徽章是否在本次级联评估中被发放
    pub fn has_granted_badge(&self, badge_id: i64) -> bool {
        if let Some(ref badges) = self.granted_badges {
            if let Some(arr) = badges.as_array() {
                return arr.iter().any(|b| {
                    b.get("badge_id")
                        .and_then(|id| id.as_i64())
                        .map(|id| id == badge_id)
                        .unwrap_or(false)
                });
            }
        }
        false
    }

    /// 检查某个徽章是否在本次级联评估中被阻止
    pub fn has_blocked_badge(&self, badge_id: i64) -> bool {
        if let Some(ref badges) = self.blocked_badges {
            if let Some(arr) = badges.as_array() {
                return arr.iter().any(|b| {
                    b.get("badge_id")
                        .and_then(|id| id.as_i64())
                        .map(|id| id == badge_id)
                        .unwrap_or(false)
                });
            }
        }
        false
    }

    /// 检查评估是否成功
    ///
    /// evaluator 使用 "completed" 表示成功完成的级联评估
    pub fn is_success(&self) -> bool {
        self.result_status == "completed"
    }

    /// 检查是否检测到循环
    pub fn is_cycle_detected(&self) -> bool {
        self.result_status == "cycle_detected"
    }
}

/// 徽章统计信息
#[derive(Debug, Clone)]
pub struct BadgeStats {
    pub badge_id: i64,
    pub granted_count: i64,
    pub max_supply: Option<i64>,
    pub issued_count: i64,
}
