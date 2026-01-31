# 事件服务动态规则加载实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 为 event-engagement-service 和 event-transaction-service 实现从数据库动态加载规则配置，支持规则的动态增删、禁用，以及完善的校验机制。

**Architecture:** 在 badge-shared 中新增 rules 模块，提供 RuleLoader（数据库加载）、RuleBadgeMapping（内存缓存）、RuleValidator（校验器）三个核心组件。事件服务启动时阻塞加载规则，之后通过定时轮询 + Kafka 即时刷新保持规则同步。

**Tech Stack:** Rust, SQLx, tokio, DashMap, rdkafka, serde

---

## Task 1: 数据库迁移 - 创建 event_types 表和扩展 badge_rules 表

**Files:**
- Create: `migrations/20250131_003_dynamic_rules.sql`

**Step 1: 创建迁移文件**

```sql
-- migrations/20250131_003_dynamic_rules.sql
-- 动态规则加载支持

-- ==================== 事件类型配置表 ====================

CREATE TABLE event_types (
    id BIGSERIAL PRIMARY KEY,
    code VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    service_group VARCHAR(50) NOT NULL,
    description TEXT,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE event_types IS '事件类型配置，定义可触发徽章规则的事件类型';
COMMENT ON COLUMN event_types.code IS '事件类型代码，如 purchase, checkin';
COMMENT ON COLUMN event_types.service_group IS '所属服务组：transaction 或 engagement';

-- 预置事件类型
INSERT INTO event_types (code, name, service_group, description) VALUES
    ('purchase', '购买', 'transaction', '用户完成购买订单'),
    ('refund', '退款', 'transaction', '用户发起退款'),
    ('order_cancel', '订单取消', 'transaction', '用户取消订单'),
    ('checkin', '签到', 'engagement', '用户每日签到'),
    ('page_view', '页面浏览', 'engagement', '用户浏览指定页面'),
    ('share', '分享', 'engagement', '用户分享内容'),
    ('profile_update', '资料更新', 'engagement', '用户更新个人资料'),
    ('review', '评价', 'engagement', '用户提交评价');

-- 触发器
CREATE TRIGGER trg_event_types_updated_at
    BEFORE UPDATE ON event_types
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ==================== 扩展 badge_rules 表 ====================

-- 添加新字段
ALTER TABLE badge_rules ADD COLUMN event_type VARCHAR(50);
ALTER TABLE badge_rules ADD COLUMN rule_code VARCHAR(100);
ALTER TABLE badge_rules ADD COLUMN global_quota INT;
ALTER TABLE badge_rules ADD COLUMN global_granted INT NOT NULL DEFAULT 0;

-- 添加约束
ALTER TABLE badge_rules ADD CONSTRAINT fk_badge_rules_event_type
    FOREIGN KEY (event_type) REFERENCES event_types(code);
ALTER TABLE badge_rules ADD CONSTRAINT uq_badge_rules_rule_code
    UNIQUE (rule_code);

-- 添加索引
CREATE INDEX idx_badge_rules_event_type_enabled
    ON badge_rules(event_type, enabled) WHERE enabled = TRUE;

COMMENT ON COLUMN badge_rules.event_type IS '触发此规则的事件类型';
COMMENT ON COLUMN badge_rules.rule_code IS '规则唯一标识符，用于日志和调试';
COMMENT ON COLUMN badge_rules.global_quota IS '全局配额限制，NULL表示不限制';
COMMENT ON COLUMN badge_rules.global_granted IS '已发放数量，用于配额控制';
```

**Step 2: 验证迁移文件语法**

Run: `cat migrations/20250131_003_dynamic_rules.sql | head -50`
Expected: SQL 文件内容正确显示

**Step 3: Commit**

```bash
git add migrations/20250131_003_dynamic_rules.sql
git commit -m "feat(db): add event_types table and extend badge_rules for dynamic loading"
```

---

## Task 2: 扩展共享配置结构

**Files:**
- Modify: `crates/shared/src/config.rs`

**Step 1: 添加 RulesConfig 和 KafkaTopicsConfig 结构**

在 `config.rs` 文件末尾（`impl AppConfig` 之前）添加：

