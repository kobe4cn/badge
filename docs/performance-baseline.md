# 徽章系统 性能基线验证报告

> **报告日期**: _待填入_
> **测试执行人**: _待填入_
> **系统版本**: _待填入 (git commit hash)_
> **报告状态**: 模板（待实际压测后填入数据）

---

## 1. 测试环境

### 1.1 硬件配置

| 组件 | 规格 | 备注 |
|------|------|------|
| 测试机 | _待填入_ (CPU/内存/磁盘) | 压测客户端所在机器 |
| 应用服务器 | _待填入_ | 运行 badge-admin / event 服务 |
| 数据库 | PostgreSQL 15 / _待填入_ (CPU/内存/连接池) | `max_connections` / `shared_buffers` |
| Redis | Redis 7 / _待填入_ (内存) | |
| Kafka | Confluent 7.5 / _待填入_ | 分区数 / 副本数 |

### 1.2 软件配置

| 配置项 | 值 | 说明 |
|--------|-----|------|
| Rust 编译模式 | `--release` | 性能测试必须使用 release 模式 |
| PG 连接池大小 | _待填入_ (`max_connections`) | 当前默认 10，生产建议 ≥ 50 |
| Redis 连接池大小 | _待填入_ | |
| Kafka consumer 线程数 | _待填入_ | |
| Tokio worker 线程数 | _待填入_ (默认 = CPU 核心数) | |
| 服务 JVM/Runtime 参数 | N/A (Rust native) | |

### 1.3 测试工具

| 工具 | 版本 | 用途 |
|------|------|------|
| `scripts/benchmark.sh` | 项目内置 | HTTP API 压测（基于 curl 并发） |
| `cargo bench` (criterion) | 0.8.x | 规则引擎微基准测试 |
| `cargo test --test performance` | 项目内置 | Rust 集成压测框架 |
| wrk (可选) | _待填入_ | 高性能 HTTP 压测 |

---

## 2. 测试方法

### 2.1 压测流程

```
1. 启动完整服务栈（release 模式）
   cargo build --release -p badge-admin-service -p event-transaction-service ...

2. 运行数据库迁移并初始化测试数据
   sqlx migrate run
   psql -f scripts/init_test_data.sql

3. 预热（10-30s 低速请求）

4. 正式压测（各场景分别执行）
   ./scripts/benchmark.sh --duration 60 --concurrency 100 --report results.json

5. 收集指标并生成报告
```

### 2.2 压测场景

| # | 场景 | API 端点 | 方法 | 性能目标 |
|---|------|----------|------|----------|
| S1 | 徽章发放 | `POST /api/admin/grants/manual` | 高并发写入 | ≥ 1000 TPS |
| S2 | 徽章列表查询 | `GET /api/admin/badges` | 高并发只读 | ≥ 5000 QPS |
| S3 | 事件接收 | `POST /api/v1/events` | 高并发写入+Kafka | ≥ 1000 events/s |
| S4 | 用户徽章查询 | `GET /api/admin/users/{id}/badges` | 热点数据只读 | ≥ 2000 QPS |
| S5 | 混合负载 | 多端点混合 | 模拟真实流量 | P99 ≤ 500ms |
| S6 | 规则引擎评估 | 内部调用 (criterion) | CPU 密集型 | ≥ 100K eval/s |

### 2.3 指标定义

| 指标 | 定义 | 采集方式 |
|------|------|----------|
| TPS / QPS | 每秒成功处理的事务/查询数 | `success_count / duration` |
| P50 延迟 | 50% 请求在此延迟内完成 | 排序后取中位数 |
| P95 延迟 | 95% 请求在此延迟内完成 | 排序后取 95 百分位 |
| P99 延迟 | 99% 请求在此延迟内完成 | 排序后取 99 百分位 |
| 错误率 | 非成功响应(非2xx/409)占比 | `failed / total * 100%` |
| CPU 使用率 | 服务进程 CPU 占用 | `top` / `pidstat` |
| 内存使用率 | 服务进程 RSS | `ps aux` / Prometheus |

