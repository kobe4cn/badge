# 事件服务动态规则加载设计方案

> 创建日期: 2026-01-31
> 状态: 已确认，待实现

## 概述

为 `event-engagement-service` 和 `event-transaction-service` 实现从数据库动态加载规则配置，支持规则的动态增删、禁用，以及完善的校验机制。

## 设计决策

| 决策项 | 选择 | 说明 |
|--------|------|------|
| 规则同步策略 | 混合模式 | 定时轮询 + Kafka 即时刷新 |
| 刷新间隔 | 可配置 | 通过配置文件设置，默认 30 秒 |
| 规则与事件类型关联 | 扩展表结构 | 新增 `event_types` 表，`badge_rules` 添加 `event_type` 字段 |
| 即时刷新触发 | Kafka 消息 | 管理后台修改规则后发送到 `badge.rule.reload` topic |
| Kafka Topics | 配置文件 + 环境变量 | 配置文件设置默认值，环境变量可覆盖 |
| 前链路 Topic 路由 | 保持硬编码 | mock-services 中的路由逻辑暂不改动 |

## 需实现的场景

- [x] 规则时间有效性（start_time/end_time）
- [x] 用户发放次数限制（max_count_per_user）
- [x] 规则启用/禁用（enabled）
- [x] 服务启动预热
- [x] 规则全局配额
- [x] 幂等窗口可配置

---

## 一、数据库表结构变更

### 1.1 新增 `event_types` 配置表

```sql
CREATE TABLE event_types (
    id BIGSERIAL PRIMARY KEY,
    code VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    service_group VARCHAR(50) NOT NULL,  -- 'transaction' 或 'engagement'
    description TEXT,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO event_types (code, name, service_group) VALUES
    ('purchase', '购买', 'transaction'),
    ('refund', '退款', 'transaction'),
    ('order_cancel', '订单取消', 'transaction'),
    ('checkin', '签到', 'engagement'),
    ('page_view', '页面浏览', 'engagement'),
    ('share', '分享', 'engagement'),
    ('profile_update', '资料更新', 'engagement'),
    ('review', '评价', 'engagement');
```

### 1.2 扩展 `badge_rules` 表

```sql
ALTER TABLE badge_rules ADD COLUMN event_type VARCHAR(50);
ALTER TABLE badge_rules ADD COLUMN rule_code VARCHAR(100);
ALTER TABLE badge_rules ADD COLUMN global_quota INT;
ALTER TABLE badge_rules ADD COLUMN global_granted INT NOT NULL DEFAULT 0;

ALTER TABLE badge_rules ADD CONSTRAINT fk_badge_rules_event_type
    FOREIGN KEY (event_type) REFERENCES event_types(code);
ALTER TABLE badge_rules ADD CONSTRAINT uq_badge_rules_rule_code UNIQUE (rule_code);
CREATE INDEX idx_badge_rules_event_type_enabled ON badge_rules(event_type, enabled);
```

### 1.3 规则加载 SQL

```sql
SELECT r.id, r.rule_code, r.badge_id, r.rule_json, r.start_time, r.end_time,
       r.max_count_per_user, r.global_quota, r.global_granted, r.event_type
FROM badge_rules r
JOIN event_types et ON r.event_type = et.code
WHERE et.service_group = $1
  AND et.enabled = TRUE
  AND r.enabled = TRUE
  AND (r.start_time IS NULL OR r.start_time <= NOW())
  AND (r.end_time IS NULL OR r.end_time > NOW());
```

---

## 二、规则加载与刷新机制

### 2.1 核心组件：`RuleLoader`

```rust
pub struct RuleLoader {
    db_pool: PgPool,
    service_group: String,
    rule_mapping: Arc<RuleBadgeMapping>,
    refresh_interval: Duration,
}

impl RuleLoader {
    pub fn start_background_refresh(&self, shutdown: watch::Receiver<bool>);
    pub async fn reload_now(&self) -> Result<usize>;
    pub async fn initial_load(&self) -> Result<usize>;
}
```

### 2.2 配置项

```toml
[rules]
refresh_interval_secs = 30
initial_load_timeout_secs = 10
idempotency_ttl_hours = 24
```

### 2.3 Kafka 即时刷新

```rust
pub const RULE_RELOAD_EVENTS: &str = "badge.rule.reload";

#[derive(Serialize, Deserialize)]
pub struct RuleReloadEvent {
    pub service_group: Option<String>,
    pub event_type: Option<String>,
    pub trigger_source: String,
    pub triggered_at: DateTime<Utc>,
}
```

