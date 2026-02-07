# 徽章系统管理功能完整度对照表

> 依据：`specs/instructions.md`、`docs/plans/`、前端 `web/admin-ui/` 与后端 `crates/` 当前实现
> 目的：逐条对照需求与实现，标注完成度与缺口，便于后续补齐与联调

## 对照说明
- **状态**：`已闭环` / `部分` / `缺失`
- **证据**：关键文件或模块位置（便于定位）
- **缺口**：与需求文档不一致或未覆盖的点

---

## 1. 徽章创建配置

| 需求条目 | 状态 | 证据（前端/后端/DB） | 主要缺口 |
|---|---|---|---|
| 1.1 三层徽章结构（分类/系列/徽章） | 已闭环 | UI：`web/admin-ui/src/pages/badges/{Categories,Series,Definitions}.tsx`；API：`crates/badge-admin-service/src/handlers/{category,series,badge}.rs`；DB：`migrations/20250128_001_init_schema.sql` | — |
| 1.1.2 徽章类型可配置 | 已闭环 | UI：`web/admin-ui/src/pages/badges/components/BadgeForm.tsx`；DB：`badges.badge_type` | 类型枚举固定（NORMAL/LIMITED/ACHIEVEMENT/EVENT），未提供动态扩展 |
| 1.1.3 基本属性配置（名称、层级、类型、唯一ID、描述、状态、规则、权益等） | 部分 | UI：徽章表单、详情抽屉；API：badge CRUD | 缺少“业务唯一编码”；规则/权益关联在徽章本体中未完整体现（规则与权益分表） |
| 1.2 发放对象设置（账号注册人/实际使用人） | 部分 | 手动发放 UI：`web/admin-ui/src/pages/grants/Manual.tsx`；DTO：`crates/badge-admin-service/src/dto/request.rs` | 仅手动发放覆盖，规则/批量/事件发放仍以 `user_id` 为主 |
| 1.3 获取徽章任务设置（多场景事件） | 部分 | 规则画布+事件类型表：`web/admin-ui/src/pages/rules/*`、`web/admin-ui/src/services/rule.ts`、`crates/badge-admin-service/src/handlers/rule.rs`、`migrations/20250202_001_dynamic_rules.sql` | 事件类型列表固定，缺业务扩展入口 |
| 1.3.2 多维度规则配置（AND/OR，3层嵌套） | 部分 | 规则画布支持逻辑节点 | 未验证 3 层嵌套的持久化与执行一致性 |
| 1.3.3~1.3.6 元数据类型与操作符/输入控件 | 部分 | `web/admin-ui/src/pages/rules/components/nodes/ConditionNodeConfig.tsx` | 输入控件未按类型区分（数值/日期/布尔/列表等），仍以文本为主 |
| 1.3.7 规则配置可独立部署/复用 | 部分 | `crates/unified-rule-engine` 已独立 | 管理端无独立规则平台入口与共享配置能力 |
| 1.4 获取徽章时间段（固定/开口/永久） | 部分 | `badge_rules.start_time/end_time` | UI 未暴露 start/end 配置；“开口区间”需 UI/校验补齐 |
| 1.5 获取次数上限（最大/无限） | 部分 | `badge_rules.max_count_per_user` | UI 未暴露此配置 |
| 1.6 持有有效期（固定/相对/永久） | 已闭环 | `badges.validity_config` + UI `BadgeForm` + 计算逻辑 `grant_service` | — |
| 1.7 徽章素材（图片/动效/3D/素材库） | 部分 | 素材库 UI：`web/admin-ui/src/pages/assets/Library.tsx`；API：`crates/badge-admin-service/src/handlers/asset.rs`；DB：`migrations/20250216_001_asset_library.sql`；徽章字段：`badges.assets` | 徽章表单与素材库的选用/关联未打通（当前仍以直接 URL/上传为主） |

---

## 2. 徽章发放配置

