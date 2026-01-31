# 徽章平台优化实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** 实现规则引擎模板化、权益系统 Trait 抽象、统一可观测性三大重构。

**Architecture:** 增量式重构，保持向后兼容，每个 Phase 独立可验证。

**Tech Stack:** Rust 2024, SQLx, async-trait, metrics, opentelemetry, tracing

---

## Phase 1: 规则引擎重构（模板参数化）

### Task 1.1: 数据库迁移 - rule_templates 表

**Files:**
- Create: `migrations/20250131_003_rule_templates.sql`

**实现要点:**

```sql
-- 规则模板表
CREATE TABLE rule_templates (
    id BIGSERIAL PRIMARY KEY,
    code VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    category VARCHAR(50) NOT NULL,
    subcategory VARCHAR(50),
    template_json JSONB NOT NULL,
    parameters JSONB NOT NULL DEFAULT '[]',
    version VARCHAR(20) NOT NULL DEFAULT '1.0',
    is_system BOOLEAN NOT NULL DEFAULT FALSE,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_rule_templates_category ON rule_templates(category, subcategory);
CREATE INDEX idx_rule_templates_code ON rule_templates(code);
CREATE INDEX idx_rule_templates_enabled ON rule_templates(enabled) WHERE enabled = TRUE;

-- 扩展 badge_rules 表
ALTER TABLE badge_rules ADD COLUMN IF NOT EXISTS template_id BIGINT REFERENCES rule_templates(id);
ALTER TABLE badge_rules ADD COLUMN IF NOT EXISTS template_version VARCHAR(20);
ALTER TABLE badge_rules ADD COLUMN IF NOT EXISTS template_params JSONB DEFAULT '{}';

CREATE INDEX idx_badge_rules_template ON badge_rules(template_id) WHERE template_id IS NOT NULL;
```

**验证:**
```bash
make db-migrate
podman exec badge-postgres psql -U badge -d badge_db -c "\d rule_templates"
```

---

### Task 1.2: 模板参数模型定义

**Files:**
- Create: `crates/unified-rule-engine/src/template/mod.rs`
- Create: `crates/unified-rule-engine/src/template/models.rs`
- Modify: `crates/unified-rule-engine/src/lib.rs`

**实现要点:**

```rust
// template/models.rs
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 模板参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterDef {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: ParameterType,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default: Option<Value>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub options: Option<Vec<ParameterOption>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Date,
    Array,
    Enum,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterOption {
    pub value: Value,
    pub label: String,
}

/// 规则模板
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleTemplate {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub category: TemplateCategory,
    pub subcategory: Option<String>,
    pub template_json: Value,
    pub parameters: Vec<ParameterDef>,
    pub version: String,
    pub is_system: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum TemplateCategory {
    Basic,
    Advanced,
    Industry,
}
```

**验证:**
```bash
cargo build -p unified-rule-engine
cargo test -p unified-rule-engine template::
```

---

### Task 1.3: 模板编译器实现

**Files:**
- Create: `crates/unified-rule-engine/src/template/compiler.rs`

**实现要点:**

```rust
// template/compiler.rs
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

use crate::error::RuleError;
use super::models::{ParameterDef, RuleTemplate};

pub struct TemplateCompiler {
    placeholder_regex: Regex,
}

impl TemplateCompiler {
    pub fn new() -> Self {
        Self {
            // 匹配 ${paramName} 格式的占位符
            placeholder_regex: Regex::new(r"\$\{([a-zA-Z_][a-zA-Z0-9_]*)\}").unwrap(),
        }
    }

    /// 从模板和参数编译出完整规则 JSON
    pub fn compile(
        &self,
        template: &RuleTemplate,
        params: &HashMap<String, Value>,
    ) -> Result<Value, RuleError> {
        // 1. 验证必填参数
        self.validate_params(&template.parameters, params)?;

        // 2. 合并默认值
        let merged_params = self.merge_with_defaults(&template.parameters, params);

        // 3. 替换占位符
        let compiled = self.replace_placeholders(&template.template_json, &merged_params)?;

        Ok(compiled)
    }

    fn validate_params(
        &self,
        definitions: &[ParameterDef],
        params: &HashMap<String, Value>,
    ) -> Result<(), RuleError> {
        for def in definitions {
            if def.required && !params.contains_key(&def.name) && def.default.is_none() {
                return Err(RuleError::MissingParameter(def.name.clone()));
            }

            if let Some(value) = params.get(&def.name) {
                self.validate_param_value(def, value)?;
            }
        }
        Ok(())
    }

    fn validate_param_value(&self, def: &ParameterDef, value: &Value) -> Result<(), RuleError> {
        // 数值范围验证
        if let Some(min) = def.min {
            if let Some(v) = value.as_f64() {
                if v < min {
                    return Err(RuleError::ParamOutOfRange {
                        name: def.name.clone(),
                        min: Some(min),
                        max: def.max,
                    });
                }
            }
        }
        if let Some(max) = def.max {
            if let Some(v) = value.as_f64() {
                if v > max {
                    return Err(RuleError::ParamOutOfRange {
                        name: def.name.clone(),
                        min: def.min,
                        max: Some(max),
                    });
                }
            }
        }
        Ok(())
    }

    fn merge_with_defaults(
        &self,
        definitions: &[ParameterDef],
        params: &HashMap<String, Value>,
    ) -> HashMap<String, Value> {
        let mut merged = params.clone();
        for def in definitions {
            if !merged.contains_key(&def.name) {
                if let Some(default) = &def.default {
                    merged.insert(def.name.clone(), default.clone());
                }
            }
        }
        merged
    }

    fn replace_placeholders(
        &self,
        template: &Value,
        params: &HashMap<String, Value>,
    ) -> Result<Value, RuleError> {
        match template {
            Value::String(s) => {
                // 检查是否是纯占位符（如 "${amount}"）
                if let Some(caps) = self.placeholder_regex.captures(s) {
                    if caps.get(0).map(|m| m.as_str()) == Some(s.as_str()) {
                        // 整个字符串就是占位符，直接返回参数值
                        let param_name = &caps[1];
                        return params.get(param_name)
                            .cloned()
                            .ok_or_else(|| RuleError::MissingParameter(param_name.to_string()));
                    }
                }
                // 字符串中包含占位符，进行字符串替换
                let result = self.placeholder_regex.replace_all(s, |caps: &regex::Captures| {
                    let param_name = &caps[1];
                    params.get(param_name)
                        .map(|v| match v {
                            Value::String(s) => s.clone(),
                            _ => v.to_string(),
                        })
                        .unwrap_or_else(|| caps[0].to_string())
                });
                Ok(Value::String(result.into_owned()))
            }
            Value::Array(arr) => {
                let compiled: Result<Vec<Value>, _> = arr.iter()
                    .map(|v| self.replace_placeholders(v, params))
                    .collect();
                Ok(Value::Array(compiled?))
            }
            Value::Object(obj) => {
                let mut compiled = serde_json::Map::new();
                for (k, v) in obj {
                    compiled.insert(k.clone(), self.replace_placeholders(v, params)?);
                }
                Ok(Value::Object(compiled))
            }
            _ => Ok(template.clone()),
        }
    }
}

impl Default for TemplateCompiler {
    fn default() -> Self {
        Self::new()
    }
}
```

**验证:**
```bash
cargo test -p unified-rule-engine template::compiler::
```

---

### Task 1.4: 模板仓储层

**Files:**
- Create: `crates/unified-rule-engine/src/template/repository.rs`

**实现要点:**

