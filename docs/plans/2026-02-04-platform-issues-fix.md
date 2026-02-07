# 徽章管理平台全面问题修复 — 实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 修复 `docs/platform-issues-report.md` 中列出的全部 Critical/High/Medium 问题，补齐测试覆盖，消除安全缺口。

**Architecture:** 按优先级从 P0→P3 逐步推进。P0 修复前后端接口不匹配和关键初始化缺陷；P1 补齐安全审计能力；P2 完善功能实现和测试；P3 清理技术债务。权益发放（3.1）使用 mock 接口返回逼真数据，通知（3.2）保留 mock 但返回成功，自动权益发放（3.3）串联 mock 接口。其余全部真实实现。

**Tech Stack:** Rust 2024 / Axum 0.8 / SQLx 0.8 / tonic 0.14 / Redis / PostgreSQL / React 19 / TypeScript / Playwright / Vitest

**用户约束：**
- 3.1 权益发放：mock 接口，返回成功 + 逼真内容（积分量、卡券号等）
- 3.2 通知：保留 mock，确保返回成功结果
- 3.3 自动权益发放：自动调用 3.1 的 mock 权益发放接口
- 其余所有部分：真实实现，不允许 mock

---

## Task 1: 修复前后端 API 参数不匹配 — 撤销徽章接口 (P0, Issue 1.2.1)

**问题：** 前端发送 `{ userBadgeId, reason }`，后端期望 `{ user_id, badge_id, quantity, reason }`。

**决策：** 修改后端以接受前端格式，通过 `user_badge_id` 查询出 `user_id` 和 `badge_id`。

**Files:**
- Modify: `crates/badge-admin-service/src/handlers/revoke.rs`（manual_revoke handler）
- Modify: `crates/badge-admin-service/src/dto/request.rs`（新增 DTO）

**Step 1: 在 dto/request.rs 中新增兼容的撤销请求 DTO**

在 `ManualRevokeRequest` 附近添加新的请求结构体：

```rust
/// 前端撤销请求（基于 user_badge_id）
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ManualRevokeByUserBadgeRequest {
    pub user_badge_id: i64,
    #[validate(length(min = 1, max = 500, message = "取消原因不能为空且不超过500字符"))]
    pub reason: String,
}
```

**Step 2: 修改 revoke.rs 的 manual_revoke handler**

改为接受 `ManualRevokeByUserBadgeRequest`，通过 `user_badge_id` 从 `user_badges` 表查出 `user_id`、`badge_id`，然后用 `quantity = 1` 执行撤销逻辑。

```rust
pub async fn manual_revoke(
    State(state): State<AppState>,
    Json(req): Json<ManualRevokeByUserBadgeRequest>,
) -> Result<Json<ApiResponse<RevokeLogDto>>, AdminError> {
    req.validate()?;

    // 通过 user_badge_id 查询用户和徽章信息
    let ub_row: Option<(String, i64)> = sqlx::query_as(
        "SELECT user_id, badge_id FROM user_badges WHERE id = $1"
    )
    .bind(req.user_badge_id)
    .fetch_optional(&state.pool)
    .await?;

    let (user_id, badge_id) = ub_row
        .ok_or_else(|| AdminError::NotFound(format!("用户徽章记录不存在: {}", req.user_badge_id)))?;

    // 后续撤销逻辑保持不变，使用 user_id, badge_id, quantity=1, reason
    // ...
}
```

**Step 3: 运行后端编译验证**

Run: `cargo check -p badge-admin-service`
Expected: 编译通过

**Step 4: Commit**

```bash
git add crates/badge-admin-service/src/handlers/revoke.rs crates/badge-admin-service/src/dto/request.rs
git commit -m "fix: 修复撤销接口前后端参数不匹配 (Issue 1.2.1)"
```

---

## Task 2: 修复前后端 API 参数不匹配 — CSV 上传接口 (P0, Issue 1.2.2)

**问题：** 前端用 `multipart/form-data` 上传文件，后端期望 JSON `{ content: "csv_string" }`。

**决策：** 修改后端支持 multipart 文件上传，保留 JSON 方式作为兼容。

**Files:**
- Modify: `crates/badge-admin-service/Cargo.toml`（添加 axum-multipart 依赖）
- Modify: `crates/badge-admin-service/src/handlers/grant.rs`（upload_user_csv handler）

**Step 1: 添加 multipart 依赖**

在 `crates/badge-admin-service/Cargo.toml` 的 `[dependencies]` 中添加：
```toml
axum-extra = { version = "0.10", features = ["multipart"] }
```
或使用 axum 自带的 multipart extractor（axum 0.8 内置）。

**Step 2: 修改 upload_user_csv 支持 multipart**

```rust
use axum::extract::Multipart;

pub async fn upload_user_csv(
    State(_state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<CsvUploadResult>>, AdminError> {
    let mut csv_content = String::new();

    while let Some(field) = multipart.next_field().await
        .map_err(|e| AdminError::Validation(format!("解析上传文件失败: {}", e)))?
    {
        if field.name() == Some("file") {
            csv_content = field.text().await
                .map_err(|e| AdminError::Validation(format!("读取文件内容失败: {}", e)))?;
            break;
        }
    }

    if csv_content.is_empty() {
        return Err(AdminError::Validation("未上传文件或文件内容为空".to_string()));
    }

    // 后续 CSV 解析逻辑保持不变
    // ...
}
```

**Step 3: 运行编译验证**

Run: `cargo check -p badge-admin-service`
Expected: 编译通过

**Step 4: Commit**

```bash
git add crates/badge-admin-service/Cargo.toml crates/badge-admin-service/src/handlers/grant.rs
git commit -m "fix: CSV 上传接口改用 multipart 文件上传 (Issue 1.2.2)"
```

---

## Task 3: 修复前后端 API 参数不匹配 — 批量任务结果下载 (P0, Issue 1.2.3)

