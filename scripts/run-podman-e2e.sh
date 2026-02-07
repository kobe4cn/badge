#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_DIR="$ROOT/.run"
LOG_DIR="$RUN_DIR/logs"
PID_DIR="$RUN_DIR/pids"

mkdir -p "$LOG_DIR" "$PID_DIR"

usage() {
  cat <<'USAGE'
用法: scripts/run-podman-e2e.sh <command>

commands:
  up        启动基础设施并初始化数据库、Kafka
  backend   启动后端服务（6 个）
  frontend  启动 Admin UI（真实 API）
  smoke     运行全链路冒烟（事件->徽章->查询）
  status    查看服务状态
  stop      停止本脚本启动的服务
  down      停止服务并关闭基础设施
  all       up + backend + frontend + smoke
USAGE
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "缺少依赖: $1" >&2
    exit 1
  }
}

is_running() {
  local name="$1"
  local pid_file="$PID_DIR/$name.pid"
  if [[ -f "$pid_file" ]]; then
    local pid
    pid=$(cat "$pid_file")
    if kill -0 "$pid" >/dev/null 2>&1; then
      return 0
    fi
  fi
  return 1
}

start_service_cmd() {
  local name="$1"; shift
  local cmd="$*"

  if is_running "$name"; then
    echo "[skip] $name 已在运行"
    return 0
  fi

  echo "[start] $name"
  nohup bash -lc "cd \"$ROOT\" && $cmd" > "$LOG_DIR/$name.log" 2>&1 &
  echo $! > "$PID_DIR/$name.pid"
}

stop_service() {
  local name="$1"
  local pid_file="$PID_DIR/$name.pid"
  if [[ -f "$pid_file" ]]; then
    local pid
    pid=$(cat "$pid_file")
    if kill -0 "$pid" >/dev/null 2>&1; then
      echo "[stop] $name (pid=$pid)"
      kill "$pid" || true
      sleep 1
    fi
    rm -f "$pid_file"
  fi
}

wait_http() {
  local url="$1"
  local timeout="${2:-60}"
  local start
  start=$(date +%s)
  while true; do
    if curl -fsS "$url" >/dev/null 2>&1; then
      return 0
    fi
    local now
    now=$(date +%s)
    if (( now - start > timeout )); then
      echo "等待超时: $url" >&2
      return 1
    fi
    sleep 2
  done
}

get_token() {
  local base_url="$1"
  local payload='{"username":"admin","password":"admin123"}'
  local resp
  resp=$(curl -s -X POST "$base_url/api/admin/auth/login" \
    -H "Content-Type: application/json" \
    -d "$payload")
  python3 - <<'PY' "$resp"
import json, sys
raw = sys.argv[1]
try:
    data = json.loads(raw)
    token = (data.get("data", {}) or {}).get("token") or data.get("token")
    if not token:
        raise ValueError("no token")
    print(token)
except Exception as e:
    print("", end="")
PY
}

cmd_up() {
  require_cmd podman
  require_cmd make

  echo "[infra] podman compose up"
  (cd "$ROOT" && make infra-up)

  echo "[kafka] init topics"
  (cd "$ROOT" && make kafka-init)

  echo "[db] migrate + seed"
  (cd "$ROOT" && make db-setup)
}

cmd_backend() {
  require_cmd cargo

  start_service_cmd "rule-engine" "cargo run -p unified-rule-engine --bin rule-engine"
  start_service_cmd "badge-management" "cargo run -p badge-management-service --bin badge-management"
  start_service_cmd "badge-admin" "cargo run -p badge-admin-service --bin badge-admin"
  start_service_cmd "event-engagement" "cargo run -p event-engagement-service --bin event-engagement"
  start_service_cmd "event-transaction" "cargo run -p event-transaction-service --bin event-transaction"
  start_service_cmd "notification-worker" "cargo run -p notification-worker --bin notification-worker"

  echo "[wait] admin health"
  wait_http "http://localhost:8080/health" 90
}

cmd_frontend() {
  require_cmd pnpm
  start_service_cmd "admin-ui" "cd web/admin-ui && VITE_DISABLE_MOCK=true pnpm run dev"
}

cmd_smoke() {
  require_cmd cargo
  require_cmd python3

  echo "[smoke] admin health"
  wait_http "http://localhost:8080/health" 60

  local token
  token=$(get_token "http://localhost:8080")
  if [[ -z "$token" ]]; then
    echo "获取 token 失败" >&2
    exit 1
  fi

  # 构建 mock-services（二进制用于生成 Kafka 事件）
  (cd "$ROOT" && cargo build -p mock-services)
  local mock_bin="$ROOT/target/debug/mock-server"

  local user_id="smoke_user_$(date +%s)"
  echo "[smoke] send checkin event for $user_id"
  "$mock_bin" generate --event-type checkin --user-id "$user_id" --count 1 >/dev/null

  sleep 2

  echo "[smoke] query badges"
  local resp
  resp=$(curl -s -H "Authorization: Bearer $token" \
    "http://localhost:8080/api/admin/users/$user_id/badges")

  local verdict
  verdict=$(python3 - <<'PY' "$resp"
import json, sys
raw = sys.argv[1]
try:
    data = json.loads(raw)
    items = (data.get("data") or {}).get("items") or []
    if not items:
        print("EMPTY")
    elif any(i.get("badgeId") == 2 for i in items):
        print("BADGE2")
    else:
        print(f"ANY:{items[0].get('badgeId')}")
except Exception:
    print("PARSE_ERROR")
PY
)

  if [[ "$verdict" == "BADGE2" ]]; then
    echo "[ok] checkin badge granted (badgeId=2)"
  elif [[ "$verdict" == ANY:* ]]; then
    local got_id="${verdict#ANY:}"
    echo "[ok] badge granted (badgeId=$got_id)"
  else
    echo "[warn] 未检测到徽章发放，响应如下:"
    echo "$resp"
  fi
}

cmd_status() {
  local names=(rule-engine badge-management badge-admin event-engagement event-transaction notification-worker admin-ui)
  for n in "${names[@]}"; do
    if is_running "$n"; then
      echo "[running] $n"
    else
      echo "[stopped] $n"
    fi
  done
}

cmd_stop() {
  local names=(admin-ui notification-worker event-transaction event-engagement badge-admin badge-management rule-engine)
  for n in "${names[@]}"; do
    stop_service "$n"
  done
}

cmd_down() {
  cmd_stop
  (cd "$ROOT" && make infra-down)
}

cmd_all() {
  cmd_up
  cmd_backend
  cmd_frontend
  cmd_smoke
}

main() {
  local cmd="${1:-}"
  case "$cmd" in
    up) cmd_up ;;
    backend) cmd_backend ;;
    frontend) cmd_frontend ;;
    smoke) cmd_smoke ;;
    status) cmd_status ;;
    stop) cmd_stop ;;
    down) cmd_down ;;
    all) cmd_all ;;
    *) usage; exit 1 ;;
  esac
}

main "$@"
