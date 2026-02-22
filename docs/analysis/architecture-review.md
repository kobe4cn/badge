# 架构与生产就绪度分析报告

> 基于 specs/instructions.md 中 **II. 技术需求** 和 **III. 其他需求** 逐维度评审。

---

## 1. 微服务架构（评分：A）

### 已实现

| 需求服务 | Crate | 状态 |
|---------|-------|------|
| event-engagement-service | `crates/event-engagement-service` | ✅ 存在，有 Dockerfile |
| event-transaction-service | `crates/event-transaction-service` | ✅ 存在，有 Dockerfile |
| badge-management-service | `crates/badge-management-service` | ✅ 存在，有 Dockerfile |
| badge-admin-service | `crates/badge-admin-service` | ✅ 存在，有 Dockerfile |
| badge-admin-UI | `web/admin-ui` | ✅ React/TypeScript，有 Dockerfile |
| unified-rule-engine | `crates/unified-rule-engine` | ✅ 存在，有 Dockerfile |

- 额外交付：`notification-worker`（异步通知处理）、`mock-services`（测试辅助）、`proto`（gRPC 接口定义）、`shared`（共享库）
- Workspace 组织清晰，依赖通过 `workspace.dependencies` 统一管理
- 每个服务有独立的 `main.rs` 入口和 `Dockerfile`
- 服务职责边界明确：admin-service 处理 B 端 REST API，management-service 处理 C 端 gRPC 业务

### 缺失

- 无。6 个需求服务全部存在且可构建。

---

## 2. 服务间通信（评分：A-）

### 已实现

**gRPC 通信**：
- `badge.proto`：定义 `BadgeManagementService`，包含 9 个 RPC 方法（GetUserBadges、GrantBadge、RevokeBadge、RedeemBadge、PinBadge、RefreshDependencyCache 等）
- `rule_engine.proto`：定义 `RuleEngineService`，包含 5 个 RPC 方法（Evaluate、BatchEvaluate、LoadRule、TestRule 等）
- admin-service 通过 gRPC 客户端连接 management-service 和 rule-engine

**Kafka 异步消息**：
- 5 个 Topic：`engagement_events`、`transaction_events`、`notifications`、`dead_letter_queue`、`rule_reload`
- 生产者/消费者封装完善（`KafkaProducer`/`KafkaConsumer`），支持 JSON 序列化、优雅关闭
- 消费者可配置 `max_poll_interval_ms`/`session_timeout_ms`/`heartbeat_interval_ms`，适配慢处理链路

**REST API**：
- admin-service 暴露完整 REST 路由（18 个路由模块），覆盖徽章、规则、发放、兑换、权益、通知等全部 B 端操作
- 外部 API（`/api/v1/`）使用独立的 API Key 认证 + 细粒度权限控制

### 缺失

- gRPC 接口未发布正式的 API 文档（OpenAPI/gRPC reflection 未启用）
- 未集成服务注册/发现机制（服务地址通过环境变量硬配置）
- gRPC 连接无重试/重连策略（连接失败仅记录 warn 日志）

---

## 3. 服务治理与性能机制（评分：C+）

### 已实现

**限流**：
- 外部 API 基于 Redis 的固定窗口限流（每分钟计数，每个 API Key 独立限制）
- Redis 故障时降级放行，避免缓存不可用导致全部 503

**连接池**：
- PostgreSQL 连接池：`max_connections=10`，`min_connections=2`，可配置超时
- Redis 连接池：`pool_size=10`
- Kafka 消费者超时调优：`max_poll_interval_ms=600000`（10 分钟）

**优雅关闭**：
- admin-service 监听 SIGTERM/Ctrl+C，通过 `with_graceful_shutdown` 停止接收新连接
- Kafka 消费者通过 `watch::Receiver<bool>` 实现关闭信号传播

### 缺失（阻塞项）

