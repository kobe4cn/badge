#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
ADMIN_USER="${ADMIN_USER:-admin}"
ADMIN_PASS="${ADMIN_PASS:-admin123}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

log() {
  printf "[%s] %s\n" "$(date +%H:%M:%S)" "$*"
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "缺少依赖: $1" >&2
    exit 1
  }
}

json_get() {
  local json="$1"
  local path="$2"
  python3 - <<'PY' "$json" "$path"
import json, sys
raw = sys.argv[1]
path = sys.argv[2].split('.') if sys.argv[2] else []
try:
    data = json.loads(raw)
except Exception:
    print("")
    sys.exit(0)
cur = data
for p in path:
    if p == "":
        continue
    if isinstance(cur, list):
        try:
            cur = cur[int(p)]
        except Exception:
            cur = None
    elif isinstance(cur, dict):
        cur = cur.get(p)
    else:
        cur = None
    if cur is None:
        break
if cur is None:
    print("")
elif isinstance(cur, (dict, list)):
    print(json.dumps(cur, ensure_ascii=False))
else:
    print(cur)
PY
}

assert_success() {
  local json="$1"
  python3 - <<'PY' "$json"
import json, sys
raw = sys.argv[1]
try:
    data = json.loads(raw)
except Exception as e:
    print(f"响应不是合法 JSON: {e}", file=sys.stderr)
    print(f"原始响应: {raw!r}", file=sys.stderr)
    sys.exit(1)
if not data.get("success", False):
    print("API 返回失败:", data, file=sys.stderr)
    sys.exit(1)
PY
}

soft_check_success() {
  local json="$1"
  python3 - <<'PY' "$json"
import json, sys
raw = sys.argv[1]
try:
    data = json.loads(raw)
except Exception as e:
    print(f"[warn] 响应不是合法 JSON: {e}")
    print(f"[warn] 原始响应: {raw!r}")
    sys.exit(0)
if not data.get("success", False):
    print("[warn] API 返回失败(允许 mock/跳过):", data)
PY
}

api() {
  local method="$1"
  local path="$2"
  local data="${3:-}"
  local url="$BASE_URL$path"
  if [[ "$method" == "GET" ]]; then
    curl -sS -H "Authorization: Bearer $TOKEN" "$url"
  else
    curl -sS -X "$method" -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
      -d "$data" "$url"
  fi
}

wait_for_badge() {
  local user_id="$1"
  local badge_id="$2"
  local timeout_secs="${3:-20}"
  local end=$((SECONDS + timeout_secs))
  while (( SECONDS < end )); do
    local resp
    resp=$(api GET "/api/admin/users/$user_id/badges?page=1&pageSize=50")
    if python3 - <<'PY' "$resp" "$badge_id"
import json, sys
raw = sys.argv[1]
badge_id = str(sys.argv[2])
try:
    data = json.loads(raw)
except Exception:
    sys.exit(1)
items = (data.get("data") or {}).get("items") or []
if any(str(i.get("badgeId")) == badge_id for i in items):
    sys.exit(0)
sys.exit(1)
PY
    then
      return 0
    fi
    sleep 2
  done
  return 1
}

require_cmd curl
require_cmd python3
require_cmd cargo
require_cmd podman

log "健康检查..."
if ! curl -fsS "$BASE_URL/health" >/dev/null 2>&1; then
  echo "健康检查失败: $BASE_URL/health" >&2
  exit 1
fi

log "登录获取 token..."
login_resp=$(curl -sS -X POST "$BASE_URL/api/admin/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"$ADMIN_USER\",\"password\":\"$ADMIN_PASS\"}")
TOKEN=$(python3 - <<'PY' "$login_resp"
import json, sys
raw = sys.argv[1]
try:
    data = json.loads(raw)
    token = (data.get("data") or {}).get("token") or data.get("token")
    if not token:
        raise ValueError("no token")
    print(token)
except Exception:
    print("")
PY
)
if [[ -z "$TOKEN" ]]; then
  echo "登录失败: $login_resp" >&2
  exit 1
fi

log "读取基础数据(分类/系列/徽章/规则/模板/事件类型)..."
assert_success "$(api GET "/api/admin/categories?page=1&pageSize=5")"
assert_success "$(api GET "/api/admin/series?page=1&pageSize=5")"
assert_success "$(api GET "/api/admin/badges?page=1&pageSize=5")"
assert_success "$(api GET "/api/admin/rules?page=1&pageSize=5")"
assert_success "$(api GET "/api/admin/templates?page=1&pageSize=5")"
assert_success "$(api GET "/api/admin/event-types")"

log "创建分类/系列/徽章并发布..."
ts=$(date +%s)
cat_resp=$(api POST "/api/admin/categories" "{\"name\":\"E2E分类-$ts\",\"iconUrl\":\"https://cdn.example.com/cat/e2e.png\",\"sortOrder\":99}")
assert_success "$cat_resp"
cat_id=$(json_get "$cat_resp" "data.id")

series_resp=$(api POST "/api/admin/series" "{\"categoryId\":$cat_id,\"name\":\"E2E系列-$ts\",\"description\":\"e2e\"}")
assert_success "$series_resp"
series_id=$(json_get "$series_resp" "data.id")

badge_payload=$(cat <<JSON
{"seriesId":$series_id,"badgeType":"NORMAL","name":"E2E徽章-$ts","description":"e2e","obtainDescription":"e2e","assets":{"iconUrl":"https://cdn.example.com/badge/e2e.png"},"validityConfig":{"validityType":"PERMANENT"}}
JSON
)
badge_resp=$(api POST "/api/admin/badges" "$badge_payload")
assert_success "$badge_resp"
badge_id=$(json_get "$badge_resp" "data.id")
assert_success "$(api POST "/api/admin/badges/$badge_id/publish" "{}")"

