#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"
source "$repo_root/scripts/agent-browser-lib.sh"

usage() {
  cat <<'USAGE'
Usage: ./scripts/viewer-pixel-world-wasm-regression.sh [options] [run-game-test options...]

Verify that the embedded pixel-world surface prefers the real wasm runtime:
- page load + `__AW_TEST__` availability
- runtime connection reaches `connected`
- pixel-world runtime status becomes `ready`
- pixel-world runtime source resolves to `wasm_bindgen_runtime`
- wasm module URL points at `pixel-world-bridge/pixel_world_bridge.js`

Options:
  --url <url>               Use an existing viewer URL; skip stack bootstrap
  --out-dir <path>          Artifact root (default: output/playwright/viewer-pixel-world-wasm)
  --startup-timeout <secs>  Wait timeout for stack URL (default: 240)
  --headed                  Open browser in headed mode
  --headless                Open browser in headless mode (default)
  -h, --help                Show this help
USAGE
}

sleep_ms() {
  python3 - "$1" <<'PY'
import sys, time
time.sleep(int(sys.argv[1]) / 1000.0)
PY
}

append_query_params() {
  python3 - "$1" <<'PY'
from urllib.parse import urlparse, parse_qsl, urlencode, urlunparse
import sys
raw = sys.argv[1]
parts = urlparse(raw)
query = dict(parse_qsl(parts.query, keep_blank_values=True))
query["render_mode"] = "viewer"
query["test_api"] = "1"
print(urlunparse(parts._replace(query=urlencode(query))))
PY
}

normalize_eval_token() {
  local raw=${1:-}
  raw=$(printf '%s' "$raw" | tr -d '\r\n')
  raw=${raw#\"}
  raw=${raw%\"}
  printf '%s' "$raw"
}

ab_state() {
  ab_eval "$session" 'window.__AW_TEST__?.getState?.() ?? null'
}

state_connection() { json_get "$1" connectionStatus; }
state_render_mode() { json_get "$1" renderMode; }
state_last_error() { json_get "$1" lastError; }
state_pixel_runtime_status() { json_get "$1" pixelWorldRuntimeStatus; }
state_pixel_runtime_source() { json_get "$1" pixelWorldRuntimeSource; }
state_pixel_runtime_url() { json_get "$1" pixelWorldRuntimeModuleUrl; }

wait_for_api() {
  local timeout_ms=${1:-20000}
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
  local timeout_ms=${1:-20000}
  local deadline=$((SECONDS * 1000 + timeout_ms))
  local state='null'
  while (( SECONDS * 1000 < deadline )); do
    state=$(ab_state)
    if [[ -n "$(state_last_error "$state")" ]]; then
      printf '%s\n' "$state"
      return 2
    fi
    if [[ "$(state_connection "$state")" == "connected" ]]; then
      printf '%s\n' "$state"
      return 0
    fi
    sleep_ms 250
  done
  printf '%s\n' "$state"
  return 1
}

wait_for_pixel_runtime_ready() {
  local timeout_ms=${1:-12000}
  local deadline=$((SECONDS * 1000 + timeout_ms))
  local state='null'
  while (( SECONDS * 1000 < deadline )); do
    state=$(ab_state)
    if [[ "$(state_pixel_runtime_status "$state")" == "ready" ]]; then
      printf '%s\n' "$state"
      return 0
    fi
    sleep_ms 250
  done
  printf '%s\n' "$state"
  return 1
}

json_to_md_summary() {
  python3 - "$@" <<'PY'
import json
import sys
payload = {
    "ok": sys.argv[1] == "true",
    "runId": sys.argv[2],
    "gameUrl": sys.argv[3],
    "renderMode": sys.argv[4],
    "pixelWorldRuntimeStatus": sys.argv[5],
    "pixelWorldRuntimeSource": sys.argv[6],
    "pixelWorldRuntimeModuleUrl": None if sys.argv[7] == "null" else sys.argv[7],
    "lastError": None if sys.argv[8] == "null" else sys.argv[8],
}
print(json.dumps(payload, ensure_ascii=False, indent=2))
PY
}

startup_timeout_secs=240
out_root="output/playwright/viewer-pixel-world-wasm"
headed=0
provided_url=""
run_game_test_args=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --url)
      provided_url=${2:-}
      shift 2
      ;;
    --out-dir)
      out_root=${2:-}
      shift 2
      ;;
    --startup-timeout)
      startup_timeout_secs=${2:-}
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
      run_game_test_args+=("$1")
      shift
      ;;
  esac
done

ab_require
require_cmd python3
require_cmd mktemp
require_cmd date

run_id="viewer-pixel-world-wasm-$(date +%Y%m%d-%H%M%S)"
out_dir="$repo_root/$out_root/$run_id"
mkdir -p "$out_dir"
ab_log="$out_dir/agent-browser.log"
summary_json_path="$out_dir/pixel-world-wasm-summary.json"
summary_md_path="$out_dir/pixel-world-wasm-summary.md"
initial_state_path="$out_dir/initial_state.json"
connected_state_path="$out_dir/connected_state.json"
ready_state_path="$out_dir/ready_state.json"
final_state_path="$out_dir/final_state.json"