- **无熔断器（Circuit Breaker）**：代码中未找到任何熔断/降级实现，搜索 `circuit_break`/`bulkhead`/`fallback`/`degrade` 仅在 rate_limit 相关文件有少量匹配。需求明确要求"熔断、限流、服务降级"
- **无服务降级策略**：gRPC 连接失败后无降级处理，仅记录日志
- **连接池规格偏小**：`max_connections=10` 对于 1000+ TPS 的需求远远不足，生产环境通常需要 50-200
- **无背压/队列管理**：Kafka 消费者无并发度控制，单线程串行处理
- **未集成服务治理框架**：需求要求接入"采购方指定的服务治理框架（如基于阿里云MSE）"，目前未实现

---

## 4. 安全（评分：B-）

### 已实现

**认证与授权**：
- JWT 认证：HMAC-SHA256 签名，含 issuer 校验、过期校验、Token 黑名单
- RBAC 权限：每个 REST 路由附加 `require_permission("module:resource:action")` 中间件
- API Key 认证：SHA256 哈希存储，支持启用/禁用、过期时间、细粒度权限
- 强制密码修改：种子用户首次登录必须修改默认密码

**传输安全**：
- HTTP 安全头：`X-Content-Type-Options: nosniff`、`X-Frame-Options: DENY`、`HSTS`
- CORS 限制：生产环境禁止通配符 `*`，必须显式列出允许来源

**密钥管理**：
- JWT 密钥：生产环境必须通过 `BADGE_JWT_SECRET` 环境变量注入，否则 panic
- 开发环境使用默认密钥，带 warn 日志

### 缺失（阻塞项）

- **未集成 MYID SSO / MFA**：需求明确要求"接入 MYID，实现 SSO，并支持 MFA"，目前使用自建用户名/密码认证
- **无字段级加密**：需求要求"支持字段级别的加密能力"，当前无实现
- **数据库凭证硬编码**：多个 examples 和 test 文件中有 `postgres://badge:badge_secret@...` 硬编码
- **TLS 未在应用层终止**：需求要求"全链路 HTTPS，TLS ≥ 1.2"，服务间通信（gRPC、Kafka）均为明文
- **无密钥轮换机制**：JWT 密钥为静态配置，无自动轮换支持
- **API Key 明文传输后才 hash**：原始 API Key 在 HTTP Header 中以明文传输（依赖 HTTPS 在代理层终止）

---

## 5. 可观测性（评分：A-）

### 已实现

**指标（Metrics）**：
- Prometheus 指标导出：独立 HTTP 端口 `/metrics`
- 丰富的业务指标：`badge_grants_total`、`redemptions_total`、`cascade_evaluations_total`、`rule_evaluations_total`、`benefit_grants_total`、`batch_tasks_total`、`badge_revokes_total`、`badge_expirations_total` 等
- 系统指标：`http_requests_total`、`http_request_duration_seconds`、`grpc_requests_total`
- Worker 健康指标：`worker_last_run_timestamp`
- Prometheus 配置覆盖全部 6 个服务的抓取

**追踪（Tracing）**：
- OpenTelemetry OTLP 支持，可导出到 Jaeger/Tempo
- W3C Trace Context 传播（inject/extract）
- HTTP 请求自动创建追踪 span，含路径规范化（防指标基数爆炸）
- Request ID 中间件（UUID v4）

**日志（Logging）**：
- tracing-subscriber 统一日志，支持 JSON 结构化 / pretty 两种格式
- 环境变量控制日志级别（`RUST_LOG` + `BADGE_*`）

**告警（Alerting）**：
- Prometheus 告警规则 3 组 10 条：业务告警（发放错误率、库存不足、兑换延迟/错误率、级联超时）、HTTP 服务告警（5xx 错误率、延迟、零流量）、规则引擎告警
- Alertmanager 配置（Slack webhook 通知）
- 每条告警含 severity 标签、runbook_url

**可视化**：
- Grafana 部署配置，含 provisioning 和 dashboards 目录
- Jaeger UI 用于追踪查看

### 缺失

