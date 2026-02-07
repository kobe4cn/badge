# 徽章系统生产就绪性实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将徽章系统从开发状态提升到生产就绪状态，覆盖容器化、核心测试、安全加固、部署流水线和质量提升

**Architecture:** 分 3 个 Sprint 递进执行。Sprint 1 解决部署阻塞（Dockerfile + 核心测试 + Token 刷新 + CD 流水线）；Sprint 2 加固安全和可靠性（安全头 + 优雅关闭 + 回滚脚本 + 前端分包）；Sprint 3 提升质量（前端服务测试 + E2E 断言强化 + 错误追踪 + 运维文档）

**Tech Stack:** Rust (axum/tonic/sqlx) + React 18 (Ant Design Pro + Zustand + React Query) + Docker + GitHub Actions + Playwright

---

## Sprint 1: 部署阻塞项

### Task 1: 创建 Dockerfile 和 .dockerignore

**Files:**
- Create: `.dockerignore`
- Create: `crates/badge-admin-service/Dockerfile`
- Create: `crates/badge-management-service/Dockerfile`
- Create: `crates/unified-rule-engine/Dockerfile`
- Create: `crates/event-engagement-service/Dockerfile`
- Create: `crates/event-transaction-service/Dockerfile`
- Create: `crates/notification-worker/Dockerfile`
- Create: `crates/mock-services/Dockerfile`

**说明：** 所有 7 个应用服务 + mock-services 都需要 Dockerfile。使用多阶段构建：builder 阶段编译，runner 阶段仅包含二进制。所有服务共享同一个 Cargo workspace，因此 builder 需要复制整个 workspace。

#### Step 1: 创建 .dockerignore

```
target/
web/
.git/
*.md
docs/
tests/
benches/
.github/
docker/
test-results/
coverage-report/
*.log
```

#### Step 2: 创建 badge-admin-service Dockerfile

```dockerfile
# ---- Builder ----
FROM rust:1.84-slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev protobuf-compiler && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

RUN cargo build --release --bin badge-admin-service

# ---- Runner ----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 curl && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/badge-admin-service /usr/local/bin/
COPY config/ /app/config/

WORKDIR /app
ENV RUST_LOG=info
EXPOSE 8080

HEALTHCHECK --interval=10s --timeout=5s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

ENTRYPOINT ["badge-admin-service"]
```

#### Step 3: 创建其他 6 个 Dockerfile

每个 Dockerfile 结构相同，仅 `--bin` 名称、端口和 HEALTHCHECK 不同：

| 服务 | bin 名称 | 端口 | 健康检查 |
|------|---------|------|---------|
| badge-management-service | badge-management | 50052 | grpc_health_probe（或 curl 不适用 gRPC，去掉 HEALTHCHECK） |
| unified-rule-engine | unified-rule-engine | 50051 | 同上 |
| event-engagement-service | event-engagement | 50053 | 无 HTTP 端点，去掉 HEALTHCHECK |
| event-transaction-service | event-transaction | 50054 | 同上 |
| notification-worker | notification-worker | 50055 | 同上 |
| mock-services | mock-services | 8090 | curl -f http://localhost:8090/health |

gRPC 服务的 Dockerfile 不加 HEALTHCHECK（依赖 docker-compose 层面的 healthcheck 配置）。Kafka consumer 类服务同理。

#### Step 4: 验证构建

```bash
cd /Users/kevin/dev/ai/badge
docker build -f crates/badge-admin-service/Dockerfile -t badge-admin:test .
```

Expected: 构建成功，镜像约 100-200MB

#### Step 5: 验证运行

```bash
docker run --rm -e BADGE_ENV=development badge-admin:test --help 2>&1 || true
```

Expected: 二进制能启动（可能因缺少数据库连接而退出，但不应 segfault）

---

### Task 2: 补充 GrantService 单元测试

**Files:**
- Create: `crates/badge-management-service/tests/grant_service_test.rs`
- Reference: `crates/badge-management-service/src/service/grant_service.rs`
- Reference: `crates/badge-management-service/tests/badge_flow_test.rs`（现有模式参考）

**说明：** GrantService 是核心发放服务（959 行），包含 10 步业务流程。测试重点覆盖幂等性、库存、用户限额、前置条件、互斥组和级联跳过逻辑。使用 `mockall` 框架 mock `BadgeRepositoryTrait` 和数据库操作。

