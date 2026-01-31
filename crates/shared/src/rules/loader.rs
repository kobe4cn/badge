//! 规则加载器
//!
//! 从数据库加载规则并维护内存映射，支持定时刷新和即时刷新。

use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;
use tokio::sync::watch;
use tokio::time::{interval, timeout};
use tracing::{error, info, warn};

use crate::error::BadgeError;

use super::mapping::RuleBadgeMapping;
use super::models::BadgeGrant;

/// 规则加载器
///
/// 负责从数据库加载规则配置并维护内存缓存。
/// 支持首次加载（阻塞式）和后台定时刷新两种模式。
pub struct RuleLoader {
    db_pool: PgPool,
    service_group: String,
    rule_mapping: Arc<RuleBadgeMapping>,
    refresh_interval: Duration,
    initial_timeout: Duration,
}

impl RuleLoader {
    pub fn new(
        db_pool: PgPool,
        service_group: impl Into<String>,
        rule_mapping: Arc<RuleBadgeMapping>,
        refresh_interval_secs: u64,
        initial_timeout_secs: u64,
    ) -> Self {
        Self {
            db_pool,
            service_group: service_group.into(),
            rule_mapping,
            refresh_interval: Duration::from_secs(refresh_interval_secs),
            initial_timeout: Duration::from_secs(initial_timeout_secs),
        }
    }

    /// 首次加载规则（阻塞，带超时）
    ///
    /// 服务启动时调用，确保规则加载完成后再处理事件。
    /// 若超时或失败，返回错误，服务应终止启动。
    pub async fn initial_load(&self) -> Result<usize, BadgeError> {
        info!(
            service_group = %self.service_group,
            timeout_secs = self.initial_timeout.as_secs(),
            "开始初始加载规则"
        );

        match timeout(self.initial_timeout, self.load_rules_from_db()).await {
            Ok(Ok(count)) => {
                info!(
                    service_group = %self.service_group,
                    rule_count = count,
                    "初始加载规则完成"
                );
                Ok(count)
            }
            Ok(Err(e)) => {
                error!(
                    service_group = %self.service_group,
                    error = %e,
                    "初始加载规则失败"
                );
                Err(e)
            }
            Err(_) => {
                error!(
                    service_group = %self.service_group,
                    timeout_secs = self.initial_timeout.as_secs(),
                    "初始加载规则超时"
                );
                Err(BadgeError::Internal("规则加载超时".to_string()))
            }
        }
    }

    /// 立即刷新规则
    pub async fn reload_now(&self) -> Result<usize, BadgeError> {
        info!(service_group = %self.service_group, "手动触发规则刷新");
        self.load_rules_from_db().await
    }

    /// 启动后台定时刷新任务
    ///
    /// 通过 watch channel 接收关闭信号，实现优雅停机。
    /// 刷新失败时记录警告日志，不会中断后续刷新。
    pub fn start_background_refresh(self: Arc<Self>, mut shutdown: watch::Receiver<bool>) {
        let loader = self.clone();

        tokio::spawn(async move {
            let mut ticker = interval(loader.refresh_interval);

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if let Err(e) = loader.load_rules_from_db().await {
                            warn!(
                                service_group = %loader.service_group,
                                error = %e,
                                "定时刷新规则失败，将在下次重试"
                            );
                        }
                    }
                    _ = shutdown.changed() => {
                        if *shutdown.borrow() {
                            info!(
                                service_group = %loader.service_group,
                                "收到关闭信号，停止规则刷新任务"
                            );
                            break;
                        }
                    }
                }
            }
        });
    }

    /// 从数据库加载规则并更新内存映射
    async fn load_rules_from_db(&self) -> Result<usize, BadgeError> {
        let rules = self.query_active_rules().await?;
        let count = rules.len();

        self.rule_mapping.replace_all(rules);

        info!(
            service_group = %self.service_group,
            rule_count = count,
            event_types = ?self.rule_mapping.event_types(),
            "规则刷新完成"
        );

        Ok(count)
    }

    /// 查询当前有效的规则
    ///
    /// 仅加载属于当前服务组的规则，并过滤掉已过期或未生效的规则。
    async fn query_active_rules(&self) -> Result<Vec<BadgeGrant>, BadgeError> {
        let rows = sqlx::query_as::<_, RuleRow>(
            r#"
            SELECT
                r.id as rule_id,
                r.rule_code,
                r.badge_id,
                b.name as badge_name,
                r.event_type,
                r.start_time,
                r.end_time,
                r.max_count_per_user,
                r.global_quota,
                r.global_granted
            FROM badge_rules r
            JOIN badges b ON r.badge_id = b.id
            JOIN event_types et ON r.event_type = et.code
            WHERE et.service_group = $1
              AND et.enabled = TRUE
              AND r.enabled = TRUE
              AND (r.start_time IS NULL OR r.start_time <= NOW())
              AND (r.end_time IS NULL OR r.end_time > NOW())
            ORDER BY r.id
            "#,
        )
        .bind(&self.service_group)
        .fetch_all(&self.db_pool)
        .await?;

        let rules: Vec<BadgeGrant> = rows
            .into_iter()
            .filter_map(|row| {
                // rule_code 为空时使用 rule_id 生成默认值
                let rule_code = row.rule_code.unwrap_or_else(|| format!("rule_{}", row.rule_id));

                Some(BadgeGrant {
                    rule_id: row.rule_id,
                    rule_code,
                    badge_id: row.badge_id,
                    badge_name: row.badge_name,
                    quantity: 1,
                    event_type: row.event_type?,
                    start_time: row.start_time,
                    end_time: row.end_time,
                    max_count_per_user: row.max_count_per_user,
                    global_quota: row.global_quota,
                    global_granted: row.global_granted,
                })
            })
            .collect();

        Ok(rules)
    }
}

/// 数据库查询结果行
#[derive(sqlx::FromRow)]
struct RuleRow {
    rule_id: i64,
    rule_code: Option<String>,
    badge_id: i64,
    badge_name: String,
    event_type: Option<String>,
    start_time: Option<chrono::DateTime<chrono::Utc>>,
    end_time: Option<chrono::DateTime<chrono::Utc>>,
    max_count_per_user: Option<i32>,
    global_quota: Option<i32>,
    global_granted: i32,
}