- **gRPC 追踪拦截器为占位实现**：`grpc::tracing_interceptor` 仅记录 debug 日志，未真正传播 trace context
- **Kafka 消息追踪为占位实现**：`kafka::extract_trace_context` 始终返回 `None`
- **无 ELK/SLS 集成**：需求要求接入"采购方统一的监控告警与日志分析平台（如基于阿里云 SLS, ARMS 等）"

---

## 6. 部署与运维（评分：B）

### 已实现

**容器化**：
- 全部 7 个服务（6 个业务 + notification-worker）有多阶段 Dockerfile
- 运行阶段使用 `debian:bookworm-slim`，镜像精简
- Dockerfile 内置 `HEALTHCHECK` 指令

**CI/CD**：
- GitHub Actions 流水线：`deploy.yml`（构建+推送+部署）和 `e2e-tests.yml`（全链路测试）
- 后端使用 matrix 并行构建 6 个服务
- 部署使用 SSH + docker-compose，含健康检查和自动回滚
- Commit SHA 作为镜像 tag，避免 `:latest` 漂移
- E2E 测试含后端、前端 Playwright、性能测试三个维度

**基础设施**：
- `docker-compose.infra.yml`：PostgreSQL 16、Redis 7、Kafka 7.5、Elasticsearch 8.12
- `docker-compose.observability.yml`：Prometheus、Grafana、Jaeger、Alertmanager
- `docker-compose.test.yml`：测试专用环境

**数据库管理**：
- 25 个有序迁移文件（20250128 ~ 20250222）
- 25 个对应的回滚脚本（`migrations/rollback/`）
- Makefile 提供 `db-migrate`、`db-reset`、`db-setup` 命令
- 数据库备份脚本（`scripts/backup-db.sh`），含保留天数和自动清理

**配置管理**：
- 多层配置加载：default.toml → {env}.toml → {service}.toml → 环境变量
- `BADGE_*` 前缀环境变量覆盖

### 缺失（阻塞项）

- **无 Kubernetes 清单**：需求要求"在采购方指定的阿里云 VPC 私有环境中完成全栈部署"，当前仅有 docker-compose，无 K8s Deployment/Service/Ingress/HPA
- **无 IaC（基础设施即代码）**：需求要求使用"Terraform, Ansible"进行环境配置，当前完全缺失
- **无多环境配置**：仅有 `config/default.toml`，无 `production.toml`/`staging.toml`
- **无配置中心集成**：需求要求接入"Nacos 或 ETCD 或其他指定配置中心"，当前使用文件+环境变量
- **无正式的服务部署拓扑图**
- **镜像仓库使用 GHCR**：需求要求"使用采购方指定的私有镜像仓库"

---

## 7. 健康检查与高可用（评分：B）

### 已实现

- admin-service 提供 `/health`（存活探针）和 `/ready`（就绪探针，检查 DB + Redis）
- Dockerfile 内置 HEALTHCHECK
- CI/CD 部署后执行健康检查，失败自动回滚
- Kafka 消费者支持优雅关闭

### 缺失

- 其他 gRPC 服务未看到标准的 gRPC Health Checking Protocol 实现
- 无水平扩展（HPA）配置
- 无多副本/多 AZ 部署方案
- 无数据库主从/读写分离配置
- 无 Redis Sentinel/Cluster 配置

---

## 8. 审计与操作日志（评分：A）

### 已实现

- 审计中间件自动记录所有写操作（POST/PUT/PATCH/DELETE）到 `operation_logs` 表
- 记录字段完整：操作人、模块、操作类型、目标资源、IP 地址、User-Agent、变更前/后数据
- 变更前数据通过 `AuditContext.snapshot()` 用 PostgreSQL `to_jsonb` 自动快照
- 异步写入避免阻塞业务（fire-and-forget）
- 操作日志查询 API（`/api/admin/logs`）

### 缺失

- 审计日志无防篡改机制（如链式哈希）
- 无日志导出功能

