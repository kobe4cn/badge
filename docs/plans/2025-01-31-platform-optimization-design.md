# 徽章平台优化设计文档

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:writing-plans to create implementation plan from this design.

**Goal:** 全面提升徽章系统的规则可配置性、权益扩展性和系统可观测性。

**Approach:** 三个领域均采用重构方案，确保架构统一、技术债最小化。

**Tech Stack:** Rust 2024, SQLx, OpenTelemetry, Prometheus, Grafana, Jaeger

---

## 一、规则引擎重构：模板参数化

### 1.1 设计目标

- 支持规则模板参数化（`${amount}` 占位符）
- 模板版本控制和溯源
- 预置 15+ 模板覆盖基础/高级/行业场景
- 前端模板选择器 + 参数表单

### 1.2 数据模型

**新增表 `rule_templates`：**

```sql
CREATE TABLE rule_templates (
    id BIGSERIAL PRIMARY KEY,
    code VARCHAR(50) NOT NULL UNIQUE,      -- 模板代码，如 'purchase_gte'
    name VARCHAR(100) NOT NULL,
    description TEXT,
    category VARCHAR(50) NOT NULL,          -- basic, advanced, industry
    subcategory VARCHAR(50),                -- e-commerce, gaming, o2o

    -- 模板定义
    template_json JSONB NOT NULL,           -- 带 ${param} 占位符的规则
    parameters JSONB NOT NULL DEFAULT '[]', -- 参数定义

    -- 版本控制
    version VARCHAR(20) NOT NULL DEFAULT '1.0',

    -- 状态
    is_system BOOLEAN NOT NULL DEFAULT FALSE, -- 系统内置模板不可删除
    enabled BOOLEAN NOT NULL DEFAULT TRUE,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_rule_templates_category ON rule_templates(category, subcategory);
CREATE INDEX idx_rule_templates_code ON rule_templates(code);
```

**扩展 `badge_rules` 表：**

```sql
ALTER TABLE badge_rules ADD COLUMN template_id BIGINT REFERENCES rule_templates(id);
ALTER TABLE badge_rules ADD COLUMN template_version VARCHAR(20);
ALTER TABLE badge_rules ADD COLUMN template_params JSONB DEFAULT '{}';
-- rule_json 保留，存储编译后的完整规则（缓存）
```

### 1.3 模板参数格式

```json
{
  "code": "purchase_amount_gte",
  "name": "消费满额触发",
  "category": "basic",
  "subcategory": "transaction",
  "parameters": [
    {
      "name": "amount",
      "type": "number",
      "label": "金额阈值",
      "default": 100,
      "min": 0,
      "required": true
    }
  ],
  "template_json": {
    "root": {
      "type": "group",
      "operator": "AND",
      "children": [
        {"type": "condition", "field": "event.type", "operator": "eq", "value": "purchase"},
        {"type": "condition", "field": "order.amount", "operator": "gte", "value": "${amount}"}
      ]
    }
  }
}
```

### 1.4 模板分类体系

```
templates/
├── basic/                    # 基础场景 (5个)
│   ├── first_event          # 首次行为
│   ├── cumulative_amount    # 累计金额
│   ├── cumulative_count     # 累计次数
│   ├── user_level_gte       # 用户等级
│   └── tag_match            # 标签匹配
│
├── advanced/                 # 高级场景 (3个)
│   ├── time_window_event    # 时间窗口
│   ├── streak_days          # 连续天数
│   └── frequency_limit      # 频次限制
│
└── industry/                 # 行业模板 (7个)
    ├── e-commerce/
    │   ├── first_purchase   # 首次购买
    │   ├── order_amount_gte # 订单满额
    │   └── repeat_purchase  # 复购
    ├── gaming/
    │   ├── level_reached    # 等级达成
    │   └── achievement      # 成就解锁
    └── o2o/
        ├── store_visit      # 到店
        └── review_posted    # 评价
```

### 1.5 规则编译流程