**问题：** 前端期望 Blob 下载，后端返回 JSON `{ taskId, resultFileUrl }`。

**决策：** 修改前端以适应后端的 JSON 响应格式，获取 URL 后再执行下载。

**Files:**
- Modify: `web/admin-ui/src/services/grant.ts`（downloadBatchResult 函数）

**Step 1: 修改前端 downloadBatchResult 函数**

```typescript
export async function downloadBatchResult(id: number): Promise<void> {
  // 先获取结果文件 URL
  const result = await get<{ taskId: number; resultFileUrl: string | null }>(
    `/admin/tasks/${id}/result`
  );

  if (!result.resultFileUrl) {
    throw new Error('任务结果文件不存在');
  }

  // 通过 URL 下载文件
  const link = document.createElement('a');
  link.href = result.resultFileUrl;
  link.download = `task-${id}-result.csv`;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
}
```

**Step 2: 运行前端类型检查**

Run: `cd web/admin-ui && npx tsc --noEmit`
Expected: 无类型错误

**Step 3: Commit**

```bash
git add web/admin-ui/src/services/grant.ts
git commit -m "fix: 批量任务结果改为先获取 URL 再下载 (Issue 1.2.3)"
```

---

## Task 4: 初始化 RedemptionService (P0, Issue 1.1)

**问题：** `main.rs` 从未调用 `set_redemption_service()`，导致兑换功能完全不可用。

**Files:**
- Modify: `crates/badge-admin-service/src/main.rs`

**Step 1: 在 main.rs 中初始化 RedemptionService**

在 `AppState` 创建之后、路由构建之前，添加：

```rust
// 初始化兑换服务
let redemption_repo = Arc::new(badge_management::repository::RedemptionRepository::new(db.pool().clone()));
let redemption_service = Arc::new(badge_management::service::RedemptionService::new(
    redemption_repo,
    cache.clone(),
    db.pool().clone(),
));
state.set_redemption_service(redemption_service);
info!("RedemptionService initialized");
```

需要确认 `badge_management` crate 的 `RedemptionRepository` 和 `RedemptionService` 的实际导入路径和构造参数。

**Step 2: 运行编译验证**

Run: `cargo check -p badge-admin-service`
Expected: 编译通过

**Step 3: Commit**

```bash
git add crates/badge-admin-service/src/main.rs
git commit -m "fix: 初始化 RedemptionService 解决兑换功能不可用 (Issue 1.1)"
```

---

## Task 5: 实现批量任务消费者 Worker (P0, Issue 1.3)

**问题：** `batch_grant` 和 `batch_revoke` 创建任务后无消费者处理，任务永远停留 pending。

**决策：** 在 admin-service 内实现后台 tokio 任务轮询 `batch_tasks` 表，处理 pending 任务。

**Files:**
- Create: `crates/badge-admin-service/src/worker/mod.rs`
- Create: `crates/badge-admin-service/src/worker/batch_task_worker.rs`
- Modify: `crates/badge-admin-service/src/main.rs`（启动 worker）
- Modify: `crates/badge-admin-service/src/lib.rs`（声明 worker 模块）

**Step 1: 创建 batch_task_worker.rs**

实现核心逻辑：
1. 每 5 秒轮询 `batch_tasks` 表中 `status = 'pending'` 的任务（SELECT ... FOR UPDATE SKIP LOCKED）
2. 将状态改为 `processing`
3. 根据 `task_type`（batch_grant / batch_revoke）下载 `file_url` 的 CSV，逐行处理
4. 对每行执行发放/撤销操作，记录成功/失败到 `batch_task_failures`
5. 更新任务进度（progress/success_count/failure_count）
6. 完成后生成结果文件 URL，标记 `completed`

```rust
pub struct BatchTaskWorker {
    pool: PgPool,
    poll_interval: Duration,
}

impl BatchTaskWorker {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            poll_interval: Duration::from_secs(5),
        }
    }

    pub async fn run(&self, mut shutdown: tokio::sync::watch::Receiver<bool>) {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(self.poll_interval) => {
                    if let Err(e) = self.process_pending_tasks().await {
                        tracing::error!(error = %e, "批量任务处理出错");
                    }
                }
                _ = shutdown.changed() => {
                    tracing::info!("BatchTaskWorker shutting down");
                    break;
                }
            }
        }
    }

    async fn process_pending_tasks(&self) -> anyhow::Result<()> {
        // 获取一个 pending 任务（行级锁避免并发重复处理）
        let task = sqlx::query_as::<_, BatchTaskRow>(
            "SELECT * FROM batch_tasks WHERE status = 'pending'
             ORDER BY created_at ASC LIMIT 1 FOR UPDATE SKIP LOCKED"
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(task) = task {
            self.execute_task(task).await?;
        }
        Ok(())
    }

    async fn execute_task(&self, task: BatchTaskRow) -> anyhow::Result<()> {
        // 标记为处理中
        sqlx::query("UPDATE batch_tasks SET status = 'processing', updated_at = NOW() WHERE id = $1")
            .bind(task.id)
            .execute(&self.pool)
            .await?;

        match task.task_type.as_str() {
            "batch_grant" => self.process_batch_grant(task).await,
            "batch_revoke" => self.process_batch_revoke(task).await,
            _ => {
                tracing::warn!(task_type = %task.task_type, "未知任务类型");
                Ok(())
            }
        }
    }
}
```

**Step 2: 在 main.rs 中启动 worker**

```rust
// 启动批量任务 worker
let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
let worker = BatchTaskWorker::new(db.pool().clone());
tokio::spawn(async move {
    worker.run(shutdown_rx).await;
});
```

**Step 3: 运行编译验证**

Run: `cargo check -p badge-admin-service`

**Step 4: Commit**

```bash
git add crates/badge-admin-service/src/worker/ crates/badge-admin-service/src/main.rs crates/badge-admin-service/src/lib.rs
git commit -m "feat: 实现批量任务消费者 Worker (Issue 1.3)"
```

