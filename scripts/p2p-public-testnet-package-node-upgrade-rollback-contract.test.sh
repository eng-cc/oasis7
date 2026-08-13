#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-package-upgrade-rollback-contract.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

make_node_fixture() {
  local root=$1
  mkdir -p "$root/releases/old/bin" "$root/releases/drift/bin" \
    "$root/config/doc/testing/evidence"
  printf 'runtime-old\n' >"$root/releases/old/bin/oasis7_chain_runtime"
  printf 'runtime-drift\n' >"$root/releases/drift/bin/oasis7_chain_runtime"
  chmod +x "$root/releases/old/bin/oasis7_chain_runtime" "$root/releases/drift/bin/oasis7_chain_runtime"
  ln -s "$root/releases/old" "$root/current"
  cat >"$root/config/doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json" <<'EOF'
{
  "schema_version": "oasis7.release_candidate_bundle.v1",
  "git_commit": "old",
  "runtime_build": {"path": "old", "ref": "old", "sha256": "00", "size_bytes": 1}
}
EOF
}

make_bundle() {
  local root=$1
  mkdir -p "$root/oasis7-linux-x64/bin"
  printf 'runtime-new\n' >"$root/oasis7-linux-x64/bin/oasis7_chain_runtime"
  chmod +x "$root/oasis7-linux-x64/bin/oasis7_chain_runtime"
  tar -czf "$root/oasis7-linux-x64-bundle.tar.gz" -C "$root" oasis7-linux-x64
}

make_fake_bin() {
  local bin=$1
  mkdir -p "$bin"
  cat >"$bin/systemctl" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
root=${FAKE_NODE_ROOT:?}
log=${FAKE_SYSTEMCTL_LOG:?}
command_name=${1:-}
if [[ -L "$root/current" ]]; then marker=$(readlink "$root/current"); else marker=regular-file; fi
printf 'systemctl %s current=%s\n' "$*" "$marker" >>"$log"
if [[ "$command_name" == start && "${FAKE_DRIFT_ON_START:-0}" == 1 ]]; then
  starts=$(grep -c '^systemctl start ' "$log" || true)
  if [[ "$starts" -eq 1 ]]; then
    rm -f "$root/current"
    ln -s "$root/releases/drift" "$root/current"
    printf 'injected_current_drift=%s\n' "$(readlink "$root/current")" >>"$log"
  fi
fi
if [[ "$command_name" == stop && "${FAKE_DRIFT_ON_STOP:-0}" == 1 ]]; then
  stops=$(grep -c '^systemctl stop ' "$log" || true)
  if [[ "$stops" -eq 2 ]]; then
    rm -f "$root/current"
    ln -s "$root/releases/drift" "$root/current"
    printf 'injected_current_drift_on_stop=%s\n' "$(readlink "$root/current")" >>"$log"
  fi
fi
if [[ "$command_name" == is-active && "${FAKE_DRIFT_ON_ACTIVE:-0}" == 1 ]]; then
  active_checks=$(grep -c '^systemctl is-active ' "$log" || true)
  if [[ "$active_checks" -eq 1 ]]; then
    rm -f "$root/current"
    ln -s "$root/releases/drift" "$root/current"
    printf 'injected_current_drift_on_active=%s\n' "$(readlink "$root/current")" >>"$log"
  fi
fi
exit 0
SH
  cat >"$bin/ps" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf 'ps-call\n' >>"${FAKE_PS_LOG:?}"
printf '777 1 harmless-process\n'
SH
  cat >"$bin/curl" <<'SH'
#!/usr/bin/env bash
if [[ "${FAKE_DRIFT_ON_CURL:-0}" == 1 ]]; then
  root=${FAKE_NODE_ROOT:?}
  rm -f "$root/current"
  ln -s "$root/releases/drift" "$root/current"
fi
printf '{"ok":false}\n'
SH
  chmod +x "$bin/systemctl" "$bin/ps" "$bin/curl"
}

