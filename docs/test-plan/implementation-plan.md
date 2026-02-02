# Badge 系统测试实施计划

## 实施概述

基于确认的测试方案，将分 **5 个阶段** 实施：

| 阶段 | 内容 | 产出物 |
|------|------|--------|
| **Phase 1** | 测试基础设施 | 测试框架、数据集、Mock 扩展 |
| **Phase 2** | 后端集成测试 | E2E 测试代码 |
| **Phase 3** | 前端自动化测试 | Playwright 测试 |
| **Phase 4** | 性能测试 | 压测脚本、报告 |
| **Phase 5** | CI/CD 集成 | GitHub Actions |

---

## Phase 1: 测试基础设施

### 1.1 目录结构
```
tests/
├── e2e/                          # 后端 E2E 测试
│   ├── mod.rs
│   ├── setup/
│   │   ├── mod.rs
│   │   ├── environment.rs        # 环境初始化
│   │   ├── services.rs           # 服务启动/健康检查
│   │   └── cleanup.rs            # 测试清理
│   ├── helpers/
│   │   ├── mod.rs
│   │   ├── api_client.rs         # REST/gRPC 客户端
│   │   ├── kafka_helper.rs       # Kafka 生产者/消费者
│   │   ├── db_verifier.rs        # 数据库断言
│   │   └── assertions.rs         # 自定义断言宏
│   ├── data/
│   │   ├── mod.rs
│   │   ├── fixtures.rs           # 测试数据 fixtures
│   │   ├── scenarios.rs          # 业务场景数据
│   │   └── generators.rs         # 数据生成器
│   └── suites/
│       ├── mod.rs
│       ├── basic_config.rs       # 基础配置测试
│       ├── rule_config.rs        # 规则配置测试
│       ├── benefit_config.rs     # 权益配置测试
│       ├── event_trigger.rs      # 事件触发测试
│       ├── cascade_trigger.rs    # 级联触发测试
│       ├── redemption.rs         # 兑换流程测试
│       ├── notification.rs       # 通知系统测试
│       ├── reverse_flow.rs       # 逆向场景测试
│       └── data_consistency.rs   # 数据一致性测试
├── performance/                  # 性能测试
│   ├── mod.rs
│   ├── scenarios/
│   │   ├── single_user.rs
│   │   ├── concurrent_events.rs
│   │   ├── cascade_perf.rs
│   │   └── competitive_redemption.rs
│   └── reports/
└── frontend/                     # Playwright 测试 (单独项目)

web/admin-ui/
├── e2e/                          # Playwright 测试
│   ├── playwright.config.ts
│   ├── fixtures/
│   │   ├── auth.fixture.ts       # 登录 fixture
│   │   └── data.fixture.ts       # 测试数据
│   ├── pages/                    # Page Objects
│   │   ├── dashboard.page.ts
│   │   ├── badges.page.ts
│   │   ├── rules.page.ts
│   │   ├── canvas.page.ts
│   │   └── benefits.page.ts
│   └── specs/
│       ├── basic-config.spec.ts
│       ├── canvas-editor.spec.ts
│       ├── benefit-config.spec.ts
│       └── full-flow.spec.ts
```

### 1.2 测试数据集
```rust
// tests/e2e/data/fixtures.rs

/// 预置测试数据
pub struct TestFixtures {
    // 用户数据
    pub users: Vec<TestUser>,
    // 分类数据
    pub categories: Vec<TestCategory>,
    // 系列数据
    pub series: Vec<TestSeries>,
    // 徽章数据
    pub badges: Vec<TestBadge>,
    // 规则数据
    pub rules: Vec<TestRule>,
    // 权益数据
    pub benefits: Vec<TestBenefit>,
}

impl TestFixtures {
    /// 加载标准测试数据集
    pub fn standard() -> Self { ... }

    /// 加载性能测试数据集 (大量数据)
    pub fn performance() -> Self { ... }

    /// 加载边界条件数据集
    pub fn edge_cases() -> Self { ... }
}
```

### 1.3 Mock 服务扩展
```rust
// crates/mock-services/src/external/

// 新增 Mock 服务
pub mod coupon_service;    // 优惠券系统 Mock
pub mod points_service;    // 积分系统 Mock
pub mod member_service;    // 会员系统 Mock
pub mod push_service;      // 推送系统 Mock
pub mod logistics_service; // 物流系统 Mock

// Mock 行为配置
pub struct MockBehavior {
    pub delay_ms: Option<u64>,      // 响应延迟
    pub fail_rate: f32,             // 失败率
    pub error_type: Option<String>, // 错误类型
}
```

