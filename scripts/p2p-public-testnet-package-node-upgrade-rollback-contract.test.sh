#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-package-upgrade-rollback-contract.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT
REAL_PYTHON3="$(command -v python3)"

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
  mkdir -p "$root/oasis7-linux-x64.deb.root/opt/oasis7/bin"
  printf 'runtime-new\n' >"$root/oasis7-linux-x64.deb.root/opt/oasis7/bin/oasis7_chain_runtime"
  chmod +x "$root/oasis7-linux-x64.deb.root/opt/oasis7/bin/oasis7_chain_runtime"
  printf 'deb-placeholder\n' >"$root/oasis7-linux-x64.deb"
  mkdir -p "$root/oasis7-linux-x64-ops-tools/bin"
  for binary in oasis7_world_repair_rebuild oasis7_governance_registry_import oasis7_governance_registry_audit; do
    printf '%s\n' "$binary" >"$root/oasis7-linux-x64-ops-tools/bin/$binary"
    chmod +x "$root/oasis7-linux-x64-ops-tools/bin/$binary"
  done
  printf '{"opsToolsSchemaVersion":1}\n' >"$root/oasis7-linux-x64-ops-tools/.oasis7-ops-tools-manifest.json"
  (
    cd "$root/oasis7-linux-x64-ops-tools"
    shasum -a 256 $(find . -type f ! -name SHA256SUMS -print | sort) >SHA256SUMS
  )
  tar -czf "$root/oasis7-linux-x64-ops-tools.tar.gz" -C "$root" oasis7-linux-x64-ops-tools
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
if [[ "${FAKE_HEALTH_OK:-0}" == 1 ]]; then
  printf '{"ok":true}\n'
else
  printf '{"ok":false}\n'
fi
SH
  cat >"$bin/dpkg-deb" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[[ "${1:-}" == --extract ]] || exit 2
source=${2:?missing source}
destination=${3:?missing destination}
mkdir -p "$destination"
cp -a "${source}.root/." "$destination/"
SH
  chmod +x "$bin/systemctl" "$bin/ps" "$bin/curl"
  chmod +x "$bin/dpkg-deb"
}

run_upgrade_failure() {
  local root=$1 log=$2 ps_log=$3
  local drift=${4:-0} drift_on_stop=${5:-0} drift_on_curl=${6:-0} drift_on_active=${7:-0} health_ok=${8:-0}
  local path_bin=${9:-$TMP_DIR/fake-bin}
  local node_arg=${10:-}
  node_arg=${node_arg:-$root}
  set +e
  FAKE_NODE_ROOT="$root" FAKE_SYSTEMCTL_LOG="$log" FAKE_PS_LOG="$ps_log" \
  FAKE_DRIFT_ON_START="$drift" FAKE_DRIFT_ON_STOP="$drift_on_stop" FAKE_DRIFT_ON_CURL="$drift_on_curl" FAKE_DRIFT_ON_ACTIVE="$drift_on_active" FAKE_HEALTH_OK="$health_ok" PATH="$path_bin:$TMP_DIR/fake-bin:$PATH" \
    "$ROOT_DIR/scripts/p2p-public-testnet-package-node-upgrade.sh" \
    --node-root "$node_arg" --package-deb "$TMP_DIR/bundle/oasis7-linux-x64.deb" \
    --ops-tools-tar "$TMP_DIR/bundle/oasis7-linux-x64-ops-tools.tar.gz" \
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

make_normalize_fsync_failure_bin() {
  local bin=$1
  mkdir -p "$bin"
  cat >"$bin/python3" <<SH
#!/usr/bin/env bash
set -euo pipefail
real_python3="$REAL_PYTHON3"
# normalize_promoted_current_link invokes python3 with the live current path
# as argv[1].  Fail its directory fsync after os.replace has exposed the
# canonical target, leaving only the earlier write-ahead promotion journal.
if [[ "\${1:-}" == "-" && "\${2:-}" == *current ]]; then
  printf '%s\n' 'injected fsync failure after canonical current exposure' >&2
  script=\$(mktemp "\${TMPDIR:-/tmp}/oasis7-fsync-failure.XXXXXX.py")
  {
    printf '%s\n' 'import os'
    printf '%s\n' 'def _fail_fsync(fd): raise OSError("injected fsync failure after current exposure")'
    printf '%s\n' 'os.fsync = _fail_fsync'
    cat
  } >"\$script"
  exec "\$real_python3" "\$script" "\${@:2}"
fi
exec "\$real_python3" "\$@"
SH
  chmod +x "$bin/python3"
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

# If the canonical current symlink becomes visible but its post-exposure
# journal fsync fails, rollback must use the earlier write-ahead promotion
# record (or otherwise recognize the exposed target) and restore the snapshot.
# The lexical node-root spelling deliberately differs from its canonical path
# so a rollback that compares only the old target spelling fails deterministically.
normalize_node="$TMP_DIR/normalize-node"
make_node_fixture "$normalize_node"
normalize_lexical_root="$TMP_DIR/normalize-alias/../normalize-node"
mkdir -p "$TMP_DIR/normalize-alias"
normalize_log="$TMP_DIR/normalize-systemctl.log"
normalize_ps="$TMP_DIR/normalize-ps.log"
make_normalize_fsync_failure_bin "$TMP_DIR/normalize-fsync-bin"
normalize_status=0
run_upgrade_failure "$normalize_node" "$normalize_log" "$normalize_ps" 0 0 0 0 1 "$TMP_DIR/normalize-fsync-bin" "$normalize_lexical_root" || normalize_status=$?
if [[ "$normalize_status" -eq 0 ]]; then
  printf '%s\n' 'upgrade unexpectedly succeeded despite canonical-current journal fsync fault' >&2
  failures=1
elif ! grep -q 'injected fsync failure after canonical current exposure' "$normalize_node/upgrade.out"; then
  printf '%s\n' 'fault injector did not reach canonical current exposure journal boundary' >&2
  cat "$normalize_node/upgrade.out" >&2
  failures=1
elif [[ ! -L "$normalize_node/current" || "$(readlink "$normalize_node/current")" != "$normalize_node/releases/old" ]]; then
  printf '%s\n' 'rollback did not restore the pre-upgrade current target after post-exposure journal failure' >&2
  readlink "$normalize_node/current" >&2 || true
  cat "$normalize_log" >&2
  failures=1
elif ! grep -q 'package_upgrade_rollback_complete=true' "$normalize_node/upgrade.out"; then
  printf '%s\n' 'post-exposure journal failure did not complete transactional rollback' >&2
  cat "$normalize_node/upgrade.out" >&2
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
