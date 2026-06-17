#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

tmp_dir="${TMPDIR:-/tmp}/oasis7-local-letai-game-test-cli-$$"
mkdir -p "$tmp_dir"
trap 'rm -rf "$tmp_dir"' EXIT

assert_contains() {
  local path="$1"
  local expected="$2"
  if ! grep -Fq -- "$expected" "$path"; then
    echo "expected $path to contain: $expected" >&2
    echo "--- $path ---" >&2
    sed -n '1,160p' "$path" >&2 || true
    exit 1
  fi
}

free_bind_addr() {
  python3 - <<'PY'
from __future__ import annotations

import socket

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("127.0.0.1", 0))
    print(f"127.0.0.1:{sock.getsockname()[1]}")
PY
}

free_port() {
  free_bind_addr | sed 's/^.*://'
}

config_path="$tmp_dir/letai.env"
cat >"$config_path" <<'EOF'
Key: test-secret-key
EOF

fake_bin_dir="$tmp_dir/bin"
mkdir -p "$fake_bin_dir"
for tool in node npm; do
  printf '#!/usr/bin/env bash\nexit 0\n' >"$fake_bin_dir/$tool"
  chmod +x "$fake_bin_dir/$tool"
done
for bin in \
  oasis7_llm_provider_probe \
  oasis7_game_launcher \
  oasis7_viewer_live \
  oasis7_chain_runtime \
  oasis7_provider_local_bridge
do
  printf '#!/usr/bin/env bash\nexit 0\n' >"$fake_bin_dir/$bin"
  chmod +x "$fake_bin_dir/$bin"
done

dist_dir="$tmp_dir/dist"
mkdir -p "$dist_dir"
touch "$dist_dir/index.html"

preflight_out="$tmp_dir/preflight.out"
OASIS7_LOCAL_LETAI_SOURCE_BIN_DIR="$fake_bin_dir" \
OASIS7_LOCAL_LETAI_VIEWER_DIST_DIR="$dist_dir" \
PATH="$fake_bin_dir:$PATH" \
	  ./scripts/run-local-letai-game-test.sh \
	    --config "$config_path" \
	    --bind "$(free_bind_addr)" \
	    --no-ensure-token-config \
	    --no-default-proxy \
	    --preflight-only \
	    --startup-profile playtest \
	    --reuse-existing-build \
	    --output-dir "$tmp_dir/out" \
	    -- \
	    --viewer-port "$(free_port)" \
	    --live-bind "$(free_bind_addr)" \
	    --web-bind "$(free_bind_addr)" \
	    --chain-disable \
	    >"$preflight_out"

assert_contains "$preflight_out" "local LetAI game test preflight passed"
assert_contains "$preflight_out" "startup_profile=playtest"
assert_contains "$preflight_out" "provider_smoke_mode=degraded"
assert_contains "$preflight_out" "source_build=reuse-existing"

missing_npm_err="$tmp_dir/missing-npm.err"
set +e
OASIS7_LOCAL_LETAI_TEST_MISSING_NPM=1 \
OASIS7_LOCAL_LETAI_VIEWER_DIST_DIR="$tmp_dir/missing-dist" \
PATH="$fake_bin_dir:$PATH" \
	  ./scripts/run-local-letai-game-test.sh \
	    --config "$config_path" \
	    --bind "$(free_bind_addr)" \
	    --no-ensure-token-config \
	    --no-default-proxy \
	    --preflight-only \
	    --output-dir "$tmp_dir/out-missing-npm" \
	    -- \
	    --viewer-port "$(free_port)" \
	    --live-bind "$(free_bind_addr)" \
	    --web-bind "$(free_bind_addr)" \
	    --chain-disable \
	    >"$tmp_dir/missing-npm.out" 2>"$missing_npm_err"
missing_npm_status=$?
set -e

if [[ "$missing_npm_status" -eq 0 ]]; then
  echo "expected missing npm preflight to fail" >&2
  exit 1
fi
assert_contains "$missing_npm_err" "error: local playtest preflight failed: missing npm and viewer dist"
assert_contains "$missing_npm_err" "Install npm, or build/copy viewer dist"
assert_contains "$missing_npm_err" "rerun with --reuse-existing-build only after a successful source build"

missing_config_err="$tmp_dir/missing-config.err"
set +e
OASIS7_LOCAL_LETAI_SOURCE_BIN_DIR="$fake_bin_dir" \
OASIS7_LOCAL_LETAI_VIEWER_DIST_DIR="$dist_dir" \
PATH="$fake_bin_dir:$PATH" \
  ./scripts/run-local-letai-game-test.sh \
    --config "$tmp_dir/missing-letai.env" \
    --bind "$(free_bind_addr)" \
    --no-ensure-token-config \
    --no-default-proxy \
    --preflight-only \
    --reuse-existing-build \
    --output-dir "$tmp_dir/out-missing-config" \
    -- \
    --viewer-port "$(free_port)" \
    --live-bind "$(free_bind_addr)" \
    --web-bind "$(free_bind_addr)" \
    --chain-disable \
    >"$tmp_dir/missing-config.out" 2>"$missing_config_err"