```rust
/// 规则加载配置
#[derive(Debug, Clone, Deserialize)]
pub struct RulesConfig {
    /// 定时刷新间隔（秒），默认 30
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_secs: u64,

    /// 启动加载超时（秒），默认 10
    #[serde(default = "default_initial_timeout")]
    pub initial_load_timeout_secs: u64,

    /// 幂等窗口（小时），默认 24
    #[serde(default = "default_idempotency_ttl")]
    pub idempotency_ttl_hours: u64,
}

fn default_refresh_interval() -> u64 { 30 }
fn default_initial_timeout() -> u64 { 10 }
fn default_idempotency_ttl() -> u64 { 24 }

impl Default for RulesConfig {
    fn default() -> Self {
        Self {
            refresh_interval_secs: default_refresh_interval(),
            initial_load_timeout_secs: default_initial_timeout(),
            idempotency_ttl_hours: default_idempotency_ttl(),
        }
    }
}

/// Kafka Topics 配置
#[derive(Debug, Clone, Deserialize)]
pub struct KafkaTopicsConfig {
    #[serde(default = "default_engagement_events")]
    pub engagement_events: String,

    #[serde(default = "default_transaction_events")]
    pub transaction_events: String,

    #[serde(default = "default_notifications")]
    pub notifications: String,

    #[serde(default = "default_dead_letter_queue")]
    pub dead_letter_queue: String,

    #[serde(default = "default_rule_reload")]
    pub rule_reload: String,
}

fn default_engagement_events() -> String { "badge.engagement.events".into() }
fn default_transaction_events() -> String { "badge.transaction.events".into() }
fn default_notifications() -> String { "badge.notifications".into() }
fn default_dead_letter_queue() -> String { "badge.dlq".into() }
fn default_rule_reload() -> String { "badge.rule.reload".into() }

impl Default for KafkaTopicsConfig {
    fn default() -> Self {
        Self {
            engagement_events: default_engagement_events(),
            transaction_events: default_transaction_events(),
            notifications: default_notifications(),
            dead_letter_queue: default_dead_letter_queue(),
            rule_reload: default_rule_reload(),
        }
    }
}
```

**Step 2: 扩展 KafkaConfig 添加 topics 字段**

修改 `KafkaConfig` 结构：

```rust
/// Kafka 配置
#[derive(Debug, Clone, Deserialize)]
pub struct KafkaConfig {
    pub brokers: String,
    pub consumer_group: String,
    pub auto_offset_reset: String,
    #[serde(default)]
    pub topics: KafkaTopicsConfig,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            brokers: "localhost:9092".to_string(),
            consumer_group: "badge-service".to_string(),
            auto_offset_reset: "earliest".to_string(),
            topics: KafkaTopicsConfig::default(),
        }
    }
}
```

**Step 3: 扩展 AppConfig 添加 rules 字段**

修改 `AppConfig` 结构：

```rust
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
    #[serde(default)]
    pub rules: RulesConfig,
}
```

**Step 4: 添加测试**

```rust
#[test]
fn test_rules_config_defaults() {
    let config = RulesConfig::default();
    assert_eq!(config.refresh_interval_secs, 30);
    assert_eq!(config.initial_load_timeout_secs, 10);
    assert_eq!(config.idempotency_ttl_hours, 24);
}

#[test]
fn test_kafka_topics_config_defaults() {
    let config = KafkaTopicsConfig::default();
    assert_eq!(config.engagement_events, "badge.engagement.events");
    assert_eq!(config.transaction_events, "badge.transaction.events");
    assert_eq!(config.notifications, "badge.notifications");
    assert_eq!(config.dead_letter_queue, "badge.dlq");
    assert_eq!(config.rule_reload, "badge.rule.reload");
}
```

**Step 5: 运行测试验证**

Run: `cargo test -p badge-shared config`
Expected: All tests pass

**Step 6: Commit**

```bash
git add crates/shared/src/config.rs
git commit -m "feat(shared): add RulesConfig and KafkaTopicsConfig"
```

---

## Task 3: 创建 rules 模块 - 数据模型

**Files:**
- Create: `crates/shared/src/rules/mod.rs`
- Create: `crates/shared/src/rules/models.rs`
- Modify: `crates/shared/src/lib.rs`

**Step 1: 创建 rules 目录和 mod.rs**

```rust
// crates/shared/src/rules/mod.rs
//! 规则加载与校验模块
//!
//! 提供从数据库动态加载规则、内存缓存、校验等功能。

pub mod models;
pub mod mapping;
pub mod loader;
pub mod validator;

pub use models::*;
pub use mapping::RuleBadgeMapping;
pub use loader::RuleLoader;
pub use validator::RuleValidator;
```

**Step 2: 创建 models.rs**