---

## 3. 测试结果

### 3.1 场景 S1：徽章发放 (POST /api/admin/grants/manual)

**目标: ≥ 1000 TPS, P99 ≤ 100ms, 错误率 ≤ 1%**

| 指标 | 结果 | 目标 | 达标 |
|------|------|------|------|
| 吞吐量 (TPS) | _待填入_ | ≥ 1000 | _待填入_ |
| P50 延迟 | _待填入_ ms | - | - |
| P95 延迟 | _待填入_ ms | - | - |
| P99 延迟 | _待填入_ ms | ≤ 100ms | _待填入_ |
| 平均延迟 | _待填入_ ms | - | - |
| 错误率 | _待填入_ % | ≤ 1% | _待填入_ |
| 总请求数 | _待填入_ | - | - |

**并发配置**: _待填入_ 并发 × _待填入_ 秒

### 3.2 场景 S2：徽章列表查询 (GET /api/admin/badges)

**目标: ≥ 5000 QPS, P99 ≤ 50ms, 错误率 ≤ 0.1%**

| 指标 | 结果 | 目标 | 达标 |
|------|------|------|------|
| 吞吐量 (QPS) | _待填入_ | ≥ 5000 | _待填入_ |
| P50 延迟 | _待填入_ ms | - | - |
| P95 延迟 | _待填入_ ms | - | - |
| P99 延迟 | _待填入_ ms | ≤ 50ms | _待填入_ |
| 平均延迟 | _待填入_ ms | - | - |
| 错误率 | _待填入_ % | ≤ 0.1% | _待填入_ |
| 总请求数 | _待填入_ | - | - |

**并发配置**: _待填入_ 并发 × _待填入_ 秒

### 3.3 场景 S3：事件接收 (POST /api/v1/events)

**目标: ≥ 1000 events/s, P99 ≤ 100ms, 错误率 ≤ 1%**

| 指标 | 结果 | 目标 | 达标 |
|------|------|------|------|
| 吞吐量 (TPS) | _待填入_ | ≥ 1000 | _待填入_ |
| P50 延迟 | _待填入_ ms | - | - |
| P95 延迟 | _待填入_ ms | - | - |
| P99 延迟 | _待填入_ ms | ≤ 100ms | _待填入_ |
| 平均延迟 | _待填入_ ms | - | - |
| 错误率 | _待填入_ % | ≤ 1% | _待填入_ |
| 总请求数 | _待填入_ | - | - |

**并发配置**: _待填入_ 并发 × _待填入_ 秒

### 3.4 场景 S4：用户徽章查询

| 指标 | 结果 | 目标 | 达标 |
|------|------|------|------|
| 吞吐量 (QPS) | _待填入_ | ≥ 2000 | _待填入_ |
| P99 延迟 | _待填入_ ms | ≤ 100ms | _待填入_ |

### 3.5 场景 S5：混合负载

| 指标 | 结果 | 目标 | 达标 |
|------|------|------|------|
| 综合吞吐量 | _待填入_ req/s | ≥ 300 | _待填入_ |
| P99 延迟 | _待填入_ ms | ≤ 500ms | _待填入_ |
| 错误率 | _待填入_ % | ≤ 1% | _待填入_ |

### 3.6 场景 S6：规则引擎微基准 (criterion)

| 测试 | 吞吐量 | 平均延迟 | P99 延迟 |
|------|--------|----------|----------|
| 简单条件评估 | _待填入_ ops/s | _待填入_ μs | _待填入_ μs |
| 复杂嵌套规则 (3×2) | _待填入_ ops/s | _待填入_ μs | _待填入_ μs |
| 批量评估 (100 rules) | _待填入_ ops/s | _待填入_ μs | _待填入_ μs |
| 并发评估 (8 threads) | _待填入_ ops/s | _待填入_ μs | _待填入_ μs |
| 规则存储并发访问 (8 threads) | _待填入_ ops/s | _待填入_ μs | _待填入_ μs |

