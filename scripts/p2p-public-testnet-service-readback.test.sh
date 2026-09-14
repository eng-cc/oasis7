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
trap 'rm -rf "$TMP_DIR"' EXIT
FAKE_BIN="$TMP_DIR/bin"
mkdir -p "$FAKE_BIN" "$TMP_DIR/web" "$TMP_DIR/web-launcher"
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
      ActiveState=inactive \
      SubState=dead \
      UnitFileState=disabled \
      NRestarts=0
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
printf '  PID COMMAND\n'
EOF
cat >"$FAKE_BIN/ss" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'State      Recv-Q Send-Q Local Address:Port Peer Address:Port\n'
EOF
cat >"$FAKE_BIN/pgrep" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exit 1
EOF
chmod +x "$FAKE_BIN/systemctl" "$FAKE_BIN/ps" "$FAKE_BIN/ss" "$FAKE_BIN/pgrep"

run_readback() {
  PATH="$FAKE_BIN:$PATH" "$HELPER" \
    --read-only \
    --role storage \
    --root /opt/oasis7/p2p-testnet \
    --service oasis7-triad-storage.service
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

if FAKE_SERVICE_LOAD_STATE=not-found run_readback >"$TMP_DIR/not-found.out" 2>&1; then
  printf 'expected missing systemd unit to be rejected\n' >&2
  exit 1
fi

python3 - "$HELPER" <<'PY'
import importlib.machinery
import importlib.util
import sys

loader = importlib.machinery.SourceFileLoader("service_readback", sys.argv[1])
spec = importlib.util.spec_from_loader(loader.name, loader)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class MissingRoot:
    def __init__(self, _value):
        self.anchor = "/"
        self.parts = ("/", "opt", "oasis7", "p2p-testnet")
    def exists(self):
        return False

module.Path = MissingRoot
try:
    module._physical_root_is_safe(module.CANONICAL_ROOT)
except SystemExit:
    pass
else:
    raise AssertionError("missing canonical root was accepted")
PY

expect_rejected() {
  local label="$1"
  shift
  if PATH="$FAKE_BIN:$PATH" "$HELPER" "$@" >"$TMP_DIR/$label.out" 2>&1; then
    printf 'expected service-readback rejection: %s\n' "$label" >&2
    cat "$TMP_DIR/$label.out" >&2
    exit 1
  fi
}

# Fixed role/root/service binding rejects target drift and command injection.
expect_rejected wrong-role \
  --read-only --role sequencer --root /opt/oasis7/p2p-testnet --service oasis7-triad-storage.service
expect_rejected wrong-root \
  --read-only --role storage --root /tmp/oasis7-testnet --service oasis7-triad-storage.service
expect_rejected wrong-service \
  --read-only --role storage --root /opt/oasis7/p2p-testnet --service oasis7-triad-sequencer.service
expect_rejected unsafe-service \
  --read-only --role storage --root /opt/oasis7/p2p-testnet --service 'oasis7-triad-storage.service;systemctl stop'

# Mutation-like or unrecognized arguments must never reach a host command.
expect_rejected start-flag \
  --read-only --role storage --root /opt/oasis7/p2p-testnet --service oasis7-triad-storage.service --start
expect_rejected stop-flag \
  --read-only --role storage --root /opt/oasis7/p2p-testnet --service oasis7-triad-storage.service --stop
expect_rejected systemctl-flag \
  --read-only --role storage --root /opt/oasis7/p2p-testnet --service oasis7-triad-storage.service --systemctl stop

# A symlink/unsafe root is rejected even though the fixed production root is
# already outside this temporary test tree.
ln -s /opt/oasis7/p2p-testnet "$TMP_DIR/root-link"
expect_rejected symlink-root \
  --read-only --role storage --root "$TMP_DIR/root-link" --service oasis7-triad-storage.service

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

printf '%s\n' 'ok: production service-readback contract and Linux staging'