```rust
// crates/shared/src/rules/models.rs
//! 规则相关数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 规则对应的徽章发放配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadgeGrant {
    /// 规则数据库主键
    pub rule_id: i64,
    /// 规则唯一标识符
    pub rule_code: String,
    /// 徽章 ID
    pub badge_id: i64,
    /// 徽章名称（缓存，避免反查）
    pub badge_name: String,
    /// 单次发放数量
    pub quantity: i32,
    /// 触发事件类型
    pub event_type: String,
    /// 规则生效开始时间
    pub start_time: Option<DateTime<Utc>>,
    /// 规则生效结束时间
    pub end_time: Option<DateTime<Utc>>,
    /// 每用户最大发放次数
    pub max_count_per_user: Option<i32>,
    /// 全局配额限制
    pub global_quota: Option<i32>,
    /// 已发放数量（缓存值）
    pub global_granted: i32,
}

/// 规则校验结果
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// 是否允许发放
    pub allowed: bool,
    /// 规则 ID
    pub rule_id: i64,
    /// 规则代码
    pub rule_code: String,
    /// 用户 ID
    pub user_id: String,
    /// 校验结果原因
    pub reason: ValidationReason,
    /// 校验上下文
    pub context: ValidationContext,
}

/// 校验结果原因
#[derive(Debug, Clone, Serialize)]
pub enum ValidationReason {
    /// 校验通过
    Allowed,
    /// 规则已过期
    RuleExpired { end_time: DateTime<Utc> },
    /// 规则未生效
    RuleNotStarted { start_time: DateTime<Utc> },
    /// 用户发放次数超限
    UserLimitExceeded { current: i32, max: i32 },
    /// 全局配额已用尽
    GlobalQuotaExhausted { granted: i32, quota: i32 },
}

impl ValidationReason {
    /// 返回标准化的拒绝代码
    pub fn deny_code(&self) -> &'static str {
        match self {
            Self::Allowed => "ALLOWED",
            Self::RuleExpired { .. } => "RULE_EXPIRED",
            Self::RuleNotStarted { .. } => "RULE_NOT_STARTED",
            Self::UserLimitExceeded { .. } => "USER_LIMIT_EXCEEDED",
            Self::GlobalQuotaExhausted { .. } => "GLOBAL_QUOTA_EXHAUSTED",
        }
    }

    /// 返回人类可读的拒绝信息
    pub fn message(&self) -> String {
        match self {
            Self::Allowed => "规则校验通过".to_string(),
            Self::RuleExpired { end_time } => {
                format!("规则已于 {} 过期", end_time.format("%Y-%m-%d %H:%M:%S"))
            }
            Self::RuleNotStarted { start_time } => {
                format!("规则将于 {} 生效", start_time.format("%Y-%m-%d %H:%M:%S"))
            }
            Self::UserLimitExceeded { current, max } => {
                format!("用户已获得 {}/{} 次，达到上限", current, max)
            }
            Self::GlobalQuotaExhausted { granted, quota } => {
                format!("全局配额已用尽 {}/{}", granted, quota)
            }
        }
    }

    /// 是否允许
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }
}

/// 校验上下文信息
#[derive(Debug, Clone, Default)]
pub struct ValidationContext {
    /// 校验时间
    pub checked_at: DateTime<Utc>,
    /// 用户已发放次数
    pub user_granted_count: Option<i32>,
    /// 全局已发放次数
    pub global_granted_count: Option<i32>,
}

/// 全局配额更新结果
#[derive(Debug, Clone)]
pub enum QuotaUpdateResult {
    /// 更新成功
    Success {
        rule_id: i64,
        previous: i32,
        current: i32,
        remaining: Option<i32>,
    },
    /// 配额已用尽
    QuotaExhausted {
        rule_id: i64,
        quota: i32,
        granted: i32,
        requested: i32,
    },
    /// 规则不存在
    RuleNotFound { rule_id: i64 },
}

impl QuotaUpdateResult {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }
}

/// 规则加载状态
#[derive(Debug, Clone, Default)]
pub struct LoadStatus {
    /// 是否已加载
    pub loaded: bool,
    /// 已加载规则数量
    pub rule_count: usize,
    /// 最后加载时间
    pub last_loaded_at: Option<DateTime<Utc>>,
    /// 已加载的事件类型
    pub event_types: Vec<String>,
}

/// 规则刷新事件（Kafka 消息）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleReloadEvent {
    /// 服务组（可选，None 表示全部刷新）
    pub service_group: Option<String>,
    /// 事件类型（可选，仅刷新特定事件类型）
    pub event_type: Option<String>,
    /// 触发来源
    pub trigger_source: String,
    /// 触发时间
    pub triggered_at: DateTime<Utc>,
}

/// 跳过的规则信息
#[derive(Debug, Clone, Serialize)]
pub struct SkippedRule {
    pub rule_id: i64,
    pub rule_code: String,
    pub skip_reason: ValidationReason,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_reason_deny_code() {
        assert_eq!(ValidationReason::Allowed.deny_code(), "ALLOWED");
        assert_eq!(
            ValidationReason::RuleExpired { end_time: Utc::now() }.deny_code(),
            "RULE_EXPIRED"
        );
        assert_eq!(
            ValidationReason::UserLimitExceeded { current: 3, max: 3 }.deny_code(),
            "USER_LIMIT_EXCEEDED"
        );
    }

    #[test]
    fn test_validation_reason_message() {
        let reason = ValidationReason::UserLimitExceeded { current: 3, max: 3 };
        assert!(reason.message().contains("3/3"));
    }

    #[test]
    fn test_quota_update_result_is_success() {
        let success = QuotaUpdateResult::Success {
            rule_id: 1,
            previous: 0,
            current: 1,
            remaining: Some(99),
        };
        assert!(success.is_success());

        let exhausted = QuotaUpdateResult::QuotaExhausted {
            rule_id: 1,
            quota: 100,
            granted: 100,
            requested: 1,
        };
        assert!(!exhausted.is_success());
    }
}
```