---

## 4. 资源使用

### 4.1 服务资源消耗（峰值负载时）

| 服务 | CPU 使用率 | 内存 (RSS) | 连接数 | 备注 |
|------|-----------|-----------|--------|------|
| badge-admin-service | _待填入_ % | _待填入_ MB | _待填入_ | HTTP API 网关 |
| badge-management-service | _待填入_ % | _待填入_ MB | _待填入_ | gRPC 后端 |
| unified-rule-engine | _待填入_ % | _待填入_ MB | _待填入_ | gRPC 规则服务 |
| event-engagement-service | _待填入_ % | _待填入_ MB | _待填入_ | Kafka 消费者 |
| PostgreSQL | _待填入_ % | _待填入_ MB | _待填入_ / max | 活跃连接/最大连接 |
| Redis | _待填入_ % | _待填入_ MB | _待填入_ | |
| Kafka | _待填入_ % | _待填入_ MB | - | |

### 4.2 数据库连接池

| 指标 | 值 | 说明 |
|------|-----|------|
| 最大连接数 | _待填入_ | `sqlx::PgPoolOptions::max_connections` |
| 峰值活跃连接 | _待填入_ | 压测期间最大活跃连接数 |
| 等待连接超时次数 | _待填入_ | 连接池耗尽时等待超时的请求数 |
| 平均连接获取时间 | _待填入_ μs | |

---

## 5. 瓶颈分析

### 5.1 已识别瓶颈

| # | 瓶颈 | 影响 | 严重度 | 建议 |
|---|------|------|--------|------|
| 1 | PG 连接池过小 (max=10) | 高并发时连接等待导致延迟飙升 | **P0** | 扩容至 50-100，参见 5.2 |
| 2 | 无读写分离 | 查询和写入竞争同一连接池 | P1 | 引入只读副本，读请求走 replica |
| 3 | 无 Redis 缓存层 | 每次查询都打到 DB | P1 | 热点数据(徽章列表/用户徽章)加 Redis 缓存 |
| 4 | 无熔断/限流 | 突发流量可能压垮后端 | P0 | 参见任务 #3 BenefitService |
| 5 | gRPC 调用无超时兜底 | 规则引擎超时会级联到上游 | P1 | 设置合理的 gRPC deadline |

### 5.2 连接池扩容建议

当前 PG 连接池 `max_connections = 10`，无法支撑 1000 TPS 目标。

**计算方式**:
```
所需连接数 ≈ TPS × 平均事务时间(秒)
1000 TPS × 10ms avg ≈ 10 并发连接（理论最低）
考虑突发和锁等待: 建议 50-100 连接
```

**建议配置**:
```toml
# shared/src/config.rs 或 application.toml
[database]
max_connections = 50      # 从 10 → 50
min_connections = 5       # 保持最少 5 个空闲连接
acquire_timeout = "3s"    # 获取连接超时
idle_timeout = "600s"     # 空闲连接回收
```

---

## 6. 优化建议

### 6.1 短期优化（可立即实施）

| # | 优化项 | 预期收益 | 工作量 |
|---|--------|----------|--------|
| 1 | PG 连接池扩容到 50 | TPS 提升 3-5x | 配置修改 |
| 2 | 徽章列表 Redis 缓存 (TTL=60s) | 查询 QPS 提升 10x+ | 0.5 天 |
| 3 | 批量插入优化（合并 INSERT） | 写入 TPS 提升 2-3x | 1 天 |
| 4 | 启用 gzip 压缩 | 网络传输减少 50-70% | 已配置 tower-http |

### 6.2 中期优化（需架构调整）

