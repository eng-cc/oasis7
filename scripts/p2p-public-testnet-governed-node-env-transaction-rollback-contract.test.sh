#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HELPER="$ROOT_DIR/scripts/p2p-public-testnet-governed-node-env-transaction.sh"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-node-env-rollback-contract.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

make_transaction() {
  local stem=$1
  local env="$TMP_DIR/$stem.env"
  local journal="$TMP_DIR/$stem.journal.json"
  cat >"$env" <<'EOF'
NODE_ID=triad-testnet-storage
SERVICE_NAME=oasis7-triad-sequencer.service
STACK_ROOT=/opt/oasis7/p2p-testnet
EOF
  "$HELPER" --env-file "$env" --service-name oasis7-triad-storage.service --journal "$journal" >/dev/null
  printf '%s %s\n' "$env" "$journal"
}

failures=0

# Rollback must recompute both descriptor hashes and reject a journal whose
# before hash is inconsistent with its embedded original bytes.
read -r hash_env hash_journal <<<"$(make_transaction hash)"
jq '.before.sha256 = ("0" * 64)' "$hash_journal" >"$TMP_DIR/hash-tampered.json"
mv "$TMP_DIR/hash-tampered.json" "$hash_journal"
set +e
"$HELPER" --rollback --journal "$hash_journal" >"$TMP_DIR/hash.out" 2>&1
hash_rc=$?
set -e
if [[ "$hash_rc" -eq 0 ]]; then
  printf '%s\n' 'rollback accepted a journal with a tampered before.sha256 descriptor' >&2
  cat "$TMP_DIR/hash.out" >&2
  failures=1
fi

# Rollback must validate stat schema/values against the original bytes and
# reject a journal that claims a different mode before attempting replacement.
read -r stat_env stat_journal <<<"$(make_transaction stat)"
jq '.before.stat.mode = 0' "$stat_journal" >"$TMP_DIR/stat-tampered.json"
mv "$TMP_DIR/stat-tampered.json" "$stat_journal"
set +e
"$HELPER" --rollback --journal "$stat_journal" >"$TMP_DIR/stat.out" 2>&1
stat_rc=$?
set -e
if [[ "$stat_rc" -eq 0 ]]; then
  printf '%s\n' 'rollback accepted a journal with a tampered before.stat.mode descriptor' >&2
  cat "$TMP_DIR/stat.out" >&2
  failures=1
fi

# The after descriptor is also authoritative: tampering its hash must not
# allow rollback to remove or replace the current file.
read -r after_env after_journal <<<"$(make_transaction after)"
jq '.after.sha256 = ("f" * 64)' "$after_journal" >"$TMP_DIR/after-tampered.json"
mv "$TMP_DIR/after-tampered.json" "$after_journal"
cp "$after_env" "$TMP_DIR/after.before"
set +e
"$HELPER" --rollback --journal "$after_journal" >"$TMP_DIR/after.out" 2>&1
after_rc=$?
set -e
if [[ "$after_rc" -eq 0 ]]; then
  printf '%s\n' 'rollback accepted a journal with a tampered after.sha256 descriptor' >&2
  cat "$TMP_DIR/after.out" >&2
  failures=1
elif ! cmp -s "$TMP_DIR/after.before" "$after_env"; then
  printf '%s\n' 'rollback mutated env bytes after rejecting a tampered after.sha256 descriptor' >&2
  failures=1
fi

if [[ "$failures" -ne 0 ]]; then
  exit 1
fi
printf '%s\n' 'ok: node.env rollback recomputes descriptor hashes and stat schema'
