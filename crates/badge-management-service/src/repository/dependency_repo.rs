//! 依赖关系仓储
//!
//! 提供徽章依赖关系和级联评估日志的数据访问

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::error::Result;

/// 依赖关系数据库行
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BadgeDependencyRow {
    pub id: i64,
    pub badge_id: i64,
    pub depends_on_badge_id: i64,
    pub dependency_type: String,
    pub required_quantity: i32,
    pub exclusive_group_id: Option<String>,
    pub auto_trigger: bool,
    pub priority: i32,
    pub dependency_group_id: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建依赖关系请求
#[derive(Debug)]
pub struct CreateDependencyRequest {
    pub badge_id: i64,
    pub depends_on_badge_id: i64,
    pub dependency_type: String,
    pub required_quantity: i32,
    pub exclusive_group_id: Option<String>,
    pub auto_trigger: bool,
    pub priority: i32,
    pub dependency_group_id: String,
}

/// 级联评估日志
#[derive(Debug)]
pub struct CascadeEvaluationLog {
    pub user_id: String,
    pub trigger_badge_id: i64,
    pub evaluation_context: serde_json::Value,
    pub result_status: String,
    pub granted_badges: Option<serde_json::Value>,
    pub blocked_badges: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration_ms: i32,
}

/// 依赖关系仓储
///
/// 负责徽章依赖关系的数据访问，支持前置条件、消耗关系和互斥关系的查询与管理
pub struct DependencyRepository {
    pool: PgPool,
}

impl DependencyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 获取所有启用的依赖关系
    ///
    /// 用于在服务启动时构建完整的依赖图
    pub async fn list_all_enabled(&self) -> Result<Vec<BadgeDependencyRow>> {
        let rows = sqlx::query_as::<_, BadgeDependencyRow>(
            r#"
            SELECT id, badge_id, depends_on_badge_id, dependency_type,
                   required_quantity, exclusive_group_id, auto_trigger,
                   priority, dependency_group_id, enabled, created_at, updated_at
            FROM badge_dependencies
            WHERE enabled = true
            ORDER BY priority ASC, id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// 获取以某徽章为依赖的所有规则（用于级联触发）
    ///
    /// 当用户获得某徽章后，查询哪些徽章可能因此满足条件并自动发放
    pub async fn get_triggered_by(&self, badge_id: i64) -> Result<Vec<BadgeDependencyRow>> {
        let rows = sqlx::query_as::<_, BadgeDependencyRow>(
            r#"
            SELECT id, badge_id, depends_on_badge_id, dependency_type,
                   required_quantity, exclusive_group_id, auto_trigger,
                   priority, dependency_group_id, enabled, created_at, updated_at
            FROM badge_dependencies
            WHERE depends_on_badge_id = $1
              AND auto_trigger = true
              AND enabled = true
            ORDER BY priority ASC, id ASC
            "#,
        )
        .bind(badge_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// 获取某徽章的所有前置条件
    ///
    /// 用于检查用户是否满足获得目标徽章的所有依赖要求
    pub async fn get_prerequisites(&self, badge_id: i64) -> Result<Vec<BadgeDependencyRow>> {
        let rows = sqlx::query_as::<_, BadgeDependencyRow>(
            r#"
            SELECT id, badge_id, depends_on_badge_id, dependency_type,
                   required_quantity, exclusive_group_id, auto_trigger,
                   priority, dependency_group_id, enabled, created_at, updated_at
            FROM badge_dependencies
            WHERE badge_id = $1 AND enabled = true
            ORDER BY dependency_group_id ASC, priority ASC, id ASC
            "#,
        )
        .bind(badge_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// 获取互斥组成员
    ///
    /// 返回同一互斥组内所有徽章的 badge_id，用于检查互斥冲突
    pub async fn get_exclusive_group(&self, group_id: &str) -> Result<Vec<i64>> {
        let rows = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT DISTINCT badge_id
            FROM badge_dependencies
            WHERE exclusive_group_id = $1 AND enabled = true
            "#,
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// 创建依赖关系
    pub async fn create(&self, request: &CreateDependencyRequest) -> Result<BadgeDependencyRow> {
        let row = sqlx::query_as::<_, BadgeDependencyRow>(
            r#"
            INSERT INTO badge_dependencies (
                badge_id, depends_on_badge_id, dependency_type,
                required_quantity, exclusive_group_id, auto_trigger,
                priority, dependency_group_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, badge_id, depends_on_badge_id, dependency_type,
                      required_quantity, exclusive_group_id, auto_trigger,
                      priority, dependency_group_id, enabled, created_at, updated_at
            "#,
        )
        .bind(request.badge_id)
        .bind(request.depends_on_badge_id)
        .bind(&request.dependency_type)
        .bind(request.required_quantity)
        .bind(&request.exclusive_group_id)
        .bind(request.auto_trigger)
        .bind(request.priority)
        .bind(&request.dependency_group_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    /// 删除依赖关系
    ///
    /// 返回是否成功删除（true 表示存在并已删除，false 表示记录不存在）
    pub async fn delete(&self, id: i64) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM badge_dependencies
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 记录级联评估日志
    ///
    /// 用于审计追踪和调试级联评估过程
    pub async fn log_evaluation(&self, log: &CascadeEvaluationLog) -> Result<i64> {
        let id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO cascade_evaluation_logs (
                user_id, trigger_badge_id, evaluation_context,
                result_status, granted_badges, blocked_badges,
                error_message, started_at, completed_at, duration_ms
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id
            "#,
        )
        .bind(&log.user_id)
        .bind(log.trigger_badge_id)
        .bind(&log.evaluation_context)
        .bind(&log.result_status)
        .bind(&log.granted_badges)
        .bind(&log.blocked_badges)
        .bind(&log.error_message)
        .bind(log.started_at)
        .bind(log.completed_at)
        .bind(log.duration_ms)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_repository_creation() {
        // 仅验证类型定义正确，不实际连接数据库
        // 实际集成测试需要配合 testcontainers 或测试数据库
    }
}
