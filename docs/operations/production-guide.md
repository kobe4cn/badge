# 徽章系统生产运维指南

## 1. 系统架构概览

### 1.1 服务列表与职责

| 服务名称 | 类型 | 端口 | 指标端口 | 职责 |
|---------|------|------|---------|------|
| `unified-rule-engine` | gRPC | 50051 | 9990 | 规则引擎核心，负责规则评估和匹配 |
| `badge-management-service` | gRPC | 50052 | 9992 | 徽章管理核心，处理徽章发放、撤回、依赖级联 |
| `badge-admin-service` | HTTP | 8080 | 9991 | B端管理后台 REST API，包含认证、RBAC、CRUD |
| `event-engagement-service` | gRPC + Kafka | 50053 | 9993 | 互动事件消费者，处理签到、浏览、分享等行为事件 |
| `event-transaction-service` | gRPC + Kafka | 50054 | 9994 | 交易事件消费者，处理购买、退款等交易事件 |
| `notification-worker` | Kafka | 50055 | 9995 | 通知工作者，消费通知队列并推送消息 |
| `admin-ui` | Nginx | 80 | - | React 管理后台前端，Nginx 反向代理 API |

### 1.2 服务间依赖关系

```
                          ┌─────────────────┐
                          │    admin-ui      │
                          │   (Nginx :80)    │
                          └────────┬─────────┘
                                   │ HTTP 反向代理
                                   ▼
                          ┌─────────────────┐
              ┌──────────►│ badge-admin-svc  │◄──────────┐
              │ gRPC      │   (HTTP :8080)   │  gRPC     │
              │           └────────┬─────────┘           │
              │                    │                      │
    ┌─────────┴────────┐           │         ┌───────────┴──────────┐
    │ badge-mgmt-svc   │           │         │ unified-rule-engine  │
    │  (gRPC :50052)   │           │         │   (gRPC :50051)      │
    └──────────────────┘           │         └──────────────────────┘
              ▲                    │                      ▲
              │           ┌───────┴────────┐             │
              │           │     Kafka      │             │
              │           │ (broker :9092) │             │
              │           └───┬────────┬───┘             │
              │               │        │                 │
    ┌─────────┴──────┐  ┌────┴────┐ ┌─┴──────────────┐  │
    │ event-engage   │  │ notif-  │ │ event-txn-svc  ├──┘
    │ (Kafka :50053) │  │ worker  │ │ (Kafka :50054) │
    └────────────────┘  │ (:50055)│ └────────────────┘
                        └─────────┘
```

**依赖说明：**

- `badge-admin-service` 通过 gRPC 调用 `badge-management-service`（刷新依赖缓存）和 `unified-rule-engine`（规则测试）。这两个连接失败不会阻止 admin 服务启动，只是对应功能降级不可用。这种设计确保管理后台在上游服务故障时仍能提供基本的 CRUD 操作。
- `event-engagement-service` 和 `event-transaction-service` 从 Kafka 消费事件，调用规则引擎评估后通过 `badge-management-service` 发放徽章。
- `notification-worker` 消费 `badge.notifications` Topic，是纯消费者角色。

### 1.3 基础设施依赖

| 组件 | 镜像版本 | 用途 |
|------|---------|------|
| PostgreSQL | `postgres:16-alpine` | 主数据库，存储所有业务数据 |
| Redis | `redis:7-alpine` | 缓存层（依赖图、规则、Token 黑名单）和 API Key 限流计数 |
| Kafka | `confluentinc/cp-kafka:7.5.0` | 事件驱动消息队列 |
| ZooKeeper | `confluentinc/cp-zookeeper:7.5.0` | Kafka 集群协调 |
| Elasticsearch | `elasticsearch:8.19.10` | 日志搜索和分析（可选） |

---

## 2. 部署流程

### 2.1 前置准备

#### 2.1.1 环境变量配置清单

以下变量来自 `docker/.env.example`，**生产环境必须修改默认值**：

**数据库连接：**

```bash
POSTGRES_USER=badge                    # PostgreSQL 用户名
POSTGRES_PASSWORD=badge_secret         # 生产环境必须使用强密码
POSTGRES_DB=badge_db                   # 数据库名
DATABASE_URL=postgres://badge:badge_secret@localhost:5432/badge_db
```

**缓存与消息队列：**

```bash
REDIS_URL=redis://localhost:6379
KAFKA_BROKERS=localhost:9092
ELASTICSEARCH_URL=http://localhost:9200    # 可选
```

