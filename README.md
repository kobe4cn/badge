# 会员徽章管理系统

会员徽章管理系统是一个完整的企业级徽章发放与管理平台，支持基于用户行为的自动徽章发放、手动管理、兑换等功能。

## 系统架构

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              管理前端 (React)                                │
│                           web/admin-ui (Port 5173)                          │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Badge Admin Service (REST)                           │
│                      徽章管理后台 API (Port 8080)                            │
└─────────────────────────────────────────────────────────────────────────────┘
          │                           │                           │
          ▼                           ▼                           ▼
┌─────────────────┐    ┌──────────────────────┐    ┌──────────────────────────┐
│  Rule Engine    │    │ Badge Management     │    │   Notification Worker    │
│  统一规则引擎   │    │ 徽章管理服务 (C端)   │    │   通知消费者             │
│  (gRPC :50051)  │    │ (gRPC :50052)        │    │   (gRPC :50055)          │
└─────────────────┘    └──────────────────────┘    └──────────────────────────┘
          │                           │                           │
          └───────────────────────────┼───────────────────────────┘
                                      │
        ┌─────────────────────────────┼─────────────────────────────┐
        ▼                             ▼                             ▼
┌───────────────┐          ┌──────────────────┐          ┌──────────────────┐
│   PostgreSQL  │          │      Redis       │          │      Kafka       │
│   数据持久化  │          │   缓存 & 会话    │          │   事件消息队列   │
│   (Port 5432) │          │   (Port 6379)    │          │   (Port 9092)    │
└───────────────┘          └──────────────────┘          └──────────────────┘
                                                                  │
                    ┌─────────────────────────────────────────────┼─────┐
                    │                                             │     │
                    ▼                                             ▼     │
         ┌──────────────────────┐                    ┌──────────────────┴───┐
         │ Event Engagement     │                    │ Event Transaction    │
         │ 互动事件处理         │                    │ 交易事件处理         │
         │ (gRPC :50053)        │                    │ (gRPC :50054)        │
         └──────────────────────┘                    └──────────────────────┘
```

## 技术栈

### 后端

| 组件 | 技术 | 说明 |
|------|------|------|
| 语言 | Rust 1.85 (Edition 2024) | 高性能、内存安全 |
| RPC | tonic + prost | gRPC 服务框架 |
| Web | axum | 异步 REST API 框架 |
| 数据库 | sqlx + PostgreSQL | 异步数据库访问 |
| 缓存 | redis-rs | Redis 客户端 |
| 消息队列 | rdkafka | Kafka 客户端 |
| 可观测性 | tracing + opentelemetry | 日志与链路追踪 |

### 前端

| 组件 | 技术 | 说明 |
|------|------|------|
| 框架 | React 18 + TypeScript | 现代前端框架 |
| UI | Ant Design 5 + Pro Components | 企业级 UI 组件 |
| 状态 | Zustand + React Query | 轻量级状态管理 |
| 图表 | ECharts | 数据可视化 |
| 流程图 | XYFlow (ReactFlow) | 规则可视化编辑 |
| 构建 | Vite 6 | 快速构建工具 |

### 基础设施

| 组件 | 版本 | 说明 |
|------|------|------|
| PostgreSQL | 16 | 主数据库 |
| Redis | 7 | 缓存与会话 |
| Kafka | 7.5 (Confluent) | 事件驱动消息 |
| Elasticsearch | 8.11 | 日志与搜索 |

## 目录结构

```
badge-impl/
├── crates/                          # Rust 工作空间
│   ├── proto/                       # Protocol Buffers 定义
│   ├── shared/                      # 共享库（配置、错误、工具）
│   ├── unified-rule-engine/         # 统一规则引擎服务
│   ├── badge-management-service/    # C端徽章管理服务
│   ├── badge-admin-service/         # B端管理后台 REST API
│   ├── event-engagement-service/    # 互动事件处理服务
│   ├── event-transaction-service/   # 交易事件处理服务
│   ├── notification-worker/         # 通知消费者服务
│   └── mock-services/               # Mock 服务与测试工具
├── web/
│   └── admin-ui/                    # React 管理前端
├── docker/                          # Docker 配置
│   ├── docker-compose.infra.yml     # 基础设施编排
│   └── .env.example                 # 环境变量示例
├── config/                          # 应用配置
├── migrations/                      # 数据库迁移脚本
├── scripts/                         # 开发脚本
├── benches/                         # 性能基准测试
├── docs/                            # 项目文档
├── Cargo.toml                       # Rust 工作空间配置
└── Makefile                         # 常用命令
```

## 快速开始

### 环境要求

- Rust 1.85+
- Docker & Docker Compose
- pnpm (前端包管理器)

### 1. 克隆并设置

```bash
# 运行开发环境设置脚本
make setup
```

这将自动：
- 启动基础设施（PostgreSQL、Redis、Kafka、Elasticsearch）
- 运行数据库迁移
- 安装前端依赖
- 构建 Rust 项目

### 2. 启动服务

```bash
# 启动后端服务
make dev-backend

# 启动前端开发服务器（新终端）
make dev-frontend
```

### 3. 访问服务

- 管理后台 UI: http://localhost:5173
- Admin REST API: http://localhost:8080
- Rule Engine gRPC: localhost:50051
- Badge Management gRPC: localhost:50052

## 常用命令

```bash
# 构建
make build              # 构建所有 Rust crates
make build-release      # 构建发布版本

# 测试
make test               # 运行所有测试
make test-verbose       # 运行测试（详细输出）

# 基础设施
make infra-up           # 启动基础设施
make infra-down         # 停止基础设施
make infra-logs         # 查看基础设施日志

# 数据库
make db-migrate         # 运行数据库迁移
make db-reset           # 重置数据库

# 代码质量
make lint               # 运行代码检查
make fmt                # 格式化代码
```

## 文档

- [API 概览](docs/api-overview.md) - gRPC 与 REST API 接口说明
- [部署指南](docs/deployment.md) - 环境部署与配置
- [开发者指南](docs/development.md) - 开发环境与代码规范

## 许可证

MIT License
