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
  }
}
EOF
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
cat >"$source_stack/manifest.json" <<'EOF'
{
  "schema_version": "oasis7.network_tier_manifest.v1",
  "runtime_refs": {
    "release_candidate_bundle_ref": "runtime-bundle.json",
    "genesis_ref": "genesis.json",
    "bootstrap_peer_ref": "bootstrap-peers.txt"
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
NETWORK_TIER_MANIFEST_PATH=\$STACK_ROOT/manifest.json
GENESIS_VALIDATOR_REGISTRY_PATH=\${STACK_ROOT}/config/genesis-validator-registry.json
TRAFFIC_MONITOR_OUTPUT_DIR=\$STACK_ROOT/output/traffic-monitor
EOF

"$ROOT_DIR/scripts/p2p-public-testnet-local-node-install.sh" \
  --source-env "$source_stack/node.env" \
  --source-manifest "$source_stack/manifest.json" \
  --runtime-build-ref "$TMP_DIR/oasis7_chain_runtime" \
  --node-root "$node_root" \
  --launchd-label oasis7.testnet.smoke >/dev/null

test -x "$node_root_abs/bin/oasis7_chain_runtime"
test -x "$node_root_abs/bin/start-node.sh"
test -f "$node_root_abs/node.env"
test -f "$node_root_abs/manifest.json"
test -f "$node_root_abs/runtime-bundle.json"
test -f "$node_root_abs/.tmp/local-node-install-test/governance-public-signers.json"
test -f "$node_root_abs/config.toml"
test -f "$node_root_abs/config/genesis-validator-registry.json"
test -f "$node_root_abs/oasis7.testnet.smoke.plist"

expected_sha=$(shasum -a 256 "$node_root_abs/bin/oasis7_chain_runtime" | awk '{print $1}')
jq -e --arg expected "$expected_sha" '.runtime_build.sha256 == $expected' \
  "$node_root_abs/runtime-bundle.json" >/dev/null
jq -e '.runtime_refs.release_candidate_bundle_ref == "runtime-bundle.json"' \
  "$node_root_abs/manifest.json" >/dev/null

grep -q "^STACK_ROOT=$node_root_abs$" "$node_root_abs/node.env"
grep -q "^BIN=$node_root_abs/bin/oasis7_chain_runtime$" "$node_root_abs/node.env"
grep -q "^NETWORK_TIER_MANIFEST_PATH=$node_root_abs/manifest.json$" "$node_root_abs/node.env"
grep -q "^GENESIS_VALIDATOR_REGISTRY_PATH=$node_root_abs/config/genesis-validator-registry.json$" "$node_root_abs/node.env"

plutil -lint "$node_root_abs/oasis7.testnet.smoke.plist" >/dev/null

echo "ok: local testnet node install pins runtime artifact under dedicated root"
