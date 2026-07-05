#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

tmp_dir="${TMPDIR:-/tmp}/oasis7-local-public-testnet-letai-test-$$"
mkdir -p "$tmp_dir/bin"
trap 'rm -rf "$tmp_dir"' EXIT

assert_contains() {
  local path="$1"
  local expected="$2"
  if ! grep -Fq -- "$expected" "$path"; then
    echo "expected $path to contain: $expected" >&2
    echo "--- $path ---" >&2
    sed -n '1,180p' "$path" >&2 || true
    exit 1
  fi
}

assert_occurrences() {
  local path="$1"
  local expected="$2"
  local count="$3"
  local actual
  actual="$(grep -F -- "$expected" "$path" | wc -l | tr -d ' ')"
  if [[ "$actual" != "$count" ]]; then
    echo "expected $path to contain $expected exactly $count times, got $actual" >&2
    echo "--- $path ---" >&2
    sed -n '1,180p' "$path" >&2 || true
    exit 1
  fi
}

free_bind_addr() {
  python3 - <<'PY'
import socket
with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("127.0.0.1", 0))
    print(f"127.0.0.1:{sock.getsockname()[1]}")
PY
}

free_port() {
  free_bind_addr | sed 's/^.*://'
}

cat >"$tmp_dir/bin/curl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
url="${@: -1}"
printf '%s\n' "$url" >>"${FAKE_CURL_LOG:?}"
attempt=1
if [[ -n "${FAKE_CURL_COUNT_FILE:-}" ]]; then
  attempt="$(( $(cat "$FAKE_CURL_COUNT_FILE" 2>/dev/null || printf '0') + 1 ))"
  printf '%s\n' "$attempt" >"$FAKE_CURL_COUNT_FILE"
fi
readiness_status="ready"
failed_gates_json="[]"
if [[ -n "${FAKE_CURL_FAIL_UNTIL:-}" && "$attempt" -le "$FAKE_CURL_FAIL_UNTIL" ]]; then
  readiness_status="syncing"
  failed_gates_json='["catching_up"]'
fi
world_resource_status="${FAKE_WORLD_RESOURCE_STATUS:-ready}"
world_resource_failed_gates_json="${FAKE_WORLD_RESOURCE_FAILED_GATES_JSON:-[]}"
world_resource_json=""
if [[ "${FAKE_OMIT_WORLD_RESOURCE:-0}" != "1" ]]; then
  world_resource_json=',"world_resource":{"readiness_status":"'"$world_resource_status"'","failed_gates":'"$world_resource_failed_gates_json"'}'
fi
if [[ -n "${FAKE_CURL_TRANSPORT_FAIL_UNTIL:-}" && "$attempt" -le "$FAKE_CURL_TRANSPORT_FAIL_UNTIL" ]]; then
  echo "curl: (7) failed to connect to fake endpoint" >&2
  exit 7
fi
case "$url" in
  http://remote.test:6631/v1/chain/status|http://read.test:19083/v1/chain/status|http://submit.test:6631/v1/chain/status)
    printf '{"node_id":"triad-testnet-sequencer","role":"sequencer","world_id":"oasis7-public-testnet-governed-20260606","network_tier":{"tier":"public_testnet","chain_id":"oasis7-public-testnet-governed-20260606","network_id":"oasis7-public-testnet-governed-20260606"},"readiness":{"status":"%s","failed_gates":%s},"consensus":{"committed_height":42,"network_committed_height":42},"observability":{"network_height_lag":0,"connected_peer_count":2}%s}\n' "$readiness_status" "$failed_gates_json" "$world_resource_json"
    ;;
  *)
    echo "unexpected curl URL: $url" >&2
    exit 22
    ;;
esac
EOF
chmod +x "$tmp_dir/bin/curl"

cat >"$tmp_dir/bin/sleep" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
chmod +x "$tmp_dir/bin/sleep"

chain_submit_only_out="$tmp_dir/chain-submit-only.out"
FAKE_CURL_LOG="$tmp_dir/chain-submit-only.urls" \
PATH="$tmp_dir/bin:$PATH" \
  ./scripts/run-local-public-testnet-letai-test-environment.sh \
    --chain-submit-base-url http://remote.test:6631 \
    --skip-newapi-bridge \
    --skip-provider-bridge \
    --skip-static-viewer \
    --viewer-api-bind "$(free_bind_addr)" \
    --viewer-ws-bind "$(free_bind_addr)" \
    --preflight-only \
    >"$chain_submit_only_out"

assert_contains "$chain_submit_only_out" "node: http://remote.test:6631"
assert_contains "$chain_submit_only_out" "chain_status_bind: remote.test:6631"
assert_contains "$chain_submit_only_out" "chain_submit_base_url: http://remote.test:6631"
assert_contains "$tmp_dir/chain-submit-only.urls" "http://remote.test:6631/v1/chain/status"

one_arg_out="$tmp_dir/one-arg.out"
FAKE_CURL_LOG="$tmp_dir/one-arg.urls" \
PATH="$tmp_dir/bin:$PATH" \
  ./scripts/run-local-public-testnet-letai-test-environment.sh \
    --public-testnet-base-url http://remote.test:6631 \
    --skip-newapi-bridge \
    --skip-provider-bridge \
    --skip-static-viewer \
    --viewer-api-bind "$(free_bind_addr)" \
    --viewer-ws-bind "$(free_bind_addr)" \
    --static-port "$(free_port)" \
    --preflight-only \
    >"$one_arg_out"