**Step 3: 在 lib.rs 中导出 rules 模块**

在 `crates/shared/src/lib.rs` 添加：

```rust
pub mod rules;
```

**Step 4: 验证编译**

Run: `cargo check -p badge-shared`
Expected: No errors

**Step 5: 运行测试**

Run: `cargo test -p badge-shared rules::models`
Expected: All tests pass

**Step 6: Commit**

```bash
git add crates/shared/src/rules crates/shared/src/lib.rs
git commit -m "feat(shared): add rules module with data models"
```

---

## Task 4: 创建 rules 模块 - RuleBadgeMapping

**Files:**
- Create: `crates/shared/src/rules/mapping.rs`

**Step 1: 创建 mapping.rs**

```rust
// crates/shared/src/rules/mapping.rs
//! 规则到徽章的内存映射
//!
//! 使用 DashMap 实现高并发读写，支持按事件类型索引规则。

use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::{DateTime, Utc};
use crossbeam_utils::atomic::AtomicCell;
use dashmap::DashMap;

use super::models::{BadgeGrant, LoadStatus};

/// 规则到徽章的映射
///
/// 按 event_type 分组存储规则，支持高并发读取。
/// 使用 DashMap 分段锁，性能优于 RwLock<HashMap>。
pub struct RuleBadgeMapping {
    /// event_type -> Vec<BadgeGrant>
    mappings: DashMap<String, Vec<BadgeGrant>>,
    /// 最后加载时间
    last_loaded_at: AtomicCell<Option<DateTime<Utc>>>,
    /// 规则总数
    rule_count: AtomicUsize,
}

impl RuleBadgeMapping {
    pub fn new() -> Self {
        Self {
            mappings: DashMap::new(),
            last_loaded_at: AtomicCell::new(None),
            rule_count: AtomicUsize::new(0),
        }
    }

    /// 根据事件类型获取所有适用规则
    pub fn get_rules_by_event_type(&self, event_type: &str) -> Vec<BadgeGrant> {
        self.mappings
            .get(event_type)
            .map(|r| r.value().clone())
            .unwrap_or_default()
    }

    /// 全量替换规则（刷新时调用）
    ///
    /// 将新规则按 event_type 分组后替换现有映射。
    /// 使用全量替换而非增量更新，确保一致性。
    pub fn replace_all(&self, rules: Vec<BadgeGrant>) {
        // 按 event_type 分组
        let mut grouped: std::collections::HashMap<String, Vec<BadgeGrant>> =
            std::collections::HashMap::new();

        for rule in &rules {
            grouped
                .entry(rule.event_type.clone())
                .or_default()
                .push(rule.clone());
        }

        // 清空现有映射
        self.mappings.clear();

        // 插入新规则
        for (event_type, rules) in grouped {
            self.mappings.insert(event_type, rules);
        }

        // 更新统计
        self.rule_count.store(rules.len(), Ordering::SeqCst);
        self.last_loaded_at.store(Some(Utc::now()));
    }

    /// 获取加载状态
    pub fn load_status(&self) -> LoadStatus {
        let event_types: Vec<String> = self
            .mappings
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        LoadStatus {
            loaded: self.last_loaded_at.load().is_some(),
            rule_count: self.rule_count.load(Ordering::SeqCst),
            last_loaded_at: self.last_loaded_at.load(),
            event_types,
        }
    }

    /// 获取规则总数
    pub fn rule_count(&self) -> usize {
        self.rule_count.load(Ordering::SeqCst)
    }

    /// 是否已加载
    pub fn is_loaded(&self) -> bool {
        self.last_loaded_at.load().is_some()
    }

    /// 获取所有事件类型
    pub fn event_types(&self) -> Vec<String> {
        self.mappings
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }
}

impl Default for RuleBadgeMapping {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_rule(rule_id: i64, event_type: &str, badge_id: i64) -> BadgeGrant {
        BadgeGrant {
            rule_id,
            rule_code: format!("rule_{}", rule_id),
            badge_id,
            badge_name: format!("Badge {}", badge_id),
            quantity: 1,
            event_type: event_type.to_string(),
            start_time: None,
            end_time: None,
            max_count_per_user: None,
            global_quota: None,
            global_granted: 0,
        }
    }

    #[test]
    fn test_empty_mapping() {
        let mapping = RuleBadgeMapping::new();
        assert!(!mapping.is_loaded());
        assert_eq!(mapping.rule_count(), 0);
        assert!(mapping.get_rules_by_event_type("purchase").is_empty());
    }

    #[test]
    fn test_replace_all() {
        let mapping = RuleBadgeMapping::new();

        let rules = vec![
            make_test_rule(1, "purchase", 100),
            make_test_rule(2, "purchase", 101),
            make_test_rule(3, "checkin", 200),
        ];

        mapping.replace_all(rules);

        assert!(mapping.is_loaded());
        assert_eq!(mapping.rule_count(), 3);

        let purchase_rules = mapping.get_rules_by_event_type("purchase");
        assert_eq!(purchase_rules.len(), 2);

        let checkin_rules = mapping.get_rules_by_event_type("checkin");
        assert_eq!(checkin_rules.len(), 1);

        let share_rules = mapping.get_rules_by_event_type("share");
        assert!(share_rules.is_empty());
    }

    #[test]
    fn test_replace_clears_old_rules() {
        let mapping = RuleBadgeMapping::new();

        // 第一次加载
        mapping.replace_all(vec![
            make_test_rule(1, "purchase", 100),
            make_test_rule(2, "checkin", 200),
        ]);
        assert_eq!(mapping.rule_count(), 2);

        // 第二次加载（只有 share）
        mapping.replace_all(vec![make_test_rule(3, "share", 300)]);
        assert_eq!(mapping.rule_count(), 1);

        // 旧规则已清除
        assert!(mapping.get_rules_by_event_type("purchase").is_empty());
        assert!(mapping.get_rules_by_event_type("checkin").is_empty());
        assert_eq!(mapping.get_rules_by_event_type("share").len(), 1);
    }

    #[test]
    fn test_load_status() {
        let mapping = RuleBadgeMapping::new();

        let status = mapping.load_status();
        assert!(!status.loaded);
        assert_eq!(status.rule_count, 0);

        mapping.replace_all(vec![
            make_test_rule(1, "purchase", 100),
            make_test_rule(2, "checkin", 200),
        ]);

        let status = mapping.load_status();
        assert!(status.loaded);
        assert_eq!(status.rule_count, 2);
        assert!(status.last_loaded_at.is_some());
        assert_eq!(status.event_types.len(), 2);
    }
}
```