```rust
// template/repository.rs
use sqlx::PgPool;
use super::models::{RuleTemplate, TemplateCategory};

pub struct TemplateRepository {
    pool: PgPool,
}

impl TemplateRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_by_category(
        &self,
        category: Option<TemplateCategory>,
        enabled_only: bool,
    ) -> Result<Vec<RuleTemplate>, sqlx::Error> {
        let mut query = String::from(
            "SELECT id, code, name, description, category, subcategory,
                    template_json, parameters, version, is_system, enabled
             FROM rule_templates WHERE 1=1"
        );

        if enabled_only {
            query.push_str(" AND enabled = TRUE");
        }
        if category.is_some() {
            query.push_str(" AND category = $1");
        }
        query.push_str(" ORDER BY category, subcategory, name");

        // 使用 sqlx::query_as 执行
        // ...实现细节...
        todo!()
    }

    pub async fn get_by_code(&self, code: &str) -> Result<Option<RuleTemplate>, sqlx::Error> {
        sqlx::query_as!(
            RuleTemplate,
            r#"SELECT id, code, name, description,
                      category as "category: TemplateCategory",
                      subcategory, template_json, parameters,
                      version, is_system, enabled
               FROM rule_templates WHERE code = $1"#,
            code
        )
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create(&self, template: &RuleTemplate) -> Result<i64, sqlx::Error> {
        let row = sqlx::query!(
            r#"INSERT INTO rule_templates
               (code, name, description, category, subcategory,
                template_json, parameters, version, is_system, enabled)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
               RETURNING id"#,
            template.code,
            template.name,
            template.description,
            template.category.to_string(),
            template.subcategory,
            template.template_json,
            serde_json::to_value(&template.parameters).unwrap(),
            template.version,
            template.is_system,
            template.enabled
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.id)
    }
}
```

**验证:**
```bash
cargo build -p unified-rule-engine
```

---

### Task 1.5: 预置模板数据

**Files:**
- Create: `migrations/20250131_004_seed_templates.sql`

**实现要点:**

```sql
-- 基础场景模板
INSERT INTO rule_templates (code, name, description, category, subcategory, template_json, parameters, is_system) VALUES
-- 首次事件
('first_event', '首次事件触发', '用户首次完成指定事件时触发', 'basic', 'event',
 '{"root": {"type": "condition", "field": "event.type", "operator": "eq", "value": "${event_type}"}}',
 '[{"name": "event_type", "type": "string", "label": "事件类型", "required": true, "options": [
    {"value": "checkin", "label": "签到"},
    {"value": "purchase", "label": "购买"},
    {"value": "share", "label": "分享"},
    {"value": "review", "label": "评价"}
 ]}]',
 true),

-- 累计金额
('cumulative_amount', '累计金额达标', '用户累计消费达到指定金额时触发', 'basic', 'transaction',
 '{"root": {"type": "group", "operator": "AND", "children": [
    {"type": "condition", "field": "event.type", "operator": "eq", "value": "purchase"},
    {"type": "condition", "field": "user.total_amount", "operator": "gte", "value": "${amount}"}
 ]}}',
 '[{"name": "amount", "type": "number", "label": "金额阈值", "required": true, "min": 0, "default": 100}]',
 true),

-- 累计次数
('cumulative_count', '累计次数达标', '用户完成指定事件达到指定次数时触发', 'basic', 'event',
 '{"root": {"type": "group", "operator": "AND", "children": [
    {"type": "condition", "field": "event.type", "operator": "eq", "value": "${event_type}"},
    {"type": "condition", "field": "user.event_count", "operator": "gte", "value": "${count}"}
 ]}}',
 '[{"name": "event_type", "type": "string", "label": "事件类型", "required": true},
   {"name": "count", "type": "number", "label": "次数阈值", "required": true, "min": 1, "default": 5}]',
 true),

-- 用户等级
('user_level_gte', '用户等级达标', '用户等级达到指定级别时触发', 'basic', 'user',
 '{"root": {"type": "condition", "field": "user.level", "operator": "gte", "value": "${level}"}}',
 '[{"name": "level", "type": "number", "label": "等级阈值", "required": true, "min": 1, "default": 5}]',
 true),

-- 标签匹配
('tag_match', '用户标签匹配', '用户拥有指定标签时触发', 'basic', 'user',
 '{"root": {"type": "condition", "field": "user.tags", "operator": "contains_any", "value": "${tags}"}}',
 '[{"name": "tags", "type": "array", "label": "标签列表", "required": true}]',
 true)
ON CONFLICT (code) DO NOTHING;

-- 高级场景模板
INSERT INTO rule_templates (code, name, description, category, subcategory, template_json, parameters, is_system) VALUES
-- 时间窗口
('time_window_event', '时间窗口内事件', '在指定时间范围内完成事件时触发', 'advanced', 'time',
 '{"root": {"type": "group", "operator": "AND", "children": [
    {"type": "condition", "field": "event.type", "operator": "eq", "value": "${event_type}"},
    {"type": "condition", "field": "event.timestamp", "operator": "after", "value": "${start_time}"},
    {"type": "condition", "field": "event.timestamp", "operator": "before", "value": "${end_time}"}
 ]}}',
 '[{"name": "event_type", "type": "string", "label": "事件类型", "required": true},
   {"name": "start_time", "type": "date", "label": "开始时间", "required": true},
   {"name": "end_time", "type": "date", "label": "结束时间", "required": true}]',
 true),

-- 连续天数
('streak_days', '连续签到天数', '用户连续签到达到指定天数时触发', 'advanced', 'streak',
 '{"root": {"type": "group", "operator": "AND", "children": [
    {"type": "condition", "field": "event.type", "operator": "eq", "value": "checkin"},
    {"type": "condition", "field": "user.streak_days", "operator": "gte", "value": "${days}"}
 ]}}',
 '[{"name": "days", "type": "number", "label": "连续天数", "required": true, "min": 1, "default": 7}]',
 true),

-- 频次限制
('frequency_limit', '频次限制事件', '用户在指定周期内完成事件达到指定次数时触发', 'advanced', 'frequency',
 '{"root": {"type": "group", "operator": "AND", "children": [
    {"type": "condition", "field": "event.type", "operator": "eq", "value": "${event_type}"},
    {"type": "condition", "field": "user.period_count", "operator": "gte", "value": "${count}"}
 ]}}',
 '[{"name": "event_type", "type": "string", "label": "事件类型", "required": true},
   {"name": "count", "type": "number", "label": "次数阈值", "required": true, "min": 1},
   {"name": "period", "type": "enum", "label": "周期", "required": true, "default": "daily",
    "options": [{"value": "daily", "label": "每日"}, {"value": "weekly", "label": "每周"}, {"value": "monthly", "label": "每月"}]}]',
 true)
ON CONFLICT (code) DO NOTHING;

-- 行业模板：电商
INSERT INTO rule_templates (code, name, description, category, subcategory, template_json, parameters, is_system) VALUES
('ecom_first_purchase', '电商首次购买', '用户完成首次购买时触发', 'industry', 'e-commerce',
 '{"root": {"type": "group", "operator": "AND", "children": [
    {"type": "condition", "field": "event.type", "operator": "eq", "value": "purchase"},
    {"type": "condition", "field": "user.purchase_count", "operator": "eq", "value": 1}
 ]}}',
 '[]',
 true),

('ecom_order_amount', '电商订单金额', '单笔订单金额达到指定值时触发', 'industry', 'e-commerce',
 '{"root": {"type": "group", "operator": "AND", "children": [
    {"type": "condition", "field": "event.type", "operator": "eq", "value": "purchase"},
    {"type": "condition", "field": "order.amount", "operator": "gte", "value": "${amount}"}
 ]}}',
 '[{"name": "amount", "type": "number", "label": "订单金额阈值", "required": true, "min": 0, "default": 100}]',
 true),

('ecom_repeat_purchase', '电商复购', '用户在指定天数内再次购买时触发', 'industry', 'e-commerce',
 '{"root": {"type": "group", "operator": "AND", "children": [
    {"type": "condition", "field": "event.type", "operator": "eq", "value": "purchase"},
    {"type": "condition", "field": "user.purchase_count", "operator": "gte", "value": "${count}"},
    {"type": "condition", "field": "user.days_since_last_purchase", "operator": "lte", "value": "${days}"}
 ]}}',
 '[{"name": "count", "type": "number", "label": "购买次数", "required": true, "min": 2, "default": 2},
   {"name": "days", "type": "number", "label": "天数限制", "required": true, "min": 1, "default": 30}]',
 true)
ON CONFLICT (code) DO NOTHING;

-- 行业模板：游戏
INSERT INTO rule_templates (code, name, description, category, subcategory, template_json, parameters, is_system) VALUES
('game_level_reached', '游戏等级达成', '玩家等级达到指定值时触发', 'industry', 'gaming',
 '{"root": {"type": "condition", "field": "user.game_level", "operator": "gte", "value": "${level}"}}',
 '[{"name": "level", "type": "number", "label": "等级", "required": true, "min": 1, "default": 10}]',
 true),

('game_achievement', '游戏成就解锁', '玩家解锁指定成就时触发', 'industry', 'gaming',
 '{"root": {"type": "group", "operator": "AND", "children": [
    {"type": "condition", "field": "event.type", "operator": "eq", "value": "achievement_unlock"},
    {"type": "condition", "field": "event.achievement_id", "operator": "eq", "value": "${achievement_id}"}
 ]}}',
 '[{"name": "achievement_id", "type": "string", "label": "成就ID", "required": true}]',
 true)
ON CONFLICT (code) DO NOTHING;

-- 行业模板：O2O
INSERT INTO rule_templates (code, name, description, category, subcategory, template_json, parameters, is_system) VALUES
('o2o_store_visit', 'O2O到店', '用户到店打卡时触发', 'industry', 'o2o',
 '{"root": {"type": "group", "operator": "AND", "children": [
    {"type": "condition", "field": "event.type", "operator": "eq", "value": "store_visit"},
    {"type": "condition", "field": "event.store_id", "operator": "eq", "value": "${store_id}"}
 ]}}',
 '[{"name": "store_id", "type": "string", "label": "门店ID", "required": false, "description": "留空表示任意门店"}]',
 true),

('o2o_review', 'O2O评价', '用户发表评价时触发', 'industry', 'o2o',
 '{"root": {"type": "group", "operator": "AND", "children": [
    {"type": "condition", "field": "event.type", "operator": "eq", "value": "review"},
    {"type": "condition", "field": "event.rating", "operator": "gte", "value": "${min_rating}"}
 ]}}',
 '[{"name": "min_rating", "type": "number", "label": "最低评分", "required": false, "min": 1, "max": 5, "default": 4}]',
 true)
ON CONFLICT (code) DO NOTHING;
```

