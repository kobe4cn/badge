# 徽章系统认证与权限管理设计

> **目标:** 为徽章系统增加完整的认证授权能力，包括用户登录、角色权限管理、API 安全控制，并进行全面的前后端集成测试。

**创建日期:** 2026-02-02
**状态:** 设计中

---

## 一、现状分析

### 1.1 认证状态

| 层面 | 状态 | 详情 |
|------|------|------|
| **前端认证** | ✅ 完整 | Zustand 状态管理、Token 拦截、路由守卫、登录页 |
| **开发 Mock** | ✅ 完整 | Vite 中间件拦截 `/api/admin/auth/*`，3 个测试账户 |
| **后端认证** | ❌ 缺失 | 无 JWT 中间件、无登录端点、API 完全开放 |

### 1.2 前后端 API 差异

**缺失的认证 API:**
- `POST /admin/auth/login` - 用户登录
- `POST /admin/auth/logout` - 用户登出
- `GET /admin/auth/me` - 获取当前用户
- `POST /admin/auth/refresh` - 刷新 Token

**缺失的业务 API (20+ 个):**
- 徽章：archive、sort、unpublish（命名不一致）
- 分类/系列：status 切换、sort 更新
- 规则：test (无 ID)、disable
- 发放：logs 详情/导出、records、upload-csv、preview-filter
- 任务：cancel、failures、result 下载
- 兑换：订单详情

**完全缺失的模块:**
- 用户管理（系统用户 CRUD）
- 角色管理（角色 CRUD、权限分配）
- 权限管理（权限定义、路由控制）

---

## 二、设计目标

### 2.1 功能目标

1. **认证系统**: JWT 登录、Token 刷新、安全登出
2. **用户管理**: 系统用户 CRUD、密码管理、状态控制
3. **角色管理**: 角色 CRUD、权限分配、多角色支持
4. **权限控制**: 路由级权限、操作级权限、数据级权限（可选）
5. **API 安全**: Bearer Token 验证、外部系统 API Key 认证
6. **集成测试**: 全面的前后端真实联动测试

### 2.2 非功能目标

- 向后兼容：现有前端代码改动最小化
- 可扩展：支持未来接入 SSO/OAuth
- 安全性：密码加密、Token 安全、防重放攻击
- 性能：权限缓存、最小化数据库查询

---

## 三、数据模型设计

### 3.1 ER 图

```
┌──────────────┐       ┌──────────────┐       ┌──────────────┐
│  admin_user  │──────<│ user_role    │>──────│    role      │
├──────────────┤       ├──────────────┤       ├──────────────┤
│ id           │       │ user_id      │       │ id           │
│ username     │       │ role_id      │       │ code         │
│ password_hash│       │ created_at   │       │ name         │
│ email        │       └──────────────┘       │ description  │
│ display_name │                              │ enabled      │
│ avatar_url   │       ┌──────────────┐       │ created_at   │
│ status       │       │ role_perm    │       │ updated_at   │
│ last_login   │       ├──────────────┤       └──────┬───────┘
│ created_at   │       │ role_id      │              │
│ updated_at   │       │ permission_id│              │
└──────────────┘       │ created_at   │       ┌──────▼───────┐
                       └──────────────┘       │ permission   │
                                              ├──────────────┤
┌──────────────┐                              │ id           │
│ api_key      │                              │ code         │
├──────────────┤                              │ name         │
│ id           │                              │ module       │
│ name         │                              │ action       │
│ key_hash     │                              │ description  │
│ permissions  │                              │ enabled      │
│ expires_at   │                              │ created_at   │
│ enabled      │                              └──────────────┘
│ last_used_at │
│ created_at   │
└──────────────┘
```

### 3.2 数据库迁移

