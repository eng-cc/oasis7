#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-local-node-install-test.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

source_stack="$TMP_DIR/source-stack"
node_root="$TMP_DIR/dedicated-node"
node_root_abs=$(python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).resolve())' "$node_root")
mkdir -p "$source_stack/config"
mkdir -p "$source_stack/generated-world/generated-scenario-world"

build_worktree="$TMP_DIR/build-worktree"
build_evidence="$build_worktree/doc/testing/evidence"
mkdir -p "$build_evidence"
for evidence_name in \
  governance-public-signers.json \
  liveops-public-signers.json \
  signer-truth-binding.md \
  genesis-validator-registry.json \
  governed-bootstrap-topology.md; do
  printf 'governed evidence: %s\n' "$evidence_name" >"$build_evidence/$evidence_name"
done

printf 'runtime-v1\n' >"$TMP_DIR/oasis7_chain_runtime"
chmod +x "$TMP_DIR/oasis7_chain_runtime"
printf 'config\n' >"$source_stack/config.toml"
cat >"$source_stack/config/genesis-validator-registry.json" <<'EOF'
{"validators":[]}
EOF

cat >"$source_stack/runtime-bundle.json" <<'EOF'
{
  "schema_version": "oasis7.release_candidate_bundle.v1",
  "runtime_build": {
    "sha256": "0000000000000000000000000000000000000000000000000000000000000000"
  },
  "generated_world_sidecar": {
    "kind": "directory",
    "ref": "generated-world/generated-scenario-world",
    "resolved_path": "/tmp/source/generated-world/generated-scenario-world"
  },
  "world_generation_provenance": {
    "kind": "file",
    "ref": "generated-world/world-generation-provenance.json",
    "resolved_path": "/tmp/source/generated-world/world-generation-provenance.json"
  }
}
EOF
printf '{"snapshot":"generated"}\n' >"$source_stack/generated-world/generated-scenario-world/snapshot.json"
printf '{"journal":"generated"}\n' >"$source_stack/generated-world/generated-scenario-world/journal.json"
printf '{"scenario_id":"asteroid_fragment_bootstrap"}\n' >"$source_stack/generated-world/world-generation-provenance.json"
cat >"$source_stack/genesis.json" <<EOF
{
  "schema_version": "test",
  "governance_bootstrap_refs": {
    "governance_public_manifest_ref": "$build_evidence/governance-public-signers.json",
    "liveops_public_manifest_ref": "$build_evidence/liveops-public-signers.json",
    "binding_notes_ref": "$build_evidence/signer-truth-binding.md",
    "genesis_validator_registry_ref": "$build_evidence/genesis-validator-registry.json",
    "topology_ref": "$build_evidence/governed-bootstrap-topology.md"
  }
}
EOF
cat >"$source_stack/bootstrap-peers.txt" <<'EOF'
/ip4/127.0.0.1/tcp/6831/p2p/test
EOF
cat >"$source_stack/config/manifest.json" <<'EOF'
{
  "schema_version": "oasis7.network_tier_manifest.v1",
  "runtime_refs": {
    "release_candidate_bundle_ref": "runtime-bundle.json",
    "genesis_ref": "genesis.json",
    "bootstrap_peer_ref": "bootstrap-peers.txt",
    "generated_world_sidecar_ref": "generated-world/generated-scenario-world",
    "world_generation_provenance_ref": "generated-world/world-generation-provenance.json"
  }
}
EOF

cat >"$source_stack/node.env" <<EOF
STACK_ROOT=$source_stack
NODE_ID=triad-testnet-fourth-local
CONFIG_PATH=\$STACK_ROOT/config.toml
EXECUTION_WORLD_DIR=\$STACK_ROOT/world
EXECUTION_RECORDS_DIR=\${STACK_ROOT}/execution-records
STORAGE_ROOT=\$STACK_ROOT/store
RUNTIME_ROOT=\$STACK_ROOT/runtime-root
REPLICATION_ROOT=\$STACK_ROOT/replication-root
NETWORK_TIER_MANIFEST_PATH=\$STACK_ROOT/config/manifest.json
GENESIS_VALIDATOR_REGISTRY_PATH=\${STACK_ROOT}/config/genesis-validator-registry.json
TRAFFIC_MONITOR_OUTPUT_DIR=\$STACK_ROOT/output/traffic-monitor
EOF