---

## Phase 2: 后端 E2E 测试

### 2.1 测试套件清单

| 套件 | 用例数 | 优先级 |
|------|--------|--------|
| basic_config | 20 | P0 |
| rule_config | 25 | P0 |
| benefit_config | 15 | P0 |
| event_trigger | 30 | P0 |
| cascade_trigger | 15 | P0 |
| redemption | 12 | P0 |
| notification | 10 | P1 |
| reverse_flow | 15 | P1 |
| data_consistency | 10 | P0 |
| **总计** | **152** | - |

### 2.2 核心测试用例示例

```rust
// tests/e2e/suites/event_trigger.rs

#[tokio::test]
async fn test_purchase_event_triggers_badge_grant() {
    // 1. 准备测试环境
    let env = TestEnvironment::setup().await;

    // 2. 创建徽章和规则
    let badge = env.create_badge(TestBadge::first_purchase()).await;
    let rule = env.create_rule(TestRule::purchase_gte(1000, badge.id)).await;

    // 3. 等待规则热加载
    env.wait_for_rule_reload().await;

    // 4. 发送购买事件到 Kafka
    let event = PurchaseEvent {
        user_id: "user-001".into(),
        order_id: "order-001".into(),
        amount: 1500,
        ..Default::default()
    };
    env.kafka.send_transaction_event(event).await;

    // 5. 等待事件处理完成
    env.wait_for_processing(Duration::from_secs(5)).await;

    // 6. 验证数据库
    let user_badges = env.db.get_user_badges("user-001").await;
    assert!(user_badges.iter().any(|b| b.badge_id == badge.id));

    // 7. 验证账本
    let ledger = env.db.get_badge_ledger(badge.id, "user-001").await;
    assert_eq!(ledger.len(), 1);
    assert_eq!(ledger[0].delta, 1);

    // 8. 验证通知
    let notifications = env.kafka.consume_notifications().await;
    assert!(notifications.iter().any(|n|
        n.notification_type == "BADGE_GRANTED" &&
        n.user_id == "user-001"
    ));

    // 9. 清理
    env.cleanup().await;
}
```

---

## Phase 3: Playwright 前端测试

### 3.1 安装配置
```bash
cd web/admin-ui
npm install -D @playwright/test
npx playwright install
```

### 3.2 Page Object 模式
```typescript
// web/admin-ui/e2e/pages/canvas.page.ts

import { Page, Locator } from '@playwright/test';

export class CanvasPage {
  readonly page: Page;
  readonly canvas: Locator;
  readonly toolbar: Locator;
  readonly saveButton: Locator;

  constructor(page: Page) {
    this.page = page;
    this.canvas = page.locator('[data-testid="rule-canvas"]');
    this.toolbar = page.locator('[data-testid="canvas-toolbar"]');
    this.saveButton = page.locator('[data-testid="save-rule-btn"]');
  }

  async goto(ruleId?: string) {
    if (ruleId) {
      await this.page.goto(`/rules/${ruleId}/edit`);
    } else {
      await this.page.goto('/rules/new');
    }
  }

  async addConditionNode(type: string) {
    await this.toolbar.locator(`[data-node-type="${type}"]`).dragTo(this.canvas);
  }

  async connectNodes(sourceId: string, targetId: string) {
    const source = this.canvas.locator(`[data-node-id="${sourceId}"] .output-handle`);
    const target = this.canvas.locator(`[data-node-id="${targetId}"] .input-handle`);
    await source.dragTo(target);
  }

  async configureCondition(nodeId: string, config: ConditionConfig) {
    await this.canvas.locator(`[data-node-id="${nodeId}"]`).dblclick();
    await this.page.locator('#field-select').selectOption(config.field);
    await this.page.locator('#operator-select').selectOption(config.operator);
    await this.page.locator('#value-input').fill(config.value.toString());
    await this.page.locator('#confirm-btn').click();
  }

  async saveRule() {
    await this.saveButton.click();
    await this.page.waitForResponse(resp =>
      resp.url().includes('/api/rules') && resp.status() === 200
    );
  }
}
```