```sql
-- 系统用户表
CREATE TABLE admin_user (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR(50) NOT NULL UNIQUE,
    password_hash VARCHAR(200) NOT NULL,
    email VARCHAR(100),
    display_name VARCHAR(100),
    avatar_url VARCHAR(500),
    status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE', -- ACTIVE, DISABLED, LOCKED
    failed_login_attempts INT NOT NULL DEFAULT 0,
    locked_until TIMESTAMPTZ,
    last_login_at TIMESTAMPTZ,
    password_changed_at TIMESTAMPTZ,
    created_by BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 角色表
CREATE TABLE role (
    id BIGSERIAL PRIMARY KEY,
    code VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    is_system BOOLEAN NOT NULL DEFAULT FALSE, -- 系统内置角色不可删除
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 权限表
CREATE TABLE permission (
    id BIGSERIAL PRIMARY KEY,
    code VARCHAR(100) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    module VARCHAR(50) NOT NULL, -- badge, rule, grant, benefit, system
    action VARCHAR(50) NOT NULL, -- create, read, update, delete, publish
    resource_pattern VARCHAR(200), -- 资源匹配模式，如 /badges/*
    description TEXT,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 用户-角色关联表
CREATE TABLE user_role (
    user_id BIGINT NOT NULL REFERENCES admin_user(id) ON DELETE CASCADE,
    role_id BIGINT NOT NULL REFERENCES role(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, role_id)
);

-- 角色-权限关联表
CREATE TABLE role_permission (
    role_id BIGINT NOT NULL REFERENCES role(id) ON DELETE CASCADE,
    permission_id BIGINT NOT NULL REFERENCES permission(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (role_id, permission_id)
);

-- 外部 API Key 表
CREATE TABLE api_key (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    key_prefix VARCHAR(10) NOT NULL, -- 用于标识 key，如 "bk_"
    key_hash VARCHAR(200) NOT NULL,
    permissions JSONB NOT NULL DEFAULT '[]', -- 允许的权限码列表
    rate_limit INT DEFAULT 1000, -- 每分钟请求限制
    expires_at TIMESTAMPTZ,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    last_used_at TIMESTAMPTZ,
    created_by BIGINT REFERENCES admin_user(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 索引
CREATE INDEX idx_admin_user_username ON admin_user(username);
CREATE INDEX idx_admin_user_status ON admin_user(status);
CREATE INDEX idx_role_code ON role(code);
CREATE INDEX idx_permission_module ON permission(module);
CREATE INDEX idx_api_key_prefix ON api_key(key_prefix);
```

### 3.3 初始数据