**安全配置（生产环境关键项）：**

```bash
# JWT 密钥 — 生产环境必须设置，否则服务启动时会 panic
# 至少 32 个字符的随机字符串
BADGE_JWT_SECRET=your-strong-random-secret-here-min-32-chars

# Token 过期时间（秒），默认 86400（24小时）
BADGE_JWT_EXPIRES_SECS=86400

# 环境标识 — 设为 production 时强制要求 JWT_SECRET 已配置
BADGE_ENV=production

# CORS 允许来源 — 生产环境必须设为实际域名，禁止使用 *
# 多个域名用逗号分隔
BADGE_CORS_ORIGINS=https://admin.example.com
```

**服务端口：**

```bash
RULE_ENGINE_PORT=50051
BADGE_MANAGEMENT_PORT=50052
BADGE_ADMIN_PORT=8080
EVENT_ENGAGEMENT_PORT=50053
EVENT_TRANSACTION_PORT=50054
NOTIFICATION_WORKER_PORT=50055
```

**Metrics 端口（Prometheus 采集用）：**

```bash
RULE_ENGINE_METRICS_PORT=9090
BADGE_MANAGEMENT_METRICS_PORT=9091
BADGE_ADMIN_METRICS_PORT=9092
EVENT_ENGAGEMENT_METRICS_PORT=9093
EVENT_TRANSACTION_METRICS_PORT=9094
NOTIFICATION_WORKER_METRICS_PORT=9095
```

**gRPC 服务间通信：**

```bash
# badge-admin-service 连接上游 gRPC 服务的地址
BADGE_MANAGEMENT_GRPC_ADDR=http://127.0.0.1:50052
RULE_ENGINE_GRPC_ADDR=http://127.0.0.1:50051

# event-transaction-service 连接配置
RULE_ENGINE_URL=http://localhost:50051
BADGE_SERVICE_URL=http://localhost:50052
```

#### 2.1.2 生产环境配置覆盖

系统使用分层配置：`config/default.toml` → `config/production.toml`。生产环境自动覆盖的关键参数：

```toml
# config/production.toml
[database]
max_connections = 50       # 连接池上限（默认 10，生产需根据实例数调整）
min_connections = 10       # 最小保持连接数
connect_timeout_seconds = 10
idle_timeout_seconds = 300

[redis]
pool_size = 50             # Redis 连接池（默认 10）

[observability]
log_level = "info"
log_format = "json"        # 生产环境使用 JSON 格式便于日志平台采集
metrics_enabled = true
tracing_enabled = true     # 生产环境开启分布式追踪
```

#### 2.1.3 Kafka Topic 初始化

服务启动前必须创建以下 Topic：

```bash
# 互动事件（签到、浏览、分享等），3 分区保证并行消费能力
kafka-topics --bootstrap-server localhost:9092 --create --if-not-exists \
  --topic badge.engagement.events --partitions 3 --replication-factor 1

# 交易事件（购买、退款等），3 分区
kafka-topics --bootstrap-server localhost:9092 --create --if-not-exists \
  --topic badge.transaction.events --partitions 3 --replication-factor 1

# 通知消息
kafka-topics --bootstrap-server localhost:9092 --create --if-not-exists \
  --topic badge.notifications --partitions 3 --replication-factor 1

# 死信队列 — 1 分区即可，处理失败的消息不需要高吞吐
kafka-topics --bootstrap-server localhost:9092 --create --if-not-exists \
  --topic badge.dlq --partitions 1 --replication-factor 1
```

> **为什么使用 3 个分区：** 事件处理服务通常部署 2-3 个实例，分区数不小于消费者实例数才能充分利用并行能力。死信队列只用于人工排查，1 个分区足够。

使用 Makefile 快捷命令：`make kafka-init`

### 2.2 服务部署

#### 2.2.1 Docker 镜像构建

所有后端服务使用多阶段构建，构建阶段基于 `rust:1.84-slim-bookworm`，运行阶段基于 `debian:bookworm-slim`：

```bash
# 构建单个后端服务
docker build -t badge-admin-service:latest \
  -f crates/badge-admin-service/Dockerfile .

# 构建前端（需要先在宿主机构建前端产物）
cd web/admin-ui && npm ci && npm run build
docker build -t admin-ui:latest -f Dockerfile .
```

镜像命名规范（GitHub Container Registry）：

