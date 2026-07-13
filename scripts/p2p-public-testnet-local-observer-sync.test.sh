#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"
source "$repo_root/config/chain-pos-defaults.env"

test_case=${OASIS7_OBSERVER_SYNC_TEST_CASE:-all}
if [[ "$test_case" == "all" ]]; then
  for test_case in \
    canonical_layout \
    reset_owned_restore_retry \
    reset_owned_restore_retry_path_shim \
    corrupt_file_metadata \
    corrupt_tree_metadata \
    corrupt_tree_file_count \
    corrupt_tree_total_bytes \
    legacy_manifest \
    completed_backup_reuse_fails_closed; do
    OASIS7_OBSERVER_SYNC_TEST_CASE="$test_case" bash "$0"
  done
  echo "ok: local observer sync accepts sequencer/storage validator env pair"
  exit 0
fi
if [[ "$test_case" != "canonical_layout" \
  && "$test_case" != "reset_owned_restore_retry" \
  && "$test_case" != "reset_owned_restore_retry_path_shim" \
  && "$test_case" != "corrupt_file_metadata" \
  && "$test_case" != "corrupt_tree_metadata" \
  && "$test_case" != "corrupt_tree_file_count" \
  && "$test_case" != "corrupt_tree_total_bytes" \
  && "$test_case" != "legacy_manifest" \
  && "$test_case" != "completed_backup_reuse_fails_closed" ]]; then
  echo "unknown observer sync test case: $test_case" >&2
  exit 2
fi

tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/oasis7-local-observer-sync.XXXXXX")
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

if [[ "$test_case" == "reset_owned_restore_retry_path_shim" ]]; then
  stable_python3=$(python3 -c 'import os, sys; print(os.path.realpath(sys.executable))')
  shim_bin="$tmp_dir/path-sensitive-shim-bin"
  mkdir -p "$shim_bin"
  cat >"$shim_bin/python3" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${OASIS7_REAL_PYTHON3:-}" ]]; then
  exec python3 "$@"
fi
exec "${OASIS7_TEST_STABLE_PYTHON3:?}" "$@"
EOF
  chmod +x "$shim_bin/python3"

  "$stable_python3" - "$repo_root/scripts/p2p-public-testnet-local-observer-sync.test.sh" "$shim_bin" "$stable_python3" <<'PY'
import os
import signal
import subprocess
import sys

script_path, shim_bin, stable_python3 = sys.argv[1:4]
env = os.environ.copy()
env["PATH"] = os.pathsep.join((shim_bin, env["PATH"]))
env["OASIS7_OBSERVER_SYNC_TEST_CASE"] = "reset_owned_restore_retry"
env["OASIS7_TEST_STABLE_PYTHON3"] = stable_python3
process = subprocess.Popen(
    ["bash", script_path],
    env=env,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
    start_new_session=True,
)
try:
    stdout, stderr = process.communicate(timeout=5)
except subprocess.TimeoutExpired:
    os.killpg(process.pid, signal.SIGKILL)
    stdout, stderr = process.communicate()
    sys.stdout.write(stdout)
    sys.stderr.write(stderr)
    raise SystemExit("path-sensitive python3 shim caused restore wrapper recursion/hang")
if process.returncode != 0:
    sys.stdout.write(stdout)
    sys.stderr.write(stderr)
    raise SystemExit(
        f"path-sensitive python3 shim restore regression failed with exit {process.returncode}"
    )
PY
  echo "ok: observer reset case $test_case"
  exit 0
fi

local_stack="$tmp_dir/local-stack"
manifest_path="$local_stack/manifest.json"
bundle_path="$local_stack/governed-bundle.json"
mkdir -p "$local_stack"

if [[ "$test_case" == "legacy_manifest" \
  || "$test_case" == "completed_backup_reuse_fails_closed" ]]; then
  cat >"$tmp_dir/local.env" <<EOF