### 2.4 启动流程（预热）

1. 初始化数据库连接池
2. 调用 `RuleLoader::initial_load()` 阻塞加载规则
3. 若加载失败或超时，服务启动失败（fail-fast）
4. 启动 Kafka 消费者（业务事件 + 规则刷新事件）
5. 启动后台定时刷新任务
6. 健康检查返回 ready

---

## 三、扩展规则结构与校验逻辑

### 3.1 扩展 `BadgeGrant` 结构

```rust
#[derive(Debug, Clone)]
pub struct BadgeGrant {
    pub rule_id: i64,
    pub rule_code: String,
    pub badge_id: i64,
    pub badge_name: String,
    pub quantity: i32,
    pub event_type: String,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub max_count_per_user: Option<i32>,
    pub global_quota: Option<i32>,
    pub global_granted: i32,
}
```

### 3.2 规则校验器 `RuleValidator`

```rust
pub struct RuleValidator {
    cache: Cache,
    db_pool: PgPool,
}

impl RuleValidator {
    pub async fn can_grant(
        &self,
        rule: &BadgeGrant,
        user_id: &str,
    ) -> Result<ValidationResult, ValidateError>;
}
```

### 3.3 校验结果返回规范

```rust
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub allowed: bool,
    pub rule_id: i64,
    pub rule_code: String,
    pub user_id: String,
    pub reason: ValidationReason,
    pub context: ValidationContext,
}

#[derive(Debug, Clone)]
pub enum ValidationReason {
    Allowed,
    RuleExpired { end_time: DateTime<Utc> },
    RuleNotStarted { start_time: DateTime<Utc> },
    UserLimitExceeded { current: i32, max: i32 },
    GlobalQuotaExhausted { granted: i32, quota: i32 },
}

impl ValidationReason {
    pub fn deny_code(&self) -> &'static str;
    pub fn message(&self) -> String;
}
```

### 3.4 全局配额更新结果

```rust
pub enum QuotaUpdateResult {
    Success { rule_id: i64, previous: i32, current: i32, remaining: Option<i32> },
    QuotaExhausted { rule_id: i64, quota: i32, granted: i32, requested: i32 },
    RuleNotFound { rule_id: i64 },
}
```

---

## 四、处理器改造与流程整合

### 4.1 改造后的处理流程

```
1. Kafka 消费事件
2. 反序列化 → EventPayload
3. 幂等检查 (Redis, TTL 可配置)
4. 从 RuleBadgeMapping 获取该 event_type 的所有规则
5. 对每条规则:
   ├─ 5.1 RuleValidator.can_grant() 校验
   │       ├─ 时间有效性
   │       ├─ 用户发放次数
   │       └─ 全局配额
   ├─ 5.2 若校验通过 → 调用规则引擎评估
   ├─ 5.3 若规则匹配 → 原子更新全局配额
   └─ 5.4 若配额更新成功 → gRPC 发放徽章
6. 收集结果 (成功/跳过/失败 分别记录)
7. 标记事件已处理
8. 发送通知 / 写入死信队列
```

### 4.2 改造 `RuleBadgeMapping`

```rust
pub struct RuleBadgeMapping {
    mappings: DashMap<String, Vec<BadgeGrant>>,
    last_loaded_at: AtomicCell<Option<DateTime<Utc>>>,
    rule_count: AtomicUsize,
}

impl RuleBadgeMapping {
    pub fn get_rules_by_event_type(&self, event_type: &str) -> Vec<BadgeGrant>;
    pub fn replace_all(&self, rules: Vec<BadgeGrant>);
    pub fn load_status(&self) -> LoadStatus;
}
```

### 4.3 处理结果扩展

```rust
pub struct EventResult {
    pub event_id: String,
    pub processed: bool,
    pub processing_time_ms: i64,
    pub rules_checked: usize,
    pub rules_validated: Vec<RuleValidationSummary>,
    pub matched_rules: Vec<MatchedRule>,
    pub granted_badges: Vec<GrantedBadge>,
    pub skipped_rules: Vec<SkippedRule>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SkippedRule {
    pub rule_id: i64,
    pub rule_code: String,
    pub skip_reason: ValidationReason,
}
```

---

## 五、共享模块与配置

### 5.1 新增共享模块结构

```
crates/shared/src/
├── lib.rs
├── ...
└── rules/
    ├── mod.rs
    ├── loader.rs
    ├── validator.rs
    ├── mapping.rs
    └── models.rs
```