```sql
-- 默认角色
INSERT INTO role (code, name, description, is_system) VALUES
('admin', '超级管理员', '拥有所有权限', TRUE),
('operator', '运营人员', '徽章和规则的日常管理', TRUE),
('viewer', '只读用户', '仅查看数据', TRUE);

-- 默认权限
INSERT INTO permission (code, name, module, action, resource_pattern) VALUES
-- 系统管理
('system:user:read', '查看用户', 'system', 'read', '/users/*'),
('system:user:write', '管理用户', 'system', 'write', '/users/*'),
('system:role:read', '查看角色', 'system', 'read', '/roles/*'),
('system:role:write', '管理角色', 'system', 'write', '/roles/*'),
-- 徽章管理
('badge:category:read', '查看分类', 'badge', 'read', '/categories/*'),
('badge:category:write', '管理分类', 'badge', 'write', '/categories/*'),
('badge:series:read', '查看系列', 'badge', 'read', '/series/*'),
('badge:series:write', '管理系列', 'badge', 'write', '/series/*'),
('badge:badge:read', '查看徽章', 'badge', 'read', '/badges/*'),
('badge:badge:write', '管理徽章', 'badge', 'write', '/badges/*'),
('badge:badge:publish', '发布徽章', 'badge', 'publish', '/badges/*/publish'),
-- 规则管理
('rule:rule:read', '查看规则', 'rule', 'read', '/rules/*'),
('rule:rule:write', '管理规则', 'rule', 'write', '/rules/*'),
('rule:rule:publish', '发布规则', 'rule', 'publish', '/rules/*/publish'),
('rule:template:read', '查看模板', 'rule', 'read', '/templates/*'),
-- 发放管理
('grant:grant:read', '查看发放', 'grant', 'read', '/grants/*'),
('grant:grant:write', '发放徽章', 'grant', 'write', '/grants/*'),
('grant:revoke:write', '取消徽章', 'grant', 'write', '/revokes/*'),
-- 权益管理
('benefit:benefit:read', '查看权益', 'benefit', 'read', '/benefits/*'),
('benefit:benefit:write', '管理权益', 'benefit', 'write', '/benefits/*'),
('benefit:redemption:read', '查看兑换', 'benefit', 'read', '/redemption/*'),
('benefit:redemption:write', '管理兑换', 'benefit', 'write', '/redemption/*'),
-- 统计与日志
('stats:read', '查看统计', 'stats', 'read', '/stats/*'),
('log:read', '查看日志', 'log', 'read', '/logs/*');

-- 角色权限分配
-- admin: 所有权限
INSERT INTO role_permission (role_id, permission_id)
SELECT 1, id FROM permission;

-- operator: 业务权限（不含系统管理）
INSERT INTO role_permission (role_id, permission_id)
SELECT 2, id FROM permission WHERE module != 'system';

-- viewer: 只读权限
INSERT INTO role_permission (role_id, permission_id)
SELECT 3, id FROM permission WHERE action = 'read';

-- 默认管理员用户 (密码: admin123)
INSERT INTO admin_user (username, password_hash, display_name, status) VALUES
('admin', '$argon2id$v=19$m=19456,t=2,p=1$...', '系统管理员', 'ACTIVE');

INSERT INTO user_role (user_id, role_id) VALUES (1, 1);
```

---

## 四、后端实现方案

### 4.1 认证流程

```
┌─────────┐     POST /auth/login      ┌─────────┐
│ Client  │ ──────────────────────────> │ Server  │
│         │ { username, password }     │         │
│         │                            │         │
│         │ <────────────────────────── │         │
│         │ { token, user, permissions}│         │
└─────────┘                            └─────────┘

Token 结构 (JWT):
{
  "sub": "1",           // user_id
  "username": "admin",
  "roles": ["admin"],
  "iat": 1706832000,
  "exp": 1706918400     // 24小时过期
}
```

### 4.2 中间件架构

```rust
// 认证中间件层级
pub fn api_routes() -> Router<AppState> {
    Router::new()
        // 公开路由（无需认证）
        .route("/auth/login", post(auth::login))
        .route("/health", get(health_check))

        // 需要认证的路由
        .nest("/", protected_routes())
        .layer(middleware::from_fn(auth_middleware))
}

fn protected_routes() -> Router<AppState> {
    Router::new()
        .merge(badge_routes())
        .merge(rule_routes())
        // ... 其他路由

        // 需要特定权限的路由
        .route("/users", get(user::list).layer(require_permission("system:user:read")))
        .route("/users", post(user::create).layer(require_permission("system:user:write")))
}
```

### 4.3 认证 API 设计

```rust
// POST /api/admin/auth/login
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

pub struct LoginResponse {
    pub token: String,
    pub user: AdminUserDto,
    pub permissions: Vec<String>,
    pub expires_at: DateTime<Utc>,
}

// POST /api/admin/auth/logout
// Header: Authorization: Bearer <token>
// 响应: 204 No Content

// GET /api/admin/auth/me
// Header: Authorization: Bearer <token>
pub struct CurrentUserResponse {
    pub user: AdminUserDto,
    pub permissions: Vec<String>,
    pub roles: Vec<RoleDto>,
}

// POST /api/admin/auth/refresh
// Header: Authorization: Bearer <token>
pub struct RefreshResponse {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}
```

