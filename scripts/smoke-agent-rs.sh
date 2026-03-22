#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SMOKE_DIR="${SMOKE_DIR:-$ROOT_DIR/.tmp/agent-rs-smoke}"
PKI_DIR="$SMOKE_DIR/pki"
STATE_DIR="$SMOKE_DIR/state"
RUNTIME_DIR="$STATE_DIR/runtime"
CONFIG_FILE="$SMOKE_DIR/agent-smoke.yaml"
AGENT_LOG="$SMOKE_DIR/agent-rs.log"
DOCTOR_LOG="$SMOKE_DIR/doctor.json"
LOG_FILE="${SMOKE_LOG_FILE:-/tmp/doro-agent-bootstrap.log}"
EDGE_HTTP="${EDGE_HTTP:-http://localhost:8080}"
EDGE_URL="${EDGE_URL:-https://localhost:8080}"
EDGE_GRPC_ADDR="${EDGE_GRPC_ADDR:-localhost:9090}"
AGENT_ID="${AGENT_ID:-agent-dev}"
DEFAULT_POLICY_NAME="${DEFAULT_POLICY_NAME:-Default Policy}"
DEFAULT_POLICY_REVISION="${DEFAULT_POLICY_REVISION:-rev-1}"
REQUESTED_BY="${REQUESTED_BY:-agent-rs-smoke}"
AGENT_BIN="${AGENT_BIN:-$ROOT_DIR/agent-rs/target/debug/doro-agent}"

require_bin() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

wait_for_ready() {
  for _ in $(seq 1 90); do
    if curl -fsS "$EDGE_HTTP/readyz" >/dev/null 2>&1; then
      return 0
    fi
    sleep 2
  done
  echo "edge-api did not become ready at $EDGE_HTTP" >&2
  exit 1
}

python_json() {
  python3 - "$@"
}

stop_agent() {
  if [[ -n "${AGENT_PID:-}" ]] && kill -0 "$AGENT_PID" >/dev/null 2>&1; then
    kill "$AGENT_PID" >/dev/null 2>&1 || true
    wait "$AGENT_PID" >/dev/null 2>&1 || true
  fi
  AGENT_PID=""
}

start_agent() {
  : >"$AGENT_LOG"
  "$AGENT_BIN" run --config "$CONFIG_FILE" >>"$AGENT_LOG" 2>&1 &
  AGENT_PID=$!
  sleep 1
  if ! kill -0 "$AGENT_PID" >/dev/null 2>&1; then
    echo "agent-rs exited early; see $AGENT_LOG" >&2
    exit 1
  fi
}

wait_for_policy_apply() {
  local snapshot_path="$RUNTIME_DIR/diagnostics-snapshot.json"
  for _ in $(seq 1 60); do
    if [[ -f "$snapshot_path" ]] && python_json "$snapshot_path" "$DEFAULT_POLICY_REVISION" "$LOG_FILE" <<'PY'
import json
import sys

snapshot_path, expected_revision, expected_path = sys.argv[1:4]
with open(snapshot_path, "r", encoding="utf-8") as handle:
    data = json.load(handle)

if data.get("current_policy_revision") != expected_revision:
    raise SystemExit(1)

source_statuses = data.get("source_statuses") or []
for item in source_statuses:
    if item.get("path") == expected_path:
        raise SystemExit(0)

raise SystemExit(1)
PY
    then
      return 0
    fi
    sleep 2
  done
  echo "agent did not apply policy for $LOG_FILE; see $AGENT_LOG" >&2
  exit 1
}

wait_for_agent_detail() {
  for _ in $(seq 1 60); do
    if curl -fsS "$EDGE_HTTP/api/v1/agents/$AGENT_ID" | python_json "$AGENT_ID" <<'PY'
import json
import sys

expected_agent_id = sys.argv[1]
data = json.load(sys.stdin)
item = data.get("item") or {}
if item.get("agent_id") != expected_agent_id:
    raise SystemExit(1)
if not item.get("version"):
    raise SystemExit(1)
raise SystemExit(0)
PY
    then
      return 0
    fi
    sleep 2
  done
  echo "agent detail did not appear for $AGENT_ID" >&2
  exit 1
}

wait_for_diagnostics() {
  for _ in $(seq 1 60); do
    if curl -fsS "$EDGE_HTTP/api/v1/agents/$AGENT_ID/diagnostics" | python_json "$AGENT_ID" <<'PY'
import json
import sys

expected_agent_id = sys.argv[1]
data = json.load(sys.stdin)
item = data.get("item") or {}
if item.get("agent_id") != expected_agent_id:
    raise SystemExit(1)
payload = item.get("payload_json") or {}
if not payload:
    raise SystemExit(1)
raise SystemExit(0)
PY
    then
      return 0
    fi
    sleep 2
  done
  echo "diagnostics did not arrive for $AGENT_ID" >&2
  exit 1
}

wait_for_log_delivery() {
  local token="$1"
  for _ in $(seq 1 90); do
    if curl -fsS \
      -X POST \
      "$EDGE_HTTP/api/v1/logs/search" \
      -H "Content-Type: application/json" \
      -d "{\"query\":\"$token\",\"limit\":20,\"offset\":0}" | python_json "$token" <<'PY'
import json
import sys

token = sys.argv[1]
data = json.load(sys.stdin)
for item in data.get("items") or []:
    if token in (item.get("message") or ""):
        raise SystemExit(0)
raise SystemExit(1)
PY
    then
      return 0
    fi
    sleep 2
  done
  echo "log line containing $token did not reach query path" >&2
  exit 1
}