**验证:**
```bash
make db-migrate
podman exec badge-postgres psql -U badge -d badge_db -c "SELECT code, name, category FROM rule_templates"
```

---

### Task 1.6: 管理后台 API - 模板 CRUD

**Files:**
- Create: `crates/badge-admin-service/src/handlers/template.rs`
- Modify: `crates/badge-admin-service/src/handlers/mod.rs`
- Modify: `crates/badge-admin-service/src/routes.rs`

**实现要点:**

```rust
// handlers/template.rs
use axum::{extract::{Path, Query, State}, Json};
use crate::{AppState, dto::*};

/// 获取模板列表
pub async fn list_templates(
    State(state): State<AppState>,
    Query(params): Query<TemplateListParams>,
) -> Result<Json<ListResponse<TemplateDto>>, AppError> {
    let templates = state.template_repo
        .list_by_category(params.category, params.enabled_only.unwrap_or(true))
        .await?;

    Ok(Json(ListResponse::new(templates.into_iter().map(Into::into).collect())))
}

/// 获取模板详情
pub async fn get_template(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<TemplateDto>, AppError> {
    let template = state.template_repo
        .get_by_code(&code)
        .await?
        .ok_or(AppError::NotFound("模板不存在".into()))?;

    Ok(Json(template.into()))
}

/// 预览模板编译结果
pub async fn preview_template(
    State(state): State<AppState>,
    Path(code): Path<String>,
    Json(params): Json<HashMap<String, Value>>,
) -> Result<Json<PreviewResult>, AppError> {
    let template = state.template_repo
        .get_by_code(&code)
        .await?
        .ok_or(AppError::NotFound("模板不存在".into()))?;

    let compiler = TemplateCompiler::new();
    let compiled = compiler.compile(&template, &params)?;

    Ok(Json(PreviewResult { rule_json: compiled }))
}

/// 从模板创建规则
pub async fn create_rule_from_template(
    State(state): State<AppState>,
    Json(req): Json<CreateRuleFromTemplateRequest>,
) -> Result<Json<RuleDto>, AppError> {
    // 1. 获取模板
    let template = state.template_repo
        .get_by_code(&req.template_code)
        .await?
        .ok_or(AppError::NotFound("模板不存在".into()))?;

    // 2. 编译规则
    let compiler = TemplateCompiler::new();
    let rule_json = compiler.compile(&template, &req.params)?;

    // 3. 创建规则记录
    let rule = state.rule_repo.create(CreateRuleInput {
        badge_id: req.badge_id,
        template_id: Some(template.id),
        template_version: Some(template.version.clone()),
        template_params: Some(serde_json::to_value(&req.params)?),
        rule_json,
        ..Default::default()
    }).await?;

    Ok(Json(rule.into()))
}
```

**路由配置:**
```rust
// routes.rs 添加
.route("/api/admin/templates", get(handlers::template::list_templates))
.route("/api/admin/templates/:code", get(handlers::template::get_template))
.route("/api/admin/templates/:code/preview", post(handlers::template::preview_template))
.route("/api/admin/rules/from-template", post(handlers::template::create_rule_from_template))
```

**验证:**
```bash
cargo build -p badge-admin-service
curl http://localhost:8080/api/admin/templates | jq
```

---

### Task 1.7: 前端模板选择器组件

**Files:**
- Create: `web/admin-ui/src/pages/rules/components/TemplateSelector.tsx`
- Create: `web/admin-ui/src/pages/rules/components/ParameterForm.tsx`
- Create: `web/admin-ui/src/services/template.ts`

**实现要点:**

```typescript
// services/template.ts
export interface RuleTemplate {
  id: number;
  code: string;
  name: string;
  description?: string;
  category: 'basic' | 'advanced' | 'industry';
  subcategory?: string;
  templateJson: Record<string, unknown>;
  parameters: ParameterDef[];
  version: string;
  isSystem: boolean;
}

export interface ParameterDef {
  name: string;
  type: 'string' | 'number' | 'boolean' | 'date' | 'array' | 'enum';
  label: string;
  description?: string;
  default?: unknown;
  required: boolean;
  min?: number;
  max?: number;
  options?: { value: unknown; label: string }[];
}

export async function listTemplates(category?: string): Promise<RuleTemplate[]> {
  const params = category ? `?category=${category}` : '';
  const res = await fetch(`/api/admin/templates${params}`);
  const data = await res.json();
  return data.items;
}

export async function previewTemplate(
  code: string,
  params: Record<string, unknown>
): Promise<{ ruleJson: Record<string, unknown> }> {
  const res = await fetch(`/api/admin/templates/${code}/preview`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
  return res.json();
}
```

