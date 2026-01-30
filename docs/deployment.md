# 部署指南

本文档描述徽章系统的部署流程与配置说明。

## 环境要求

### 硬件要求

| 环境 | CPU | 内存 | 磁盘 |
|------|-----|------|------|
| 开发 | 2 核 | 8 GB | 20 GB |
| 测试 | 4 核 | 16 GB | 50 GB |
| 生产 | 8 核+ | 32 GB+ | 100 GB+ |

### 软件要求

| 组件 | 版本 | 说明 |
|------|------|------|
| Docker | 24.0+ | 容器运行时 |
| Docker Compose | 2.20+ | 容器编排 |
| Rust | 1.85+ | 仅编译时需要 |

### 基础设施依赖

| 服务 | 版本 | 端口 |
|------|------|------|
| PostgreSQL | 16 | 5432 |
| Redis | 7 | 6379 |
| Kafka | 7.5 (Confluent) | 9092 |
| Zookeeper | 7.5 (Confluent) | 2181 |
| Elasticsearch | 8.11 | 9200 |

---

## Docker 部署

### 1. 准备环境变量

```bash
# 复制环境变量模板
cp docker/.env.example docker/.env

# 编辑环境变量
vim docker/.env
```

**环境变量说明：**

```bash
# PostgreSQL
POSTGRES_USER=badge
POSTGRES_PASSWORD=badge_secret    # 生产环境请修改
POSTGRES_DB=badge_db

# Redis
REDIS_URL=redis://localhost:6379

# Kafka
KAFKA_BROKERS=localhost:9092

# 服务端口
RULE_ENGINE_PORT=50051
BADGE_MANAGEMENT_PORT=50052
BADGE_ADMIN_PORT=8080
EVENT_ENGAGEMENT_PORT=50053
EVENT_TRANSACTION_PORT=50054
NOTIFICATION_WORKER_PORT=50055
```

### 2. 启动基础设施

```bash
# 启动所有基础设施服务
docker compose -f docker/docker-compose.infra.yml up -d

# 检查服务状态
docker compose -f docker/docker-compose.infra.yml ps

# 查看日志
docker compose -f docker/docker-compose.infra.yml logs -f
```

### 3. 运行数据库迁移

```bash
# 等待 PostgreSQL 就绪后执行迁移
docker exec -i badge-postgres psql -U badge -d badge_db < migrations/20250128_001_init_schema.sql
```

### 4. 构建与运行应用

```bash
# 构建 Release 版本
cargo build --workspace --release

# 启动所有服务（推荐使用 Makefile）
make dev-backend

# 或者手动启动各服务：

# 核心服务（必需）
./target/release/rule-engine &          # gRPC 规则引擎 :50051
./target/release/badge-management &     # gRPC 徽章管理 :50052
./target/release/badge-admin &          # HTTP 管理后台 :8080

# 事件处理服务（Kafka 消费者）
./target/release/event-engagement &     # 行为事件处理 :50053
./target/release/event-transaction &    # 交易事件处理 :50054
./target/release/notification-worker &  # 通知推送服务 :50055
```

### 5. 开发环境使用 Mock 服务

```bash
# 启动 Mock HTTP 服务器（生成测试数据）
make mock-server

# 生成模拟事件到 Kafka
make mock-generate TYPE=purchase USER=test_user COUNT=10

# 运行预定义场景
make mock-scenario NAME=first_purchase USER=test_user
```

---

## 配置说明

### 应用配置文件

配置文件位于 `config/` 目录：

```toml
# config/default.toml

[server]
host = "0.0.0.0"
port = 8080

[database]
url = "postgres://badge:badge_secret@localhost:5432/badge_db"
max_connections = 10
min_connections = 2
connect_timeout_seconds = 30
idle_timeout_seconds = 600

[redis]
url = "redis://localhost:6379"
pool_size = 10

[kafka]
brokers = "localhost:9092"
consumer_group = "badge-service"
auto_offset_reset = "earliest"

[observability]
log_level = "info"
log_format = "pretty"           # pretty | json
metrics_enabled = true
metrics_port = 9090
tracing_enabled = false
```

### 环境变量覆盖

所有配置项都可以通过环境变量覆盖，使用 `__` 分隔层级：

```bash
# 覆盖数据库连接
export DATABASE__URL="postgres://user:pass@host:5432/db"

# 覆盖日志级别
export OBSERVABILITY__LOG_LEVEL="debug"

# 覆盖服务端口
export SERVER__PORT=8081
```