STACK_ROOT=$local_stack
NODE_ID=triad-testnet-local
EXECUTION_WORLD_DIR=\$STACK_ROOT/world
EXECUTION_RECORDS_DIR=\$STACK_ROOT/execution-records
STORAGE_ROOT=\$STACK_ROOT/store
RUNTIME_ROOT=\$STACK_ROOT/runtime-root
REPLICATION_ROOT=\$STACK_ROOT/replication-root
NETWORK_TIER_MANIFEST_PATH=$manifest_path
EOF
  cat >"$manifest_path" <<'EOF'
{
  "network_id": "public_testnet",
  "runtime_refs": {
    "release_candidate_bundle_ref": "legacy-bundle.json"
  }
}
EOF
  mkdir -p "$local_stack/world" "$local_stack/execution-records" "$local_stack/store"
  printf '{"height":1233}\n' >"$local_stack/world/snapshot.json"
  ./scripts/p2p-public-testnet-local-observer-sync.sh reset-state \
    --local-env "$tmp_dir/local.env" \
    --backup-dir "$tmp_dir/reset-backup"
  test -f "$tmp_dir/reset-backup/execution-world/snapshot.json"
  test ! -e "$local_stack/world"

  if [[ "$test_case" == "completed_backup_reuse_fails_closed" ]]; then
    mkdir -p "$local_stack/world"
    printf 'new live state\n' >"$local_stack/world/new-live-marker"
    reuse_stderr="$tmp_dir/reuse.stderr"
    reuse_failed=0
    if ./scripts/p2p-public-testnet-local-observer-sync.sh reset-state \
      --local-env "$tmp_dir/local.env" \
      --backup-dir "$tmp_dir/reset-backup" \
      >"$tmp_dir/reuse.stdout" 2>"$reuse_stderr"; then
      echo "expected completed backup reuse to fail closed" >&2
      reuse_failed=1
    fi
    if [[ ! -f "$local_stack/world/new-live-marker" ]]; then
      echo "completed backup reuse deleted newly created live state" >&2
      reuse_failed=1
    fi
    if [[ ! -f "$tmp_dir/reset-backup/execution-world/snapshot.json" ]]; then
      echo "completed backup reuse damaged the original backup" >&2
      reuse_failed=1
    fi
    if [[ "$reuse_failed" -ne 0 ]]; then
      exit 1
    fi
    grep -q 'refusing to overwrite existing backup' "$reuse_stderr"
  fi

  echo "ok: observer reset case $test_case"
  exit 0
fi

if [[ "$test_case" == "canonical_layout" ]]; then
  sidecar_ref="generated-world/generated-scenario-world"
  provenance_ref="generated-world/world-generation-provenance.json"
else
  sidecar_ref="world/generated-scenario-world"
  provenance_ref="world/world-generation-provenance.json"
fi
sidecar_path="$local_stack/$sidecar_ref"
provenance_path="$local_stack/$provenance_ref"

cat >"$tmp_dir/local.env" <<EOF
HOST_LABEL=local-a
SERVICE_NAME=triad-testnet-local
STACK_ROOT=$local_stack
NODE_ID=triad-testnet-local
WORLD_ID=oasis7-public-testnet-governed-20260606
NODE_ROLE=observer
STATUS_BIND=127.0.0.1:19082
NODE_GOSSIP_BIND=0.0.0.0:19385
NODE_AUTO_ATTEST_FLAG=--node-no-auto-attest-all
CONFIG_PATH=\$STACK_ROOT/config.toml
EXECUTION_WORLD_DIR=\$STACK_ROOT/world
EXECUTION_RECORDS_DIR=\$STACK_ROOT/execution-records
STORAGE_ROOT=\$STACK_ROOT/store
RUNTIME_ROOT=\$STACK_ROOT/runtime-root
REPLICATION_ROOT=\$STACK_ROOT/replication-root
REPLICATION_NETWORK_LISTEN_ADDRS_CSV=/ip4/0.0.0.0/tcp/19375
TRAFFIC_PROFILE=triad_low_traffic
TRAFFIC_MONITOR_ENABLE=0
TRAFFIC_MONITOR_INTERVAL_SECS=30
TRAFFIC_MONITOR_WINDOW_MINUTES=10
TRAFFIC_MONITOR_TOP_N=5
TRAFFIC_MONITOR_OUTPUT_DIR=\$STACK_ROOT/output/traffic-monitor
P2P_NODE_ROLE=observer_light
NETWORK_TIER_MANIFEST_PATH=$manifest_path
EOF

