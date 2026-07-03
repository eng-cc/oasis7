#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-local-node-install-test.XXXXXX")"
REPO_TMP_FIXTURE="$ROOT_DIR/.tmp/local-node-install-test"
trap 'rm -rf "$TMP_DIR" "$REPO_TMP_FIXTURE"' EXIT

source_stack="$TMP_DIR/source-stack"
node_root="$TMP_DIR/dedicated-node"
node_root_abs=$(python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).resolve())' "$node_root")
mkdir -p "$source_stack/config"
mkdir -p "$source_stack/generated-world/generated-scenario-world"
mkdir -p "$REPO_TMP_FIXTURE"

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
cat >"$REPO_TMP_FIXTURE/governance-public-signers.json" <<'EOF'
{"signers":[]}
EOF
cat >"$source_stack/genesis.json" <<'EOF'
{
  "schema_version": "test",
  "governance_bootstrap_refs": {
    "governance_public_manifest_ref": ".tmp/local-node-install-test/governance-public-signers.json"
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
test -f "$node_root_abs/.tmp/local-node-install-test/governance-public-signers.json"
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

grep -q "^STACK_ROOT=$node_root_abs$" "$node_root_abs/node.env"
grep -q "^BIN=$node_root_abs/bin/oasis7_chain_runtime$" "$node_root_abs/node.env"
grep -q "^NETWORK_TIER_MANIFEST_PATH=$node_root_abs/manifest.json$" "$node_root_abs/node.env"
grep -q "^GENESIS_VALIDATOR_REGISTRY_PATH=$node_root_abs/config/genesis-validator-registry.json$" "$node_root_abs/node.env"

plutil -lint "$node_root_abs/oasis7.testnet.smoke.plist" >/dev/null

mkdir -p "$node_root_abs/replication-root" "$node_root_abs/execution-records" "$node_root_abs/store/blobs" "$node_root_abs/world-simulator-mirror"
printf '{"committed_height":1233}\n' >"$node_root_abs/replication-root/node_pos_state.json"
printf '{"height":1233}\n' >"$node_root_abs/execution-records/latest.json"
printf 'old blob\n' >"$node_root_abs/store/blobs/old"
printf '{"mirror":"old"}\n' >"$node_root_abs/world-simulator-mirror/snapshot.json"

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

test -f "$node_root_abs/replication-root/node_pos_state.json"
test -f "$node_root_abs/execution-records/latest.json"
test -f "$node_root_abs/store/blobs/old"
test -f "$node_root_abs/world-simulator-mirror/snapshot.json"

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
test ! -e "$node_root_abs/replication-root/node_pos_state.json"
test ! -e "$node_root_abs/execution-records/latest.json"
test ! -e "$node_root_abs/store/blobs/old"
test ! -e "$node_root_abs/world-simulator-mirror/snapshot.json"

echo "ok: local testnet node install pins runtime artifact under dedicated root"
