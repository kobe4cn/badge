# E-Badge System 系统设计文档

> 版本: 1.0
> 日期: 2025-01-28
> 状态: 待实现

## 1. 概述

### 1.1 项目背景

会员徽章管理系统旨在为迪士尼会员运营工作提供全面、高效的徽章管理支持。通过游戏化的徽章体系，围绕会员的完整生命周期，设计丰富多样的徽章，增强会员与平台的互动性，提升会员忠诚度。

### 1.2 项目范围

完整系统交付，包括：

| 服务 | 类型 | 职责 |
|------|------|------|
| event-engagement-service | Backend | 处理高并发行为事件（5000 TPS） |
| event-transaction-service | Backend | 处理复杂订单事件（500 TPS） |
| badge-management-service | Backend | C端徽章查询、兑换、展示 |
| badge-admin-service | Backend | B端配置管理、报表统计 |
| badge-admin-UI | Frontend | React 管理界面 |
| unified-rule-engine | Backend | 可复用规则引擎 |
| mock-services | Backend | 完整模拟外部系统 |

### 1.3 技术决策摘要

| 维度 | 决策 |
|------|------|
| 通信协议 | gRPC（同步）+ Kafka（异步）+ REST（管理后台） |
| 数据存储 | PostgreSQL + Redis + Elasticsearch + OSS |
| 规则引擎 | JSON 规则 + 编译优化执行树 |
| UI 风格 | 企业级专业风格（Ant Design Pro）|
| 项目结构 | Monorepo（Cargo Workspace + pnpm workspace）|

---

## 2. 整体架构

### 2.1 架构概览

系统采用**事件驱动 + 微服务**架构，分为三层：

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              接入层                                      │
│   badge-admin-UI (REST) ──> badge-admin-service                         │
│   C端应用 (gRPC) ──> badge-management-service                           │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
┌─────────────────────────────────────────────────────────────────────────┐
│                              业务层                                      │
│   ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐        │
│   │ event-engage-   │  │ event-trans-    │  │ badge-manage-   │        │
│   │ ment-service    │  │ action-service  │  │ ment-service    │        │
│   └────────┬────────┘  └────────┬────────┘  └────────┬────────┘        │
│            │                    │                    │                  │
│            └────────────────────┼────────────────────┘                  │
│                                 │                                       │
│                    ┌────────────▼────────────┐                         │
│                    │  unified-rule-engine    │                         │
│                    └─────────────────────────┘                         │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
┌─────────────────────────────────────────────────────────────────────────┐
│                              数据层                                      │
│   PostgreSQL │ Redis │ Kafka │ Elasticsearch │ OSS                      │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.2 服务职责

| 服务 | 职责 | 并发要求 |
|------|------|----------|
| event-engagement-service | 消费 Kafka 行为事件，触发徽章发放 | 5000 TPS |
| event-transaction-service | 消费 Kafka 订单事件，处理复杂业务 | 500 TPS |
| badge-management-service | C端徽章查询、兑换、展示 | 1000 TPS |
| badge-admin-service | B端配置、发放、统计、权限 | 100 TPS |
| unified-rule-engine | 规则解析、编译、执行 | 10000 TPS |

---

## 3. 数据模型

### 3.1 徽章三层结构

```
第一层：徽章大类（Category）    → 用于分类统计，如"交易徽章"、"互动徽章"
  └── 第二层：徽章系列（Series）  → 用于分组展示，如"2024春节系列"
        └── 第三层：徽章（Badge） → 实际发放给用户的徽章实体
```

### 3.2 核心实体