escape_node_root="$TMP_DIR/symlink-escape-node"
escape_external="$TMP_DIR/symlink-escape-external"
mkdir -p "$escape_node_root/config" "$escape_external"
printf 'external sentinel\n' >"$escape_external/sentinel"
ln -s "$escape_external" "$escape_node_root/config/doc"
set +e
escape_output=$("$ROOT_DIR/scripts/p2p-public-testnet-local-node-install.sh" \
  --source-env "$source_stack/node.env" \
  --source-manifest "$source_stack/config/manifest.json" \
  --runtime-build-ref "$TMP_DIR/oasis7_chain_runtime" \
  --node-root "$escape_node_root" \
  --launchd-label oasis7.testnet.symlink-escape 2>&1)
escape_status=$?
set -e
if [[ "$escape_status" -eq 0 ]]; then
  echo "expected local node install to reject governed-evidence symlink escape" >&2
  exit 1
fi
grep -q 'local node install target contains symlink component' <<<"$escape_output"
test ! -e "$escape_node_root/bin/oasis7_chain_runtime"
test "$(cat "$escape_external/sentinel")" = 'external sentinel'
test ! -e "$escape_external/governance-public-signers.json"

"$ROOT_DIR/scripts/p2p-public-testnet-local-node-install.sh" \
  --source-env "$source_stack/node.env" \
  --source-manifest "$source_stack/config/manifest.json" \
  --runtime-build-ref "$TMP_DIR/oasis7_chain_runtime" \
  --node-root "$node_root" \
  --launchd-label oasis7.testnet.smoke >/dev/null

test -x "$node_root_abs/bin/oasis7_chain_runtime"
test -x "$node_root_abs/bin/start-node.sh"
test -f "$node_root_abs/node.env"
test -f "$node_root_abs/manifest.json"
test -f "$node_root_abs/runtime-bundle.json"
test -f "$node_root_abs/generated-world/generated-scenario-world/snapshot.json"
test -f "$node_root_abs/generated-world/generated-scenario-world/journal.json"
test -f "$node_root_abs/generated-world/world-generation-provenance.json"
test -f "$node_root_abs/config.toml"
test -f "$node_root_abs/config/genesis-validator-registry.json"
test -f "$node_root_abs/oasis7.testnet.smoke.plist"

expected_sha=$(shasum -a 256 "$node_root_abs/bin/oasis7_chain_runtime" | awk '{print $1}')
jq -e --arg expected "$expected_sha" '.runtime_build.sha256 == $expected' \
  "$node_root_abs/runtime-bundle.json" >/dev/null
jq -e '.runtime_refs.release_candidate_bundle_ref == "runtime-bundle.json"' \
  "$node_root_abs/manifest.json" >/dev/null
jq -e \
  '.runtime_refs.generated_world_sidecar_ref == "generated-world/generated-scenario-world"
    and .runtime_refs.world_generation_provenance_ref == "generated-world/world-generation-provenance.json"' \
  "$node_root_abs/manifest.json" >/dev/null
jq -e \
  --arg sidecar "$node_root_abs/generated-world/generated-scenario-world" \
  --arg provenance "$node_root_abs/generated-world/world-generation-provenance.json" \
  '.generated_world_sidecar.resolved_path == $sidecar
    and .world_generation_provenance.resolved_path == $provenance' \
  "$node_root_abs/runtime-bundle.json" >/dev/null
genesis_sha_after_first=$(shasum -a 256 "$node_root_abs/genesis.json" | awk '{print $1}')