---

## Task 6: 实现操作日志审计中间件 (P1, Issue 2.1)

**问题：** `operation_logs` 表存在但无写入机制，操作日志永远为空。

**Files:**
- Create: `crates/badge-admin-service/src/middleware/audit.rs`
- Modify: `crates/badge-admin-service/src/middleware/mod.rs`
- Modify: `crates/badge-admin-service/src/main.rs`（挂载中间件）

**Step 1: 实现审计中间件**

中间件在响应后异步写入日志，不阻塞请求处理：

```rust
/// 审计中间件：在变更操作（POST/PUT/PATCH/DELETE）成功后写入 operation_logs
pub async fn audit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    // 只记录写操作
    if !matches!(method, Method::POST | Method::PUT | Method::PATCH | Method::DELETE) {
        return next.run(request).await;
    }

    // 跳过认证路由
    if path.starts_with("/api/admin/auth/") {
        return next.run(request).await;
    }

    // 从 Claims 中获取操作人信息
    let claims = request.extensions().get::<Claims>().cloned();

    let response = next.run(request).await;

    // 仅在成功响应时记录日志
    if response.status().is_success() {
        if let Some(claims) = claims {
            let pool = state.pool.clone();
            let (module, action) = parse_module_action(&path, &method);
            tokio::spawn(async move {
                let _ = sqlx::query(
                    r#"INSERT INTO operation_logs
                       (operator_id, operator_name, module, action, target_type, target_id, ip_address, created_at)
                       VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())"#
                )
                .bind(&claims.sub)
                .bind(&claims.display_name)
                .bind(&module)
                .bind(&action)
                .bind(extract_target_type(&path))
                .bind(extract_target_id(&path))
                .bind::<Option<String>>(None) // IP 从连接信息获取
                .execute(&pool)
                .await;
            });
        }
    }

    response
}

/// 从 URL 路径和 HTTP 方法推断操作模块和动作
fn parse_module_action(path: &str, method: &Method) -> (String, String) {
    // /api/admin/badges/123 + DELETE => ("badge", "delete")
    // /api/admin/grants/manual + POST => ("grant", "create")
    let segments: Vec<&str> = path.trim_start_matches("/api/admin/")
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let module = segments.first().unwrap_or(&"unknown").to_string();
    let action = match *method {
        Method::POST => "create",
        Method::PUT => "update",
        Method::PATCH => "update",
        Method::DELETE => "delete",
        _ => "unknown",
    }.to_string();

    (module, action)
}
```

**Step 2: 在 main.rs 中挂载审计中间件（在认证中间件之后）**

**Step 3: 运行编译验证**

Run: `cargo check -p badge-admin-service`

**Step 4: Commit**

```bash
git add crates/badge-admin-service/src/middleware/audit.rs crates/badge-admin-service/src/middleware/mod.rs crates/badge-admin-service/src/main.rs
git commit -m "feat: 实现操作日志审计中间件自动写入 (Issue 2.1)"
```

---

## Task 7: 规则测试接口对接 rule-engine gRPC (P1, Issue 2.2)

**问题：** `test_rule` 和 `test_rule_definition` 永远返回硬编码 `matched: true`。

**Files:**
- Modify: `crates/badge-admin-service/src/handlers/rule.rs`
- Modify: `crates/badge-admin-service/src/state.rs`（如需添加 rule-engine client）

**Step 1: 在 AppState 中添加 rule-engine gRPC 客户端**

通过环境变量 `RULE_ENGINE_GRPC_ADDR` 配置连接地址，在 main.rs 中初始化。

**Step 2: 修改 test_rule handler 调用 gRPC**

```rust
pub async fn test_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(test_data): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AdminError> {
    let rule = fetch_rule_by_id(&state.pool, id).await?;

    // 获取 rule-engine gRPC 客户端
    let mut client = state.get_rule_engine_client().await
        .ok_or_else(|| AdminError::Internal("规则引擎服务不可用".to_string()))?;

    // 构建 gRPC 请求
    let request = tonic::Request::new(EvaluateRuleRequest {
        rule_json: rule.rule_json.to_string(),
        context: test_data.to_string(),
    });

    let response = client.evaluate_rule(request).await
        .map_err(|e| AdminError::Internal(format!("规则评估失败: {}", e)))?;

    let inner = response.into_inner();
    let result = serde_json::json!({
        "matched": inner.matched,
        "matchedConditions": inner.matched_conditions,
        "evaluationTimeMs": inner.evaluation_time_ms,
    });

    Ok(Json(ApiResponse::success(result)))
}
```

**Step 3: 同样修改 test_rule_definition**

**Step 4: 运行编译验证**

Run: `cargo check -p badge-admin-service`

**Step 5: Commit**

```bash
git add crates/badge-admin-service/src/handlers/rule.rs crates/badge-admin-service/src/state.rs crates/badge-admin-service/src/main.rs
git commit -m "feat: 规则测试接口对接 rule-engine gRPC 服务 (Issue 2.2)"
```

---

## Task 8: 实现权益同步和徽章关联 (P1, Issue 2.2)

**问题：** `list_sync_logs` 返回空列表、`trigger_sync` 返回假 sync_id、`link_badge_to_benefit` 不创建记录。

**Files:**
- Create: `migrations/20250211_001_benefit_sync_link.sql`
- Modify: `crates/badge-admin-service/src/handlers/benefit.rs`

**Step 1: 创建缺失的数据库表**

