//! 徽章过期处理 Worker
//!
//! 定期扫描即将过期和已过期的用户徽章：
//! 1. 对即将过期的徽章发送提醒通知（提前 N 天）
//! 2. 将已过期的徽章状态变更为 expired
//!
//! 使用 `FOR UPDATE SKIP LOCKED` 保证多实例部署时不会重复处理

use std::time::Duration;

use badge_shared::observability::metrics;
use chrono::{DateTime, Utc};
use serde_json;
use sqlx::PgPool;
use tracing::{error, info};

/// 过期处理 Worker
///
/// 以固定间隔轮询数据库，处理即将过期和已过期的用户徽章。
/// 设计为可在多实例环境中安全运行。
pub struct ExpireWorker {
    pool: PgPool,
    /// 轮询间隔（建议 300 秒）
    poll_interval: Duration,
    /// 每批处理的最大记录数
    batch_size: i64,
    /// 过期提醒提前天数（默认 3 天）
    advance_days: i64,
}

/// 即将过期的徽章记录
#[derive(sqlx::FromRow)]
struct ExpiringBadge {
    id: i64,
    user_id: String,
    badge_id: i64,
    expires_at: DateTime<Utc>,
}

/// 已过期的徽章记录
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct ExpiredBadge {
    id: i64,
    user_id: String,
    badge_id: i64,
    /// 保留用于日志记录和未来扩展
    expires_at: DateTime<Utc>,
}

impl ExpireWorker {
    /// 创建 ExpireWorker 实例
    ///
    /// # 参数
    /// - `pool`: 数据库连接池
    /// - `poll_interval_secs`: 轮询间隔（秒）
    /// - `batch_size`: 每批处理的最大记录数
    /// - `advance_days`: 过期提醒提前天数
    pub fn new(pool: PgPool, poll_interval_secs: u64, batch_size: i64, advance_days: i64) -> Self {
        Self {
            pool,
            poll_interval: Duration::from_secs(poll_interval_secs),
            batch_size,
            advance_days,
        }
    }

    /// 使用默认配置创建 ExpireWorker
    pub fn with_defaults(pool: PgPool) -> Self {
        Self::new(pool, 300, 1000, 3)
    }

    /// 主循环：持续处理过期任务直到进程退出
    pub async fn run(&self) {
        info!(
            poll_interval = ?self.poll_interval,
            batch_size = self.batch_size,
            advance_days = self.advance_days,
            "ExpireWorker 已启动"
        );

        loop {
            // 先处理过期提醒，再处理已过期徽章
            if let Err(e) = self.process_expire_reminders().await {
                error!(error = %e, "处理过期提醒出错");
            }

            if let Err(e) = self.process_expired_badges().await {
                error!(error = %e, "处理已过期徽章出错");
            }

            // 记录 Worker 健康状态
            metrics::set_worker_last_run("expire_worker");

            tokio::time::sleep(self.poll_interval).await;
        }
    }

    /// 处理即将过期的徽章，发送提醒通知
    ///
    /// 查找 N 天内即将过期且尚未发送提醒的活跃徽章
    async fn process_expire_reminders(&self) -> Result<(), sqlx::Error> {
        let remind_before = Utc::now() + chrono::Duration::days(self.advance_days);

        let mut tx = self.pool.begin().await?;

        // 查找需要发送提醒的徽章
        let badges = sqlx::query_as::<_, ExpiringBadge>(
            r#"
            SELECT id, user_id, badge_id, expires_at
            FROM user_badges
            WHERE status = 'active'
              AND expires_at IS NOT NULL
              AND expires_at <= $1
              AND expire_reminded = false
            ORDER BY expires_at ASC
            FOR UPDATE SKIP LOCKED
            LIMIT $2
            "#,
        )
        .bind(remind_before)
        .bind(self.batch_size)
        .fetch_all(&mut *tx)
        .await?;

        if badges.is_empty() {
            tx.rollback().await?;
            return Ok(());
        }

        let badge_ids: Vec<i64> = badges.iter().map(|b| b.id).collect();
        let count = badge_ids.len();

        info!(count, "发现即将过期的徽章，准备发送提醒");

        // 批量标记为已提醒
        sqlx::query(
            r#"
            UPDATE user_badges
            SET expire_reminded = true, updated_at = NOW()
            WHERE id = ANY($1)
            "#,
        )
        .bind(&badge_ids)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        // 异步发送通知（实际实现需要对接通知服务）
        for badge in badges {
            self.send_expire_reminder(&badge).await;
        }

        // 记录过期提醒指标
        metrics::record_expire_reminder(count as u64);

        info!(count, "过期提醒处理完成");
        Ok(())
    }

    /// 发送过期提醒通知
    ///
    /// 创建通知任务，由通知 Worker 异步处理发送
    async fn send_expire_reminder(&self, badge: &ExpiringBadge) {
        let days_left = (badge.expires_at - Utc::now()).num_days();
        info!(
            user_id = %badge.user_id,
            badge_id = badge.badge_id,
            expires_at = %badge.expires_at,
            days_left,
            "发送徽章过期提醒"
        );

        // 查询通知配置，创建通知任务
        if let Err(e) = self.create_expire_notification(badge, days_left).await {
            error!(
                user_id = %badge.user_id,
                badge_id = badge.badge_id,
                error = %e,
                "创建过期提醒通知任务失败"
            );
        }
    }

