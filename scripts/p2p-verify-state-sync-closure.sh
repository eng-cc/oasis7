#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/p2p-verify-state-sync-closure.sh \
    --world-dir <dir> \
    --execution-records-dir <dir> \
    --store-dir <dir> \
    [--out <path>]

Description:
  Verify that a world + execution-records snapshot has the blob closure needed
  to restore execution state from store/blobs. The script scans world json and
  execution-record json files for referenced blob ids and reports any missing
  blobs under <store-dir>/blobs.
EOF
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

WORLD_DIR=""
EXECUTION_RECORDS_DIR=""
STORE_DIR=""
OUT_PATH="-"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --world-dir)
      WORLD_DIR=${2:-}
      shift 2
      ;;
    --execution-records-dir)
      EXECUTION_RECORDS_DIR=${2:-}
      shift 2
      ;;
    --store-dir)
      STORE_DIR=${2:-}
      shift 2
      ;;
    --out)
      OUT_PATH=${2:-}
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

[[ -d "$WORLD_DIR" ]] || die "--world-dir not found: $WORLD_DIR"
[[ -d "$EXECUTION_RECORDS_DIR" ]] || die "--execution-records-dir not found: $EXECUTION_RECORDS_DIR"
[[ -d "$STORE_DIR" ]] || die "--store-dir not found: $STORE_DIR"
[[ -d "$STORE_DIR/blobs" ]] || die "store blob directory not found: $STORE_DIR/blobs"

tmp_report=$(mktemp "${TMPDIR:-/tmp}/oasis7-state-sync-closure.XXXXXX")
trap 'rm -f "$tmp_report"' EXIT

python3 - "$WORLD_DIR" "$EXECUTION_RECORDS_DIR" "$STORE_DIR" >"$tmp_report" <<'PY'
import json
import pathlib
import re
import sys

world_dir = pathlib.Path(sys.argv[1])
execution_records_dir = pathlib.Path(sys.argv[2])
store_dir = pathlib.Path(sys.argv[3])
blob_dir = store_dir / "blobs"

HEX_64 = re.compile(r"^[0-9a-f]{64}$")
interesting_keys = {
    "latest_state_ref",
    "snapshot_ref",
    "journal_ref",
    "external_effect_ref",
    "content_hash",
}

refs = {}

def visit(value, source, path="root", key_name=None):
    if isinstance(value, dict):
        for key, child in value.items():
            visit(child, source, f"{path}.{key}", key)
    elif isinstance(value, list):
        for idx, child in enumerate(value):
            visit(child, source, f"{path}[{idx}]", key_name)
    elif isinstance(value, str):
        if key_name in interesting_keys and HEX_64.match(value):
            refs.setdefault(value, []).append({"source": source, "path": path})

files = []
for name in ("snapshot.json", "journal.json", "snapshot.manifest.json", "journal.segments.json", "module_registry.json"):
    path = world_dir / name
    if path.exists():
        files.append(path)
for path in sorted(execution_records_dir.glob("*.json")):
    files.append(path)

for path in files:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise SystemExit(f"failed to parse json: {path}: {exc}")
    visit(payload, str(path))

missing = []
present = []
for ref, locations in sorted(refs.items()):
    blob_path = blob_dir / f"{ref}.blob"
    entry = {
        "ref": ref,
        "blob_path": str(blob_path),
        "sources": locations,
    }
    if blob_path.exists():
        present.append(entry)
    else:
        missing.append(entry)

report = {
    "ok": len(missing) == 0,
    "world_dir": str(world_dir),
    "execution_records_dir": str(execution_records_dir),
    "store_dir": str(store_dir),
    "referenced_blob_count": len(refs),
    "present_blob_count": len(present),
    "missing_blob_count": len(missing),
    "missing": missing,
}
print(json.dumps(report, indent=2))
PY

if [[ "$OUT_PATH" == "-" ]]; then
  cat "$tmp_report"
else
  mkdir -p "$(dirname "$OUT_PATH")"
  cp "$tmp_report" "$OUT_PATH"
fi

if [[ "$(jq -r '.ok' "$tmp_report")" != "true" ]]; then
  jq -r '.missing[] | "missing ref=\(.ref) blob_path=\(.blob_path)"' "$tmp_report" >&2 || true
  exit 1
fi