grep -q "^STACK_ROOT=$node_root_abs$" "$node_root_abs/node.env"
grep -q "^BIN=$node_root_abs/bin/oasis7_chain_runtime$" "$node_root_abs/node.env"
grep -q "^NETWORK_TIER_MANIFEST_PATH=$node_root_abs/manifest.json$" "$node_root_abs/node.env"
grep -q "^GENESIS_VALIDATOR_REGISTRY_PATH=$node_root_abs/config/genesis-validator-registry.json$" "$node_root_abs/node.env"

plutil -lint "$node_root_abs/oasis7.testnet.smoke.plist" >/dev/null

mkdir -p "$node_root_abs/replication-root" "$node_root_abs/execution-records" "$node_root_abs/store/blobs" "$node_root_abs/world-simulator-mirror" "$node_root_abs/world" "$node_root_abs/runtime-root"
printf '{"committed_height":1233}\n' >"$node_root_abs/replication-root/node_pos_state.json"
printf '{"height":1233}\n' >"$node_root_abs/execution-records/latest.json"
printf 'old blob\n' >"$node_root_abs/store/blobs/old"
printf '{"mirror":"old"}\n' >"$node_root_abs/world-simulator-mirror/snapshot.json"
printf '{"world":"old"}\n' >"$node_root_abs/world/snapshot.json"
printf '{"runtime":"old"}\n' >"$node_root_abs/runtime-root/reward-runtime-execution-bridge-state.json"

set +e
install_output=$("$ROOT_DIR/scripts/p2p-public-testnet-local-node-install.sh" \
  --source-env "$source_stack/node.env" \
  --source-manifest "$source_stack/config/manifest.json" \
  --runtime-build-ref "$TMP_DIR/oasis7_chain_runtime" \
  --node-root "$node_root" 2>&1)
install_status=$?
set -e
if [[ "$install_status" -eq 0 ]]; then
  echo "expected local node install to fail closed when persisted state exists" >&2
  exit 1
fi
grep -q 'contains persisted chain state' <<<"$install_output"
grep -q -- '--preserve-state' <<<"$install_output"
grep -q -- '--reset-state' <<<"$install_output"

set +e
conflicting_output=$("$ROOT_DIR/scripts/p2p-public-testnet-local-node-install.sh" \
  --source-env "$source_stack/node.env" \
  --source-manifest "$source_stack/config/manifest.json" \
  --runtime-build-ref "$TMP_DIR/oasis7_chain_runtime" \
  --node-root "$node_root" \
  --preserve-state \
  --reset-state 2>&1)
conflicting_status=$?
set -e
if [[ "$conflicting_status" -eq 0 ]]; then
  echo "expected local node install to reject conflicting state mode flags" >&2
  exit 1
fi
grep -q 'conflicts with --preserve-state' <<<"$conflicting_output"

"$ROOT_DIR/scripts/p2p-public-testnet-local-node-install.sh" \
  --source-env "$source_stack/node.env" \
  --source-manifest "$source_stack/config/manifest.json" \
  --runtime-build-ref "$TMP_DIR/oasis7_chain_runtime" \
  --node-root "$node_root" \
  --preserve-state >/dev/null

test "$(shasum -a 256 "$node_root_abs/genesis.json" | awk '{print $1}')" = "$genesis_sha_after_first"

test -f "$node_root_abs/replication-root/node_pos_state.json"
test -f "$node_root_abs/execution-records/latest.json"
test -f "$node_root_abs/store/blobs/old"
test -f "$node_root_abs/world-simulator-mirror/snapshot.json"
test -f "$node_root_abs/world/snapshot.json"
test -f "$node_root_abs/runtime-root/reward-runtime-execution-bridge-state.json"

reset_symlink_backup="$TMP_DIR/reset-state-symlink-backup"
reset_symlink_external="$TMP_DIR/reset-state-symlink-external"
mkdir -p "$reset_symlink_external"
mv "$node_root_abs/config/doc" "$node_root_abs/config/doc.real"
ln -s "$reset_symlink_external" "$node_root_abs/config/doc"
set +e
reset_symlink_output=$("$ROOT_DIR/scripts/p2p-public-testnet-local-node-install.sh" \
  --source-env "$source_stack/node.env" \
  --source-manifest "$source_stack/config/manifest.json" \
  --runtime-build-ref "$TMP_DIR/oasis7_chain_runtime" \
  --node-root "$node_root" \
  --reset-state \
  --state-backup-dir "$reset_symlink_backup" 2>&1)