| 需求条目 | 状态 | 证据（前端/后端/DB） | 主要缺口 |
|---|---|---|---|
| 2.1 实时事件触发自动发放 | 已闭环（主流程） | 事件服务 `event-engagement/transaction` + 动态规则加载 `crates/shared/src/rules/*` | 规则创建入口不完整影响可用性 |
| 2.1.2 徽章获得情况触发 | 已闭环（级联） | 依赖配置与级联评估：`Dependencies.tsx`、`CascadeEvaluator` | 复杂互斥/优先级冲突策略的运营可视化不足 |
| 2.2 定时触发发放 | 部分 | 调度 Worker：`crates/badge-admin-service/src/worker/scheduled_task_worker.rs`；调度字段：`migrations/20250215_001_scheduled_tasks.sql`；UI：`web/admin-ui/src/pages/grants/components/CreateBatchTaskModal.tsx` | 创建任务时未写入 `schedule_type/scheduled_at/cron_expression`，前端调度配置未落库，端到端未闭环 |
| 2.3 手动发放（单人） | 已闭环 | UI `grants/Manual.tsx`；API `handlers/grant.rs` | — |
| 2.3 批量发放（CSV/大规模） | 部分 | UI 批量任务：`web/admin-ui/src/pages/grants/components/CreateBatchTaskModal.tsx`；API：`crates/badge-admin-service/src/handlers/batch_task.rs`；Worker：`crates/badge-admin-service/src/worker/batch_task_worker.rs` | CSV 上传接口返回结构与前端不匹配（`upload_user_csv` vs `CsvParseResult`）；`file_url` 实际存储/下载链路未闭环；worker 忽略 `quantity`（固定 +1）；50 万级压测未验证 |
| 2.4 发放通知 | 部分 | 通知配置/任务 UI：`web/admin-ui/src/pages/notifications/{Configs,Tasks}.tsx`；API：`crates/badge-admin-service/src/handlers/notification.rs` | 发送执行与重发链路未见（依赖外部系统/worker，可暂用 mock） |
| 2.5 发放记录 | 部分 | 日志/导出已实现 | 批量失败清单/重试已在任务层支持，需确认与发放记录视图的联动与运营闭环 |

---

## 3. 徽章取消/过期配置

| 需求条目 | 状态 | 证据（前端/后端/DB） | 主要缺口 |
|---|---|---|---|
| 3.1 自动取消（条件不再满足） | 部分 | API：`crates/badge-admin-service/src/handlers/revoke.rs`（`auto_revoke`） | 外部触发链路未见（账号注销/身份变更等事件接入需确认） |
| 3.2 手动取消（单人） | 已闭环 | 会员徽章撤销 UI、`handlers/revoke.rs` | — |
| 3.2.2/3.2.3 批量取消 | 部分 | UI：`web/admin-ui/src/pages/revokes/Batch.tsx`；API：`crates/badge-admin-service/src/handlers/revoke.rs`；Worker：`crates/badge-admin-service/src/worker/batch_task_worker.rs` | CSV 上传/`file_url` 链路未闭环（同批量发放）；前端仍使用占位 `uploaded://` |
| 3.3 徽章过期 | 已闭环 | 过期 Worker：`crates/badge-admin-service/src/worker/expire_worker.rs`；启动：`crates/badge-admin-service/src/main.rs` | — |
| 3.4 取消/过期通知 | 部分 | 过期通知任务写入：`crates/badge-admin-service/src/worker/expire_worker.rs`；通知配置/任务 UI 已有 | 取消通知/重发执行链路未闭环（允许 mock） |
| 3.5 取消/过期记录 | 部分 | 账本与日志有记录 | 记录导出与筛选维度不足 |

---

## 4. 徽章兑换配置