wait_for_agent_count() {
  local expected="$1"
  for _ in $(seq 1 30); do
    if curl -fsS "$EDGE_HTTP/api/v1/agents" | python_json "$AGENT_ID" "$expected" <<'PY'
import json
import sys

agent_id = sys.argv[1]
expected = int(sys.argv[2])
data = json.load(sys.stdin)
count = sum(1 for item in data.get("items") or [] if item.get("agent_id") == agent_id)
if count != expected:
    raise SystemExit(1)
raise SystemExit(0)
PY
    then
      return 0
    fi
    sleep 1
  done
  echo "expected $expected records for $AGENT_ID" >&2
  exit 1
}

require_bin docker
require_bin curl
require_bin cargo
require_bin python3

mkdir -p "$PKI_DIR" "$STATE_DIR" "$SMOKE_DIR"
trap stop_agent EXIT

docker compose up -d --build >/dev/null
wait_for_ready

EDGE_CONTAINER_ID="$(docker compose ps -q edge-api)"
if [[ -z "$EDGE_CONTAINER_ID" ]]; then
  echo "edge-api container is not running" >&2
  exit 1
fi

docker cp "$EDGE_CONTAINER_ID:/certs/ca.crt" "$PKI_DIR/ca.crt" >/dev/null
docker cp "$EDGE_CONTAINER_ID:/certs/agent.crt" "$PKI_DIR/agent.crt" >/dev/null
docker cp "$EDGE_CONTAINER_ID:/certs/agent.key" "$PKI_DIR/agent.key" >/dev/null

POLICY_ID="$(
  curl -fsS "$EDGE_HTTP/api/v1/policies" | python_json "$DEFAULT_POLICY_NAME" <<'PY'
import json
import sys

policy_name = sys.argv[1]
data = json.load(sys.stdin)
for item in data.get("items") or []:
    if item.get("name") == policy_name:
        print(item["policy_id"])
        raise SystemExit(0)
raise SystemExit(f"policy {policy_name!r} was not found")
PY
)"

POLICY_REVISION_ID="$(
  curl -fsS "$EDGE_HTTP/api/v1/policies/$POLICY_ID/revisions" | python_json "$DEFAULT_POLICY_REVISION" <<'PY'
import json
import sys

revision_name = sys.argv[1]
data = json.load(sys.stdin)
items = data.get("items") or []
for item in items:
    if item.get("revision") == revision_name:
        print(item["policy_revision_id"])
        raise SystemExit(0)
if items:
    print(items[0]["policy_revision_id"])
    raise SystemExit(0)
raise SystemExit("no policy revisions were returned")
PY
)"

EXPIRES_AT_UNIX_MS="$(python3 - <<'PY'
import time
print(int((time.time() + 3600) * 1000))
PY
)"

BOOTSTRAP_TOKEN="$(
  curl -fsS \
    -X POST \
    "$EDGE_HTTP/api/v1/agents/bootstrap-tokens" \
    -H "Content-Type: application/json" \
    -d "{\"policy_id\":\"$POLICY_ID\",\"policy_revision_id\":\"$POLICY_REVISION_ID\",\"requested_by\":\"$REQUESTED_BY\",\"expires_at_unix_ms\":$EXPIRES_AT_UNIX_MS}" | python_json <<'PY'
import json
import sys

data = json.load(sys.stdin)
item = data.get("item") or {}
token = item.get("bootstrap_token")
if not token:
    raise SystemExit("bootstrap token was not returned")
print(token)
PY
)"

mkdir -p "$(dirname "$LOG_FILE")"
: >"$LOG_FILE"
mkdir -p "$STATE_DIR/spool"

cat >"$CONFIG_FILE" <<EOF
edge_url: "$EDGE_URL"
edge_grpc_addr: "$EDGE_GRPC_ADDR"
bootstrap_token: "$BOOTSTRAP_TOKEN"
state_dir: "$STATE_DIR"
log_level: "info"

transport:
  mode: "edge"

install:
  mode: "auto"

policy:
  refresh_interval_sec: 5

heartbeat:
  interval_sec: 5

diagnostics:
  interval_sec: 5

spool:
  enabled: true
  dir: "$STATE_DIR/spool"
  max_disk_bytes: 268435456

tls:
  ca_path: "$PKI_DIR/ca.crt"
  cert_path: "$PKI_DIR/agent.crt"
  key_path: "$PKI_DIR/agent.key"
EOF

cargo build --manifest-path "$ROOT_DIR/agent-rs/Cargo.toml" >/dev/null
"$AGENT_BIN" doctor --config "$CONFIG_FILE" --json >"$DOCTOR_LOG"

start_agent
wait_for_policy_apply
wait_for_agent_detail
wait_for_diagnostics

UNIQUE_TOKEN="agent-smoke-$(date +%s)"
printf '%s %s\n' "$UNIQUE_TOKEN" "first-line" >>"$LOG_FILE"
wait_for_log_delivery "$UNIQUE_TOKEN"
wait_for_agent_count 1

stop_agent
rm -f "$STATE_DIR/state.db"

start_agent
wait_for_policy_apply
wait_for_agent_detail
wait_for_agent_count 1

echo "agent-rs smoke passed"
echo "agent_id=$AGENT_ID"
echo "config=$CONFIG_FILE"
echo "state_dir=$STATE_DIR"
echo "doctor_log=$DOCTOR_LOG"
echo "agent_log=$AGENT_LOG"