missing_config_status=$?
set -e

if [[ "$missing_config_status" -eq 0 ]]; then
  echo "expected missing config preflight to fail" >&2
  exit 1
fi
assert_contains "$missing_config_err" "error: local playtest preflight failed: LetAI config not found"
assert_contains "$missing_config_err" "pass --config <path> or set OASIS7_LETAI_CONFIG_PATH"

occupied_port_file="$tmp_dir/occupied-port.txt"
python3 - "$occupied_port_file" <<'PY' &
from __future__ import annotations

import socket
import sys
import time

port_file = sys.argv[1]
sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.bind(("127.0.0.1", 0))
sock.listen(1)
with open(port_file, "w", encoding="utf-8") as handle:
    handle.write(str(sock.getsockname()[1]))
    handle.flush()
time.sleep(30)
PY
occupied_listener_pid=$!
for _ in $(seq 1 50); do
  [[ -s "$occupied_port_file" ]] && break
  sleep 0.1
done
occupied_port="$(cat "$occupied_port_file")"
occupied_err="$tmp_dir/occupied-bind.err"
set +e
OASIS7_LOCAL_LETAI_SOURCE_BIN_DIR="$fake_bin_dir" \
OASIS7_LOCAL_LETAI_VIEWER_DIST_DIR="$dist_dir" \
PATH="$fake_bin_dir:$PATH" \
  ./scripts/run-local-letai-game-test.sh \
    --config "$config_path" \
    --bind "127.0.0.1:$occupied_port" \
    --no-ensure-token-config \
    --no-default-proxy \
    --preflight-only \
    --reuse-existing-build \
    --output-dir "$tmp_dir/out-occupied-bind" \
    -- \
    --viewer-port "$(free_port)" \
    --live-bind "$(free_bind_addr)" \
    --web-bind "$(free_bind_addr)" \
    --chain-disable \
    >"$tmp_dir/occupied-bind.out" 2>"$occupied_err"
occupied_status=$?
set -e
kill "$occupied_listener_pid" >/dev/null 2>&1 || true
wait "$occupied_listener_pid" >/dev/null 2>&1 || true

if [[ "$occupied_status" -eq 0 ]]; then
  echo "expected occupied bind preflight to fail" >&2
  exit 1
fi
assert_contains "$occupied_err" "error: local playtest preflight failed: provider bind address is already in use"
assert_contains "$occupied_err" "Stop the previous local playtest stack, or pass --bind <free host:port>"

missing_node_err="$tmp_dir/missing-node.err"
set +e
OASIS7_LOCAL_LETAI_TEST_MISSING_NODE=1 \
OASIS7_LOCAL_LETAI_SOURCE_BIN_DIR="$fake_bin_dir" \
OASIS7_LOCAL_LETAI_VIEWER_DIST_DIR="$dist_dir" \
PATH="$fake_bin_dir:$PATH" \
	  ./scripts/run-local-letai-game-test.sh \
	    --config "$config_path" \
	    --bind "$(free_bind_addr)" \
	    --no-ensure-token-config \
	    --no-default-proxy \
	    --preflight-only \
	    --reuse-existing-build \
	    --output-dir "$tmp_dir/out-missing-node" \
	    -- \
	    --viewer-port "$(free_port)" \
	    --live-bind "$(free_bind_addr)" \
	    --web-bind "$(free_bind_addr)" \
	    --chain-disable \
	    >"$tmp_dir/missing-node.out" 2>"$missing_node_err"
missing_node_status=$?
set -e

if [[ "$missing_node_status" -eq 0 ]]; then
  echo "expected missing Node.js preflight to fail" >&2
  exit 1
fi
assert_contains "$missing_node_err" "error: local playtest preflight failed: missing Node.js runtime"
assert_contains "$missing_node_err" "Install Node.js and npm, or use a prepared bundle/source build that does not require frontend rebuilds"

missing_build_err="$tmp_dir/missing-build.err"
set +e
OASIS7_LOCAL_LETAI_SOURCE_BIN_DIR="$tmp_dir/missing-bin" \
OASIS7_LOCAL_LETAI_VIEWER_DIST_DIR="$dist_dir" \
PATH="$fake_bin_dir:$PATH" \
	  ./scripts/run-local-letai-game-test.sh \
	    --config "$config_path" \
	    --bind "$(free_bind_addr)" \
	    --no-ensure-token-config \
	    --no-default-proxy \
	    --preflight-only \
	    --reuse-existing-build \
	    --output-dir "$tmp_dir/out-missing-build" \
	    -- \
	    --viewer-port "$(free_port)" \
	    --live-bind "$(free_bind_addr)" \
	    --web-bind "$(free_bind_addr)" \
	    --chain-disable \
	    >"$tmp_dir/missing-build.out" 2>"$missing_build_err"
missing_build_status=$?
set -e