### 3.3 测试用例示例
```typescript
// web/admin-ui/e2e/specs/canvas-editor.spec.ts

import { test, expect } from '@playwright/test';
import { CanvasPage } from '../pages/canvas.page';

test.describe('画布规则编辑器', () => {
  let canvasPage: CanvasPage;

  test.beforeEach(async ({ page }) => {
    canvasPage = new CanvasPage(page);
    await canvasPage.goto();
  });

  test('创建简单条件规则', async ({ page }) => {
    // 添加条件节点
    await canvasPage.addConditionNode('condition');

    // 配置条件
    await canvasPage.configureCondition('node-1', {
      field: 'order.amount',
      operator: 'gte',
      value: 1000
    });

    // 保存规则
    await canvasPage.saveRule();

    // 验证保存成功
    await expect(page.locator('.success-toast')).toBeVisible();
  });

  test('创建 AND 组合规则', async ({ page }) => {
    // 添加 AND 组节点
    await canvasPage.addConditionNode('and-group');

    // 添加两个条件节点
    await canvasPage.addConditionNode('condition');
    await canvasPage.addConditionNode('condition');

    // 配置条件
    await canvasPage.configureCondition('node-2', {
      field: 'order.amount',
      operator: 'gte',
      value: 500
    });
    await canvasPage.configureCondition('node-3', {
      field: 'user.is_vip',
      operator: 'eq',
      value: true
    });

    // 连接节点到 AND 组
    await canvasPage.connectNodes('node-2', 'node-1');
    await canvasPage.connectNodes('node-3', 'node-1');

    // 保存并验证
    await canvasPage.saveRule();
    await expect(page.locator('.success-toast')).toBeVisible();
  });

  test('加载已有规则并编辑', async ({ page }) => {
    // 创建一个规则用于编辑
    const ruleId = await createTestRule();

    // 加载规则
    await canvasPage.goto(ruleId);

    // 验证画布渲染
    await expect(canvasPage.canvas.locator('[data-node-type]')).toHaveCount.greaterThan(0);

    // 修改条件
    await canvasPage.configureCondition('node-1', {
      field: 'order.amount',
      operator: 'gte',
      value: 2000  // 修改值
    });

    // 保存
    await canvasPage.saveRule();
    await expect(page.locator('.success-toast')).toBeVisible();
  });
});
```

---

## Phase 4: 性能测试

### 4.1 性能测试框架
```rust
// tests/performance/mod.rs

use criterion::{criterion_group, Criterion, Throughput};
use std::time::Duration;

/// 性能测试配置
pub struct PerfTestConfig {
    pub concurrent_users: usize,
    pub duration: Duration,
    pub ramp_up: Duration,
}

/// 性能测试结果
pub struct PerfTestResult {
    pub total_requests: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub throughput: f64,
    pub latency_p50: Duration,
    pub latency_p95: Duration,
    pub latency_p99: Duration,
}
```

### 4.2 压测场景
```rust
// tests/performance/scenarios/concurrent_events.rs

#[tokio::test]
async fn test_concurrent_event_processing() {
    let config = PerfTestConfig {
        concurrent_users: 1000,
        duration: Duration::from_secs(60),
        ramp_up: Duration::from_secs(10),
    };

    let env = TestEnvironment::setup_for_perf().await;

    // 预热
    env.warmup(100).await;

    // 执行压测
    let result = env.run_load_test(config, |user_id| async move {
        let event = PurchaseEvent::random(user_id);
        kafka.send_transaction_event(event).await
    }).await;

    // 断言性能指标
    assert!(result.throughput > 5000.0, "TPS should > 5000");
    assert!(result.latency_p99 < Duration::from_millis(500));
    assert!(result.error_rate() < 0.001);

    // 生成报告
    result.generate_report("concurrent_events").await;
}
```

---

## Phase 5: CI/CD 集成

