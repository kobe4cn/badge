# E-Badge System 实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 构建完整的会员徽章管理系统，包含 6 个 Rust 后端服务、1 个 React 管理前端、以及完整的模拟外部系统。

**Architecture:** 采用事件驱动 + 微服务架构。后端使用 Rust 2024 + Axum/Tonic + SQLx + Kafka，前端使用 React 18 + Ant Design Pro + React Flow。所有服务通过 gRPC 通信，异步事件通过 Kafka 传递。

**Tech Stack:**
- Backend: Rust 2024, Axum 0.8, Tonic 0.12, SQLx 0.8, rdkafka, Redis
- Frontend: React 18, TypeScript, Ant Design 5, ProComponents, React Flow, Zustand
- Infrastructure: PostgreSQL, Redis, Kafka, Elasticsearch

---

## 阶段概览

| 阶段 | 内容 | 预计任务数 |
|------|------|-----------|
| Phase 1 | 项目骨架与基础设施 | 8 |
| Phase 2 | Proto 定义与共享库 | 6 |
| Phase 3 | 规则引擎服务 | 10 |
| Phase 4 | 徽章管理服务（C端） | 12 |
| Phase 5 | 徽章管理服务（B端） | 10 |
| Phase 6 | 事件处理服务 | 8 |
| Phase 7 | 模拟外部系统 | 8 |
| Phase 8 | 管理后台前端 | 15 |
| Phase 9 | 集成测试与优化 | 5 |

---

## Phase 1: 项目骨架与基础设施

### Task 1.1: 创建 Cargo Workspace 根配置

**Files:**
- Create: `Cargo.toml`
- Create: `rust-toolchain.toml`
- Create: `.cargo/config.toml`

**Step 1: 创建 Cargo.toml**

```toml
[workspace]
resolver = "2"
members = [
    "crates/proto",
    "crates/shared",
    "crates/unified-rule-engine",
    "crates/badge-management-service",
    "crates/badge-admin-service",
    "crates/event-engagement-service",
    "crates/event-transaction-service",
    "crates/notification-worker",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.83"
authors = ["Badge Team"]
license = "MIT"

[workspace.dependencies]
# Async runtime
tokio = { version = "1.43", features = ["full"] }

# gRPC
tonic = "0.12"
tonic-build = "0.12"
prost = "0.13"
prost-types = "0.13"

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "json", "chrono", "uuid", "migrate"] }

# Redis
redis = { version = "0.27", features = ["tokio-comp", "cluster-async"] }

# Kafka
rdkafka = { version = "0.37", features = ["cmake-build"] }

# Web framework (for admin service)
axum = "0.8"
axum-extra = { version = "0.10", features = ["typed-header"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace", "compression-gzip", "timeout"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
thiserror = "2.0"
anyhow = "1.0"

# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
opentelemetry = "0.27"
opentelemetry-otlp = "0.27"
metrics = "0.24"
metrics-exporter-prometheus = "0.16"

# Utilities
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.11", features = ["v4", "v7", "serde"] }
config = "0.14"
async-trait = "0.1"
futures = "0.3"
dashmap = "6.0"
parking_lot = "0.12"

# Validation
validator = { version = "0.18", features = ["derive"] }

# Testing
mockall = "0.13"
tokio-test = "0.4"
fake = { version = "3.0", features = ["derive", "chrono", "uuid"] }
```

**Step 2: 创建 rust-toolchain.toml**

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

**Step 3: 创建 .cargo/config.toml**

```toml
[build]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[target.aarch64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[alias]
t = "test"
c = "clippy"
b = "build"
r = "run"
```

**Step 4: 验证配置**

Run: `cargo --version && rustc --version`
Expected: 显示 cargo 和 rustc 版本

**Step 5: 提交**

```bash
git add Cargo.toml rust-toolchain.toml .cargo/
git commit -m "chore: 初始化 Cargo workspace 配置"
```

---

### Task 1.2: 创建 crates 目录结构

**Files:**
- Create: `crates/proto/Cargo.toml`
- Create: `crates/proto/src/lib.rs`
- Create: `crates/shared/Cargo.toml`
- Create: `crates/shared/src/lib.rs`

**Step 1: 创建 proto crate**

`crates/proto/Cargo.toml`:
```toml
[package]
name = "badge-proto"
version.workspace = true
edition.workspace = true

[dependencies]
prost = { workspace = true }
prost-types = { workspace = true }
tonic = { workspace = true }
serde = { workspace = true }
chrono = { workspace = true }

[build-dependencies]
tonic-build = { workspace = true }
```

`crates/proto/src/lib.rs`:
```rust
//! gRPC/Protobuf 定义
//!
//! 此 crate 包含所有服务间通信的 protobuf 定义和生成的 Rust 代码。

pub mod badge {
    // 将在后续任务中添加 proto 生成代码
}
```

**Step 2: 创建 shared crate**

`crates/shared/Cargo.toml`:
```toml
[package]
name = "badge-shared"
version.workspace = true
edition.workspace = true

[dependencies]
tokio = { workspace = true }
sqlx = { workspace = true }
redis = { workspace = true }
rdkafka = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
config = { workspace = true }
async-trait = { workspace = true }
```

`crates/shared/src/lib.rs`:
```rust
//! 共享库
//!
//! 包含所有服务共用的配置、错误处理、数据库连接、缓存、Kafka 等基础设施代码。

pub mod config;
pub mod error;
pub mod database;
pub mod cache;
pub mod kafka;
pub mod telemetry;
```

**Step 3: 创建空模块文件**

```bash
mkdir -p crates/shared/src
touch crates/shared/src/config.rs
touch crates/shared/src/error.rs
touch crates/shared/src/database.rs
touch crates/shared/src/cache.rs
touch crates/shared/src/kafka.rs
touch crates/shared/src/telemetry.rs
```

**Step 4: 验证编译**

Run: `cargo check -p badge-proto -p badge-shared`
Expected: 编译成功（可能有 unused 警告）

**Step 5: 提交**

```bash
git add crates/
git commit -m "chore: 创建 proto 和 shared crate 骨架"
```

---

### Task 1.3: 创建服务 crate 骨架

**Files:**
- Create: `crates/unified-rule-engine/Cargo.toml`
- Create: `crates/unified-rule-engine/src/main.rs`
- Create: `crates/badge-management-service/Cargo.toml`
- Create: `crates/badge-management-service/src/main.rs`
- Create: `crates/badge-admin-service/Cargo.toml`
- Create: `crates/badge-admin-service/src/main.rs`
- Create: `crates/event-engagement-service/Cargo.toml`
- Create: `crates/event-engagement-service/src/main.rs`
- Create: `crates/event-transaction-service/Cargo.toml`
- Create: `crates/event-transaction-service/src/main.rs`
- Create: `crates/notification-worker/Cargo.toml`
- Create: `crates/notification-worker/src/main.rs`

**Step 1: 创建 unified-rule-engine**

`crates/unified-rule-engine/Cargo.toml`:
```toml
[package]
name = "unified-rule-engine"
version.workspace = true
edition.workspace = true

[[bin]]
name = "rule-engine"
path = "src/main.rs"

[dependencies]
badge-proto = { path = "../proto" }
badge-shared = { path = "../shared" }
tokio = { workspace = true }
tonic = { workspace = true }
prost = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
chrono = { workspace = true }
dashmap = { workspace = true }
parking_lot = { workspace = true }

[dev-dependencies]
mockall = { workspace = true }
tokio-test = { workspace = true }
```