| 需求条目 | 状态 | 证据（前端/后端/DB） | 主要缺口 |
|---|---|---|---|
| 4.1 兑换有效期（固定/开口/相对/永久） | 部分 | `badge_redemption_rules.start_time/end_time` + API `crates/badge-admin-service/src/handlers/redemption.rs` | `relative_days` 字段已存在但管理端表单未提供入口（需确认是否隐藏/遗漏） |
| 4.2 兑换频次（时间维度/账号维度） | 部分 | `frequency_config` 支持日/周/月/年/用户上限（UI：`web/admin-ui/src/pages/redemptions/components/RedemptionRuleForm.tsx`） | 缺“每 X 天/周/月/年”类周期化参数 |
| 4.3 兑换次数上限/无限 | 部分 | `frequency_config` | 无限次数需前端表达与文案补齐 |
| 4.4 单徽章/多徽章兑换 + 手动/自动 | 已闭环 | 规则支持多徽章 + `auto_redeem`；手动兑换 UI：`web/admin-ui/src/pages/redemptions/Manual.tsx`；自动兑换缓存：`crates/badge-management-service/src/auto_benefit/rule_cache.rs` | — |
| 4.5 兑换通知与记录 | 部分 | 兑换记录 UI：`web/admin-ui/src/pages/redemptions/Records.tsx`；API：`crates/badge-admin-service/src/handlers/redemption.rs` | 通知触发/重发链路未闭环（允许 mock） |

---

## 5. 权益与自动权益

| 需求条目 | 状态 | 证据（前端/后端/DB） | 主要缺口 |
|---|---|---|---|
| 权益管理（CRUD） | 已闭环 | UI `benefits/List.tsx`、API `handlers/benefit.rs`、表 `benefits` | — |
| 权益发放记录 | 已闭环 | UI `benefits/Grants.tsx`、表 `benefit_grants` | 失败重发入口缺失 |
| 自动权益发放 | 已闭环（后端） | `auto_benefit_*` 模块 + 迁移 `20250207_001_auto_benefit.sql` | 管理端可视化/配置入口缺失 |

---

## 6. 系统与权限

| 需求条目 | 状态 | 证据（前端/后端/DB） | 主要缺口 |
|---|---|---|---|
| 登录/权限/RBAC | 已闭环 | `web/admin-ui/src/pages/auth/*`、`crates/badge-admin-service/src/handlers/auth.rs`、`migrations/20250208_001_auth_rbac.sql` | — |
| 用户/角色/API Key 管理 | 已闭环 | `web/admin-ui/src/pages/system/*`、`handlers/{system_user,system_role,api_key}.rs` | — |

---

## 7. 统计/会员视图

| 需求条目 | 状态 | 证据（前端/后端/DB） | 主要缺口 |
|---|---|---|---|
| 运营仪表盘 | 已闭环 | `web/admin-ui/src/pages/dashboard/index.tsx`、`handlers/stats.rs` | — |
| 会员徽章查询 | 已闭环 | `web/admin-ui/src/pages/members/Search.tsx`、`handlers/user_view.rs` | — |

---

## 关键断点（优先级建议）

1. **批量发放/撤销链路不闭环**：CSV 上传返回结构与前端不一致；`file_url` 存储/下载未打通；worker 忽略 `quantity`；50 万级压测未验证。
2. **定时任务端到端缺口**：调度字段与 Worker 已有，但创建任务未写入 `schedule_type/scheduled_at/cron_expression`，前端调度配置不生效。
3. **通知执行链路不完整**：配置/任务列表已具备，但发送/重发依赖外部系统或 worker 未见（允许 mock，但生产需接入）。
4. **外部事件触发接入**：`auto_revoke` API 已有，但账号注销/身份变更等事件源的调用链路需确认。
5. **可观测性与运维流程**：metrics 代码已具备，告警/仪表盘/灾备 SOP 需运维侧确认与演练。

---

## 附：联调测速入口索引（Podman）

- 基础设施：`make infra-up`、`make kafka-init`、`make db-setup`
- 后端：`make dev-backend`
- 前端（真实 API）：`VITE_DISABLE_MOCK=true pnpm run dev`
- 性能用例：`tests/performance/scenarios/e2e_benchmark.rs`
- 管理端集成测试：`web/admin-ui/e2e/INTEGRATION_TEST.md`