reset_symlink_status=$?
set -e
if [[ "$reset_symlink_status" -eq 0 ]]; then
  echo "expected reset-state to reject a config/doc symlink before mutating state" >&2
  exit 1
fi
grep -q 'local node install target contains symlink component' <<<"$reset_symlink_output"
# Regression contract: an unsafe governed target must fail before either the
# persisted state or the caller-selected backup root is changed.
if [[ ! -f "$node_root_abs/replication-root/node_pos_state.json" \
  || ! -f "$node_root_abs/execution-records/latest.json" \
  || ! -f "$node_root_abs/store/blobs/old" \
  || -e "$reset_symlink_backup" ]]; then
  echo "reset-state symlink rejection mutated persisted state or created a backup" >&2
  exit 1
fi
rm "$node_root_abs/config/doc"
mv "$node_root_abs/config/doc.real" "$node_root_abs/config/doc"

reset_backup="$TMP_DIR/reset-backup"
"$ROOT_DIR/scripts/p2p-public-testnet-local-node-install.sh" \
  --source-env "$source_stack/node.env" \
  --source-manifest "$source_stack/config/manifest.json" \
  --runtime-build-ref "$TMP_DIR/oasis7_chain_runtime" \
  --node-root "$node_root" \
  --reset-state \
  --state-backup-dir "$reset_backup" >/dev/null

test -f "$reset_backup/replication-root/node_pos_state.json"
test -f "$reset_backup/execution-records/latest.json"
test -f "$reset_backup/store/blobs/old"
test -f "$reset_backup/world-simulator-mirror/snapshot.json"
test -f "$reset_backup/world/snapshot.json"
test -f "$reset_backup/runtime-root/reward-runtime-execution-bridge-state.json"
test ! -e "$node_root_abs/replication-root/node_pos_state.json"
test ! -e "$node_root_abs/execution-records/latest.json"
test ! -e "$node_root_abs/store/blobs/old"
test ! -e "$node_root_abs/world-simulator-mirror/snapshot.json"
test ! -e "$node_root_abs/world/snapshot.json"
test ! -e "$node_root_abs/runtime-root/reward-runtime-execution-bridge-state.json"

test "$(shasum -a 256 "$node_root_abs/genesis.json" | awk '{print $1}')" = "$genesis_sha_after_first"
python3 - "$node_root_abs" "$build_worktree" <<'PY'
from pathlib import Path
import json
import sys

node_root = Path(sys.argv[1])
build_worktree = sys.argv[2]
genesis_path = node_root / "genesis.json"
genesis = json.loads(genesis_path.read_text(encoding="utf-8"))
refs = genesis["governance_bootstrap_refs"]
expected_names = {
    "governance_public_manifest_ref": "governance-public-signers.json",
    "liveops_public_manifest_ref": "liveops-public-signers.json",
    "binding_notes_ref": "signer-truth-binding.md",
    "genesis_validator_registry_ref": "genesis-validator-registry.json",
    "topology_ref": "governed-bootstrap-topology.md",
}
assert set(refs) == set(expected_names), f"unexpected governed ref keys: {sorted(refs)}"
for key, name in expected_names.items():
    expected = node_root / "config" / "doc" / "testing" / "evidence" / name
    assert refs[key] == str(expected), f"{key} was not localized: {refs[key]!r}"
    assert expected.is_file(), f"localized governed evidence missing: {expected}"
assert build_worktree not in genesis_path.read_text(encoding="utf-8"), (
    "build-worktree absolute governance ref survived local install"
)
assert not genesis_path.read_bytes().startswith(b"\xef\xbb\xbf"), (
    "localized installed genesis must be UTF-8 without BOM"
)
PY

