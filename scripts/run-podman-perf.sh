#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# 可通过环境变量覆盖
EVENT_TYPE="${EVENT_TYPE:-purchase}"
TOTAL_USERS="${TOTAL_USERS:-1000}"
EVENTS_PER_USER="${EVENTS_PER_USER:-1}"
CONCURRENCY="${CONCURRENCY:-10}"
AMOUNT="${AMOUNT:-100}"
ADMIN_URL="${ADMIN_URL:-http://localhost:8080}"
WAIT_SECS="${WAIT_SECS:-5}"

usage() {
  cat <<'USAGE'
用法: scripts/run-podman-perf.sh

环境变量:
  EVENT_TYPE     事件类型（purchase/checkin/share...）
  TOTAL_USERS    用户数（默认 1000）
  EVENTS_PER_USER 每用户事件数（默认 1）
  CONCURRENCY    并发数（默认 10）
  AMOUNT         purchase 事件金额（默认 100）
  ADMIN_URL      管理端地址（默认 http://localhost:8080）
  WAIT_SECS      发送后等待处理时间（默认 5 秒）

示例:
  TOTAL_USERS=10000 CONCURRENCY=50 EVENT_TYPE=purchase ./scripts/run-podman-perf.sh
USAGE
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "缺少依赖: $1" >&2
    exit 1
  }
}

main() {
  require_cmd cargo
  require_cmd python3
  require_cmd xargs

  echo "[build] mock-services"
  (cd "$ROOT" && cargo build -p mock-services)
  local mock_bin="$ROOT/target/debug/mock-server"

  local total_events
  total_events=$((TOTAL_USERS * EVENTS_PER_USER))

  echo "[perf] EVENT_TYPE=$EVENT_TYPE TOTAL_USERS=$TOTAL_USERS EVENTS_PER_USER=$EVENTS_PER_USER CONCURRENCY=$CONCURRENCY"

  local start_ts
  start_ts=$(date +%s)

  seq 1 "$TOTAL_USERS" | xargs -P "$CONCURRENCY" -I{} bash -lc "\
    '$mock_bin' generate --event-type '$EVENT_TYPE' --user-id 'perf_user_{}' --count '$EVENTS_PER_USER' --amount '$AMOUNT' >/dev/null\
  "

  local end_ts
  end_ts=$(date +%s)
  local duration
  duration=$((end_ts - start_ts))
  if (( duration == 0 )); then duration=1; fi

  local throughput
  throughput=$(python3 - <<PY "$total_events" "$duration"
import sys
n = float(sys.argv[1])
d = float(sys.argv[2])
print(f"{n/d:.2f}")
PY
  )

  echo "[perf] 发送完成: total_events=$total_events duration=${duration}s throughput=${throughput} events/s"

  echo "[perf] 等待处理 $WAIT_SECS 秒..."
  sleep "$WAIT_SECS"

  # 抽样验证最后一个用户
  local user_id="perf_user_${TOTAL_USERS}"
  local token
  token=$(curl -s -X POST "$ADMIN_URL/api/admin/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"admin","password":"admin123"}' | \
    python3 - <<'PY'
import json, sys
try:
    data = json.load(sys.stdin)
    print((data.get('data', {}) or {}).get('token') or data.get('token') or '')
except Exception:
    print('')
PY
  )

  if [[ -n "$token" ]]; then
    local resp
    resp=$(curl -s -H "Authorization: Bearer $token" "$ADMIN_URL/api/admin/users/$user_id/badges")
    if echo "$resp" | grep -q '"badgeId"'; then
      echo "[perf] 抽样用户徽章查询成功: $user_id"
    else
      echo "[perf][warn] 抽样用户徽章查询无结果: $user_id"
      echo "$resp"
    fi
  else
    echo "[perf][warn] 登录失败，跳过抽样验证"
  fi
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

main "$@"