    /// 创建过期提醒通知任务
    ///
    /// 根据徽章的通知配置，向 notification_tasks 表插入待处理任务
    async fn create_expire_notification(
        &self,
        badge: &ExpiringBadge,
        days_left: i64,
    ) -> Result<(), sqlx::Error> {
        // 查询该徽章是否有过期提醒通知配置
        let config_exists: (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM notification_configs
                WHERE badge_id = $1
                  AND trigger_type = 'badge_expire_remind'
                  AND status = 'active'
            )
            "#,
        )
        .bind(badge.badge_id)
        .fetch_one(&self.pool)
        .await?;

        if !config_exists.0 {
            // 没有配置过期提醒通知，跳过
            return Ok(());
        }

        // 获取徽章名称用于通知内容
        let badge_name: Option<String> =
            sqlx::query_scalar("SELECT name FROM badges WHERE id = $1")
                .bind(badge.badge_id)
                .fetch_optional(&self.pool)
                .await?;

        // 构建模板参数
        let template_params = serde_json::json!({
            "badge_name": badge_name.unwrap_or_else(|| "徽章".to_string()),
            "expires_at": badge.expires_at.to_rfc3339(),
            "days_remaining": days_left
        });

        // 创建通知任务
        sqlx::query(
            r#"
            INSERT INTO notification_tasks (
                user_id, trigger_type, template_params, status, created_at
            )
            SELECT
                $1,
                'badge_expire_remind',
                $2,
                'pending',
                NOW()
            FROM notification_configs nc
            WHERE nc.badge_id = $3
              AND nc.trigger_type = 'badge_expire_remind'
              AND nc.status = 'active'
            "#,
        )
        .bind(&badge.user_id)
        .bind(&template_params)
        .bind(badge.badge_id)
        .execute(&self.pool)
        .await?;

        info!(
            user_id = %badge.user_id,
            badge_id = badge.badge_id,
            "过期提醒通知任务已创建"
        );

        Ok(())
    }

    /// 处理已过期的徽章
    ///
    /// 将过期徽章状态变更为 expired，并记录流水
    async fn process_expired_badges(&self) -> Result<(), sqlx::Error> {
        let now = Utc::now();

        let mut tx = self.pool.begin().await?;

        // 查找已过期的活跃徽章
        let badges = sqlx::query_as::<_, ExpiredBadge>(
            r#"
            SELECT id, user_id, badge_id, expires_at
            FROM user_badges
            WHERE status = 'active'
              AND expires_at IS NOT NULL
              AND expires_at <= $1
            ORDER BY expires_at ASC
            FOR UPDATE SKIP LOCKED
            LIMIT $2
            "#,
        )
        .bind(now)
        .bind(self.batch_size)
        .fetch_all(&mut *tx)
        .await?;

        if badges.is_empty() {
            tx.rollback().await?;
            return Ok(());
        }

        let count = badges.len();
        info!(count, "发现已过期的徽章，准备处理");

        for badge in &badges {
            // 更新徽章状态为 expired
            sqlx::query(
                r#"
                UPDATE user_badges
                SET status = 'expired', expired_at = NOW(), updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(badge.id)
            .execute(&mut *tx)
            .await?;

            // 减少用户徽章计数
            sqlx::query(
                r#"
                UPDATE user_badge_counts
                SET quantity = quantity - 1, updated_at = NOW()
                WHERE user_id = $1 AND badge_id = $2 AND quantity > 0
                "#,
            )
            .bind(&badge.user_id)
            .bind(badge.badge_id)
            .execute(&mut *tx)
            .await?;

            // 记录流水日志
            sqlx::query(
                r#"
                INSERT INTO user_badge_ledger (user_id, badge_id, change_type, source_type, quantity, remark, created_at)
                VALUES ($1, $2, 'decrease', 'expire', 1, '徽章到期自动过期', NOW())
                "#,
            )
            .bind(&badge.user_id)
            .bind(badge.badge_id)
            .execute(&mut *tx)
            .await?;

            info!(
                user_badge_id = badge.id,
                user_id = %badge.user_id,
                badge_id = badge.badge_id,
                "徽章已过期处理完成"
            );
        }

        tx.commit().await?;

        // 记录过期处理指标
        metrics::record_badge_expiration(count as u64);

        info!(count, "已过期徽章处理完成");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_expire_worker_creation() {
        // 验证默认配置
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test").unwrap();
        let worker = ExpireWorker::with_defaults(pool);

        assert_eq!(worker.poll_interval.as_secs(), 300);
        assert_eq!(worker.batch_size, 1000);
        assert_eq!(worker.advance_days, 3);
    }

    #[tokio::test]
    async fn test_expire_worker_custom_config() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test").unwrap();
        let worker = ExpireWorker::new(pool, 60, 500, 7);

        assert_eq!(worker.poll_interval.as_secs(), 60);
        assert_eq!(worker.batch_size, 500);
        assert_eq!(worker.advance_days, 7);
    }
}