---

## 9. 数据可靠性与备份（评分：C+）

### 已实现

- 数据库备份脚本（`pg_dump` + gzip 压缩 + 自动清理旧备份）
- 迁移回滚脚本覆盖全部 25 个迁移
- Kafka DLQ（死信队列）处理失败消息
- 幂等处理（`idempotency_ttl_hours=24`）

### 缺失（阻塞项）

- **无自动化备份调度**：备份脚本需手动执行，无 cron/K8s CronJob
- **无跨区域备份**：需求要求 RPO ≤ 30 分钟、RTO ≤ 8 小时
- **无数据恢复测试/演练文档**
- **PostgreSQL 未配置 WAL 归档/流复制**

---

## 10. 外部系统集成（评分：D）

### 已实现

- 通知系统架构设计（notification-worker + 多渠道支持）
- Kafka 事件消费（engagement/transaction 两类事件）
- 外部 API（`/api/v1/`）为第三方系统提供访问入口

### 缺失（阻塞项）

- **未对接任何实际外部系统**：订单服务、Profile 服务、Coupon 服务、IRP 系统、事件追踪系统、风控系统等均未实际集成
- **无 Mock/Stub 适配层**：mock-services 仅用于测试数据生成，非生产适配层
- **通知渠道为占位实现**：APP Notification、短信、微信订阅消息、邮件等渠道未实际对接

> **说明**：外部系统对接通常需要采购方提供接口规范和测试环境，当前属于预期中的待完成项。

---

## 生产就绪度总评

### 综合评分：B-（有条件可进入受控环境测试，不满足直接生产部署条件）

| 维度 | 评分 | 生产阻塞 |
|------|------|---------|
| 微服务架构 | A | 否 |
| 服务间通信 | A- | 否 |
| 服务治理与性能 | C+ | **是** |
| 安全 | B- | **是** |
| 可观测性 | A- | 否 |
| 部署与运维 | B | **是** |
| 健康检查与高可用 | B | 部分 |
| 审计与操作日志 | A | 否 |
| 数据可靠性与备份 | C+ | **是** |
| 外部系统集成 | D | **是**（待采购方配合） |

### P0 阻塞项清单（必须在上线前解决）

1. **熔断器与服务降级**：实现 Circuit Breaker 模式（推荐使用 tower 的 `ServiceBuilder` + 自定义 Layer，或引入 `failsafe-rs`），为 gRPC 调用和外部 API 调用添加熔断/超时/降级策略
2. **连接池扩容**：PostgreSQL `max_connections` 从 10 提升到至少 50（根据实例规格调整），Redis `pool_size` 同步扩容
3. **Kubernetes 部署清单**：编写 Deployment、Service、Ingress、HPA、PDB 等 K8s 资源清单
4. **MYID SSO/MFA 集成**：替换或补充当前自建认证为 MYID SSO，添加 MFA 支持
5. **TLS 全链路**：gRPC 启用 TLS，Kafka 配置 SASL_SSL，确保服务间通信加密
6. **字段级数据加密**：实现敏感字段（用户信息等）的加密/脱敏存储
7. **配置中心集成**：接入 Nacos/ETCD 实现配置外部化、版本化和动态刷新
8. **自动化数据库备份**：配置 K8s CronJob 或 RDS 自动备份，满足 RPO ≤ 30 分钟

### P1 改进项（建议在首次迭代后完成）

1. 补全 gRPC 追踪拦截器和 Kafka 消息追踪传播的完整实现
2. 编写 production.toml 环境配置，区分开发/测试/生产参数
3. 实现 IaC（Terraform 管理阿里云资源）
4. 添加 gRPC Health Checking Protocol 到所有 gRPC 服务
5. 清理测试/示例文件中的硬编码凭证
6. 编写正式的 API 文档（OpenAPI 规范 + gRPC reflection）
7. 实现 API 版本管理策略
8. 添加数据库读写分离和 Redis Sentinel/Cluster 支持
