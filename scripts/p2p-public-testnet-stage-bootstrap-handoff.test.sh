#!/usr/bin/env bash
set -euo pipefail

# Cross-surface contract: a stage emitted by the governed-stage generator must
# be consumable by fresh-host bootstrap without path rewrites or implicit
# authority.  The test uses only temporary roots and OASIS7_TEST_ONLY=1; it
# never starts a service or mutates a host.
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAGE_BUILDER="$ROOT_DIR/scripts/p2p-public-testnet-build-deployment-stage.sh"
BOOTSTRAP="$ROOT_DIR/scripts/p2p-public-testnet-bootstrap-fresh-validator-host.sh"
START_NODE="$ROOT_DIR/scripts/p2p-triad-node-start.sh"
INVENTORY="$ROOT_DIR/scripts/public-testnet-validator-triad-inventory.v1.json"
TRIAD_PEERS="$ROOT_DIR/doc/testing/evidence/public-testnet-governed-bootstrap-validator-triad-bootstrap-peers-2026-09-15.txt"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-stage-bootstrap-handoff.XXXXXX")"
trap 'rm -rf -- "$TMP_DIR"' EXIT

fail() {
  printf 'FAIL: %s\n' "$*" >&2
  exit 1
}

expect_fail() {
  local expected=$1 label=$2
  shift 2
  local output="$TMP_DIR/$label.out"
  if "$@" >"$output" 2>&1; then
    cat "$output" >&2
    fail "expected failure for $label: $*"
  fi
  grep -Fq -- "$expected" "$output" || {
    cat "$output" >&2
    fail "$label did not contain expected failure: $expected"
  }
}

test -x "$STAGE_BUILDER" || fail "missing stage builder: $STAGE_BUILDER"
test -x "$BOOTSTRAP" || fail "missing fresh-host bootstrap: $BOOTSTRAP"
test -x "$START_NODE" || fail "missing node launcher: $START_NODE"

runtime_fixture="$TMP_DIR/validator-47-runtime"
cat >"$runtime_fixture" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

[[ "${1:-}" == identity-receipt ]] || exit 64
shift
config_dir=""
node_id=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --config-dir) config_dir=${2:-}; shift 2 ;;
    --node-id) node_id=${2:-}; shift 2 ;;
    *) exit 64 ;;
  esac
done
key_path="$config_dir/node-keypair.toml"
receipt_path="$config_dir/identity-receipt.json"
test -f "$key_path" -a -f "$receipt_path"
python3 - "$config_dir" "$node_id" "$key_path" "$receipt_path" <<'PY'
import hashlib
import json
import pathlib
import sys

config_dir = pathlib.Path(sys.argv[1]).resolve()
node_id = sys.argv[2]
key_path = pathlib.Path(sys.argv[3]).resolve()
receipt_path = pathlib.Path(sys.argv[4])
receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
metadata = key_path.stat()
print(json.dumps({
    "schema_version": "oasis7.identity_receipt.v1",
    "node_id": node_id,
    "peer_id": receipt["libp2p_peer_id"],
    "key_path": str(key_path),
    "key_sha256": hashlib.sha256(key_path.read_bytes()).hexdigest(),
    "key_size_bytes": metadata.st_size,
    "key_mode": 384,
    "key_uid": metadata.st_uid,
    "key_gid": metadata.st_gid,
    "root_public_key": receipt["root_public_key"],
    "finality_public_key": receipt["finality_public_key"],
    "libp2p_peer_id": receipt["libp2p_peer_id"],
    "node_keypair_config_path": str(key_path),
    "node_keypair_config_exists": True,
    "node_keypair_config_mode": "0600",
}, sort_keys=True))
PY
SH
chmod 0755 "$runtime_fixture"

identity_dir="$TMP_DIR/validator-47-identity"
mkdir -p "$identity_dir"
chmod 0700 "$identity_dir"
validator47_root_key=$(jq -r '.nodes["validator-47"].root_public_key' "$INVENTORY")
validator47_finality_key=$(jq -r '.nodes["validator-47"].finality_public_key' "$INVENTORY")
validator47_peer_id=$(jq -r '.nodes["validator-47"].libp2p_peer_id' "$INVENTORY")
printf 'handoff-fixture-key\n' >"$identity_dir/node-keypair.toml"
chmod 0600 "$identity_dir/node-keypair.toml"
cat >"$identity_dir/identity-receipt.json" <<EOF
{"schema_version":"oasis7.identity_provision.v1","node_id":"triad-testnet-validator-47","root_public_key":"$validator47_root_key","finality_public_key":"$validator47_finality_key","libp2p_peer_id":"$validator47_peer_id"}
EOF
chmod 0600 "$identity_dir/identity-receipt.json"

