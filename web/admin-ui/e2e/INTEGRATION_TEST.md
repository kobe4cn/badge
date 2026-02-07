# 前后端集成测试指南

## 概述

集成测试用于验证前端与真实后端服务的交互。与普通 E2E 测试不同，集成测试禁用了 mock 模式，所有 API 请求都会发送到真实的后端服务。

## 环境要求

### 后端服务

在运行集成测试前，需要启动以下服务：

1. **基础设施**
   ```bash
   cd docker
   docker compose -f docker-compose.infra.yml up -d
   ```

2. **运行数据库迁移**
   ```bash
   sqlx migrate run --database-url "postgresql://badge:badge_secret@localhost:5432/badge_db"
   ```

3. **启动后端服务**
   ```bash
   cargo run -p badge-admin-service
   ```
   服务默认运行在 `http://localhost:8080`

### 前端服务

集成测试需要在禁用 mock 的模式下运行前端：

```bash
npm run dev:real
```

或者让测试自动启动服务。

## 运行测试

### 完整集成测试

```bash
npm run test:integration
```

### 带界面的集成测试（便于调试）

```bash
npm run test:integration:headed
```

### 运行单个测试文件

```bash
VITE_DISABLE_MOCK=true npx playwright test e2e/specs/integration.spec.ts
```

### 运行特定测试

```bash
VITE_DISABLE_MOCK=true npx playwright test -g "管理员登录"
```

## 测试用户

集成测试使用以下预置用户：

| 用户名 | 密码 | 角色 | 权限 |
|--------|------|------|------|
| admin | admin123 | 超级管理员 | 全部权限 |
| operator | operator123 | 运营人员 | 除系统管理外的所有权限 |
| viewer | viewer123 | 只读用户 | 仅查看权限 |

## 测试场景

### 认证流程
- 登录成功
- 登录失败
- 登出
- Token 刷新

### 徽章管理
- 分类 CRUD
- 系列 CRUD
- 徽章创建和发布

### 系统管理
- 用户列表
- 角色管理
- API Key 管理

### 权限控制
- 运营人员权限限制
- 只读用户权限限制

## 数据清理

测试结束后会自动清理以 `INT_` 前缀创建的测试数据。

## 故障排除

### 连接失败

确保后端服务在 `localhost:8080` 运行：
```bash
curl http://localhost:8080/api/admin/health
```

### 认证失败

检查数据库迁移是否正确运行，确保 admin_user 表中有预置用户：
```sql
SELECT username, status FROM admin_user;
```

### 测试超时

增加 playwright 超时配置或检查网络连接。