tree_digest() {
  python3 - "$1" <<'PY'
from pathlib import Path
import hashlib
import sys

root = Path(sys.argv[1])
digest = hashlib.sha256()
if root.exists():
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root).as_posix()
        if path.is_symlink():
            kind = b"link"
            payload = path.readlink().as_posix().encode()
        elif path.is_dir():
            kind = b"dir"
            payload = b""
        else:
            kind = b"file"
            payload = path.read_bytes()
        digest.update(kind + b"\0" + relative.encode() + b"\0" + payload + b"\0")
print(digest.hexdigest())
PY
}

negative_contract_failures=0
for negative_case in missing_source basename_collision; do
  negative_source="$TMP_DIR/$negative_case-source"
  negative_root="$TMP_DIR/$negative_case-node"
  negative_backup="$TMP_DIR/$negative_case-backup"
  cp -R "$source_stack" "$negative_source"
  sed -i.bak "s|^STACK_ROOT=.*|STACK_ROOT=$negative_source|" "$negative_source/node.env"
  rm "$negative_source/node.env.bak"

  if [[ "$negative_case" == "missing_source" ]]; then
    python3 - "$negative_source/genesis.json" "$negative_source/missing-governance.json" <<'PY'
from pathlib import Path
import json
import sys

path = Path(sys.argv[1])
data = json.loads(path.read_text(encoding="utf-8"))
data["governance_bootstrap_refs"]["liveops_public_manifest_ref"] = sys.argv[2]
path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
PY
    expected_failure='genesis governance ref source missing'
  else
    first_source="$negative_source/collision-a/shared-governance.json"
    second_source="$negative_source/collision-b/shared-governance.json"
    mkdir -p "$(dirname "$first_source")" "$(dirname "$second_source")"
    printf 'first\n' >"$first_source"
    printf 'second\n' >"$second_source"
    python3 - "$negative_source/genesis.json" "$first_source" "$second_source" <<'PY'
from pathlib import Path
import json
import sys

path = Path(sys.argv[1])
data = json.loads(path.read_text(encoding="utf-8"))
refs = data["governance_bootstrap_refs"]
refs["governance_public_manifest_ref"] = sys.argv[2]
refs["liveops_public_manifest_ref"] = sys.argv[3]
path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
PY
    expected_failure='genesis governance refs collide at localized target'
  fi

  mkdir -p "$negative_root/world" "$negative_root/config"
  printf 'persisted state must survive\n' >"$negative_root/world/sentinel"
  printf 'existing config must survive\n' >"$negative_root/config/sentinel"
  before_digest=$(tree_digest "$negative_root")
  if "$ROOT_DIR/scripts/p2p-public-testnet-local-node-install.sh" \
    --source-env "$negative_source/node.env" \
    --source-manifest "$negative_source/config/manifest.json" \
    --runtime-build-ref "$TMP_DIR/oasis7_chain_runtime" \
    --node-root "$negative_root" \
    --reset-state \
    --state-backup-dir "$negative_backup" \
    >"$TMP_DIR/$negative_case.stdout" 2>"$TMP_DIR/$negative_case.stderr"; then
    echo "expected $negative_case governed-ref preflight failure" >&2
    exit 1
  fi
  grep -q "$expected_failure" "$TMP_DIR/$negative_case.stderr"
  mutation_detected=0
  if [[ "$(tree_digest "$negative_root")" != "$before_digest" ]]; then
    echo "local install $negative_case preflight mutated node root" >&2
    mutation_detected=1
  fi
  if [[ -e "$negative_backup" ]]; then
    echo "local install $negative_case preflight moved persisted state into backup" >&2
    mutation_detected=1
  fi
  if [[ "$mutation_detected" -ne 0 ]]; then
    negative_contract_failures=1
  fi
done

if [[ "$negative_contract_failures" -ne 0 ]]; then
  exit 1
fi

echo "ok: local testnet node install pins runtime artifact under dedicated root"