assert_contains "$one_arg_out" "node: http://remote.test:6631"
assert_contains "$one_arg_out" "chain_status_bind: remote.test:6631"
assert_contains "$one_arg_out" "chain_submit_base_url: http://remote.test:6631"
assert_contains "$tmp_dir/one-arg.urls" "http://remote.test:6631/v1/chain/status"

explicit_split_out="$tmp_dir/explicit-split.out"
FAKE_CURL_LOG="$tmp_dir/explicit-split.urls" \
PATH="$tmp_dir/bin:$PATH" \
  ./scripts/run-local-public-testnet-letai-test-environment.sh \
    --node-base-url http://read.test:19083 \
    --chain-submit-base-url http://submit.test:6631 \
    --skip-newapi-bridge \
    --skip-provider-bridge \
    --skip-static-viewer \
    --viewer-api-bind "$(free_bind_addr)" \
    --viewer-ws-bind "$(free_bind_addr)" \
    --preflight-only \
    >"$explicit_split_out"

assert_contains "$explicit_split_out" "node: http://read.test:19083"
assert_contains "$explicit_split_out" "chain_status_bind: read.test:19083"
assert_contains "$explicit_split_out" "chain_submit_base_url: http://submit.test:6631"
assert_contains "$tmp_dir/explicit-split.urls" "http://read.test:19083/v1/chain/status"
assert_contains "$tmp_dir/explicit-split.urls" "http://submit.test:6631/v1/chain/status"

retry_out="$tmp_dir/retry.out"
FAKE_CURL_LOG="$tmp_dir/retry.urls" \
FAKE_CURL_COUNT_FILE="$tmp_dir/retry.count" \
FAKE_CURL_FAIL_UNTIL=2 \
PATH="$tmp_dir/bin:$PATH" \
  ./scripts/run-local-public-testnet-letai-test-environment.sh \
    --public-testnet-base-url http://remote.test:6631 \
    --skip-newapi-bridge \
    --skip-provider-bridge \
    --skip-static-viewer \
    --viewer-api-bind "$(free_bind_addr)" \
    --viewer-ws-bind "$(free_bind_addr)" \
    --preflight-only \
    >"$retry_out" 2>&1

assert_contains "$retry_out" "public_testnet submit endpoint not ready yet (attempt 1/5); retrying"
assert_contains "$retry_out" "public_testnet submit endpoint not ready yet (attempt 2/5); retrying"
assert_occurrences "$tmp_dir/retry.urls" "http://remote.test:6631/v1/chain/status" "4"

transport_retry_out="$tmp_dir/transport-retry.out"
FAKE_CURL_LOG="$tmp_dir/transport-retry.urls" \
FAKE_CURL_COUNT_FILE="$tmp_dir/transport-retry.count" \
FAKE_CURL_TRANSPORT_FAIL_UNTIL=2 \
PATH="$tmp_dir/bin:$PATH" \
  ./scripts/run-local-public-testnet-letai-test-environment.sh \
    --public-testnet-base-url http://remote.test:6631 \
    --skip-newapi-bridge \
    --skip-provider-bridge \
    --skip-static-viewer \
    --viewer-api-bind "$(free_bind_addr)" \
    --viewer-ws-bind "$(free_bind_addr)" \
    --preflight-only \
    >"$transport_retry_out" 2>&1

assert_contains "$transport_retry_out" "public_testnet submit endpoint not reachable yet (attempt 1/5); retrying"
assert_contains "$transport_retry_out" "public_testnet submit endpoint not reachable yet (attempt 2/5); retrying"
assert_occurrences "$tmp_dir/transport-retry.urls" "http://remote.test:6631/v1/chain/status" "4"

world_resource_fail_out="$tmp_dir/world-resource-fail.out"
if FAKE_CURL_LOG="$tmp_dir/world-resource-fail.urls" \
FAKE_WORLD_RESOURCE_STATUS=not_ready \
FAKE_WORLD_RESOURCE_FAILED_GATES_JSON='["world_resource_world_id_mismatch"]' \
PATH="$tmp_dir/bin:$PATH" \
  ./scripts/run-local-public-testnet-letai-test-environment.sh \
    --public-testnet-base-url http://remote.test:6631 \
    --skip-newapi-bridge \
    --skip-provider-bridge \
    --skip-static-viewer \
    --viewer-api-bind "$(free_bind_addr)" \
    --viewer-ws-bind "$(free_bind_addr)" \
    --preflight-only \
    >"$world_resource_fail_out" 2>&1; then
  echo "expected world_resource not_ready to fail preflight" >&2
  exit 1
fi
assert_contains "$world_resource_fail_out" "world_resource_readiness='not_ready'"
assert_contains "$world_resource_fail_out" "world_resource_failed_gates=['world_resource_world_id_mismatch']"

assert_contains scripts/run-local-public-testnet-letai-test-environment.sh "stop_stale_viewer_live_services"
assert_contains scripts/run-local-public-testnet-letai-test-environment.sh "oasis7.local-public-testnet.viewer-live-clean"
assert_contains scripts/run-local-public-testnet-letai-test-environment.sh '[[ "$REUSE_EXISTING" == "1" ]]'
python3 - <<'PY'
from pathlib import Path
source = Path("scripts/run-local-public-testnet-letai-test-environment.sh").read_text()
public_testnet_index = source.index("check_public_testnet_node")
cleanup_index = source.index("stop_stale_viewer_live_services", public_testnet_index)
preflight_index = source.index('check_or_reuse_port "viewer live API" "$VIEWER_API_BIND" >/dev/null || true')
if not public_testnet_index < cleanup_index < preflight_index:
    raise SystemExit("expected stale viewer live cleanup before viewer port preflight")
PY

echo "run-local-public-testnet-letai-test-environment checks passed"