```
ghcr.io/{owner}/{repo}/{service}:{tag}
```

例如：`ghcr.io/org/badge/badge-admin-service:abc1234`

每次构建产生两个 tag：`latest` 和 git commit SHA，便于精确回滚。

#### 2.2.2 服务启动顺序

严格按照以下顺序启动，每一步都等待前一步健康检查通过后再继续：

```
1. 基础设施层
   PostgreSQL → Redis → ZooKeeper → Kafka → (Elasticsearch)

2. 数据库迁移
   执行 migrations/ 下的 SQL 文件（详见 §4.1）

3. Kafka Topic 创建
   执行 kafka-init（详见 §2.1.3）

4. 核心后端服务（无外部依赖，可并行启动）
   unified-rule-engine + badge-management-service

5. 管理后台服务
   badge-admin-service（启动时尝试连接 rule-engine 和 badge-mgmt）

6. 事件消费服务（依赖 Kafka 和上游 gRPC 服务）
   event-engagement-service + event-transaction-service + notification-worker

7. 前端
   admin-ui (Nginx)
```

> **为什么 admin-service 可以在上游 gRPC 服务未就绪时启动：** 代码中对 gRPC 连接失败做了容错处理（`warn!` 而非 `panic!`），连接失败时规则测试和跨服务缓存刷新功能降级，但基本的 CRUD 和认证功能不受影响。

#### 2.2.3 健康检查端点

**badge-admin-service（HTTP 服务）：**

```bash
# 存活探针 — 进程正常即返回 200
GET /health
# 响应: {"status": "ok", "service": "badge-admin-service"}

# 就绪探针 — 检查 PostgreSQL 和 Redis 连接
GET /ready
# 响应（正常）: {"status": "ok", "service": "badge-admin-service", "checks": {"database": "ok", "redis": "ok"}}
# 响应（降级）: {"status": "degraded", "checks": {"database": "ok", "redis": "fail"}}
```

**Docker 健康检查配置（已内置于 Dockerfile）：**

```dockerfile
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1
```

**基础设施健康检查：**

| 组件 | 检查方式 | 间隔 | 超时 | 重试 |
|------|---------|------|------|------|
| PostgreSQL | `pg_isready -U badge` | 5s | 5s | 5 次 |
| Redis | `redis-cli ping` | 5s | 5s | 5 次 |
| ZooKeeper | `echo ruok \| nc localhost 2181` | 5s | 5s | 5 次 |
| Kafka | `kafka-topics --list` | 10s | 10s | 5 次 |
| Elasticsearch | `curl /_cluster/health` | 10s | 10s | 5 次 |

### 2.3 CI/CD 流程

#### 2.3.1 GitHub Actions 自动部署

部署流水线定义在 `.github/workflows/deploy.yml`：

**触发条件：**
- `push` 到 `main` 分支：自动构建全部服务
- `workflow_dispatch` 手动触发：可指定构建特定服务

**后端构建：** 使用 matrix 策略并行构建 6 个 Rust 微服务镜像，推送到 GitHub Container Registry (ghcr.io)。设置 `fail-fast: false` 确保单个服务构建失败不影响其他服务。

**前端构建：** 在 CI 环境执行 `npm ci && npm run build` 生成前端产物，再打包为 Nginx 镜像。

**并发控制：** 同一分支的多次推送只保留最新一次构建：

```yaml
concurrency:
  group: deploy-${{ github.ref }}
  cancel-in-progress: true
```

#### 2.3.2 手动触发部署

在 GitHub Actions 页面点击 "Run workflow"，`services` 参数支持：
- `all` — 构建全部服务
- `badge-admin-service` — 仅构建指定服务
- `badge-admin-service,admin-ui` — 逗号分隔构建多个服务

```bash
# 或通过 CLI 触发
gh workflow run deploy.yml -f services=badge-admin-service
```

#### 2.3.3 回滚操作

```bash
# 回滚到指定 commit 的镜像版本
docker pull ghcr.io/{owner}/{repo}/badge-admin-service:{previous-commit-sha}
docker stop badge-admin-service
docker run -d --name badge-admin-service \
  ghcr.io/{owner}/{repo}/badge-admin-service:{previous-commit-sha}

# 如果需要同时回滚数据库，参见 §4.1.3 回滚步骤
```

---

## 3. 监控与告警

### 3.1 健康检查

建议的 Kubernetes 探针配置：