`crates/unified-rule-engine/src/main.rs`:
```rust
//! 统一规则引擎服务
//!
//! 提供规则解析、编译、执行能力，支持复杂条件组合和嵌套逻辑。

use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting unified-rule-engine...");

    // TODO: 实现 gRPC 服务器

    Ok(())
}
```

**Step 2: 创建其他服务（模式相同）**

为每个服务创建类似的 Cargo.toml 和 main.rs，调整依赖和描述：

- `badge-management-service`: C端徽章服务，依赖 axum 用于健康检查
- `badge-admin-service`: B端管理服务，主要使用 axum
- `event-engagement-service`: 行为事件服务，依赖 rdkafka
- `event-transaction-service`: 订单事件服务，依赖 rdkafka
- `notification-worker`: 通知服务，依赖 rdkafka

**Step 3: 验证所有服务编译**

Run: `cargo check --workspace`
Expected: 所有 crate 编译成功

**Step 4: 提交**

```bash
git add crates/
git commit -m "chore: 创建所有服务 crate 骨架"
```

---

### Task 1.4: 创建 Docker 基础设施配置

**Files:**
- Create: `docker/docker-compose.infra.yml`
- Create: `docker/.env.example`

**Step 1: 创建 docker-compose.infra.yml**

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:16-alpine
    container_name: badge-postgres
    environment:
      POSTGRES_USER: ${POSTGRES_USER:-badge}
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:-badge_secret}
      POSTGRES_DB: ${POSTGRES_DB:-badge_db}
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ${POSTGRES_USER:-badge}"]
      interval: 5s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    container_name: badge-redis
    command: redis-server --appendonly yes
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 5s
      retries: 5

  zookeeper:
    image: confluentinc/cp-zookeeper:7.5.0
    container_name: badge-zookeeper
    environment:
      ZOOKEEPER_CLIENT_PORT: 2181
      ZOOKEEPER_TICK_TIME: 2000
    ports:
      - "2181:2181"

  kafka:
    image: confluentinc/cp-kafka:7.5.0
    container_name: badge-kafka
    depends_on:
      - zookeeper
    ports:
      - "9092:9092"
      - "29092:29092"
    environment:
      KAFKA_BROKER_ID: 1
      KAFKA_ZOOKEEPER_CONNECT: zookeeper:2181
      KAFKA_ADVERTISED_LISTENERS: PLAINTEXT://kafka:29092,PLAINTEXT_HOST://localhost:9092
      KAFKA_LISTENER_SECURITY_PROTOCOL_MAP: PLAINTEXT:PLAINTEXT,PLAINTEXT_HOST:PLAINTEXT
      KAFKA_INTER_BROKER_LISTENER_NAME: PLAINTEXT
      KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR: 1
      KAFKA_AUTO_CREATE_TOPICS_ENABLE: 'true'
    healthcheck:
      test: ["CMD", "kafka-topics", "--bootstrap-server", "localhost:9092", "--list"]
      interval: 10s
      timeout: 10s
      retries: 5

  elasticsearch:
    image: elasticsearch:8.11.0
    container_name: badge-elasticsearch
    environment:
      - discovery.type=single-node
      - xpack.security.enabled=false
      - "ES_JAVA_OPTS=-Xms512m -Xmx512m"
    ports:
      - "9200:9200"
    volumes:
      - es_data:/usr/share/elasticsearch/data
    healthcheck:
      test: ["CMD-SHELL", "curl -s http://localhost:9200/_cluster/health | grep -q 'green\\|yellow'"]
      interval: 10s
      timeout: 10s
      retries: 5

volumes:
  postgres_data:
  redis_data:
  es_data:

networks:
  default:
    name: badge-network
```

**Step 2: 创建 .env.example**

```env
# PostgreSQL
POSTGRES_USER=badge
POSTGRES_PASSWORD=badge_secret
POSTGRES_DB=badge_db
DATABASE_URL=postgres://badge:badge_secret@localhost:5432/badge_db

# Redis
REDIS_URL=redis://localhost:6379

# Kafka
KAFKA_BROKERS=localhost:9092

# Elasticsearch
ELASTICSEARCH_URL=http://localhost:9200

# Service Ports
RULE_ENGINE_PORT=50051
BADGE_MANAGEMENT_PORT=50052
BADGE_ADMIN_PORT=8080
EVENT_ENGAGEMENT_PORT=50053
EVENT_TRANSACTION_PORT=50054
NOTIFICATION_WORKER_PORT=50055
```

**Step 3: 验证 Docker Compose 配置**

Run: `docker compose -f docker/docker-compose.infra.yml config`
Expected: 配置验证通过，无错误

**Step 4: 提交**

```bash
git add docker/
git commit -m "chore: 添加基础设施 Docker Compose 配置"
```

---

### Task 1.5: 创建数据库迁移基础

**Files:**
- Create: `migrations/20250128_001_init_schema.sql`

**Step 1: 创建初始 schema 迁移**

```sql
-- 徽章系统初始化 schema
-- 包含核心表结构：徽章分类、系列、徽章、规则、用户徽章、账本等

-- 启用必要的扩展
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- ==================== 徽章结构 ====================

-- 一级分类
CREATE TABLE badge_category (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    sort_order INT NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'active', -- active, inactive
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT badge_category_name_unique UNIQUE (name)
);

-- 二级系列
CREATE TABLE badge_series (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    category_id UUID NOT NULL REFERENCES badge_category(id),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    sort_order INT NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT badge_series_name_unique UNIQUE (category_id, name)
);

