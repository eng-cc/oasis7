#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HELPER="$ROOT_DIR/scripts/p2p-public-testnet-governed-node-env-transaction.sh"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-node-env-transaction.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

# This is the repository-owned contract for the implementation slice. The
# helper is intentionally absent on the RED head so the failure is deterministic.
if [[ ! -x "$HELPER" ]]; then
  printf 'missing governed node.env transaction helper: %s\n' "$HELPER" >&2
  exit 1
fi

file_metadata() {
  python3 - "$1" <<'PY'
import stat
import sys
from pathlib import Path

path = Path(sys.argv[1])
metadata = path.stat()
print(metadata.st_uid, metadata.st_gid, stat.S_IMODE(metadata.st_mode))
PY
}

run_replace() {
  "$HELPER" --env-file "$1" --service-name "$2" --journal "$3"
}

expect_rejected() {
  local label=$1
  local candidate=$2
  shift 2
  local before="$TMP_DIR/$label.before"
  local journal="$TMP_DIR/$label.journal.json"
  cp "$candidate" "$before" 2>/dev/null || true
  set +e
  "$@" >"$TMP_DIR/$label.out" 2>&1
  local status=$?
  set -e
  test "$status" -ne 0
  if [[ -e "$candidate" && -e "$before" ]]; then
    cmp -s "$before" "$candidate"
  fi
  test ! -e "$journal"
}

valid_env="$TMP_DIR/valid-node.env"
cat >"$valid_env" <<'EOF'
# governed validator environment
NODE_ID=triad-testnet-storage
SERVICE_NAME=oasis7-triad-sequencer.service
STACK_ROOT=/opt/oasis7/p2p-testnet
EOF
cp "$valid_env" "$TMP_DIR/valid-node.env.before"
read -r before_uid before_gid before_mode <<<"$(file_metadata "$valid_env")"
valid_journal="$TMP_DIR/valid.journal.json"
run_replace "$valid_env" oasis7-triad-storage.service "$valid_journal"

# Exactly one assignment changes; comments, ordering, and trailing newline remain stable.
test "$(grep -c '^SERVICE_NAME=' "$valid_env")" = 1
grep -Fx '# governed validator environment' "$valid_env"
grep -Fx 'NODE_ID=triad-testnet-storage' "$valid_env"
grep -Fx 'SERVICE_NAME=oasis7-triad-storage.service' "$valid_env"
grep -Fx 'STACK_ROOT=/opt/oasis7/p2p-testnet' "$valid_env"
test "$(file_metadata "$valid_env")" = "$before_uid $before_gid $before_mode"
test -z "$(find "$TMP_DIR" -maxdepth 1 -name '.valid-node.env.*.tmp' -print -quit)"
python3 - "$TMP_DIR/valid-node.env.before" "$valid_env" <<'PY'
from pathlib import Path
import sys

before = Path(sys.argv[1]).read_bytes().splitlines(keepends=True)
after = Path(sys.argv[2]).read_bytes().splitlines(keepends=True)
changed = [index for index, pair in enumerate(zip(before, after)) if pair[0] != pair[1]]
assert len(before) == len(after), (len(before), len(after))
assert changed == [2], changed
assert before[2] == b"SERVICE_NAME=oasis7-triad-sequencer.service\n"
assert after[2] == b"SERVICE_NAME=oasis7-triad-storage.service\n"
PY

# The journal is the durable transaction authority, not an operator claim.
jq -e '
  .schema_version == "oasis7.node_env_transaction.v2"
  and .phase == "committed"
  and (.env_path | type == "string")
  and (.service_name | type == "string")
  and (.before.sha256 | test("^[0-9a-f]{64}$"))
  and (.after.sha256 | test("^[0-9a-f]{64}$"))
  and (.before.stat.uid | type == "number")
  and (.before.stat.gid | type == "number")
  and (.before.stat.mode | type == "number")
  and (.after.stat.uid | type == "number")
  and (.after.stat.gid | type == "number")
  and (.after.stat.mode | type == "number")
' "$valid_journal" >/dev/null

# Rollback restores exact original bytes and metadata from the journal authority.
"$HELPER" --rollback --journal "$valid_journal"
cmp -s "$TMP_DIR/valid-node.env.before" "$valid_env"
test "$(file_metadata "$valid_env")" = "$before_uid $before_gid $before_mode"
jq -e '.phase == "rolled_back" and (.rollback.sha256 | test("^[0-9a-f]{64}$"))' \
  "$valid_journal" >/dev/null

missing_env="$TMP_DIR/missing-node.env"
expect_rejected missing "$missing_env" \
  run_replace "$missing_env" oasis7-triad-storage.service "$TMP_DIR/missing.journal.json"

symlink_target="$TMP_DIR/symlink-target.env"
cp "$TMP_DIR/valid-node.env.before" "$symlink_target"
symlink_env="$TMP_DIR/symlink-node.env"
ln -s "$symlink_target" "$symlink_env"
expect_rejected symlink "$symlink_env" \
  run_replace "$symlink_env" oasis7-triad-storage.service "$TMP_DIR/symlink.journal.json"

duplicate_env="$TMP_DIR/duplicate-node.env"
printf 'SERVICE_NAME=one.service\nSERVICE_NAME=two.service\n' >"$duplicate_env"
expect_rejected duplicate "$duplicate_env" \
  run_replace "$duplicate_env" oasis7-triad-storage.service "$TMP_DIR/duplicate.journal.json"

non_utf8_env="$TMP_DIR/non-utf8-node.env"
printf 'SERVICE_NAME=old.service\nBROKEN=\377\n' >"$non_utf8_env"
expect_rejected non-utf8 "$non_utf8_env" \
  run_replace "$non_utf8_env" oasis7-triad-storage.service "$TMP_DIR/non-utf8.journal.json"

# A relative env-file must be journaled as an absolute identity.  Rollback is
# intentionally invoked from a different cwd to prove it does not resolve the
# recorded path against an operator's current directory.
relative_root="$TMP_DIR/relative"
relative_work="$relative_root/work"
relative_other="$relative_root/other"
mkdir -p "$relative_work" "$relative_other"
relative_env="$relative_work/node.env"
relative_journal="$relative_work/node.journal.json"
cat >"$relative_env" <<'EOF'
NODE_ID=relative-test
SERVICE_NAME=old.service
EOF
cp "$relative_env" "$relative_root/relative.before"
(
  cd "$relative_work"
  "$HELPER" --env-file node.env --service-name new.service --journal node.journal.json >/dev/null
)
expected_relative_env="$(python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).resolve())' "$relative_env")"
jq -e --arg expected "$expected_relative_env" '.env_path == $expected and (.env_path | startswith("/"))' \
  "$relative_journal" >/dev/null || {
    printf '%s\n' 'journal did not persist absolute env path for relative --env-file' >&2
    jq '.env_path' "$relative_journal" >&2
    exit 1
  }
(
  cd "$relative_other"
  "$HELPER" --rollback --journal "$relative_journal" >/dev/null
)
cmp -s "$relative_root/relative.before" "$relative_env"

printf '%s\n' 'ok: governed node.env transaction contract'