| 实体 | 说明 | 关键字段 |
|------|------|----------|
| badge_category | 一级分类 | id, name, sort_order, status |
| badge_series | 二级系列 | id, category_id, name, description |
| badge | 徽章定义 | id, series_id, type, name, rules, assets, validity_config |
| badge_rule | 获取规则 | id, badge_id, rule_json, time_range, max_count |
| badge_redemption_rule | 兑换规则 | id, badge_id, required_badges, benefit_id, frequency_config |
| user_badge | 用户持有徽章 | id, user_id, badge_id, status, quantity, acquired_at, expires_at |
| user_badge_log | 发放/取消/兑换日志 | id, user_badge_id, action, reason, operator |
| benefit | 权益定义 | id, type, name, config, stock |

### 3.3 兑换明细与对账模型

| 实体 | 说明 | 关键字段 |
|------|------|----------|
| redemption_order | 兑换订单 | id, user_id, benefit_id, status, created_at |
| redemption_detail | 兑换消耗明细 | id, order_id, user_badge_id, badge_id, quantity |
| badge_ledger | 徽章账本（流水） | id, user_id, badge_id, change_type, quantity, balance_after, ref_id, ref_type |

**徽章账本（badge_ledger）设计**

采用复式记账思想，每一笔徽章变动都记录：

```
change_type 枚举：
  - ACQUIRE     获取（+）
  - EXPIRE      过期（-）
  - CANCEL      取消（-）
  - REDEEM_OUT  兑换消耗（-）
  - REDEEM_FAIL 兑换失败回滚（+）

ref_type 关联类型：
  - EVENT       事件触发
  - SCHEDULED   定时任务
  - MANUAL      手动发放
  - REDEMPTION  兑换订单
  - SYSTEM      系统操作
```

### 3.4 徽章状态流转

```
[未获取] → [已获取/有效] → [已过期]
                ↓
           [已取消]
                ↓
           [已兑换]（可部分兑换）
```

---

## 4. 规则引擎（unified-rule-engine）

### 4.1 架构定位

规则引擎作为独立服务，提供 gRPC 接口，支持：
- 徽章获取规则评估
- 徽章兑换条件校验
- 未来可复用于其他业务系统

### 4.2 规则 JSON Schema

```json
{
  "version": "1.0",
  "root": {
    "type": "group",
    "operator": "AND",
    "children": [
      {
        "type": "condition",
        "field": "event.type",
        "operator": "eq",
        "value": "PURCHASE"
      },
      {
        "type": "group",
        "operator": "OR",
        "children": [
          { "type": "condition", "field": "order.amount", "operator": "gte", "value": 500 },
          { "type": "condition", "field": "user.is_vip", "operator": "eq", "value": true }
        ]
      }
    ]
  }
}
```

### 4.3 支持的操作符

| 数据类型 | 操作符 |
|----------|--------|
| 数值 | eq, neq, gt, gte, lt, lte, between, in |
| 字符串 | eq, neq, contains, starts_with, ends_with, regex, in |
| 布尔 | eq, neq |
| 时间/日期 | eq, before, after, between |
| 列表 | contains, contains_any, contains_all, is_empty |

### 4.4 编译优化策略

- 规则加载时编译成 AST 执行树，缓存在内存
- 短路求值：AND 遇 false 立即返回，OR 遇 true 立即返回
- 字段索引：高频字段预提取，避免重复解析
- 热更新：规则变更通过 Redis Pub/Sub 通知各实例重载

### 4.5 规则配置画布

采用可视化决策树 + 拖拽式编辑，让运营人员无需理解 JSON 即可配置复杂规则。

**画布核心组件**

| 组件 | 功能 | 交互方式 |
|------|------|----------|
| 条件节点 | 单个条件配置 | 拖入画布，点击配置字段/操作符/值 |
| 逻辑组节点 | AND/OR 容器 | 拖入画布，子节点拖入其中 |
| 连接线 | 表达嵌套关系 | 自动连接，可调整层级 |
| 组件面板 | 左侧物料区 | 拖拽条件/逻辑组到画布 |
| 属性面板 | 右侧配置区 | 选中节点后编辑详情 |

**关键特性**