#### Step 1: 阅读现有测试模式

阅读 `crates/badge-management-service/tests/badge_flow_test.rs` 了解 MockBadgeStore 的实现方式，复用已有的 test fixture 生成函数。

#### Step 2: 创建测试文件

关键测试场景（至少 15 个测试用例）：

```rust
// grant_service_test.rs
use badge_management::*;

// === 基础发放 ===
#[tokio::test]
async fn test_grant_badge_success()                    // 正常发放，quantity > 0
#[tokio::test]
async fn test_grant_badge_incremental()                // 已有记录时增量发放

// === 幂等性 ===
#[tokio::test]
async fn test_grant_idempotent_same_key()              // 相同 idempotency_key 返回已存在
#[tokio::test]
async fn test_grant_different_key_creates_new()        // 不同 key 创建新记录

// === 有效性检查 ===
#[tokio::test]
async fn test_grant_badge_not_found()                  // 徽章不存在 → BadgeNotFound
#[tokio::test]
async fn test_grant_badge_inactive()                   // 徽章未上线 → BadgeInactive

// === 库存检查 ===
#[tokio::test]
async fn test_grant_badge_out_of_stock()               // 库存不足 → BadgeOutOfStock
#[tokio::test]
async fn test_grant_badge_unlimited_stock()            // max_supply = None → 无限量

// === 用户限额 ===
#[tokio::test]
async fn test_grant_user_limit_reached()               // 达到 max_count_per_user → AcquisitionLimitReached
#[tokio::test]
async fn test_grant_no_user_limit()                    // 无限额时正常发放

// === 前置条件 ===
#[tokio::test]
async fn test_grant_prerequisite_not_met()             // 前置徽章缺失 → PrerequisiteNotMet
#[tokio::test]
async fn test_grant_prerequisite_met()                 // 满足前置条件 → 成功

// === 互斥组 ===
#[tokio::test]
async fn test_grant_exclusive_conflict()               // 已有互斥徽章 → ExclusiveConflict

// === 级联发放跳过检查 ===
#[tokio::test]
async fn test_grant_cascade_skips_prerequisite()       // source_type=Cascade 跳过前置条件
#[tokio::test]
async fn test_grant_cascade_skips_exclusive()           // source_type=Cascade 跳过互斥检查

// === 批量发放 ===
#[tokio::test]
async fn test_batch_grant_partial_failure()            // 部分成功统计正确
```

#### Step 3: 运行测试

```bash
cd /Users/kevin/dev/ai/badge
cargo test --test grant_service_test -- --nocapture
```

Expected: 全部 PASS

**注意：** 由于 GrantService 内部直接使用 `sqlx::query_as` 而非通过 Repository trait 进行数据库操作，某些测试可能需要使用 `sqlx::test` 宏配合实际测试数据库。如果无法纯 mock，则使用内存数据库或标记 `#[ignore]` 并在 CI 中通过 `--ignored` 运行。

---

### Task 3: 补充 RevokeService 单元测试

**Files:**
- Create: `crates/badge-management-service/tests/revoke_service_test.rs`
- Reference: `crates/badge-management-service/src/service/revoke_service.rs`

**说明：** RevokeService 处理徽章取消和退款逻辑。测试重点覆盖余额检查、状态变更（归零→Revoked）、退款三种场景（全额/部分保留/部分撤销）和幂等处理。

#### Step 1: 创建测试文件

关键测试场景（至少 12 个测试用例）：

```rust
// === 基础取消 ===
#[tokio::test]
async fn test_revoke_badge_success()                   // 正常取消
#[tokio::test]
async fn test_revoke_badge_partial()                   // 部分取消（quantity < 持有量）
#[tokio::test]
async fn test_revoke_to_zero_sets_revoked()            // 归零后状态变为 Revoked

// === 状态和余额检查 ===
#[tokio::test]
async fn test_revoke_not_found()                       // 用户无此徽章 → UserBadgeNotFound
#[tokio::test]
async fn test_revoke_insufficient_balance()            // 余额不足 → InsufficientBadges
#[tokio::test]
async fn test_revoke_already_revoked()                 // 已取消状态 → 错误

// === 退款处理 ===
#[tokio::test]
async fn test_handle_full_refund_revokes_all()         // 全额退款 → 撤销所有
#[tokio::test]
async fn test_handle_partial_refund_keeps_badge()      // 部分退款仍满足条件 → 保留
#[tokio::test]
async fn test_handle_partial_refund_revokes_badge()    // 部分退款不满足条件 → 撤销

// === 幂等性 ===
#[tokio::test]
async fn test_refund_idempotent()                      // 相同 event_id 不重复处理

// === 批量取消 ===
#[tokio::test]
async fn test_batch_revoke_partial_failure()           // 部分失败统计
#[tokio::test]
async fn test_batch_revoke_all_success()               // 全部成功
```