cat >"$tmp_dir/sequencer.env" <<EOF
WORLD_ID=oasis7-public-testnet-governed-20260606
NODE_ROLE=sequencer
STORAGE_PROFILE=dev_local
NODE_GOSSIP_PEERS_CSV=39.104.205.67:6732
REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV=/ip4/39.104.204.172/tcp/6831/p2p/12D3KooWMyPapumCaTABq27umWdHqXDr8AoTse21eMVnXeJEsbNp
REPLICATION_REMOTE_WRITERS_CSV=aa,bb
POS_SLOT_CLOCK_GENESIS_UNIX_MS=1779068751846
POS_ADAPTIVE_TICK_SCHEDULER=0
NODE_TICK_MS=200
POS_SLOT_DURATION_MS=${POS_SLOT_DURATION_MS}
POS_TICKS_PER_SLOT=${POS_TICKS_PER_SLOT}
POS_PROPOSAL_TICK_PHASE=${POS_PROPOSAL_TICK_PHASE}
POS_MAX_PAST_SLOT_LAG=${POS_MAX_PAST_SLOT_LAG}
REWARD_RUNTIME_ENABLE=1
REWARD_RUNTIME_EPOCH_DURATION_SECS=60
REWARD_POINTS_PER_CREDIT=100
REWARD_RUNTIME_AUTO_REDEEM=0
NODE_VALIDATORS_CSV=triad-testnet-sequencer:100,triad-testnet-storage:50
NODE_VALIDATOR_SIGNERS_CSV=triad-testnet-sequencer:65c27d,triad-testnet-storage:858e97
EOF

cat >"$tmp_dir/storage.env" <<EOF
WORLD_ID=oasis7-public-testnet-governed-20260606
NODE_ROLE=storage
STORAGE_PROFILE=dev_local
NODE_GOSSIP_PEERS_CSV=39.104.204.172:6731
REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV=/ip4/39.104.205.67/tcp/6832/p2p/12D3KooWAuNCCEDu7CdUUDwALuAhuLekZHgVWxAYp4Ag5ti79fJj
REPLICATION_REMOTE_WRITERS_CSV=bb,cc
POS_SLOT_CLOCK_GENESIS_UNIX_MS=1779068751846
POS_ADAPTIVE_TICK_SCHEDULER=0
NODE_TICK_MS=200
POS_SLOT_DURATION_MS=${POS_SLOT_DURATION_MS}
POS_TICKS_PER_SLOT=${POS_TICKS_PER_SLOT}
POS_PROPOSAL_TICK_PHASE=${POS_PROPOSAL_TICK_PHASE}
POS_MAX_PAST_SLOT_LAG=${POS_MAX_PAST_SLOT_LAG}
REWARD_RUNTIME_ENABLE=1
REWARD_RUNTIME_EPOCH_DURATION_SECS=60
REWARD_POINTS_PER_CREDIT=100
REWARD_RUNTIME_AUTO_REDEEM=0
NODE_VALIDATORS_CSV=triad-testnet-sequencer:100,triad-testnet-storage:50
NODE_VALIDATOR_SIGNERS_CSV=triad-testnet-sequencer:65c27d,triad-testnet-storage:858e97
EOF

cat >"$manifest_path" <<EOF
{
  "network_id": "public_testnet",
  "runtime_refs": {
    "release_candidate_bundle_ref": "governed-bundle.json",
    "generated_world_sidecar_ref": "$sidecar_ref",
    "world_generation_provenance_ref": "$provenance_ref"
  }
}
EOF

cat >"$bundle_path" <<EOF
{
  "generated_world_sidecar": {
    "kind": "directory",
    "ref": "$sidecar_ref",
    "resolved_path": "$sidecar_path"
  },
  "world_generation_provenance": {
    "kind": "file",
    "ref": "$provenance_ref",
    "resolved_path": "$provenance_path"
  }
}
EOF