cleanup() {
  if [[ -n "${launcher_pid:-}" ]]; then
    kill "$launcher_pid" >/dev/null 2>&1 || true
    wait "$launcher_pid" >/dev/null 2>&1 || true
  fi
  if [[ -n "${session:-}" ]]; then
    ab_cmd "$session" close >>"$ab_log" 2>&1 || true
  fi
}
trap cleanup EXIT

game_url="$provided_url"
if [[ -z "$game_url" ]]; then
  run_log="$out_dir/run-game-test.log"
  ./scripts/run-game-test.sh --skip-llm-provider-preflight "${run_game_test_args[@]}" >"$run_log" 2>&1 &
  launcher_pid=$!
  deadline=$((SECONDS + startup_timeout_secs))
  while (( SECONDS < deadline )); do
    game_url=$(python3 - "$run_log" <<'PY'
import pathlib
import re
import sys
path = pathlib.Path(sys.argv[1])
if not path.exists():
    raise SystemExit(1)
text = path.read_text(encoding="utf-8", errors="ignore")
matches = re.findall(r"(http://[^\s]+)", text)
for candidate in reversed(matches):
    candidate = candidate.rstrip(')"\'')
    if "test_api=1" in candidate and "ws=" in candidate:
        print(candidate)
        raise SystemExit(0)
raise SystemExit(1)
PY
) || true
    if [[ -n "$game_url" ]]; then
      break
    fi
    sleep 1
  done
fi

[[ -n "$game_url" ]] || { echo "error: unable to resolve viewer URL" >&2; exit 1; }
game_url=$(append_query_params "$game_url")
session="viewer-pixel-world-wasm-$RANDOM"

ab_open "$session" "$headed" "$game_url" >>"$ab_log" 2>&1
ab_cmd "$session" wait --load networkidle >>"$ab_log" 2>&1 || true

wait_for_api 20000 || { echo "error: __AW_TEST__ unavailable" >&2; exit 1; }
initial_state=$(ab_state)
json_to_file "$initial_state" "$initial_state_path"

connected_state=$(wait_for_connected 30000) || {
  json_to_file "$connected_state" "$connected_state_path"
  echo "error: viewer did not reach connected state" >&2
  exit 1
}
json_to_file "$connected_state" "$connected_state_path"

render_mode=$(state_render_mode "$connected_state")
[[ "$render_mode" == "viewer" || "$render_mode" == "software_safe" ]] || {
  echo "error: expected renderMode=viewer-compatible, got $render_mode" >&2
  exit 1
}

ready_state=$(wait_for_pixel_runtime_ready 12000) || {
  json_to_file "$ready_state" "$ready_state_path"
  echo "error: pixel world runtime did not reach ready state" >&2
  exit 1
}
json_to_file "$ready_state" "$ready_state_path"

pixel_runtime_source=$(state_pixel_runtime_source "$ready_state")
pixel_runtime_status=$(state_pixel_runtime_status "$ready_state")
pixel_runtime_url=$(state_pixel_runtime_url "$ready_state")
last_error=$(state_last_error "$ready_state")

[[ "$pixel_runtime_source" == "wasm_bindgen_runtime" ]] || {
  echo "error: expected pixelWorldRuntimeSource=wasm_bindgen_runtime, got $pixel_runtime_source" >&2
  exit 1
}
[[ "$pixel_runtime_status" == "ready" ]] || {
  echo "error: expected pixelWorldRuntimeStatus=ready, got $pixel_runtime_status" >&2
  exit 1
}
[[ "$pixel_runtime_url" == *"pixel-world-bridge/pixel_world_bridge.js"* ]] || {
  echo "error: unexpected pixelWorldRuntimeModuleUrl: $pixel_runtime_url" >&2
  exit 1
}

final_state=$(ab_state)
json_to_file "$final_state" "$final_state_path"
ab_screenshot "$session" "$out_dir/final.png" >>"$ab_log" 2>&1 || true

json_to_md_summary "true" "$run_id" "$game_url" "$render_mode" "$pixel_runtime_status" "$pixel_runtime_source" "${pixel_runtime_url:-null}" "${last_error:-null}" >"$summary_json_path"
python3 - "$summary_json_path" "$summary_md_path" <<'PY'
import json
import pathlib
import sys

summary = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
lines = [
    "# viewer pixel world wasm regression",
    "",
    f"- ok: {'true' if summary['ok'] else 'false'}",
    f"- runId: `{summary['runId']}`",
    f"- gameUrl: `{summary['gameUrl']}`",
    f"- renderMode: `{summary['renderMode']}`",
    f"- pixelWorldRuntimeStatus: `{summary['pixelWorldRuntimeStatus']}`",
    f"- pixelWorldRuntimeSource: `{summary['pixelWorldRuntimeSource']}`",
    f"- pixelWorldRuntimeModuleUrl: `{summary['pixelWorldRuntimeModuleUrl'] or '(null)'}`",
    f"- lastError: `{summary['lastError'] or '(null)'}`",
]
pathlib.Path(sys.argv[2]).write_text("\n".join(lines) + "\n", encoding="utf-8")
PY

cat "$summary_md_path"
