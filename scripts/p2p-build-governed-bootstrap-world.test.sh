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

hash_tree() {
  python3 - "$1" <<'PY'
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
}

tmp_root=".tmp/p2p-build-governed-bootstrap-world-test"
rm -rf "$tmp_root"
mkdir -p "$tmp_root"

first_out="$tmp_root/first"
second_out="$tmp_root/second"
generated_out="$tmp_root/generated"
generated_second_out="$tmp_root/generated-second"

run ./scripts/p2p-build-governed-bootstrap-world.sh create \
  --genesis doc/testing/evidence/public-testnet-governed-bootstrap-genesis-2026-06-06.json \
  --out-dir "$generated_out" \
  --world-scenario asteroid_fragment_bootstrap \
  --allow-overwrite

run ./scripts/p2p-build-governed-bootstrap-world.sh validate \
  --world-dir "$generated_out/world" \
  --merged-public-manifest "$generated_out/merged-public-manifest-entries.json"

assert_file_exists "$generated_out/world-generation-provenance.json"
assert_file_exists "$generated_out/world/module_registry.json"
assert_file_exists "$generated_out/generated-scenario-world/snapshot.json"
assert_file_exists "$generated_out/generated-scenario-world/journal.json"

generated_module_registry_updated_at=$(jq '.updated_at' "$generated_out/world/module_registry.json")
assert_eq "$generated_module_registry_updated_at" "0"

generated_locations=$(jq '.model.locations | length' "$generated_out/generated-scenario-world/snapshot.json")
if [[ "$generated_locations" -le 0 ]]; then
  echo "error: generated world should include scenario-generated locations" >&2
  exit 1
fi

generated_chunk_events=$(jq '[.events[] | select(.kind.type == "ChunkGenerated")] | length' "$generated_out/generated-scenario-world/journal.json")
if [[ "$generated_chunk_events" -le 0 ]]; then
  echo "error: generated world should persist chunk generation events" >&2
  exit 1
fi

scenario_id=$(jq -r '.scenario_id' "$generated_out/world-generation-provenance.json")
assert_eq "$scenario_id" "asteroid_fragment_bootstrap"

provenance_seed=$(jq '.seed' "$generated_out/world-generation-provenance.json")
if [[ "$provenance_seed" -le 0 ]]; then
  echo "error: generated world provenance should record deterministic seed" >&2
  exit 1
fi

provenance_config_keys=$(jq '.config | keys | length' "$generated_out/world-generation-provenance.json")
if [[ "$provenance_config_keys" -le 0 ]]; then
  echo "error: generated world provenance should record scenario config" >&2
  exit 1
fi

expected_manifest_hash=$(python3 - "$generated_out/merged-public-manifest-entries.json" <<'PY'
import hashlib
import pathlib
import sys

print(hashlib.sha256(pathlib.Path(sys.argv[1]).read_bytes()).hexdigest())
PY
)
provenance_manifest_hash=$(jq -r '.public_manifest_sha256' "$generated_out/world-generation-provenance.json")
assert_eq "$provenance_manifest_hash" "$expected_manifest_hash"

manifest_entry_count=$(jq 'length' "$generated_out/merged-public-manifest-entries.json")
provenance_manifest_entry_count=$(jq '.public_manifest_entry_count' "$generated_out/world-generation-provenance.json")
assert_eq "$provenance_manifest_entry_count" "$manifest_entry_count"

run ./scripts/p2p-build-governed-bootstrap-world.sh create \
  --genesis doc/testing/evidence/public-testnet-governed-bootstrap-genesis-2026-06-06.json \
  --out-dir "$generated_second_out" \
  --world-scenario asteroid_fragment_bootstrap \
  --allow-overwrite

run ./scripts/p2p-build-governed-bootstrap-world.sh validate \
  --world-dir "$generated_second_out/world" \
  --merged-public-manifest "$generated_second_out/merged-public-manifest-entries.json"

generated_governed_first_tree=$(hash_tree "$generated_out/world")
generated_governed_second_tree=$(hash_tree "$generated_second_out/world")
assert_eq "$generated_governed_first_tree" "$generated_governed_second_tree"

generated_scenario_first_tree=$(hash_tree "$generated_out/generated-scenario-world")
generated_scenario_second_tree=$(hash_tree "$generated_second_out/generated-scenario-world")
assert_eq "$generated_scenario_first_tree" "$generated_scenario_second_tree"

cmp -s "$generated_out/world-generation-provenance.json" "$generated_second_out/world-generation-provenance.json" || {
  echo "error: generated world provenance differs across rebuilds" >&2
  exit 1
}

cmp -s "$generated_out/merged-public-manifest-entries.json" "$generated_second_out/merged-public-manifest-entries.json" || {
  echo "error: generated merged manifests differ across rebuilds" >&2
  exit 1
}

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

first_tree=$(hash_tree "$first_out/world")
second_tree=$(hash_tree "$second_out/world")

assert_eq "$first_tree" "$second_tree"

cmp -s "$first_out/merged-public-manifest-entries.json" "$second_out/merged-public-manifest-entries.json" || {
  echo "error: merged manifests differ across rebuilds" >&2
  exit 1
}

echo "p2p-build-governed-bootstrap-world checks passed"
