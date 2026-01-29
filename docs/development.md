# 开发者指南

本文档面向参与徽章系统开发的工程师，包含开发环境设置、代码结构说明和开发规范。

## 开发环境设置

### 前置依赖

```bash
# 安装 Rust (使用 rustup)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 验证版本 (需要 1.85+)
rustc --version

# 安装 pnpm (前端包管理器)
npm install -g pnpm

# 安装 Docker
# macOS: brew install --cask docker
# Linux: 参考 https://docs.docker.com/engine/install/
```

### 一键设置

```bash
# 运行设置脚本
make setup
```

脚本会自动完成：
1. 启动基础设施（PostgreSQL、Redis、Kafka、Elasticsearch）
2. 运行数据库迁移
3. 安装前端依赖
4. 构建 Rust 项目

### 手动设置

```bash
# 1. 启动基础设施
make infra-up

# 2. 等待服务就绪（约 10 秒）
sleep 10

# 3. 运行数据库迁移
make db-migrate

# 4. 安装前端依赖
cd web/admin-ui && pnpm install && cd ../..

# 5. 构建项目
make build
```

### 启动开发服务

```bash
# 终端 1: 启动后端
make dev-backend

# 终端 2: 启动前端
make dev-frontend
```

---

## 代码结构说明

### Rust Workspace

```
crates/
├── proto/                        # Protocol Buffers 定义
│   ├── src/
│   │   ├── badge.proto          # 徽章服务 Proto
│   │   └── rule_engine.proto    # 规则引擎 Proto
│   └── build.rs                 # Proto 编译脚本
│
├── shared/                       # 共享库
│   └── src/
│       ├── config.rs            # 配置管理
│       ├── error.rs             # 错误类型定义
│       └── utils.rs             # 工具函数
│
├── unified-rule-engine/          # 规则引擎服务
│   └── src/
│       ├── main.rs              # 服务入口
│       ├── lib.rs               # 库导出
│       ├── evaluator.rs         # 规则评估器
│       ├── parser.rs            # 规则解析器
│       └── service.rs           # gRPC 服务实现
│
├── badge-management-service/     # C端徽章服务
│   └── src/
│       ├── main.rs              # 服务入口
│       ├── repository.rs        # 数据访问层
│       └── service.rs           # gRPC 服务实现
│
├── badge-admin-service/          # B端管理后台
│   └── src/
│       ├── main.rs              # 服务入口
│       ├── routes.rs            # 路由定义
│       ├── handlers/            # HTTP 处理器
│       ├── dto/                 # 数据传输对象
│       ├── models/              # 数据模型
│       ├── state.rs             # 应用状态
│       └── error.rs             # 错误处理
│
├── event-engagement-service/     # 互动事件处理
│   └── src/
│       ├── main.rs              # 服务入口
│       └── consumer.rs          # Kafka 消费者
│
├── event-transaction-service/    # 交易事件处理
│   └── src/
│       ├── main.rs              # 服务入口
│       └── consumer.rs          # Kafka 消费者
│
├── notification-worker/          # 通知消费者
│   └── src/
│       └── main.rs              # 服务入口
│
└── mock-services/                # Mock 服务
    └── src/
        ├── main.rs              # CLI 入口
        ├── cli.rs               # 命令行定义
        ├── server.rs            # Mock HTTP 服务
        ├── generators/          # 事件生成器
        └── scenarios/           # 测试场景
```

### 前端结构

```
web/admin-ui/src/
├── App.tsx                       # 应用入口
├── main.tsx                      # 渲染入口
├── components/                   # 可复用组件
│   ├── RuleEditor/              # 规则可视化编辑器
│   └── BadgeCard/               # 徽章卡片
├── pages/                        # 页面组件
│   ├── Dashboard/               # 首页仪表盘
│   ├── BadgeList/               # 徽章列表
│   ├── RuleEditor/              # 规则编辑
│   └── UserView/                # 用户视图
├── services/                     # API 服务
│   ├── api.ts                   # API 客户端
│   ├── badge.ts                 # 徽章 API
│   └── rule.ts                  # 规则 API
├── stores/                       # 状态管理
│   └── index.ts                 # Zustand Store
├── hooks/                        # 自定义 Hooks
├── types/                        # TypeScript 类型
├── utils/                        # 工具函数
├── config/                       # 配置
└── theme/                        # 主题配置
```

---

## 测试

### 运行测试

```bash
# 运行所有测试
make test

# 运行测试（详细输出）
make test-verbose

# 运行特定 crate 的测试
cargo test -p unified-rule-engine

# 运行特定测试
cargo test -p badge-admin-service test_routes

# 运行集成测试
cargo test --test integration
```

### 测试覆盖率

```bash
# 安装 tarpaulin
cargo install cargo-tarpaulin

# 生成覆盖率报告
cargo tarpaulin --workspace --out Html
```

### 基准测试