```tsx
// components/TemplateSelector.tsx
import { useState, useEffect } from 'react';
import { Card, Tabs, List, Tag, Button } from 'antd';
import { listTemplates, RuleTemplate } from '@/services/template';

interface Props {
  onSelect: (template: RuleTemplate) => void;
}

export function TemplateSelector({ onSelect }: Props) {
  const [templates, setTemplates] = useState<RuleTemplate[]>([]);
  const [category, setCategory] = useState<string>('basic');

  useEffect(() => {
    listTemplates(category).then(setTemplates);
  }, [category]);

  const categoryLabels = {
    basic: '基础场景',
    advanced: '高级场景',
    industry: '行业模板',
  };

  return (
    <Card title="选择规则模板">
      <Tabs
        activeKey={category}
        onChange={setCategory}
        items={Object.entries(categoryLabels).map(([key, label]) => ({
          key,
          label,
        }))}
      />
      <List
        dataSource={templates}
        renderItem={(template) => (
          <List.Item
            actions={[
              <Button type="primary" onClick={() => onSelect(template)}>
                使用
              </Button>,
            ]}
          >
            <List.Item.Meta
              title={template.name}
              description={template.description}
            />
            {template.subcategory && <Tag>{template.subcategory}</Tag>}
          </List.Item>
        )}
      />
    </Card>
  );
}
```

```tsx
// components/ParameterForm.tsx
import { Form, Input, InputNumber, Switch, Select, DatePicker } from 'antd';
import { ParameterDef } from '@/services/template';

interface Props {
  parameters: ParameterDef[];
  onChange: (values: Record<string, unknown>) => void;
}

export function ParameterForm({ parameters, onChange }: Props) {
  const [form] = Form.useForm();

  const renderField = (param: ParameterDef) => {
    const rules = param.required ? [{ required: true, message: `请输入${param.label}` }] : [];

    switch (param.type) {
      case 'number':
        return (
          <Form.Item key={param.name} name={param.name} label={param.label} rules={rules}>
            <InputNumber min={param.min} max={param.max} style={{ width: '100%' }} />
          </Form.Item>
        );
      case 'boolean':
        return (
          <Form.Item key={param.name} name={param.name} label={param.label} valuePropName="checked">
            <Switch />
          </Form.Item>
        );
      case 'enum':
        return (
          <Form.Item key={param.name} name={param.name} label={param.label} rules={rules}>
            <Select options={param.options?.map((o) => ({ value: o.value, label: o.label }))} />
          </Form.Item>
        );
      case 'date':
        return (
          <Form.Item key={param.name} name={param.name} label={param.label} rules={rules}>
            <DatePicker style={{ width: '100%' }} />
          </Form.Item>
        );
      default:
        return (
          <Form.Item key={param.name} name={param.name} label={param.label} rules={rules}>
            <Input />
          </Form.Item>
        );
    }
  };

  return (
    <Form
      form={form}
      layout="vertical"
      initialValues={Object.fromEntries(
        parameters.filter((p) => p.default !== undefined).map((p) => [p.name, p.default])
      )}
      onValuesChange={(_, values) => onChange(values)}
    >
      {parameters.map(renderField)}
    </Form>
  );
}
```

**验证:**
```bash
cd web/admin-ui && pnpm run build
```

---

### Task 1.8: Phase 1 集成测试

**Files:**
- Create: `crates/unified-rule-engine/tests/template_integration.rs`

**实现要点:**

```rust
#[tokio::test]
async fn test_template_compile_and_evaluate() {
    // 1. 创建模板
    let template = RuleTemplate {
        code: "test_amount".into(),
        template_json: serde_json::json!({
            "root": {
                "type": "condition",
                "field": "order.amount",
                "operator": "gte",
                "value": "${amount}"
            }
        }),
        parameters: vec![ParameterDef {
            name: "amount".into(),
            param_type: ParameterType::Number,
            label: "金额".into(),
            required: true,
            default: Some(serde_json::json!(100)),
            ..Default::default()
        }],
        ..Default::default()
    };

    // 2. 编译
    let compiler = TemplateCompiler::new();
    let params = [("amount".to_string(), serde_json::json!(500))].into();
    let rule_json = compiler.compile(&template, &params).unwrap();

    // 3. 验证编译结果
    assert_eq!(
        rule_json["root"]["value"],
        serde_json::json!(500)
    );

    // 4. 使用规则引擎评估
    let rule: Rule = serde_json::from_value(rule_json).unwrap();
    let context = EvaluationContext::new(serde_json::json!({
        "order": { "amount": 600 }
    }));

    let executor = RuleExecutor::new();
    let result = executor.execute(&rule, &context).unwrap();
    assert!(result.matched);
}
```

**验证:**
```bash
cargo test -p unified-rule-engine template_integration
```

---

## Phase 2: 权益系统重构（Trait 抽象）

### Task 2.1: 数据库迁移 - benefit_grants 表

**Files:**
- Create: `migrations/20250131_005_benefit_grants.sql`

**实现要点:**

```sql
-- 权益发放记录表
CREATE TABLE benefit_grants (
    id BIGSERIAL PRIMARY KEY,
    grant_no VARCHAR(50) NOT NULL UNIQUE,
    user_id VARCHAR(100) NOT NULL,
    benefit_id BIGINT NOT NULL REFERENCES benefits(id),
    redemption_order_id BIGINT REFERENCES redemption_orders(id),

    -- 状态追踪
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    status_message TEXT,

    -- 外部系统
    external_ref VARCHAR(200),
    external_response JSONB,

    -- 权益数据
    payload JSONB,

    -- 时间
    granted_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    revoke_reason VARCHAR(50),

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_benefit_grants_user ON benefit_grants(user_id);
CREATE INDEX idx_benefit_grants_status ON benefit_grants(status);
CREATE INDEX idx_benefit_grants_benefit ON benefit_grants(benefit_id);
CREATE INDEX idx_benefit_grants_order ON benefit_grants(redemption_order_id);

-- 扩展权益类型
-- 注意：需要先删除依赖该类型的列的默认值，修改类型，再恢复
-- 这里假设可以直接添加新值（PostgreSQL 支持 ALTER TYPE ADD VALUE）

-- 添加新的权益类型
DO $$
BEGIN
    -- 检查并添加 POINTS 类型
    IF NOT EXISTS (SELECT 1 FROM pg_enum WHERE enumlabel = 'POINTS'
                   AND enumtypid = (SELECT oid FROM pg_type WHERE typname = 'benefit_type')) THEN
        -- PostgreSQL 不支持直接 ADD VALUE 在事务中，使用替代方案
        ALTER TABLE benefits ALTER COLUMN benefit_type TYPE VARCHAR(50);
    END IF;
END $$;
```

**验证:**
```bash
make db-migrate
podman exec badge-postgres psql -U badge -d badge_db -c "\d benefit_grants"
```

---

### Task 2.2: 扩展 BenefitType 枚举

**Files:**
- Modify: `crates/badge-management-service/src/models/enums.rs`

**实现要点:**