**Step 2: 更新 mod.rs 导出**

确保 `mapping` 已在 `mod.rs` 中导出。

**Step 3: 添加 crossbeam-utils 依赖**

在 `crates/shared/Cargo.toml` 添加：

```toml
crossbeam-utils = "0.8"
```

**Step 4: 验证编译**

Run: `cargo check -p badge-shared`
Expected: No errors

**Step 5: 运行测试**

Run: `cargo test -p badge-shared rules::mapping`
Expected: All tests pass

**Step 6: Commit**

```bash
git add crates/shared/src/rules/mapping.rs crates/shared/Cargo.toml
git commit -m "feat(shared): add RuleBadgeMapping with concurrent access support"
```

---

## Task 5: 创建 rules 模块 - RuleValidator

**Files:**
- Create: `crates/shared/src/rules/validator.rs`

**Step 1: 创建 validator.rs**

```rust
// crates/shared/src/rules/validator.rs
//! 规则校验器
//!
//! 在发放徽章前校验规则的各项限制条件。

use std::time::Instant;

use chrono::Utc;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::cache::Cache;
use crate::error::BadgeError;

use super::models::{BadgeGrant, ValidationContext, ValidationReason, ValidationResult};

/// 规则校验器
pub struct RuleValidator {
    cache: Cache,
    db_pool: PgPool,
}

impl RuleValidator {
    pub fn new(cache: Cache, db_pool: PgPool) -> Self {
        Self { cache, db_pool }
    }

    /// 综合校验规则是否允许发放
    ///
    /// 校验顺序：
    /// 1. 时间有效性（start_time, end_time）
    /// 2. 用户发放次数限制（max_count_per_user）
    /// 3. 全局配额限制（global_quota）
    pub async fn can_grant(
        &self,
        rule: &BadgeGrant,
        user_id: &str,
    ) -> Result<ValidationResult, BadgeError> {
        let start = Instant::now();
        let now = Utc::now();
        let mut context = ValidationContext {
            checked_at: now,
            ..Default::default()
        };

        // 1. 检查规则是否已过期
        if let Some(end_time) = rule.end_time {
            if now > end_time {
                let result = self.build_result(rule, user_id, ValidationReason::RuleExpired { end_time }, context);
                self.log_validation(&result, start.elapsed().as_millis() as u64);
                return Ok(result);
            }
        }

        // 2. 检查规则是否未生效
        if let Some(start_time) = rule.start_time {
            if now < start_time {
                let result = self.build_result(rule, user_id, ValidationReason::RuleNotStarted { start_time }, context);
                self.log_validation(&result, start.elapsed().as_millis() as u64);
                return Ok(result);
            }
        }

        // 3. 检查用户发放次数限制
        if let Some(max_count) = rule.max_count_per_user {
            let user_count = self.get_user_grant_count(user_id, rule.badge_id).await?;
            context.user_granted_count = Some(user_count);

            if user_count >= max_count {
                let result = self.build_result(
                    rule,
                    user_id,
                    ValidationReason::UserLimitExceeded {
                        current: user_count,
                        max: max_count,
                    },
                    context,
                );
                self.log_validation(&result, start.elapsed().as_millis() as u64);
                return Ok(result);
            }
        }

        // 4. 检查全局配额
        if let Some(quota) = rule.global_quota {
            context.global_granted_count = Some(rule.global_granted);

            if rule.global_granted >= quota {
                let result = self.build_result(
                    rule,
                    user_id,
                    ValidationReason::GlobalQuotaExhausted {
                        granted: rule.global_granted,
                        quota,
                    },
                    context,
                );
                self.log_validation(&result, start.elapsed().as_millis() as u64);
                return Ok(result);
            }
        }

        // 所有校验通过
        let result = self.build_result(rule, user_id, ValidationReason::Allowed, context);
        self.log_validation(&result, start.elapsed().as_millis() as u64);
        Ok(result)
    }

    /// 查询用户对某徽章的已发放次数
    async fn get_user_grant_count(&self, user_id: &str, badge_id: i64) -> Result<i32, BadgeError> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM user_badge_logs
            WHERE user_id = $1 AND badge_id = $2 AND action = 'grant'
            "#,
        )
        .bind(user_id)
        .bind(badge_id)
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| BadgeError::Database(e.to_string()))?;

        Ok(count.0 as i32)
    }

    fn build_result(
        &self,
        rule: &BadgeGrant,
        user_id: &str,
        reason: ValidationReason,
        context: ValidationContext,
    ) -> ValidationResult {
        ValidationResult {
            allowed: reason.is_allowed(),
            rule_id: rule.rule_id,
            rule_code: rule.rule_code.clone(),
            user_id: user_id.to_string(),
            reason,
            context,
        }
    }

    fn log_validation(&self, result: &ValidationResult, elapsed_ms: u64) {
        if result.allowed {
            info!(
                rule_id = result.rule_id,
                rule_code = %result.rule_code,
                user_id = %result.user_id,
                validation_ms = elapsed_ms,
                "规则校验通过"
            );
        } else {
            warn!(
                rule_id = result.rule_id,
                rule_code = %result.rule_code,
                user_id = %result.user_id,
                deny_code = result.reason.deny_code(),
                deny_message = %result.reason.message(),
                user_granted_count = ?result.context.user_granted_count,
                global_granted_count = ?result.context.global_granted_count,
                validation_ms = elapsed_ms,
                "规则校验未通过"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    // 集成测试需要数据库连接，暂时跳过
    // 将在 Task 9 中添加完整的集成测试
}
```