```bash
# 运行规则引擎基准测试
cargo bench -p unified-rule-engine

# 运行负载测试
cargo bench --bench load_test
```

### Mock 服务

Mock 服务用于本地开发和测试：

```bash
# 启动 Mock HTTP 服务
cargo run -p mock-services -- server --port 3000

# 生成测试事件
cargo run -p mock-services -- generate --event-type purchase --user-id user_001 --count 10

# 运行测试场景
cargo run -p mock-services -- scenario --name vip-upgrade --user-id user_001

# 填充测试数据
cargo run -p mock-services -- populate --users 100 --orders medium
```

---

## 代码规范

### Rust 规范

**格式化：**
```bash
# 格式化所有代码
make fmt

# 检查格式
make fmt-check
```

**Lint 检查：**
```bash
# 运行 Clippy
make lint
```

**命名约定：**
- 模块名：`snake_case`
- 结构体/枚举：`PascalCase`
- 函数/变量：`snake_case`
- 常量：`SCREAMING_SNAKE_CASE`

**错误处理：**
```rust
// 使用 thiserror 定义错误类型
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("Badge not found: {0}")]
    NotFound(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

// 使用 Result 返回错误
pub async fn get_badge(id: &str) -> Result<Badge, ServiceError> {
    // ...
}
```

**日志规范：**
```rust
use tracing::{info, warn, error, debug, instrument};

#[instrument(skip(db))]
pub async fn process_event(db: &Pool, event: Event) -> Result<()> {
    info!(event_id = %event.id, "Processing event");

    if let Err(e) = validate(&event) {
        warn!(error = %e, "Event validation failed");
        return Err(e);
    }

    debug!("Event processed successfully");
    Ok(())
}
```

### TypeScript 规范

**Lint 检查：**
```bash
cd web/admin-ui && pnpm run lint
```

**类型定义：**
```typescript
// 使用 interface 定义数据结构
interface Badge {
  id: string;
  name: string;
  type: BadgeType;
  createdAt: string;
}

// 使用 type 定义联合类型
type BadgeType = 'transaction' | 'engagement' | 'identity' | 'seasonal';

// 组件 Props 使用 interface
interface BadgeCardProps {
  badge: Badge;
  onClick?: (id: string) => void;
}
```

---

## Git 工作流

### 分支命名

| 类型 | 格式 | 示例 |
|------|------|------|
| 功能 | `feature/<description>` | `feature/add-badge-export` |
| 修复 | `fix/<description>` | `fix/rule-evaluation-bug` |
| 文档 | `docs/<description>` | `docs/api-reference` |
| 重构 | `refactor/<description>` | `refactor/service-layer` |

### 提交规范

使用 Conventional Commits 格式：

```
<type>(<scope>): <subject>

<body>

<footer>
```

**类型：**
- `feat`: 新功能
- `fix`: Bug 修复
- `docs`: 文档更新
- `style`: 代码格式（不影响逻辑）
- `refactor`: 重构
- `test`: 测试相关
- `chore`: 构建/工具变更

**示例：**
```
feat(rule-engine): add support for regex operator

Add REGEX operator support in condition evaluation.
This allows rules to match field values using regular expressions.

Closes #123
```

### 代码审查

提交 PR 前确保：
1. 所有测试通过：`make test`
2. 代码格式正确：`make fmt-check`
3. Lint 检查通过：`make lint`
4. 有足够的测试覆盖

---

## 调试技巧

### Rust 调试

```bash
# 启用 Debug 日志
RUST_LOG=debug cargo run -p badge-admin-service

# 模块级日志控制
RUST_LOG=badge_admin=debug,sqlx=warn cargo run -p badge-admin-service

# 使用 lldb/gdb 调试
cargo build
lldb ./target/debug/badge-admin
```

### 数据库调试

```bash
# 连接数据库
docker exec -it badge-postgres psql -U badge -d badge_db

# 查看表结构
\dt

# 查询数据
SELECT * FROM badges LIMIT 10;
```

### API 调试

```bash
# 使用 curl 测试 REST API
curl -X GET http://localhost:8080/api/admin/badges | jq

# 使用 grpcurl 测试 gRPC
grpcurl -plaintext -d '{"rule_id": "rule_001", "context": {}}' \
  localhost:50051 badge.rule_engine.RuleEngineService/Evaluate
```

---

## 常见问题

**Q: cargo build 失败，提示 rdkafka 编译错误**

A: 需要安装 CMake 和 librdkafka：
```bash
# macOS
brew install cmake librdkafka

# Ubuntu
sudo apt install cmake librdkafka-dev
```

**Q: 前端启动失败，提示 pnpm 版本不兼容**

A: 升级 pnpm 到最新版：
```bash
npm install -g pnpm@latest
```

**Q: 数据库迁移失败**

A: 确保 PostgreSQL 容器已启动并就绪：
```bash
docker logs badge-postgres
make db-reset
```
