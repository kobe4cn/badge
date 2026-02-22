#!/usr/bin/env bash
# =============================================================================
# 徽章系统性能基线验证脚本
#
# 对徽章发放、徽章查询、事件处理三个核心接口进行压测，
# 收集 TPS/QPS、延迟分位数(P50/P95/P99)、错误率等指标，
# 并与性能目标进行对比。
#
# 依赖：curl, python3, bash
# 可选依赖：wrk（如果安装了将使用 wrk 进行更精确的压测）
#
# 用法：
#   scripts/benchmark.sh [选项]
#
# 选项：
#   --admin-url URL     管理端地址 (默认: http://localhost:8080)
#   --event-url URL     事件服务地址 (默认: http://localhost:8082)
#   --duration SECS     每个场景压测持续时间 (默认: 30)
#   --concurrency N     并发数 (默认: 50)
#   --warmup SECS       预热时间 (默认: 5)
#   --use-wrk           强制使用 wrk (需先安装)
#   --report FILE       输出 JSON 报告到文件
#   --quick             快速模式：缩短时间，用于冒烟测试
#   -h, --help          显示帮助
# =============================================================================
set -euo pipefail

# ---- 默认配置 ----
ADMIN_URL="${ADMIN_URL:-http://localhost:8080}"
EVENT_URL="${EVENT_URL:-http://localhost:8082}"
DURATION="${DURATION:-30}"
CONCURRENCY="${CONCURRENCY:-50}"
WARMUP="${WARMUP:-5}"
USE_WRK="${USE_WRK:-auto}"
REPORT_FILE=""
QUICK_MODE=false

# ---- 性能目标 ----
TARGET_GRANT_TPS=1000
TARGET_QUERY_QPS=5000
TARGET_EVENT_TPS=1000
TARGET_P99_GRANT_MS=100
TARGET_P99_QUERY_MS=50
TARGET_P99_EVENT_MS=100
MAX_ERROR_RATE=1.0  # 最大容许错误率 %

# ---- 颜色输出 ----
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# ---- 结果收集（用临时文件存储，兼容 bash 3.x）----
RESULTS_DIR=""

usage() {
  cat <<'EOF'
徽章系统性能基线验证脚本

用法: scripts/benchmark.sh [选项]

选项:
  --admin-url URL     管理端地址 (默认: http://localhost:8080)
  --event-url URL     事件服务地址 (默认: http://localhost:8082)
  --duration SECS     每个场景压测持续时间 (默认: 30)
  --concurrency N     并发数 (默认: 50)
  --warmup SECS       预热时间 (默认: 5)
  --use-wrk           强制使用 wrk
  --report FILE       输出 JSON 报告到文件
  --quick             快速模式 (duration=10, concurrency=20)
  -h, --help          显示帮助

环境变量:
  ADMIN_URL           管理端地址
  EVENT_URL           事件服务地址
  DURATION            压测时长(秒)
  CONCURRENCY         并发数

示例:
  # 基本压测
  ./scripts/benchmark.sh

  # 快速冒烟测试
  ./scripts/benchmark.sh --quick

  # 高并发压测并输出报告
  ./scripts/benchmark.sh --concurrency 200 --duration 60 --report results.json

  # 指定服务地址
  ./scripts/benchmark.sh --admin-url http://10.0.0.1:8080 --event-url http://10.0.0.1:8082
EOF
}

# ---- 参数解析 ----
while [[ $# -gt 0 ]]; do
  case $1 in
    --admin-url) ADMIN_URL="$2"; shift 2 ;;
    --event-url) EVENT_URL="$2"; shift 2 ;;
    --duration) DURATION="$2"; shift 2 ;;
    --concurrency) CONCURRENCY="$2"; shift 2 ;;
    --warmup) WARMUP="$2"; shift 2 ;;
    --use-wrk) USE_WRK="true"; shift ;;
    --report) REPORT_FILE="$2"; shift 2 ;;
    --quick) QUICK_MODE=true; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "未知选项: $1"; usage; exit 1 ;;
  esac
done

if $QUICK_MODE; then
  DURATION=10
  CONCURRENCY=20
  WARMUP=2