rendered_env="$tmp_dir/rendered.env"
./scripts/p2p-public-testnet-local-observer-sync.sh render \
  --local-env "$tmp_dir/local.env" \
  --sequencer-env "$tmp_dir/sequencer.env" \
  --storage-env "$tmp_dir/storage.env" \
  --manifest-path "$manifest_path" \
  --out "$rendered_env"

grep -q '^WORLD_ID=oasis7-public-testnet-governed-20260606$' "$rendered_env"
grep -q '^NODE_ROLE=observer$' "$rendered_env"
grep -Eq '^RUNTIME_ROOT=.*/runtime-root$' "$rendered_env"
grep -Eq '^REPLICATION_ROOT=.*/replication-root$' "$rendered_env"
grep -q '^NODE_GOSSIP_PEERS_CSV=39.104.204.172:6731,39.104.205.67:6732$' "$rendered_env"
grep -q '^REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV=/ip4/39.104.205.67/tcp/6832/p2p/12D3KooWAuNCCEDu7CdUUDwALuAhuLekZHgVWxAYp4Ag5ti79fJj,/ip4/39.104.204.172/tcp/6831/p2p/12D3KooWMyPapumCaTABq27umWdHqXDr8AoTse21eMVnXeJEsbNp$' "$rendered_env"
grep -q '^REPLICATION_REMOTE_WRITERS_CSV=bb,cc,aa$' "$rendered_env"

grep -q 'remote_optional_resolved_env_value.*REPLICATION_ROOT' scripts/p2p-public-testnet-local-observer-sync.sh
grep -q 'remote_replication_root="$remote_stack_root/output/node-distfs/$remote_node_id"' scripts/p2p-public-testnet-local-observer-sync.sh
grep -q 'REMOTE_EXECUTION_BRIDGE_STATE_REQUIRED' scripts/p2p-public-testnet-local-observer-sync.sh
grep -q 'execution_bridge_state_path_for_root "$remote_stack_root" "$remote_node_id" "$remote_runtime_root"' scripts/p2p-public-testnet-local-observer-sync.sh
grep -q 'generated_world_sidecar_ref' scripts/p2p-public-testnet-local-observer-sync.sh
grep -q 'world_generation_provenance_ref' scripts/p2p-public-testnet-local-observer-sync.sh
grep -q 'target_ref = os.path.basename(os.path.normpath(ref)) if os.path.isabs(ref) else ref' scripts/p2p-public-testnet-local-observer-sync.sh
grep -q 'ref source and localized target must differ' scripts/p2p-public-testnet-local-observer-sync.sh
grep -q 'bundle\["generated_world_sidecar"\]\["resolved_path"\]' scripts/p2p-public-testnet-local-observer-sync.sh

mkdir -p \
  "$local_stack/world" \
  "$sidecar_path" \
  "$local_stack/world-simulator-mirror" \
  "$local_stack/execution-records" \
  "$local_stack/store/blobs" \
  "$local_stack/runtime-root" \
  "$local_stack/replication-root"
printf '{"height":1233}\n' >"$local_stack/world/snapshot.json"
printf '{"generated":"snapshot"}\n' >"$sidecar_path/snapshot.json"
printf '{"generated":"journal"}\n' >"$sidecar_path/journal.json"
printf '{"generated":"complete-tree-member"}\n' >"$sidecar_path/bootstrap-metadata.json"
printf '{"scenario_id":"asteroid_fragment_bootstrap"}\n' >"$provenance_path"
cp "$sidecar_path/snapshot.json" "$tmp_dir/expected-generated-snapshot.json"
cp "$sidecar_path/journal.json" "$tmp_dir/expected-generated-journal.json"
cp "$sidecar_path/bootstrap-metadata.json" "$tmp_dir/expected-bootstrap-metadata.json"
cp "$provenance_path" "$tmp_dir/expected-world-generation-provenance.json"
printf '{"mirror":"old"}\n' >"$local_stack/world-simulator-mirror/snapshot.json"
printf '{"height":1233}\n' >"$local_stack/execution-records/latest.json"
printf 'old blob\n' >"$local_stack/store/blobs/old"
printf '{"runtime":"old"}\n' >"$local_stack/runtime-root/reward-runtime-execution-bridge-state.json"
printf '{"committed_height":1233}\n' >"$local_stack/replication-root/node_pos_state.json"

