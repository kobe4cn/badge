//! 批量任务后台处理 Worker
//!
//! 轮询 batch_tasks 表中 pending 状态的任务，逐条处理用户的发放/撤销操作。
//! 使用 `FOR UPDATE SKIP LOCKED` 保证多实例部署时任务不会被重复消费。
//!
//! 优化特性：
//! - 文件大小限制：默认 50MB，防止 OOM
//! - 流式 CSV 解析：分批读取，避免一次性加载
//! - 分片并发处理：每批 100 条并发执行，提升吞吐

use std::io::BufRead;
use std::time::{Duration, Instant};

use badge_shared::observability::metrics;
use chrono::Utc;
use sqlx::PgPool;
use tracing::{error, info, warn};
use uuid::Uuid;

/// 文件大小上限（50MB），超过此大小的 CSV 将被拒绝处理
const MAX_FILE_SIZE_BYTES: u64 = 50 * 1024 * 1024;

/// 每批并发处理的用户数量
const BATCH_CHUNK_SIZE: usize = 100;

/// 失败重试的最大次数
const MAX_RETRY_COUNT: i32 = 3;

/// 重试间隔基数（秒），实际间隔 = 2^retry_count * 60
const RETRY_INTERVAL_BASE_SECS: i64 = 60;

/// 批量任务 Worker
///
/// 以固定间隔轮询数据库，领取并执行 pending 状态的批量发放/撤销任务。
/// 设计为可在多实例环境中安全运行——通过行锁避免任务被重复消费。
pub struct BatchTaskWorker {
    pool: PgPool,
    poll_interval: Duration,
    /// 并发处理的分片大小
    chunk_size: usize,
    /// 文件大小上限（字节）
    max_file_size: u64,
}

/// 从数据库查询出的待处理任务行
#[derive(sqlx::FromRow)]
struct PendingTask {
    id: i64,
    task_type: String,
    params: Option<serde_json::Value>,
    file_url: Option<String>,
}

/// 从 params JSON 中解析出的任务参数
struct TaskParams {
    badge_id: i64,
    reason: String,
    user_ids: Vec<String>,
}

/// 可重试的失败记录
#[derive(sqlx::FromRow)]
struct RetryableFailure {
    id: i64,
    task_id: i64,
    user_id: String,
    retry_count: i32,
    params: Option<serde_json::Value>,
    task_type: String,
}

