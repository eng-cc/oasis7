#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

usage() {
  cat <<'USAGE'
Usage: ./scripts/oasis7-pure-api-parity-smoke.sh [options] [run-game-test options...]

Validate the pure API gameplay path against the live TCP protocol using
`oasis7_pure_api_client`.

Default flow:
1. bootstrap a fresh stack via ./scripts/run-game-test.sh
2. build the local `oasis7_pure_api_client` binary
3. capture initial player_gameplay snapshot
4. submit canonical `gameplay_action` to build the first smelter
5. advance until the smelter is ready, then submit the first iron-ingot recipe
6. advance until the first resilient production milestone is visible
7. capture reconnect-sync recovery ack
6. emit JSON/Markdown summary plus raw command outputs

Options:
  --tier <required|full>      Validation tier (default: required)
  --live-addr <host:port>     Reuse an existing live TCP endpoint instead of bootstrapping
  --bundle-dir <path>         Pass through to run-game-test for fresh bundle validation
  --out-dir <path>            Artifact root (default: output/playwright/playability)
  --startup-timeout <secs>    Wait timeout for stack startup / TCP listener (default: 240)
  --step-a <count>            Steps to settle the first factory build (default: 2)
  --step-b <count>            Steps to settle the first recipe run (default: 2)
  --step-c <count>            Extra full-tier follow-up steps after milestone (default: 8)
  --player-id <id>            Player id for reconnect-sync (default: player-api-smoke)
  -h, --help                  Show this help

Examples:
  ./scripts/oasis7-pure-api-parity-smoke.sh --bundle-dir output/release/game-launcher-local --no-llm
  ./scripts/oasis7-pure-api-parity-smoke.sh --tier full --live-addr 127.0.0.1:5023
USAGE
}

wait_for_tcp_listener() {
  local host=$1
  local port=$2
  local timeout_secs=${3:-20}
  local step
  for step in $(seq 1 "$timeout_secs"); do
    if python3 - "$host" "$port" <<'PY'
import socket
import sys

host = sys.argv[1]
port = int(sys.argv[2])
try:
    with socket.create_connection((host, port), timeout=1):
        pass
except OSError:
    raise SystemExit(1)
raise SystemExit(0)
PY
    then
      return 0
    fi
    sleep 1
  done
  return 1
}

tier="required"
live_addr=""
bundle_dir=""
out_root="output/playwright/playability"
startup_timeout_secs=240
step_a=2
step_b=2
step_c=8
player_id="player-api-smoke"
stack_args=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tier)
      tier="${2:-}"
      shift 2
      ;;
    --live-addr)
      live_addr="${2:-}"
      shift 2
      ;;
    --bundle-dir)
      bundle_dir="${2:-}"
      stack_args+=("$1" "$bundle_dir")
      shift 2
      ;;
    --out-dir)
      out_root="${2:-}"
      shift 2
      ;;
    --startup-timeout)
      startup_timeout_secs="${2:-}"
      shift 2
      ;;
    --step-a)
      step_a="${2:-}"
      shift 2
      ;;
    --step-b)
      step_b="${2:-}"
      shift 2
      ;;
    --step-c)
      step_c="${2:-}"
      shift 2
      ;;
    --player-id)
      player_id="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      stack_args+=("$1")
      shift
      ;;
  esac
done

[[ "$tier" == "required" || "$tier" == "full" ]] || {
  echo "error: --tier must be required or full" >&2
  exit 2
}
[[ -n "$out_root" ]] || { echo "error: --out-dir cannot be empty" >&2; exit 2; }
[[ "$startup_timeout_secs" =~ ^[0-9]+$ ]] && [[ "$startup_timeout_secs" -gt 0 ]] || {
  echo "error: --startup-timeout must be a positive integer" >&2
  exit 2
}
for value_name in step_a step_b step_c; do
  value="${!value_name}"
  [[ "$value" =~ ^[0-9]+$ ]] && [[ "$value" -gt 0 ]] || {
    echo "error: --${value_name//_/-} must be a positive integer" >&2
    exit 2
  }
done

