#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
HELPER="$ROOT_DIR/scripts/service-readback"
BUNDLE_BUILDER="$ROOT_DIR/scripts/build-game-launcher-bundle.sh"
FRESH_HOST_BOOTSTRAP="$ROOT_DIR/scripts/p2p-public-testnet-bootstrap-fresh-validator-host.sh"

# RED contract: the governed remote readback must be a repository-owned
# executable before a package or host can claim the fixed service contract.
if [[ ! -f "$HELPER" || ! -x "$HELPER" ]]; then
  printf 'RED: missing production service-readback executable: %s\n' "$HELPER" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-service-readback.XXXXXX")"
TMP_DIR="$(cd "$TMP_DIR" && pwd -P)"
trap 'rm -rf "$TMP_DIR"' EXIT
FAKE_BIN="$TMP_DIR/bin"
TEST_ROOT="$TMP_DIR/canonical-root"
mkdir -p "$FAKE_BIN" "$TEST_ROOT" "$TMP_DIR/web" "$TMP_DIR/web-launcher"
printf '<!doctype html>\n' >"$TMP_DIR/web/index.html"
printf '<!doctype html>\n' >"$TMP_DIR/web-launcher/index.html"

# The helper may use only read-only host tools. These deterministic fixtures
# keep the contract test independent of the developer workstation's service
# manager and listener table.
cat >"$FAKE_BIN/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  is-active) printf '%s\n' inactive ;;
  is-enabled) printf '%s\n' disabled ;;
  show)
    printf '%s\n' \
      LoadState="${FAKE_SERVICE_LOAD_STATE:-loaded}" \
      ActiveState="${FAKE_ACTIVE_STATE:-inactive}" \
      SubState="${FAKE_SUB_STATE:-dead}" \
      UnitFileState="${FAKE_UNIT_FILE_STATE:-disabled}" \
      NRestarts="${FAKE_NRESTARTS:-0}" \
      MainPID="${FAKE_MAIN_PID:-0}"
    ;;
  *)
    printf 'unexpected systemctl operation\n' >&2
    exit 2
    ;;
esac
EOF
cat >"$FAKE_BIN/ps" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "-p" && "${FAKE_MAIN_PID:-0}" != "0" ]]; then
  printf '/opt/oasis7/p2p-testnet/current/bin/oasis7_chain_runtime\n'
  exit 0
fi
printf '  PID COMMAND\n'
EOF
cat >"$FAKE_BIN/ss" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'State      Recv-Q Send-Q Local Address:Port Peer Address:Port\n'
for port in ${FAKE_PORTS:-}; do
  printf 'LISTEN     0      128    127.0.0.1:%s      0.0.0.0:*\n' "$port"
done
EOF
cat >"$FAKE_BIN/pgrep" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exit 1
EOF
chmod +x "$FAKE_BIN/systemctl" "$FAKE_BIN/ps" "$FAKE_BIN/ss" "$FAKE_BIN/pgrep"

invoke_readback() {
  PATH="$FAKE_BIN:$PATH" READBACK_CANONICAL_ROOT="${READBACK_CANONICAL_ROOT:-$TEST_ROOT}" \
    python3 - "$HELPER" "$@" <<'PY'
import importlib.machinery
import importlib.util
import os
import sys

loader = importlib.machinery.SourceFileLoader("service_readback_test_target", sys.argv[1])
spec = importlib.util.spec_from_loader(loader.name, loader)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
module.CANONICAL_ROOT = os.environ["READBACK_CANONICAL_ROOT"]
raise SystemExit(module.main(sys.argv[2:]))
PY
}

run_readback() {
  run_role_readback storage oasis7-triad-storage.service
}

run_role_readback() {
  local role=$1
  local service=$2
  invoke_readback \
    --read-only \
    --role "$role" \
    --root "$TEST_ROOT" \
    --service "$service"
}

readback_json="$(run_readback)"
python3 - "$readback_json" <<'PY'
import json
import sys