### 4.4 外部 API 认证

```
┌────────────────┐     GET /api/v1/users/{id}/badges     ┌─────────┐
│ External System│ ──────────────────────────────────────> │ Server  │
│                │ Header: X-API-Key: bk_xxx...           │         │
│                │                                        │         │
│                │ <────────────────────────────────────── │         │
│                │ { badges: [...] }                      │         │
└────────────────┘                                        └─────────┘
```

API Key 验证流程：
1. 从 Header 提取 `X-API-Key`
2. 从 key 提取 prefix（如 `bk_`）
3. 查询数据库验证 key_hash
4. 检查 enabled、expires_at、rate_limit
5. 验证请求的权限是否在 permissions 列表中

---

## 五、后端新增 API 清单

### 5.1 认证模块 (4 个)

| 方法 | 路径 | 功能 | 权限 |
|------|------|------|------|
| POST | /auth/login | 用户登录 | 公开 |
| POST | /auth/logout | 用户登出 | 已认证 |
| GET | /auth/me | 获取当前用户 | 已认证 |
| POST | /auth/refresh | 刷新 Token | 已认证 |

### 5.2 用户管理模块 (6 个)

| 方法 | 路径 | 功能 | 权限 |
|------|------|------|------|
| GET | /system/users | 获取用户列表 | system:user:read |
| POST | /system/users | 创建用户 | system:user:write |
| GET | /system/users/{id} | 获取用户详情 | system:user:read |
| PUT | /system/users/{id} | 更新用户 | system:user:write |
| DELETE | /system/users/{id} | 删除用户 | system:user:write |
| POST | /system/users/{id}/reset-password | 重置密码 | system:user:write |

### 5.3 角色管理模块 (5 个)

| 方法 | 路径 | 功能 | 权限 |
|------|------|------|------|
| GET | /system/roles | 获取角色列表 | system:role:read |
| POST | /system/roles | 创建角色 | system:role:write |
| GET | /system/roles/{id} | 获取角色详情 | system:role:read |
| PUT | /system/roles/{id} | 更新角色 | system:role:write |
| DELETE | /system/roles/{id} | 删除角色 | system:role:write |

### 5.4 权限管理模块 (2 个)

| 方法 | 路径 | 功能 | 权限 |
|------|------|------|------|
| GET | /system/permissions | 获取权限列表 | system:role:read |
| GET | /system/permissions/tree | 获取权限树 | system:role:read |

### 5.5 API Key 管理模块 (5 个)

| 方法 | 路径 | 功能 | 权限 |
|------|------|------|------|
| GET | /system/api-keys | 获取 API Key 列表 | admin |
| POST | /system/api-keys | 创建 API Key | admin |
| GET | /system/api-keys/{id} | 获取 API Key 详情 | admin |
| DELETE | /system/api-keys/{id} | 删除 API Key | admin |
| POST | /system/api-keys/{id}/regenerate | 重新生成 Key | admin |

### 5.6 缺失的业务 API (20+ 个)

详见第一节 API 差异分析，包括：
- 徽章：archive、sort、unpublish
- 分类/系列：status、sort
- 规则：test (无 ID)、disable
- 发放：logs 详情/导出、records、upload-csv、preview-filter
- 任务：cancel、failures、result
- 兑换：订单详情

---

## 六、前端改动

### 6.1 服务层适配