```sql
-- 权益同步日志表
CREATE TABLE IF NOT EXISTS benefit_sync_logs (
    id BIGSERIAL PRIMARY KEY,
    sync_type VARCHAR(50) NOT NULL DEFAULT 'full',
    status VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    total_count INT NOT NULL DEFAULT 0,
    success_count INT NOT NULL DEFAULT 0,
    failed_count INT NOT NULL DEFAULT 0,
    error_message TEXT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 徽章-权益关联表
CREATE TABLE IF NOT EXISTS badge_benefit_links (
    id BIGSERIAL PRIMARY KEY,
    badge_id BIGINT NOT NULL REFERENCES badges(id),
    benefit_id BIGINT NOT NULL REFERENCES benefits(id),
    quantity INT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(badge_id, benefit_id)
);
CREATE INDEX idx_badge_benefit_links_badge ON badge_benefit_links(badge_id);
CREATE INDEX idx_badge_benefit_links_benefit ON badge_benefit_links(benefit_id);
```

**Step 2: 修改 list_sync_logs 查询真实表**

```rust
pub async fn list_sync_logs(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<ApiResponse<PageResponse<BenefitSyncLogDto>>>, AdminError> {
    let offset = pagination.offset();
    let limit = pagination.limit();

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM benefit_sync_logs")
        .fetch_one(&state.pool)
        .await?;

    let rows = sqlx::query_as::<_, BenefitSyncLogRow>(
        "SELECT * FROM benefit_sync_logs ORDER BY created_at DESC LIMIT $1 OFFSET $2"
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let items: Vec<BenefitSyncLogDto> = rows.into_iter().map(Into::into).collect();
    let response = PageResponse::new(items, total.0, pagination.page, pagination.page_size);
    Ok(Json(ApiResponse::success(response)))
}
```

**Step 3: 修改 trigger_sync 写入同步日志并执行异步同步**

**Step 4: 修改 link_badge_to_benefit 写入 badge_benefit_links 表**

```rust
// 在验证通过后，创建关联记录
sqlx::query(
    "INSERT INTO badge_benefit_links (badge_id, benefit_id, quantity, created_at)
     VALUES ($1, $2, $3, NOW())
     ON CONFLICT (badge_id, benefit_id) DO UPDATE SET quantity = $3"
)
.bind(req.badge_id)
.bind(benefit_id)
.bind(req.quantity)
.execute(&state.pool)
.await?;
```

**Step 5: 运行编译验证**

Run: `cargo check -p badge-admin-service`

**Step 6: Commit**

```bash
git add migrations/20250211_001_benefit_sync_link.sql crates/badge-admin-service/src/handlers/benefit.rs
git commit -m "feat: 实现权益同步日志和徽章关联的真实存储 (Issue 2.2)"
```

---

## Task 9: 实现 API Key 限流 (P1, Issue 2.3)

**问题：** `api_key.rate_limit` 字段存在但中间件不检查。

**Files:**
- Modify: `crates/badge-admin-service/src/middleware/api_key_auth.rs`

**Step 1: 在 api_key_auth_middleware 中添加限流检查**

查询 `rate_limit` 字段，使用 Redis 滑动窗口计数器实现：

```rust
// 查询时额外获取 rate_limit
let row: Option<(i64, String, JsonValue, bool, Option<DateTime<Utc>>, Option<i32>)> =
    sqlx::query_as(
        "SELECT id, name, permissions, enabled, expires_at, rate_limit FROM api_key WHERE key_hash = $1"
    )
    .bind(&key_hash)
    .fetch_optional(&pool)
    .await?;

// 限流检查
if let Some(rate_limit) = rate_limit {
    let cache_key = format!("api_key_rate:{}:{}", key_id, current_minute());
    let count: i64 = redis_conn.incr(&cache_key, 1).await?;
    if count == 1 {
        redis_conn.expire(&cache_key, 60).await?;
    }
    if count > rate_limit as i64 {
        return Err((StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded").into_response());
    }
}
```

需要将 Redis `Cache` 通过 State 传入中间件。修改 `external_api_routes` 签名以传递 cache。

**Step 2: 运行编译验证**

Run: `cargo check -p badge-admin-service`

**Step 3: Commit**

```bash
git add crates/badge-admin-service/src/middleware/api_key_auth.rs crates/badge-admin-service/src/routes.rs crates/badge-admin-service/src/main.rs
git commit -m "feat: 实现 API Key 基于 Redis 的滑动窗口限流 (Issue 2.3)"
```

---

## Task 10: 实现 Token 黑名单 (P1, Issue 2.4)

**问题：** logout 是空操作，泄露的 JWT 在过期前无法失效。

**Files:**
- Modify: `crates/badge-admin-service/src/handlers/auth.rs`（logout handler）
- Modify: `crates/badge-admin-service/src/middleware/auth.rs`（认证中间件）

**Step 1: 修改 logout 将 Token 加入 Redis 黑名单**

```rust
pub async fn logout(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<ApiResponse<()>>> {
    if let Some(claims) = request.extensions().get::<Claims>() {
        // 计算 Token 剩余有效期作为 Redis TTL
        let ttl = claims.exp - Utc::now().timestamp();
        if ttl > 0 {
            let token_jti = &claims.jti; // 需要在 Claims 中添加 jti 字段
            state.cache.set_with_ttl(
                &format!("token_blacklist:{}", token_jti),
                "1",
                ttl as u64,
            ).await.ok();
        }
    }
    Ok(Json(ApiResponse::success(())))
}
```

**Step 2: 修改认证中间件检查黑名单**

在 JWT 验证成功后，检查 Redis 中是否存在该 Token 的黑名单记录。

**Step 3: 运行编译验证**

Run: `cargo check -p badge-admin-service`

**Step 4: Commit**

```bash
git add crates/badge-admin-service/src/handlers/auth.rs crates/badge-admin-service/src/middleware/auth.rs
git commit -m "feat: 实现 JWT Token 黑名单机制 (Issue 2.4)"
```

---

## Task 11: 用户筛选预览接口真实实现 (P1, Issue 2.2)

**问题：** `preview_user_filter` 永远返回 `{ total: 0, users: [] }`。

**Files:**
- Modify: `crates/badge-admin-service/src/handlers/grant.rs`

**Step 1: 修改 preview_user_filter 查询真实数据**