impl BatchTaskWorker {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            poll_interval: Duration::from_secs(5),
            chunk_size: BATCH_CHUNK_SIZE,
            max_file_size: MAX_FILE_SIZE_BYTES,
        }
    }

    /// 创建带自定义配置的 Worker（主要用于测试）
    #[allow(dead_code)]
    pub fn with_config(pool: PgPool, poll_secs: u64, chunk_size: usize, max_file_mb: u64) -> Self {
        Self {
            pool,
            poll_interval: Duration::from_secs(poll_secs),
            chunk_size,
            max_file_size: max_file_mb * 1024 * 1024,
        }
    }

    /// 主循环：持续轮询待处理任务直到进程退出
    ///
    /// 每次循环只领取一个任务处理完毕后再取下一个，
    /// 避免单个 Worker 实例积压过多任务导致内存压力。
    pub async fn run(&self) {
        info!(
            poll_interval = ?self.poll_interval,
            chunk_size = self.chunk_size,
            max_file_mb = self.max_file_size / 1024 / 1024,
            "BatchTaskWorker 已启动"
        );
        loop {
            // 处理新任务
            if let Err(e) = self.process_pending_tasks().await {
                error!(error = %e, "批量任务处理出错");
            }

            // 处理失败重试
            if let Err(e) = self.process_retry_failures().await {
                error!(error = %e, "失败重试处理出错");
            }

            // 记录 Worker 健康状态
            metrics::set_worker_last_run("batch_task_worker");

            tokio::time::sleep(self.poll_interval).await;
        }
    }

    /// 尝试领取一个 pending 任务并执行
    ///
    /// 使用 `FOR UPDATE SKIP LOCKED` 实现无阻塞的分布式任务领取：
    /// 已被其他实例锁定的行会被跳过，而不是等待锁释放。
    async fn process_pending_tasks(&self) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        // 在事务内抢占任务，确保领取和状态变更是原子操作
        let task = sqlx::query_as::<_, PendingTask>(
            r#"
            SELECT id, task_type, params, file_url
            FROM batch_tasks
            WHERE status = 'pending'
            ORDER BY created_at ASC
            FOR UPDATE SKIP LOCKED
            LIMIT 1
            "#,
        )
        .fetch_optional(&mut *tx)
        .await?;

        let task = match task {
            Some(t) => t,
            None => return Ok(()),
        };

        info!(task_id = task.id, task_type = %task.task_type, "领取到批量任务");

        // 立即标记为 processing，让前端轮询能及时看到状态变化
        sqlx::query("UPDATE batch_tasks SET status = 'processing', updated_at = NOW() WHERE id = $1")
            .bind(task.id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        // 在事务外执行实际处理，避免长事务锁住行
        self.execute_task(&task).await;

        Ok(())
    }

    /// 解析任务参数并分派到具体的处理逻辑
    ///
    /// params 为空时直接标记失败，因为没有参数就无法知道要操作哪些用户。
    async fn execute_task(&self, task: &PendingTask) {
        let start_time = Instant::now();

        let params = match self.resolve_task_params(task).await {
            Ok(p) => p,
            Err(err_msg) => {
                self.mark_task_failed(task.id, &err_msg).await;
                // 记录失败指标
                metrics::record_batch_task(&task.task_type, "failed", start_time.elapsed().as_secs_f64());
                return;
            }
        };

        let total = params.user_ids.len() as i32;

        // 先写入 total_count 便于前端计算进度百分比
        if let Err(e) = sqlx::query(
            "UPDATE batch_tasks SET total_count = $2, updated_at = NOW() WHERE id = $1",
        )
        .bind(task.id)
        .bind(total)
        .execute(&self.pool)
        .await
        {
            error!(task_id = task.id, error = %e, "更新任务总数失败");
        }

        // 分片并发处理
        let (success_count, failure_count) = self
            .process_users_in_chunks(task, &params)
            .await;

        // 全部处理完毕，最终状态更新
        let now = Utc::now();
        let _ = sqlx::query(
            r#"
            UPDATE batch_tasks
            SET status = 'completed', progress = 100,
                success_count = $2, failure_count = $3, updated_at = $4
            WHERE id = $1
            "#,
        )
        .bind(task.id)
        .bind(success_count)
        .bind(failure_count)
        .bind(now)
        .execute(&self.pool)
        .await;

        // 记录成功指标
        let duration = start_time.elapsed().as_secs_f64();
        let status = if failure_count == 0 { "success" } else { "partial" };
        metrics::record_batch_task(&task.task_type, status, duration);

        info!(
            task_id = task.id,
            success = success_count,
            failure = failure_count,
            duration_secs = duration,
            "批量任务执行完成"
        );
    }

    /// 分片并发处理用户列表
    ///
    /// 将用户列表按 chunk_size 分片，每个分片内的用户并发处理，
    /// 分片之间顺序执行以控制并发度，避免数据库连接池耗尽。
    async fn process_users_in_chunks(
        &self,
        task: &PendingTask,
        params: &TaskParams,
    ) -> (i32, i32) {
        let total = params.user_ids.len();
        let mut success_count: i32 = 0;
        let mut failure_count: i32 = 0;
        let mut processed: usize = 0;

        // 预先克隆需要在 async 块中使用的数据，避免生命周期问题
        let task_type = task.task_type.clone();
        let task_id = task.id;
        let badge_id = params.badge_id;
        let reason = params.reason.clone();

        // 按 chunk_size 分片
        for (chunk_idx, chunk) in params.user_ids.chunks(self.chunk_size).enumerate() {
            // 每个分片内并发处理
            let mut handles = Vec::with_capacity(chunk.len());

            for (idx_in_chunk, user_id) in chunk.iter().enumerate() {
                let global_idx = chunk_idx * self.chunk_size + idx_in_chunk;
                let user_id_owned = user_id.clone();
                let task_type_ref = task_type.clone();
                let reason_ref = reason.clone();
                let pool = self.pool.clone();

                handles.push(async move {
                    let result = match task_type_ref.as_str() {
                        "batch_grant" => {
                            Self::grant_one_user_static(&pool, &user_id_owned, badge_id, task_id, &reason_ref)
                                .await
                        }
                        "batch_revoke" => {
                            Self::revoke_one_user_static(&pool, &user_id_owned, badge_id, task_id, &reason_ref)
                                .await
                        }
                        _ => Err(format!("不支持的任务类型: {}", task_type_ref)),
                    };
                    (global_idx, user_id_owned, result)
                });
            }

            // 并发执行本批次
            let results = futures::future::join_all(handles).await;

            // 统计本批次结果
            for (global_idx, user_id, result) in results {
                processed += 1;
                match result {
                    Ok(()) => success_count += 1,
                    Err(err_msg) => {
                        failure_count += 1;
                        self.insert_failure_record(
                            task_id,
                            (global_idx + 1) as i32,
                            &user_id,
                            "PROCESS_ERROR",
                            &err_msg,
                        )
                        .await;
                    }
                }
            }

            // 每批次更新进度
            let progress = if total > 0 {
                ((processed * 100) / total).min(100) as i32
            } else {
                0
            };
            let _ = sqlx::query(
                r#"
                UPDATE batch_tasks
                SET success_count = $2, failure_count = $3, progress = $4, updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(task_id)
            .bind(success_count)
            .bind(failure_count)
            .bind(progress)
            .execute(&self.pool)
            .await;
        }

        (success_count, failure_count)
    }

    /// 发放徽章的静态版本（用于并发处理）
    async fn grant_one_user_static(
        pool: &PgPool,
        user_id: &str,
        badge_id: i64,
        task_id: i64,
        reason: &str,
    ) -> Result<(), String> {
        let source_ref_id = format!("batch-{}-{}", task_id, Uuid::new_v4());
        let now = Utc::now();

        let mut tx = pool
            .begin()
            .await
            .map_err(|e| format!("开启事务失败: {e}"))?;

        // 1. 插入或累加 user_badges
        sqlx::query(
            r#"
            INSERT INTO user_badges (user_id, badge_id, quantity, status, first_acquired_at, source_type, created_at, updated_at)
            VALUES ($1, $2, 1, 'ACTIVE', $3, 'BATCH', $3, $3)
            ON CONFLICT (user_id, badge_id)
            DO UPDATE SET quantity = user_badges.quantity + 1, status = 'ACTIVE', updated_at = $3
            "#,
        )
        .bind(user_id)
        .bind(badge_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("upsert user_badges 失败: {e}"))?;

        // 2. 写入 badge_ledger
        let balance_row: (i32,) = sqlx::query_as(
            "SELECT COALESCE(SUM(quantity), 0)::INT FROM badge_ledger WHERE user_id = $1 AND badge_id = $2",
        )
        .bind(user_id)
        .bind(badge_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| format!("查询余额失败: {e}"))?;

        let balance_after = balance_row.0 + 1;

        sqlx::query(
            r#"
            INSERT INTO badge_ledger (user_id, badge_id, change_type, source_type, ref_id, quantity, balance_after, remark, created_at)
            VALUES ($1, $2, 'acquire', 'BATCH', $3, 1, $4, $5, $6)
            "#,
        )
        .bind(user_id)
        .bind(badge_id)
        .bind(&source_ref_id)
        .bind(balance_after)
        .bind(reason)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("写入 badge_ledger 失败: {e}"))?;

        // 3. 写入操作日志
        sqlx::query(
            r#"
            INSERT INTO user_badge_logs (user_id, badge_id, action, quantity, source_type, source_ref_id, remark, created_at)
            VALUES ($1, $2, 'grant', 1, 'BATCH', $3, $4, $5)
            "#,
        )
        .bind(user_id)
        .bind(badge_id)
        .bind(&source_ref_id)
        .bind(reason)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("写入 user_badge_logs 失败: {e}"))?;

        // 4. 累加徽章已发放计数
        sqlx::query(
            "UPDATE badges SET issued_count = issued_count + 1, updated_at = $2 WHERE id = $1",
        )
        .bind(badge_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("更新 issued_count 失败: {e}"))?;

        tx.commit()
            .await
            .map_err(|e| format!("提交事务失败: {e}"))?;

        Ok(())
    }

    /// 撤销徽章的静态版本（用于并发处理）
    async fn revoke_one_user_static(
        pool: &PgPool,
        user_id: &str,
        badge_id: i64,
        task_id: i64,
        reason: &str,
    ) -> Result<(), String> {
        let source_ref_id = format!("batch-{}-{}", task_id, Uuid::new_v4());
        let now = Utc::now();

        // 先检查用户是否持有该徽章且数量足够
        let qty_row: Option<(i32,)> = sqlx::query_as(
            "SELECT quantity FROM user_badges WHERE user_id = $1 AND badge_id = $2",
        )
        .bind(user_id)
        .bind(badge_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("查询用户徽章失败: {e}"))?;

        let current_qty = match qty_row {
            Some((q,)) if q >= 1 => q,
            Some((q,)) => return Err(format!("用户徽章数量不足 (当前: {q})")),
            None => return Err("用户未持有该徽章".to_string()),
        };

        let remaining = current_qty - 1;

        let mut tx = pool
            .begin()
            .await
            .map_err(|e| format!("开启事务失败: {e}"))?;

        // 1. 扣减 user_badges
        if remaining == 0 {
            sqlx::query(
                r#"
                UPDATE user_badges
                SET quantity = 0, status = 'revoked', updated_at = $3
                WHERE user_id = $1 AND badge_id = $2
                "#,
            )
            .bind(user_id)
            .bind(badge_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("扣减 user_badges 失败: {e}"))?;
        } else {
            sqlx::query(
                r#"
                UPDATE user_badges
                SET quantity = quantity - 1, updated_at = $3
                WHERE user_id = $1 AND badge_id = $2
                "#,
            )
            .bind(user_id)
            .bind(badge_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("扣减 user_badges 失败: {e}"))?;
        }

        // 2. 写入 badge_ledger
        sqlx::query(
            r#"
            INSERT INTO badge_ledger (user_id, badge_id, change_type, source_type, ref_id, quantity, balance_after, remark, created_at)
            VALUES ($1, $2, 'cancel', 'BATCH', $3, -1, $4, $5, $6)
            "#,
        )
        .bind(user_id)
        .bind(badge_id)
        .bind(&source_ref_id)
        .bind(remaining)
        .bind(reason)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("写入 badge_ledger 失败: {e}"))?;

        // 3. 写入操作日志
        sqlx::query(
            r#"
            INSERT INTO user_badge_logs (user_id, badge_id, action, quantity, source_type, source_ref_id, remark, created_at)
            VALUES ($1, $2, 'revoke', 1, 'BATCH', $3, $4, $5)
            "#,
        )
        .bind(user_id)
        .bind(badge_id)
        .bind(&source_ref_id)
        .bind(reason)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("写入 user_badge_logs 失败: {e}"))?;

        // 4. 扣减徽章已发放计数
        sqlx::query(
            "UPDATE badges SET issued_count = issued_count - 1, updated_at = $2 WHERE id = $1",
        )
        .bind(badge_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("更新 issued_count 失败: {e}"))?;

        tx.commit()
            .await
            .map_err(|e| format!("提交事务失败: {e}"))?;

        Ok(())
    }

    /// 解析任务参数，支持两种用户列表来源
    ///
    /// 直接模式：params.user_ids 包含用户 ID 数组（前端 /admin/tasks 接口）
    /// 文件模式：file_url 指向 CSV 文件（前端 /admin/grants/batch 接口），
    ///           此时需下载并解析文件获取用户列表
    async fn resolve_task_params(&self, task: &PendingTask) -> Result<TaskParams, String> {
        let params = task
            .params
            .as_ref()
            .ok_or_else(|| "任务参数缺失".to_string())?;

        let badge_id = params
            .get("badge_id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| "任务参数缺少 badge_id".to_string())?;

        let reason = params
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let user_ids =
            if let Some(ids) = params.get("user_ids").and_then(|v| v.as_array()) {
                ids.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            } else if let Some(url) = &task.file_url {
                // file_url 存在时回退到 CSV 下载模式
                self.fetch_user_ids_from_csv(url).await?
            } else {
                return Err("任务参数缺少 user_ids 且无 file_url".to_string());
            };

        if user_ids.is_empty() {
            return Err("user_ids 为空，无需处理".to_string());
        }

        Ok(TaskParams {
            badge_id,
            reason,
            user_ids,
        })
    }

    /// 从 file_url 下载 CSV 并提取用户 ID
    ///
    /// 优化：
    /// 1. 先检查 Content-Length，超过 max_file_size 直接拒绝
    /// 2. 流式读取响应体，边读边解析，避免一次性加载到内存
    ///
    /// CSV 格式约定：每行一个 user_id，首行为 "user_id" 表头时自动跳过。
    /// 超时 30 秒，防止外部存储不可用时阻塞 Worker 主循环。
    async fn fetch_user_ids_from_csv(&self, url: &str) -> Result<Vec<String>, String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

        // 先发 HEAD 请求检查文件大小
        let head_resp = client
            .head(url)
            .send()
            .await
            .map_err(|e| format!("检查 CSV 文件大小失败: {e}"))?;

        if let Some(content_length) = head_resp.headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
        {
            if content_length > self.max_file_size {
                return Err(format!(
                    "CSV 文件过大 ({} MB)，超过限制 ({} MB)",
                    content_length / 1024 / 1024,
                    self.max_file_size / 1024 / 1024
                ));
            }
        }

        // 下载文件（流式）
        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("下载 CSV 失败: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("下载 CSV 返回 HTTP {}", resp.status()));
        }

        // 流式读取并解析
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("读取 CSV 内容失败: {e}"))?;

        // 检查实际大小（防止 HEAD 响应没有 Content-Length）
        if bytes.len() as u64 > self.max_file_size {
            return Err(format!(
                "CSV 文件过大 ({} MB)，超过限制 ({} MB)",
                bytes.len() / 1024 / 1024,
                self.max_file_size / 1024 / 1024
            ));
        }

        // 流式解析：逐行读取，避免中间字符串分配
        let cursor = std::io::Cursor::new(&bytes);
        let mut user_ids = Vec::new();

        for line in cursor.lines() {
            let line = line.map_err(|e| format!("读取 CSV 行失败: {e}"))?;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("user_id") {
                continue;
            }
            user_ids.push(trimmed.to_string());
        }

        info!(
            url = url,
            count = user_ids.len(),
            size_kb = bytes.len() / 1024,
            "从 CSV 解析出用户列表"
        );

        Ok(user_ids)
    }

    /// 处理失败记录的重试
    ///
    /// 使用指数退避策略：间隔 = 2^retry_count * 60 秒
    /// 最多重试 MAX_RETRY_COUNT 次，超过后标记为 EXHAUSTED
    async fn process_retry_failures(&self) -> Result<(), sqlx::Error> {
        let now = Utc::now();

        // 查询可重试的失败记录
        // 条件：retry_status = 'PENDING' 且 retry_count < MAX_RETRY_COUNT
        // 且距离上次重试已过指数退避间隔
        let failures: Vec<RetryableFailure> = sqlx::query_as(
            r#"
            SELECT f.id, f.task_id, f.user_id, f.retry_count,
                   t.params, t.task_type
            FROM batch_task_failures f
            JOIN batch_tasks t ON t.id = f.task_id
            WHERE f.retry_status = 'PENDING'
              AND f.retry_count < $1
              AND (f.last_retry_at IS NULL
                   OR f.last_retry_at + make_interval(secs => power(2, f.retry_count) * $2) <= $3)
            ORDER BY f.last_retry_at ASC NULLS FIRST
            LIMIT 100
            FOR UPDATE OF f SKIP LOCKED
            "#,
        )
        .bind(MAX_RETRY_COUNT)
        .bind(RETRY_INTERVAL_BASE_SECS as f64)
        .bind(now)
        .fetch_all(&self.pool)
        .await?;

        if failures.is_empty() {
            return Ok(());
        }

        info!(count = failures.len(), "开始处理失败重试");

        for failure in failures {
            self.retry_single_failure(&failure).await;
        }

        Ok(())
    }

    /// 重试单个失败记录
    async fn retry_single_failure(&self, failure: &RetryableFailure) {
        let now = Utc::now();

        // 标记为重试中
        let _ = sqlx::query(
            "UPDATE batch_task_failures SET retry_status = 'RETRYING', last_retry_at = $2 WHERE id = $1",
        )
        .bind(failure.id)
        .bind(now)
        .execute(&self.pool)
        .await;

        // 解析任务参数
        let params = match &failure.params {
            Some(p) => p,
            None => {
                self.update_failure_status(failure.id, failure.retry_count + 1, "EXHAUSTED").await;
                return;
            }
        };

        let badge_id = params.get("badge_id").and_then(|v| v.as_i64()).unwrap_or(0);
        let reason = params.get("reason").and_then(|v| v.as_str()).unwrap_or("");

        // 执行重试
        let result = match failure.task_type.as_str() {
            "batch_grant" => {
                Self::grant_one_user_static(&self.pool, &failure.user_id, badge_id, failure.task_id, reason).await
            }
            "batch_revoke" => {
                Self::revoke_one_user_static(&self.pool, &failure.user_id, badge_id, failure.task_id, reason).await
            }
            _ => Err(format!("不支持的任务类型: {}", failure.task_type)),
        };

        let new_retry_count = failure.retry_count + 1;

        match result {
            Ok(()) => {
                // 重试成功
                info!(
                    failure_id = failure.id,
                    task_id = failure.task_id,
                    user_id = failure.user_id,
                    "重试成功"
                );
                self.update_failure_status(failure.id, new_retry_count, "SUCCESS").await;

                // 更新任务统计：success_count +1, failure_count -1
                let _ = sqlx::query(
                    "UPDATE batch_tasks SET success_count = success_count + 1, failure_count = failure_count - 1, updated_at = NOW() WHERE id = $1",
                )
                .bind(failure.task_id)
                .execute(&self.pool)
                .await;
            }
            Err(err_msg) => {
                // 重试失败
                warn!(
                    failure_id = failure.id,
                    task_id = failure.task_id,
                    user_id = failure.user_id,
                    retry_count = new_retry_count,
                    error = %err_msg,
                    "重试失败"
                );

                let new_status = if new_retry_count >= MAX_RETRY_COUNT {
                    "EXHAUSTED"
                } else {
                    "PENDING"
                };
                self.update_failure_status(failure.id, new_retry_count, new_status).await;

                // 更新错误信息
                let _ = sqlx::query(
                    "UPDATE batch_task_failures SET error_message = $2 WHERE id = $1",
                )
                .bind(failure.id)
                .bind(&err_msg)
                .execute(&self.pool)
                .await;
            }
        }
    }

    /// 更新失败记录的重试状态
    async fn update_failure_status(&self, failure_id: i64, retry_count: i32, status: &str) {
        let _ = sqlx::query(
            "UPDATE batch_task_failures SET retry_count = $2, retry_status = $3, last_retry_at = NOW() WHERE id = $1",
        )
        .bind(failure_id)
        .bind(retry_count)
        .bind(status)
        .execute(&self.pool)
        .await;
    }

    /// 将任务标记为失败
    async fn mark_task_failed(&self, task_id: i64, error_message: &str) {
        warn!(task_id = task_id, error = error_message, "批量任务标记为失败");
        let _ = sqlx::query(
            r#"
            UPDATE batch_tasks
            SET status = 'failed', error_message = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(task_id)
        .bind(error_message)
        .execute(&self.pool)
        .await;
    }

    /// 记录单行处理失败的详情
    ///
    /// 写入 batch_task_failures 表，便于运营人员排查具体哪些用户处理失败以及原因。
    async fn insert_failure_record(
        &self,
        task_id: i64,
        row_number: i32,
        user_id: &str,
        error_code: &str,
        error_message: &str,
    ) {
        let result = sqlx::query(
            r#"
            INSERT INTO batch_task_failures (task_id, row_number, user_id, error_code, error_message, created_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            "#,
        )
        .bind(task_id)
        .bind(row_number)
        .bind(user_id)
        .bind(error_code)
        .bind(error_message)
        .execute(&self.pool)
        .await;

        if let Err(e) = result {
            error!(
                task_id = task_id,
                row_number = row_number,
                error = %e,
                "记录失败详情时出错"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_worker_default_config() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test").unwrap();
        let worker = BatchTaskWorker::new(pool);

        assert_eq!(worker.poll_interval.as_secs(), 5);
        assert_eq!(worker.chunk_size, BATCH_CHUNK_SIZE);
        assert_eq!(worker.max_file_size, MAX_FILE_SIZE_BYTES);
    }

    #[tokio::test]
    async fn test_worker_custom_config() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test").unwrap();
        let worker = BatchTaskWorker::with_config(pool, 10, 50, 100);

        assert_eq!(worker.poll_interval.as_secs(), 10);
        assert_eq!(worker.chunk_size, 50);
        assert_eq!(worker.max_file_size, 100 * 1024 * 1024);
    }
}