python3 - "$bundle_path" "$sidecar_path" "$provenance_path" <<'PY'
import hashlib
import json
import pathlib
import sys

bundle_path, sidecar_path, provenance_path = map(pathlib.Path, sys.argv[1:4])

def file_sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

combined = hashlib.sha256()
file_count = 0
total_bytes = 0
for child in sorted(path for path in sidecar_path.rglob("*") if path.is_file()):
    relative = child.relative_to(sidecar_path).as_posix()
    digest = file_sha256(child)
    size = child.stat().st_size
    combined.update(relative.encode("utf-8"))
    combined.update(b"\0")
    combined.update(digest.encode("ascii"))
    combined.update(b"\0")
    combined.update(str(size).encode("ascii"))
    combined.update(b"\n")
    file_count += 1
    total_bytes += size

bundle = json.loads(bundle_path.read_text(encoding="utf-8"))
bundle["generated_world_sidecar"].update(
    {
        "sha256_tree": combined.hexdigest(),
        "file_count": file_count,
        "total_bytes": total_bytes,
    }
)
bundle["world_generation_provenance"].update(
    {
        "sha256": file_sha256(provenance_path),
        "size_bytes": provenance_path.stat().st_size,
    }
)
bundle_path.write_text(json.dumps(bundle, indent=2) + "\n", encoding="utf-8")
PY

if [[ "$test_case" == "corrupt_file_metadata" \
  || "$test_case" == "corrupt_tree_metadata" \
  || "$test_case" == "corrupt_tree_file_count" \
  || "$test_case" == "corrupt_tree_total_bytes" ]]; then
  original_sha256_tree=$(jq -r '.generated_world_sidecar.sha256_tree' "$bundle_path")
  if [[ "$test_case" == "corrupt_file_metadata" ]]; then
    jq '.world_generation_provenance.sha256 = ("0" * 64)' "$bundle_path" >"$bundle_path.tmp"
    expected_failure='world_generation_provenance sha256 drift'
  elif [[ "$test_case" == "corrupt_tree_metadata" ]]; then
    jq '.generated_world_sidecar.sha256_tree = ("0" * 64)' "$bundle_path" >"$bundle_path.tmp"
    expected_failure='generated_world_sidecar sha256_tree drift'
  elif [[ "$test_case" == "corrupt_tree_file_count" ]]; then
    jq '.generated_world_sidecar.file_count += 1' "$bundle_path" >"$bundle_path.tmp"
    expected_failure='generated_world_sidecar file_count drift'
  else
    jq '.generated_world_sidecar.total_bytes += 1' "$bundle_path" >"$bundle_path.tmp"
    expected_failure='generated_world_sidecar total_bytes drift'
  fi
  mv "$bundle_path.tmp" "$bundle_path"
  if [[ "$test_case" == "corrupt_tree_file_count" \
    || "$test_case" == "corrupt_tree_total_bytes" ]]; then
    jq -e \
      --arg expected "$original_sha256_tree" \
      '.generated_world_sidecar.sha256_tree == $expected' \
      "$bundle_path" >/dev/null
  fi
  reset_stderr="$tmp_dir/reset.stderr"
  if ./scripts/p2p-public-testnet-local-observer-sync.sh reset-state \
    --local-env "$tmp_dir/local.env" \
    --backup-dir "$tmp_dir/reset-backup" \
    >"$tmp_dir/reset.stdout" 2>"$reset_stderr"; then
    echo "expected corrupt governed artifact metadata to reject reset" >&2
    exit 1
  fi
  grep -q "$expected_failure" "$reset_stderr"
  test -f "$local_stack/world/snapshot.json"
  test ! -e "$tmp_dir/reset-backup/execution-world"
  echo "ok: observer reset case $test_case"
  exit 0
fi