seq_signer=$(jq -r '.nodes["sequencer-204"].finality_signer_public_key' "$INVENTORY")
storage_signer=$(jq -r '.nodes["storage-205"].finality_signer_public_key' "$INVENTORY")
triad_stage="$TMP_DIR/stage-triad"
"$STAGE_BUILDER" \
  --runtime-build-ref "$runtime_fixture" \
  --bootstrap-peers-file "$TRIAD_PEERS" \
  --sequencer-finality-public-key "$seq_signer" \
  --storage-finality-public-key "$storage_signer" \
  --extra-validator "triad-testnet-validator-47:$validator47_finality_key:100" \
  --validator-47-identity-dir "$identity_dir" \
  --out-dir "$triad_stage" >/dev/null

triad_config="$triad_stage/config"
triad_world="$triad_stage/generated-world"
triad_manifest="$triad_config/public-testnet-governed-bootstrap-manifest-2026-06-06.json"
triad_registry="$triad_config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json"
triad_env="$triad_config/node.env"
for required in \
  "$triad_manifest" "$triad_registry" "$triad_env" \
  "$triad_config/public-testnet-validator-triad-inventory.v1.json" \
  "$triad_config/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt" \
  "$triad_world/world/snapshot.json" \
  "$triad_world/world-generation-provenance.json" \
  "$triad_world/generated-scenario-world/snapshot.json"; do
  test -e "$required" || fail "stage missing handoff input: $required"
done
jq -e \
  '.deployment_inventory.ref == "scripts/public-testnet-validator-triad-inventory.v1.json"
   and .deployment_validator_registry.ref == "config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json"
   and .network_id == "oasis7-public-testnet-governed-20260606"
   and .chain_id == "oasis7-public-testnet-governed-20260606"' \
  "$triad_manifest" >/dev/null
grep -qx 'WORLD_ID=oasis7-public-testnet-governed-20260606' "$triad_env"
grep -qx 'NODE_ROLE=storage' "$triad_env"
grep -qx 'P2P_NODE_ROLE=full_storage' "$triad_env"

# Relative authority paths must remain relative to the installed stack, not to
# the operator's current directory.  Use the generated manifest/registry in a
# non-managed observer environment and inspect the real launcher command.
observer_stage="$TMP_DIR/stage-observer"
"$STAGE_BUILDER" \
  --runtime-build-ref "$runtime_fixture" \
  --bootstrap-peers-file "$TRIAD_PEERS" \
  --sequencer-finality-public-key "$seq_signer" \
  --storage-finality-public-key "$storage_signer" \
  --extra-validator "triad-testnet-fourth-local:f640bc1ceb82b261baf51ab1504a2dc4c10901873252e67551dcfe1f5b7b21af:100" \
  --out-dir "$observer_stage" >/dev/null
observer_config="$observer_stage/config"
observer_env="$observer_config/node.env"
cat >"$observer_env" <<'EOF'
NODE_ID=triad-testnet-fourth-local
NODE_ROLE=observer
P2P_NODE_ROLE=observer_light
WORLD_ID=oasis7-public-testnet-governed-20260606
STATUS_BIND=127.0.0.1:0
NODE_GOSSIP_BIND=127.0.0.1:0
CONFIG_PATH=config/node-keypair.toml
EXECUTION_WORLD_DIR=generated-world/world
EXECUTION_RECORDS_DIR=data/execution-records
STORAGE_ROOT=data/storage
STORAGE_PROFILE=release_default
NODE_TICK_MS=200
POS_SLOT_DURATION_MS=8000
POS_TICKS_PER_SLOT=10
POS_PROPOSAL_TICK_PHASE=9
POS_MAX_PAST_SLOT_LAG=256
REWARD_RUNTIME_ENABLE=0
REWARD_RUNTIME_EPOCH_DURATION_SECS=60
REWARD_POINTS_PER_CREDIT=100
REWARD_RUNTIME_AUTO_REDEEM=0
NETWORK_TIER_MANIFEST_PATH=config/public-testnet-governed-bootstrap-manifest-2026-06-06.json
GENESIS_VALIDATOR_REGISTRY_PATH=config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json
REPLICATION_NETWORK_LISTEN_ADDRS_CSV=/ip4/127.0.0.1/tcp/0
EOF
observer_launch_log="$TMP_DIR/observer-launch.out"
observer_physical_root=$(python3 - "$observer_stage" <<'PY'
import os
import sys
print(os.path.realpath(sys.argv[1]))
PY
)
(
  cd "$observer_stage"
  APP_ROOT="$PWD" ENV_FILE="$PWD/config/node.env" BIN="$runtime_fixture" \
    OASIS7_NODE_START_DRY_RUN=1 bash "$START_NODE" >"$observer_launch_log"
)
grep -q '^runtime command:' "$observer_launch_log"
grep -Eq -- '--network-tier-manifest[[:space:]]+config/public-testnet-governed-bootstrap-manifest-2026-06-06.json' "$observer_launch_log"
grep -Eq -- "--genesis-validator-registry[[:space:]]+$observer_physical_root/config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json" "$observer_launch_log"
if grep -q -- '--deployment-inventory' "$observer_launch_log"; then
  fail "non-managed observer inherited managed deployment inventory"