value = json.loads(sys.argv[1])
assert set(value) == {
    "schema_version",
    "active",
    "running",
    "service_state",
    "independently_observed",
    "listeners",
}, value
assert value["schema_version"] == "oasis7.human_direct_ssh_readback.v1", value
assert value["active"] is False, value
assert value["running"] is False, value
assert value["service_state"] == "stopped", value
assert value["independently_observed"] is True, value
assert value["listeners"] == [], value
PY

# Active storage and sequencer units are valid read-only observations.  The
# service helper must preserve the structured output schema while exposing the
# live state needed by the governed triad adapter.
active_storage_json="$({
  FAKE_ACTIVE_STATE=active \
  FAKE_SUB_STATE=running \
  FAKE_UNIT_FILE_STATE=enabled \
  FAKE_MAIN_PID=12345 \
  FAKE_PORTS='6632 6832' \
  run_role_readback storage oasis7-triad-storage.service
})"
active_sequencer_json="$({
  FAKE_ACTIVE_STATE=active \
  FAKE_SUB_STATE=running \
  FAKE_UNIT_FILE_STATE=enabled \
  FAKE_MAIN_PID=12345 \
  FAKE_PORTS='6631 6831' \
  run_role_readback sequencer oasis7-triad-sequencer.service
})"
python3 - "$active_storage_json" "$active_sequencer_json" <<'PY'
import json
import sys

for value, ports in zip(
    (json.loads(sys.argv[1]), json.loads(sys.argv[2])),
    (['6632', '6832'], ['6631', '6831']),
):
    assert set(value) == {
        'schema_version',
        'active',
        'running',
        'service_state',
        'independently_observed',
        'listeners',
    }, value
    assert value['active'] is True, value
    assert value['running'] is True, value
    assert value['service_state'] == 'running', value
    assert value['independently_observed'] is True, value
    assert value['listeners'] == ports, value
PY

# validator-47 remains a no-start role: an active unit is rejected before any
# identity/inventory readback can be treated as evidence.
if FAKE_ACTIVE_STATE=active FAKE_SUB_STATE=running FAKE_UNIT_FILE_STATE=enabled \
  FAKE_MAIN_PID=12345 FAKE_PORTS='6634 6834' \
  run_role_readback validator-47 oasis7-triad-validator-47.service >"$TMP_DIR/validator-47-active.out" 2>&1; then
  printf 'expected validator-47 active service to be rejected\n' >&2
  exit 1
fi

# Only active/running is a valid live service state for the Type=simple chain
# units. Socket/oneshot-like states and a missing active MainPID fail closed.
for invalid_substate in listening exited failed; do
  if FAKE_ACTIVE_STATE=active FAKE_SUB_STATE="$invalid_substate" FAKE_UNIT_FILE_STATE=enabled \
    FAKE_MAIN_PID=12345 FAKE_PORTS='6632 6832' \
    run_readback >"$TMP_DIR/active-$invalid_substate.out" 2>&1; then
    printf 'expected active/%s service state to be rejected\n' "$invalid_substate" >&2
    exit 1
  fi
done
if FAKE_ACTIVE_STATE=active FAKE_SUB_STATE=running FAKE_UNIT_FILE_STATE=enabled \
  FAKE_MAIN_PID=0 FAKE_PORTS='6632 6832' \
  run_readback >"$TMP_DIR/active-missing-main-pid.out" 2>&1; then
  printf 'expected active service without a MainPID to be rejected\n' >&2
  exit 1
fi
if FAKE_ACTIVE_STATE=active FAKE_SUB_STATE=running FAKE_UNIT_FILE_STATE=enabled \
  FAKE_MAIN_PID=not-a-pid FAKE_PORTS='6632 6832' \
  run_readback >"$TMP_DIR/active-malformed-main-pid.out" 2>&1; then
  printf 'expected active service with malformed MainPID to be rejected\n' >&2
  exit 1
fi

if FAKE_SERVICE_LOAD_STATE=not-found run_readback >"$TMP_DIR/not-found.out" 2>&1; then
  printf 'expected missing systemd unit to be rejected\n' >&2
  exit 1