```rust
/// 权益类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "varchar", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BenefitType {
    /// 数字资产 - NFT、虚拟物品等
    DigitalAsset,
    /// 优惠券 - 折扣券、满减券等
    Coupon,
    /// 预约资格 - VIP 通道、优先预约等
    Reservation,
    /// 积分 - 可累加的虚拟货币
    Points,
    /// 实物奖品 - 需要物流配送
    Physical,
    /// 会员权益 - VIP 等级、会员时长等
    Membership,
    /// 外部回调 - 通用异步权益
    ExternalCallback,
}

impl BenefitType {
    /// 判断是否为同步发放类型
    pub fn is_sync(&self) -> bool {
        matches!(self, Self::Coupon | Self::Points | Self::DigitalAsset)
    }

    /// 判断是否支持回收
    pub fn is_revocable(&self) -> bool {
        matches!(self, Self::Coupon | Self::Points | Self::Membership)
    }
}

/// 权益发放状态
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "varchar", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GrantStatus {
    #[default]
    Pending,
    Success,
    Failed,
    Revoked,
}

/// 回收原因
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "varchar", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RevokeReason {
    UserRequest,
    OrderRefund,
    Expiration,
    Violation,
    SystemError,
}
```

**验证:**
```bash
cargo build -p badge-management-service
```

---

### Task 2.3: BenefitHandler Trait 定义

**Files:**
- Create: `crates/badge-management-service/src/benefit/mod.rs`
- Create: `crates/badge-management-service/src/benefit/handler.rs`
- Create: `crates/badge-management-service/src/benefit/dto.rs`
- Modify: `crates/badge-management-service/src/lib.rs`

**实现要点:**

```rust
// benefit/dto.rs
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::models::enums::{BenefitType, GrantStatus};

#[derive(Debug, Clone)]
pub struct GrantRequest {
    pub user_id: String,
    pub benefit_id: i64,
    pub benefit_config: Value,
    pub redemption_order_id: Option<i64>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrantResult {
    pub grant_id: i64,
    pub grant_no: String,
    pub status: GrantStatus,
    pub external_ref: Option<String>,
    pub payload: Value,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RevokeResult {
    pub success: bool,
    pub message: Option<String>,
}

// benefit/handler.rs
use async_trait::async_trait;
use crate::error::ServiceError;
use super::dto::*;

/// 权益处理器 Trait
#[async_trait]
pub trait BenefitHandler: Send + Sync {
    /// 权益类型
    fn benefit_type(&self) -> BenefitType;

    /// 发放权益
    async fn grant(&self, request: GrantRequest) -> Result<GrantResult, ServiceError>;

    /// 查询发放状态
    async fn query_status(&self, grant_id: i64) -> Result<GrantStatus, ServiceError>;

    /// 回收权益（默认不支持）
    async fn revoke(&self, grant_id: i64, reason: RevokeReason) -> Result<RevokeResult, ServiceError> {
        Err(ServiceError::NotSupported("此权益类型不支持回收".into()))
    }

    /// 验证配置
    fn validate_config(&self, config: &Value) -> Result<(), ServiceError>;
}
```

**验证:**
```bash
cargo build -p badge-management-service
```

---

### Task 2.4: 具体 Handler 实现

**Files:**
- Create: `crates/badge-management-service/src/benefit/handlers/mod.rs`
- Create: `crates/badge-management-service/src/benefit/handlers/coupon.rs`
- Create: `crates/badge-management-service/src/benefit/handlers/points.rs`
- Create: `crates/badge-management-service/src/benefit/handlers/physical.rs`

**实现要点:**

```rust
// handlers/coupon.rs
use async_trait::async_trait;
use super::super::{BenefitHandler, dto::*};

pub struct CouponHandler {
    // 可以注入外部优惠券服务客户端
}

#[async_trait]
impl BenefitHandler for CouponHandler {
    fn benefit_type(&self) -> BenefitType {
        BenefitType::Coupon
    }

    async fn grant(&self, request: GrantRequest) -> Result<GrantResult, ServiceError> {
        // 1. 从配置中获取优惠券模板ID
        let coupon_id = request.benefit_config["couponId"]
            .as_str()
            .ok_or(ServiceError::InvalidConfig("缺少 couponId".into()))?;

        // 2. 调用优惠券服务发放（这里模拟）
        let coupon_code = format!("CPN-{}-{}", coupon_id, uuid::Uuid::new_v4());

        // 3. 返回结果
        Ok(GrantResult {
            grant_id: 0, // 由上层设置
            grant_no: String::new(),
            status: GrantStatus::Success,
            external_ref: Some(coupon_code.clone()),
            payload: serde_json::json!({
                "couponCode": coupon_code,
                "couponId": coupon_id,
            }),
            message: None,
        })
    }

    async fn query_status(&self, grant_id: i64) -> Result<GrantStatus, ServiceError> {
        // 优惠券是同步发放，直接返回成功
        Ok(GrantStatus::Success)
    }

    async fn revoke(&self, grant_id: i64, reason: RevokeReason) -> Result<RevokeResult, ServiceError> {
        // 调用优惠券服务作废
        Ok(RevokeResult {
            success: true,
            message: Some("优惠券已作废".into()),
        })
    }

    fn validate_config(&self, config: &Value) -> Result<(), ServiceError> {
        if config.get("couponId").is_none() {
            return Err(ServiceError::InvalidConfig("缺少 couponId".into()));
        }
        Ok(())
    }
}

// handlers/points.rs
pub struct PointsHandler {
    // 积分服务客户端
}

#[async_trait]
impl BenefitHandler for PointsHandler {
    fn benefit_type(&self) -> BenefitType {
        BenefitType::Points
    }

    async fn grant(&self, request: GrantRequest) -> Result<GrantResult, ServiceError> {
        let points = request.benefit_config["points"]
            .as_i64()
            .ok_or(ServiceError::InvalidConfig("缺少 points".into()))?;

        // 调用积分服务增加积分
        Ok(GrantResult {
            grant_id: 0,
            grant_no: String::new(),
            status: GrantStatus::Success,
            external_ref: None,
            payload: serde_json::json!({ "points": points }),
            message: Some(format!("发放 {} 积分", points)),
        })
    }

    async fn revoke(&self, grant_id: i64, reason: RevokeReason) -> Result<RevokeResult, ServiceError> {
        // 扣减积分
        Ok(RevokeResult {
            success: true,
            message: Some("积分已扣减".into()),
        })
    }

    fn validate_config(&self, config: &Value) -> Result<(), ServiceError> {
        if config.get("points").is_none() {
            return Err(ServiceError::InvalidConfig("缺少 points".into()));
        }
        Ok(())
    }
}

// handlers/physical.rs (异步权益示例)
pub struct PhysicalHandler {
    kafka_producer: Arc<KafkaProducer>,
}

#[async_trait]
impl BenefitHandler for PhysicalHandler {
    fn benefit_type(&self) -> BenefitType {
        BenefitType::Physical
    }

    async fn grant(&self, request: GrantRequest) -> Result<GrantResult, ServiceError> {
        // 实物发放是异步的，发送到 Kafka 队列
        let message = serde_json::json!({
            "type": "physical_grant",
            "userId": request.user_id,
            "benefitId": request.benefit_id,
            "config": request.benefit_config,
        });

        self.kafka_producer.send("benefit.grants", &message).await?;

        // 返回 pending 状态
        Ok(GrantResult {
            grant_id: 0,
            grant_no: String::new(),
            status: GrantStatus::Pending,
            external_ref: None,
            payload: serde_json::json!({}),
            message: Some("已提交发货申请，等待处理".into()),
        })
    }

    async fn query_status(&self, grant_id: i64) -> Result<GrantStatus, ServiceError> {
        // 查询外部系统状态
        // ...
        Ok(GrantStatus::Pending)
    }

    fn validate_config(&self, config: &Value) -> Result<(), ServiceError> {
        // 验证收货地址等必填信息
        Ok(())
    }
}
```

**验证:**
```bash
cargo build -p badge-management-service
```

---

### Task 2.5: Handler 注册表和服务