fi

# Build the smallest verified package/ops archives accepted by the real
# bootstrap script.  The package runtime is the same identity-readback fixture
# used by the stage generator, so bootstrap cannot accidentally validate a
# different binary than the stage's governed runtime ref.
package_deb="$TMP_DIR/oasis7-linux-x64.deb"
package_root="$TMP_DIR/oasis7-linux-x64.deb.root/opt/oasis7"
mkdir -p "$package_root/bin" "$TMP_DIR/oasis7-linux-x64.deb.root/usr/bin"
cp "$runtime_fixture" "$package_root/bin/oasis7_chain_runtime"
chmod 0755 "$package_root/bin/oasis7_chain_runtime"
package_commit=$(jq -r '.git_commit' "$triad_config/public-testnet-governed-bootstrap-bundle-2026-06-06.json")
cat >"$package_root/BUILDINFO" <<EOF
commit=$package_commit
package_version=0.0.0+testnet.handoff
run_id=3701
platform=linux-x64
EOF
(cd "$package_root" && shasum -a 256 BUILDINFO bin/oasis7_chain_runtime >SHA256SUMS)
printf 'handoff-package\n' >"$package_deb"
ln -s /opt/oasis7/run-client.sh "$TMP_DIR/oasis7-linux-x64.deb.root/usr/bin/oasis7-client"

ops_dir="$TMP_DIR/oasis7-linux-x64-ops-tools"
mkdir -p "$ops_dir/bin"
for binary in oasis7_world_repair_rebuild oasis7_governance_registry_import oasis7_governance_registry_audit; do
  cat >"$ops_dir/bin/$binary" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[[ "${1:-}" == --help ]] || exit 0
printf '%s\n' '--generated-world-dir'
SH
  chmod 0755 "$ops_dir/bin/$binary"
