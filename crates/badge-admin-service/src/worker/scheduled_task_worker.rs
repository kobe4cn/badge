//! 定时任务调度 Worker
//!
//! 负责检查定时任务和周期任务，在到达执行时间时将其状态改为 pending，
//! 由 BatchTaskWorker 执行实际处理。支持三种调度模式：
//! - immediate: 立即执行（默认行为，直接进入 pending 状态）
//! - once: 定时单次执行，到达 scheduled_at 时间后执行一次
//! - recurring: 周期执行，按 cron 表达式周期性触发

use std::str::FromStr;
use std::time::Duration;

use badge_shared::observability::metrics;
use chrono::{DateTime, Utc};
use cron::Schedule;
use sqlx::PgPool;
use tracing::{error, info, warn};

/// 定时任务调度 Worker
///
/// 轮询检查定时任务表，将到期的任务标记为 pending 状态。
/// 对于周期任务，执行后计算下次执行时间。
pub struct ScheduledTaskWorker {
    pool: PgPool,
    poll_interval: Duration,
}

/// 定时任务记录
///
/// 部分字段仅用于日志记录，sqlx 自动解析但不直接使用
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct ScheduledTask {
    id: i64,
    schedule_type: Option<String>,
    scheduled_at: Option<DateTime<Utc>>,
    cron_expression: Option<String>,
    next_run_at: Option<DateTime<Utc>>,
}