**Files:**
- Create: `crates/badge-management-service/src/benefit/registry.rs`
- Create: `crates/badge-management-service/src/benefit/service.rs`

**实现要点:**

```rust
// registry.rs
use std::collections::HashMap;
use std::sync::Arc;
use super::{BenefitHandler, BenefitType};

pub struct HandlerRegistry {
    handlers: HashMap<BenefitType, Arc<dyn BenefitHandler>>,
}

impl HandlerRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register(&mut self, handler: Arc<dyn BenefitHandler>) {
        self.handlers.insert(handler.benefit_type(), handler);
    }

    pub fn get(&self, benefit_type: BenefitType) -> Option<Arc<dyn BenefitHandler>> {
        self.handlers.get(&benefit_type).cloned()
    }
}

// service.rs
pub struct BenefitService {
    registry: Arc<HandlerRegistry>,
    grant_repo: Arc<BenefitGrantRepository>,
    benefit_repo: Arc<BenefitRepository>,
}

impl BenefitService {
    pub async fn grant_benefit(&self, request: GrantRequest) -> Result<GrantResult, ServiceError> {
        // 1. 获取权益定义
        let benefit = self.benefit_repo.get_by_id(request.benefit_id).await?
            .ok_or(ServiceError::NotFound("权益不存在".into()))?;

        // 2. 获取 Handler
        let handler = self.registry.get(benefit.benefit_type)
            .ok_or(ServiceError::NotSupported(format!("不支持的权益类型: {:?}", benefit.benefit_type)))?;

        // 3. 验证配置
        handler.validate_config(&benefit.config)?;

        // 4. 创建发放记录
        let grant_no = self.generate_grant_no();
        let grant_id = self.grant_repo.create(&CreateGrantInput {
            grant_no: grant_no.clone(),
            user_id: request.user_id.clone(),
            benefit_id: request.benefit_id,
            redemption_order_id: request.redemption_order_id,
            status: GrantStatus::Pending,
        }).await?;

        // 5. 调用 Handler 发放
        let mut result = handler.grant(GrantRequest {
            benefit_config: benefit.config.clone(),
            ..request
        }).await?;

        result.grant_id = grant_id;
        result.grant_no = grant_no;

        // 6. 更新发放记录
        self.grant_repo.update_status(grant_id, &result).await?;

        Ok(result)
    }

    pub async fn revoke_grant(&self, grant_id: i64, reason: RevokeReason) -> Result<RevokeResult, ServiceError> {
        // 1. 获取发放记录
        let grant = self.grant_repo.get_by_id(grant_id).await?
            .ok_or(ServiceError::NotFound("发放记录不存在".into()))?;

        // 2. 检查状态
        if grant.status != GrantStatus::Success {
            return Err(ServiceError::InvalidState("只有成功的发放才能回收".into()));
        }

        // 3. 获取权益和 Handler
        let benefit = self.benefit_repo.get_by_id(grant.benefit_id).await?
            .ok_or(ServiceError::NotFound("权益不存在".into()))?;

        if !benefit.benefit_type.is_revocable() {
            return Err(ServiceError::NotSupported("此权益类型不支持回收".into()));
        }

        let handler = self.registry.get(benefit.benefit_type)
            .ok_or(ServiceError::NotSupported("Handler 未注册".into()))?;

        // 4. 调用 Handler 回收
        let result = handler.revoke(grant_id, reason).await?;

        // 5. 更新记录
        if result.success {
            self.grant_repo.mark_revoked(grant_id, reason).await?;

            // 6. 恢复库存
            self.benefit_repo.restore_stock(grant.benefit_id).await?;
        }

        Ok(result)
    }

    fn generate_grant_no(&self) -> String {
        format!("BG{}{:06}",
            chrono::Utc::now().format("%Y%m%d%H%M%S"),
            rand::random::<u32>() % 1000000
        )
    }
}
```

**验证:**
```bash
cargo build -p badge-management-service
cargo test -p badge-management-service benefit::
```

---

### Task 2.6: 集成到兑换服务

**Files:**
- Modify: `crates/badge-management-service/src/service/redemption_service.rs`

**实现要点:**

```rust
// 在 RedemptionService 中注入 BenefitService
pub struct RedemptionService {
    // ... 现有字段 ...
    benefit_service: Arc<BenefitService>,
}

impl RedemptionService {
    pub async fn redeem(&self, request: RedeemRequest) -> Result<RedeemResponse, ServiceError> {
        // ... 现有验证逻辑 ...

        // 替换原有的直接权益处理逻辑
        // 使用 BenefitService
        let grant_result = self.benefit_service.grant_benefit(GrantRequest {
            user_id: request.user_id.clone(),
            benefit_id: rule.benefit_id,
            benefit_config: benefit.config.clone(),
            redemption_order_id: Some(order_id),
            idempotency_key: request.idempotency_key.clone(),
        }).await?;

        // 更新订单状态
        self.order_repo.update_benefit_result(order_id, &grant_result).await?;

        // ... 后续逻辑 ...
    }
}
```

**验证:**
```bash
cargo build -p badge-management-service
cargo test -p badge-management-service redemption
```

---

### Task 2.7: Phase 2 集成测试

**Files:**
- Create: `crates/badge-management-service/tests/benefit_integration.rs`

**实现要点:**

```rust
#[tokio::test]
async fn test_coupon_grant_and_revoke() {
    let service = setup_benefit_service().await;

    // 1. 发放优惠券
    let result = service.grant_benefit(GrantRequest {
        user_id: "test-user".into(),
        benefit_id: 1, // 预置的优惠券权益
        benefit_config: serde_json::json!({"couponId": "test-coupon"}),
        redemption_order_id: None,
        idempotency_key: Some("test-key".into()),
    }).await.unwrap();

    assert_eq!(result.status, GrantStatus::Success);
    assert!(result.payload["couponCode"].is_string());

    // 2. 回收
    let revoke_result = service.revoke_grant(result.grant_id, RevokeReason::UserRequest).await.unwrap();
    assert!(revoke_result.success);

    // 3. 验证状态
    let status = service.query_grant_status(result.grant_id).await.unwrap();
    assert_eq!(status, GrantStatus::Revoked);
}

#[tokio::test]
async fn test_physical_async_grant() {
    let service = setup_benefit_service().await;

    // 发放实物（异步）
    let result = service.grant_benefit(GrantRequest {
        user_id: "test-user".into(),
        benefit_id: 2, // 预置的实物权益
        benefit_config: serde_json::json!({"productId": "PROD-001"}),
        redemption_order_id: None,
        idempotency_key: None,
    }).await.unwrap();

    // 异步权益返回 pending
    assert_eq!(result.status, GrantStatus::Pending);
}
```

**验证:**
```bash
cargo test -p badge-management-service benefit_integration
```

---

## Phase 3: 可观测性重构（统一标准）

### Task 3.1: 可观测性模块结构

**Files:**
- Create: `crates/shared/src/observability/mod.rs`
- Create: `crates/shared/src/observability/metrics.rs`
- Create: `crates/shared/src/observability/tracing.rs`
- Create: `crates/shared/src/observability/middleware.rs`
- Modify: `crates/shared/src/lib.rs`

**实现要点:**