done
cat >"$ops_dir/bin/service-readback" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' '{"schema_version":"oasis7.human_direct_ssh_readback.v1","active":false,"running":false,"service_state":"stopped","independently_observed":true,"unit_file_state":"disabled","no_process":true,"no_listener":true,"listeners":[]}'
SH
chmod 0755 "$ops_dir/bin/service-readback"
printf '{"schema_version":"oasis7.ops-tools.v1"}\n' >"$ops_dir/.oasis7-ops-tools-manifest.json"
(cd "$ops_dir" && shasum -a 256 bin/* >SHA256SUMS)
ops_tools_tar="$TMP_DIR/oasis7-linux-x64-ops-tools.tar.gz"
(cd "$TMP_DIR" && tar -czf "$ops_tools_tar" oasis7-linux-x64-ops-tools)

fake_bin="$TMP_DIR/fake-bin"
mkdir -p "$fake_bin"
cat >"$fake_bin/dpkg-deb" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[[ "${1:-}" == --extract ]] || exit 2
mkdir -p "$3"
cp -a "$2.root/." "$3/"
SH
chmod 0755 "$fake_bin/dpkg-deb"
export PATH="$fake_bin:$PATH"

bootstrap_with_stage() {
  local target_root=$1 target_systemd=$2 target_receipt=$3 source_stage=$4
  OASIS7_TEST_ONLY=1 "$BOOTSTRAP" \
    --allow-test-stack-root \
    --test-root-prefix "$TMP_DIR" \
    --systemd-unit-dir "$target_systemd" \
    --stack-root "$target_root" \
    --package-deb "$package_deb" \
    --ops-tools-tar "$ops_tools_tar" \
    --config-dir "$source_stage/config" \
    --world-dir "$source_stage/generated-world" \
    --identity-dir "$identity_dir" \
    --node-id triad-testnet-validator-47 \
    --service-name oasis7-triad-validator-47.service \
    --receipt "$target_receipt"
}

# The generated stage must be directly consumable.  This is the positive
# stage->bootstrap handoff and also proves the generated world path contract.
bootstrap_root="$TMP_DIR/bootstrapped-opt/oasis7/p2p-testnet"
bootstrap_systemd="$TMP_DIR/bootstrapped-systemd"
bootstrap_receipt="$TMP_DIR/bootstrapped-receipt.json"
bootstrap_with_stage "$bootstrap_root" "$bootstrap_systemd" "$bootstrap_receipt" "$triad_stage" \
  >"$TMP_DIR/bootstrap-positive.out"
grep -q '^fresh_validator_host_bootstrap=complete ' "$TMP_DIR/bootstrap-positive.out"
test -L "$bootstrap_root/current"
test -f "$bootstrap_root/staged-world/snapshot.json"
test -f "$bootstrap_root/staged-world/generated-scenario-world/snapshot.json"
python3 - "$bootstrap_systemd/oasis7-triad-validator-47.service" "$bootstrap_root" <<'PY'
import os
import sys
from pathlib import Path

unit = Path(sys.argv[1])
root = Path(sys.argv[2])
values = {}
for line in unit.read_text(encoding="utf-8").splitlines():
    if "=" in line:
        key, value = line.split("=", 1)
        values[key] = value
expected = {
    "WorkingDirectory": root,
    "EnvironmentFile": root / "config/node.env",
    "ExecStart": root / "bin/start-node.sh",
}
for key, path in expected.items():
    assert os.path.realpath(values.get(key, "")) == os.path.realpath(path), (key, values.get(key), path)
PY
jq -e \
  '.no_service_started == true
   and .identity.mode == "imported"
   and (.config.node_env.path | endswith("/config/node.env"))
   and (.world.snapshot.path | endswith("/staged-world/snapshot.json"))' \
  "$bootstrap_receipt" >/dev/null
cmp -s "$identity_dir/node-keypair.toml" "$bootstrap_root/config/node-keypair.toml"
cmp -s "$identity_dir/identity-receipt.json" "$bootstrap_root/config/identity-receipt.json"
test ! -e "$TMP_DIR/systemctl.log"

# Bootstrap must reject explicit stale world identity and role drift before
# creating a target root.  These are source-stage mutations, not host actions.
stale_stage="$TMP_DIR/stage-stale-world"
cp -a "$triad_stage" "$stale_stage"
sed -i.bak 's/^WORLD_ID=.*/WORLD_ID=stale-public-testnet-world/' "$stale_stage/config/node.env"
rm -f "$stale_stage/config/node.env.bak"
stale_root="$TMP_DIR/stale-opt/oasis7/p2p-testnet"
expect_fail 'validator-47 node.env WORLD_ID mismatch' stale-world \
  bootstrap_with_stage "$stale_root" "$TMP_DIR/stale-systemd" "$TMP_DIR/stale-receipt.json" "$stale_stage"
test ! -e "$stale_root"

role_drift_stage="$TMP_DIR/stage-role-drift"
cp -a "$triad_stage" "$role_drift_stage"
sed -i.bak 's/^NODE_ROLE=.*/NODE_ROLE=observer/' "$role_drift_stage/config/node.env"
rm -f "$role_drift_stage/config/node.env.bak"
role_drift_root="$TMP_DIR/role-drift-opt/oasis7/p2p-testnet"
expect_fail 'validator-47 node.env NODE_ROLE mismatch' role-drift \
  bootstrap_with_stage "$role_drift_root" "$TMP_DIR/role-drift-systemd" "$TMP_DIR/role-drift-receipt.json" "$role_drift_stage"
test ! -e "$role_drift_root"

provider_role_drift_stage="$TMP_DIR/stage-provider-role-drift"
cp -a "$triad_stage" "$provider_role_drift_stage"
sed -i.bak 's/^P2P_NODE_ROLE=.*/P2P_NODE_ROLE=observer_light/' "$provider_role_drift_stage/config/node.env"
rm -f "$provider_role_drift_stage/config/node.env.bak"
provider_role_drift_root="$TMP_DIR/provider-role-drift-opt/oasis7/p2p-testnet"
expect_fail 'validator-47 node.env P2P_NODE_ROLE mismatch' provider-role-drift \
  bootstrap_with_stage "$provider_role_drift_root" "$TMP_DIR/provider-role-drift-systemd" "$TMP_DIR/provider-role-drift-receipt.json" "$provider_role_drift_stage"
test ! -e "$provider_role_drift_root"

printf '%s\n' 'p2p-public-testnet-stage-bootstrap-handoff checks passed'
