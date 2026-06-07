#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-state-sync-closure-test.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

mkdir -p "$TMP_DIR/world" "$TMP_DIR/execution-records" "$TMP_DIR/store/blobs"

cat >"$TMP_DIR/world/snapshot.json" <<'JSON'
{
  "state": {},
  "tick_consensus_records": []
}
JSON

cat >"$TMP_DIR/world/journal.json" <<'JSON'
[]
JSON

cat >"$TMP_DIR/execution-records/latest.json" <<'JSON'
{
  "snapshot_ref": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "journal_ref": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
  "external_effect_ref": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
}
JSON

printf 'blob-a\n' >"$TMP_DIR/store/blobs/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.blob"
printf 'blob-b\n' >"$TMP_DIR/store/blobs/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.blob"

if "$ROOT_DIR/scripts/p2p-verify-state-sync-closure.sh" \
  --world-dir "$TMP_DIR/world" \
  --execution-records-dir "$TMP_DIR/execution-records" \
  --store-dir "$TMP_DIR/store" \
  --out "$TMP_DIR/report.json"; then
  echo "expected missing blob failure" >&2
  exit 1
fi

jq -e '.ok == false and .missing_blob_count == 1' "$TMP_DIR/report.json" >/dev/null

printf 'blob-c\n' >"$TMP_DIR/store/blobs/cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc.blob"

"$ROOT_DIR/scripts/p2p-verify-state-sync-closure.sh" \
  --world-dir "$TMP_DIR/world" \
  --execution-records-dir "$TMP_DIR/execution-records" \
  --store-dir "$TMP_DIR/store" \
  --out "$TMP_DIR/report-ok.json"

jq -e '.ok == true and .missing_blob_count == 0' "$TMP_DIR/report-ok.json" >/dev/null