```
模板 + 参数 → 编译器 → 完整规则 JSON
     │
     ▼
┌─────────────────────────────────────────────┐
│ RuleCompiler::compile_from_template()       │
│                                             │
│ 1. 加载模板 template_json                    │
│ 2. 验证参数完整性                            │
│ 3. 替换 ${param} 占位符                      │
│ 4. 验证生成的规则语法                        │
│ 5. 缓存到 badge_rules.rule_json             │
└─────────────────────────────────────────────┘
```

---

## 二、权益系统重构：Trait 抽象

### 2.1 设计目标

- 引入 `BenefitHandler` Trait 多态
- 扩展权益类型（积分、实物、会员、外部回调）
- 异步发放队列 + 状态追踪
- 权益回收机制

### 2.2 Trait 定义

```rust
/// 权益处理器 Trait
#[async_trait]
pub trait BenefitHandler: Send + Sync {
    /// 权益类型标识
    fn benefit_type(&self) -> BenefitType;

    /// 发放权益
    async fn grant(&self, request: GrantRequest) -> Result<GrantResult>;

    /// 查询发放状态
    async fn query_status(&self, grant_id: &str) -> Result<GrantStatus>;

    /// 回收权益（可选）
    async fn revoke(&self, grant_id: &str) -> Result<RevokeResult> {
        Err(Error::NotSupported("此权益类型不支持回收".into()))
    }

    /// 验证配置
    fn validate_config(&self, config: &Value) -> Result<()>;
}
```

### 2.3 权益类型扩展

```rust
pub enum BenefitType {
    // 现有
    Coupon,           // 优惠券
    DigitalAsset,     // 数字资产
    Reservation,      // 预约资格

    // 新增
    Points,           // 积分
    Physical,         // 实物奖品
    Membership,       // 会员权益
    ExternalCallback, // 外部回调（通用）
}
```

### 2.4 发放状态追踪

**新增表 `benefit_grants`：**

```sql
CREATE TABLE benefit_grants (
    id BIGSERIAL PRIMARY KEY,
    grant_no VARCHAR(50) NOT NULL UNIQUE,

    -- 关联
    user_id VARCHAR(100) NOT NULL,
    benefit_id BIGINT NOT NULL REFERENCES benefits(id),
    redemption_order_id BIGINT REFERENCES redemption_orders(id),

    -- 状态追踪
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    -- pending, success, failed, revoked
    status_message TEXT,

    -- 外部系统
    external_ref VARCHAR(200),
    external_response JSONB,

    -- 权益数据
    payload JSONB,  -- 优惠券码、积分数、实物信息等

    -- 时间
    granted_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_benefit_grants_user ON benefit_grants(user_id);
CREATE INDEX idx_benefit_grants_status ON benefit_grants(status);
CREATE INDEX idx_benefit_grants_benefit ON benefit_grants(benefit_id);
```

### 2.5 异步发放流程

```
同步权益（Coupon, Points）:
  请求 → Handler.grant() → 直接返回结果

异步权益（Physical, ExternalCallback）:
  请求 → 创建 pending 记录 → 写入 Kafka(benefit.grants)
       → 外部系统处理 → Webhook 回调 → 更新状态
```

### 2.6 回收机制

```rust
pub enum RevokeReason {
    UserRequest,      // 用户主动退回
    OrderRefund,      // 订单退款
    Expiration,       // 过期清理
    Violation,        // 违规收回
    SystemError,      // 发放错误修正
}
```

**回收流程：**
1. 检查权益是否可回收（`is_revocable`）
2. 调用 Handler.revoke()
3. 更新 grant 状态为 revoked
4. 恢复库存（如适用）
5. 写入审计日志

---

## 三、可观测性重构：统一标准

### 3.1 设计目标

- 统一可观测性中间件层
- Prometheus 指标导出
- OpenTelemetry 分布式追踪
- Grafana 预置仪表盘 + 告警

### 3.2 架构图