log "创建规则并测试..."
rule_payload=$(cat <<JSON
{"badgeId":$badge_id,"ruleCode":"e2e_rule_$ts","name":"E2E规则-$ts","eventType":"checkin","ruleJson":{"type":"condition","field":"location","operator":"eq","value":"app"}}
JSON
)
rule_resp=$(api POST "/api/admin/rules" "$rule_payload")
assert_success "$rule_resp"
rule_id=$(json_get "$rule_resp" "data.id")

rule_test_resp=$(api POST "/api/admin/rules/$rule_id/test" "{\"context\":{\"location\":\"app\"}}")
assert_success "$rule_test_resp"

log "构建 mock-server 并发送事件..."
(cd "$ROOT" && cargo build -p mock-services >/dev/null)
mock_bin="$ROOT/target/debug/mock-server"

user_event="e2e_event_$ts"
"$mock_bin" generate --event-type checkin --user-id "$user_event" --count 1 >/dev/null
sleep 2
if ! wait_for_badge "$user_event" 2 20; then
  echo "checkin 未发放 badgeId=2" >&2
  exit 1
fi

"$mock_bin" generate --event-type share --user-id "$user_event" --count 1 >/dev/null
sleep 2
if ! wait_for_badge "$user_event" 6 20; then
  echo "share 未发放 badgeId=6" >&2
  exit 1
fi

"$mock_bin" generate --event-type purchase --user-id "$user_event" --count 1 --amount 150 >/dev/null
sleep 2
if ! wait_for_badge "$user_event" 3 20; then
  echo "purchase 未发放 badgeId=3" >&2
  exit 1
fi

log "检查级联徽章(互动KOC badgeId=8)..."
if ! wait_for_badge "$user_event" 8 30; then
  echo "级联徽章 badgeId=8 未发放" >&2
  exit 1
fi

log "执行兑换(允许 mock 失败)..."
redeem_resp=$(api POST "/api/admin/redemption/redeem" "{\"userId\":\"$user_event\",\"ruleId\":1}")
soft_check_success "$redeem_resp"

log "通知配置与测试发送(允许 mock 失败)..."
notify_resp=$(api POST "/api/admin/notification-configs" "{\"badgeId\":2,\"triggerType\":\"grant\",\"channels\":[\"in_app\"],\"retryCount\":1,\"retryIntervalSeconds\":30}")
soft_check_success "$notify_resp"

notify_test_resp=$(api POST "/api/admin/notification-configs/test" "{\"userId\":\"$user_event\",\"channels\":[\"in_app\"]}")
soft_check_success "$notify_test_resp"

log "手动发放 + 撤销..."
manual_user="e2e_manual_$ts"
manual_grant_resp=$(api POST "/api/admin/grants/manual" "{\"userId\":\"$manual_user\",\"badgeId\":1,\"quantity\":1,\"reason\":\"e2e手动发放\"}")
assert_success "$manual_grant_resp"

user_badge_id=$(podman exec -i badge-postgres psql -U badge -d badge_db -t -A -c "SELECT id FROM user_badges WHERE user_id='${manual_user}' AND badge_id=1 ORDER BY id DESC LIMIT 1" | tr -d ' ')
if [[ -z "$user_badge_id" ]]; then
  echo "无法获取 user_badge_id" >&2
  exit 1
fi

manual_revoke_resp=$(api POST "/api/admin/revokes/manual" "{\"userBadgeId\":$user_badge_id,\"reason\":\"e2e手动撤销\"}")
assert_success "$manual_revoke_resp"

log "批量任务(发放/撤销)..."
batch_users=$(cat <<JSON
["e2e_batch_${ts}_1","e2e_batch_${ts}_2"]
JSON
)
create_task_payload=$(cat <<JSON
{"taskType":"batch_grant","params":{"badge_id":2,"reason":"e2e批量发放","user_ids":$batch_users}}
JSON
)
create_task_resp=$(api POST "/api/admin/tasks" "$create_task_payload")
assert_success "$create_task_resp"
task_id=$(json_get "$create_task_resp" "data.id")

log "等待批量发放任务完成..."
end=$((SECONDS + 60))
while (( SECONDS < end )); do
  task_resp=$(api GET "/api/admin/tasks/$task_id")
  assert_success "$task_resp"
  status=$(json_get "$task_resp" "data.status")
  if [[ "$status" == "completed" ]]; then
    break
  fi
  if [[ "$status" == "failed" ]]; then
    echo "批量发放任务失败: $task_resp" >&2
    exit 1
  fi
  sleep 2
done

create_revoke_payload=$(cat <<JSON
{"taskType":"batch_revoke","params":{"badge_id":2,"reason":"e2e批量撤销","user_ids":$batch_users}}
JSON
)
create_revoke_resp=$(api POST "/api/admin/tasks" "$create_revoke_payload")
assert_success "$create_revoke_resp"
revoke_task_id=$(json_get "$create_revoke_resp" "data.id")

log "等待批量撤销任务完成..."
end=$((SECONDS + 60))
while (( SECONDS < end )); do
  task_resp=$(api GET "/api/admin/tasks/$revoke_task_id")
  assert_success "$task_resp"
  status=$(json_get "$task_resp" "data.status")
  if [[ "$status" == "completed" ]]; then
    break
  fi
  if [[ "$status" == "failed" ]]; then
    echo "批量撤销任务失败: $task_resp" >&2
    exit 1
  fi
  sleep 2
done

log "统计与审计接口..."
assert_success "$(api GET "/api/admin/stats/overview")"
assert_success "$(api GET "/api/admin/stats/today")"
assert_success "$(api GET "/api/admin/logs?page=1&pageSize=5")"

log "全量联调用例执行完成"
