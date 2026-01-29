# API 概览

本文档描述徽章系统提供的 gRPC 与 REST API 接口。

## gRPC 服务

### 1. RuleEngineService (Port: 50051)

统一规则引擎服务，负责徽章发放规则的评估与管理。

| 方法 | 描述 |
|------|------|
| `Evaluate` | 评估单条规则 |
| `BatchEvaluate` | 批量评估多条规则 |
| `LoadRule` | 加载/更新规则到引擎 |
| `DeleteRule` | 删除规则 |
| `TestRule` | 测试规则（不持久化） |

**Proto 定义:** `crates/proto/src/rule_engine.proto`

#### 示例：评估规则

```protobuf
message EvaluateRequest {
  string rule_id = 1;
  google.protobuf.Struct context = 2;
}

message EvaluateResponse {
  bool matched = 1;
  string rule_id = 2;
  string rule_name = 3;
  repeated string matched_conditions = 4;
  int64 evaluation_time_ms = 5;
}
```

### 2. BadgeManagementService (Port: 50052)

C端徽章管理服务，处理用户徽章的查询、发放、取消与兑换。

| 方法 | 描述 |
|------|------|
| `GetUserBadges` | 获取用户徽章列表 |
| `GetBadgeDetail` | 获取徽章详情 |
| `GetBadgeWall` | 获取用户徽章墙 |
| `GrantBadge` | 发放徽章（内部调用） |
| `RevokeBadge` | 取消徽章（内部调用） |
| `RedeemBadge` | 兑换徽章 |
| `PinBadge` | 置顶/佩戴徽章 |

**Proto 定义:** `crates/proto/src/badge.proto`

---

## REST API (Port: 8080)

管理后台 REST API，所有接口以 `/api/admin` 为前缀。

### 徽章管理

| 方法 | 端点 | 描述 |
|------|------|------|
| POST | `/api/admin/categories` | 创建分类 |
| GET | `/api/admin/categories` | 获取分类列表 |
| GET | `/api/admin/categories/{id}` | 获取分类详情 |
| PUT | `/api/admin/categories/{id}` | 更新分类 |
| DELETE | `/api/admin/categories/{id}` | 删除分类 |
| POST | `/api/admin/series` | 创建系列 |
| GET | `/api/admin/series` | 获取系列列表 |
| GET | `/api/admin/series/{id}` | 获取系列详情 |
| PUT | `/api/admin/series/{id}` | 更新系列 |
| DELETE | `/api/admin/series/{id}` | 删除系列 |
| POST | `/api/admin/badges` | 创建徽章 |
| GET | `/api/admin/badges` | 获取徽章列表 |
| GET | `/api/admin/badges/{id}` | 获取徽章详情 |
| PUT | `/api/admin/badges/{id}` | 更新徽章 |
| DELETE | `/api/admin/badges/{id}` | 删除徽章 |
| POST | `/api/admin/badges/{id}/publish` | 发布徽章 |
| POST | `/api/admin/badges/{id}/offline` | 下线徽章 |

### 规则管理

| 方法 | 端点 | 描述 |
|------|------|------|
| POST | `/api/admin/rules` | 创建规则 |
| GET | `/api/admin/rules` | 获取规则列表 |
| GET | `/api/admin/rules/{id}` | 获取规则详情 |
| PUT | `/api/admin/rules/{id}` | 更新规则 |
| DELETE | `/api/admin/rules/{id}` | 删除规则 |
| POST | `/api/admin/rules/{id}/publish` | 发布规则 |
| POST | `/api/admin/rules/{id}/test` | 测试规则 |

### 发放管理

| 方法 | 端点 | 描述 |
|------|------|------|
| POST | `/api/admin/grants/manual` | 手动发放徽章 |
| POST | `/api/admin/grants/batch` | 批量发放徽章 |
| GET | `/api/admin/grants` | 查询发放记录 |

### 取消管理

| 方法 | 端点 | 描述 |
|------|------|------|
| POST | `/api/admin/revokes/manual` | 手动取消徽章 |
| POST | `/api/admin/revokes/batch` | 批量取消徽章 |
| GET | `/api/admin/revokes` | 查询取消记录 |

### 统计报表

| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/admin/stats/overview` | 获取总览数据 |
| GET | `/api/admin/stats/trends` | 获取趋势数据 |
| GET | `/api/admin/stats/ranking` | 获取排行榜 |
| GET | `/api/admin/stats/badges/{id}` | 获取单徽章统计 |

### 会员视图

| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/admin/users/{user_id}/badges` | 获取用户徽章 |
| GET | `/api/admin/users/{user_id}/redemptions` | 获取用户兑换记录 |
| GET | `/api/admin/users/{user_id}/stats` | 获取用户统计 |
| GET | `/api/admin/users/{user_id}/ledger` | 获取用户账本流水 |

### 操作日志

| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/admin/logs` | 查询操作日志 |

### 批量任务

| 方法 | 端点 | 描述 |
|------|------|------|
| POST | `/api/admin/tasks` | 创建批量任务 |
| GET | `/api/admin/tasks` | 查询任务列表 |
| GET | `/api/admin/tasks/{id}` | 查询任务详情/进度 |

---

## 认证说明

### REST API 认证

管理后台 API 使用 JWT Token 进行认证：

```http
Authorization: Bearer <token>
```

### gRPC 认证

gRPC 服务使用 metadata 传递认证信息：

```rust
let mut request = tonic::Request::new(request_body);
request.metadata_mut().insert(
    "authorization",
    format!("Bearer {}", token).parse().unwrap()
);
```

### 认证流程

1. 用户通过登录接口获取 JWT Token
2. 后续请求携带 Token 在 Header 中
3. 服务端验证 Token 有效性
4. Token 过期后需要刷新

---

## 错误码规范

### HTTP 状态码

| 状态码 | 说明 |
|--------|------|
| 200 | 成功 |
| 400 | 请求参数错误 |
| 401 | 未认证 |
| 403 | 无权限 |
| 404 | 资源不存在 |
| 409 | 资源冲突 |
| 500 | 服务器内部错误 |

### 错误响应格式

```json
{
  "code": "BADGE_NOT_FOUND",
  "message": "Badge with id 'xxx' not found",
  "details": {}
}
```

---

## 分页规范

列表接口统一支持分页：

**请求参数：**

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `page` | int | 1 | 页码（从1开始） |
| `page_size` | int | 20 | 每页数量（最大100） |

**响应格式：**

```json
{
  "data": [],
  "total": 100,
  "page": 1,
  "page_size": 20
}
```