**Step 2: 验证编译**

Run: `cargo check -p badge-shared`
Expected: No errors

**Step 3: Commit**

```bash
git add crates/shared/src/rules/validator.rs
git commit -m "feat(shared): add RuleValidator for rule validation"
```

---

## Task 6: 创建 rules 模块 - RuleLoader

**Files:**
- Create: `crates/shared/src/rules/loader.rs`

**Step 1: 创建 loader.rs**

```rust
// crates/shared/src/rules/loader.rs
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

    /// 从数据库加载规则
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
        .await
        .map_err(|e| BadgeError::Database(e.to_string()))?;

        let rules: Vec<BadgeGrant> = rows
            .into_iter()
            .filter_map(|row| {
                // rule_code 为空时使用 rule_id 生成
                let rule_code = row.rule_code.unwrap_or_else(|| format!("rule_{}", row.rule_id));

                Some(BadgeGrant {
                    rule_id: row.rule_id,
                    rule_code,
                    badge_id: row.badge_id,
                    badge_name: row.badge_name,
                    quantity: 1, // 默认发放 1 个
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

#[cfg(test)]
mod tests {
    // 集成测试需要数据库连接，暂时跳过
    // 将在 Task 9 中添加完整的集成测试
}
```

**Step 2: 验证编译**