```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 30
  timeoutSeconds: 3
  failureThreshold: 3

readinessProbe:
  httpGet:
    path: /ready
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 10
  timeoutSeconds: 5
  failureThreshold: 3
```

> **为什么 readiness 检查间隔更短：** readiness 失败会从负载均衡移除实例，需要更快速地发现和恢复。liveness 触发 Pod 重启，过于频繁会造成不必要的服务闪断。

### 3.2 日志

#### 3.2.1 日志级别配置

通过 `RUST_LOG` 环境变量控制：

```bash
# 生产推荐：全局 info，特定模块 debug
RUST_LOG=info

# 排查问题时临时调整
RUST_LOG=badge_admin_service=debug,sqlx=warn

# 极端排查：开启全量 trace（会产生大量日志，仅短期使用）
RUST_LOG=trace
```

#### 3.2.2 日志格式

生产环境在 `config/production.toml` 中配置 `log_format = "json"`，输出结构化 JSON 日志便于 ELK/Loki 等平台采集：

```json
{"timestamp":"2025-02-04T10:30:00Z","level":"INFO","target":"badge_admin_service","message":"Listening on 0.0.0.0:8080","request_id":"abc-123"}
```

#### 3.2.3 日志收集建议

- 使用 sidecar 或 DaemonSet 模式采集容器 stdout/stderr
- 保留至少 30 天的日志用于审计和故障分析
- 对 `level=ERROR` 配置实时告警

### 3.3 关键指标

每个服务暴露 Prometheus 格式的 metrics 端点，端口见 §1.1 表格。

#### 3.3.1 API 响应时间

| 指标 | 告警阈值 | 说明 |
|------|---------|------|
| P99 延迟 | > 500ms | REST API 端点 P99 延迟过高 |
| P95 延迟 | > 200ms | 日常监控基线 |
| 错误率 | > 1% | 5xx 响应占比超过阈值 |

#### 3.3.2 数据库连接池

| 指标 | 告警阈值 | 说明 |
|------|---------|------|
| 活跃连接数 | > 40 (80%) | 生产环境 `max_connections=50`，接近上限需扩容 |
| 等待连接的请求 | > 0 持续 30s | 连接池耗尽，请求开始排队 |
| 连接获取超时 | 任何出现 | 配置的 `connect_timeout_seconds=10` 内未获取到连接 |

#### 3.3.3 Kafka 消费延迟

| 指标 | 告警阈值 | 说明 |
|------|---------|------|
| Consumer Lag | > 10000 条 | 消费者积压过多，可能需要扩容消费者实例 |
| DLQ 消息数 | > 0 | 死信队列出现消息，需要人工排查处理失败原因 |

#### 3.3.4 Redis

| 指标 | 告警阈值 | 说明 |
|------|---------|------|
| 连接池使用率 | > 80% | 生产环境 `pool_size=50` |
| 命中率 | < 80% | 缓存效果下降，检查是否有大量缓存穿透 |
| 内存使用 | > 80% maxmemory | 需要清理或扩容 |

---

## 4. 数据库运维

### 4.1 迁移管理

#### 4.1.1 迁移文件命名规范

```
{YYYYMMDD}_{sequence}_{description}.sql
```

例如：`20250128_001_init_schema.sql`

- 日期前缀确保文件按时间排序
- 序列号区分同一天的多个迁移
- 描述使用下划线分隔的英文小写

#### 4.1.2 迁移文件清单与执行顺序

以下是完整的迁移文件列表，**必须严格按序执行**：

```bash
# 1. 基础表结构
psql -U badge -d badge_db < migrations/20250128_001_init_schema.sql

# 2. 徽章依赖关系
psql -U badge -d badge_db < migrations/20250130_001_badge_dependency.sql

# 3. 级联操作日志
psql -U badge -d badge_db < migrations/20250131_001_cascade_log.sql

# 4. 用户徽章日志
psql -U badge -d badge_db < migrations/20250201_001_user_badge_logs.sql

# 5. 动态规则
psql -U badge -d badge_db < migrations/20250202_001_dynamic_rules.sql

# 6. Schema 对齐
psql -U badge -d badge_db < migrations/20250203_001_schema_alignment.sql

# 7. 规则模板
psql -U badge -d badge_db < migrations/20250204_001_rule_templates.sql

# 8. 种子模板数据
psql -U badge -d badge_db < migrations/20250205_001_seed_templates.sql

# 9. 权益发放
psql -U badge -d badge_db < migrations/20250206_001_benefit_grants.sql

# 10. 自动权益
psql -U badge -d badge_db < migrations/20250207_001_auto_benefit.sql

# 11. 认证与 RBAC
psql -U badge -d badge_db < migrations/20250208_001_auth_rbac.sql

# 12. 密码哈希修复
psql -U badge -d badge_db < migrations/20250209_001_fix_password_hash.sql

# 13. 批量任务失败明细
psql -U badge -d badge_db < migrations/20250210_001_batch_task_failures.sql

# 14. 权益同步关联
psql -U badge -d badge_db < migrations/20250211_001_benefit_sync_link.sql
```