```rust
// observability/mod.rs
pub mod metrics;
pub mod tracing;
pub mod middleware;

use crate::config::ObservabilityConfig;

pub struct ObservabilityGuard {
    _meter_provider: Option<opentelemetry_sdk::metrics::SdkMeterProvider>,
    _tracer_provider: Option<opentelemetry_sdk::trace::TracerProvider>,
}

impl Drop for ObservabilityGuard {
    fn drop(&mut self) {
        // 清理资源
        if let Some(provider) = self._meter_provider.take() {
            let _ = provider.shutdown();
        }
    }
}

/// 初始化可观测性
pub fn init(config: &ObservabilityConfig, service_name: &str) -> Result<ObservabilityGuard, Box<dyn std::error::Error>> {
    // 1. 初始化 tracing
    tracing::init(config, service_name)?;

    // 2. 初始化 metrics
    let meter_provider = if config.metrics_enabled {
        Some(metrics::init(config, service_name)?)
    } else {
        None
    };

    // 3. 初始化分布式追踪
    let tracer_provider = if config.tracing_enabled {
        Some(tracing::init_otlp(config, service_name)?)
    } else {
        None
    };

    Ok(ObservabilityGuard {
        _meter_provider: meter_provider,
        _tracer_provider: tracer_provider,
    })
}
```

**验证:**
```bash
cargo build -p shared
```

---

### Task 3.2: Metrics 实现

**Files:**
- Modify: `crates/shared/src/observability/metrics.rs`

**实现要点:**

```rust
// metrics.rs
use metrics::{counter, gauge, histogram, describe_counter, describe_gauge, describe_histogram};
use metrics_exporter_prometheus::PrometheusBuilder;

/// 初始化 Prometheus metrics exporter
pub fn init(config: &ObservabilityConfig, service_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 创建 Prometheus exporter
    let builder = PrometheusBuilder::new();
    builder
        .with_http_listener(([0, 0, 0, 0], config.metrics_port))
        .install()?;

    // 注册指标描述
    register_metrics();

    Ok(())
}

fn register_metrics() {
    // 徽章发放
    describe_counter!("badge_grants_total", "徽章发放总数");
    describe_histogram!("badge_grant_duration_seconds", "徽章发放耗时");

    // 级联触发
    describe_counter!("cascade_evaluations_total", "级联评估总数");
    describe_histogram!("cascade_evaluation_duration_seconds", "级联评估耗时");

    // 兑换
    describe_counter!("redemptions_total", "兑换总数");
    describe_histogram!("redemption_duration_seconds", "兑换耗时");

    // 规则引擎
    describe_counter!("rule_evaluations_total", "规则评估总数");
    describe_histogram!("rule_evaluation_duration_seconds", "规则评估耗时");

    // 权益
    describe_counter!("benefit_grants_total", "权益发放总数");
    describe_gauge!("benefit_remaining_stock", "权益剩余库存");

    // HTTP/gRPC
    describe_counter!("http_requests_total", "HTTP请求总数");
    describe_histogram!("http_request_duration_seconds", "HTTP请求耗时");
    describe_counter!("grpc_requests_total", "gRPC请求总数");
    describe_histogram!("grpc_request_duration_seconds", "gRPC请求耗时");
}

/// 徽章系统指标记录器
pub struct BadgeMetrics;

impl BadgeMetrics {
    pub fn grant_total(badge_id: i64, source: &str, status: &str) {
        counter!("badge_grants_total",
            "badge_id" => badge_id.to_string(),
            "source" => source.to_owned(),
            "status" => status.to_owned()
        ).increment(1);
    }

    pub fn grant_duration(duration: std::time::Duration) {
        histogram!("badge_grant_duration_seconds").record(duration.as_secs_f64());
    }

    pub fn cascade_total(depth: u32, status: &str) {
        counter!("cascade_evaluations_total",
            "depth" => depth.to_string(),
            "status" => status.to_owned()
        ).increment(1);
    }

    pub fn cascade_duration(duration: std::time::Duration) {
        histogram!("cascade_evaluation_duration_seconds").record(duration.as_secs_f64());
    }

    pub fn redemption_total(rule_id: i64, status: &str) {
        counter!("redemptions_total",
            "rule_id" => rule_id.to_string(),
            "status" => status.to_owned()
        ).increment(1);
    }

    pub fn redemption_duration(duration: std::time::Duration) {
        histogram!("redemption_duration_seconds").record(duration.as_secs_f64());
    }

    pub fn rule_evaluation(matched: bool, duration: std::time::Duration) {
        counter!("rule_evaluations_total", "matched" => matched.to_string()).increment(1);
        histogram!("rule_evaluation_duration_seconds").record(duration.as_secs_f64());
    }

    pub fn benefit_grant(benefit_type: &str, status: &str) {
        counter!("benefit_grants_total",
            "benefit_type" => benefit_type.to_owned(),
            "status" => status.to_owned()
        ).increment(1);
    }

    pub fn benefit_stock(benefit_id: i64, remaining: i64) {
        gauge!("benefit_remaining_stock", "benefit_id" => benefit_id.to_string())
            .set(remaining as f64);
    }
}
```

**验证:**
```bash
cargo build -p shared
```

---

### Task 3.3: 分布式追踪实现

**Files:**
- Modify: `crates/shared/src/observability/tracing.rs`

**实现要点:**

```rust
// tracing.rs
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{trace as sdktrace, Resource};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// 初始化基础 tracing（日志）
pub fn init(config: &ObservabilityConfig, service_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    let fmt_layer = if config.log_format == "json" {
        tracing_subscriber::fmt::layer()
            .json()
            .with_target(true)
            .boxed()
    } else {
        tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_level(true)
            .boxed()
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();

    Ok(())
}

/// 初始化 OpenTelemetry 追踪
pub fn init_otlp(config: &ObservabilityConfig, service_name: &str) -> Result<sdktrace::TracerProvider, Box<dyn std::error::Error>> {
    let endpoint = config.tracing_endpoint.as_deref().unwrap_or("http://localhost:4317");

    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(endpoint);

    let tracer_provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(
            sdktrace::Config::default()
                .with_resource(Resource::new(vec![
                    opentelemetry::KeyValue::new(SERVICE_NAME, service_name.to_string()),
                ]))
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    // 设置全局 tracer provider
    opentelemetry::global::set_tracer_provider(tracer_provider.clone());

    Ok(tracer_provider)
}

/// 追踪上下文传播
pub mod propagation {
    use opentelemetry::propagation::{TextMapPropagator, Injector, Extractor};
    use opentelemetry_sdk::propagation::TraceContextPropagator;
    use std::collections::HashMap;

    /// 从 HTTP headers 提取追踪上下文
    pub fn extract_from_headers(headers: &http::HeaderMap) -> opentelemetry::Context {
        let propagator = TraceContextPropagator::new();
        let extractor = HeaderExtractor(headers);
        propagator.extract(&extractor)
    }

    /// 注入追踪上下文到 HTTP headers
    pub fn inject_to_headers(cx: &opentelemetry::Context, headers: &mut http::HeaderMap) {
        let propagator = TraceContextPropagator::new();
        let mut injector = HeaderInjector(headers);
        propagator.inject_context(cx, &mut injector);
    }

    struct HeaderExtractor<'a>(&'a http::HeaderMap);

    impl<'a> Extractor for HeaderExtractor<'a> {
        fn get(&self, key: &str) -> Option<&str> {
            self.0.get(key).and_then(|v| v.to_str().ok())
        }

        fn keys(&self) -> Vec<&str> {
            self.0.keys().map(|k| k.as_str()).collect()
        }
    }

    struct HeaderInjector<'a>(&'a mut http::HeaderMap);

    impl<'a> Injector for HeaderInjector<'a> {
        fn set(&mut self, key: &str, value: String) {
            if let Ok(name) = http::header::HeaderName::try_from(key) {
                if let Ok(val) = http::header::HeaderValue::try_from(&value) {
                    self.0.insert(name, val);
                }
            }
        }
    }
}
```

**验证:**
```bash
cargo build -p shared
```

---

### Task 3.4: HTTP/gRPC 中间件

