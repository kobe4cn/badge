# Podman 全链路联调与压测计划

> 目标：在 Podman 环境下完成“事件 → 规则评估 → 徽章发放 → 兑换/权益/通知（可 mock） → 管理端查询”的端到端联调，并形成可重复执行的压测流程与脚本。

## 1. 适用范围
- 基础设施：Postgres、Redis、Kafka、Elasticsearch（Podman Compose）。
- 后端服务：
  - rule-engine
  - badge-management-service
  - badge-admin-service
  - event-engagement-service
  - event-transaction-service
  - notification-worker
- 前端：`web/admin-ui`（禁用 mock）。
- 外部依赖：权益发放/通知可 mock，不作为阻断项。

## 2. 前置条件
- 安装：`podman`、`podman compose`、Rust 工具链、`pnpm`。
- 确保 `.env` 与 `config/*.toml` 已配置正确（DB、Kafka、端口等）。
- 已存在默认管理员账号：`admin / admin123`（由 `migrations/20250208_001_auth_rbac.sql` 创建）。

## 3. 联调流程（全链路）

### 3.1 基础设施与数据
1. 启动基础设施（Podman Compose）。
2. 初始化 Kafka topics。
3. 执行数据库迁移与测试数据。

### 3.2 启动服务
1. 启动 6 个后端服务（Rule 引擎 / 管理 / 事件消费 / 通知）。
2. 启动 Admin UI（`VITE_DISABLE_MOCK=true`）。

### 3.3 冒烟与回归场景
- 冒烟检查：
  - `http://localhost:8080/health`
  - `POST /api/admin/auth/login` 成功返回 token
- 事件驱动链路：
  - 使用 `mock-services generate` 发送 `checkin/purchase/share` 事件到 Kafka
  - 通过管理端查询用户徽章确认发放
- 兑换链路：
  - 使用 `redemptions/Manual` 手动兑换（验证规则与权益记录）
- 通知链路（可 mock）：
  - 创建通知配置 → 触发任务 → 任务列表可见

## 4. 压测流程（基于 Kafka 事件）

### 4.1 压测思路
- 通过 `mock-services generate` 向 Kafka 批量发送事件（`purchase` 或 `checkin`）。
- 观察：
  - 事件发送吞吐（客户端侧）
  - 处理完成延迟（抽样查询用户徽章）
  - 可观测性指标（/metrics）

### 4.2 建议的分级压测
- **Level 1（基线）**：1k 用户 × 1 事件（并发 10）
- **Level 2（负载）**：10k 用户 × 1 事件（并发 50）
- **Level 3（压力）**：50k 用户 × 1 事件（并发 100）

### 4.3 通过标准（示例，可按实际调整）
- 成功率 ≥ 99%
- 发放结果可在 3s 内查询到（P95）
- 批处理无严重堆积（`batch_task_pending_count`、消费滞后可控）

## 5. 观测与产出
- 服务日志：`.run/logs/*.log`
- 指标：各服务 `/metrics`（默认端口见 `config/*.toml`）
- 结果输出：
  - 压测耗时与吞吐
  - 抽样用户徽章验证

## 6. 执行脚本
- 联调脚本：`scripts/run-podman-e2e.sh`
- 压测脚本：`scripts/run-podman-perf.sh`

## 7. 清理
- 停止服务：联调脚本 `down` 子命令
- 关闭基础设施：`make infra-down`