使用 Makefile 快捷命令：`make db-migrate`

> 注意：Makefile 中的 `db-migrate` 可能未包含最新的迁移文件（如 20250208 之后的文件），生产部署前请核对上述完整列表。

#### 4.1.3 回滚步骤

回滚脚本位于 `migrations/rollback/` 目录，命名规范为 `{original_name}_down.sql`。

**回滚必须按迁移的反序执行。** 例如，要回滚到 20250207 之前的状态：

```bash
# 反序执行回滚脚本
psql -U badge -d badge_db < migrations/rollback/20250210_001_batch_task_failures_down.sql
psql -U badge -d badge_db < migrations/rollback/20250209_001_fix_password_hash_down.sql
psql -U badge -d badge_db < migrations/rollback/20250208_001_auth_rbac_down.sql
psql -U badge -d badge_db < migrations/rollback/20250207_001_auto_benefit_down.sql
```

**可用的回滚脚本：**

| 回滚脚本 | 影响范围 |
|---------|---------|
| `20250128_001_init_schema_down.sql` | 删除所有基础表 |
| `20250130_001_badge_dependency_down.sql` | 删除依赖关系表 |
| `20250131_001_cascade_log_down.sql` | 删除级联日志 |
| `20250201_001_user_badge_logs_down.sql` | 删除用户徽章日志 |
| `20250202_001_dynamic_rules_down.sql` | 删除动态规则表 |
| `20250204_001_rule_templates_down.sql` | 删除规则模板 |
| `20250206_001_benefit_grants_down.sql` | 删除权益发放表 |
| `20250207_001_auto_benefit_down.sql` | 删除自动权益 |
| `20250208_001_auth_rbac_down.sql` | 删除 RBAC 全部表（admin_user, role, permission, api_key 等）|
| `20250209_001_fix_password_hash_down.sql` | 撤销密码哈希修复 |
| `20250210_001_batch_task_failures_down.sql` | 删除 batch_task_failures 表和 batch_tasks 新增列 |

> **警告：** `20250208_001_auth_rbac_down.sql` 会删除所有管理员用户、角色、权限和 API Key 数据。执行此回滚前必须确认已备份相关数据。

#### 4.1.4 完整数据库重置

仅限开发/测试环境使用，**严禁在生产环境执行**：

```bash
# 清空整个 public schema 并重新迁移
make db-reset

# 清空 + 迁移 + 初始化测试数据
make db-reset-full
```

### 4.2 备份策略

#### 4.2.1 定时备份

```bash
# 全量备份（建议每日凌晨执行）
pg_dump -U badge -d badge_db -Fc -f badge_db_$(date +%Y%m%d_%H%M%S).dump

# WAL 归档（实现增量备份和时间点恢复）
# 在 postgresql.conf 中配置：
# archive_mode = on
# archive_command = 'cp %p /backup/wal/%f'
```

**备份保留策略建议：**
- 每日全量备份保留 30 天
- WAL 归档保留 7 天
- 每月首日的备份保留 1 年

#### 4.2.2 恢复流程

```bash
# 从 dump 文件恢复
pg_restore -U badge -d badge_db -c badge_db_20250204_030000.dump

# 时间点恢复（PITR）
# 1. 停止 PostgreSQL
# 2. 恢复最近的全量备份
# 3. 配置 recovery.conf 指定恢复目标时间
# 4. 启动 PostgreSQL 执行恢复
```

---

## 5. 故障排查

### 5.1 常见问题

#### 5.1.1 服务无法启动

**现象：** `badge-admin-service` 启动时 panic

**排查步骤：**