1. 智能字段选择：根据元数据类型自动匹配可用操作符和输入控件
2. 实时 JSON 预览：画布操作实时生成对应 JSON，支持双向编辑
3. 规则测试：输入模拟事件数据，实时显示规则匹配结果
4. 模板库：常用规则模式可保存为模板，一键复用
5. 撤销/重做：完整的操作历史，支持 Ctrl+Z 回退

**技术实现**

- 画布引擎：React Flow 或 AntV X6
- 状态管理：Zustand（记录操作历史）
- JSON Schema 校验：Ajv（前端实时校验规则合法性）

---

## 5. 事件处理服务

### 5.1 职责划分

| 服务 | 事件类型 | 特点 | 并发要求 |
|------|----------|------|----------|
| event-engagement-service | 行为事件 | 低耦合、高频、简单规则 | 5000 TPS |
| event-transaction-service | 订单事件 | 高耦合、低频、复杂逻辑 | 500 TPS |

### 5.2 事件类型

**event-engagement-service 处理的事件**

- 账号注册完成
- 个人信息完善（生日、邮箱等）
- 关注公众号
- 浏览指定页面
- 点击指定按钮
- 绑定第三方账号
- 参加线上活动

**event-transaction-service 处理的事件**

- 购买门票
- 购买尊享卡
- 预定酒店
- 餐厅消费
- 商店购物
- 订单取消（触发徽章回收）
- 身份变更（年卡用户、Club33会员等）

### 5.3 事件处理流程

```
Kafka Topic ──> 事件服务 ──> 解析事件 ──> 调用 rule-engine
                                              │
                                              ▼
                                         返回匹配徽章
                                              │
                                              ▼
                                         调用风控检查
                                              │
                                              ▼
                                         badge-management
                                         (发放/记录/通知)
```

### 5.4 高并发设计

1. **Kafka 分区消费**：按 user_id 分区，保证同一用户事件顺序处理
2. **批量处理**：小窗口聚合事件，批量调用规则引擎
3. **本地缓存**：热点规则本地缓存，减少 gRPC 调用
4. **背压控制**：消费速度自适应，防止下游过载
5. **幂等处理**：基于 event_id 去重，防止重复发放

### 5.5 故障恢复

- 消费失败自动重试（指数退避）
- 死信队列（DLQ）存放无法处理的事件
- 积压监控告警，支持 1.5 倍速追赶

---

## 6. 徽章业务服务

### 6.1 badge-management-service（C端服务）

**gRPC 接口**

| 接口 | 说明 | 性能要求 |
|------|------|----------|
| GetUserBadges | 获取用户徽章列表 | P95 ≤ 200ms |
| GetBadgeDetail | 获取徽章详情 | P95 ≤ 200ms |
| GetBadgeWall | 获取个人徽章墙 | P95 ≤ 200ms |
| RedeemBadge | 发起徽章兑换 | P95 ≤ 500ms |
| GrantBadge | 发放徽章（内部） | P95 ≤ 500ms |
| RevokeBadge | 取消徽章（内部） | P95 ≤ 500ms |
| PinBadge | 置顶/佩戴徽章 | P95 ≤ 200ms |

**缓存策略**

| 缓存键 | 内容 | TTL |
|--------|------|-----|
| user:badge:{user_id} | 用户徽章摘要 | 5min |
| badge:detail:{badge_id} | 徽章详情 | 10min |
| badge:config:{badge_id} | 徽章配置 | 30min |
| user:badge:count:{user_id} | 徽章数量 | 1min |

**兑换流程（带事务）**

1. 校验兑换条件（调用 rule-engine）
2. 开启数据库事务
   - 锁定用户徽章记录（SELECT FOR UPDATE）
   - 校验徽章数量充足
   - 创建兑换订单
   - 创建兑换明细
   - 扣减用户徽章
   - 写入徽章账本
   - 提交事务
3. 异步调用权益服务发放权益
4. 发送兑换成功通知
5. 清除相关缓存