```typescript
// services/auth.ts - 无需改动，已完整实现

// services/system.ts - 新增
export const systemService = {
  // 用户管理
  listUsers: (params: PaginationParams) => getList<AdminUser>('/admin/system/users', params),
  createUser: (data: CreateUserRequest) => post<AdminUser>('/admin/system/users', data),
  getUser: (id: number) => get<AdminUser>(`/admin/system/users/${id}`),
  updateUser: (id: number, data: UpdateUserRequest) => put<AdminUser>(`/admin/system/users/${id}`, data),
  deleteUser: (id: number) => del(`/admin/system/users/${id}`),
  resetPassword: (id: number, password: string) => post(`/admin/system/users/${id}/reset-password`, { password }),

  // 角色管理
  listRoles: (params?: PaginationParams) => getList<Role>('/admin/system/roles', params),
  createRole: (data: CreateRoleRequest) => post<Role>('/admin/system/roles', data),
  getRole: (id: number) => get<Role>(`/admin/system/roles/${id}`),
  updateRole: (id: number, data: UpdateRoleRequest) => put<Role>(`/admin/system/roles/${id}`, data),
  deleteRole: (id: number) => del(`/admin/system/roles/${id}`),

  // 权限
  listPermissions: () => get<Permission[]>('/admin/system/permissions'),
  getPermissionTree: () => get<PermissionTree>('/admin/system/permissions/tree'),

  // API Key
  listApiKeys: () => getList<ApiKey>('/admin/system/api-keys'),
  createApiKey: (data: CreateApiKeyRequest) => post<ApiKeyWithSecret>('/admin/system/api-keys', data),
  deleteApiKey: (id: number) => del(`/admin/system/api-keys/${id}`),
  regenerateApiKey: (id: number) => post<ApiKeyWithSecret>(`/admin/system/api-keys/${id}/regenerate`),
};
```

### 6.2 路由权限配置

```typescript
// components/Auth/authUtils.ts
const ROUTE_PERMISSIONS: Permission[] = [
  // 系统管理（仅 admin）
  { path: '/system', roles: ['admin'] },
  { path: '/system/users', roles: ['admin'] },
  { path: '/system/roles', roles: ['admin'] },
  { path: '/system/api-keys', roles: ['admin'] },

  // 徽章管理
  { path: '/badges', roles: ['admin', 'operator', 'viewer'] },
  { path: '/badges/create', roles: ['admin', 'operator'] },

  // 规则管理
  { path: '/rules', roles: ['admin', 'operator', 'viewer'] },
  { path: '/rules/create', roles: ['admin', 'operator'] },

  // 发放管理
  { path: '/grants', roles: ['admin', 'operator'] },

  // 权益管理
  { path: '/benefits', roles: ['admin', 'operator', 'viewer'] },
  { path: '/redemptions', roles: ['admin', 'operator', 'viewer'] },

  // 数据看板
  { path: '/dashboard', roles: ['admin', 'operator', 'viewer'] },
];
```

### 6.3 新增页面

```
web/admin-ui/src/pages/system/
├── Users.tsx           # 用户列表
├── UserForm.tsx        # 用户表单
├── Roles.tsx           # 角色列表
├── RoleForm.tsx        # 角色表单（含权限分配）
├── ApiKeys.tsx         # API Key 管理
└── index.ts
```

---

## 七、集成测试计划

### 7.1 测试环境准备

```bash
# 启动完整后端服务栈
make infra-up        # PostgreSQL, Redis, Kafka
make db-migrate      # 数据库迁移
make dev-backend     # 所有后端服务

# 前端开发服务器（禁用 Mock）
cd web/admin-ui
VITE_DISABLE_MOCK=true pnpm run dev
```

### 7.2 测试场景清单

#### 认证流程
- [ ] 正确凭据登录成功
- [ ] 错误凭据登录失败
- [ ] Token 过期后自动跳转登录页
- [ ] Token 刷新功能
- [ ] 登出后清除状态

#### 用户管理
- [ ] 创建用户
- [ ] 编辑用户信息
- [ ] 重置用户密码
- [ ] 禁用/启用用户
- [ ] 删除用户

#### 角色权限
- [ ] 创建角色并分配权限
- [ ] 用户分配角色
- [ ] 权限验证（操作被拒绝）
- [ ] 角色切换后权限生效