1. 检查 `BADGE_ENV` 是否设为 `production`，若是则 `BADGE_JWT_SECRET` 必须设置：
   ```bash
   # 代码中的强制校验逻辑：
   # if BADGE_ENV == "production" && BADGE_JWT_SECRET 未设置 → panic
   ```

2. 检查数据库连接：
   ```bash
   psql -U badge -h localhost -d badge_db -c "SELECT 1"
   ```

3. 检查 Redis 连接：
   ```bash
   redis-cli -u redis://localhost:6379 ping
   ```

4. 检查配置文件是否存在：
   ```bash
   ls /app/config/badge-admin-service.toml
   ls /app/config/default.toml
   ```

#### 5.1.2 数据库连接失败

**现象：** `/ready` 端点返回 `{"status": "degraded", "checks": {"database": "fail"}}`

**排查步骤：**

1. 确认 PostgreSQL 服务运行正常：`pg_isready -U badge`
2. 检查连接池是否耗尽（生产环境 `max_connections=50`）：
   ```sql
   SELECT count(*) FROM pg_stat_activity WHERE datname = 'badge_db';
   ```
3. 检查 `DATABASE_URL` 环境变量是否正确
4. 检查网络连通性（容器间网络、DNS 解析）

#### 5.1.3 Kafka 消费者积压

**现象：** 事件处理延迟增大，Consumer Lag 持续增长

**排查步骤：**

1. 检查消费者组状态：
   ```bash
   kafka-consumer-groups --bootstrap-server localhost:9092 \
     --describe --group event-engagement-service
   ```

2. 检查是否有消费者实例异常退出（消费者组内的 member 数量）

3. 检查死信队列是否有消息积累：
   ```bash
   kafka-console-consumer --bootstrap-server localhost:9092 \
     --topic badge.dlq --from-beginning --max-messages 10
   ```

4. 如果是规则引擎响应慢导致的积压，检查 `unified-rule-engine` 的日志和指标

#### 5.1.4 Token 刷新失败

**现象：** 前端报 401 错误，用户被迫重新登录

**排查步骤：**

1. 检查 JWT 密钥是否在服务重启后发生变化。如果 `BADGE_JWT_SECRET` 改变，所有已签发的 Token 都会失效。这是安全设计，但需要在密钥轮换时有预期。

2. 检查 Token 是否已被加入黑名单（用户执行了 logout）：
   ```bash
   redis-cli keys "token_blacklist:*"
   ```

3. 检查 Token 是否已过期（默认 `BADGE_JWT_EXPIRES_SECS=86400` 即 24 小时）

4. 检查服务器时间同步（NTP）。JWT 验证依赖时间戳，如果服务器时钟偏差过大会导致有效 Token 被误判过期。

#### 5.1.5 API Key 认证失败

**现象：** 外部系统调用 `/api/v1/` 接口返回 401

**排查步骤：**

1. 确认请求携带了 `X-API-Key` Header
2. 检查 API Key 是否已被禁用或过期：
   ```sql
   SELECT id, name, enabled, expires_at, last_used_at
   FROM api_key WHERE key_hash = encode(sha256('{your-key}'::bytea), 'hex');
   ```
3. 检查是否触发限流（HTTP 429 响应）。限流基于 Redis 滑动窗口，每分钟计数：
   ```bash
   redis-cli keys "rate_limit:*"
   ```

### 5.2 紧急处理流程

#### 5.2.1 服务降级策略

| 故障场景 | 降级行为 | 用户影响 |
|---------|---------|---------|
| `unified-rule-engine` 不可用 | `badge-admin-service` 规则测试功能不可用，其余功能正常 | 无法测试规则，但可创建和管理规则 |
| `badge-management-service` 不可用 | 跨服务缓存刷新降级为本地缓存 | 依赖图更新可能有延迟 |
| Redis 不可用 | `/ready` 返回 degraded，Token 黑名单检查失效，API Key 限流降级放行 | 已登出的 Token 可能短暂有效；限流失效 |
| Kafka 不可用 | 事件消费服务暂停，事件会在 Kafka 恢复后继续消费 | 自动徽章发放延迟，通知推送延迟 |
| PostgreSQL 不可用 | 所有服务不可用 | 完全不可用，需最高优先级恢复 |

#### 5.2.2 紧急回滚步骤

**服务回滚（无数据库变更时）：**