### 生产环境配置建议

```toml
[server]
host = "0.0.0.0"
port = 8080

[database]
url = "${DATABASE_URL}"          # 使用环境变量
max_connections = 50             # 增加连接池
min_connections = 10
connect_timeout_seconds = 10
idle_timeout_seconds = 300

[redis]
url = "${REDIS_URL}"
pool_size = 50

[kafka]
brokers = "${KAFKA_BROKERS}"
consumer_group = "badge-service-prod"
auto_offset_reset = "latest"     # 生产环境使用 latest

[observability]
log_level = "info"
log_format = "json"              # 生产环境使用 JSON 格式
metrics_enabled = true
metrics_port = 9090
tracing_enabled = true           # 启用链路追踪
```

---

## 健康检查

### 基础设施健康检查

```bash
# PostgreSQL
docker exec badge-postgres pg_isready -U badge
# 输出: localhost:5432 - accepting connections

# Redis
docker exec badge-redis redis-cli ping
# 输出: PONG

# Kafka
docker exec badge-kafka kafka-topics --bootstrap-server localhost:9092 --list
# 输出: (topic list)

# Elasticsearch
curl -s http://localhost:9200/_cluster/health | jq .status
# 输出: "green" 或 "yellow"
```

### 应用健康检查

各服务提供 HTTP 健康检查端点：

| 服务 | 端点 | 说明 |
|------|------|------|
| Badge Admin | `GET /health` | REST API 健康检查 |
| Rule Engine | `GET /health` | gRPC 服务健康检查 |
| Badge Management | `GET /health` | gRPC 服务健康检查 |

**示例：**

```bash
# 检查 Admin API
curl -s http://localhost:8080/health
# 输出: {"status":"healthy","version":"0.1.0"}

# 检查 gRPC 服务（使用 grpcurl）
grpcurl -plaintext localhost:50051 grpc.health.v1.Health/Check
# 输出: {"status":"SERVING"}
```

### Kubernetes 探针配置

```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 10

readinessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 5
```

---

## 监控指标

### Prometheus 指标端点

各服务在 `:9090/metrics` 暴露 Prometheus 指标：

```bash
curl http://localhost:9090/metrics
```

### 核心指标

| 指标 | 类型 | 说明 |
|------|------|------|
| `http_requests_total` | Counter | HTTP 请求总数 |
| `http_request_duration_seconds` | Histogram | 请求延迟分布 |
| `grpc_server_handled_total` | Counter | gRPC 调用总数 |
| `rule_evaluations_total` | Counter | 规则评估总数 |
| `badge_grants_total` | Counter | 徽章发放总数 |
| `db_pool_connections` | Gauge | 数据库连接池状态 |

---

## 日志管理

### 日志格式

**开发环境（Pretty）：**
```
2025-01-29T10:30:00.000Z  INFO badge_admin: Server started on 0.0.0.0:8080
```

**生产环境（JSON）：**
```json
{"timestamp":"2025-01-29T10:30:00.000Z","level":"INFO","target":"badge_admin","message":"Server started on 0.0.0.0:8080"}
```

### 日志级别

| 级别 | 说明 |
|------|------|
| `error` | 错误，需要关注 |
| `warn` | 警告，可能有问题 |
| `info` | 常规信息 |
| `debug` | 调试信息 |
| `trace` | 详细追踪 |

通过环境变量调整：

```bash
# 全局日志级别
export RUST_LOG=info

# 按模块设置
export RUST_LOG=badge_admin=debug,rule_engine=info
```

---

## 故障排查

### 常见问题

**1. 数据库连接失败**

```bash
# 检查 PostgreSQL 状态
docker logs badge-postgres

# 检查连接参数
psql -h localhost -U badge -d badge_db
```

**2. Kafka 消费者无法连接**

```bash
# 检查 Kafka 状态
docker logs badge-kafka

# 验证 Topic 存在
docker exec badge-kafka kafka-topics --bootstrap-server localhost:9092 --list
```

**3. 服务启动失败**

```bash
# 查看详细日志
RUST_LOG=debug ./target/release/badge-admin

# 检查配置文件
cat config/default.toml
```

### 日志收集

推荐使用 ELK Stack 或 Loki 进行日志收集：

```bash
# 查看 Elasticsearch 中的日志
curl "http://localhost:9200/badge-logs-*/_search?q=level:error&size=10"
```
