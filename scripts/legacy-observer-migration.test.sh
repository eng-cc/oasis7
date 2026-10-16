#!/usr/bin/env bash
set -euo pipefail
# No Cargo invocation. The parent supplies a normal runtime and a real-driver
# unit-test fixture exporter compiled from the same source revision.
legacy_migration_bin=${OASIS7_CHAIN_RUNTIME_BIN:?normal runtime binary required}
legacy_migration_fixture_bin=${OASIS7_NODE_LIFECYCLE_TEST_BIN:?runtime unit-test executable required}
legacy_migration_tmp=$(mktemp -d "${TMPDIR:-/tmp}/oasis7-legacy-migration.XXXXXX")
legacy_migration_tmp=$(cd "$legacy_migration_tmp" && pwd -P)
legacy_migration_apply_pid=''
cleanup() {
  local outcome=$?
  if [[ -n "$legacy_migration_apply_pid" ]]; then
    kill -KILL "$legacy_migration_apply_pid" 2>/dev/null || true
    wait "$legacy_migration_apply_pid" 2>/dev/null || true
  fi
  if (( outcome == 0 )); then rm -rf "$legacy_migration_tmp";
  else echo "preserved legacy migration failure evidence: $legacy_migration_tmp" >&2; fi
}
trap cleanup EXIT
env OASIS7_LEGACY_MIGRATION_FIXTURE_OUT="$legacy_migration_tmp" OASIS7_LEGACY_MIGRATION_RUNTIME_BIN="$legacy_migration_bin" "$legacy_migration_fixture_bin" --exact node_lifecycle::legacy_migration::tests::legacy_export_real_source_fixture --nocapture >"$legacy_migration_tmp/fixture.log" 2>&1
# A large opaque reward-prefixed file makes the interrupted normal-process
# copy observable. It remains part of the exact inventory, even when disabled.
dd if=/dev/zero of="$legacy_migration_tmp/old-runtime/reward-runtime-migration-extra.bin" bs=1048576 count=128 2>"$legacy_migration_tmp/dd.log"
"$legacy_migration_bin" legacy-migrate plan --source "$legacy_migration_tmp/source.json" --destination "$legacy_migration_tmp/migrated" --operation-id cli-resume >"$legacy_migration_tmp/plan.json"
legacy_migration_stage=$(python3 - "$legacy_migration_tmp/plan.json" "$legacy_migration_tmp" <<'PYTHON'
import json,pathlib,sys
plan=json.load(open(sys.argv[1]));root=pathlib.Path(sys.argv[2]);stage=pathlib.Path(plan['stage'])
assert stage.is_absolute() and stage.parent==root and stage.name=='.oasis7-legacy-migrate-cli-resume'
print(stage)
PYTHON
)
"$legacy_migration_bin" legacy-migrate apply --source "$legacy_migration_tmp/source.json" --destination "$legacy_migration_tmp/migrated" --operation-id cli-resume >"$legacy_migration_tmp/first.json" 2>"$legacy_migration_tmp/first.stderr" &
legacy_migration_apply_pid=$!
for ((legacy_migration_attempt=0;legacy_migration_attempt<1000;legacy_migration_attempt++)); do
  if [[ -f "$legacy_migration_stage/migration-progress.json" && ! -e "$legacy_migration_tmp/migrated" ]]; then break; fi
  sleep 0.01
done
[[ -f "$legacy_migration_stage/migration-progress.json" && ! -e "$legacy_migration_tmp/migrated" ]]
# This is the exact still-owned, unreaped migration child, never a stored PID.
kill -KILL "$legacy_migration_apply_pid"
wait "$legacy_migration_apply_pid" 2>/dev/null || true
legacy_migration_apply_pid=''
"$legacy_migration_bin" legacy-migrate resume --source "$legacy_migration_tmp/source.json" --destination "$legacy_migration_tmp/migrated" --operation-id cli-resume >"$legacy_migration_tmp/resumed.json"
"$legacy_migration_bin" legacy-migrate status --source "$legacy_migration_tmp/source.json" --destination "$legacy_migration_tmp/migrated" --operation-id cli-resume >"$legacy_migration_tmp/status.json"
python3 - "$legacy_migration_tmp" <<'PYTHON'
import hashlib,json,pathlib,sys
root=pathlib.Path(sys.argv[1]);plan=json.load(open(root/'plan.json'));receipt=json.load(open(root/'resumed.json'));status=json.load(open(root/'status.json'))
assert receipt==status and receipt['source_preserved'] and not receipt['switched_source_launch']
assert receipt['closure']==plan['closure'];assert receipt['closure']['committed_height']==1
assert receipt['closure']['pending_actions']>0 and receipt['closure']['feedback_outbox_count']==1
assert receipt['closure']['replication_cas_blobs']==1
assert plan['reward_signer_id']=='old-reward-signer' and plan['reward_signer_id']!=plan['node_id']
launch=json.load(open(root/'migrated/legacy-launch.json'));assert launch['peer_id']==plan['peer_id'] and launch['public_key']==plan['public_key']
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
for entry in plan['files']:
    source=pathlib.Path(entry['source']);target=root/'migrated'/entry['target'];assert source.exists() and target.exists()
    if not entry['directory']:assert sha(source)==sha(target)==entry['sha256']
reward=next(e for e in plan['files'] if e['source'].endswith('/reward-runtime-migration-extra.bin'));assert reward['bytes']==128*1024*1024
assert not pathlib.Path(plan['stage']).exists()
PYTHON
echo 'ok: normal legacy observer migration, interrupted exact-stage resume, original head/key/peer/reward signer/pending/outbox/independent CAS preserved'