### 6.2 badge-admin-service（B端服务）

**REST API 模块**

| 模块 | 接口示例 | 说明 |
|------|----------|------|
| 徽章管理 | POST /badges, PUT /badges/{id} | 徽章 CRUD |
| 规则配置 | POST /rules, PUT /rules/{id}/publish | 规则创建发布 |
| 发放管理 | POST /grants/manual, POST /grants/batch | 手动/批量发放 |
| 取消管理 | POST /revokes/manual, POST /revokes/batch | 手动/批量取消 |
| 权益管理 | GET /benefits, POST /benefits/sync | 权益查看同步 |
| 数据统计 | GET /stats/overview, GET /stats/badges/{id} | 看板报表 |
| 系统管理 | GET /logs, GET /users, PUT /roles | 日志账号权限 |

**批量导入设计（50万数据）**

1. 前端分片上传文件至 OSS
2. 调用 /grants/batch 接口，传入 OSS 文件地址
3. 后端创建异步任务，返回 task_id
4. 后端流式读取文件，分批处理（每批 1000 条）
5. 前端轮询 /tasks/{task_id} 查看进度
6. 处理完成生成结果报告

---

## 7. 管理后台 UI

### 7.1 技术选型

| 功能 | 技术方案 |
|------|----------|
| 框架 | React 18 + TypeScript |
| UI 组件 | Ant Design 5 + ProComponents |
| 状态管理 | Zustand |
| 请求层 | React Query + Axios |
| 规则画布 | React Flow |
| 图表 | ECharts / AntV G2 |
| 3D 预览 | Three.js |
| 文件上传 | 阿里云 OSS SDK |

### 7.2 页面结构

| 页面 | 功能 | 关键交互 |
|------|------|----------|
| 概览看板 | 核心数据指标、趋势图、告警 | 时间筛选、图表钻取 |
| 徽章分类 | 三层结构树形管理 | 拖拽排序、右键菜单 |
| 徽章列表 | 徽章 CRUD、状态管理 | 高级筛选、批量操作 |
| 徽章编辑 | 完整徽章配置 | 分步表单、实时预览 |
| 规则画布 | 可视化规则配置 | 拖拽、嵌套、测试 |
| 素材库 | 素材上传、管理 | 瀑布流、格式筛选 |
| 发放记录 | 发放历史查询 | 多维筛选、导出 |
| 手动发放 | 单个/批量发放 | 文件上传、进度条 |
| 会员视图 | 360° 会员画像 | 搜索、多 Tab 展示 |
| 数据报表 | 自定义报表 | 图表类型、维度选择 |

### 7.3 配置即所得

徽章编辑页面右侧提供手机预览模拟器，支持：
- 实时预览徽章在不同状态下的展示效果
- 切换状态预览：未获取 / 已获取 / 已兑换
- 分步表单：基本信息 → 获取规则 → 有效期 → 兑换设置 → 素材

---

## 8. 通知系统与权益管理

### 8.1 多渠道通知

**支持渠道**

- APP Push → 迪士尼推送服务
- SMS → 短信网关
- WeChat → 微信订阅消息服务
- Email → 邮件服务
- InApp → 页面弹窗(WebSocket)

**通知配置**

| 字段 | 说明 |
|------|------|
| badge_id | 关联徽章 |
| trigger_type | GRANT / REVOKE / EXPIRE / REDEEM |
| channels | 启用渠道列表 |
| template_id | 消息模板 ID |
| advance_days | 提前通知天数（过期场景） |
| retry_config | 重试配置 |

**通知流程**

1. 业务事件触发 → 写入 notification_task 表
2. 通知 Worker 消费任务，按渠道并行发送
3. 失败自动重试（指数退避，最多 3 次）
4. 最终失败 → 写入失败清单，通知运营人员
5. 支持手动重发

### 8.2 权益管理

