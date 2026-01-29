# Mock Services

Mock 服务模块，为徽章系统提供模拟的外部服务，用于开发和测试环境。

## 概述

`mock-services` 提供以下功能：

- **Mock REST API 服务**：模拟订单、用户资料、优惠券等外部服务
- **事件生成器**：生成各类业务事件并发送到 Kafka
- **场景模拟器**：执行预定义或自定义的用户行为场景
- **测试数据生成**：批量生成用户、订单、优惠券等测试数据

## 安装与启动

### 编译

```bash
cargo build -p mock-services
```

### 启动服务

```bash
# 基本启动
cargo run -p mock-services --bin mock-server -- server

# 指定端口并预填充测试数据
cargo run -p mock-services --bin mock-server -- server --port 8090 --populate --user-count 100
```

## CLI 命令

### server - 启动 Mock 服务

启动 HTTP REST API 服务器，提供订单、用户、优惠券等 Mock API。

```bash
# 默认启动（端口 8090）
mock-server server

# 自定义端口并预填充数据
mock-server server --port 9000 --populate --user-count 200

# 设置日志级别
mock-server --log-level debug server
```

参数说明：
- `--port, -p`：服务端口（默认：8090）
- `--populate`：启动时预填充测试数据
- `--user-count`：预填充用户数量（默认：100）

### generate - 生成事件

生成并发送事件到 Kafka。

```bash
# 生成购买事件
mock-server generate -e purchase -u user-001 --amount 199.99

# 批量生成签到事件
mock-server generate -e checkin -u user-001 -c 5

# 生成页面浏览事件
mock-server generate -e pageview -u user-001
```

参数说明：
- `--event-type, -e`：事件类型（purchase, checkin, refund, pageview, share）
- `--user-id, -u`：用户 ID
- `--count, -c`：生成数量（默认：1）
- `--amount`：金额（仅 purchase/refund 事件）

### scenario - 运行场景

执行预定义或自定义的用户行为场景。

```bash
# 列出所有预定义场景
mock-server scenario -n list

# 执行首购场景
mock-server scenario -n first_purchase

# 执行场景并覆盖用户 ID
mock-server scenario -n vip_upgrade -u my-test-user

# 从文件加载自定义场景
mock-server scenario -n custom -f /path/to/scenario.yaml
```

参数说明：
- `--name, -n`：场景名称（使用 "list" 列出所有场景）
- `--user-id, -u`：覆盖场景中的用户 ID
- `--file, -f`：自定义场景文件路径（JSON/YAML）

### populate - 批量生成数据

批量生成测试数据，可输出到文件。

```bash
# 生成 50 个用户的数据
mock-server populate -u 50

# 指定每用户订单数量范围并输出到文件
mock-server populate -u 100 --orders 5-20 -o test-data.json
```

参数说明：
- `--users, -u`：用户数量（默认：100）
- `--orders`：每用户订单数量范围，格式 min-max（默认：1-10）
- `--output, -o`：输出文件路径（JSON 格式）

## 预定义场景

| 场景名称 | 描述 |
|---------|------|
| `first_purchase` | 新用户首购场景：浏览 -> 首次购买 -> 签到 |
| `vip_upgrade` | VIP 升级场景：连续 5 次大额购买 |
| `consecutive_checkin` | 连续签到场景：7 天连续签到 |
| `refund_flow` | 退款场景：购买 -> 等待 -> 退款 |
| `active_user` | 活跃用户场景：签到、浏览、购买、分享等多种行为 |
| `social_butterfly` | 社交达人场景：分享到所有社交平台 |

## API 端点

### 健康检查

| 端点 | 方法 | 描述 |
|------|------|------|
| `/health` | GET | 健康检查，返回 `{"status": "healthy"}` |
| `/ready` | GET | 就绪检查，返回 `{"status": "ready", "services": ["order", "profile", "coupon"]}` |

### 订单服务

| 端点 | 方法 | 描述 |
|------|------|------|
| `/orders` | GET | 列出所有订单（支持分页） |
| `/orders` | POST | 创建订单 |
| `/orders/{order_id}` | GET | 获取订单详情 |
| `/orders/{order_id}/status` | POST | 更新订单状态 |
| `/users/{user_id}/orders` | GET | 获取用户订单列表 |

### 用户服务

| 端点 | 方法 | 描述 |
|------|------|------|
| `/users` | GET | 列出所有用户 |
| `/users` | POST | 创建用户 |
| `/users/{user_id}` | GET | 获取用户资料 |
| `/users/{user_id}` | PUT | 更新用户资料 |
| `/users/{user_id}/membership` | GET | 获取会员等级信息 |
| `/users/{user_id}/membership` | POST | 升级会员等级 |

### 优惠券服务

| 端点 | 方法 | 描述 |
|------|------|------|
| `/coupons` | POST | 发放优惠券 |
| `/coupons/batch` | POST | 批量发放优惠券 |
| `/coupons/{coupon_id}` | GET | 获取优惠券详情 |
| `/coupons/{coupon_id}/redeem` | POST | 核销优惠券 |
| `/users/{user_id}/coupons` | GET | 获取用户优惠券列表 |

## 自定义场景文件格式

支持 JSON 和 YAML 格式：

```yaml
name: my_custom_scenario
description: 自定义测试场景
steps:
  - type: purchase
    user_id: user-001
    amount: 100.0
    delay_ms: 100
  - type: wait
    duration_ms: 500
  - type: checkin
    user_id: user-001
    consecutive_days: 1
  - type: share
    user_id: user-001
    platform: wechat
```

支持的步骤类型：
- `purchase`：购买事件
- `checkin`：签到事件
- `refund`：退款事件
- `pageview`：页面浏览事件
- `share`：分享事件
- `wait`：等待延迟
- `repeat`：重复执行

## 全局选项

```bash
mock-server --help

# 可用选项：
#   --log-level, -l    日志级别 (trace, debug, info, warn, error)
#   --kafka-brokers    Kafka brokers 地址（默认：localhost:9092）
```

## 示例

### 完整测试流程

```bash
# 1. 启动服务并预填充数据
mock-server server --populate --user-count 50 &

# 2. 检查服务健康状态
curl http://localhost:8090/health
# {"status":"healthy"}

curl http://localhost:8090/ready
# {"status":"ready","services":["order","profile","coupon"]}

# 3. 查看用户列表
curl http://localhost:8090/users

# 4. 运行测试场景
mock-server scenario -n first_purchase -u test-user-001

# 5. 生成更多事件
mock-server generate -e purchase -u test-user-001 -c 10 --amount 99.99
```

## 依赖

- `axum`：HTTP 框架
- `tokio`：异步运行时
- `rdkafka`：Kafka 客户端
- `clap`：命令行解析
- `serde`/`serde_json`/`serde_yaml`：序列化
- `fake`：测试数据生成