```bash
# 1. 确定要回滚到的版本（git commit SHA）
git log --oneline -10

# 2. 拉取历史版本镜像
docker pull ghcr.io/{owner}/{repo}/{service}:{previous-sha}

# 3. 滚动更新到历史版本
# Kubernetes:
kubectl set image deployment/{service} {service}=ghcr.io/{owner}/{repo}/{service}:{previous-sha}
# Docker Compose:
docker-compose up -d {service}
```

**服务 + 数据库回滚：**

```bash
# 1. 先停止受影响的服务
# 2. 执行数据库回滚脚本（反序）
psql -U badge -d badge_db < migrations/rollback/{migration_name}_down.sql
# 3. 回滚服务镜像
# 4. 重启服务并验证健康检查
```

#### 5.2.3 联系人和升级路径

| 级别 | 响应时间 | 负责人 | 场景 |
|------|---------|--------|------|
| P0 | 15 分钟 | 平台值班工程师 | 全站不可用、数据丢失 |
| P1 | 30 分钟 | 服务负责人 | 核心功能不可用（徽章发放、认证） |
| P2 | 2 小时 | 开发团队 | 非核心功能异常（统计报表、通知延迟） |
| P3 | 下一工作日 | 开发团队 | 页面展示异常、非关键日志错误 |

---

## 6. 安全配置

### 6.1 HTTP 安全头

`badge-admin-service` 内置了 `security_headers` 中间件，为所有响应自动注入以下安全头：

| Header | 值 | 作用 |
|--------|---|------|
| `X-Content-Type-Options` | `nosniff` | 禁止浏览器猜测 Content-Type，防止将非可执行内容误判为脚本 |
| `X-Frame-Options` | `DENY` | 禁止页面被嵌入 iframe，防止点击劫持攻击 |
| `Strict-Transport-Security` | `max-age=31536000; includeSubDomains` | 强制后续访问使用 HTTPS，有效期一年 |
| `X-XSS-Protection` | `0` | 显式禁用旧版 XSS 过滤器（现代浏览器已内置防护，旧的 filter 反而可能引入侧信道漏洞） |

> 这些安全头作为纵深防御的一环，即使上游反向代理（Nginx/Envoy）未正确配置，应用层仍能提供基本的浏览器安全策略。

### 6.2 API Key 管理

外部系统通过 `/api/v1/` 前缀的接口访问，使用 API Key 认证（而非 JWT），通过 `X-API-Key` Header 传递。

**API Key 特性：**
- 存储时使用 SHA256 哈希，数据库中不保存明文
- 支持设置过期时间（`expires_at` 字段）
- 支持启用/禁用状态控制
- 支持细粒度权限：`read:badges`、`read:users`、`write:redemption`、`read:redemption`、`read:grants`
- 通配符 `*` 表示拥有全部权限
- 每次使用自动更新 `last_used_at` 时间戳（异步写入，不阻塞请求）

**API Key 限流：**
- 基于 Redis 的固定窗口计数限流（每分钟）
- 通过 `rate_limit` 字段配置每分钟最大请求数
- `rate_limit` 为 NULL 或 0 表示不限流（适用于高信任内部 Key）
- Redis 故障时降级放行，避免缓存不可用导致外部 API 全部 503

**管理操作（需要 `system:apikey:write` 权限）：**

```bash
# 创建 API Key
POST /api/admin/system/api-keys

# 列出所有 API Key
GET /api/admin/system/api-keys

# 重新生成 Key
POST /api/admin/system/api-keys/{id}/regenerate

# 启用/禁用
PATCH /api/admin/system/api-keys/{id}/status

# 删除
DELETE /api/admin/system/api-keys/{id}
```

### 6.3 JWT Token 配置

| 配置项 | 环境变量 | 默认值 | 说明 |
|-------|---------|--------|------|
| 签名密钥 | `BADGE_JWT_SECRET` | 开发环境有默认值 | **生产环境必须设置**，否则服务 panic |
| 过期时间 | `BADGE_JWT_EXPIRES_SECS` | 86400 (24h) | 根据安全要求调整 |
| 签发者 | 硬编码 | `badge-admin-service` | 用于 Token 验证 |

**Token 黑名单机制：** 用户 logout 时 Token 被加入 Redis 黑名单。认证中间件在验证签名后会额外检查黑名单，确保已注销的 Token 无法继续使用。

**公开路由（免认证）：**
- `POST /api/admin/auth/login` — 登录
- `POST /api/admin/auth/logout` — 登出
- `GET /api/admin/auth/me` — 获取当前用户信息
- `POST /api/admin/auth/refresh` — 刷新 Token
- `GET /health` — 健康检查
- `/api/v1/*` — 外部 API（使用 API Key 认证，跳过 JWT）