if [[ "$missing_build_status" -eq 0 ]]; then
  echo "expected missing build preflight to fail" >&2
  exit 1
fi
assert_contains "$missing_build_err" "error: local playtest preflight failed: --reuse-existing-build requested but required binaries are missing"
assert_contains "$missing_build_err" "Run without --reuse-existing-build once"

strict_plan="$tmp_dir/strict-plan.out"
OASIS7_LOCAL_LETAI_SOURCE_BIN_DIR="$fake_bin_dir" \
OASIS7_LOCAL_LETAI_VIEWER_DIST_DIR="$dist_dir" \
PATH="$fake_bin_dir:$PATH" \
	  ./scripts/run-local-letai-game-test.sh \
	    --config "$config_path" \
	    --bind "$(free_bind_addr)" \
	    --no-ensure-token-config \
	    --no-default-proxy \
	    --dry-run-launch \
	    --provider-smoke-mode strict \
	    --reuse-existing-build \
	    --output-dir "$tmp_dir/out-strict" \
	    -- \
	    --viewer-port "$(free_port)" \
	    --live-bind "$(free_bind_addr)" \
	    --web-bind "$(free_bind_addr)" \
	    --chain-disable \
	    >"$strict_plan"

assert_contains "$strict_plan" "provider_smoke_mode=strict"
assert_contains "$strict_plan" "bridge_smoke=required"
if grep -Fq -- "--skip-bridge-smoke" "$strict_plan"; then
  echo "strict provider smoke mode must not skip bridge smoke" >&2
  exit 1
fi
if grep -Fq -- "--skip-llm-provider-preflight" "$strict_plan"; then
  echo "strict provider smoke mode must not skip launcher provider preflight" >&2
  exit 1
fi

degraded_plan="$tmp_dir/degraded-plan.out"
OASIS7_LOCAL_LETAI_SOURCE_BIN_DIR="$fake_bin_dir" \
OASIS7_LOCAL_LETAI_VIEWER_DIST_DIR="$dist_dir" \
PATH="$fake_bin_dir:$PATH" \
	  ./scripts/run-local-letai-game-test.sh \
	    --config "$config_path" \
	    --bind "$(free_bind_addr)" \
	    --no-ensure-token-config \
	    --no-default-proxy \
	    --dry-run-launch \
	    --provider-smoke-mode degraded \
	    --reuse-existing-build \
	    --output-dir "$tmp_dir/out-degraded" \
	    -- \
	    --viewer-port "$(free_port)" \
	    --live-bind "$(free_bind_addr)" \
	    --web-bind "$(free_bind_addr)" \
	    --chain-disable \
	    >"$degraded_plan"

assert_contains "$degraded_plan" "provider_smoke_mode=degraded"
assert_contains "$degraded_plan" "bridge_smoke=degrade-on-failure"
assert_contains "$degraded_plan" "degraded startup will continue after provider smoke failure"
assert_contains "$degraded_plan" "--skip-llm-provider-preflight"
assert_contains "$degraded_plan" "OASIS7_RUN_LAUNCHER_STACK_SKIP_SOURCE_BUILD=1"

detached_out="$tmp_dir/detached.out"
detached_dir="$tmp_dir/out-detached"
detached_abs_dir="$(mkdir -p "$detached_dir" && cd "$detached_dir" && pwd)"
OASIS7_LOCAL_LETAI_SOURCE_BIN_DIR="$fake_bin_dir" \
OASIS7_LOCAL_LETAI_VIEWER_DIST_DIR="$dist_dir" \
OASIS7_LOCAL_LETAI_TEST_DETACH_NO_SUBMIT=1 \
PATH="$fake_bin_dir:$PATH" \
	  ./scripts/run-local-letai-game-test.sh \
	    --config "$config_path" \
	    --bind "$(free_bind_addr)" \
	    --no-ensure-token-config \
	    --no-default-proxy \
	    --detach \
	    --provider-smoke-mode degraded \
	    --reuse-existing-build \
	    --output-dir "$detached_dir" \
	    -- \
	    --viewer-port "$(free_port)" \
	    --live-bind "$(free_bind_addr)" \
	    --web-bind "$(free_bind_addr)" \
	    --chain-disable \
	    >"$detached_out"

assert_contains "$detached_out" "local LetAI game test detached"
assert_contains "$detached_out" "supervisor_script=$detached_abs_dir/local-letai-game-test.detached.sh"
assert_contains "$detached_abs_dir/local-letai-game-test.detached.sh" "OASIS7_RUN_LAUNCHER_STACK_SKIP_SOURCE_BUILD=1"
assert_contains "$detached_abs_dir/local-letai-game-test.detached.sh" "--provider-smoke-mode degraded"
assert_contains "$detached_abs_dir/local-letai-game-test.detached.sh" "--reuse-existing-build"
assert_contains "$detached_abs_dir/local-letai-game-test.detached.sh" "--skip-llm-provider-preflight"

echo "local letai game test CLI smoke passed"