#### Step 2: 运行测试

```bash
cargo test --test revoke_service_test -- --nocapture
```

---

### Task 4: 补充 RedemptionService 单元测试

**Files:**
- Create: `crates/badge-management-service/tests/redemption_service_test.rs`
- Reference: `crates/badge-management-service/src/service/redemption_service.rs`

**说明：** RedemptionService 处理徽章兑换权益流程（13 步）。测试重点覆盖规则有效性、权益库存、多徽章消耗、订单状态机和幂等性。

#### Step 1: 创建测试文件

关键测试场景（至少 10 个测试用例）：

```rust
// === 基础兑换 ===
#[tokio::test]
async fn test_redeem_badge_success()                   // 单徽章兑换成功
#[tokio::test]
async fn test_redeem_multi_badge()                     // 多种徽章组合兑换

// === 幂等性 ===
#[tokio::test]
async fn test_redeem_idempotent()                      // 相同 idempotency_key 返回已存在

// === 规则有效性 ===
#[tokio::test]
async fn test_redeem_rule_not_found()                  // 规则不存在 → RedemptionRuleNotFound
#[tokio::test]
async fn test_redeem_rule_inactive()                   // 规则未启用 → RedemptionRuleInactive
#[tokio::test]
async fn test_redeem_rule_expired()                    // 超出时间范围 → 错误

// === 权益检查 ===
#[tokio::test]
async fn test_redeem_benefit_out_of_stock()            // 权益库存不足 → BenefitOutOfStock

// === 余额检查 ===
#[tokio::test]
async fn test_redeem_insufficient_badges()             // 徽章余额不足 → InsufficientBadges

// === 状态变更 ===
#[tokio::test]
async fn test_redeem_badge_to_zero_sets_redeemed()     // 兑换后归零 → Redeemed 状态

// === 查询历史 ===
#[tokio::test]
async fn test_get_user_redemptions()                   // 查询兑换历史
```

#### Step 2: 运行测试

```bash
cargo test --test redemption_service_test -- --nocapture
```

---

### Task 5: 激活后端 E2E 测试

**Files:**
- Modify: `.github/workflows/e2e-tests.yml`
- Modify: `tests/e2e/suites/data_consistency.rs`
- Modify: `tests/e2e/suites/event_trigger.rs`
- Modify: `tests/e2e/suites/cascade_trigger.rs`
- Modify: `tests/e2e/suites/redemption.rs`
- Modify: `tests/e2e/suites/notification.rs`

**说明：** 当前所有后端 E2E 测试都标记 `#[ignore = "需要运行服务"]`。CI 中使用 `-- --ignored` 来运行它们，这本身是正确的（E2E 测试需要服务启动后才能跑）。问题不在 `#[ignore]` 标记，而在于 CI 中服务启动后的等待和健康检查不够可靠。

**实际修改策略：**

1. **保留 `#[ignore]`** — 这是正确的做法，避免 `cargo test` 默认运行 E2E
2. **优化 CI 中的服务启动等待逻辑** — 用健康检查轮询代替固定 `sleep 30`
3. **添加环境变量 `SKIP_HEALTH_CHECK`** 的 CI 默认值

#### Step 1: 修改 CI 工作流

在 `.github/workflows/e2e-tests.yml` 的 backend-e2e job 中，将 `sleep 30` 替换为健康检查轮询：

```yaml
# 替换固定 sleep，改为轮询健康检查
- name: Wait for services to be ready
  run: |
    echo "Waiting for badge-admin-service..."
    for i in $(seq 1 60); do
      if curl -sf http://localhost:8080/health > /dev/null 2>&1; then
        echo "badge-admin-service is ready"
        break
      fi
      echo "Attempt $i/60..."
      sleep 2
    done
    curl -sf http://localhost:8080/health || (echo "Service not ready after 120s" && exit 1)
```

