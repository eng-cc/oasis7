#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"
source "$repo_root/scripts/agent-browser-lib.sh"

usage() {
  cat <<'USAGE'
Usage: ./scripts/viewer-primary-web-entry-regression.sh [options]

Run a browser QA regression for the formal Web entry contract:
- default browser entry (`/`) must land in `software_safe`
- `render_mode=auto` must still land in `software_safe`
- explicit `render_mode=standard` must stay on the standard viewer surface

Options:
  --scenario <name>          oasis7_game_launcher scenario (default: llm_bootstrap)
  --live-bind <host:port>    live tcp bind (default: 127.0.0.1:5023)
  --web-bind <host:port>     web bridge bind (default: 127.0.0.1:5011)
  --chain-status-bind <a:p>  chain status HTTP bind (default: web-bind port + 110)
  --viewer-host <host>       web viewer host (default: 127.0.0.1)
  --viewer-port <port>       web viewer port (default: 4173)
  --viewer-static-dir <dir>  viewer static asset dir (default: web)
  --out-dir <path>           artifact root (default: output/playwright/viewer-primary-web-entry)
  --headed                   open browser in headed mode
  --headless                 open browser in headless mode (default)
  -h, --help                 show this help

Artifacts:
  <out-dir>/<run-id>/launcher.log
  <out-dir>/<run-id>/default_state.json
  <out-dir>/<run-id>/auto_state.json
  <out-dir>/<run-id>/standard_initial_state.json
  <out-dir>/<run-id>/standard_final_state.json
  <out-dir>/<run-id>/*.png
  <out-dir>/<run-id>/summary.json
  <out-dir>/<run-id>/summary.md
USAGE
}

sleep_ms() {
  python3 - "$1" <<'PY'
import sys
import time
time.sleep(int(sys.argv[1]) / 1000.0)
PY
}

wait_for_port() {
  local host=$1
  local port=$2
  local timeout_secs=${3:-120}
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

wait_for_http() {
  local url=$1
  local timeout_secs=${2:-120}
  local step
  for step in $(seq 1 "$timeout_secs"); do
    if curl -fsS "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  return 1
}

normalize_eval_token() {
  local raw=${1:-}
  raw=$(printf '%s' "$raw" | tr -d '\r\n')
  raw=${raw#\"}
  raw=${raw%\"}
  printf '%s' "$raw"
}

json_to_file() {
  local raw_json=$1
  local out_path=$2
  python3 - "$raw_json" "$out_path" <<'PY'
import json
import pathlib
import sys

raw = sys.argv[1]
out = pathlib.Path(sys.argv[2])
try:
    payload = json.loads(raw)
except Exception:
    out.write_text(raw + "\n", encoding="utf-8")
else:
    out.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY
}

ab_state() {
  ab_eval "$1" 'window.__AW_TEST__?.getState?.() ?? null'
}

state_connection() { json_get "$1" connectionStatus; }
state_render_mode() { json_get "$1" renderMode; }
state_reason() { json_get "$1" softwareSafeReason; }
state_selected_kind() { json_get "$1" selectedKind; }
state_last_error() { json_get "$1" lastError; }

wait_for_api() {
  local session=$1
  local timeout_ms=${2:-30000}
  local deadline=$((SECONDS * 1000 + timeout_ms))
  while (( SECONDS * 1000 < deadline )); do
    local ready
    ready=$(normalize_eval_token "$(ab_eval "$session" 'typeof window.__AW_TEST__ === "object" ? "ready" : "missing"')")
    if [[ "$ready" == "ready" || "$ready" == "true" ]]; then
      return 0
    fi
    sleep_ms 200
  done
  return 1
}

wait_for_connected() {
  local session=$1
  local timeout_ms=${2:-30000}
  local deadline=$((SECONDS * 1000 + timeout_ms))
  local state='null'
  while (( SECONDS * 1000 < deadline )); do
    state=$(ab_state "$session")
    if [[ "$(state_connection "$state")" == "connected" ]]; then
      printf '%s\n' "$state"
      return 0
    fi
    sleep_ms 250
  done
  printf '%s\n' "$state"
  return 1
}

wait_for_selected_kind() {
  local session=$1
  local expected_kind=$2
  local timeout_ms=${3:-10000}
  local deadline=$((SECONDS * 1000 + timeout_ms))
  local state='null'
  while (( SECONDS * 1000 < deadline )); do
    state=$(ab_state "$session")
    if [[ "$(state_selected_kind "$state")" == "$expected_kind" ]]; then
      printf '%s\n' "$state"
      return 0
    fi
    sleep_ms 250
  done
  printf '%s\n' "$state"
  return 1
}

assert_eval_true() {
  local session=$1
  local script=$2
  local timeout_ms=${3:-8000}
  local deadline=$((SECONDS * 1000 + timeout_ms))
  while (( SECONDS * 1000 < deadline )); do
    local value
    value=$(normalize_eval_token "$(ab_eval "$session" "$script")")
    if [[ "$value" == "true" ]]; then
      return 0
    fi
    sleep_ms 250
  done
  return 1
}

scenario="llm_bootstrap"
live_bind="127.0.0.1:5023"
web_bind="127.0.0.1:5011"
chain_status_bind=""
viewer_host="127.0.0.1"
viewer_port="4173"
viewer_static_dir="web"
out_root="output/playwright/viewer-primary-web-entry"
headed=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --scenario)
      scenario=${2:-}
      shift 2
      ;;
    --live-bind)
      live_bind=${2:-}
      shift 2
      ;;
    --web-bind)
      web_bind=${2:-}
      shift 2
      ;;
    --chain-status-bind)
      chain_status_bind=${2:-}
      shift 2
      ;;
    --viewer-host)
      viewer_host=${2:-}
      shift 2
      ;;
    --viewer-port)
      viewer_port=${2:-}
      shift 2
      ;;
    --viewer-static-dir)
      viewer_static_dir=${2:-}
      shift 2
      ;;
    --out-dir)
      out_root=${2:-}
      shift 2
      ;;
    --headed)
      headed=1
      shift
      ;;
    --headless)
      headed=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

ab_require

if [[ "$web_bind" != *:* ]]; then
  echo "error: --web-bind must be in <host:port> format" >&2
  exit 2
fi

web_bind_host=${web_bind%:*}
web_bind_port=${web_bind##*:}
if [[ -z "$web_bind_host" || -z "$web_bind_port" || ! "$web_bind_port" =~ ^[0-9]+$ ]]; then
  echo "error: invalid --web-bind: $web_bind" >&2
  exit 2
fi

if [[ -n "$chain_status_bind" ]]; then
  if [[ "$chain_status_bind" != *:* ]]; then
    echo "error: --chain-status-bind must be in <host:port> format" >&2
    exit 2
  fi
else
  derived_chain_status_port=$((web_bind_port + 110))
  if (( derived_chain_status_port > 65535 )); then
    echo "error: derived chain status port exceeds 65535" >&2
    exit 2
  fi
  chain_status_bind="${web_bind_host}:${derived_chain_status_port}"
fi

mkdir -p "$out_root"
stamp=$(date '+%Y%m%d-%H%M%S')
run_id="viewer-primary-web-entry-${stamp}"
out_dir="$out_root/$run_id"
mkdir -p "$out_dir"

launcher_log="$out_dir/launcher.log"
default_state_path="$out_dir/default_state.json"
auto_state_path="$out_dir/auto_state.json"
standard_initial_state_path="$out_dir/standard_initial_state.json"
standard_final_state_path="$out_dir/standard_final_state.json"
default_body_path="$out_dir/default_body.txt"
auto_body_path="$out_dir/auto_body.txt"
standard_body_path="$out_dir/standard_body.txt"
summary_json_path="$out_dir/summary.json"
summary_md_path="$out_dir/summary.md"

resolved_viewer_static_dir=$(resolve_viewer_static_dir_for_web_closure \
  "$repo_root" \
  "$viewer_static_dir" \
  "$out_dir")
viewer_static_dir="$resolved_viewer_static_dir"

live_args=(
  "--scenario" "$scenario"
  "--live-bind" "$live_bind"
  "--web-bind" "$web_bind"
  "--chain-status-bind" "$chain_status_bind"
  "--viewer-host" "$viewer_host"
  "--viewer-port" "$viewer_port"
  "--viewer-static-dir" "$viewer_static_dir"
  "--no-open-browser"
)

launcher_pid=""
cleanup() {
  set +e
  if [[ -n "$launcher_pid" ]]; then
    kill "$launcher_pid" >/dev/null 2>&1 || true
    wait "$launcher_pid" >/dev/null 2>&1 || true
  fi
  ab_cmd "${run_id}-default" close >/dev/null 2>&1 || true
  ab_cmd "${run_id}-auto" close >/dev/null 2>&1 || true
  ab_cmd "${run_id}-standard" close >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "+ env -u RUSTC_WRAPPER cargo build -p oasis7 --bin oasis7_viewer_live --bin oasis7_chain_runtime"
env -u RUSTC_WRAPPER cargo build -p oasis7 --bin oasis7_viewer_live --bin oasis7_chain_runtime >>"$launcher_log" 2>&1

echo "+ env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_game_launcher -- ${live_args[*]}"
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_game_launcher -- "${live_args[@]}" >"$launcher_log" 2>&1 &
launcher_pid=$!

wait_for_port "$web_bind_host" "$web_bind_port" 240 || { echo "error: web bridge did not come up on $web_bind" >&2; exit 1; }
wait_for_http "http://${viewer_host}:${viewer_port}/" 240 || { echo "error: viewer server did not become ready" >&2; exit 1; }

base_query="ws=$(python3 -c 'import sys, urllib.parse; print(urllib.parse.quote(sys.argv[1], safe=""))' "ws://${web_bind}")&test_api=1"
default_url="http://${viewer_host}:${viewer_port}/?${base_query}"
auto_url="http://${viewer_host}:${viewer_port}/?${base_query}&render_mode=auto"
standard_url="http://${viewer_host}:${viewer_port}/?${base_query}&render_mode=standard"

default_session="${run_id}-default"
auto_session="${run_id}-auto"
standard_session="${run_id}-standard"

ab_open "$default_session" "$headed" "$default_url" >/dev/null
ab_cmd "$default_session" wait --load networkidle >/dev/null 2>&1 || true
wait_for_api "$default_session" 60000 || { echo "error: default route missing __AW_TEST__" >&2; exit 1; }
default_state=$(wait_for_connected "$default_session" 30000) || { echo "error: default route did not connect" >&2; exit 1; }
json_to_file "$default_state" "$default_state_path"
ab_cmd "$default_session" get text body >"$default_body_path"
ab_screenshot "$default_session" "$out_dir/default-entry.png" >/dev/null
default_url_final=$(normalize_eval_token "$(ab_cmd "$default_session" get url)")

[[ "$(state_render_mode "$default_state")" == "software_safe" ]] || { echo "error: default route did not land in software_safe" >&2; exit 1; }
[[ "$(state_reason "$default_state")" == "primary_web_entry" ]] || { echo "error: default route reason mismatch: $(state_reason "$default_state")" >&2; exit 1; }
[[ "$default_url_final" == *"software_safe.html"* ]] || { echo "error: default route did not redirect to software_safe.html" >&2; exit 1; }
grep -q "Formal Gameplay Summary" "$default_body_path" || { echo "error: default route body missing Formal Gameplay Summary" >&2; exit 1; }
grep -q "Missing Action Handoff" "$default_body_path" || { echo "error: default route body missing Missing Action Handoff" >&2; exit 1; }

ab_open "$auto_session" "$headed" "$auto_url" >/dev/null
ab_cmd "$auto_session" wait --load networkidle >/dev/null 2>&1 || true
wait_for_api "$auto_session" 60000 || { echo "error: auto route missing __AW_TEST__" >&2; exit 1; }
auto_state=$(wait_for_connected "$auto_session" 30000) || { echo "error: auto route did not connect" >&2; exit 1; }
json_to_file "$auto_state" "$auto_state_path"
ab_cmd "$auto_session" get text body >"$auto_body_path"
ab_screenshot "$auto_session" "$out_dir/auto-entry.png" >/dev/null
auto_url_final=$(normalize_eval_token "$(ab_cmd "$auto_session" get url)")

[[ "$(state_render_mode "$auto_state")" == "software_safe" ]] || { echo "error: auto route did not land in software_safe" >&2; exit 1; }
[[ "$(state_reason "$auto_state")" == "auto_primary_web_entry" ]] || { echo "error: auto route reason mismatch: $(state_reason "$auto_state")" >&2; exit 1; }
[[ "$auto_url_final" == *"software_safe.html"* ]] || { echo "error: auto route did not redirect to software_safe.html" >&2; exit 1; }

ab_open "$standard_session" "$headed" "$standard_url" >/dev/null
ab_cmd "$standard_session" wait --load networkidle >/dev/null 2>&1 || true
wait_for_api "$standard_session" 60000 || { echo "error: standard route missing __AW_TEST__" >&2; exit 1; }
standard_initial_state=$(wait_for_connected "$standard_session" 30000) || { echo "error: standard route did not connect" >&2; exit 1; }
json_to_file "$standard_initial_state" "$standard_initial_state_path"
standard_url_final=$(normalize_eval_token "$(ab_cmd "$standard_session" get url)")
ab_eval "$standard_session" 'window.__AW_TEST__?.runSteps?.("mode=3d;focus=first_location;zoom=0.85;select=first_agent;wait=0.3") ?? null' >/dev/null
standard_final_state=$(wait_for_selected_kind "$standard_session" "agent" 10000) || { echo "error: standard route did not select an agent after runSteps" >&2; exit 1; }
json_to_file "$standard_final_state" "$standard_final_state_path"
ab_cmd "$standard_session" get text body >"$standard_body_path" || true
ab_screenshot "$standard_session" "$out_dir/standard-entry.png" >/dev/null

[[ "$(state_render_mode "$standard_initial_state")" == "standard" ]] || { echo "error: standard route did not stay in standard mode" >&2; exit 1; }
[[ -z "$(state_reason "$standard_initial_state")" ]] || { echo "error: standard route unexpectedly reported softwareSafeReason=$(state_reason "$standard_initial_state")" >&2; exit 1; }
[[ "$standard_url_final" != *"software_safe.html"* ]] || { echo "error: standard route redirected to software_safe.html" >&2; exit 1; }
[[ -z "$(state_last_error "$standard_final_state")" ]] || { echo "error: standard route reported lastError=$(state_last_error "$standard_final_state")" >&2; exit 1; }
assert_eval_true "$standard_session" 'Boolean(document.querySelector("canvas"))' 10000 || { echo "error: standard route never rendered a canvas" >&2; exit 1; }

python3 - "$summary_json_path" <<'PY' "$default_state_path" "$auto_state_path" "$standard_initial_state_path" "$standard_final_state_path" "$default_url_final" "$auto_url_final" "$standard_url_final" "$out_dir/default-entry.png" "$out_dir/auto-entry.png" "$out_dir/standard-entry.png"
import json
import pathlib
import sys

def load(path):
    return json.loads(pathlib.Path(path).read_text(encoding="utf-8"))

summary = {
    "ok": True,
    "default_entry": {
        "final_url": sys.argv[6],
        "state": load(sys.argv[2]),
        "screenshot": sys.argv[9],
    },
    "auto_entry": {
        "final_url": sys.argv[7],
        "state": load(sys.argv[3]),
        "screenshot": sys.argv[10],
    },
    "standard_entry": {
        "final_url": sys.argv[8],
        "initial_state": load(sys.argv[4]),
        "final_state": load(sys.argv[5]),
        "screenshot": sys.argv[11],
    },
}
pathlib.Path(sys.argv[1]).write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY

python3 - "$summary_json_path" "$summary_md_path" <<'PY'
import json
import pathlib
import sys

summary = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
default_state = summary["default_entry"]["state"]
auto_state = summary["auto_entry"]["state"]
standard_initial = summary["standard_entry"]["initial_state"]
standard_final = summary["standard_entry"]["final_state"]
lines = [
    "# Viewer primary Web entry regression summary",
    "",
    "## Verdict",
    "- Overall: `pass`",
    "- Formal gameplay entry (`/`): `software_safe`",
    "- Auto entry (`?render_mode=auto`): `software_safe`",
    "- Explicit visual QA entry (`?render_mode=standard`): `standard`",
    "",
    "## Default entry",
    f"- Final URL: `{summary['default_entry']['final_url']}`",
    f"- Render mode: `{default_state.get('renderMode')}`",
    f"- Entry reason: `{default_state.get('softwareSafeReason')}`",
    f"- Screenshot: `{summary['default_entry']['screenshot']}`",
    "",
    "## Auto entry",
    f"- Final URL: `{summary['auto_entry']['final_url']}`",
    f"- Render mode: `{auto_state.get('renderMode')}`",
    f"- Entry reason: `{auto_state.get('softwareSafeReason')}`",
    f"- Screenshot: `{summary['auto_entry']['screenshot']}`",
    "",
    "## Standard visual QA entry",
    f"- Final URL: `{summary['standard_entry']['final_url']}`",
    f"- Initial render mode: `{standard_initial.get('renderMode')}`",
    f"- Final selected kind: `{standard_final.get('selectedKind')}`",
    f"- Screenshot: `{summary['standard_entry']['screenshot']}`",
]
pathlib.Path(sys.argv[2]).write_text("\n".join(lines) + "\n", encoding="utf-8")
PY

printf 'ok: artifacts written to %s\n' "$out_dir"