```rust
pub async fn preview_user_filter(
    State(state): State<AppState>,
    Json(req): Json<UserFilterRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AdminError> {
    // 根据筛选条件查询 user_badges 表
    let total: (i64,) = sqlx::query_as(
        r#"SELECT COUNT(DISTINCT user_id) FROM user_badges
           WHERE ($1::bigint IS NULL OR badge_id = $1)
             AND ($2::int IS NULL OR quantity >= $2)
             AND ($3::text IS NULL OR status = $3)"#
    )
    .bind(req.badge_id)
    .bind(req.min_quantity)
    .bind(&req.status)
    .fetch_one(&state.pool)
    .await?;

    let users: Vec<(String,)> = sqlx::query_as(
        r#"SELECT DISTINCT user_id FROM user_badges
           WHERE ($1::bigint IS NULL OR badge_id = $1)
             AND ($2::int IS NULL OR quantity >= $2)
             AND ($3::text IS NULL OR status = $3)
           LIMIT 20"#
    )
    .bind(req.badge_id)
    .bind(req.min_quantity)
    .bind(&req.status)
    .fetch_all(&state.pool)
    .await?;

    let result = serde_json::json!({
        "total": total.0,
        "users": users.iter().map(|u| &u.0).collect::<Vec<_>>(),
    });

    Ok(Json(ApiResponse::success(result)))
}
```

**Step 2: 运行编译验证**

**Step 3: Commit**

```bash
git add crates/badge-admin-service/src/handlers/grant.rs
git commit -m "feat: 用户筛选预览接口查询真实数据 (Issue 2.2)"
```

---

## Task 12: 权益发放 Mock 接口增强 (P2, Issue 3.1)

**用户要求：** mock 接口返回发放成功以及逼真内容（积分量、卡券号等）。

**Files:**
- Modify: `crates/badge-management-service/src/benefit/handlers/points.rs`
- Modify: `crates/badge-management-service/src/benefit/handlers/coupon.rs`
- Modify: `crates/badge-management-service/src/benefit/handlers/physical.rs`

**Step 1: 增强 PointsHandler 的 mock 返回**

```rust
async fn grant_points(config: &PointsConfig, user_id: &str) -> Result<GrantDetail> {
    let transaction_id = Uuid::now_v7().to_string();
    let point_amount = config.point_amount;

    info!(
        user_id = user_id,
        point_amount = point_amount,
        transaction_id = %transaction_id,
        "[Mock] 积分发放成功"
    );

    Ok(GrantDetail {
        transaction_id,
        details: serde_json::json!({
            "pointAmount": point_amount,
            "pointType": config.point_type,
            "currentBalance": point_amount + 1000, // 模拟当前余额
            "expiresAt": (Utc::now() + chrono::Duration::days(config.validity_days.unwrap_or(365) as i64))
                .format("%Y-%m-%d").to_string(),
            "remark": config.remark.clone().unwrap_or_else(|| "徽章权益积分发放".to_string()),
        }),
    })
}
```

**Step 2: 增强 CouponHandler 的 mock 返回**

```rust
async fn issue_coupon(config: &CouponConfig, user_id: &str) -> Result<GrantDetail> {
    let coupon_id = Uuid::now_v7().to_string();
    let coupon_code = format!("CPN{}", &coupon_id.replace("-", "")[..8].to_uppercase());

    info!(
        user_id = user_id,
        coupon_code = %coupon_code,
        "[Mock] 优惠券发放成功"
    );

    Ok(GrantDetail {
        transaction_id: coupon_id.clone(),
        details: serde_json::json!({
            "couponId": coupon_id,
            "couponCode": coupon_code,
            "templateId": config.coupon_template_id,
            "quantity": config.quantity,
            "expiresAt": (Utc::now() + chrono::Duration::days(config.validity_days.unwrap_or(30) as i64))
                .format("%Y-%m-%d").to_string(),
            "status": "ACTIVE",
        }),
    })
}
```

**Step 3: 增强 PhysicalHandler 的 mock 返回**

```rust
async fn send_shipment_message(&self, ...) -> Result<String> {
    let shipment_id = format!("SHP{}", Uuid::now_v7().to_string().replace("-", "")[..12].to_uppercase());

    info!(
        shipment_id = %shipment_id,
        "[Mock] 物流单创建成功"
    );

    // 返回包含物流信息的响应
    Ok(shipment_id)
}
```

同时确保 `revoke_points` 和 `revoke_coupon` 也返回成功结果。

**Step 4: 运行编译验证**

Run: `cargo check -p badge-management-service`

**Step 5: Commit**

```bash
git add crates/badge-management-service/src/benefit/handlers/
git commit -m "feat: 权益发放 mock 接口返回逼真成功数据 (Issue 3.1)"
```

---

## Task 13: 通知系统 Mock 确保返回成功 (P2, Issue 3.2)

**用户要求：** 保留 mock 但返回成功结果。

**Files:**
- Modify: `crates/notification-worker/src/sender.rs`

**Step 1: 确认四个发送器都返回 success: true**

当前实现已经返回 `success: true`。需要确认：
1. `SendResult.success` 字段确实为 `true`
2. `SendResult.message_id` 有合理值（UUID）
3. 无 error 字段

检查后如已满足则无需修改，仅确认测试覆盖。

**Step 2: 确保 badge-management-service 的 NotificationSender 错误处理正确**

验证 `send_badge_granted` 等方法在 fire-and-forget 模式下不会因 mock 返回导致恐慌。

**Step 3: Commit（如有修改）**

```bash
git add crates/notification-worker/src/sender.rs
git commit -m "fix: 确保通知 mock 接口返回成功结果 (Issue 3.2)"
```

---

## Task 14: 串联自动权益发放流程 (P2, Issue 3.3)

**问题：** `AutoBenefitEvaluator` 存在但未被调用。

**决策：** 在 main.rs 初始化 AutoBenefitEvaluator 并注入 BenefitService（包含 Task 12 的 mock 处理器），在徽章发放后触发自动权益评估。