#### Step 2: 确保 E2E 测试在 CI 中实际执行

验证 CI 中的命令：
```bash
cargo test --test e2e -- --ignored --test-threads=1 2>&1 | head -50
```

确认输出包含实际测试执行（非 0 tests），如果全部被跳过说明服务未就绪。

#### Step 3: 本地验证（可选）

```bash
# 启动基础设施
make infra-up
# 启动服务
make dev-backend &
sleep 10
# 运行 E2E
cargo test --test e2e -- --ignored --test-threads=1 --nocapture
```

---

### Task 6: 实现 Token 自动刷新

**Files:**
- Modify: `web/admin-ui/src/services/api.ts`（响应拦截器）
- Modify: `web/admin-ui/src/services/auth.ts`（refreshToken 已定义，需调用）
- Modify: `web/admin-ui/src/stores/authStore.ts`（添加 updateToken 方法）

**说明：** 当前 401 响应直接清除认证状态并重定向。需要改为：401 时自动尝试 refreshToken()，成功则重试原请求，失败才清除状态。并发请求需要排队等待刷新完成。

#### Step 1: 在 authStore 中添加 updateToken

```typescript
// authStore.ts - 添加 updateToken action
updateToken: (newToken: string) => {
  localStorage.setItem('auth_token', newToken);
  set({ token: newToken });
}
```

#### Step 2: 修改响应拦截器

在 `api.ts` 中实现刷新队列机制：

```typescript
import { refreshToken } from './auth';

let isRefreshing = false;
let failedQueue: Array<{
  resolve: (token: string) => void;
  reject: (error: unknown) => void;
}> = [];

function processQueue(error: unknown, token: string | null) {
  failedQueue.forEach(({ resolve, reject }) => {
    if (error) reject(error);
    else resolve(token!);
  });
  failedQueue = [];
}
```

在 401 分支中（非登录接口），替换 `clearAuthAndRedirect()` 为：

```typescript
case 401:
  if (error.config?.url?.includes('/auth/login') || error.config?.url?.includes('/auth/refresh')) {
    // 登录或刷新接口本身 401，不重试
    if (!error.config.url.includes('/auth/login')) {
      clearAuthAndRedirect();
    }
  } else {
    // 非登录接口 401，尝试刷新 token
    if (isRefreshing) {
      // 已有刷新请求进行中，排队等待
      return new Promise((resolve, reject) => {
        failedQueue.push({ resolve, reject });
      }).then((token) => {
        error.config!.headers.Authorization = `Bearer ${token}`;
        return apiClient(error.config!);
      });
    }

    isRefreshing = true;
    try {
      const { token: newToken } = await refreshToken();
      // 更新存储
      localStorage.setItem('auth_token', newToken);
      const { getAuthState } = await import('@/stores/authStore');
      getAuthState().updateToken(newToken);
      // 重试原请求
      error.config!.headers.Authorization = `Bearer ${newToken}`;
      processQueue(null, newToken);
      return apiClient(error.config!);
    } catch (refreshError) {
      processQueue(refreshError, null);
      clearAuthAndRedirect();
      return Promise.reject(refreshError);
    } finally {
      isRefreshing = false;
    }
  }
  break;
```

#### Step 3: 验证

```bash
cd /Users/kevin/dev/ai/badge/web/admin-ui && npm run build
```

Expected: 编译通过，无 TypeScript 错误

---

### Task 7: 创建 CD 部署工作流

**Files:**
- Create: `.github/workflows/deploy.yml`

**说明：** CI/CD 流水线：main 分支合并后自动构建 Docker 镜像并推送到 GitHub Container Registry (ghcr.io)。暂不实现自动部署到生产（需要目标环境信息），但提供 `workflow_dispatch` 手动触发部署。

#### Step 1: 创建 deploy.yml