### 5.2 配置结构扩展

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct RulesConfig {
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_secs: u64,  // 默认 30

    #[serde(default = "default_initial_timeout")]
    pub initial_load_timeout_secs: u64,  // 默认 10

    #[serde(default = "default_idempotency_ttl")]
    pub idempotency_ttl_hours: u64,  // 默认 24
}

#[derive(Debug, Clone, Deserialize)]
pub struct KafkaTopicsConfig {
    #[serde(default)] pub engagement_events: String,
    #[serde(default)] pub transaction_events: String,
    #[serde(default)] pub notifications: String,
    #[serde(default)] pub dead_letter_queue: String,
    #[serde(default)] pub rule_reload: String,
}
```

### 5.3 配置文件示例

```toml
# config/default.toml
[kafka]
brokers = "localhost:9092"
consumer_group = "badge-service"

[kafka.topics]
engagement_events = "badge.engagement.events"
transaction_events = "badge.transaction.events"
notifications = "badge.notifications"
dead_letter_queue = "badge.dlq"
rule_reload = "badge.rule.reload"

[rules]
refresh_interval_secs = 30
initial_load_timeout_secs = 10
idempotency_ttl_hours = 24
```

### 5.4 环境变量覆盖

```bash
export KAFKA__BROKERS="kafka-prod:9092"
export KAFKA__TOPICS__ENGAGEMENT_EVENTS="prod.badge.engagement.events"
```

---

## 六、实现任务清单

| # | 任务 | 文件 | 优先级 |
|---|------|------|--------|
| 1 | 执行数据库迁移 | migrations/20250131_003_dynamic_rules.sql | P0 |
| 2 | 扩展配置结构 | crates/shared/src/config.rs | P0 |
| 3 | 实现 rules 共享模块 | crates/shared/src/rules/*.rs | P0 |
| 4 | 添加数据库连接到事件服务 | 两个服务的 Cargo.toml, main.rs | P0 |
| 5 | 改造 Processor 使用新的校验流程 | processor.rs (两个服务) | P0 |
| 6 | 实现 Kafka 规则刷新消费 | consumer.rs (两个服务) | P1 |
| 7 | 更新配置文件 | config/*.toml | P1 |
| 8 | 管理后台添加事件类型管理 | badge-admin-service | P2 |
| 9 | 补充单元测试 | 各模块 tests | P1 |
| 10 | 全链路测试 | examples/full_pipeline_test.rs | P1 |

---

## 架构图

```
┌─────────────────────────────────────────────────────────────────────┐
│                          前链路（事件生产方）                         │
└──────────────────────────┬──────────────────────────────────────────┘
                           │
              ┌────────────┴────────────┐
              ▼                         ▼
┌─────────────────────────┐   ┌─────────────────────────┐
│ badge.transaction.events │   │ badge.engagement.events  │
└───────────┬─────────────┘   └───────────┬──────────────┘
            │                             │
            ▼                             ▼
┌─────────────────────────┐   ┌─────────────────────────┐
│ event-transaction-service│   │ event-engagement-service │
│                          │   │                          │
│  ┌─────────────────┐     │   │  ┌─────────────────┐     │
│  │  RuleLoader     │◄────┼───┼──│  RuleLoader     │     │
│  │  (from DB)      │     │   │  │  (from DB)      │     │
│  └────────┬────────┘     │   │  └────────┬────────┘     │
│           ▼              │   │           ▼              │
│  ┌─────────────────┐     │   │  ┌─────────────────┐     │
│  │ RuleBadgeMapping│     │   │  │ RuleBadgeMapping│     │
│  └────────┬────────┘     │   │  └────────┬────────┘     │
│           ▼              │   │           ▼              │
│  ┌─────────────────┐     │   │  ┌─────────────────┐     │
│  │ RuleValidator   │     │   │  │ RuleValidator   │     │
│  └────────┬────────┘     │   │  └────────┬────────┘     │
│           ▼              │   │           ▼              │
│  ┌─────────────────┐     │   │  ┌─────────────────┐     │
│  │   Processor     │     │   │  │   Processor     │     │
│  └─────────────────┘     │   │  └─────────────────┘     │
└─────────────────────────┘   └──────────────────────────┘
            │                             │
            └──────────┬──────────────────┘
                       ▼
              ┌─────────────────┐
              │ badge.rule.reload│ (Kafka - 即时刷新)
              └─────────────────┘
                       ▲
                       │
              ┌─────────────────┐
              │ badge-admin-svc  │ (管理后台触发)
              └─────────────────┘
```