**Files:**
- Modify: `crates/badge-admin-service/src/main.rs`
- Modify: `crates/badge-admin-service/src/handlers/grant.rs`（发放成功后触发评估）
- Modify: `crates/badge-admin-service/src/state.rs`（添加 auto_benefit_evaluator 字段）

**Step 1: 在 AppState 中添加 AutoBenefitEvaluator**

```rust
pub auto_benefit_evaluator: Option<Arc<AutoBenefitEvaluator>>,
```

**Step 2: 在 main.rs 中初始化完整的权益发放链**

1. 创建 HandlerRegistry，注册 PointsHandler、CouponHandler、PhysicalHandler（mock 版）
2. 创建 BenefitService
3. 创建 AutoBenefitEvaluator，注入 BenefitService
4. 将 evaluator 存入 AppState

**Step 3: 在 manual_grant 成功后触发自动权益评估**

```rust
// 发放成功后，异步触发自动权益评估
if let Some(evaluator) = &state.auto_benefit_evaluator {
    let evaluator = evaluator.clone();
    let user_id = req.user_id.clone();
    let badge_id = req.badge_id;
    tokio::spawn(async move {
        let context = AutoBenefitContext {
            user_id,
            trigger_badge_id: badge_id,
            trigger_event: "badge_granted".to_string(),
        };
        if let Err(e) = evaluator.evaluate(context).await {
            tracing::warn!(error = %e, "自动权益评估失败");
        }
    });
}
```

**Step 4: 运行编译验证**

Run: `cargo check -p badge-admin-service`

**Step 5: Commit**

```bash
git add crates/badge-admin-service/src/main.rs crates/badge-admin-service/src/state.rs crates/badge-admin-service/src/handlers/grant.rs
git commit -m "feat: 串联自动权益发放流程 (Issue 3.3)"
```

---

## Task 15: 修复 24 处空断言 (P1, Issue 4.2)

**问题：** 5 个 E2E spec 文件中 24 处 `expect(true).toBeTruthy()` 空断言。

**Files:**
- Modify: `web/admin-ui/e2e/specs/benefits-extended.spec.ts`
- Modify: `web/admin-ui/e2e/specs/benefit-sync.spec.ts`
- Modify: `web/admin-ui/e2e/specs/categories.spec.ts`
- Modify: `web/admin-ui/e2e/specs/series.spec.ts`
- Modify: `web/admin-ui/e2e/specs/badge-crud.spec.ts`

**Step 1: 替换 categories.spec.ts 中的空断言**

将 `expect(true).toBeTruthy()` 替换为实际的 DOM 断言：

```typescript
// 原：expect(true).toBeTruthy();
// 改为实际检查元素存在
const editButton = page.locator('button:has-text("编辑")').first();
await expect(editButton).toBeVisible();
```

每个空断言根据其上下文（test name 描述了要验证什么）替换为对应的 `toBeVisible()`、`toHaveCount()`、`toHaveText()` 等断言。

**Step 2: 同样处理其余 4 个文件**

**Step 3: 运行 E2E 测试验证**

Run: `cd web/admin-ui && npx playwright test --config=e2e/playwright.config.ts`
Expected: 所有断言测试通过

**Step 4: Commit**

```bash
git add web/admin-ui/e2e/specs/
git commit -m "fix: 替换 24 处空断言为真实 DOM 验证 (Issue 4.2)"
```

---

## Task 16: 补齐 error.rs 测试覆盖 (P2, Issue 5.2)

**问题：** 20 个错误变体仅 4 个有测试。

**Files:**
- Modify: `crates/badge-admin-service/src/error.rs`（tests 模块）

**Step 1: 为所有 16 个未测试变体添加 status_code 和 error_code 测试**

```rust
#[test]
fn test_all_error_status_codes() {
    use StatusCode;
    let cases: Vec<(AdminError, StatusCode)> = vec![
        (AdminError::Unauthorized("test".into()), StatusCode::UNAUTHORIZED),
        (AdminError::Forbidden("test".into()), StatusCode::FORBIDDEN),
        (AdminError::InvalidCredentials, StatusCode::UNAUTHORIZED),
        (AdminError::UserDisabled, StatusCode::FORBIDDEN),
        (AdminError::UserLocked, StatusCode::FORBIDDEN),
        (AdminError::UserNotFound("1".into()), StatusCode::NOT_FOUND),
        (AdminError::CategoryNotFound(1), StatusCode::NOT_FOUND),
        (AdminError::SeriesNotFound(1), StatusCode::NOT_FOUND),
        (AdminError::RuleNotFound(1), StatusCode::NOT_FOUND),
        (AdminError::TaskNotFound(1), StatusCode::NOT_FOUND),
        (AdminError::DependencyNotFound(1), StatusCode::NOT_FOUND),
        (AdminError::BenefitNotFound(1), StatusCode::NOT_FOUND),
        (AdminError::NotFound("test".into()), StatusCode::NOT_FOUND),
        (AdminError::InvalidRuleJson("test".into()), StatusCode::BAD_REQUEST),
        (AdminError::FileProcessingError("test".into()), StatusCode::UNPROCESSABLE_ENTITY),
        (AdminError::InsufficientStock, StatusCode::CONFLICT),
        (AdminError::InsufficientUserBadge, StatusCode::CONFLICT),
        (AdminError::Redis("test".into()), StatusCode::INTERNAL_SERVER_ERROR),
        (AdminError::Internal("test".into()), StatusCode::INTERNAL_SERVER_ERROR),
    ];
    for (error, expected_status) in cases {
        assert_eq!(error.status_code(), expected_status, "Failed for {:?}", error);
    }
}

#[test]
fn test_all_error_codes() {
    let cases: Vec<(AdminError, &str)> = vec![
        (AdminError::Unauthorized("test".into()), "UNAUTHORIZED"),
        (AdminError::Forbidden("test".into()), "FORBIDDEN"),
        (AdminError::InvalidCredentials, "INVALID_CREDENTIALS"),
        (AdminError::UserDisabled, "USER_DISABLED"),
        (AdminError::UserLocked, "USER_LOCKED"),
        // ... 全部 20 个变体
    ];
    for (error, expected_code) in cases {
        assert_eq!(error.error_code(), expected_code, "Failed for {:?}", error);
    }
}

#[test]
fn test_into_response_sanitizes_system_errors() {
    // 验证 Database/Redis/Internal 错误不暴露详细信息
    let error = AdminError::Internal("sensitive db details".into());
    let response = error.into_response();
    // 检查响应 body 不包含 "sensitive db details"
}

#[test]
fn test_from_validation_errors() {
    // 验证 From<validator::ValidationErrors> 转换
}
```