```yaml
name: Build & Push Docker Images

on:
  push:
    branches: [main]
    paths-ignore:
      - '*.md'
      - 'docs/**'
      - 'web/**'
  workflow_dispatch:
    inputs:
      services:
        description: 'Services to build (comma-separated, or "all")'
        default: 'all'
        required: true

env:
  REGISTRY: ghcr.io
  IMAGE_PREFIX: ${{ github.repository }}

jobs:
  build-and-push:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write

    strategy:
      matrix:
        service:
          - name: badge-admin-service
            dockerfile: crates/badge-admin-service/Dockerfile
          - name: badge-management-service
            dockerfile: crates/badge-management-service/Dockerfile
          - name: unified-rule-engine
            dockerfile: crates/unified-rule-engine/Dockerfile
          - name: event-engagement-service
            dockerfile: crates/event-engagement-service/Dockerfile
          - name: event-transaction-service
            dockerfile: crates/event-transaction-service/Dockerfile
          - name: notification-worker
            dockerfile: crates/notification-worker/Dockerfile

    steps:
      - uses: actions/checkout@v4

      - name: Log in to Container Registry
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Build and push
        uses: docker/build-push-action@v6
        with:
          context: .
          file: ${{ matrix.service.dockerfile }}
          push: true
          tags: |
            ${{ env.REGISTRY }}/${{ env.IMAGE_PREFIX }}/${{ matrix.service.name }}:latest
            ${{ env.REGISTRY }}/${{ env.IMAGE_PREFIX }}/${{ matrix.service.name }}:${{ github.sha }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

  build-frontend:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write

    steps:
      - uses: actions/checkout@v4

      - name: Log in to Container Registry
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 20

      - name: Build frontend
        working-directory: web/admin-ui
        run: npm ci && npm run build

      - name: Build and push nginx image
        uses: docker/build-push-action@v6
        with:
          context: web/admin-ui
          file: web/admin-ui/Dockerfile
          push: true
          tags: |
            ${{ env.REGISTRY }}/${{ env.IMAGE_PREFIX }}/admin-ui:latest
            ${{ env.REGISTRY }}/${{ env.IMAGE_PREFIX }}/admin-ui:${{ github.sha }}
```

#### Step 2: 创建前端 Dockerfile

**Create: `web/admin-ui/Dockerfile`**

```dockerfile
FROM nginx:1.27-alpine
COPY dist/ /usr/share/nginx/html/
COPY nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
```

#### Step 3: 创建 nginx 配置

**Create: `web/admin-ui/nginx.conf`**

```nginx
server {
    listen 80;
    root /usr/share/nginx/html;
    index index.html;

    location / {
        try_files $uri $uri/ /index.html;
    }

    location /api/ {
        proxy_pass http://badge-admin-service:8080/api/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

#### Step 4: 验证

```bash
cd /Users/kevin/dev/ai/badge/web/admin-ui && npm run build
docker build -t admin-ui:test .
```

---

## Sprint 2: 安全加固

### Task 8: 添加 HTTP 安全头中间件

**Files:**
- Modify: `crates/badge-admin-service/src/main.rs:153-173`

**说明：** 在 Router 构建链中添加安全头。使用 `tower-http` 的 `SetResponseHeader` 或自定义 middleware。

#### Step 1: 添加安全头 middleware

在 `main.rs` 的 Router 构建中，在 CORS 层之后添加：

```rust
use axum::http::header;
use tower_http::set_header::SetResponseHeaderLayer;

let app = Router::new()
    // ... 现有路由 ...
    .layer(SetResponseHeaderLayer::if_not_present(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    ))
    .layer(SetResponseHeaderLayer::if_not_present(
        header::X_FRAME_OPTIONS,
        HeaderValue::from_static("DENY"),
    ))
    .layer(SetResponseHeaderLayer::if_not_present(
        HeaderName::from_static("x-xss-protection"),
        HeaderValue::from_static("1; mode=block"),
    ))
    .layer(SetResponseHeaderLayer::if_not_present(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    ))
    // ... 审计、CORS、认证中间件 ...
```

#### Step 2: 验证

```bash
cargo build --bin badge-admin-service
```

---

### Task 9: 实现 HTTP 优雅关闭

**Files:**
- Modify: `crates/badge-admin-service/src/main.rs:175-178`

**说明：** 当前 `axum::serve(listener, app).await?` 没有配置 graceful shutdown。容器编排发送 SIGTERM 时，需要停止接收新请求并等待现有请求完成。

#### Step 1: 添加信号处理

```rust
// 在 main() 函数中，替换最后的 axum::serve 调用：