**Files:**
- Modify: `crates/shared/src/observability/middleware.rs`

**实现要点:**

```rust
// middleware.rs
use axum::{extract::Request, middleware::Next, response::Response};
use std::time::Instant;
use super::metrics::BadgeMetrics;

/// Axum HTTP 可观测性中间件
pub async fn http_observability(request: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    // 提取追踪上下文
    let _cx = super::tracing::propagation::extract_from_headers(request.headers());

    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status().as_u16().to_string();

    // 记录指标
    metrics::counter!("http_requests_total",
        "method" => method.clone(),
        "path" => normalize_path(&path),
        "status" => status
    ).increment(1);

    metrics::histogram!("http_request_duration_seconds",
        "method" => method,
        "path" => normalize_path(&path)
    ).record(duration.as_secs_f64());

    response
}

/// 规范化路径（移除动态参数）
fn normalize_path(path: &str) -> String {
    // /api/users/123/badges -> /api/users/:id/badges
    let parts: Vec<&str> = path.split('/').collect();
    parts.iter()
        .map(|p| {
            if p.parse::<i64>().is_ok() {
                ":id"
            } else {
                *p
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// 创建带可观测性的 Axum layer
pub fn observability_layer() -> axum::middleware::from_fn<fn(Request, Next) -> _> {
    axum::middleware::from_fn(http_observability)
}
```

**验证:**
```bash
cargo build -p shared
```

---

### Task 3.5: 服务集成

**Files:**
- Modify: `crates/badge-admin-service/src/main.rs`
- Modify: `crates/badge-management-service/src/main.rs`

**实现要点:**

```rust
// badge-admin-service/src/main.rs
use shared::observability;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载配置
    let config = AppConfig::load("badge-admin-service")?;

    // 初始化可观测性（替换原有的 init_tracing）
    let _guard = observability::init(&config.observability, "badge-admin-service")?;

    // ... 其余代码 ...

    // 添加中间件
    let app = Router::new()
        .merge(routes::create_routes(state))
        .layer(axum::middleware::from_fn(observability::middleware::http_observability))
        // ... 其他 layer ...

    // ...
}
```

**验证:**
```bash
cargo build -p badge-admin-service
cargo build -p badge-management-service
```

---

### Task 3.6: Grafana 仪表盘和告警

**Files:**
- Create: `docker/grafana/provisioning/dashboards/dashboard.yml`
- Create: `docker/grafana/dashboards/badge-overview.json`
- Create: `docker/prometheus/prometheus.yml`
- Create: `docker/prometheus/alerts.yml`
- Create: `docker/docker-compose.observability.yml`

**实现要点:**

```yaml
# docker/docker-compose.observability.yml
version: '3.8'

services:
  prometheus:
    image: prom/prometheus:v2.50.0
    container_name: badge-prometheus
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus/prometheus.yml:/etc/prometheus/prometheus.yml
      - ./prometheus/alerts.yml:/etc/prometheus/alerts.yml
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
    networks:
      - badge-network

  grafana:
    image: grafana/grafana:10.3.0
    container_name: badge-grafana
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
      - GF_USERS_ALLOW_SIGN_UP=false
    volumes:
      - ./grafana/provisioning:/etc/grafana/provisioning
      - ./grafana/dashboards:/var/lib/grafana/dashboards
    depends_on:
      - prometheus
    networks:
      - badge-network

  jaeger:
    image: jaegertracing/all-in-one:1.54
    container_name: badge-jaeger
    ports:
      - "16686:16686"
      - "4317:4317"
      - "4318:4318"
    environment:
      - COLLECTOR_OTLP_ENABLED=true
    networks:
      - badge-network

networks:
  badge-network:
    external: true
```

```yaml
# docker/prometheus/prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - alerts.yml

scrape_configs:
  - job_name: 'badge-admin'
    static_configs:
      - targets: ['host.docker.internal:9091']

  - job_name: 'badge-management'
    static_configs:
      - targets: ['host.docker.internal:9092']

  - job_name: 'rule-engine'
    static_configs:
      - targets: ['host.docker.internal:9093']
```

```yaml
# docker/prometheus/alerts.yml
groups:
  - name: badge_alerts
    rules:
      - alert: HighGrantErrorRate
        expr: |
          sum(rate(badge_grants_total{status="error"}[5m]))
          / sum(rate(badge_grants_total[5m])) > 0.05
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "徽章发放错误率过高"
          description: "过去5分钟错误率超过5%"

      - alert: BenefitStockLow
        expr: benefit_remaining_stock < 100
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "权益库存不足"
          description: "权益 {{ $labels.benefit_id }} 剩余库存不足100"

      - alert: RedemptionLatencyHigh
        expr: |
          histogram_quantile(0.95,
            sum(rate(redemption_duration_seconds_bucket[5m])) by (le)
          ) > 2
        for: 3m
        labels:
          severity: warning
        annotations:
          summary: "兑换延迟过高"
          description: "P95延迟超过2秒"
```

**验证:**
```bash
podman compose -f docker/docker-compose.observability.yml up -d
curl http://localhost:9090/-/healthy
curl http://localhost:3000/api/health
```

---

### Task 3.7: Phase 3 集成测试

**Files:**
- Create: `crates/shared/tests/observability_integration.rs`

**实现要点:**

```rust
#[tokio::test]
async fn test_metrics_recording() {
    use shared::observability::metrics::BadgeMetrics;

    // 记录一些指标
    BadgeMetrics::grant_total(1, "event", "success");
    BadgeMetrics::grant_duration(std::time::Duration::from_millis(50));
    BadgeMetrics::cascade_total(2, "success");
    BadgeMetrics::benefit_stock(1, 500);

    // 验证指标可以被 scrape（需要启动 metrics server）
    // 这里只验证不会 panic
}

#[tokio::test]
async fn test_tracing_propagation() {
    use shared::observability::tracing::propagation;

    let mut headers = http::HeaderMap::new();
    headers.insert(
        "traceparent",
        "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".parse().unwrap()
    );

    let cx = propagation::extract_from_headers(&headers);

    // 验证上下文被正确提取
    assert!(!cx.span().span_context().trace_id().to_string().is_empty());
}
```

**验证:**
```bash
cargo test -p shared observability_integration
```

---

## 最终验证

### 全量构建

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

### 前端构建

```bash
cd web/admin-ui && pnpm run build
```

### E2E 测试

```bash
# 启动基础设施
make infra-up
make kafka-init

# 运行迁移
make db-migrate

# 启动可观测性组件
podman compose -f docker/docker-compose.observability.yml up -d

# 启动服务
make dev-backend

# 验证模板功能
curl http://localhost:8080/api/admin/templates | jq

# 验证指标
curl http://localhost:9091/metrics | grep badge_

# 验证追踪（打开 Jaeger UI）
open http://localhost:16686
```

---

## 验证清单

### Phase 1: 规则引擎
- [ ] `rule_templates` 表创建成功
- [ ] 15 个预置模板导入成功
- [ ] 模板编译器参数替换正确
- [ ] 从模板创建规则 API 正常
- [ ] 前端模板选择器可用

### Phase 2: 权益系统
- [ ] `benefit_grants` 表创建成功
- [ ] 新权益类型（Points, Physical）可用
- [ ] BenefitHandler Trait 实现正确
- [ ] 权益发放状态追踪正常
- [ ] 权益回收流程完整

### Phase 3: 可观测性
- [ ] Prometheus metrics 可见
- [ ] Jaeger 追踪链路完整
- [ ] Grafana 仪表盘展示正常
- [ ] 告警规则配置正确
- [ ] HTTP/gRPC 中间件工作正常