Run: `cargo check -p badge-shared`
Expected: No errors

**Step 3: Commit**

```bash
git add crates/shared/src/rules/loader.rs
git commit -m "feat(shared): add RuleLoader for database rule loading"
```

---

## Task 7: 更新配置文件

**Files:**
- Modify: `config/default.toml`
- Modify: `config/event-engagement-service.toml`
- Modify: `config/event-transaction-service.toml`

**Step 1: 更新 default.toml**

在 `config/default.toml` 添加：

```toml
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

**Step 2: 更新 event-engagement-service.toml**

```toml
[server]
port = 50053

[rules]
refresh_interval_secs = 30
initial_load_timeout_secs = 10
idempotency_ttl_hours = 24
```

**Step 3: 更新 event-transaction-service.toml**

```toml
[server]
port = 50054

[rules]
refresh_interval_secs = 30
initial_load_timeout_secs = 10
idempotency_ttl_hours = 24
```

**Step 4: Commit**

```bash
git add config/
git commit -m "feat(config): add rules and kafka topics configuration"
```

---

## Task 8: 改造 event-engagement-service

**Files:**
- Modify: `crates/event-engagement-service/Cargo.toml`
- Modify: `crates/event-engagement-service/src/main.rs`
- Modify: `crates/event-engagement-service/src/processor.rs`
- Delete: `crates/event-engagement-service/src/rule_mapping.rs`

**Step 1: 更新 Cargo.toml 添加 sqlx 依赖**

```toml
[dependencies]
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "chrono"] }
```

**Step 2: 改造 main.rs**

移除硬编码的 `add_mapping` 调用，使用 `RuleLoader` 从数据库加载：

```rust
// 关键改动部分
use badge_shared::database::Database;
use badge_shared::rules::{RuleBadgeMapping, RuleLoader, RuleValidator};

#[tokio::main]
async fn main() -> Result<()> {
    // ... 现有初始化代码 ...

    // 初始化数据库连接
    let db = Database::connect(&config.database).await?;
    let db_pool = db.pool().clone();

    // 初始化规则组件
    let rule_mapping = Arc::new(RuleBadgeMapping::new());
    let rule_loader = Arc::new(RuleLoader::new(
        db_pool.clone(),
        "engagement",
        rule_mapping.clone(),
        config.rules.refresh_interval_secs,
        config.rules.initial_load_timeout_secs,
    ));
    let rule_validator = Arc::new(RuleValidator::new(cache.clone(), db_pool.clone()));

    // 初始加载规则（阻塞，失败则终止启动）
    rule_loader.initial_load().await?;

    // 启动后台刷新任务
    rule_loader.clone().start_background_refresh(shutdown_rx.clone());

    // 创建处理器（使用新的规则组件）
    let processor = EngagementEventProcessor::new(
        cache,
        Arc::new(rule_client),
        rule_mapping,
        rule_validator,
    );

    // ... 其余启动代码 ...
}
```

**Step 3: 改造 processor.rs**

更新处理器使用 `RuleValidator`：

```rust
pub struct EngagementEventProcessor {
    cache: Cache,
    rule_client: Arc<dyn BadgeRuleService>,
    rule_mapping: Arc<RuleBadgeMapping>,
    rule_validator: Arc<RuleValidator>,
}

impl EngagementEventProcessor {
    pub fn new(
        cache: Cache,
        rule_client: Arc<dyn BadgeRuleService>,
        rule_mapping: Arc<RuleBadgeMapping>,
        rule_validator: Arc<RuleValidator>,
    ) -> Self {
        Self {
            cache,
            rule_client,
            rule_mapping,
            rule_validator,
        }
    }
}