#### 徽章管理
- [ ] 创建分类
- [ ] 创建系列
- [ ] 创建徽章
- [ ] 徽章发布/下架
- [ ] 依赖关系配置
- [ ] 依赖图可视化

#### 规则管理
- [ ] 画布创建规则
- [ ] 规则节点拖拽
- [ ] 规则条件配置
- [ ] 规则测试
- [ ] 规则发布
- [ ] 模板创建规则

#### 发放与兑换
- [ ] 手动发放徽章
- [ ] 批量发放任务
- [ ] 发放日志查询
- [ ] 兑换规则配置
- [ ] 执行兑换
- [ ] 兑换记录查询

#### 权益管理
- [ ] 创建权益
- [ ] 权益关联徽章
- [ ] 自动权益发放
- [ ] 权益发放记录

#### 会员视图
- [ ] 用户搜索
- [ ] 用户徽章列表
- [ ] 用户兑换记录
- [ ] 用户权益查看

#### 外部 API
- [ ] API Key 创建
- [ ] API Key 认证请求
- [ ] 权限限制验证
- [ ] 过期 Key 拒绝

---

## 八、实现计划

### Phase 1: 数据库与基础认证 (优先级: P0)

**Task 1.1**: 数据库迁移
- 创建 admin_user, role, permission, user_role, role_permission 表
- 初始化默认角色和权限
- 创建默认管理员用户

**Task 1.2**: 认证 API 实现
- 实现 JWT 生成和验证
- 实现登录/登出/获取当前用户/刷新 Token
- 添加认证中间件

**Task 1.3**: 权限中间件
- 实现权限检查中间件
- 集成到现有路由

### Phase 2: 用户与角色管理 (优先级: P0)

**Task 2.1**: 用户管理 API
- CRUD 接口实现
- 密码加密与重置

**Task 2.2**: 角色管理 API
- CRUD 接口实现
- 权限分配接口

**Task 2.3**: 前端系统管理页面
- 用户列表和表单
- 角色列表和表单
- 权限树选择器

### Phase 3: API 补齐 (优先级: P1)

**Task 3.1**: 补齐缺失的业务 API
- 徽章相关 API
- 规则相关 API
- 发放相关 API
- 任务相关 API

**Task 3.2**: 外部 API 安全
- API Key 管理
- API Key 认证中间件

### Phase 4: 集成测试 (优先级: P1)

**Task 4.1**: E2E 测试用例
- 认证流程测试
- 全链路业务测试

**Task 4.2**: 测试文档
- 测试场景说明
- 测试数据准备

---

## 九、安全考量

### 9.1 密码安全
- 使用 Argon2id 算法加密
- 密码强度要求（最小长度、复杂度）
- 登录失败锁定机制

### 9.2 Token 安全
- JWT 使用 RS256 或 HS256 签名
- Token 过期时间 24 小时
- 刷新 Token 机制
- 登出时 Token 黑名单（可选）

### 9.3 API 安全
- HTTPS 强制
- 请求频率限制
- 敏感操作审计日志
- API Key 权限最小化

### 9.4 数据安全
- 敏感字段加密存储
- 日志脱敏
- 数据备份策略

---

## 十、后续扩展

### 10.1 SSO 集成
- 预留 OAuth2/OIDC 接口
- 支持企业微信/钉钉登录

### 10.2 多租户
- 组织级数据隔离
- 跨组织数据共享

### 10.3 审计增强
- 操作审计详情
- 数据变更追踪
- 合规报表

---

## 附录

### A. 技术选型

| 组件 | 选择 | 理由 |
|------|------|------|
| 密码加密 | Argon2id | 安全性高，OWASP 推荐 |
| JWT 库 | jsonwebtoken (Rust) | 成熟稳定 |
| 权限模型 | RBAC | 满足当前需求，易于理解 |

### B. 参考资料

- OWASP Authentication Cheat Sheet
- JWT Best Practices
- Rust Axum Authentication Examples