impl ScheduledTaskWorker {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            poll_interval: Duration::from_secs(30),
        }
    }

    /// 创建带自定义配置的 Worker（主要用于测试）
    #[allow(dead_code)]
    pub fn with_config(pool: PgPool, poll_secs: u64) -> Self {
        Self {
            pool,
            poll_interval: Duration::from_secs(poll_secs),
        }
    }

    /// 主循环：持续检查定时任务直到进程退出
    pub async fn run(&self) {
        info!(
            poll_interval = ?self.poll_interval,
            "ScheduledTaskWorker 已启动"
        );
        loop {
            // 处理定时单次任务
            if let Err(e) = self.process_once_tasks().await {
                error!(error = %e, "定时任务处理出错");
            }

            // 处理周期任务
            if let Err(e) = self.process_recurring_tasks().await {
                error!(error = %e, "周期任务处理出错");
            }

            // 记录 Worker 健康状态，供 Prometheus 告警判断 Worker 是否存活
            metrics::set_worker_last_run("scheduled_task_worker");

            tokio::time::sleep(self.poll_interval).await;
        }
    }

    /// 处理定时单次任务（schedule_type = 'once'）
    ///
    /// 找出已到期的定时任务，将其状态从 scheduled 改为 pending，
    /// 让 BatchTaskWorker 执行实际处理。
    /// 使用显式事务包裹 `FOR UPDATE SKIP LOCKED`，确保多实例部署时的互斥正确性。
    async fn process_once_tasks(&self) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        let mut tx = self.pool.begin().await?;

        let tasks = sqlx::query_as::<_, ScheduledTask>(
            r#"
            SELECT id, schedule_type, scheduled_at, cron_expression, next_run_at
            FROM batch_tasks
            WHERE schedule_type = 'once'
              AND status = 'scheduled'
              AND scheduled_at <= $1
            ORDER BY scheduled_at ASC
            LIMIT 10
            FOR UPDATE SKIP LOCKED
            "#,
        )
        .bind(now)
        .fetch_all(&mut *tx)
        .await?;

        if tasks.is_empty() {
            tx.rollback().await?;
            return Ok(());
        }

        info!(count = tasks.len(), "发现到期的定时任务");

        for task in &tasks {
            let result = sqlx::query(
                r#"
                UPDATE batch_tasks
                SET status = 'pending', updated_at = NOW()
                WHERE id = $1 AND status = 'scheduled'
                "#,
            )
            .bind(task.id)
            .execute(&mut *tx)
            .await;

            match result {
                Ok(r) if r.rows_affected() > 0 => {
                    info!(task_id = task.id, "定时任务已触发");
                }
                Ok(_) => {
                    warn!(task_id = task.id, "定时任务状态已被其他实例修改");
                }
                Err(e) => {
                    error!(task_id = task.id, error = %e, "触发定时任务失败");
                }
            }
        }

        tx.commit().await?;
        Ok(())
    }

    /// 处理周期任务（schedule_type = 'recurring'）
    ///
    /// 找出 next_run_at 已到期的周期任务，执行后计算下次执行时间。
    /// 周期任务会创建一个新的任务记录来执行，原记录保持 recurring 状态。
    /// 查询使用显式事务 + `FOR UPDATE SKIP LOCKED` 保证多实例安全。
    async fn process_recurring_tasks(&self) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        let mut tx = self.pool.begin().await?;

        let tasks = sqlx::query_as::<_, ScheduledTask>(
            r#"
            SELECT id, schedule_type, scheduled_at, cron_expression, next_run_at
            FROM batch_tasks
            WHERE schedule_type = 'recurring'
              AND status = 'active'
              AND next_run_at <= $1
            ORDER BY next_run_at ASC
            LIMIT 10
            FOR UPDATE SKIP LOCKED
            "#,
        )
        .bind(now)
        .fetch_all(&mut *tx)
        .await?;

        if tasks.is_empty() {
            tx.rollback().await?;
            return Ok(());
        }

        info!(count = tasks.len(), "发现到期的周期任务");

        for task in &tasks {
            let next_run = task
                .cron_expression
                .as_ref()
                .and_then(|expr| self.calculate_next_run(expr, now));

            match next_run {
                Some(next) => {
                    // 1. 在事务内更新 next_run_at，防止其他实例重复触发
                    if let Err(e) = sqlx::query(
                        r#"
                        UPDATE batch_tasks
                        SET next_run_at = $2, updated_at = NOW()
                        WHERE id = $1
                        "#,
                    )
                    .bind(task.id)
                    .bind(next)
                    .execute(&mut *tx)
                    .await
                    {
                        error!(task_id = task.id, error = %e, "更新周期任务下次执行时间失败");
                        continue;
                    }

                    // 2. 在事务内创建子任务，确保原子性
                    match self.create_child_task_in_tx(&mut tx, task.id).await {
                        Ok(child_id) => {
                            info!(
                                parent_task_id = task.id,
                                child_task_id = child_id,
                                next_run = %next,
                                "周期任务已触发"
                            );
                        }
                        Err(e) => {
                            error!(task_id = task.id, error = %e, "创建周期任务子任务失败");
                        }
                    }
                }
                None => {
                    warn!(
                        task_id = task.id,
                        cron = task.cron_expression,
                        "无法计算周期任务的下次执行时间，任务已标记为无效"
                    );
                    let _ = sqlx::query(
                        r#"
                        UPDATE batch_tasks
                        SET status = 'invalid', error_message = '无效的 cron 表达式或无下次执行时间', updated_at = NOW()
                        WHERE id = $1
                        "#,
                    )
                    .bind(task.id)
                    .execute(&mut *tx)
                    .await;
                }
            }
        }

        tx.commit().await?;
        Ok(())
    }

    /// 在已有事务内为周期任务创建子任务
    ///
    /// 与 next_run_at 更新在同一事务中执行，保证不会出现
    /// "next_run_at 已更新但子任务未创建"的中间状态。
    async fn create_child_task_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        parent_task_id: i64,
    ) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            r#"
            INSERT INTO batch_tasks (
                name, task_type, badge_id, quantity, params, file_url, reason,
                status, schedule_type, parent_task_id, created_by, created_at, updated_at
            )
            SELECT
                name || ' (周期执行 ' || NOW()::date || ')',
                task_type,
                badge_id,
                quantity,
                params,
                file_url,
                reason,
                'pending',
                'immediate',
                id,
                created_by,
                NOW(),
                NOW()
            FROM batch_tasks
            WHERE id = $1
            RETURNING id
            "#,
        )
        .bind(parent_task_id)
        .fetch_one(&mut **tx)
        .await?;

        Ok(row.0)
    }

    /// 根据 cron 表达式计算下次执行时间
    ///
    /// 使用 cron crate 解析表达式，找到当前时间之后的第一个执行时间点。
    fn calculate_next_run(&self, cron_expression: &str, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let schedule = match Schedule::from_str(cron_expression) {
            Ok(s) => s,
            Err(e) => {
                error!(cron = cron_expression, error = %e, "无效的 cron 表达式");
                return None;
            }
        };

        // 获取 after 之后的下一个执行时间
        schedule.after(&after).next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 独立测试 cron 表达式解析，不依赖 worker
    fn parse_and_get_next(cron_expression: &str, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let schedule = Schedule::from_str(cron_expression).ok()?;
        schedule.after(&after).next()
    }

    #[test]
    fn test_calculate_next_run() {
        // 测试每小时执行
        let now = Utc::now();
        let next = parse_and_get_next("0 0 * * * *", now);
        assert!(next.is_some());
        assert!(next.unwrap() > now);

        // 测试无效表达式
        let invalid = parse_and_get_next("invalid cron", now);
        assert!(invalid.is_none());
    }

    #[test]
    fn test_cron_parsing() {
        // 测试常用的 cron 表达式格式
        let expressions = [
            "0 0 * * * *",      // 每小时整点
            "0 30 * * * *",     // 每小时30分
            "0 0 9 * * *",      // 每天9点
            "0 0 9 * * 1-5",    // 工作日9点
            "0 0 0 1 * *",      // 每月1号0点
        ];

        for expr in expressions {
            let schedule: Result<Schedule, _> = Schedule::from_str(expr);
            assert!(schedule.is_ok(), "应该能解析: {}", expr);
        }
    }
}