jq -e \
  --arg sidecar "$sidecar_path" \
  --arg provenance "$provenance_path" \
  '.generated_world_sidecar.resolved_path == $sidecar
    and .world_generation_provenance.resolved_path == $provenance' \
  "$bundle_path" >/dev/null
test -f "$sidecar_path/snapshot.json"
test -f "$sidecar_path/journal.json"
test -f "$provenance_path"

reset_backup="$tmp_dir/reset-backup"

if [[ "$test_case" == "reset_owned_restore_retry" ]]; then
  real_python3=$(python3 -c 'import os, sys; print(os.path.realpath(sys.executable))')
  fake_bin="$tmp_dir/fake-bin"
  restore_failure_marker="$tmp_dir/restore-copy-failed-once"
  mkdir -p "$fake_bin"
  cat >"$fake_bin/python3" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

if [[ "${OASIS7_TEST_FAIL_RESTORE_COPY_ONCE:-0}" == "1" \
  && "${1:-}" == "-" \
  && "${2:-}" == "restore" \
  && ! -e "${OASIS7_TEST_RESTORE_FAILURE_MARKER:?}" ]]; then
  touch "$OASIS7_TEST_RESTORE_FAILURE_MARKER"
  exec "${OASIS7_REAL_PYTHON3:?}" -c '
import shutil
import sys

source = sys.stdin.read()
original_copy2 = shutil.copy2
copy_count = 0

def fail_second_copy(src, dst, *args, **kwargs):
    global copy_count
    copy_count += 1
    if copy_count == 2:
        raise OSError("forced governed bootstrap restore copy failure")
    return original_copy2(src, dst, *args, **kwargs)

shutil.copy2 = fail_second_copy
sys.argv = sys.argv[1:]
exec(compile(source, "<observer-sync-restore>", "exec"))
' "$@"
fi

exec "${OASIS7_REAL_PYTHON3:?}" "$@"
EOF
  chmod +x "$fake_bin/python3"

  first_reset_stdout="$tmp_dir/first-reset.stdout"
  first_reset_stderr="$tmp_dir/first-reset.stderr"
  set +e
  PATH="$fake_bin:$PATH" \
    OASIS7_REAL_PYTHON3="$real_python3" \
    OASIS7_TEST_FAIL_RESTORE_COPY_ONCE=1 \
    OASIS7_TEST_RESTORE_FAILURE_MARKER="$restore_failure_marker" \
    ./scripts/p2p-public-testnet-local-observer-sync.sh reset-state \
      --local-env "$tmp_dir/local.env" \
      --backup-dir "$reset_backup" \
      >"$first_reset_stdout" 2>"$first_reset_stderr"
  first_reset_rc=$?
  set -e

  if [[ "$first_reset_rc" -eq 0 ]]; then
    echo "expected forced governed bootstrap restore copy failure" >&2
    exit 1
  fi
  if ! grep -q 'forced governed bootstrap restore copy failure' "$first_reset_stderr"; then
    echo "expected injected restore-copy failure signature" >&2
    cat "$first_reset_stderr" >&2
    exit 1
  fi
  test -f "$reset_backup/execution-world/generated-scenario-world/snapshot.json"
  test -f "$reset_backup/execution-world/generated-scenario-world/journal.json"
  test -f "$reset_backup/execution-world/world-generation-provenance.json"

  retry_contract_failed=0
  if [[ -e "$sidecar_path" || -e "$provenance_path" ]]; then
    echo "expected failed governed bootstrap restore to leave no partial live target" >&2
    retry_contract_failed=1
  fi

  retry_stdout="$tmp_dir/retry.stdout"
  retry_stderr="$tmp_dir/retry.stderr"
  set +e
  PATH="$fake_bin:$PATH" \
    OASIS7_REAL_PYTHON3="$real_python3" \
    OASIS7_TEST_FAIL_RESTORE_COPY_ONCE=1 \
    OASIS7_TEST_RESTORE_FAILURE_MARKER="$restore_failure_marker" \
    ./scripts/p2p-public-testnet-local-observer-sync.sh reset-state \
      --local-env "$tmp_dir/local.env" \
      --backup-dir "$reset_backup" \
      >"$retry_stdout" 2>"$retry_stderr"
  retry_rc=$?
  set -e
  if [[ "$retry_rc" -ne 0 ]]; then
    echo "expected same-command retry to restore from complete retained backup" >&2
    cat "$retry_stderr" >&2
    retry_contract_failed=1
  fi
  if [[ "$retry_contract_failed" -ne 0 ]]; then
    exit 1
  fi