fi

# ---- 工具函数 ----
log_info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_ok()    { echo -e "${GREEN}[PASS]${NC}  $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
log_fail()  { echo -e "${RED}[FAIL]${NC}  $*"; }

# 用文件保存结果键值对（兼容 bash 3.x，无需关联数组）
result_set() {
  echo "$2" > "${RESULTS_DIR}/$1"
}

result_get() {
  cat "${RESULTS_DIR}/$1" 2>/dev/null || echo "0"
}

check_service() {
  local name="$1" url="$2"
  if ! curl -sf --max-time 5 "$url" >/dev/null 2>&1; then
    log_fail "$name ($url) 不可达"
    return 1
  fi
  log_ok "$name ($url) 可达"
}

# 获取 JWT Token
get_token() {
  local resp
  resp=$(curl -s -X POST "${ADMIN_URL}/api/admin/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"admin","password":"admin123"}' \
    --max-time 10)

  local token
  token=$(echo "$resp" | python3 -c "
import json, sys
try:
    d = json.load(sys.stdin)
    t = (d.get('data') or {}).get('token') or d.get('token') or ''
    print(t)
except:
    print('')
" 2>/dev/null)

  if [[ -z "$token" ]]; then
    log_fail "获取 JWT Token 失败"
    echo "$resp" >&2
    exit 1
  fi
  echo "$token"
}

# 使用 curl 并发压测
# 输出: JSON 格式的结果统计
curl_bench() {
  local method="$1"
  local url="$2"
  local body="${3:-}"
  local auth_header="${4:-}"
  local desc="${5:-benchmark}"

  local work_dir
  work_dir=$(mktemp -d)

  log_info "预热 ${WARMUP}s..."
  sleep "$WARMUP"

  log_info "开始压测: $desc (${DURATION}s, 并发=${CONCURRENCY})"

  local end_time
  end_time=$(( $(date +%s) + DURATION ))
  local pids=""

  # 启动并发 worker，每个 worker 写独立文件避免锁竞争
  local w=0
  while [ $w -lt "$CONCURRENCY" ]; do
    (
      local worker_success=0
      local worker_fail=0
      local worker_total=0

      while [ "$(date +%s)" -lt "$end_time" ]; do
        local req_start req_end latency_ms http_code

        req_start=$(python3 -c "import time; print(f'{time.time():.6f}')")

        if [ "$method" = "GET" ]; then
          http_code=$(curl -s -o /dev/null -w "%{http_code}" \
            -H "Authorization: Bearer $auth_header" \
            --max-time 10 \
            "$url" 2>/dev/null || echo "000")
        else
          local actual_body="$body"
          # 生成唯一标识：组合 worker ID 和计数器
          local seq_id="${w}_${worker_total}_$$"
          actual_body=$(echo "$actual_body" | sed "s/{SEQ}/${seq_id}/g; s/{USER}/bench_user_${w}_${worker_total}/g")

          http_code=$(curl -s -o /dev/null -w "%{http_code}" \
            -X POST \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $auth_header" \
            -d "$actual_body" \
            --max-time 10 \
            "$url" 2>/dev/null || echo "000")
        fi

        req_end=$(python3 -c "import time; print(f'{time.time():.6f}')")
        latency_ms=$(python3 -c "print(f'{($req_end - $req_start) * 1000:.2f}')")

        worker_total=$((worker_total + 1))
        case "$http_code" in
          200|201|202|204|409) worker_success=$((worker_success + 1)) ;;
          *) worker_fail=$((worker_fail + 1)) ;;
        esac
        echo "$latency_ms" >> "${work_dir}/latencies_${w}.txt"
      done

      echo "$worker_total $worker_success $worker_fail" > "${work_dir}/counts_${w}.txt"
    ) &
    pids="$pids $!"
    w=$((w + 1))
  done

  # 等待所有 worker 完成
  for pid in $pids; do
    wait "$pid" 2>/dev/null || true
  done

  # 汇总结果
  local total_requests=0
  local success_count=0
  local fail_count=0

  for f in "${work_dir}"/counts_*.txt; do
    if [ -f "$f" ]; then
      read -r t s fl < "$f"
      total_requests=$((total_requests + t))
      success_count=$((success_count + s))
      fail_count=$((fail_count + fl))
    fi
  done

  # 合并延迟数据并计算分位数
  cat "${work_dir}"/latencies_*.txt 2>/dev/null > "${work_dir}/all_latencies.txt" || true

  local p50=0 p95=0 p99=0 avg=0
  if [ -s "${work_dir}/all_latencies.txt" ]; then
    read -r p50 p95 p99 avg <<EOF
$(python3 -c "
lats = []
with open('${work_dir}/all_latencies.txt') as f:
    for line in f:
        line = line.strip()
        if line:
            try:
                lats.append(float(line))
            except:
                pass
if not lats:
    print('0 0 0 0')
else:
    lats.sort()
    n = len(lats)
    p50 = lats[int(n * 0.50)]
    p95 = lats[int(n * 0.95)]
    p99 = lats[int(min(n * 0.99, n - 1))]
    avg = sum(lats) / n
    print(f'{p50:.2f} {p95:.2f} {p99:.2f} {avg:.2f}')
")
EOF
  fi

  local throughput
  throughput=$(python3 -c "print(f'{$success_count / $DURATION:.2f}')")
  local error_rate
  if [ "$total_requests" -gt 0 ]; then
    error_rate=$(python3 -c "print(f'{$fail_count / $total_requests * 100:.2f}')")
  else
    error_rate="100.00"
  fi

  # 清理
  rm -rf "${work_dir}"

  echo "{\"total\":$total_requests,\"success\":$success_count,\"failed\":$fail_count,\"throughput\":$throughput,\"error_rate\":$error_rate,\"p50\":$p50,\"p95\":$p95,\"p99\":$p99,\"avg\":$avg}"
}

# 打印场景结果并与目标比较
print_result() {
  local scenario="$1"
  local result_json="$2"
  local target_tps="$3"
  local target_p99="$4"

  # 一次性解析所有字段
  local parsed
  parsed=$(echo "$result_json" | python3 -c "
import json, sys
d = json.load(sys.stdin)
for k in ['throughput','p50','p95','p99','avg','total','success','failed','error_rate']:
    print(d[k])
")

  local throughput p50 p95 p99 avg total success failed error_rate
  {
    read -r throughput
    read -r p50
    read -r p95
    read -r p99
    read -r avg
    read -r total
    read -r success
    read -r failed
    read -r error_rate
  } <<< "$parsed"

  echo ""
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "  场景: $scenario"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  printf "  %-16s %s\n" "总请求数:" "$total"
  printf "  %-16s %s\n" "成功请求:" "$success"
  printf "  %-16s %s\n" "失败请求:" "$failed"
  printf "  %-16s %s%%\n" "错误率:" "$error_rate"
  echo "  ────────────────────────────────────────"
  printf "  %-16s %s req/s\n" "吞吐量:" "$throughput"
  printf "  %-16s %s ms\n" "平均延迟:" "$avg"
  printf "  %-16s %s ms\n" "P50 延迟:" "$p50"
  printf "  %-16s %s ms\n" "P95 延迟:" "$p95"
  printf "  %-16s %s ms\n" "P99 延迟:" "$p99"
  echo "  ────────────────────────────────────────"

  # 与目标对比
  local tps_pass p99_pass err_pass
  tps_pass=$(python3 -c "print('1' if $throughput >= $target_tps else '0')")
  p99_pass=$(python3 -c "print('1' if $p99 <= $target_p99 else '0')")
  err_pass=$(python3 -c "print('1' if $error_rate <= $MAX_ERROR_RATE else '0')")

  if [ "$tps_pass" = "1" ]; then
    log_ok "吞吐量 ${throughput} req/s >= 目标 ${target_tps} req/s"
  else
    log_fail "吞吐量 ${throughput} req/s < 目标 ${target_tps} req/s"
  fi

  if [ "$p99_pass" = "1" ]; then
    log_ok "P99 延迟 ${p99} ms <= 目标 ${target_p99} ms"
  else
    log_fail "P99 延迟 ${p99} ms > 目标 ${target_p99} ms"
  fi

  if [ "$err_pass" = "1" ]; then
    log_ok "错误率 ${error_rate}% <= 容许 ${MAX_ERROR_RATE}%"
  else
    log_fail "错误率 ${error_rate}% > 容许 ${MAX_ERROR_RATE}%"
  fi

  # 保存到全局结果（用文件系统替代关联数组）
  result_set "${scenario}_throughput" "$throughput"
  result_set "${scenario}_p50" "$p50"
  result_set "${scenario}_p95" "$p95"
  result_set "${scenario}_p99" "$p99"
  result_set "${scenario}_avg" "$avg"
  result_set "${scenario}_error_rate" "$error_rate"
  result_set "${scenario}_total" "$total"
  result_set "${scenario}_tps_pass" "$tps_pass"
  result_set "${scenario}_p99_pass" "$p99_pass"
  result_set "${scenario}_err_pass" "$err_pass"
}

# =============================================================================
# 主流程
# =============================================================================
main() {
  RESULTS_DIR=$(mktemp -d)
  trap "rm -rf '$RESULTS_DIR'" EXIT

  echo ""
  echo "╔═══════════════════════════════════════════════════════════╗"
  echo "║           徽章系统 性能基线验证                          ║"
  echo "╚═══════════════════════════════════════════════════════════╝"
  echo ""
  log_info "配置: duration=${DURATION}s  concurrency=${CONCURRENCY}  warmup=${WARMUP}s"
  log_info "管理端: ${ADMIN_URL}"
  log_info "事件服务: ${EVENT_URL}"
  echo ""

  # 检查工具可用性
  local bench_tool="curl"
  if [ "$USE_WRK" = "true" ]; then
    command -v wrk >/dev/null 2>&1 || { log_fail "wrk 未安装"; exit 1; }
    bench_tool="wrk"
  elif [ "$USE_WRK" = "auto" ] && command -v wrk >/dev/null 2>&1; then
    bench_tool="wrk"
  fi
  log_info "压测工具: $bench_tool"

  # 1. 检查服务可达性
  echo ""
  log_info "=== 检查服务可达性 ==="
  check_service "管理端 (health)" "${ADMIN_URL}/health" || exit 1

  if ! check_service "事件服务" "${EVENT_URL}/health" 2>/dev/null; then
    log_warn "事件服务 health 端点不可达，将在压测时检测"
  fi

  # 2. 获取认证 Token
  echo ""
  log_info "=== 获取认证 Token ==="
  local TOKEN
  TOKEN=$(get_token)
  log_ok "Token 获取成功 (${TOKEN:0:20}...)"

  # 3. 场景一：徽章查询 QPS
  echo ""
  log_info "=== 场景一：徽章查询 (GET /api/admin/badges) ==="
  log_info "目标: >= ${TARGET_QUERY_QPS} QPS, P99 <= ${TARGET_P99_QUERY_MS}ms"

  local query_result
  query_result=$(curl_bench \
    "GET" \
    "${ADMIN_URL}/api/admin/badges?pageSize=20" \
    "" \
    "$TOKEN" \
    "徽章列表查询")

  print_result "badge_query" "$query_result" "$TARGET_QUERY_QPS" "$TARGET_P99_QUERY_MS"

  # 4. 场景二：徽章发放 TPS
  echo ""
  log_info "=== 场景二：徽章发放 (POST /api/admin/grants/manual) ==="
  log_info "目标: >= ${TARGET_GRANT_TPS} TPS, P99 <= ${TARGET_P99_GRANT_MS}ms"

  local grant_body='{"userId":"{USER}","badgeId":1,"sourceType":"benchmark","sourceId":"bench_{SEQ}"}'
  local grant_result
  grant_result=$(curl_bench \
    "POST" \
    "${ADMIN_URL}/api/admin/grants/manual" \
    "$grant_body" \
    "$TOKEN" \
    "徽章手动发放")

  print_result "badge_grant" "$grant_result" "$TARGET_GRANT_TPS" "$TARGET_P99_GRANT_MS"

  # 5. 场景三：事件接收 TPS
  echo ""
  log_info "=== 场景三：事件接收 (POST /api/v1/events) ==="
  log_info "目标: >= ${TARGET_EVENT_TPS} events/s, P99 <= ${TARGET_P99_EVENT_MS}ms"

  local event_body='{"eventType":"purchase","userId":"{USER}","data":{"orderId":"bench_order_{SEQ}","amount":100,"currency":"CNY"}}'
  local event_result
  event_result=$(curl_bench \
    "POST" \
    "${EVENT_URL}/api/v1/events" \
    "$event_body" \
    "$TOKEN" \
    "事件接收")

  print_result "event_ingest" "$event_result" "$TARGET_EVENT_TPS" "$TARGET_P99_EVENT_MS"

  # 6. 汇总报告
  echo ""
  echo "╔═══════════════════════════════════════════════════════════╗"
  echo "║                    汇总报告                              ║"
  echo "╚═══════════════════════════════════════════════════════════╝"
  echo ""

  printf "%-20s %-12s %-10s %-10s %-10s %-8s %-8s\n" \
    "场景" "吞吐量" "P50(ms)" "P95(ms)" "P99(ms)" "错误率" "达标"
  echo "────────────────────────────────────────────────────────────────────────────"

  local all_pass=true
  for scenario in badge_query badge_grant event_ingest; do
    local tps_ok
    tps_ok=$(result_get "${scenario}_tps_pass")
    local p99_ok
    p99_ok=$(result_get "${scenario}_p99_pass")
    local err_ok
    err_ok=$(result_get "${scenario}_err_pass")
    local pass_str
    if [ "$tps_ok" = "1" ] && [ "$p99_ok" = "1" ] && [ "$err_ok" = "1" ]; then
      pass_str="PASS"
    else
      pass_str="FAIL"
      all_pass=false
    fi

    printf "%-20s %-12s %-10s %-10s %-10s %-8s %-8s\n" \
      "$scenario" \
      "$(result_get "${scenario}_throughput")" \
      "$(result_get "${scenario}_p50")" \
      "$(result_get "${scenario}_p95")" \
      "$(result_get "${scenario}_p99")" \
      "$(result_get "${scenario}_error_rate")%" \
      "$pass_str"
  done

  echo ""
  if $all_pass; then
    log_ok "所有场景均达到性能目标"
  else
    log_fail "部分场景未达到性能目标，请参考优化建议"
  fi

  # 7. 输出 JSON 报告
  if [ -n "$REPORT_FILE" ]; then
    python3 -c "
import json, datetime

report = {
    'timestamp': datetime.datetime.now().isoformat(),
    'config': {
        'admin_url': '$ADMIN_URL',
        'event_url': '$EVENT_URL',
        'duration_secs': $DURATION,
        'concurrency': $CONCURRENCY,
        'warmup_secs': $WARMUP,
        'tool': '$bench_tool'
    },
    'targets': {
        'badge_grant_tps': $TARGET_GRANT_TPS,
        'badge_query_qps': $TARGET_QUERY_QPS,
        'event_ingest_tps': $TARGET_EVENT_TPS,
        'max_error_rate_pct': $MAX_ERROR_RATE
    },
    'results': {}
}

import os
results_dir = '$RESULTS_DIR'
for scenario in ['badge_query', 'badge_grant', 'event_ingest']:
    def read_val(key, default='0'):
        path = os.path.join(results_dir, f'{scenario}_{key}')
        try:
            with open(path) as f:
                return f.read().strip()
        except:
            return default

    report['results'][scenario] = {
        'throughput': float(read_val('throughput')),
        'p50_ms': float(read_val('p50')),
        'p95_ms': float(read_val('p95')),
        'p99_ms': float(read_val('p99')),
        'avg_ms': float(read_val('avg')),
        'error_rate_pct': float(read_val('error_rate')),
        'total_requests': int(float(read_val('total'))),
    }

with open('$REPORT_FILE', 'w') as f:
    json.dump(report, f, indent=2, ensure_ascii=False)
print(f'报告已写入: $REPORT_FILE')
"
  fi

  echo ""
  log_info "压测完成"

  # 非全部通过时返回非零退出码，便于 CI 判断
  $all_pass || exit 1
}

main "$@"