// 在 process 方法中：
// 1. 使用 rule_mapping.get_rules_by_event_type() 获取规则
// 2. 对每条规则调用 rule_validator.can_grant() 校验
// 3. 校验通过才调用规则引擎评估和徽章发放
```

**Step 4: 删除旧的 rule_mapping.rs**

移除 `crates/event-engagement-service/src/rule_mapping.rs`

**Step 5: 验证编译**

Run: `cargo check -p event-engagement-service`
Expected: No errors

**Step 6: Commit**

```bash
git add crates/event-engagement-service/
git commit -m "feat(engagement): use RuleLoader for dynamic rule loading"
```

---

## Task 9: 改造 event-transaction-service

**Files:**
- Modify: `crates/event-transaction-service/Cargo.toml`
- Modify: `crates/event-transaction-service/src/main.rs`
- Modify: `crates/event-transaction-service/src/processor.rs`
- Delete: `crates/event-transaction-service/src/rule_mapping.rs`

**Step 1-6:** 与 Task 8 类似，使用 `"transaction"` 作为 service_group

**Commit:**

```bash
git add crates/event-transaction-service/
git commit -m "feat(transaction): use RuleLoader for dynamic rule loading"
```

---

## Task 10: 实现 Kafka 规则刷新消费

**Files:**
- Modify: `crates/event-engagement-service/src/consumer.rs`
- Modify: `crates/event-transaction-service/src/consumer.rs`

**Step 1: 添加规则刷新消费逻辑**

在两个服务的 consumer 中：
1. 订阅 `rule_reload` topic
2. 收到刷新消息时调用 `rule_loader.reload_now()`

```rust
// 在消费循环中添加对 rule_reload topic 的处理
if msg.topic == config.kafka.topics.rule_reload {
    if let Ok(event) = serde_json::from_slice::<RuleReloadEvent>(&msg.payload) {
        // 检查是否需要刷新（service_group 匹配或为空）
        if event.service_group.is_none()
            || event.service_group.as_deref() == Some("engagement") {
            if let Err(e) = rule_loader.reload_now().await {
                warn!(error = %e, "Kafka 触发规则刷新失败");
            }
        }
    }
}
```

**Step 2: 更新订阅列表**

```rust
consumer.subscribe(&[
    &config.kafka.topics.engagement_events,
    &config.kafka.topics.rule_reload,
])?;
```

**Step 3: 验证编译**

Run: `cargo check -p event-engagement-service -p event-transaction-service`
Expected: No errors

**Step 4: Commit**

```bash
git add crates/event-engagement-service/src/consumer.rs
git add crates/event-transaction-service/src/consumer.rs
git commit -m "feat(events): add Kafka rule reload subscription"
```

---

## Task 11: 单元测试与集成测试

**Files:**
- Modify: `crates/shared/src/rules/validator.rs` (添加测试)
- Modify: `crates/shared/src/rules/loader.rs` (添加测试)
- Create: `crates/badge-management-service/examples/rule_loading_test.rs`

**Step 1: 添加 validator 集成测试**

使用 sqlx test fixtures 或 mock。

**Step 2: 添加 loader 集成测试**

**Step 3: 创建端到端测试脚本**

```rust
// examples/rule_loading_test.rs
// 测试从数据库加载规则 -> 事件触发 -> 规则校验 -> 徽章发放
```

**Step 4: 运行所有测试**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 5: Commit**

```bash
git add .
git commit -m "test: add unit and integration tests for rule loading"
```

---

## Task 12: 全链路测试

**Files:**
- Modify: `crates/badge-management-service/examples/full_pipeline_test.rs`

**Step 1: 更新全链路测试**

1. 确保数据库有规则配置
2. 启动所有服务
3. 通过 mock-services 发送事件
4. 验证徽章发放

**Step 2: 运行全链路测试**

Run: `cargo run --example full_pipeline_test -p badge-management-service`
Expected: All scenarios pass

**Step 3: Final Commit**

```bash
git add .
git commit -m "test: update full pipeline test for dynamic rules"
```

---

## 验收标准

- [ ] 数据库迁移成功执行
- [ ] event-engagement-service 从数据库加载规则
- [ ] event-transaction-service 从数据库加载规则
- [ ] 定时刷新正常工作（30 秒间隔）
- [ ] Kafka 即时刷新正常工作
- [ ] 规则时间有效性校验生效
- [ ] 用户发放次数限制生效
- [ ] 全局配额限制生效
- [ ] 服务启动预热成功（规则加载失败则启动失败）
- [ ] 幂等窗口可配置
- [ ] 全链路测试通过