-- 徽章定义
CREATE TABLE badge (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    series_id UUID NOT NULL REFERENCES badge_series(id),
    code VARCHAR(50) NOT NULL UNIQUE, -- 业务唯一标识
    name VARCHAR(100) NOT NULL,
    description TEXT,
    badge_type VARCHAR(50) NOT NULL, -- transaction, engagement, identity, seasonal

    -- 素材
    icon_url TEXT,
    icon_3d_url TEXT,

    -- 获取配置
    acquire_time_start TIMESTAMPTZ,
    acquire_time_end TIMESTAMPTZ,
    max_acquire_count INT, -- NULL 表示无限

    -- 持有有效期配置
    validity_type VARCHAR(20) NOT NULL DEFAULT 'permanent', -- fixed, flexible, permanent
    validity_fixed_date TIMESTAMPTZ, -- validity_type = fixed 时使用
    validity_days INT, -- validity_type = flexible 时使用

    -- 发放对象
    grant_target VARCHAR(20) NOT NULL DEFAULT 'account', -- account, actual_user

    status VARCHAR(20) NOT NULL DEFAULT 'draft', -- draft, active, inactive
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_badge_series ON badge(series_id);
CREATE INDEX idx_badge_type ON badge(badge_type);
CREATE INDEX idx_badge_status ON badge(status);

-- ==================== 规则配置 ====================

-- 徽章获取规则
CREATE TABLE badge_rule (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    badge_id UUID NOT NULL REFERENCES badge(id),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    rule_json JSONB NOT NULL, -- 规则 JSON
    priority INT NOT NULL DEFAULT 0, -- 优先级，数值越大越优先
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_badge_rule_badge ON badge_rule(badge_id);
CREATE INDEX idx_badge_rule_json ON badge_rule USING GIN(rule_json);

-- ==================== 用户徽章 ====================

-- 用户徽章持有
CREATE TABLE user_badge (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id VARCHAR(100) NOT NULL, -- SWID
    badge_id UUID NOT NULL REFERENCES badge(id),
    quantity INT NOT NULL DEFAULT 1,
    status VARCHAR(20) NOT NULL DEFAULT 'active', -- active, expired, revoked, redeemed
    acquired_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,

    -- 发放来源
    source_type VARCHAR(20) NOT NULL, -- event, scheduled, manual
    source_ref VARCHAR(200), -- 来源引用（事件ID、任务ID等）

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_user_badge_user ON user_badge(user_id);
CREATE INDEX idx_user_badge_badge ON user_badge(badge_id);
CREATE INDEX idx_user_badge_status ON user_badge(status);
CREATE INDEX idx_user_badge_user_status ON user_badge(user_id, status);

-- 徽章账本（流水）
CREATE TABLE badge_ledger (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id VARCHAR(100) NOT NULL,
    badge_id UUID NOT NULL REFERENCES badge(id),
    user_badge_id UUID REFERENCES user_badge(id),

    change_type VARCHAR(20) NOT NULL, -- acquire, expire, cancel, redeem_out, redeem_fail
    quantity INT NOT NULL, -- 正数增加，负数减少
    balance_after INT NOT NULL, -- 变更后余额

    -- 关联来源
    ref_type VARCHAR(20) NOT NULL, -- event, scheduled, manual, redemption, system
    ref_id VARCHAR(200),

    reason TEXT,
    operator VARCHAR(100), -- 操作人（手动操作时）

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_badge_ledger_user ON badge_ledger(user_id);
CREATE INDEX idx_badge_ledger_badge ON badge_ledger(badge_id);
CREATE INDEX idx_badge_ledger_ref ON badge_ledger(ref_type, ref_id);
CREATE INDEX idx_badge_ledger_time ON badge_ledger(created_at);

-- ==================== 兑换相关 ====================

-- 权益定义
CREATE TABLE benefit (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    code VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    benefit_type VARCHAR(50) NOT NULL, -- digital_asset, coupon, reservation

    -- 外部系统关联
    external_id VARCHAR(200),
    external_system VARCHAR(50),

    -- 库存
    total_stock INT,
    remaining_stock INT,

    status VARCHAR(20) NOT NULL DEFAULT 'active',
    config JSONB, -- 权益特定配置

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 兑换规则
CREATE TABLE badge_redemption_rule (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    benefit_id UUID NOT NULL REFERENCES benefit(id),

    -- 所需徽章配置
    required_badges JSONB NOT NULL, -- [{badge_id, quantity}]

    -- 兑换时间限制
    redeem_time_start TIMESTAMPTZ,
    redeem_time_end TIMESTAMPTZ,
    redeem_after_acquire_days INT, -- 获取后N天内可兑换

    -- 兑换频次限制
    frequency_type VARCHAR(20), -- daily, weekly, monthly, yearly, account
    frequency_limit INT,

    -- 自动兑换
    auto_redeem BOOLEAN NOT NULL DEFAULT FALSE,

    status VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_redemption_rule_benefit ON badge_redemption_rule(benefit_id);

-- 兑换订单
CREATE TABLE redemption_order (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id VARCHAR(100) NOT NULL,
    redemption_rule_id UUID NOT NULL REFERENCES badge_redemption_rule(id),
    benefit_id UUID NOT NULL REFERENCES benefit(id),

    status VARCHAR(20) NOT NULL DEFAULT 'pending', -- pending, completed, failed, cancelled

    -- 权益发放结果
    benefit_grant_ref VARCHAR(200), -- 外部系统权益发放ID
    benefit_grant_at TIMESTAMPTZ,

    failure_reason TEXT,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_redemption_order_user ON redemption_order(user_id);
CREATE INDEX idx_redemption_order_status ON redemption_order(status);

-- 兑换明细
CREATE TABLE redemption_detail (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    order_id UUID NOT NULL REFERENCES redemption_order(id),
    user_badge_id UUID NOT NULL REFERENCES user_badge(id),
    badge_id UUID NOT NULL REFERENCES badge(id),
    quantity INT NOT NULL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_redemption_detail_order ON redemption_detail(order_id);

-- ==================== 通知相关 ====================

-- 通知配置
CREATE TABLE notification_config (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    badge_id UUID REFERENCES badge(id),
    benefit_id UUID REFERENCES benefit(id),

    trigger_type VARCHAR(20) NOT NULL, -- grant, revoke, expire, expire_remind, redeem
    channels JSONB NOT NULL, -- ["app_push", "sms", "wechat", "email"]
    template_id VARCHAR(100),
    advance_days INT, -- 提前通知天数（过期提醒）

    retry_count INT NOT NULL DEFAULT 3,
    retry_interval_seconds INT NOT NULL DEFAULT 60,

    status VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 通知任务
CREATE TABLE notification_task (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id VARCHAR(100) NOT NULL,

    trigger_type VARCHAR(20) NOT NULL,
    channels JSONB NOT NULL,
    template_id VARCHAR(100),
    template_params JSONB,

    status VARCHAR(20) NOT NULL DEFAULT 'pending', -- pending, processing, completed, failed
    retry_count INT NOT NULL DEFAULT 0,
    max_retries INT NOT NULL DEFAULT 3,

    last_error TEXT,
    completed_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notification_task_status ON notification_task(status);
CREATE INDEX idx_notification_task_user ON notification_task(user_id);

-- ==================== 系统管理 ====================

-- 操作日志
CREATE TABLE operation_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    operator_id VARCHAR(100) NOT NULL,
    operator_name VARCHAR(100),

    module VARCHAR(50) NOT NULL,
    action VARCHAR(50) NOT NULL,
    target_type VARCHAR(50),
    target_id VARCHAR(200),

    before_data JSONB,
    after_data JSONB,

    ip_address VARCHAR(50),
    user_agent TEXT,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_operation_log_operator ON operation_log(operator_id);
CREATE INDEX idx_operation_log_module ON operation_log(module);
CREATE INDEX idx_operation_log_time ON operation_log(created_at);

-- 批量任务
CREATE TABLE batch_task (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    task_type VARCHAR(50) NOT NULL, -- batch_grant, batch_revoke, data_export

    file_url TEXT, -- 上传的文件地址
    total_count INT NOT NULL DEFAULT 0,
    success_count INT NOT NULL DEFAULT 0,
    failure_count INT NOT NULL DEFAULT 0,

    status VARCHAR(20) NOT NULL DEFAULT 'pending', -- pending, processing, completed, failed
    progress INT NOT NULL DEFAULT 0, -- 0-100

    result_file_url TEXT, -- 结果文件地址
    error_message TEXT,

    created_by VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_batch_task_status ON batch_task(status);
CREATE INDEX idx_batch_task_creator ON batch_task(created_by);

-- ==================== 触发器 ====================

-- 更新 updated_at 触发器函数
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- 为所有表添加 updated_at 触发器
CREATE TRIGGER update_badge_category_updated_at BEFORE UPDATE ON badge_category FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_badge_series_updated_at BEFORE UPDATE ON badge_series FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_badge_updated_at BEFORE UPDATE ON badge FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_badge_rule_updated_at BEFORE UPDATE ON badge_rule FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_user_badge_updated_at BEFORE UPDATE ON user_badge FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_benefit_updated_at BEFORE UPDATE ON benefit FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_badge_redemption_rule_updated_at BEFORE UPDATE ON badge_redemption_rule FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_redemption_order_updated_at BEFORE UPDATE ON redemption_order FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_notification_config_updated_at BEFORE UPDATE ON notification_config FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_notification_task_updated_at BEFORE UPDATE ON notification_task FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_batch_task_updated_at BEFORE UPDATE ON batch_task FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
```

**Step 2: 验证 SQL 语法**

启动 PostgreSQL 并执行迁移（需要先启动 docker）：

Run: `docker compose -f docker/docker-compose.infra.yml up -d postgres && sleep 5`

Run: `docker exec -i badge-postgres psql -U badge -d badge_db < migrations/20250128_001_init_schema.sql`
Expected: 所有表创建成功

**Step 3: 提交**

```bash
git add migrations/
git commit -m "feat: 添加数据库初始化迁移脚本"
```

---

### Task 1.6: 创建前端项目骨架

**Files:**
- Create: `web/admin-ui/package.json`
- Create: `web/admin-ui/tsconfig.json`
- Create: `web/admin-ui/vite.config.ts`
- Create: `web/admin-ui/src/main.tsx`
- Create: `web/admin-ui/src/App.tsx`
- Create: `web/admin-ui/index.html`

**Step 1: 创建 package.json**

```json
{
  "name": "badge-admin-ui",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "lint": "eslint . --ext ts,tsx --report-unused-disable-directives --max-warnings 0",
    "format": "prettier --write \"src/**/*.{ts,tsx,css}\""
  },
  "dependencies": {
    "react": "^18.3.1",
    "react-dom": "^18.3.1",
    "react-router-dom": "^7.1.0",
    "antd": "^5.23.0",
    "@ant-design/pro-components": "^2.8.0",
    "@ant-design/icons": "^5.6.0",
    "zustand": "^5.0.0",
    "@tanstack/react-query": "^5.62.0",
    "axios": "^1.7.0",
    "@xyflow/react": "^12.4.0",
    "echarts": "^5.5.0",
    "echarts-for-react": "^3.0.0",
    "dayjs": "^1.11.0",
    "ahooks": "^3.8.0",
    "lodash-es": "^4.17.21"
  },
  "devDependencies": {
    "@types/react": "^18.3.0",
    "@types/react-dom": "^18.3.0",
    "@types/lodash-es": "^4.17.12",
    "@vitejs/plugin-react": "^4.3.0",
    "typescript": "^5.7.0",
    "vite": "^6.0.0",
    "eslint": "^9.17.0",
    "@eslint/js": "^9.17.0",
    "eslint-plugin-react-hooks": "^5.0.0",
    "eslint-plugin-react-refresh": "^0.4.0",
    "prettier": "^3.4.0",
    "typescript-eslint": "^8.0.0"
  }
}
```

**Step 2: 创建 tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "isolatedModules": true,
    "moduleDetection": "force",
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"]
    }
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

**Step 3: 创建 vite.config.ts**

```typescript
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { resolve } from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
  server: {
    port: 3000,
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      },
    },
  },
});
```

**Step 4: 创建入口文件**

`web/admin-ui/index.html`:
```html
<!DOCTYPE html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <link rel="icon" type="image/svg+xml" href="/vite.svg" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>徽章管理系统</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

`web/admin-ui/src/main.tsx`:
```tsx
import React from 'react';
import ReactDOM from 'react-dom/client';
import { ConfigProvider } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import App from './App';
import 'antd/dist/reset.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ConfigProvider locale={zhCN}>
      <App />
    </ConfigProvider>
  </React.StrictMode>,
);
```

`web/admin-ui/src/App.tsx`:
```tsx
import { BrowserRouter } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 5 * 60 * 1000,
      retry: 1,
    },
  },
});

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <div>
          <h1>徽章管理系统</h1>
          <p>系统初始化中...</p>
        </div>
      </BrowserRouter>
    </QueryClientProvider>
  );
}

export default App;
```

**Step 5: 安装依赖并验证**

Run: `cd web/admin-ui && pnpm install && pnpm run build`
Expected: 构建成功

**Step 6: 提交**

```bash
git add web/
git commit -m "chore: 创建前端项目骨架"
```

---

### Task 1.7: 创建开发脚本

**Files:**
- Create: `scripts/dev-setup.sh`
- Create: `scripts/run-tests.sh`
- Create: `Makefile`

**Step 1: 创建 dev-setup.sh**

```bash
#!/bin/bash
set -e

echo "🚀 Setting up development environment..."

# 检查依赖
command -v docker >/dev/null 2>&1 || { echo "❌ Docker is required but not installed."; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "❌ Cargo is required but not installed."; exit 1; }
command -v pnpm >/dev/null 2>&1 || { echo "❌ pnpm is required but not installed."; exit 1; }

# 启动基础设施
echo "📦 Starting infrastructure..."
docker compose -f docker/docker-compose.infra.yml up -d

# 等待服务就绪
echo "⏳ Waiting for services to be ready..."
sleep 10

# 运行数据库迁移
echo "🗃️ Running database migrations..."
docker exec -i badge-postgres psql -U badge -d badge_db < migrations/20250128_001_init_schema.sql || true

# 安装前端依赖
echo "📦 Installing frontend dependencies..."
cd web/admin-ui && pnpm install && cd ../..

# 构建 Rust 项目
echo "🔨 Building Rust project..."
cargo build

echo "✅ Development environment is ready!"
echo ""
echo "Available commands:"
echo "  make dev-backend   - Start all backend services"
echo "  make dev-frontend  - Start frontend dev server"
echo "  make test          - Run all tests"
echo "  make infra-up      - Start infrastructure"
echo "  make infra-down    - Stop infrastructure"
```

**Step 2: 创建 run-tests.sh**

```bash
#!/bin/bash
set -e

echo "🧪 Running tests..."

# Rust 测试
echo "📦 Running Rust tests..."
cargo test --workspace

# 前端测试
echo "📦 Running frontend tests..."
cd web/admin-ui && pnpm run lint && cd ../..

echo "✅ All tests passed!"
```

**Step 3: 创建 Makefile**

```makefile
.PHONY: all setup build test clean dev-backend dev-frontend infra-up infra-down

# 默认目标
all: build

# 开发环境设置
setup:
	./scripts/dev-setup.sh

# 构建
build:
	cargo build --workspace
	cd web/admin-ui && pnpm run build

# 测试
test:
	./scripts/run-tests.sh

# 清理
clean:
	cargo clean
	rm -rf web/admin-ui/dist
	rm -rf web/admin-ui/node_modules

# 启动后端开发服务
dev-backend:
	cargo run --bin rule-engine &
	cargo run --bin badge-management &
	cargo run --bin badge-admin &

# 启动前端开发服务
dev-frontend:
	cd web/admin-ui && pnpm run dev

# 基础设施管理
infra-up:
	docker compose -f docker/docker-compose.infra.yml up -d

infra-down:
	docker compose -f docker/docker-compose.infra.yml down

infra-logs:
	docker compose -f docker/docker-compose.infra.yml logs -f

# 数据库迁移
db-migrate:
	docker exec -i badge-postgres psql -U badge -d badge_db < migrations/20250128_001_init_schema.sql

# 代码检查
lint:
	cargo clippy --workspace -- -D warnings
	cd web/admin-ui && pnpm run lint

# 格式化
fmt:
	cargo fmt --all
	cd web/admin-ui && pnpm run format
```

**Step 4: 设置执行权限**

Run: `chmod +x scripts/*.sh`

**Step 5: 验证**

Run: `make --version && make build`
Expected: Make 版本显示，构建成功

**Step 6: 提交**

```bash
git add scripts/ Makefile
git commit -m "chore: 添加开发脚本和 Makefile"
```

---

### Task 1.8: 完成 Phase 1 验证

**Step 1: 运行完整构建**

Run: `cargo build --workspace`
Expected: 所有 crate 构建成功

**Step 2: 运行 lint**

Run: `cargo clippy --workspace -- -D warnings`
Expected: 无警告（或只有预期的 unused 警告）

**Step 3: 检查项目结构**

Run: `find . -type f -name "*.rs" | head -20`
Expected: 显示所有 Rust 源文件

**Step 4: 提交 Phase 1 完成标记**

```bash
git add -A
git commit -m "milestone: 完成 Phase 1 - 项目骨架与基础设施"
```

---

## Phase 2: Proto 定义与共享库

### Task 2.1: 定义规则引擎 Proto

**Files:**
- Create: `crates/proto/src/rule_engine.proto`
- Modify: `crates/proto/build.rs`

**Step 1: 创建 rule_engine.proto**

```protobuf
syntax = "proto3";

package badge.rule_engine;

import "google/protobuf/struct.proto";
import "google/protobuf/timestamp.proto";

// 规则引擎服务
service RuleEngineService {
  // 评估规则
  rpc Evaluate(EvaluateRequest) returns (EvaluateResponse);

  // 批量评估规则
  rpc BatchEvaluate(BatchEvaluateRequest) returns (BatchEvaluateResponse);

  // 加载/更新规则
  rpc LoadRule(LoadRuleRequest) returns (LoadRuleResponse);

  // 删除规则
  rpc DeleteRule(DeleteRuleRequest) returns (DeleteRuleResponse);

  // 测试规则
  rpc TestRule(TestRuleRequest) returns (TestRuleResponse);
}

// 规则定义
message Rule {
  string id = 1;
  string name = 2;
  string version = 3;
  RuleNode root = 4;
  google.protobuf.Timestamp created_at = 5;
  google.protobuf.Timestamp updated_at = 6;
}

// 规则节点（条件或组）
message RuleNode {
  oneof node {
    ConditionNode condition = 1;
    GroupNode group = 2;
  }
}

// 条件节点
message ConditionNode {
  string field = 1;
  Operator operator = 2;
  google.protobuf.Value value = 3;
}

// 组节点
message GroupNode {
  LogicalOperator operator = 1;
  repeated RuleNode children = 2;
}

// 操作符
enum Operator {
  OPERATOR_UNSPECIFIED = 0;
  EQ = 1;
  NEQ = 2;
  GT = 3;
  GTE = 4;
  LT = 5;
  LTE = 6;
  BETWEEN = 7;
  IN = 8;
  NOT_IN = 9;
  CONTAINS = 10;
  STARTS_WITH = 11;
  ENDS_WITH = 12;
  REGEX = 13;
  IS_EMPTY = 14;
  IS_NOT_EMPTY = 15;
  CONTAINS_ANY = 16;
  CONTAINS_ALL = 17;
  BEFORE = 18;
  AFTER = 19;
}

// 逻辑操作符
enum LogicalOperator {
  LOGICAL_OPERATOR_UNSPECIFIED = 0;
  AND = 1;
  OR = 2;
}

// 评估请求
message EvaluateRequest {
  string rule_id = 1;
  google.protobuf.Struct context = 2; // 上下文数据
}

// 评估响应
message EvaluateResponse {
  bool matched = 1;
  string rule_id = 2;
  string rule_name = 3;
  repeated string matched_conditions = 4; // 匹配的条件路径
  int64 evaluation_time_ms = 5;
}

// 批量评估请求
message BatchEvaluateRequest {
  repeated string rule_ids = 1;
  google.protobuf.Struct context = 2;
}

// 批量评估响应
message BatchEvaluateResponse {
  repeated EvaluateResponse results = 1;
  int64 total_evaluation_time_ms = 2;
}

// 加载规则请求
message LoadRuleRequest {
  Rule rule = 1;
}

// 加载规则响应
message LoadRuleResponse {
  bool success = 1;
  string message = 2;
}

// 删除规则请求
message DeleteRuleRequest {
  string rule_id = 1;
}

// 删除规则响应
message DeleteRuleResponse {
  bool success = 1;
  string message = 2;
}

// 测试规则请求
message TestRuleRequest {
  Rule rule = 1; // 待测试的规则（不需要先加载）
  google.protobuf.Struct context = 2;
}

// 测试规则响应
message TestRuleResponse {
  bool matched = 1;
  repeated string matched_conditions = 2;
  repeated string evaluation_trace = 3; // 评估过程追踪
  int64 evaluation_time_ms = 4;
}
```

**Step 2: 创建 build.rs**

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/generated")
        .compile_protos(
            &["src/rule_engine.proto"],
            &["src/"],
        )?;
    Ok(())
}
```

**Step 3: 创建 generated 目录并更新 lib.rs**

```bash
mkdir -p crates/proto/src/generated
```

`crates/proto/src/lib.rs`:
```rust
//! gRPC/Protobuf 定义

pub mod rule_engine {
    include!("generated/badge.rule_engine.rs");
}
```

**Step 4: 编译验证**

Run: `cargo build -p badge-proto`
Expected: proto 编译成功，生成 Rust 代码

**Step 5: 提交**

```bash
git add crates/proto/
git commit -m "feat(proto): 添加规则引擎 proto 定义"
```

---

### Task 2.2: 定义徽章服务 Proto

**Files:**
- Create: `crates/proto/src/badge.proto`
- Modify: `crates/proto/build.rs`

**Step 1: 创建 badge.proto**

```protobuf
syntax = "proto3";

package badge.management;

import "google/protobuf/timestamp.proto";
import "google/protobuf/wrappers.proto";

// 徽章管理服务（C端）
service BadgeManagementService {
  // 获取用户徽章列表
  rpc GetUserBadges(GetUserBadgesRequest) returns (GetUserBadgesResponse);

  // 获取徽章详情
  rpc GetBadgeDetail(GetBadgeDetailRequest) returns (GetBadgeDetailResponse);

  // 获取徽章墙
  rpc GetBadgeWall(GetBadgeWallRequest) returns (GetBadgeWallResponse);

  // 发放徽章（内部调用）
  rpc GrantBadge(GrantBadgeRequest) returns (GrantBadgeResponse);

  // 取消徽章（内部调用）
  rpc RevokeBadge(RevokeBadgeRequest) returns (RevokeBadgeResponse);

  // 兑换徽章
  rpc RedeemBadge(RedeemBadgeRequest) returns (RedeemBadgeResponse);

  // 置顶/佩戴徽章
  rpc PinBadge(PinBadgeRequest) returns (PinBadgeResponse);
}

// 徽章状态
enum BadgeStatus {
  BADGE_STATUS_UNSPECIFIED = 0;
  ACTIVE = 1;
  EXPIRED = 2;
  REVOKED = 3;
  REDEEMED = 4;
}

// 徽章类型
enum BadgeType {
  BADGE_TYPE_UNSPECIFIED = 0;
  TRANSACTION = 1;
  ENGAGEMENT = 2;
  IDENTITY = 3;
  SEASONAL = 4;
}

// 徽章信息
message Badge {
  string id = 1;
  string code = 2;
  string name = 3;
  string description = 4;
  BadgeType badge_type = 5;
  string category_name = 6;
  string series_name = 7;
  string icon_url = 8;
  string icon_3d_url = 9;
}

// 用户徽章信息
message UserBadge {
  string id = 1;
  Badge badge = 2;
  int32 quantity = 3;
  BadgeStatus status = 4;
  google.protobuf.Timestamp acquired_at = 5;
  google.protobuf.Timestamp expires_at = 6;
  bool is_pinned = 7;
}

// 获取用户徽章列表请求
message GetUserBadgesRequest {
  string user_id = 1;
  google.protobuf.StringValue badge_type = 2;
  google.protobuf.StringValue status = 3;
  int32 page = 4;
  int32 page_size = 5;
}

// 获取用户徽章列表响应
message GetUserBadgesResponse {
  repeated UserBadge badges = 1;
  int32 total = 2;
  int32 page = 3;
  int32 page_size = 4;
}

// 获取徽章详情请求
message GetBadgeDetailRequest {
  string badge_id = 1;
  string user_id = 2; // 可选，用于获取用户持有状态
}

// 获取徽章详情响应
message GetBadgeDetailResponse {
  Badge badge = 1;
  google.protobuf.Int32Value user_quantity = 2;
  google.protobuf.Timestamp user_acquired_at = 3;
  google.protobuf.Timestamp user_expires_at = 4;
  bool can_redeem = 5;
}

// 获取徽章墙请求
message GetBadgeWallRequest {
  string user_id = 1;
  string sort_by = 2; // name, type, acquired_at
  string sort_order = 3; // asc, desc
  repeated string badge_types = 4; // 筛选类型
}

// 获取徽章墙响应
message GetBadgeWallResponse {
  repeated UserBadge badges = 1;
  int32 total_count = 2;
  int32 active_count = 3;
  int32 expired_count = 4;
  int32 redeemed_count = 5;
}

// 发放徽章请求
message GrantBadgeRequest {
  string user_id = 1;
  string badge_id = 2;
  int32 quantity = 3;
  string source_type = 4; // event, scheduled, manual
  string source_ref = 5;
  string operator = 6; // 手动发放时的操作人
}

// 发放徽章响应
message GrantBadgeResponse {
  bool success = 1;
  string user_badge_id = 2;
  string message = 3;
}

// 取消徽章请求
message RevokeBadgeRequest {
  string user_id = 1;
  string badge_id = 2;
  int32 quantity = 3;
  string reason = 4;
  string operator = 5;
}

// 取消徽章响应
message RevokeBadgeResponse {
  bool success = 1;
  string message = 2;
}

// 兑换徽章请求
message RedeemBadgeRequest {
  string user_id = 1;
  string redemption_rule_id = 2;
}

// 兑换徽章响应
message RedeemBadgeResponse {
  bool success = 1;
  string order_id = 2;
  string benefit_id = 3;
  string benefit_name = 4;
  string message = 5;
}

// 置顶徽章请求
message PinBadgeRequest {
  string user_id = 1;
  string user_badge_id = 2;
  bool pin = 3;
}

// 置顶徽章响应
message PinBadgeResponse {
  bool success = 1;
  string message = 2;
}
```

**Step 2: 更新 build.rs**

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/generated")
        .compile_protos(
            &[
                "src/rule_engine.proto",
                "src/badge.proto",
            ],
            &["src/"],
        )?;
    Ok(())
}
```

**Step 3: 更新 lib.rs**

```rust
//! gRPC/Protobuf 定义

pub mod rule_engine {
    include!("generated/badge.rule_engine.rs");
}

pub mod badge {
    include!("generated/badge.management.rs");
}
```

**Step 4: 编译验证**

Run: `cargo build -p badge-proto`
Expected: 编译成功

**Step 5: 提交**

```bash
git add crates/proto/
git commit -m "feat(proto): 添加徽章管理服务 proto 定义"
```

---

### Task 2.3: 实现共享错误处理模块

**Files:**
- Modify: `crates/shared/src/error.rs`

**Step 1: 实现错误类型**

```rust
//! 统一错误处理模块
//!
//! 定义系统中所有共享的错误类型，使用 thiserror 提供良好的错误信息。

use thiserror::Error;

/// 系统错误类型
#[derive(Debug, Error)]
pub enum BadgeError {
    // ==================== 数据库错误 ====================
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),

    #[error("记录未找到: {entity} id={id}")]
    NotFound { entity: String, id: String },

    #[error("记录已存在: {entity} {field}={value}")]
    AlreadyExists {
        entity: String,
        field: String,
        value: String,
    },

    // ==================== 缓存错误 ====================
    #[error("Redis 错误: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("缓存未命中: {key}")]
    CacheMiss { key: String },

    // ==================== Kafka 错误 ====================
    #[error("Kafka 错误: {0}")]
    Kafka(String),

    // ==================== 业务逻辑错误 ====================
    #[error("徽章余额不足: 需要 {required}, 实际 {actual}")]
    InsufficientBalance { required: i32, actual: i32 },

    #[error("徽章已过期: badge_id={badge_id}")]
    BadgeExpired { badge_id: String },

    #[error("兑换条件不满足: {reason}")]
    RedemptionConditionNotMet { reason: String },

    #[error("操作频率超限: {operation}")]
    RateLimitExceeded { operation: String },

    #[error("徽章不可用: {reason}")]
    BadgeUnavailable { reason: String },

    // ==================== 规则引擎错误 ====================
    #[error("规则解析失败: {0}")]
    RuleParseFailed(String),

    #[error("规则执行失败: {0}")]
    RuleExecutionFailed(String),

    #[error("规则未找到: rule_id={rule_id}")]
    RuleNotFound { rule_id: String },

    // ==================== 验证错误 ====================
    #[error("参数验证失败: {0}")]
    Validation(String),

    #[error("无效的参数: {field} - {message}")]
    InvalidArgument { field: String, message: String },

    // ==================== 权限错误 ====================
    #[error("未授权访问")]
    Unauthorized,

    #[error("权限不足: {operation}")]
    Forbidden { operation: String },

    // ==================== 外部服务错误 ====================
    #[error("外部服务错误: {service} - {message}")]
    ExternalService { service: String, message: String },

    #[error("外部服务超时: {service}")]
    ExternalServiceTimeout { service: String },

    // ==================== 通用错误 ====================
    #[error("内部错误: {0}")]
    Internal(String),

    #[error("{0}")]
    Custom(String),
}

/// 错误结果类型别名
pub type Result<T> = std::result::Result<T, BadgeError>;

impl BadgeError {
    /// 获取错误码
    pub fn code(&self) -> &'static str {
        match self {
            Self::Database(_) => "DATABASE_ERROR",
            Self::NotFound { .. } => "NOT_FOUND",
            Self::AlreadyExists { .. } => "ALREADY_EXISTS",
            Self::Redis(_) => "REDIS_ERROR",
            Self::CacheMiss { .. } => "CACHE_MISS",
            Self::Kafka(_) => "KAFKA_ERROR",
            Self::InsufficientBalance { .. } => "INSUFFICIENT_BALANCE",
            Self::BadgeExpired { .. } => "BADGE_EXPIRED",
            Self::RedemptionConditionNotMet { .. } => "REDEMPTION_CONDITION_NOT_MET",
            Self::RateLimitExceeded { .. } => "RATE_LIMIT_EXCEEDED",
            Self::BadgeUnavailable { .. } => "BADGE_UNAVAILABLE",
            Self::RuleParseFailed(_) => "RULE_PARSE_FAILED",
            Self::RuleExecutionFailed(_) => "RULE_EXECUTION_FAILED",
            Self::RuleNotFound { .. } => "RULE_NOT_FOUND",
            Self::Validation(_) => "VALIDATION_ERROR",
            Self::InvalidArgument { .. } => "INVALID_ARGUMENT",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden { .. } => "FORBIDDEN",
            Self::ExternalService { .. } => "EXTERNAL_SERVICE_ERROR",
            Self::ExternalServiceTimeout { .. } => "EXTERNAL_SERVICE_TIMEOUT",
            Self::Internal(_) => "INTERNAL_ERROR",
            Self::Custom(_) => "CUSTOM_ERROR",
        }
    }

    /// 是否为可重试错误
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Database(_)
                | Self::Redis(_)
                | Self::Kafka(_)
                | Self::ExternalServiceTimeout { .. }
        )
    }

    /// 转换为 gRPC 状态码
    pub fn to_grpc_status(&self) -> tonic::Status {
        use tonic::{Code, Status};

        let (code, message) = match self {
            Self::NotFound { .. } => (Code::NotFound, self.to_string()),
            Self::AlreadyExists { .. } => (Code::AlreadyExists, self.to_string()),
            Self::Validation(_) | Self::InvalidArgument { .. } => {
                (Code::InvalidArgument, self.to_string())
            }
            Self::Unauthorized => (Code::Unauthenticated, self.to_string()),
            Self::Forbidden { .. } => (Code::PermissionDenied, self.to_string()),
            Self::RateLimitExceeded { .. } => (Code::ResourceExhausted, self.to_string()),
            Self::ExternalServiceTimeout { .. } => (Code::DeadlineExceeded, self.to_string()),
            _ => (Code::Internal, self.to_string()),
        };

        Status::new(code, message)
    }
}

impl From<BadgeError> for tonic::Status {
    fn from(err: BadgeError) -> Self {
        err.to_grpc_status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code() {
        let err = BadgeError::NotFound {
            entity: "Badge".to_string(),
            id: "123".to_string(),
        };
        assert_eq!(err.code(), "NOT_FOUND");
    }

    #[test]
    fn test_is_retryable() {
        let db_err = BadgeError::Database(sqlx::Error::PoolTimedOut);
        assert!(db_err.is_retryable());

        let not_found = BadgeError::NotFound {
            entity: "Badge".to_string(),
            id: "123".to_string(),
        };
        assert!(!not_found.is_retryable());
    }
}
```

**Step 2: 编译并运行测试**

Run: `cargo test -p badge-shared error`
Expected: 测试通过

**Step 3: 提交**

```bash
git add crates/shared/
git commit -m "feat(shared): 实现统一错误处理模块"
```

---

### Task 2.4: 实现共享配置模块

**Files:**
- Modify: `crates/shared/src/config.rs`
- Create: `config/default.toml`

**Step 1: 实现配置模块**

```rust
//! 配置管理模块
//!
//! 支持多格式配置文件加载，环境变量覆盖，以及类型安全的配置访问。

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::path::Path;

/// 数据库配置
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://badge:badge_secret@localhost:5432/badge_db".to_string(),
            max_connections: 10,
            min_connections: 2,
            connect_timeout_seconds: 30,
            idle_timeout_seconds: 600,
        }
    }
}

/// Redis 配置
#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            pool_size: 10,
        }
    }
}

/// Kafka 配置
#[derive(Debug, Clone, Deserialize)]
pub struct KafkaConfig {
    pub brokers: String,
    pub consumer_group: String,
    pub auto_offset_reset: String,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            brokers: "localhost:9092".to_string(),
            consumer_group: "badge-service".to_string(),
            auto_offset_reset: "earliest".to_string(),
        }
    }
}

/// 服务配置
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            workers: None,
        }
    }
}

/// 可观测性配置
#[derive(Debug, Clone, Deserialize)]
pub struct ObservabilityConfig {
    pub log_level: String,
    pub log_format: String, // json, pretty
    pub metrics_enabled: bool,
    pub metrics_port: u16,
    pub tracing_enabled: bool,
    pub tracing_endpoint: Option<String>,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            log_format: "pretty".to_string(),
            metrics_enabled: true,
            metrics_port: 9090,
            tracing_enabled: false,
            tracing_endpoint: None,
        }
    }
}

/// 应用配置
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AppConfig {
    pub service_name: String,
    pub environment: String,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub kafka: KafkaConfig,
    pub observability: ObservabilityConfig,
}

impl AppConfig {
    /// 从配置文件和环境变量加载配置
    ///
    /// 加载顺序：
    /// 1. config/default.toml（默认配置）
    /// 2. config/{environment}.toml（环境特定配置）
    /// 3. 环境变量（BADGE_ 前缀）
    pub fn load(service_name: &str) -> Result<Self, ConfigError> {
        let env = std::env::var("BADGE_ENV").unwrap_or_else(|_| "development".to_string());

        let config_dir = std::env::var("CONFIG_DIR").unwrap_or_else(|_| "config".to_string());

        let builder = Config::builder()
            // 默认配置
            .set_default("service_name", service_name)?
            .set_default("environment", env.clone())?
            // 加载默认配置文件
            .add_source(File::from(Path::new(&config_dir).join("default.toml")).required(false))
            // 加载环境特定配置
            .add_source(
                File::from(Path::new(&config_dir).join(format!("{}.toml", env))).required(false),
            )
            // 环境变量覆盖（BADGE_DATABASE_URL -> database.url）
            .add_source(
                Environment::with_prefix("BADGE")
                    .separator("_")
                    .try_parsing(true),
            );

        builder.build()?.try_deserialize()
    }

    /// 获取服务地址
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    /// 是否为生产环境
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.database.max_connections, 10);
    }

    #[test]
    fn test_server_addr() {
        let config = AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3000,
                workers: None,
            },
            ..Default::default()
        };
        assert_eq!(config.server_addr(), "127.0.0.1:3000");
    }
}
```

**Step 2: 创建默认配置文件**

`config/default.toml`:
```toml
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
log_format = "pretty"
metrics_enabled = true
metrics_port = 9090
tracing_enabled = false
```

**Step 3: 编译并测试**

Run: `cargo test -p badge-shared config`
Expected: 测试通过

**Step 4: 提交**

```bash
git add crates/shared/ config/
git commit -m "feat(shared): 实现配置管理模块"
```

---

### Task 2.5: 实现共享数据库连接模块

**Files:**
- Modify: `crates/shared/src/database.rs`

**Step 1: 实现数据库模块**

```rust
//! 数据库连接管理模块
//!
//! 提供 PostgreSQL 连接池管理，支持健康检查和连接配置。

use crate::config::DatabaseConfig;
use crate::error::{BadgeError, Result};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use tracing::{info, instrument};

/// 数据库连接池包装
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// 创建数据库连接池
    #[instrument(skip(config))]
    pub async fn connect(config: &DatabaseConfig) -> Result<Self> {
        info!("Connecting to database...");

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(config.connect_timeout_seconds))
            .idle_timeout(Duration::from_secs(config.idle_timeout_seconds))
            .connect(&config.url)
            .await?;

        info!("Database connection pool created");

        Ok(Self { pool })
    }

    /// 获取连接池引用
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(BadgeError::from)
    }

    /// 关闭连接池
    pub async fn close(&self) {
        self.pool.close().await;
        info!("Database connection pool closed");
    }

    /// 运行迁移
    #[instrument(skip(self))]
    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations...");
        sqlx::migrate!("../../migrations")
            .run(&self.pool)
            .await
            .map_err(|e| BadgeError::Database(e.into()))?;
        info!("Database migrations completed");
        Ok(())
    }
}

impl std::ops::Deref for Database {
    type Target = PgPool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要数据库连接
    async fn test_database_connection() {
        let config = DatabaseConfig::default();
        let db = Database::connect(&config).await.unwrap();
        db.health_check().await.unwrap();
    }
}
```

**Step 2: 编译验证**

Run: `cargo build -p badge-shared`
Expected: 编译成功

**Step 3: 提交**

```bash
git add crates/shared/
git commit -m "feat(shared): 实现数据库连接管理模块"
```

---

### Task 2.6: 实现共享 Redis 缓存模块

**Files:**
- Modify: `crates/shared/src/cache.rs`

**Step 1: 实现缓存模块**

```rust
//! Redis 缓存管理模块
//!
//! 提供 Redis 连接管理和常用缓存操作封装。

use crate::config::RedisConfig;
use crate::error::{BadgeError, Result};
use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, Client};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use tracing::{info, instrument};

/// Redis 缓存客户端
#[derive(Clone)]
pub struct Cache {
    client: Client,
}

impl Cache {
    /// 创建 Redis 客户端
    pub fn new(config: &RedisConfig) -> Result<Self> {
        let client = Client::open(config.url.as_str())?;
        info!("Redis client created");
        Ok(Self { client })
    }

    /// 获取连接
    async fn get_conn(&self) -> Result<MultiplexedConnection> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(BadgeError::from)
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<()> {
        let mut conn = self.get_conn().await?;
        redis::cmd("PING")
            .query_async::<String>(&mut conn)
            .await
            .map(|_| ())
            .map_err(BadgeError::from)
    }

    /// 获取值
    #[instrument(skip(self))]
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.get_conn().await?;
        let value: Option<String> = conn.get(key).await?;

        match value {
            Some(v) => {
                let parsed: T = serde_json::from_str(&v)
                    .map_err(|e| BadgeError::Internal(format!("Cache deserialization error: {}", e)))?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    /// 设置值
    #[instrument(skip(self, value))]
    pub async fn set<T: Serialize>(&self, key: &str, value: &T, ttl: Duration) -> Result<()> {
        let mut conn = self.get_conn().await?;
        let serialized = serde_json::to_string(value)
            .map_err(|e| BadgeError::Internal(format!("Cache serialization error: {}", e)))?;

        conn.set_ex(key, serialized, ttl.as_secs()).await?;
        Ok(())
    }

    /// 删除值
    #[instrument(skip(self))]
    pub async fn delete(&self, key: &str) -> Result<()> {
        let mut conn = self.get_conn().await?;
        conn.del(key).await?;
        Ok(())
    }

    /// 批量删除（按模式）
    #[instrument(skip(self))]
    pub async fn delete_pattern(&self, pattern: &str) -> Result<u64> {
        let mut conn = self.get_conn().await?;
        let keys: Vec<String> = conn.keys(pattern).await?;

        if keys.is_empty() {
            return Ok(0);
        }

        let count: u64 = conn.del(keys).await?;
        Ok(count)
    }

    /// 检查键是否存在
    pub async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.get_conn().await?;
        let exists: bool = conn.exists(key).await?;
        Ok(exists)
    }

    /// 获取或设置（缓存穿透保护）
    #[instrument(skip(self, loader))]
    pub async fn get_or_set<T, F, Fut>(
        &self,
        key: &str,
        ttl: Duration,
        loader: F,
    ) -> Result<T>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        // 尝试从缓存获取
        if let Some(cached) = self.get::<T>(key).await? {
            return Ok(cached);
        }

        // 从数据源加载
        let value = loader().await?;

        // 写入缓存
        self.set(key, &value, ttl).await?;

        Ok(value)
    }

    /// 增量操作
    pub async fn incr(&self, key: &str, delta: i64) -> Result<i64> {
        let mut conn = self.get_conn().await?;
        let result: i64 = conn.incr(key, delta).await?;
        Ok(result)
    }

    /// 设置过期时间
    pub async fn expire(&self, key: &str, ttl: Duration) -> Result<()> {
        let mut conn = self.get_conn().await?;
        conn.expire(key, ttl.as_secs() as i64).await?;
        Ok(())
    }
}

/// 缓存键生成器
pub struct CacheKey;

impl CacheKey {
    pub fn user_badges(user_id: &str) -> String {
        format!("user:badge:{}", user_id)
    }

    pub fn badge_detail(badge_id: &str) -> String {
        format!("badge:detail:{}", badge_id)
    }

    pub fn badge_config(badge_id: &str) -> String {
        format!("badge:config:{}", badge_id)
    }

    pub fn user_badge_count(user_id: &str) -> String {
        format!("user:badge:count:{}", user_id)
    }

    pub fn rule(rule_id: &str) -> String {
        format!("rule:{}", rule_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_generation() {
        assert_eq!(CacheKey::user_badges("123"), "user:badge:123");
        assert_eq!(CacheKey::badge_detail("abc"), "badge:detail:abc");
    }
}
```

**Step 2: 编译验证**

Run: `cargo build -p badge-shared`
Expected: 编译成功

**Step 3: 提交**

```bash
git add crates/shared/
git commit -m "feat(shared): 实现 Redis 缓存管理模块"
```

---

## Phase 3-9: 后续阶段

由于篇幅限制，后续阶段（Phase 3-9）将在实现过程中逐步展开。每个阶段包含：

### Phase 3: 规则引擎服务 (10 tasks)
- 规则 JSON 解析器
- 规则编译器（AST 构建）
- 规则执行器（短路求值）
- 规则缓存与热更新
- gRPC 服务实现
- 单元测试与集成测试

### Phase 4: 徽章管理服务 C端 (12 tasks)
- 徽章查询服务
- 徽章墙服务
- 徽章发放服务
- 徽章取消服务
- 兑换服务（含事务）
- 账本记录服务
- gRPC 服务实现

### Phase 5: 徽章管理服务 B端 (10 tasks)
- 徽章 CRUD API
- 规则配置 API
- 发放管理 API
- 统计报表 API
- 批量导入服务
- 系统管理 API

### Phase 6: 事件处理服务 (8 tasks)
- Kafka 消费者实现
- 事件处理管道
- 规则匹配与徽章发放
- 幂等处理与去重
- 死信队列处理

### Phase 7: 模拟外部系统 (8 tasks)
- Mock 订单服务
- Mock Profile 服务
- Mock Coupon 服务
- Mock 事件生成器
- 场景模拟器

### Phase 8: 管理后台前端 (15 tasks)
- 布局与路由
- 徽章管理页面
- 规则画布组件
- 发放管理页面
- 数据看板
- 会员视图

### Phase 9: 集成测试与优化 (5 tasks)
- 端到端测试
- 性能测试
- 安全审计
- 文档完善

---

## 执行检查点

每个 Phase 完成后需要验证：

1. **编译通过**: `cargo build --workspace`
2. **测试通过**: `cargo test --workspace`
3. **Lint 通过**: `cargo clippy --workspace -- -D warnings`
4. **提交代码**: 创建 milestone commit

---

## 下一步

计划已保存至 `docs/plans/2025-01-28-badge-impl-plan.md`。

两种执行方式：

**1. Subagent-Driven（当前会话）** - 每个任务派发新 subagent，任务间进行代码审查，快速迭代

**2. Parallel Session（独立会话）** - 在新会话中使用 executing-plans，批量执行带检查点

你更倾向于哪种方式？