else
  ./scripts/p2p-public-testnet-local-observer-sync.sh reset-state \
    --local-env "$tmp_dir/local.env" \
    --backup-dir "$reset_backup"
fi

if [[ "$test_case" == "canonical_layout" ]]; then
  test ! -e "$reset_backup/execution-world/generated-scenario-world"
  test ! -e "$reset_backup/execution-world/world-generation-provenance.json"
else
  test -f "$reset_backup/execution-world/generated-scenario-world/snapshot.json"
  test -f "$reset_backup/execution-world/generated-scenario-world/journal.json"
  test -f "$reset_backup/execution-world/world-generation-provenance.json"
fi

test -f "$reset_backup/execution-world/snapshot.json"
test -f "$reset_backup/execution-world-simulator-mirror/snapshot.json"
test -f "$reset_backup/execution-records/latest.json"
test -f "$reset_backup/storage/blobs/old"
test -f "$reset_backup/runtime-root/reward-runtime-execution-bridge-state.json"
test -f "$reset_backup/replication-root/node_pos_state.json"
test ! -e "$local_stack/world/snapshot.json"
test ! -e "$local_stack/world-simulator-mirror/snapshot.json"
test ! -e "$local_stack/execution-records/latest.json"
test ! -e "$local_stack/store/blobs/old"
test ! -e "$local_stack/runtime-root/reward-runtime-execution-bridge-state.json"
test ! -e "$local_stack/replication-root/node_pos_state.json"

if [[ ! -d "$sidecar_path" ]]; then
  echo "expected reset-state to restore governed generated_world_sidecar directory" >&2
  exit 1
fi
for required_sidecar_file in snapshot.json journal.json; do
  if [[ ! -f "$sidecar_path/$required_sidecar_file" ]]; then
    echo "expected reset-state to restore governed generated_world_sidecar/$required_sidecar_file" >&2
    exit 1
  fi
done
if [[ ! -f "$sidecar_path/bootstrap-metadata.json" ]]; then
  echo "expected reset-state to restore complete governed generated_world_sidecar tree" >&2
  exit 1
fi
if [[ ! -f "$provenance_path" ]]; then
  echo "expected reset-state to restore governed world_generation_provenance" >&2
  exit 1
fi
if [[ "$test_case" == "reset_owned_restore_retry" ]]; then
  cmp -s \
    "$reset_backup/execution-world/generated-scenario-world/snapshot.json" \
    "$sidecar_path/snapshot.json"
  cmp -s \
    "$reset_backup/execution-world/generated-scenario-world/journal.json" \
    "$sidecar_path/journal.json"
  cmp -s \
    "$reset_backup/execution-world/generated-scenario-world/bootstrap-metadata.json" \
    "$sidecar_path/bootstrap-metadata.json"
  cmp -s \
    "$reset_backup/execution-world/world-generation-provenance.json" \
    "$provenance_path"
else
  cmp -s "$tmp_dir/expected-generated-snapshot.json" "$sidecar_path/snapshot.json"
  cmp -s "$tmp_dir/expected-generated-journal.json" "$sidecar_path/journal.json"
  cmp -s "$tmp_dir/expected-bootstrap-metadata.json" "$sidecar_path/bootstrap-metadata.json"
  cmp -s "$tmp_dir/expected-world-generation-provenance.json" "$provenance_path"
fi
jq -e \
  --arg sidecar "$sidecar_path" \
  --arg provenance "$provenance_path" \
  '.generated_world_sidecar.resolved_path == $sidecar
    and .world_generation_provenance.resolved_path == $provenance' \
  "$bundle_path" >/dev/null

echo "ok: observer reset case $test_case"