| # | 优化项 | 预期收益 | 工作量 |
|---|--------|----------|--------|
| 1 | 读写分离 (PG replica) | 查询 QPS 翻倍 | 2-3 天 |
| 2 | 本地缓存 + Redis 二级缓存 | 热点数据 < 1ms | 1-2 天 |
| 3 | Kafka 分区扩容 + 并行消费 | 事件吞吐量线性扩展 | 1 天 |
| 4 | 连接池预热 | 冷启动延迟降低 | 0.5 天 |

### 6.3 长期优化

- 引入 CQRS 架构（读写模型分离）
- 规则引擎结果缓存（相同上下文 + 规则版本 → 缓存命中）
- 数据库分表（按 user_id hash 分片）
- CDN 加速静态资源

---

## 7. 结论

### 7.1 性能目标达成情况

| 场景 | 目标 | 实测 | 达标 |
|------|------|------|------|
| 徽章发放 TPS | ≥ 1000 | _待填入_ | _待填入_ |
| 徽章查询 QPS | ≥ 5000 | _待填入_ | _待填入_ |
| 事件接收 TPS | ≥ 1000 | _待填入_ | _待填入_ |
| P99 延迟 | ≤ 100ms (API) | _待填入_ | _待填入_ |
| 错误率 | ≤ 1% | _待填入_ | _待填入_ |

### 7.2 综合评估

_待填入：根据实际压测数据，给出以下结论_

- [ ] 系统在当前配置下是否达到 1000 TPS 目标
- [ ] 主要瓶颈在哪个环节（网络/CPU/DB/Kafka）
- [ ] 通过 5.2 所述连接池扩容后，预计能否达标
- [ ] 是否需要进一步的架构优化

### 7.3 建议行动项

1. **立即执行**: PG 连接池扩容至 50（配置变更，零代码）
2. **本周内**: 热点查询加 Redis 缓存
3. **下一迭代**: 读写分离 + 熔断器
4. **上线前**: 使用生产等配硬件重新验证

---

## 附录

### A. 复现步骤

```bash
# 1. 启动基础设施
docker-compose up -d postgres redis kafka

# 2. 编译并启动服务（release 模式）
cargo build --release
./target/release/badge-admin &
./target/release/badge-management &
./target/release/rule-engine &
./target/release/event-engagement &
./target/release/event-transaction &

# 3. 运行压测脚本
./scripts/benchmark.sh --duration 60 --concurrency 100 --report results.json

# 4. 运行 criterion 基准测试
cargo bench

# 5. 运行 Rust 集成压测
cargo test --test performance -- --ignored --test-threads=1
```

### B. JSON 报告示例

压测脚本使用 `--report` 参数可输出 JSON 格式报告：

```json
{
  "timestamp": "2026-02-22T10:00:00",
  "config": {
    "admin_url": "http://localhost:8080",
    "event_url": "http://localhost:8082",
    "duration_secs": 60,
    "concurrency": 100,
    "warmup_secs": 10,
    "tool": "curl"
  },
  "targets": {
    "badge_grant_tps": 1000,
    "badge_query_qps": 5000,
    "event_ingest_tps": 1000,
    "max_error_rate_pct": 1.0
  },
  "results": {
    "badge_query": {
      "throughput": 0,
      "p50_ms": 0,
      "p95_ms": 0,
      "p99_ms": 0,
      "avg_ms": 0,
      "error_rate_pct": 0,
      "total_requests": 0
    }
  }
}
```

### C. CI 集成

性能测试已集成到 `.github/workflows/e2e-tests.yml` 的 `performance` job 中，
手动触发 workflow_dispatch 并选择 `performance` 即可在 CI 中运行。

Criterion 基准测试可通过以下方式在 CI 中运行：

```yaml
- name: Run benchmarks
  run: cargo bench -- --output-format bencher | tee bench-output.txt

- name: Store benchmark result
  uses: benchmark-action/github-action-benchmark@v1
  with:
    tool: 'cargo'
    output-file-path: bench-output.txt
```