**Step 2: 运行测试**

Run: `cargo test -p badge-admin-service -- error::tests`
Expected: 全部通过

**Step 3: Commit**

```bash
git add crates/badge-admin-service/src/error.rs
git commit -m "test: 补齐 error.rs 全部 20 个变体的测试覆盖 (Issue 5.2)"
```

---

## Task 17: 事件类型管理 CRUD API (P2, Issue 3.4)

**问题：** `event_types` 表有数据但无 CRUD API。

**Files:**
- Create: `crates/badge-admin-service/src/handlers/event_type.rs`
- Modify: `crates/badge-admin-service/src/handlers/mod.rs`
- Modify: `crates/badge-admin-service/src/routes.rs`

**Step 1: 实现 event_type handler**

包含 `list_event_types`、`get_event_type`、`create_event_type`、`update_event_type`、`delete_event_type` 五个 CRUD 操作。

**Step 2: 在 routes.rs 中注册路由**

```rust
fn event_type_routes() -> Router<AppState> {
    Router::new()
        .route("/event-types", get(handlers::event_type::list_event_types)
            .layer(axum_mw::from_fn(require_permission("rule:event-type:read"))))
        .route("/event-types/{id}", get(handlers::event_type::get_event_type)
            .layer(axum_mw::from_fn(require_permission("rule:event-type:read"))))
        .route("/event-types", post(handlers::event_type::create_event_type)
            .layer(axum_mw::from_fn(require_permission("rule:event-type:write"))))
        .route("/event-types/{id}", put(handlers::event_type::update_event_type)
            .layer(axum_mw::from_fn(require_permission("rule:event-type:write"))))
        .route("/event-types/{id}", delete(handlers::event_type::delete_event_type)
            .layer(axum_mw::from_fn(require_permission("rule:event-type:write"))))
}
```

**Step 3: 运行编译验证**

**Step 4: Commit**

```bash
git add crates/badge-admin-service/src/handlers/event_type.rs crates/badge-admin-service/src/handlers/mod.rs crates/badge-admin-service/src/routes.rs
git commit -m "feat: 实现事件类型管理 CRUD API (Issue 3.4)"
```

---

## Task 18: 完善配置文件和环境变量文档 (P3, Issue 3.5)

**Files:**
- Modify: `config/badge-admin-service.toml`
- Modify: `docker/.env.example`

**Step 1: 补充 badge-admin-service.toml**

```toml
[server]
port = 8080

[database]
url = "postgres://badge:badge_password@localhost:5432/badge_db"
max_connections = 20
min_connections = 5

[redis]
url = "redis://localhost:6379"

[kafka]
consumer_group = "badge-admin-service"

[observability]
metrics_port = 9991
```

**Step 2: 补充 .env.example 缺失变量**

```env
# CORS 配置（生产环境必须设置具体域名，不允许 *）
BADGE_CORS_ORIGINS=http://localhost:3001,http://localhost:5173

# 登录安全策略
BADGE_MAX_LOGIN_ATTEMPTS=5
BADGE_LOCK_DURATION_MINS=30

# gRPC 服务地址
BADGE_MANAGEMENT_GRPC_ADDR=http://127.0.0.1:50052
RULE_ENGINE_GRPC_ADDR=http://127.0.0.1:50051

# 运行环境标识（production 时强制要求 JWT_SECRET）
BADGE_ENV=development
```

**Step 3: Commit**

```bash
git add config/badge-admin-service.toml docker/.env.example
git commit -m "docs: 完善配置文件和环境变量文档 (Issue 3.5)"
```

---

## Task 19: 安全加固 (P1, Issue 8.1-8.4)

**Files:**
- Modify: `crates/badge-admin-service/src/middleware/auth.rs`（清理无效公开路径）
- Modify: `crates/badge-admin-service/src/main.rs`（CORS 生产环境校验）
- Modify: `crates/badge-admin-service/src/handlers/system_user.rs`（密码策略增强）
- Modify: `crates/badge-admin-service/src/handlers/benefit.rs`（消除 unwrap）
- Modify: `crates/badge-admin-service/src/middleware/api_key_auth.rs`（权限解析错误处理）

**Step 1: 清理 auth 中间件公开路径**

```rust
let public_paths = [
    "/api/admin/auth/login",
    "/api/v1/",  // API Key 认证路由，由 api_key_auth_middleware 保护
    "/health",
    "/ready",
];
```

移除不存在的 `/api/admin/health`。

**Step 2: CORS 生产环境强制校验**

```rust
if std::env::var("BADGE_ENV").unwrap_or_default() == "production" && allowed_origins == "*" {
    panic!("BADGE_CORS_ORIGINS must not be '*' in production environment");
}
```

**Step 3: 密码策略增强**

在 `ResetPasswordRequest` 验证中添加复杂度检查。

**Step 4: 消除 benefit.rs 中的 unwrap()**

将 `.and_hms_opt(0,0,0).unwrap()` 改为 `.and_hms_opt(0,0,0).ok_or_else(|| ...)?`。

**Step 5: 修复 api_key_auth.rs 权限解析**