**权益类型**

| 类型 | 说明 | 对接系统 |
|------|------|----------|
| DIGITAL_ASSET | 数字资产 | 数字资产中心 |
| COUPON | 优惠券 | Coupon 服务 |
| RESERVATION | 预约资格 | IRP 系统 |

**权益同步**

- Kafka 实时同步或 REST API 定时拉取
- 同步延迟 ≤ 1 秒
- 库存实时更新，支持预警

---

## 9. 数据统计与分析

### 9.1 运营看板

**核心指标**

- 徽章创建数量
- 徽章发放数量
- 会员获取率
- 徽章兑换数量
- 徽章兑换率
- 人均徽章获取数量
- 权益核销率

**可视化图表**

| 图表 | 类型 | 数据维度 |
|------|------|----------|
| 发放趋势 | 折线图 | 按日/周/月 |
| 徽章类型分布 | 饼图 | 各类型占比 |
| 热门徽章 TOP10 | 柱状图 | 发放排名 |
| 兑换漏斗 | 漏斗图 | 转化率 |
| 权益核销率 | 环形图 | 已核销/已发放 |

### 9.2 会员 360° 视图

**分析维度**

| Tab | 内容 |
|-----|------|
| 徽章概览 | 持有统计、类型分布、获取趋势 |
| 徽章明细 | 全部徽章列表，支持筛选导出 |
| 兑换记录 | 兑换历史、消耗明细、获得权益 |
| 权益记录 | 权益列表、核销状态、使用时间 |
| 行为分析 | 消费数据、互动数据、偏好标签 |

### 9.3 自定义报表

- 支持选择指标、维度、图表类型
- 支持保存报表模板
- 支持导出 Excel / CSV / PDF

---

## 10. 系统安全与权限

### 10.1 角色权限（RBAC）

| 角色 | 权限范围 |
|------|----------|
| 系统管理员 | 全部权限 |
| 运营主管 | 徽章管理 + 统计 + 审批 |
| 运营人员 | 徽章配置 + 发放 |
| 内容配置员 | 素材管理 + 徽章编辑 |
| 数据分析师 | 只读统计报表 |
| 客服人员 | 会员视图（只读） |

### 10.2 认证方式

- SSO 集成 MYID
- 支持 MFA（多因素认证）
- Session 过期自动登出

### 10.3 监控告警

**监控层级**

- 基础资源：CPU、内存、磁盘、网络
- 应用指标：QPS、延迟、错误率
- 业务指标：发放量、兑换量、失败量
- 安全指标：异常请求、高频操作

**告警规则示例**

| 指标 | 阈值 | 级别 |
|------|------|------|
| API 错误率 | > 1% 持续 5min | P1 |
| 响应延迟 P99 | > 2s 持续 3min | P2 |
| 徽章发放失败 | > 100/min | P1 |
| Kafka 积压 | > 10000 条 | P2 |

### 10.4 异常请求处置

| 级别 | 触发条件 | 处置 |
|------|----------|------|
| 轻度 | 1min 内 20-50 次 | 限流 10 次/min |
| 中度 | 1min 内 50-100 次 | 临时封禁 30min |
| 重度 | 1min 内 ≥100 次 | 永久封禁 |

---

## 11. 模拟外部系统

### 11.1 服务清单

| 服务 | 核心接口 |
|------|----------|
| mock-order-service | GetOrder, ListOrders, OnOrderChange |
| mock-profile-service | GetUser, UpdateUser, OnProfileChange |
| mock-coupon-service | IssueCoupon, RevokeCoupon, GetCouponStock |
| mock-irp-service | CheckReservation, ConsumeReservation |
| mock-event-tracker | PublishEvent |
| mock-search-service | IndexBadge, SearchBadges |
| mock-risk-service | EvaluateRisk |
| mock-notification | SendPush, SendSMS, SendEmail |
| mock-data-generator | 测试数据生成 |