resolve_bootstrap_live_addr() {
  local resolved="127.0.0.1:5023"
  local index=0
  while (( index < ${#stack_args[@]} )); do
    if [[ "${stack_args[$index]}" == "--live-bind" ]]; then
      if (( index + 1 >= ${#stack_args[@]} )); then
        echo "error: --live-bind requires an address" >&2
        exit 2
      fi
      resolved="${stack_args[$((index + 1))]}"
      break
    fi
    index=$((index + 1))
  done
  printf '%s\n' "$resolved"
}

stamp=$(date +%Y%m%d-%H%M%S)
run_id="pure-api-${tier}-${stamp}"
out_dir="$out_root/$run_id"
mkdir -p "$out_dir"

run_log="$out_dir/run-game-test.log"
summary_json_path="$out_dir/pure-api-summary.json"
summary_md_path="$out_dir/pure-api-summary.md"
initial_snapshot_path="$out_dir/snapshot-initial.json"
step_a_path="$out_dir/step-a.json"
step_b_path="$out_dir/step-b.json"
step_c_path="$out_dir/step-c.json"
build_action_path="$out_dir/gameplay-build-smelter.json"
recipe_action_path="$out_dir/gameplay-iron-ingot.json"
recovery_path="$out_dir/reconnect-sync.json"
keygen_path="$out_dir/keygen.json"

stack_pid=""
stack_logs_dir=""
probe_live_addr="${live_addr:-$(resolve_bootstrap_live_addr)}"

cleanup() {
  local exit_code=$?
  trap - EXIT INT TERM
  if [[ -n "$stack_pid" ]] && kill -0 "$stack_pid" >/dev/null 2>&1; then
    kill "$stack_pid" >/dev/null 2>&1 || true
    wait "$stack_pid" >/dev/null 2>&1 || true
  fi
  exit "$exit_code"
}
trap cleanup EXIT INT TERM

env -u RUSTC_WRAPPER cargo build -q -p oasis7 --bin oasis7_pure_api_client
client_bin="$repo_root/target/debug/oasis7_pure_api_client"
[[ -x "$client_bin" ]] || {
  echo "error: expected pure API client binary at $client_bin" >&2
  exit 1
}

if [[ -z "$live_addr" ]]; then
  ./scripts/run-game-test.sh "${stack_args[@]}" > >(tee "$run_log") 2>&1 &
  stack_pid=$!

  for ((i = 0; i < startup_timeout_secs; i++)); do
    if ! kill -0 "$stack_pid" >/dev/null 2>&1; then
      echo "error: run-game-test.sh exited unexpectedly" >&2
      tail -n 120 "$run_log" >&2 || true
      exit 1
    fi
    if [[ -f "$run_log" ]]; then
      stack_logs_dir="$(sed -n 's/^- Logs: \(.*\)$/\1/p' "$run_log" | tail -n 1)"
    fi
    if wait_for_tcp_listener "${probe_live_addr%:*}" "${probe_live_addr##*:}" 1; then
      break
    fi
  done
else
  wait_for_tcp_listener "${probe_live_addr%:*}" "${probe_live_addr##*:}" "$startup_timeout_secs" || {
    echo "error: timeout waiting for live TCP listener at $probe_live_addr" >&2
    exit 1
  }
fi

if ! wait_for_tcp_listener "${probe_live_addr%:*}" "${probe_live_addr##*:}" 1; then
  echo "error: timeout waiting for live TCP listener at $probe_live_addr" >&2
  if [[ -n "$stack_pid" ]]; then
    tail -n 120 "$run_log" >&2 || true
  fi
  exit 1
fi

"$client_bin" keygen >"$keygen_path"
"$client_bin" --addr "$probe_live_addr" snapshot --player-gameplay-only >"$initial_snapshot_path"

json_field() {
  local path=$1
  local key=$2
  python3 - "$path" "$key" <<'PY'
import json
import sys

payload = json.load(open(sys.argv[1], encoding="utf-8"))
value = payload.get(sys.argv[2], "")
if value is None:
    value = ""
print(value)
PY
}

find_action_target() {
  local path=$1
  local action_id=$2
  python3 - "$path" "$action_id" <<'PY'
import json
import sys

payload = json.load(open(sys.argv[1], encoding="utf-8"))
for action in payload.get("available_actions", []):
    if action.get("action_id") == sys.argv[2]:
        print(action.get("target_agent_id") or "")
        raise SystemExit(0)
raise SystemExit(1)
PY
}

public_key_hex=$(json_field "$keygen_path" "public_key_hex")
private_key_hex=$(json_field "$keygen_path" "private_key_hex")
target_agent_id=$(find_action_target "$initial_snapshot_path" "build_factory_smelter_mk1")
[[ -n "$public_key_hex" && -n "$private_key_hex" && -n "$target_agent_id" ]] || {
  echo "error: failed to resolve gameplay_action bootstrap inputs" >&2
  exit 1
}

"$client_bin" --addr "$probe_live_addr" gameplay-action \
  --action-id build_factory_smelter_mk1 \
  --target-agent-id "$target_agent_id" \
  --player-id "$player_id" \
  --private-key-hex "$private_key_hex" \
  --public-key-hex "$public_key_hex" \
  --with-snapshot >"$build_action_path"
"$client_bin" --addr "$probe_live_addr" step --count "$step_a" >"$step_a_path"
"$client_bin" --addr "$probe_live_addr" gameplay-action \
  --action-id schedule_recipe_smelter_iron_ingot \
  --target-agent-id "$target_agent_id" \
  --player-id "$player_id" \
  --private-key-hex "$private_key_hex" \
  --public-key-hex "$public_key_hex" \
  --with-snapshot >"$recipe_action_path"
"$client_bin" --addr "$probe_live_addr" step --count "$step_b" >"$step_b_path"
if [[ "$tier" == "full" ]]; then
  "$client_bin" --addr "$probe_live_addr" step --count "$step_c" >"$step_c_path"
fi
"$client_bin" --addr "$probe_live_addr" reconnect-sync --player-id "$player_id" --with-snapshot >"$recovery_path"

python3 - "$tier" \
  "$probe_live_addr" \
  "$player_id" \
  "$keygen_path" \
  "$initial_snapshot_path" \
  "$build_action_path" \
  "$step_a_path" \
  "$recipe_action_path" \
  "$step_b_path" \
  "$step_c_path" \
  "$recovery_path" \
  "$summary_json_path" \
  "$summary_md_path" \
  "$stack_logs_dir" <<'PY'
import json
import pathlib
import sys

tier = sys.argv[1]
live_addr = sys.argv[2]
player_id = sys.argv[3]
keygen_path = pathlib.Path(sys.argv[4])
initial_snapshot_path = pathlib.Path(sys.argv[5])
build_action_path = pathlib.Path(sys.argv[6])
step_a_path = pathlib.Path(sys.argv[7])
recipe_action_path = pathlib.Path(sys.argv[8])
step_b_path = pathlib.Path(sys.argv[9])
step_c_path = pathlib.Path(sys.argv[10])
recovery_path = pathlib.Path(sys.argv[11])
summary_json_path = pathlib.Path(sys.argv[12])
summary_md_path = pathlib.Path(sys.argv[13])
stack_logs_dir = sys.argv[14]

keygen = json.loads(keygen_path.read_text(encoding="utf-8"))
initial_snapshot = json.loads(initial_snapshot_path.read_text(encoding="utf-8"))
build_action = json.loads(build_action_path.read_text(encoding="utf-8"))
step_a = json.loads(step_a_path.read_text(encoding="utf-8"))
recipe_action = json.loads(recipe_action_path.read_text(encoding="utf-8"))
step_b = json.loads(step_b_path.read_text(encoding="utf-8"))
step_c = (
    json.loads(step_c_path.read_text(encoding="utf-8"))
    if tier == "full" and step_c_path.exists()
    else None
)
recovery = json.loads(recovery_path.read_text(encoding="utf-8"))

def response_by_type(payload, response_type):
    for item in payload.get("responses", []):
        if item.get("type") == response_type:
            return item
    return None

def has_protocol_action(payload, action_name):
    for item in payload.get("available_actions", []):
        if item.get("protocol_action") == action_name:
            return True
    return False

def has_action_id(payload, action_id):
    for item in payload.get("available_actions", []):
        if item.get("action_id") == action_id:
            return True
    return False

build_ack = response_by_type(build_action, "gameplay_action_ack")
step_a_ack = response_by_type(step_a, "control_completion_ack")
recipe_ack = response_by_type(recipe_action, "gameplay_action_ack")
step_b_ack = response_by_type(step_b, "control_completion_ack")
step_c_ack = response_by_type(step_c, "control_completion_ack") if step_c else None
recovery_ack = response_by_type(recovery, "authoritative_recovery_ack")

initial_stage = initial_snapshot.get("stage_id")
build_snapshot = build_action.get("latest_snapshot") or {}
step_a_gameplay = step_a.get("player_gameplay") or {}
followup_gameplay = step_b.get("player_gameplay") or {}
followup_stage = followup_gameplay.get("stage_id")
followup_feedback = followup_gameplay.get("recent_feedback") or {}
followup_snapshot = step_b.get("latest_snapshot") or {}
followup_time = followup_snapshot.get("time")
recovery_snapshot = recovery.get("latest_snapshot")

checks = {
    "hello_live_profile": (step_a.get("hello_ack") or {}).get("control_profile") == "live",
    "initial_stage_first_session_loop": initial_stage == "first_session_loop",
    "initial_actions_include_snapshot": has_protocol_action(initial_snapshot, "request_snapshot"),
    "initial_actions_include_step": has_protocol_action(initial_snapshot, "live_control.step"),
    "initial_actions_include_play": has_protocol_action(initial_snapshot, "live_control.play"),
    "initial_actions_include_build_smelter": has_action_id(initial_snapshot, "build_factory_smelter_mk1"),
    "build_action_ack": (build_ack or {}).get("ack", {}).get("action_id") == "build_factory_smelter_mk1",
    "build_snapshot_present": bool(build_snapshot),
    "step_a_offers_recipe": has_action_id(step_a_gameplay, "schedule_recipe_smelter_iron_ingot"),
    "step_a_advanced": (step_a_ack or {}).get("ack", {}).get("status") == "advanced",
    "recipe_action_ack": (recipe_ack or {}).get("ack", {}).get("action_id") == "schedule_recipe_smelter_iron_ingot",
    "step_b_advanced": (step_b_ack or {}).get("ack", {}).get("status") == "advanced",
    "followup_stage_post_onboarding": followup_stage == "post_onboarding",
    "followup_goal_resilient_production": followup_gameplay.get("goal_id") == "post_onboarding.stabilize_first_line_after_output",
    "followup_progress_midloop_floor": (followup_gameplay.get("progress_percent") or 0) >= 80,
    "followup_has_next_step": bool(followup_gameplay.get("next_step_hint")),
    "followup_has_recent_feedback": bool(followup_feedback.get("stage")),
    "reconnect_sync_ack": (recovery_ack or {}).get("ack", {}).get("status") == "catch_up_ready",
    "recovery_snapshot_present": recovery_snapshot is not None,
    "recovery_player_gameplay_present": bool(recovery.get("player_gameplay")),
}
if tier == "full":
    checks["step_c_advanced"] = (step_c_ack or {}).get("ack", {}).get("status") == "advanced"
    checks["step_c_snapshot_present"] = bool((step_c or {}).get("latest_snapshot"))

failed_checks = [name for name, ok in checks.items() if not ok]
summary = {
    "tier": tier,
    "live_addr": live_addr,
    "player_id": player_id,
    "keygen_public_key": keygen.get("public_key_hex"),
    "stack_logs_dir": stack_logs_dir or None,
    "checks": checks,
    "failed_checks": failed_checks,
    "result": "pass" if not failed_checks else "block",
    "initial_stage": initial_stage,
    "followup_stage": followup_stage,
    "followup_goal_id": followup_gameplay.get("goal_id"),
    "followup_progress_percent": followup_gameplay.get("progress_percent"),
    "followup_next_step_hint": followup_gameplay.get("next_step_hint"),
    "followup_recent_feedback_stage": followup_feedback.get("stage"),
    "followup_time": followup_time,
    "recovery_status": (recovery_ack or {}).get("ack", {}).get("status"),
    "recovery_snapshot_present": recovery_snapshot is not None,
    "notes": [
        "This smoke validates the pure API player path via oasis7_pure_api_client, not browser UI rendering.",
        "Parity_verified still requires separate UI/API matrix review and broader long-run sampling.",
    ],
}
summary_json_path.write_text(
    json.dumps(summary, ensure_ascii=False, indent=2) + "\n",
    encoding="utf-8",
)

lines = [
    f"# Pure API {tier.upper()} 验证摘要",
    "",
    f"- 结论: `{summary['result']}`",
    f"- Live 地址: `{live_addr}`",
    f"- Player ID: `{player_id}`",
    f"- 初始阶段: `{initial_stage}`",
    f"- 跟进阶段: `{followup_stage}`",
    f"- 跟进目标: `{summary['followup_goal_id']}`",
    f"- 跟进进度: `{summary['followup_progress_percent']}`",
    f"- 最近反馈: `{summary['followup_recent_feedback_stage']}`",
    f"- 恢复状态: `{summary['recovery_status']}`",
    "",
    "## 检查项",
]
for name, ok in checks.items():
    lines.append(f"- `{name}`: `{'pass' if ok else 'block'}`")
if failed_checks:
    lines.extend([
        "",
        "## 阻断项",
        *[f"- `{name}`" for name in failed_checks],
    ])
summary_md_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY

cat "$summary_md_path"