fi

if READBACK_CANONICAL_ROOT="$TMP_DIR/missing-root" invoke_readback \
  --read-only --role storage --root "$TMP_DIR/missing-root" \
  --service oasis7-triad-storage.service >"$TMP_DIR/missing-root.out" 2>&1; then
  printf 'expected missing canonical root to be rejected\n' >&2
  exit 1
fi

expect_rejected() {
  local label="$1"
  shift
  if invoke_readback "$@" >"$TMP_DIR/$label.out" 2>&1; then
    printf 'expected service-readback rejection: %s\n' "$label" >&2
    cat "$TMP_DIR/$label.out" >&2
    exit 1
  fi
}

# Fixed role/root/service binding rejects target drift and command injection.
expect_rejected wrong-role \
  --read-only --role sequencer --root "$TEST_ROOT" --service oasis7-triad-storage.service
expect_rejected wrong-root \
  --read-only --role storage --root /tmp/oasis7-testnet --service oasis7-triad-storage.service
expect_rejected wrong-service \
  --read-only --role storage --root "$TEST_ROOT" --service oasis7-triad-sequencer.service
expect_rejected unsafe-service \
  --read-only --role storage --root "$TEST_ROOT" --service 'oasis7-triad-storage.service;systemctl stop'

# Mutation-like or unrecognized arguments must never reach a host command.
expect_rejected start-flag \
  --read-only --role storage --root "$TEST_ROOT" --service oasis7-triad-storage.service --start
expect_rejected stop-flag \
  --read-only --role storage --root "$TEST_ROOT" --service oasis7-triad-storage.service --stop
expect_rejected systemctl-flag \
  --read-only --role storage --root "$TEST_ROOT" --service oasis7-triad-storage.service --systemctl stop

# A symlink/unsafe root is rejected even though the fixed production root is
# already outside this temporary test tree.
ln -s "$TEST_ROOT" "$TMP_DIR/root-link"
if READBACK_CANONICAL_ROOT="$TMP_DIR/root-link" invoke_readback \
  --read-only --role storage --root "$TMP_DIR/root-link" \
  --service oasis7-triad-storage.service >"$TMP_DIR/symlink-root.out" 2>&1; then
  printf 'expected symlink canonical root to be rejected\n' >&2
  exit 1
fi

# Packaging contract: the helper is staged in the Linux ops-tools bundle and
# the fresh-host deployment copies that bundle into the active validator bin.
grep -Fq 'service-readback' "$BUNDLE_BUILDER"
grep -Fq 'service-readback' "$FRESH_HOST_BOOTSTRAP"
grep -Fq 'SERVICE_READBACK_COMMAND = f"{PRODUCTION_STACK_ROOT}/current/bin/service-readback --read-only"' \
  "$ROOT_DIR/scripts/p2p-public-testnet-validator-pair-rebuild.py"
grep -Fq 'command = f"{SERVICE_READBACK_COMMAND} --role' \
  "$ROOT_DIR/scripts/p2p-public-testnet-validator-pair-rebuild.py"
dry_run_output="$TMP_DIR/linux-bundle-dry-run.out"
PATH="$FAKE_BIN:$PATH" "$BUNDLE_BUILDER" \
  --dry-run \
  --platform linux-x64 \
  --target-triple x86_64-unknown-linux-gnu \
  --web-dist "$TMP_DIR/web" \
  --web-launcher-dist "$TMP_DIR/web-launcher" \
  --out-dir "$TMP_DIR/assets" \
  --ops-out-dir "$TMP_DIR/assets/oasis7-linux-x64-ops-tools" \
  >"$dry_run_output"
grep -Fq 'service-readback' "$dry_run_output"
grep -Fq 'oasis7-linux-x64-ops-tools/bin/service-readback' "$dry_run_output"
grep -Fqx '  run bash ./scripts/p2p-public-testnet-service-readback.test.sh' \
  "$ROOT_DIR/scripts/ci-tests.sh"

printf '%s\n' 'ok: production service-readback contract and Linux staging'
