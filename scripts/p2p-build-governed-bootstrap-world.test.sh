#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

run() {
  echo "+ $*"
  "$@"
}

assert_file_exists() {
  local path=$1
  [[ -f "$path" ]] || {
    echo "error: expected file: $path" >&2
    exit 1
  }
}

assert_not_exists() {
  local path=$1
  [[ ! -e "$path" ]] || {
    echo "error: did not expect path: $path" >&2
    exit 1
  }
}

assert_eq() {
  local actual=$1
  local expected=$2
  [[ "$actual" == "$expected" ]] || {
    echo "error: expected '$expected' got '$actual'" >&2
    exit 1
  }
}

tmp_root=".tmp/p2p-build-governed-bootstrap-world-test"
rm -rf "$tmp_root"
mkdir -p "$tmp_root"

first_out="$tmp_root/first"
second_out="$tmp_root/second"

run ./scripts/p2p-build-governed-bootstrap-world.sh create \
  --genesis doc/testing/evidence/public-testnet-governed-bootstrap-genesis-2026-06-06.json \
  --out-dir "$first_out" \
  --allow-overwrite

run ./scripts/p2p-build-governed-bootstrap-world.sh validate \
  --world-dir "$first_out/world" \
  --merged-public-manifest "$first_out/merged-public-manifest-entries.json"

run ./scripts/p2p-build-governed-bootstrap-world.sh create \
  --genesis doc/testing/evidence/public-testnet-governed-bootstrap-genesis-2026-06-06.json \
  --out-dir "$second_out" \
  --allow-overwrite

run ./scripts/p2p-build-governed-bootstrap-world.sh validate \
  --world-dir "$second_out/world" \
  --merged-public-manifest "$second_out/merged-public-manifest-entries.json"

for path in \
  "$first_out/world/snapshot.json" \
  "$first_out/world/journal.json" \
  "$first_out/world/journal.segments.json" \
  "$first_out/world/snapshot.manifest.json" \
  "$first_out/world/module_registry.json" \
  "$first_out/merged-public-manifest-entries.json"; do
  assert_file_exists "$path"
done

assert_not_exists "$first_out/world/.distfs-state"
assert_not_exists "$first_out/world/distfs.recovery.audit.json"

updated_at=$(jq '.updated_at' "$first_out/world/module_registry.json")
assert_eq "$updated_at" "0"

first_tree=$(python3 - "$first_out/world" <<'PY'
import hashlib
import pathlib
import sys

root = pathlib.Path(sys.argv[1]).resolve()
combined = hashlib.sha256()
for child in sorted(p for p in root.rglob("*") if p.is_file()):
    rel = child.relative_to(root).as_posix()
    digest = hashlib.sha256(child.read_bytes()).hexdigest()
    stat = child.stat()
    combined.update(rel.encode("utf-8"))
    combined.update(b"\0")
    combined.update(digest.encode("ascii"))
    combined.update(b"\0")
    combined.update(str(stat.st_size).encode("ascii"))
    combined.update(b"\n")
print(combined.hexdigest())
PY
)

second_tree=$(python3 - "$second_out/world" <<'PY'
import hashlib
import pathlib
import sys

root = pathlib.Path(sys.argv[1]).resolve()
combined = hashlib.sha256()
for child in sorted(p for p in root.rglob("*") if p.is_file()):
    rel = child.relative_to(root).as_posix()
    digest = hashlib.sha256(child.read_bytes()).hexdigest()
    stat = child.stat()
    combined.update(rel.encode("utf-8"))
    combined.update(b"\0")
    combined.update(digest.encode("ascii"))
    combined.update(b"\0")
    combined.update(str(stat.st_size).encode("ascii"))
    combined.update(b"\n")
print(combined.hexdigest())
PY
)

assert_eq "$first_tree" "$second_tree"

cmp -s "$first_out/merged-public-manifest-entries.json" "$second_out/merged-public-manifest-entries.json" || {
  echo "error: merged manifests differ across rebuilds" >&2
  exit 1
}

echo "p2p-build-governed-bootstrap-world checks passed"
