# 徽章管理平台 — 问题与缺陷全面分析报告

> 生成日期：2026-02-04
> 基于代码分支：main (commit 749dbdd)
> 测试基线：前端 249/251 通过，后端 826 通过 / 118 ignored

---

## 目录

- [一、Critical — 必须修复的功能缺陷](#一critical--必须修复的功能缺陷)
- [二、High — 严重影响功能](#二high--严重影响功能)
- [三、Medium — 功能缺失/不完整](#三medium--功能缺失不完整)
- [四、前端测试覆盖缺陷](#四前端测试覆盖缺陷)
- [五、后端测试覆盖缺陷](#五后端测试覆盖缺陷)
- [六、生产代码 TODO 清单](#六生产代码-todo-清单)
- [七、数据库 Schema 与应用不一致](#七数据库-schema-与应用不一致)
- [八、安全缺口](#八安全缺口)
- [九、修复优先级建议](#九修复优先级建议)

---

## 一、Critical — 必须修复的功能缺陷

### 1.1 兑换服务未初始化

- **位置**：`crates/badge-admin-service/src/main.rs`
- **问题**：`main.rs` 中从未调用 `set_redemption_service()`，导致 `state.redemption_service` 永远是 `None`
- **影响**：`POST /redemption/redeem` 路由存在但调用必定返回内部错误，兑换功能完全不可用
- **修复方向**：在 `main.rs` 初始化阶段创建 `RedemptionService` 实例并注入 AppState

### 1.2 前后端 API 参数不匹配（3 处）

#### 1.2.1 撤销徽章接口

| 端 | 文件 | 发送/期望 |
|---|---|---|
| 前端 | `web/admin-ui/src/services/member.ts:59-63` | `{ userBadgeId, reason }` |
| 后端 | `handlers/revoke.rs:348-353` | `{ user_id, badge_id, quantity, reason }` |

#### 1.2.2 CSV 上传接口

| 端 | 文件 | 发送/期望 |
|---|---|---|
| 前端 | `web/admin-ui/src/services/grant.ts:237-238` | `multipart/form-data` 文件上传 |
| 后端 | `handlers/grant.rs:509-516` | `JSON { content: "csv_string" }` |

#### 1.2.3 批量任务结果下载接口

| 端 | 文件 | 发送/期望 |
|---|---|---|
| 前端 | `web/admin-ui/src/services/grant.ts:225-230` | 期望 `Blob` 直接下载 |
| 后端 | `handlers/batch_task.rs:321-347` | 返回 `JSON { taskId, resultFileUrl }` |

### 1.3 批量任务无消费者

- **位置**：`handlers/grant.rs:233`、`handlers/revoke.rs:235`
- **问题**：`batch_grant` 和 `batch_revoke` 创建任务记录后标注 `// TODO: 发送消息到任务队列`，但不存在任何 worker 来处理
- **影响**：批量发放/取消任务创建后永远停留 `pending` 状态
- **修复方向**：实现异步任务处理器（轮询 `batch_tasks` 表或 Kafka 消费者）

---

## 二、High — 严重影响功能

### 2.1 操作日志只读不写

- **位置**：`operation_logs` 表（init_schema.sql）、`handlers/operation_log.rs`
- **问题**：handler 只实现了 `list_logs` 查询，没有任何代码往此表写入数据
- **影响**：操作日志表永远为空，无审计追踪能力
- **修复方向**：实现审计中间件，在关键操作（CRUD、发放、取消等）后自动写入日志

### 2.2 六个路由返回桩/假数据

| 路由 | 处理器位置 | 问题 |
|---|---|---|
| `POST /rules/{id}/test` | `handlers/rule.rs:322-338` | 永远返回 `matched: true` 硬编码结果 |
| `POST /rules/test` | `handlers/rule.rs:378-395` | 同上 |
| `POST /grants/preview-filter` | `handlers/grant.rs:581-590` | 永远返回 `{ total: 0, users: [] }` |
| `GET /benefits/sync-logs` | `handlers/benefit.rs:739-752` | 永远返回空列表 |
| `POST /benefits/sync` | `handlers/benefit.rs:757-778` | 仅打日志返回伪造 sync_id |
| `POST /benefits/{id}/link-badge` | `handlers/benefit.rs:783-822` | 验证但不创建关联记录 |

### 2.3 API Key 限流未实现

- **位置**：`middleware/api_key_auth.rs`
- **问题**：`api_key` 表有 `rate_limit` 字段，但中间件不检查限流
- **影响**：外部 API 可无限制调用
- **修复方向**：基于 Redis 实现滑动窗口或令牌桶限流

### 2.4 Token 黑名单缺失

- **位置**：`handlers/auth.rs:276-280`
- **问题**：`POST /auth/logout` 是空操作（no-op），不实现 Token 黑名单
- **影响**：已泄露的 JWT Token 在过期前无法被服务端失效
- **修复方向**：Redis Token 黑名单 + 中间件校验

---

## 三、Medium — 功能缺失/不完整

### 3.1 权益发放全部为模拟实现

| 文件 | 函数 | 问题 |
|---|---|---|
| `benefit/handlers/points.rs:80-100` | `grant_points()` | 模拟积分发放，仅打印日志 |
| `benefit/handlers/points.rs:111-125` | `revoke_points()` | 模拟积分撤销，仅打印日志 |
| `benefit/handlers/points.rs:190-201` | `revoke()` | 使用 0 作为撤销金额占位 |
| `benefit/handlers/coupon.rs:78-105` | `issue_coupon()` | 模拟优惠券发放 |
| `benefit/handlers/coupon.rs:113-128` | `revoke_coupon()` | 模拟优惠券撤销 |
| `benefit/handlers/physical.rs:144-150` | `send_shipment_message()` | 模拟 Kafka 物流消息 |
| `benefit/handlers/physical.rs:246-256` | `check_status()` | 物流状态查询未实现 |

### 3.2 通知系统全部 Mock

- **位置**：`crates/notification-worker/src/sender.rs`
- **问题**：4 个通知发送器（AppPush, SMS, WeChat, Email）都只打日志
- **附加问题**：
  - `notification_configs` 表存在但无 CRUD API
  - `notification_tasks` 表存在但不被使用
  - 模板引擎已创建但未接入（变量前缀下划线 `_template_engine`）

### 3.3 自动权益发放流程缺失

- **位置**：`migrations/20250207_001_auto_benefit.sql`
- **问题**：`auto_benefit_grants` 和 `auto_benefit_evaluation_logs` 表已建，但没有触发/评估代码
- **影响**：自动权益发放在整个系统中没有实际的执行路径

### 3.4 事件类型管理缺失

- **位置**：`migrations/20250202_001_dynamic_rules.sql`
- **问题**：
  - `event_types` 表有预置数据（purchase, checkin, share 等）但无 CRUD API
  - `badge_rules.event_type` 字段在创建/更新规则时不被设置
  - 事件服务如何消费这些配置不明确

### 3.5 配置文件不完整

文件 `config/badge-admin-service.toml` 仅包含 `server.port`、`kafka.consumer_group`、`metrics.port`。

**缺失的配置项**：

| 缺失项 | 当前状态 |
|---|---|
| `database.*` | 依赖 `AppConfig::load` 默认值 |
| `redis.*` | 同上 |
| CORS origins | 仅通过环境变量 |
| JWT secret / expires | 仅通过环境变量 |
| 登录安全策略 | 硬编码在 auth.rs |
| gRPC 连接地址 | 仅通过环境变量 |

**`.env.example` 缺失的环境变量**：

| 变量 | 使用位置 |
|---|---|
| `BADGE_CORS_ORIGINS` | `main.rs:80` |
| `BADGE_MAX_LOGIN_ATTEMPTS` | `handlers/auth.rs:162` |
| `BADGE_LOCK_DURATION_MINS` | `handlers/auth.rs:166` |
| `BADGE_MANAGEMENT_GRPC_ADDR` | `main.rs:60` |
| `BADGE_ENV` | `main.rs:38` |

---

## 四、前端测试覆盖缺陷

### 4.1 零单元测试（高）

- 47 个页面组件、所有 hooks 和 services 完全无单元测试
- `package.json` 中未安装 Jest、Vitest 或 `@testing-library/react`
- 所有测试都是 Playwright E2E 测试

### 4.2 24 处空断言（高）

5 个 spec 文件中共有 24 处 `expect(true).toBeTruthy()`，这些断言永远通过：

| 文件 | 出现次数 |
|---|---|
| `e2e/specs/benefits-extended.spec.ts` | 11 |
| `e2e/specs/benefit-sync.spec.ts` | 6 |
| `e2e/specs/categories.spec.ts` | 3 |
| `e2e/specs/series.spec.ts` | 2 |
| `e2e/specs/badge-crud.spec.ts` | 2 |

受影响的场景：编辑按钮存在、删除按钮存在、状态切换开关存在、新建权益按钮可见、同步按钮可见、导出按钮、权益搜索等。

### 4.3 核心业务流程测试不足（高）

#### 手动发放流程

`pages/grants/Manual.tsx` 实现了完整的 4 步发放流程，但 E2E 测试仅验证页面是否加载。**未覆盖**：

- 用户搜索和选择流程
- 步骤前进/后退导航
- 表单验证（如未选用户不能下一步）
- 徽章选择和数量设置
- 确认页面的汇总信息展示
- 发放执行和结果展示（成功/失败/部分成功）

#### 会员搜索页面

`pages/members/Search.tsx` 实现了搜索、徽章墙、状态筛选、视图切换、详情弹窗、撤销功能，但测试仅验证页面加载。

#### 规则编辑器

`pages/rules/Canvas.tsx` 使用 ReactFlow 实现可视化编辑器，测试仅验证画布加载和节点可见。**未覆盖**：节点拖拽、连线验证、撤销/重做、快捷键、保存发布。

### 4.4 移动端测试全部跳过（中）

5 个主要 spec 文件在 mobile 项目下 `test.skip`：

- `categories.spec.ts`
- `series.spec.ts`
- `badge-crud.spec.ts`
- `rule-editor.spec.ts`
- `file-upload.spec.ts`

仅 `dashboard.spec.ts` 有 1 个响应式测试（768px 和 375px 视口）。

### 4.5 零 a11y 测试（中）

- 未安装任何 a11y 测试工具（无 `@axe-core/playwright`、`pa11y`、`lighthouse`）
- 无色彩对比度检测
- 无键盘导航测试
- 无屏幕阅读器兼容性测试

### 4.6 错误状态未覆盖（中）

以下场景完全缺失：

- 网络错误处理（后端不可用时的 UI 表现）
- Token 过期时的自动刷新
- 各列表页无数据时的空状态展示
- 大数据量下的分页切换
- 表单重复提交（连续快速点击）
- CSV 大文件上传限制验证

### 4.7 路由覆盖度

| 覆盖状态 | 数量 | 路由 |
|---|---|---|
| 完整覆盖 | 15 | 大部分路由 |
| 弱覆盖（仅页面加载） | 4 | `/grants/manual`, `/grants/logs`, `/members/search`, `/rules/:ruleId/edit` |
| 未覆盖 | 1 | 404 页面 |

---

## 五、后端测试覆盖缺陷

### 5.1 零测试的关键模块（高）

| 模块 | 文件 | 影响 |
|---|---|---|
| `auth.rs` | `handlers/auth.rs` | 认证核心：密码验证、账户锁定、Token 刷新无测试 |
| `api_key.rs` | `handlers/api_key.rs` | 密钥生成 `bk_` + SHA256 哈希无测试 |

### 5.2 error.rs 测试覆盖不足（高）

20 个错误变体中仅 4 个已测试（Validation, BadgeNotFound, BadgeAlreadyPublished, Internal）。

**完全未测试的 16 个变体**：
`Unauthorized`, `Forbidden`, `InvalidCredentials`, `UserDisabled`, `UserLocked`, `UserNotFound`, `CategoryNotFound`, `SeriesNotFound`, `RuleNotFound`, `TaskNotFound`, `DependencyNotFound`, `BenefitNotFound`, `NotFound`, `InvalidRuleJson`, `FileProcessingError`, `InsufficientStock`, `InsufficientUserBadge`, `Database`, `Redis`

**未测试的关键行为**：
- `IntoResponse` 实现（JSON 响应体格式、系统错误的日志脱敏逻辑）
- `From<validator::ValidationErrors>` 转换
- `From<badge_management::BadgeError>` 转换

### 5.3 全部 5 个事件吞吐量性能测试为空壳（高）

文件：`tests/performance/scenarios/event_throughput.rs`

| 测试 | 状态 |
|---|---|
| `test_transaction_event_throughput` | Kafka 发送逻辑被注释 |
| `test_event_processing_latency` | 仅有 TODO 注释 |
| `test_rule_reload_performance` | 检查逻辑被注释 |
| `test_dlq_processing` | 完全空（仅 3 行 TODO） |
| `test_backpressure_handling` | 发送和验证均被注释 |

### 5.4 Handler 单元测试仅覆盖序列化/验证（中）

所有 16 个有测试的 handler 模块，测试内容仅包括：
- 请求结构体的 `Validate` trait 验证
- DTO 的 `Serialize`/`Deserialize` 往返
- 默认值和字段转换

**未覆盖**：实际数据库交互、handler 逻辑（mock DB 或 in-memory DB 测试）。

### 5.5 Handler 未测试的错误路径

| Handler | 未测试的错误路径 |
|---|---|
| `grant.rs` | 徽章不存在、库存不足、事务失败、CSV 为空/缺列 |
| `revoke.rs` | 用户徽章不足 (`InsufficientUserBadge`)、徽章不存在 |
| `auth.rs` | 用户禁用、用户锁定、密码错误累计锁定逻辑、Claims 解析失败 |
| `api_key.rs` | 删除/regenerate/toggle 不存在的 key |
| `redemption.rs` | 权益不存在、有订单的规则删除拒绝、RedemptionService 未配置 |
| `rule.rs` | 已启用的规则重复发布、已禁用的规则重复禁用 |

### 5.6 gRPC 负载测试为空壳（中）

`tests/performance/scenarios/rule_engine.rs:243` — `test_grpc_rule_engine_load` 仅有注释"需要 tonic 客户端"。

### 5.7 E2E 中被注释的验证逻辑

| 文件 | 位置 | 问题 |
|---|---|---|
| `rule_config.rs:389-391` | `test_rule_with_global_quota` | `global_quota` 断言被注释（API 不支持该字段） |
| `rule_config.rs:444` | `test_rule_hot_reload` | 事件发送验证被标记 TODO |
| `data/scenarios.rs:300` | `setup_complete_scenario` | 徽章-权益关联步骤缺失 |

### 5.8 各 Crate 测试覆盖总览

| Crate | 单元测试 | 集成测试 | 评价 |
|---|---|---|---|
| `badge-admin-service` | 16/18 handler 有测试 | 无 | **中** — auth/api_key 无测试，仅覆盖验证和序列化 |
| `badge-management-service` | 有 | `badge_flow_test.rs` | **良好** |
| `unified-rule-engine` | 有 | `template_integration.rs` | **良好** |
| `shared` | 15 文件有测试 | `observability_integration.rs` | **中** — `telemetry.rs` 空模块, `rules/validator.rs` 和 `rules/loader.rs` 无测试 |
| `event-engagement-service` | 有 | 无 | **中** — 仅类型和序列化测试 |
| `event-transaction-service` | 有 | 无 | **中** — 同上 |
| `notification-worker` | 有 | 无 | **良好** — 模板渲染覆盖充分 |
| `mock-services` | 有 | 无 | **良好** |
| `proto` | **无** | **无** | **无测试** — 仅 generated 代码 |

### 5.9 占位模块

| 文件 | 问题 |
|---|---|
| `shared/src/telemetry.rs` | 仅 1 行注释 `//! 模块占位`，无任何代码 |
| `shared/src/observability/middleware.rs:168-226` | `grpc::tracing_interceptor` 和 `kafka::inject/extract_trace_context` 均为占位实现 |

---

## 六、生产代码 TODO 清单

共 23 处 TODO，其中 12 处在权益处理器中。

### 高优先级（生产代码）

| 文件 | 行号 | 内容 |
|---|---|---|
| `handlers/rule.rs` | 330 | `对接 rule-engine gRPC 服务进行真实规则评估` |
| `handlers/rule.rs` | 386 | 同上 |
| `handlers/grant.rs` | 233 | `发送消息到任务队列触发异步处理` |
| `handlers/revoke.rs` | 235 | `投递至任务队列异步处理` |
| `benefit/handlers/physical.rs` | 143 | `替换为实际的 Kafka producer 调用` |
| `benefit/handlers/physical.rs` | 247 | `实现实际的状态查询逻辑` |
| `benefit/handlers/points.rs` | 79 | `替换为实际的积分服务 SDK 调用` |
| `benefit/handlers/points.rs` | 110 | `替换为实际的积分服务 SDK 调用` |
| `benefit/handlers/points.rs` | 193 | `实际实现需要从数据库查询发放金额` |
| `benefit/handlers/coupon.rs` | 77 | `替换为实际的优惠券服务 SDK 调用` |
| `benefit/handlers/coupon.rs` | 112 | `替换为实际的优惠券服务 SDK 调用` |
| `benefit/handlers/coupon.rs` | 197 | `实际实现需要从数据库查询 coupon_id` |

### 中优先级（测试代码）

| 文件 | 内容 |
|---|---|
| `tests/e2e/suites/rule_config.rs:389` | `API CreateRuleRequest 尚未支持 global_quota 字段` |
| `tests/e2e/suites/rule_config.rs:444` | `需要发送事件并验证处理结果` |
| `tests/e2e/data/scenarios.rs:300` | `关联徽章和权益（需要对应 API）` |
| `tests/performance/scenarios/event_throughput.rs` | 10 处 TODO（空壳测试） |

---

## 七、数据库 Schema 与应用不一致

### 7.1 存在但从未被应用使用的表

| 表名 | 迁移文件 | 问题 |
|---|---|---|
| `notification_configs` | `init_schema.sql:304` | 无 CRUD API，notification-worker 也不读取 |
| `notification_tasks` | `init_schema.sql:332` | 无代码使用 |
| `auto_benefit_evaluation_logs` | `auto_benefit.sql:57` | 无代码使用 |
| `auto_benefit_grants` | `auto_benefit.sql:11` | 无代码使用 |
| `event_types` | `dynamic_rules.sql:7` | 有预置数据但无 CRUD API |

### 7.2 代码引用但不存在的表

| 表名 | 引用位置 | 问题 |
|---|---|---|
| `badge_benefit_links` | `handlers/benefit.rs:819` | 注释中引用但迁移文件中不存在 |
| `benefit_sync_logs` | `handlers/benefit.rs:739` | 注释说"可能不存在"，实际确实不存在 |

### 7.3 Schema 中定义但应用不使用的字段

| 表.字段 | 问题 |
|---|---|
| `api_key.rate_limit` | 中间件不检查限流 |
| `badge_rules.event_type` | handlers 不读写 |
| `badge_rules.rule_code` | handlers 不读写 |
| `badge_rules.global_quota` / `global_granted` | handlers 不读写 |
| `badge_rules.template_id` / `template_version` / `template_params` | 仅 `create_rule_from_template` 设置，列表/详情查询不返回 |
| `admin_user.created_by` | 创建用户时不设置 |
| `benefits.remaining_stock` | 无扣减逻辑，库存只增不减 |

---

## 八、安全缺口

### 8.1 认证中间件公开路由

文件：`middleware/auth.rs:29-34`

```
/api/admin/auth/login   — 正确
/api/admin/health       — 实际不存在（无害但不准确）
/api/v1/                — 外部 API 路由（使用 API Key 认证）
/health                 — 正确
```

`/api/v1/` 使用 `starts_with` 匹配跳过 JWT 认证。如果该前缀下新增路由忘记加 API Key 中间件，会导致无认证暴露。

### 8.2 密码策略不足

- `ResetPasswordRequest` 只验证长度（6-100），无复杂度要求
- 无密码历史记录机制
- 默认用户密码（admin123, operator123, viewer123）写在迁移文件中

### 8.3 生产代码中的 unwrap() 调用

| 文件 | 行号 | 问题 |
|---|---|---|
| `handlers/benefit.rs` | 658, 663 | `.and_hms_opt(0,0,0).unwrap()` 日期转换 |
| `handlers/badge.rs` | 57-62 | `serde_json::from_value().unwrap_or(...)` 静默吞掉反序列化错误 |
| `middleware/api_key_auth.rs` | 118 | 权限 JSON 解析失败返回空列表 |
| `system_role.rs` | 457 | 角色查询失败默认 false，可能跳过系统角色保护检查 |

### 8.4 CORS 生产环境无强制限制

`BADGE_CORS_ORIGINS=*` 时允许所有来源，但不像 JWT secret 那样在 `BADGE_ENV=production` 时 panic。

---

## 九、修复优先级建议

### P0 — 阻塞核心功能

| # | 问题 | 涉及文件 |
|---|---|---|
| 1 | 修复前后端 API 参数不匹配（撤销、CSV 上传、结果下载） | `member.ts`, `grant.ts`, `revoke.rs`, `grant.rs`, `batch_task.rs` |
| 2 | 初始化 `redemption_service` | `main.rs` |
| 3 | 实现批量任务消费者 worker | 新建 worker crate 或 background task |

### P1 — 安全与审计

| # | 问题 | 涉及文件 |
|---|---|---|
| 4 | 为 `auth.rs` 和 `api_key.rs` 补充单元测试 | `handlers/auth.rs`, `handlers/api_key.rs` |
| 5 | 实现操作日志写入机制（审计中间件） | 新建 `middleware/audit.rs` |
| 6 | 实现 API Key 限流 | `middleware/api_key_auth.rs` |
| 7 | 修复 24 处 `expect(true).toBeTruthy()` 空断言 | 5 个 spec 文件 |

### P2 — 功能完善

| # | 问题 | 涉及文件 |
|---|---|---|
| 8 | 规则测试接口对接 rule-engine gRPC | `handlers/rule.rs` |
| 9 | 权益同步和链接的真实实现 | `handlers/benefit.rs` |
| 10 | 补充手动发放、会员搜索的 E2E 测试 | 新建/扩展 spec 文件 |
| 11 | 引入前端单元测试框架（Vitest） | `package.json`, 新建测试文件 |
| 12 | 补齐 `error.rs` 测试覆盖（20 个变体） | `error.rs` tests module |
| 13 | 填充 `event_throughput.rs` 性能测试 | `tests/performance/scenarios/event_throughput.rs` |

### P3 — 技术债务

| # | 问题 | 涉及文件 |
|---|---|---|
| 14 | 完善 `.env.example` 环境变量文档 | `docker/.env.example` |
| 15 | 清理 5 张未使用的数据库表或补充对应功能 | 迁移文件 |
| 16 | 修复移动端测试跳过问题 | 5 个 spec 文件 |
| 17 | 引入 a11y 测试 | `playwright.config.ts`, 新建 spec |
| 18 | 清理 `shared/telemetry.rs` 空模块 | `shared/src/telemetry.rs` |