### 5.1 GitHub Actions 工作流
```yaml
# .github/workflows/test.yml

name: Badge System Tests

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  DATABASE_URL: postgres://badge:badge@localhost:5432/badge_test
  REDIS_URL: redis://localhost:6379
  KAFKA_BROKERS: localhost:9092

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Run unit tests
        run: cargo test --workspace --lib

  integration-tests:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_USER: badge
          POSTGRES_PASSWORD: badge
          POSTGRES_DB: badge_test
        ports:
          - 5432:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
      redis:
        image: redis:7
        ports:
          - 6379:6379
      kafka:
        image: confluentinc/cp-kafka:7.5.0
        ports:
          - 9092:9092
        env:
          KAFKA_ADVERTISED_LISTENERS: PLAINTEXT://localhost:9092
          KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR: 1

    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Run migrations
        run: cargo run -p badge-shared --bin migrate

      - name: Start services
        run: |
          cargo build --workspace
          cargo run -p unified-rule-engine &
          cargo run -p badge-management-service &
          cargo run -p badge-admin-service &
          cargo run -p event-engagement-service &
          cargo run -p event-transaction-service &
          cargo run -p notification-worker &
          sleep 10  # 等待服务启动

      - name: Run E2E tests
        run: cargo test --test e2e -- --test-threads=1

  frontend-tests:
    runs-on: ubuntu-latest
    needs: integration-tests
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'
          cache-dependency-path: web/admin-ui/package-lock.json

      - name: Install dependencies
        run: |
          cd web/admin-ui
          npm ci
          npx playwright install --with-deps

      - name: Run Playwright tests
        run: |
          cd web/admin-ui
          npm run test:e2e

      - uses: actions/upload-artifact@v4
        if: always()
        with:
          name: playwright-report
          path: web/admin-ui/playwright-report/

  performance-tests:
    runs-on: ubuntu-latest
    needs: integration-tests
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Run benchmarks
        run: cargo bench --bench rule_engine_bench -- --save-baseline main

      - name: Run load tests
        run: cargo test --test performance -- --ignored

      - uses: actions/upload-artifact@v4
        with:
          name: benchmark-results
          path: target/criterion/
```

### 5.2 测试报告集成
```yaml
# .github/workflows/test-report.yml

- name: Generate test report
  run: |
    cargo test --workspace -- --format json > test-results.json

- name: Publish test results
  uses: EnricoMi/publish-unit-test-result-action@v2
  with:
    files: test-results.json
```

---

## 任务分解

### 总任务清单

| ID | 任务 | 阶段 | 依赖 | 预估工时 |
|----|------|------|------|----------|
| T01 | 创建测试目录结构 | P1 | - | 1h |
| T02 | 实现测试环境初始化 | P1 | T01 | 2h |
| T03 | 实现 API 客户端 | P1 | T01 | 2h |
| T04 | 实现 Kafka 辅助工具 | P1 | T01 | 2h |
| T05 | 实现数据库验证工具 | P1 | T01 | 2h |
| T06 | 创建标准测试数据集 | P1 | T01 | 3h |
| T07 | 扩展 Mock 服务 | P1 | - | 4h |
| T08 | 基础配置测试套件 | P2 | T02-T06 | 4h |
| T09 | 规则配置测试套件 | P2 | T08 | 5h |
| T10 | 权益配置测试套件 | P2 | T08 | 4h |
| T11 | 事件触发测试套件 | P2 | T07,T09 | 6h |
| T12 | 级联触发测试套件 | P2 | T11 | 3h |
| T13 | 兑换流程测试套件 | P2 | T11 | 3h |
| T14 | 通知系统测试套件 | P2 | T11 | 2h |
| T15 | 逆向场景测试套件 | P2 | T11 | 3h |
| T16 | 数据一致性测试 | P2 | T08-T15 | 2h |
| T17 | Playwright 初始化 | P3 | - | 2h |
| T18 | Page Objects 实现 | P3 | T17 | 4h |
| T19 | 基础配置前端测试 | P3 | T18 | 3h |
| T20 | 画布编辑器前端测试 | P3 | T18 | 5h |
| T21 | 权益配置前端测试 | P3 | T18 | 3h |
| T22 | 全链路前端测试 | P3 | T19-T21 | 4h |
| T23 | 性能测试框架 | P4 | T11 | 3h |
| T24 | 并发事件压测 | P4 | T23 | 2h |
| T25 | 级联性能压测 | P4 | T23 | 2h |
| T26 | 竞争兑换压测 | P4 | T23 | 2h |
| T27 | 性能报告生成 | P4 | T24-T26 | 2h |
| T28 | GitHub Actions 配置 | P5 | T08-T16 | 3h |
| T29 | 测试报告集成 | P5 | T28 | 2h |
| T30 | 文档更新 | P5 | All | 2h |

**总预估工时: ~80h**

---

## 执行建议

1. **Phase 1 (测试基础设施)** 是所有后续工作的基础，建议优先完成
2. **Phase 2 (后端测试)** 和 **Phase 3 (前端测试)** 可以并行进行
3. **Phase 4 (性能测试)** 依赖 Phase 2 完成
4. **Phase 5 (CI/CD)** 可以在 Phase 2 基本完成后开始

请确认是否开始执行？从哪个任务开始？
