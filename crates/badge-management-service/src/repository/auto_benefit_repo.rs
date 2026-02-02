//! 自动权益发放仓储
//!
//! 提供 `auto_benefit_grants` 和 `auto_benefit_evaluation_logs` 表的数据访问操作。
//! 实现幂等插入、状态更新和统计查询等功能。

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Row};

use crate::auto_benefit::{AutoBenefitEvaluationLog, AutoBenefitGrant, AutoBenefitStatus, NewAutoBenefitGrant};
use crate::error::Result;

/// 自动权益发放仓储
///
/// 负责自动权益发放记录和评估日志的数据库操作
pub struct AutoBenefitRepository {
    pool: PgPool,
}

/// 数据库行映射结构
///
/// 用于从数据库查询结果映射到领域模型
#[derive(FromRow)]
struct AutoBenefitGrantRow {
    id: i64,
    rule_id: i64,
    benefit_grant_id: Option<i64>,
    status: String,
}

impl AutoBenefitRepository {
    /// 创建仓储实例
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ==================== 幂等与查询操作 ====================

    /// 检查幂等键是否已存在
    ///
    /// 用于在创建记录前快速检查是否已处理过相同请求
    pub async fn exists_by_idempotency_key(&self, key: &str) -> Result<bool> {
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM auto_benefit_grants WHERE idempotency_key = $1)",
        )
        .bind(key)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    /// 根据 ID 获取自动发放记录
    pub async fn get_by_id(&self, id: i64) -> Result<Option<AutoBenefitGrant>> {
        let row = sqlx::query_as::<_, AutoBenefitGrantRow>(
            r#"
            SELECT id, rule_id, benefit_grant_id, status
            FROM auto_benefit_grants
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| self.map_row_to_grant(r)))
    }

    /// 根据幂等键获取自动发放记录
    pub async fn get_by_idempotency_key(&self, key: &str) -> Result<Option<AutoBenefitGrant>> {
        let row = sqlx::query_as::<_, AutoBenefitGrantRow>(
            r#"
            SELECT id, rule_id, benefit_grant_id, status
            FROM auto_benefit_grants
            WHERE idempotency_key = $1
            "#,
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| self.map_row_to_grant(r)))
    }

    // ==================== 写入操作 ====================

    /// 创建自动发放记录（幂等插入）
    ///
    /// 使用 `ON CONFLICT DO NOTHING` 确保相同幂等键不会重复插入。
    /// 如果记录已存在则返回 None，新建成功则返回记录。
    pub async fn create_grant(&self, grant: &NewAutoBenefitGrant) -> Result<Option<AutoBenefitGrant>> {
        let row = sqlx::query(
            r#"
            INSERT INTO auto_benefit_grants (
                user_id, rule_id, trigger_badge_id, trigger_user_badge_id,
                idempotency_key, status
            )
            VALUES ($1, $2, $3, $4, $5, 'PENDING')
            ON CONFLICT (idempotency_key) DO NOTHING
            RETURNING id, rule_id, benefit_grant_id, status
            "#,
        )
        .bind(&grant.user_id)
        .bind(grant.rule_id)
        .bind(grant.trigger_badge_id)
        .bind(grant.trigger_user_badge_id)
        .bind(&grant.idempotency_key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| AutoBenefitGrant {
            id: r.get("id"),
            rule_id: r.get("rule_id"),
            benefit_grant_id: r.get("benefit_grant_id"),
            status: parse_status(r.get("status")),
        }))
    }

    /// 更新发放状态
    ///
    /// 同时更新状态、关联的权益发放记录 ID、错误信息和完成时间。
    /// 当状态为终态（Success/Failed/Skipped）时自动设置 completed_at。
    pub async fn update_status(
        &self,
        id: i64,
        status: AutoBenefitStatus,
        benefit_grant_id: Option<i64>,
        error_message: Option<&str>,
    ) -> Result<()> {
        // 终态时设置完成时间
        let completed_at = if matches!(
            status,
            AutoBenefitStatus::Success | AutoBenefitStatus::Failed | AutoBenefitStatus::Skipped
        ) {
            Some(Utc::now())
        } else {
            None
        };

        sqlx::query(
            r#"
            UPDATE auto_benefit_grants
            SET status = $2, benefit_grant_id = $3, error_message = $4, completed_at = $5
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(status.to_string())
        .bind(benefit_grant_id)
        .bind(error_message)
        .bind(completed_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ==================== 统计查询 ====================

    /// 统计用户在指定时间范围内的规则发放次数
    ///
    /// 用于频率限制检查。只统计状态为 SUCCESS 的记录。
    /// 如果 `since` 为 None，则统计所有历史记录。
    pub async fn count_user_grants(
        &self,
        user_id: &str,
        rule_id: i64,
        since: Option<DateTime<Utc>>,
    ) -> Result<i64> {
        let count = if let Some(since) = since {
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*)
                FROM auto_benefit_grants
                WHERE user_id = $1 AND rule_id = $2 AND status = 'SUCCESS' AND created_at >= $3
                "#,
            )
            .bind(user_id)
            .bind(rule_id)
            .bind(since)
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*)
                FROM auto_benefit_grants
                WHERE user_id = $1 AND rule_id = $2 AND status = 'SUCCESS'
                "#,
            )
            .bind(user_id)
            .bind(rule_id)
            .fetch_one(&self.pool)
            .await?
        };

        Ok(count)
    }

    /// 列出用户某规则下的所有发放记录
    ///
    /// 按创建时间降序排列，用于查看用户的自动发放历史
    pub async fn list_user_grants(
        &self,
        user_id: &str,
        rule_id: i64,
        limit: i64,
    ) -> Result<Vec<AutoBenefitGrant>> {
        let rows = sqlx::query_as::<_, AutoBenefitGrantRow>(
            r#"
            SELECT id, rule_id, benefit_grant_id, status
            FROM auto_benefit_grants
            WHERE user_id = $1 AND rule_id = $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(user_id)
        .bind(rule_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| self.map_row_to_grant(r)).collect())
    }

    // ==================== 评估日志操作 ====================

    /// 记录评估日志
    ///
    /// 保存每次自动权益评估的执行情况，便于问题排查和性能分析
    pub async fn log_evaluation(&self, log: &AutoBenefitEvaluationLog) -> Result<i64> {
        let id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO auto_benefit_evaluation_logs (
                user_id, trigger_badge_id, evaluation_context,
                rules_evaluated, rules_matched, grants_created, duration_ms
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id
            "#,
        )
        .bind(&log.user_id)
        .bind(log.trigger_badge_id)
        .bind(&log.evaluation_context)
        .bind(log.rules_evaluated)
        .bind(log.rules_matched)
        .bind(log.grants_created)
        .bind(log.duration_ms)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    // ==================== 私有辅助方法 ====================

    /// 将数据库行映射为领域模型
    fn map_row_to_grant(&self, row: AutoBenefitGrantRow) -> AutoBenefitGrant {
        AutoBenefitGrant {
            id: row.id,
            rule_id: row.rule_id,
            benefit_grant_id: row.benefit_grant_id,
            status: parse_status(&row.status),
        }
    }
}

/// 解析状态字符串为枚举
fn parse_status(s: &str) -> AutoBenefitStatus {
    match s {
        "PENDING" => AutoBenefitStatus::Pending,
        "PROCESSING" => AutoBenefitStatus::Processing,
        "SUCCESS" => AutoBenefitStatus::Success,
        "FAILED" => AutoBenefitStatus::Failed,
        "SKIPPED" => AutoBenefitStatus::Skipped,
        _ => AutoBenefitStatus::Pending,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_status() {
        assert!(matches!(parse_status("PENDING"), AutoBenefitStatus::Pending));
        assert!(matches!(parse_status("PROCESSING"), AutoBenefitStatus::Processing));
        assert!(matches!(parse_status("SUCCESS"), AutoBenefitStatus::Success));
        assert!(matches!(parse_status("FAILED"), AutoBenefitStatus::Failed));
        assert!(matches!(parse_status("SKIPPED"), AutoBenefitStatus::Skipped));
        // 未知状态默认为 Pending
        assert!(matches!(parse_status("UNKNOWN"), AutoBenefitStatus::Pending));
    }
}