// 监听关闭信号：SIGTERM (K8s) 和 Ctrl+C（本地开发）
let shutdown = async {
    let ctrl_c = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let mut sigterm = tokio::signal::unix::signal(
            tokio::signal::unix::SignalKind::terminate(),
        ).expect("注册 SIGTERM 失败");
        tokio::select! {
            _ = ctrl_c => info!("Received Ctrl+C, shutting down..."),
            _ = sigterm.recv() => info!("Received SIGTERM, shutting down..."),
        }
    }
    #[cfg(not(unix))]
    {
        ctrl_c.await.expect("Failed to listen for Ctrl+C");
        info!("Received Ctrl+C, shutting down...");
    }
};

axum::serve(listener, app)
    .with_graceful_shutdown(shutdown)
    .await?;

info!("Server shutdown complete");
```

#### Step 2: 验证

```bash
cargo build --bin badge-admin-service
```

---

### Task 10: 补全数据库回滚脚本

**Files:**
- Create: `migrations/rollback/20250128_001_init_schema_down.sql`
- Create: `migrations/rollback/20250130_001_badge_dependency_down.sql`
- Create: `migrations/rollback/20250131_001_cascade_log_down.sql`
- Create: `migrations/rollback/20250201_001_user_badge_logs_down.sql`
- Create: `migrations/rollback/20250202_001_dynamic_rules_down.sql`
- Create: `migrations/rollback/20250204_001_rule_templates_down.sql`
- Create: `migrations/rollback/20250209_001_fix_password_hash_down.sql`
- Create: `migrations/rollback/20250210_001_batch_task_failures_down.sql`

**说明：** 已有 3 个回滚脚本（benefit_grants, auto_benefit, auth_rbac）。补全其余 8 个。回滚脚本应按逆序删除对应迁移创建的表、索引和触发器。

#### Step 1: 读取每个迁移文件

读取每个 `migrations/202501*.sql` 文件，提取 CREATE TABLE/INDEX/FUNCTION/TRIGGER 语句。

#### Step 2: 创建对应的 DROP 语句

每个回滚脚本的模式：

```sql
-- 20250128_001_init_schema_down.sql
-- 按依赖关系逆序删除
DROP TABLE IF EXISTS user_badge_logs CASCADE;
DROP TABLE IF EXISTS badge_ledger CASCADE;
DROP TABLE IF EXISTS user_badges CASCADE;
DROP TABLE IF EXISTS badge_rules CASCADE;
DROP TABLE IF EXISTS badges CASCADE;
DROP TABLE IF EXISTS badge_series CASCADE;
DROP TABLE IF EXISTS badge_categories CASCADE;
DROP TABLE IF EXISTS operation_logs CASCADE;
DROP FUNCTION IF EXISTS update_updated_at_column() CASCADE;
DROP EXTENSION IF EXISTS pg_trgm;
```

#### Step 3: 验证回滚脚本语法

```bash
# 连接测试数据库验证语法
psql $DATABASE_URL -f migrations/rollback/20250128_001_init_schema_down.sql --echo-errors
```

---

### Task 11: 前端代码分割优化

**Files:**
- Modify: `web/admin-ui/vite.config.ts`

**说明：** 当前两个 chunk 超过 1MB（gzip 后 ~350KB）。将 antd、echarts、xyflow 分离到独立 chunk，利用浏览器缓存减少重复加载。

#### Step 1: 添加 manualChunks 配置

在 `vite.config.ts` 的 `build` 配置中添加：

```typescript
build: {
  rollupOptions: {
    output: {
      manualChunks: {
        'vendor-antd': ['antd', '@ant-design/icons'],
        'vendor-pro': ['@ant-design/pro-components'],
        'vendor-charts': ['echarts', 'echarts-for-react'],
        'vendor-flow': ['@xyflow/react'],
        'vendor-core': ['react', 'react-dom', 'react-router-dom'],
      },
    },
  },
},
```

#### Step 2: 验证

```bash
cd /Users/kevin/dev/ai/badge/web/admin-ui && npm run build
```

Expected: 无 chunk 超过 500KB 的警告（或至少大幅减少）

---

## Sprint 3: 质量提升

### Task 12: 前端服务层单元测试

**Files:**
- Create: `web/admin-ui/src/services/__tests__/auth.test.ts`
- Create: `web/admin-ui/src/services/__tests__/api.test.ts`

**说明：** 前端 5 个 service 文件 (auth, api, badge, system, grant) 零测试。优先测试 auth 和 api 这两个核心文件，验证登录、登出、token 注入、错误处理和刷新队列逻辑。

#### Step 1: 创建 auth.test.ts

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock api 模块
vi.mock('./api', () => ({
  post: vi.fn(),
  get: vi.fn(),
}));

import { login, logout, getCurrentUser, refreshToken } from '../auth';
import { post, get } from '../api';

describe('auth service', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('login', () => {
    it('正确转换后端响应格式', async () => {
      // mock 后端返回
      (post as ReturnType<typeof vi.fn>).mockResolvedValue({
        token: 'jwt-token',
        user: { id: 1, username: 'admin', displayName: 'Admin' },
        permissions: ['system:user:write', 'badge:view'],
      });

      const result = await login('admin', 'password');
      expect(result.token).toBe('jwt-token');
      expect(result.user.roles).toContain('admin');
      expect(result.user.permissions).toContain('system:user:write');
    });

    it('无 system:write 权限推导为 operator', async () => { ... });
    it('无任何 write 权限推导为 viewer', async () => { ... });
  });

  describe('logout', () => {
    it('调用后端登出 API', async () => { ... });
  });

  describe('refreshToken', () => {
    it('调用 /admin/auth/refresh', async () => { ... });
  });
});
```