run_upgrade_failure() {
  local root=$1 log=$2 ps_log=$3 drift=${4:-0} drift_on_stop=${5:-0} drift_on_curl=${6:-0} drift_on_active=${7:-0}
  set +e
  FAKE_NODE_ROOT="$root" FAKE_SYSTEMCTL_LOG="$log" FAKE_PS_LOG="$ps_log" \
  FAKE_DRIFT_ON_START="$drift" FAKE_DRIFT_ON_STOP="$drift_on_stop" FAKE_DRIFT_ON_CURL="$drift_on_curl" FAKE_DRIFT_ON_ACTIVE="$drift_on_active" PATH="$TMP_DIR/fake-bin:$PATH" \
    "$ROOT_DIR/scripts/p2p-public-testnet-package-node-upgrade.sh" \
    --node-root "$root" --bundle-tar "$TMP_DIR/bundle/oasis7-linux-x64-bundle.tar.gz" \
    --package-version 0.0.0+rollback-contract \
    --commit abcdef1234567890abcdef1234567890abcdef12 \
    --run-id 3191-rollback-contract \
    --systemd-service oasis7-testnet-rollback-contract.service --restart-service \
    --post-restart-health-url http://127.0.0.1:6631/healthz --post-restart-timeout-secs 1 \
    >"$root/upgrade.out" 2>&1
  local rc=$?
  set -e
  return "$rc"
}

make_bundle "$TMP_DIR/bundle"
make_fake_bin "$TMP_DIR/fake-bin"
failures=0

# Post-start readiness failure must stop and prove quiescence before restoring.
quiescence_node="$TMP_DIR/quiescence-node"
make_node_fixture "$quiescence_node"
quiescence_log="$TMP_DIR/quiescence-systemctl.log"
quiescence_ps="$TMP_DIR/quiescence-ps.log"
run_upgrade_failure "$quiescence_node" "$quiescence_log" "$quiescence_ps" 0 || true
stop_count=$(grep -c '^systemctl stop ' "$quiescence_log" || true)
ps_count=$(wc -l <"$quiescence_ps" | tr -d ' ')
new_current="$quiescence_node/releases/0.0.0+rollback-contract"
second_stop_new=$(grep -c "^systemctl stop .*current=$new_current$" "$quiescence_log" || true)
if [[ "$stop_count" -lt 2 || "$ps_count" -lt 2 || "$second_stop_new" -lt 1 ]]; then
  printf '%s\n' 'rollback did not stop and prove quiescence before restoring the current snapshot' >&2
  printf 'stop_count=%s ps_count=%s second_stop_new=%s\n' "$stop_count" "$ps_count" "$second_stop_new" >&2
  cat "$quiescence_log" >&2
  cat "$quiescence_ps" >&2
  failures=1
fi

# If current drifts after a failed post-start probe, rollback must fail closed
# before remove_path instead of replacing the drift target with the snapshot.
drift_node="$TMP_DIR/drift-node"
make_node_fixture "$drift_node"
drift_log="$TMP_DIR/drift-systemctl.log"
run_upgrade_failure "$drift_node" "$drift_log" "$TMP_DIR/drift-ps.log" 0 0 0 1 || true
if ! grep -q 'injected_current_drift_on_active=' "$drift_log"; then
  printf '%s\n' 'fault injector did not reach post-start current drift boundary' >&2
  cat "$drift_log" >&2
  failures=1
elif [[ "$(readlink "$drift_node/current")" != "$drift_node/releases/drift" ]]; then
  printf '%s\n' 'rollback removed or replaced a drifted current target instead of failing closed' >&2
  cat "$drift_log" >&2
  failures=1
fi

# Static contract: durability, descriptor/schema, and regular-current metadata.
set +e
python3 - "$ROOT_DIR/scripts/p2p-public-testnet-package-node-upgrade.sh" <<'PY'
from pathlib import Path
import sys

text = Path(sys.argv[1]).read_text(encoding="utf-8")
def body(start: str, end: str) -> str:
    left = text.index(start)
    return text[left:text.index(end, left)]
journal = body("journal_transaction_phase() {", "create_transaction_snapshot() {")
snapshot = body("create_transaction_snapshot() {", "rollback_transaction() {")
rollback = body("rollback_transaction() {", "transaction_dir=\"\"")
current_file = body("elif current.is_file():", "else:\n    current_state = {\"kind\": \"absent\"}")
current_restore = body('elif kind == "file":', 'elif kind != "absent":')
assert "os.fsync" in journal and "os.replace" in journal
assert "os.fsync" in snapshot
assert '"uid"' in current_file and '"gid"' in current_file
assert "os.chown" in current_restore
remove_index = rollback.index("remove_path(current)")
assert "readlink" in rollback[:remove_index] or "resolve" in rollback[:remove_index], \
    "rollback must read back current identity before remove_path"
PY
static_rc=$?
set -e
if [[ "$static_rc" -ne 0 ]]; then
  printf '%s\n' 'rollback durability/stat/current-drift static contract missing on current implementation' >&2
  failures=1
fi

if [[ "$failures" -ne 0 ]]; then exit 1; fi
printf '%s\n' 'ok: package rollback quiescence, durability, drift, and metadata contracts'