### 11.2 用户画像模板

- NEW_VISITOR：新用户
- ACTIVE_VISITOR：活跃游客
- ANNUAL_PASS_HOLDER：年卡用户
- CLUB33_MEMBER：Club 33 会员
- HOTEL_GUEST：酒店住客

### 11.3 场景模拟

**业务场景**

- 入园日行为序列
- 线上互动序列
- 典型家庭游购买组合
- 季节性活动（春节、万圣节、圣诞季）

**异常场景**

- 订单取消（徽章回收）
- 重复事件（幂等测试）
- 服务超时/不可用（熔断测试）
- 高频请求（限流测试）
- 黄牛行为（风控测试）

---

## 12. 高可用与性能

### 12.1 性能指标

| 指标 | 需求 | 设计目标 |
|------|------|----------|
| 核心接口 TPS | ≥ 1000 | 1500 |
| 峰值 TPS | ≥ 5000 | 6000 |
| 查询接口 P95 | ≤ 200ms | 150ms |
| 事务接口 P95 | ≤ 500ms | 400ms |
| 事件吞吐量 | ≥ 5000/s | 6000/s |

### 12.2 高可用设计

- 多可用区部署
- 服务自动扩缩容
- 熔断降级策略
- 多级缓存架构
- 数据库读写分离
- 分表策略

### 12.3 弹性策略

**自动扩缩容**

- event-engagement: 3-20 实例
- badge-management: 3-15 实例
- 基于 CPU / Kafka Lag 指标

**熔断降级**

| 服务 | 熔断条件 | 降级方案 |
|------|----------|----------|
| rule-engine | 错误率 > 50% | 使用本地缓存 |
| coupon-service | 超时 > 3s | 异步重试 |
| notification | 失败率 > 30% | 写入重试队列 |

---

## 13. 部署架构

### 13.1 阿里云部署

- 接入层：WAF + SLB + API 网关
- 服务层：ACK（Kubernetes）
- 数据层：RDS PG + Redis Cluster + Kafka + ES + OSS

### 13.2 环境规划

| 环境 | 用途 | 配置 |
|------|------|------|
| DEV | 开发调试 | 最小配置 |
| SIT | 集成测试 | 中等配置 |
| UAT | 验收测试 | 接近生产 |
| PROD | 生产环境 | 完整配置 |

### 13.3 发布策略

- 蓝绿部署
- 灰度发布：5% → 20% → 50% → 100%

---

## 14. 项目结构

### 14.1 Monorepo 结构

```
badge/
├── Cargo.toml                  # Workspace 根配置
├── crates/                     # Rust 服务和库
│   ├── proto/                  # gRPC 定义
│   ├── shared/                 # 共享库
│   ├── unified-rule-engine/
│   ├── event-engagement-service/
│   ├── event-transaction-service/
│   ├── badge-management-service/
│   ├── badge-admin-service/
│   └── notification-worker/
├── mock-services/              # 模拟外部系统
├── web/admin-ui/               # React 前端
├── migrations/                 # 数据库迁移
├── docker/                     # Docker 配置
├── deploy/                     # K8s & Terraform
├── docs/                       # 文档
└── scripts/                    # 脚本
```

### 14.2 技术规范

**Rust**

- Edition: 2024
- Rust Version: 1.83+
- Async Runtime: Tokio
- gRPC: Tonic
- Database: SQLx
- Error Handling: thiserror + anyhow

**Frontend**

- React 18 + TypeScript
- Ant Design 5
- Zustand + React Query
- Vite

---

## 15. 下一步

1. 创建项目骨架结构
2. 定义 Protobuf 接口
3. 实现 unified-rule-engine
4. 实现核心数据模型和迁移
5. 实现 badge-management-service
6. 实现 badge-admin-service
7. 实现 event 处理服务
8. 实现 admin-UI
9. 实现 mock-services
10. 集成测试和性能测试