将 `serde_json::from_value(permissions_json).unwrap_or_default()` 改为带日志的错误处理。

**Step 6: 运行编译验证**

**Step 7: Commit**

```bash
git add crates/badge-admin-service/src/
git commit -m "fix: 安全加固 — 清理公开路径、CORS 校验、密码策略、消除 unwrap (Issue 8.1-8.4)"
```

---

## Task 20: 安装前端单元测试框架 (P2, Issue 4.1)

**Files:**
- Modify: `web/admin-ui/package.json`
- Create: `web/admin-ui/vitest.config.ts`
- Create: `web/admin-ui/src/services/__tests__/auth.test.ts`（示例）

**Step 1: 安装 Vitest 和 Testing Library**

```bash
cd web/admin-ui && npm install -D vitest @testing-library/react @testing-library/jest-dom jsdom
```

**Step 2: 创建 vitest.config.ts**

```typescript
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test-setup.ts'],
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
});
```

**Step 3: 创建示例单元测试**

为 `services/auth.ts` 编写基础测试。

**Step 4: 运行测试**

Run: `cd web/admin-ui && npx vitest run`
Expected: 测试通过

**Step 5: Commit**

```bash
git add web/admin-ui/package.json web/admin-ui/vitest.config.ts web/admin-ui/src/
git commit -m "feat: 安装 Vitest 单元测试框架并添加示例测试 (Issue 4.1)"
```

---

## Task 21: 清理未使用的数据库表和占位模块 (P3, Issue 7.1, 5.9)

**Files:**
- Modify: `crates/shared/src/telemetry.rs`（填充或移除）
- 确认 `notification_configs`、`notification_tasks` 表是否需要保留

**Step 1: 清理 telemetry.rs 空模块**

如果不需要独立的 telemetry 模块（observability 模块已涵盖），移除该文件及其在 lib.rs 中的引用。

**Step 2: 为 notification_configs 添加简单的 CRUD（如需要）或添加注释说明其预留用途**

**Step 3: Commit**

```bash
git add crates/shared/src/
git commit -m "chore: 清理空模块和未使用代码 (Issue 5.9, 7.1)"
```

---

## Task 22: 补充 admin_user.created_by 和 benefits.remaining_stock 逻辑 (P3, Issue 7.3)

**Files:**
- Modify: `crates/badge-admin-service/src/handlers/system_user.rs`（创建用户时设置 created_by）
- Modify: `crates/badge-admin-service/src/handlers/benefit.rs`（权益发放时扣减 remaining_stock）

**Step 1: 在 create_user 中设置 created_by**

从 Claims 中获取当前操作人 ID 并写入 `admin_user.created_by`。

**Step 2: 在权益发放流程中扣减 remaining_stock**

```sql
UPDATE benefits SET remaining_stock = remaining_stock - $1
WHERE id = $2 AND remaining_stock >= $1
```

**Step 3: 运行编译验证**

**Step 4: Commit**

```bash
git add crates/badge-admin-service/src/handlers/
git commit -m "fix: 补充 created_by 设置和库存扣减逻辑 (Issue 7.3)"
```

---

## Task 23: 补充 badge_rules 字段使用 (P3, Issue 7.3)

**问题：** `event_type`、`rule_code`、`global_quota`/`global_granted` 字段在 handler 中未读写。

**Files:**
- Modify: `crates/badge-admin-service/src/handlers/rule.rs`（create/update/list 补充这些字段）
- Modify: `crates/badge-admin-service/src/dto/request.rs`（在 CreateRuleRequest 中添加字段）
- Modify: `crates/badge-admin-service/src/dto/response.rs`（在 RuleDto 中添加字段）

**Step 1: 在 CreateRuleRequest 和 UpdateRuleRequest 中添加缺失字段**

```rust
pub event_type: Option<String>,
pub rule_code: Option<String>,
pub global_quota: Option<i64>,
```

**Step 2: 在 create_rule 和 update_rule SQL 中写入这些字段**

**Step 3: 在 RuleDto 和查询中返回这些字段**

**Step 4: 运行编译验证**

**Step 5: Commit**

```bash
git add crates/badge-admin-service/src/handlers/rule.rs crates/badge-admin-service/src/dto/
git commit -m "fix: 补充 badge_rules 表中 event_type/rule_code/global_quota 字段的读写 (Issue 7.3)"
```

---

## 执行顺序总结

| 顺序 | Task | 优先级 | 主题 |
|------|------|--------|------|
| 1 | Task 1 | P0 | 撤销接口参数修复 |
| 2 | Task 2 | P0 | CSV 上传 multipart 支持 |
| 3 | Task 3 | P0 | 任务结果下载修复 |
| 4 | Task 4 | P0 | RedemptionService 初始化 |
| 5 | Task 5 | P0 | 批量任务 Worker |
| 6 | Task 6 | P1 | 审计中间件 |
| 7 | Task 7 | P1 | 规则测试 gRPC 对接 |
| 8 | Task 8 | P1 | 权益同步和链接 |
| 9 | Task 9 | P1 | API Key 限流 |
| 10 | Task 10 | P1 | Token 黑名单 |
| 11 | Task 11 | P1 | 用户筛选预览 |
| 12 | Task 12 | P2 | 权益发放 mock 增强 |
| 13 | Task 13 | P2 | 通知 mock 确认 |
| 14 | Task 14 | P2 | 自动权益发放串联 |
| 15 | Task 15 | P1 | 修复 24 处空断言 |
| 16 | Task 16 | P2 | error.rs 测试覆盖 |
| 17 | Task 17 | P2 | 事件类型 CRUD |
| 18 | Task 18 | P3 | 配置文件完善 |
| 19 | Task 19 | P1 | 安全加固 |
| 20 | Task 20 | P2 | 前端单测框架 |
| 21 | Task 21 | P3 | 清理空模块 |
| 22 | Task 22 | P3 | 字段使用修复 |
| 23 | Task 23 | P3 | 规则字段补充 |