#### Step 2: 创建 api.test.ts

测试拦截器行为：
- token 注入
- 401 响应处理
- 网络错误处理
- 超时处理

#### Step 3: 运行测试

```bash
cd /Users/kevin/dev/ai/badge/web/admin-ui && npx vitest run
```

---

### Task 13: 强化 E2E 断言

**Files:**
- Modify: `web/admin-ui/e2e/specs/api-integration.spec.ts`

**说明：** 当前大量使用 `expect(result).toBeTruthy()` 和 `expect(x || y || z).toBeTruthy()` 等弱断言。这些断言无法区分成功和错误状态。需要替换为具体的值断言。

#### Step 1: 搜索弱断言

```bash
grep -n "toBeTruthy\|toBeFalsy" web/admin-ui/e2e/specs/api-integration.spec.ts | head -30
```

#### Step 2: 逐个替换

替换规则：
```typescript
// ❌ 弱断言
expect(res?.data?.id).toBeTruthy();
// ✅ 强断言
expect(res?.data?.id).toBeDefined();
expect(typeof res?.data?.id).toBe('number');

// ❌ 过度容错
expect(res?.data || res?.code === 0 || res?.success).toBeTruthy();
// ✅ 明确检查
expect(res?.data).toBeDefined();
expect(res?.code).toBe(0);

// ❌ 布尔值用 toBeTruthy
expect(hasError).toBeTruthy();
// ✅ 明确
expect(hasError).toBe(true);
```

#### Step 3: 运行验证

```bash
cd /Users/kevin/dev/ai/badge/web/admin-ui && npx playwright test e2e/specs/api-integration.spec.ts --reporter=list 2>&1 | tail -20
```

---

### Task 14: 生产运维文档

**Files:**
- Create: `docs/deployment-guide.md`

**说明：** 仅在用户明确要求时创建。文档应包含：

1. **环境变量参考** — 列出所有必需和可选的环境变量
2. **Docker Compose 部署** — 生产环境 docker-compose 配置
3. **数据库迁移步骤** — 首次和增量迁移
4. **健康检查端点** — 所有服务的存活和就绪探针
5. **监控配置** — Prometheus + Grafana + Jaeger 配置
6. **回滚流程** — 服务和数据库的回滚步骤
7. **常见问题排查** — 连接失败、token 过期、Kafka 消费延迟等

#### Step 1: 汇总所有环境变量

从 `docker/.env.example`、`config/*.toml` 和源码中的 `std::env::var` 调用中提取完整列表。

#### Step 2: 编写部署指南

包含完整的部署命令序列和预期输出。

---

## 全局验证

每个 Sprint 完成后执行：

```bash
# 后端编译
cd /Users/kevin/dev/ai/badge && cargo build --workspace

# 后端测试（非 E2E）
cargo test --workspace

# 前端编译
cd web/admin-ui && npm run build

# 前端单元测试
npx vitest run

# Docker 构建验证（Sprint 1 完成后）
docker build -f crates/badge-admin-service/Dockerfile -t badge-admin:test .
```