```
┌─────────────────────────────────────────────────────────────┐
│                      应用服务层                              │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐           │
│  │ Admin   │ │ Mgmt    │ │ Rule    │ │ Event   │           │
│  │ Service │ │ Service │ │ Engine  │ │ Services│           │
│  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘           │
│       └──────────┴──────────┴──────────┘                   │
│                        │                                    │
│              ┌─────────▼─────────┐                         │
│              │ ObservabilityLayer │  ◀── 统一中间件         │
│              └─────────┬─────────┘                         │
└────────────────────────┼───────────────────────────────────┘
                         │
        ┌────────────────┼────────────────┐
        ▼                ▼                ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│  Prometheus  │ │    Jaeger    │ │    Loki      │
└──────┬───────┘ └──────┬───────┘ └──────┬───────┘
       └────────────────┼────────────────┘
                        ▼
               ┌──────────────┐
               │   Grafana    │
               └──────────────┘
```

### 3.3 核心指标

```rust
// 徽章发放
badge_grants_total{badge_id, source, status}
badge_grant_duration_ms

// 级联触发
cascade_evaluations_total{depth, status}
cascade_evaluation_duration_ms

// 兑换
redemptions_total{rule_id, status}
redemption_duration_ms

// 规则引擎
rule_evaluations_total{matched}
rule_evaluation_duration_ms

// 权益
benefit_grants_total{benefit_type, status}
benefit_remaining_stock{benefit_id}

// 系统
http_requests_total{method, path, status}
http_request_duration_ms
grpc_requests_total{service, method, status}
```

### 3.4 追踪传播

**支持的传播格式：**
- HTTP: W3C Trace Context (`traceparent`, `tracestate`)
- gRPC: 同上，通过 metadata
- Kafka: 消息头

**追踪 Span 层级：**
```
[HTTP Request] badge-admin POST /api/admin/grants
  └── [gRPC Call] badge-management GrantBadge
        ├── [DB Query] INSERT user_badges
        ├── [Redis] SET user:xxx:badges
        └── [Cascade] evaluate
              └── [gRPC Call] badge-management GrantBadge (recursive)
```

### 3.5 Grafana 仪表盘

| 仪表盘 | 面板 |
|--------|------|
| **Badge Overview** | 发放趋势、Top 10 徽章、成功率、P95 延迟 |
| **Cascade & Redemption** | 级联深度分布、兑换漏斗、库存水位 |
| **System Health** | 服务状态、错误率热力图、依赖拓扑 |

### 3.6 告警规则

| 告警 | 条件 | 级别 |
|------|------|------|
| HighGrantErrorRate | 错误率 > 5% (5分钟) | critical |
| CascadeTimeout | 超时次数 > 0 (1分钟) | warning |
| BenefitStockLow | 库存 < 100 | warning |
| RedemptionLatencyHigh | P95 > 2s | warning |
| ServiceDown | 服务不可用 | critical |

### 3.7 基础设施扩展

```yaml
# docker/docker-compose.observability.yml
services:
  prometheus:
    image: prom/prometheus:v2.50.0
    ports: ["9090:9090"]

  grafana:
    image: grafana/grafana:10.3.0
    ports: ["3000:3000"]

  jaeger:
    image: jaegertracing/all-in-one:1.54
    ports: ["16686:16686", "4317:4317"]

  loki:
    image: grafana/loki:2.9.4
    ports: ["3100:3100"]
```

---

## 四、实现计划概览

| Phase | 内容 | 预计工作量 |
|-------|------|-----------|
| **Phase 1** | 规则引擎重构 | 4-5 天 |
| **Phase 2** | 权益系统重构 | 3-4 天 |
| **Phase 3** | 可观测性重构 | 4-5 天 |
| **Phase 4** | 测试场景扩展 | 2-3 天 |
| **总计** | | 13-17 天 |

---

## 五、验证清单

- [ ] 规则模板 CRUD 正常
- [ ] 模板参数化编译正确
- [ ] 从模板创建规则流程完整
- [ ] 新权益类型发放成功
- [ ] 异步权益状态追踪正确
- [ ] 权益回收流程完整
- [ ] Prometheus 指标可见
- [ ] Jaeger 追踪链路完整
- [ ] Grafana 仪表盘展示正常
- [ ] 告警规则触发正确