### 6.4 CORS 策略

通过 `BADGE_CORS_ORIGINS` 环境变量配置：

```bash
# 生产环境：指定具体域名
BADGE_CORS_ORIGINS=https://admin.example.com,https://admin-staging.example.com

# 开发环境默认值
BADGE_CORS_ORIGINS=http://localhost:3001,http://localhost:5173
```

> **安全警告：** 设置为 `*` 时，如果 `BADGE_ENV=production`，服务会记录警告日志。通配符 CORS 在生产环境中是严重安全隐患，可能导致跨站请求伪造。

### 6.5 RBAC 权限体系

系统采用基于角色的访问控制（RBAC），权限格式为 `{模块}:{资源}:{操作}`：

| 权限 | 说明 |
|------|------|
| `system:user:read/write` | 系统用户管理 |
| `system:role:read/write` | 角色管理 |
| `system:apikey:read/write` | API Key 管理 |
| `badge:category:read/write` | 分类管理 |
| `badge:series:read/write` | 系列管理 |
| `badge:badge:read/write/publish` | 徽章管理（publish 控制发布/下线/归档） |
| `badge:dependency:read/write` | 依赖关系管理 |
| `rule:rule:read/write/publish/test` | 规则管理 |
| `rule:template:read` | 规则模板查看 |
| `grant:grant:read/write` | 发放管理 |
| `grant:revoke:read/write` | 撤回管理 |
| `grant:task:read/write` | 批量任务管理 |
| `benefit:benefit:read/write` | 权益管理 |
| `benefit:grant:read` | 权益发放记录 |
| `benefit:redemption:read/write` | 兑换管理 |
| `stats:overview:read` | 统计报表 |
| `user:view:read` | 用户视图 |
| `user:badge:read` | 用户徽章查看 |
| `log:operation:read` | 操作日志 |

### 6.6 网络隔离建议

```
                    ┌─────────── 公网 ──────────┐
                    │                            │
                    │     [CDN / WAF / LB]       │
                    │          ↓                  │
                    └──────────┬─────────────────┘
                               │ HTTPS :443
                    ┌──────────┴─────────────────┐
                    │       DMZ 区域              │
                    │   admin-ui (Nginx :80)      │
                    │   badge-admin-svc (:8080)   │
                    └──────────┬─────────────────┘
                               │ 内部网络
                    ┌──────────┴─────────────────┐
                    │      服务内网区域            │
                    │  rule-engine (:50051)       │
                    │  badge-mgmt (:50052)        │
                    │  event-engage (:50053)      │
                    │  event-txn (:50054)         │
                    │  notif-worker (:50055)      │
                    └──────────┬─────────────────┘
                               │ 数据层网络
                    ┌──────────┴─────────────────┐
                    │      数据层（仅内部可达）    │
                    │  PostgreSQL (:5432)         │
                    │  Redis (:6379)              │
                    │  Kafka (:9092)              │
                    │  Elasticsearch (:9200)      │
                    └────────────────────────────┘
```

- 只有 `admin-ui` 和 `badge-admin-service` 需要对外暴露
- gRPC 服务（50051-50055）仅允许内部服务间通信
- 数据库和中间件端口严禁对外暴露
- 建议在负载均衡器前部署 WAF，防御常见 Web 攻击

---

## 7. 优雅关闭

所有后端服务实现了优雅关闭机制：

- 监听 `SIGTERM`（Kubernetes 停止 Pod 时发送）和 `Ctrl+C` 信号
- 收到信号后停止接收新连接，等待已有请求处理完毕
- Kubernetes 部署时建议设置 `terminationGracePeriodSeconds: 30`，给予足够时间完成在途请求

```yaml
# Kubernetes 部署建议
spec:
  terminationGracePeriodSeconds: 30
  containers:
    - name: badge-admin-service
      lifecycle:
        preStop:
          exec:
            command: ["sleep", "5"]  # 等待负载均衡器摘除流量后再开始关闭
```

> **为什么需要 preStop sleep：** Kubernetes 摘除 endpoints 和发送 SIGTERM 是并行的。如果服务收到 SIGTERM 立即停止接收连接，可能丢失负载均衡器尚未感知到的请求。5 秒的等待确保负载均衡器完成流量切换。
