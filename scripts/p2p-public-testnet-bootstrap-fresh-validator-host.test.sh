#!/usr/bin/env bash
set -euo pipefail

# RED contract for an erased Linux validator host. This test deliberately
# exercises only the explicitly dual-gated test override; it must never touch
# a real host, /opt, or systemctl.
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BOOTSTRAP="$ROOT_DIR/scripts/p2p-public-testnet-bootstrap-fresh-validator-host.sh"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-fresh-validator-bootstrap-test.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

fail() {
  printf 'FAIL: %s\n' "$*" >&2
  exit 1
}

expect_fail() {
  local expected=$1
  shift
  local output="$TMP_DIR/negative-$RANDOM.out"
  if "$@" >"$output" 2>&1; then
    fail "expected command to fail: $*"
  fi
  grep -Fq "$expected" "$output" || {
    cat "$output" >&2
    fail "expected failure containing: $expected"
  }
}

test -f "$BOOTSTRAP" || fail "missing fresh validator host bootstrap script: $BOOTSTRAP"

stack_root="$TMP_DIR/opt/oasis7/p2p-testnet"
bundle_dir="$TMP_DIR/bundle/oasis7-linux-x64.deb.root/opt/oasis7"
ops_bundle_dir="$TMP_DIR/bundle/oasis7-linux-x64-ops-tools"
package_deb="$TMP_DIR/bundle/oasis7-linux-x64.deb"
ops_tools_tar="$TMP_DIR/bundle/oasis7-linux-x64-ops-tools.tar.gz"
config_dir="$TMP_DIR/config"
world_dir="$TMP_DIR/world"
systemd_dir="$TMP_DIR/systemd"
receipt="$TMP_DIR/bootstrap-receipt.json"
mkdir -p "$bundle_dir/bin" "$ops_bundle_dir/bin" "$config_dir/doc/testing/evidence" "$world_dir/generated-scenario-world" "$systemd_dir" "$TMP_DIR/fake-bin"

for binary in oasis7_world_repair_rebuild oasis7_governance_registry_import oasis7_governance_registry_audit; do
  cat >"$ops_bundle_dir/bin/$binary" <<'SH'
#!/usr/bin/env bash
if [[ "${1:-}" == "--help" ]]; then
  printf '%s\n' '--generated-world-dir'
fi
SH
  chmod +x "$ops_bundle_dir/bin/$binary"
done
cat >"$ops_bundle_dir/bin/service-readback" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' '{"schema_version":"oasis7.human_direct_ssh_readback.v1","active":false,"running":false,"service_state":"stopped","independently_observed":true,"listeners":[]}'
SH
chmod +x "$ops_bundle_dir/bin/service-readback"
cat >"$bundle_dir/bin/oasis7_chain_runtime" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${OASIS7_TEST_RUNTIME_LOG:-}" ]]; then
  printf '%s\n' "${1:-}" >>"$OASIS7_TEST_RUNTIME_LOG"
fi
command_name=${1:-}
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
[[ "$config_dir" == /* && -n "$node_id" ]] || exit 64
key_path="$config_dir/node-keypair.toml"
if [[ "$command_name" == provision-identity ]]; then
  mkdir -p "$config_dir"
  umask 077
  printf 'fixture-key-file-only\n' >"$key_path"
  chmod 600 "$key_path"
  python3 - "$config_dir" "$node_id" "$key_path" <<'PY'
import json
import pathlib
import sys

config_dir = pathlib.Path(sys.argv[1]).resolve()
node_id = sys.argv[2]
key_path = pathlib.Path(sys.argv[3]).resolve()
print(json.dumps({
    "node_id": node_id,
    "root_public_key": "fixture-root-public-key",
    "finality_public_key": "fixture-finality-public-key",
    "libp2p_peer_id": "fixture-libp2p-peer-id",
    "node_keypair_config_path": str(key_path),
    "node_keypair_config_exists": True,
    "node_keypair_config_mode": "0600",
}, sort_keys=True))
PY
elif [[ "$command_name" == identity-receipt ]]; then
  test -f "$key_path"
  peer_id=${OASIS7_TEST_RUNTIME_PEER_ID:-validator-47-peer}
  python3 - "$config_dir" "$node_id" "$key_path" "$peer_id" <<'PY'
import hashlib
import json
import pathlib
import sys

config_dir = pathlib.Path(sys.argv[1]).resolve()
node_id = sys.argv[2]
key_path = pathlib.Path(sys.argv[3]).resolve()
peer_id = sys.argv[4]
metadata = key_path.stat()
print(json.dumps({
    "schema_version": "oasis7.identity_receipt.v1",
    "node_id": node_id,
    "peer_id": peer_id,
    "key_path": str(key_path),
    "key_sha256": hashlib.sha256(key_path.read_bytes()).hexdigest(),
    "key_size_bytes": metadata.st_size,
    "key_mode": 384,
    "key_uid": metadata.st_uid,
    "key_gid": metadata.st_gid,
}, sort_keys=True))
PY
else
  exit 64
fi
SH
chmod +x "$bundle_dir/bin/oasis7_chain_runtime"
runtime_sha="$(shasum -a 256 "$bundle_dir/bin/oasis7_chain_runtime" | awk '{print $1}')"
runtime_size="$(wc -c <"$bundle_dir/bin/oasis7_chain_runtime" | tr -d ' ')"
cat >"$bundle_dir/BUILDINFO" <<EOF
commit=abcdef1234567890abcdef1234567890abcdef12
package_version=0.0.0+testnet.test.abcdef123456
run_id=2737
platform=linux-x64
EOF
(cd "$bundle_dir" && shasum -a 256 BUILDINFO "bin/oasis7_chain_runtime" >SHA256SUMS)
printf '{"schema_version":"oasis7.ops-tools.v1"}\n' >"$ops_bundle_dir/.oasis7-ops-tools-manifest.json"
(cd "$ops_bundle_dir" && shasum -a 256 bin/* >SHA256SUMS)
tar -czf "$ops_tools_tar" -C "$TMP_DIR/bundle" oasis7-linux-x64-ops-tools
printf 'fixture-deb\n' >"$package_deb"
# Native Debian installers expose launcher symlinks under /usr/bin.  The safe
# extraction boundary must allow those package-owned links while still
# rejecting symlinks inside the consumed /opt/oasis7 subtree.
mkdir -p "$TMP_DIR/bundle/oasis7-linux-x64.deb.root/usr/bin"
ln -s /opt/oasis7/run-client.sh \
  "$TMP_DIR/bundle/oasis7-linux-x64.deb.root/usr/bin/oasis7-client"
cat >"$TMP_DIR/fake-bin/dpkg-deb" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[[ "${1:-}" == "--extract" ]] || exit 2
mkdir -p "$3"
cp -a "$2.root/." "$3/"
SH
chmod +x "$TMP_DIR/fake-bin/dpkg-deb"
export PATH="$TMP_DIR/fake-bin:$PATH"
export OASIS7_TEST_RUNTIME_LOG="$TMP_DIR/runtime-commands.log"

cat >"$config_dir/public-testnet-governed-bootstrap-bundle-2026-06-06.json" <<EOF
{"git_commit":"abcdef1234567890abcdef1234567890abcdef12","runtime_build":{"git_commit":"abcdef1234567890abcdef1234567890abcdef12","sha256":"$runtime_sha","size_bytes":$runtime_size}}
EOF
cat >"$config_dir/public-testnet-governed-bootstrap-genesis-2026-06-06.json" <<'EOF'
{"governance_bootstrap_refs":{"governance_public_manifest_ref":"doc/testing/evidence/public-testnet-governance-public-signers.json"}}
EOF
cat >"$config_dir/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json" <<'EOF'
{"validators":[{"node_id":"triad-testnet-sequencer","finality_signer_public_key":"fixture-public-key","stake":100}]}
EOF
printf '[]\n' >"$config_dir/doc/testing/evidence/public-testnet-governance-public-signers.json"
printf 'NODE_ID=triad-testnet-sequencer\nGENESIS_VALIDATOR_REGISTRY_PATH=config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json\n' >"$config_dir/node.env"
printf '{"world":true}\n' >"$world_dir/snapshot.json"
printf '{"provenance":true}\n' >"$world_dir/world-generation-provenance.json"
printf '{"generated":true}\n' >"$world_dir/generated-scenario-world/manifest.json"

bootstrap_with() {
  OASIS7_TEST_ONLY=1 "$BOOTSTRAP" \
    --allow-test-stack-root \
    --test-root-prefix "$TMP_DIR" \
    --systemd-unit-dir "$systemd_dir" \
    "$@"
}

bootstrap() {
  bootstrap_with \
    --stack-root "$stack_root" \
    --package-deb "$package_deb" \
    --ops-tools-tar "$ops_tools_tar" \
    --config-dir "$config_dir" \
    --world-dir "$world_dir" \
    --node-id triad-testnet-sequencer \
    --service-name oasis7-triad-sequencer.service \
    --receipt "$receipt"
}

# Happy path: verified package, empty safe root, all C1 binaries, generated
# local key, installed-but-not-started unit, and public-only provenance.
bootstrap
stack_root_abs="$(python3 -c 'import os,sys; print(os.path.abspath(sys.argv[1]))' "$stack_root")"
test -L "$stack_root_abs/current"
for binary in oasis7_chain_runtime oasis7_world_repair_rebuild oasis7_governance_registry_import oasis7_governance_registry_audit; do
  test -x "$stack_root_abs/current/bin/$binary" || fail "missing C1 binary: $binary"
done
python3 "$ROOT_DIR/scripts/p2p-verify-linux-package-bundle.py" \
  "$stack_root_abs/releases/0.0.0+testnet.test.abcdef123456" \
  "0.0.0+testnet.test.abcdef123456" \
  "abcdef1234567890abcdef1234567890abcdef12" \
  "2737"
test -f "$stack_root_abs/config/node-keypair.toml"
test "$(python3 -c 'import os,stat,sys; print(oct(stat.S_IMODE(os.stat(sys.argv[1]).st_mode))[2:])' "$stack_root_abs/config/node-keypair.toml")" = "600"
test -f "$stack_root_abs/config/public-testnet-governed-bootstrap-bundle-2026-06-06.json"
test -f "$stack_root_abs/config/public-testnet-governed-bootstrap-genesis-2026-06-06.json"
test -f "$stack_root_abs/config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json"
test -f "$stack_root_abs/staged-world/snapshot.json"
test -f "$stack_root_abs/staged-world/world-generation-provenance.json"
test -f "$stack_root_abs/staged-world/generated-scenario-world/manifest.json"
test -f "$systemd_dir/oasis7-triad-sequencer.service"
test ! -e "$TMP_DIR/systemctl.log" || fail "test-only bootstrap must render only and never call systemctl"
jq -e \
  --arg root "$stack_root_abs" \
  --arg runtime "$stack_root_abs/current/bin/oasis7_chain_runtime" \
  '.schema_version == "oasis7.fresh_validator_host_bootstrap.v1"
   and .stack_root == $root
   and .runtime.path == $runtime
   and (.runtime.sha256 | test("^[0-9a-f]{64}$"))
   and (.node.public_key | type == "string" and length > 0)
   and ((tostring | test("private|secret"; "i")) | not)' \
  "$receipt" >/dev/null

# Validator-47 imports an identity staged by a prior no-start operation.  The
# import must preserve key bytes, validate the public finality identity against
# the governed registry before creating the target root, and use the runtime's
# read-only identity-receipt command instead of provision-identity.
validator47_config="$TMP_DIR/validator47-config"
validator47_identity="$TMP_DIR/validator47-identity"
validator47_stack_root="$TMP_DIR/validator47-opt/oasis7/p2p-testnet"
validator47_systemd_dir="$TMP_DIR/validator47-systemd"
validator47_receipt="$TMP_DIR/validator47-receipt.json"
mkdir -p "$validator47_config/doc/testing/evidence" "$validator47_identity" "$validator47_systemd_dir"
chmod 0700 "$validator47_identity"
cp -a "$config_dir/." "$validator47_config/"
cp "$ROOT_DIR/scripts/public-testnet-validator-triad-inventory.v1.json" \
  "$validator47_config/public-testnet-validator-triad-inventory.v1.json"
cp "$ROOT_DIR/doc/testing/evidence/public-testnet-governed-bootstrap-validator-triad-bootstrap-peers-2026-09-15.txt" \
  "$validator47_config/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt"
triad_inventory_sha256="$(shasum -a 256 "$validator47_config/public-testnet-validator-triad-inventory.v1.json" | awk '{print $1}')"
python3 - "$validator47_config/public-testnet-validator-triad-inventory.v1.json" \
  "$validator47_config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json" <<'PY'
import json
import pathlib
import sys

inventory = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
validators = []
for name in ("sequencer-204", "storage-205", "validator-47"):
    node = inventory["nodes"][name]
    entry = {
        "node_id": node["node_id"],
        "scheme": "ed25519",
        "finality_signer_public_key": node["finality_signer_public_key"],
        "stake": node["stake"],
    }
    if name == "validator-47":
        entry.update({field: node[field] for field in ("root_public_key", "finality_public_key", "libp2p_peer_id")})
    validators.append(entry)
payload = {
    "slot_id": "governance.finality.v1",
    "threshold": 2,
    "threshold_bps": 0,
    "quorum": {"numerator": 2, "denominator": 3, "total_stake": 300, "required_stake": 200},
    "governance": {"signer_count": 3, "threshold": 2, "threshold_bps": 6667},
    "validators": validators,
}
pathlib.Path(sys.argv[2]).write_text(json.dumps(payload, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
PY
validator47_finality_key="$(jq -r '.validators[] | select(.node_id == "triad-testnet-validator-47") | .finality_signer_public_key' "$validator47_config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json")"
validator47_root_key="$(jq -r '.nodes["validator-47"].root_public_key' "$validator47_config/public-testnet-validator-triad-inventory.v1.json")"
validator47_peer_id="$(jq -r '.nodes["validator-47"].libp2p_peer_id' "$validator47_config/public-testnet-validator-triad-inventory.v1.json")"
validator47_key="$validator47_identity/node-keypair.toml"
validator47_public_receipt="$validator47_identity/identity-receipt.json"
printf 'already-staged-validator-47-key\n' >"$validator47_key"
chmod 0600 "$validator47_key"
cat >"$validator47_public_receipt" <<EOF
{"schema_version":"oasis7.identity_provision.v1","node_id":"triad-testnet-validator-47","root_public_key":"$validator47_root_key","finality_public_key":"$validator47_finality_key","libp2p_peer_id":"$validator47_peer_id"}
EOF
chmod 0600 "$validator47_public_receipt"
validator47_key_sha256="$(shasum -a 256 "$validator47_key" | awk '{print $1}')"
validator47_receipt_sha256="$(shasum -a 256 "$validator47_public_receipt" | awk '{print $1}')"
cat >"$validator47_config/public-testnet-governed-bootstrap-manifest-2026-06-06.json" <<EOF
{"network_id":"oasis7-public-testnet-governed-20260606","chain_id":"oasis7-public-testnet-governed-20260606","tier":"public_testnet","deployment_inventory":{"ref":"scripts/public-testnet-validator-triad-inventory.v1.json","sha256":"$triad_inventory_sha256"},"bootstrap_peer_authority":{"ref":"public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt","sha256":"c7d0b977937adb5d27733ed0ad3e2212ccd0f3ac1b2273214e8cc57df110e5d6"},"deployment_validator_registry":{"ref":"config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json","sha256":"8bfb4411f3895ab5f1a2a3de1bcaa08ce97567202d4198444b323ef437a88f78","semantic_sha256":"aa6f6f7f367470d3b2c7282d489422d3eef14370446aa1fe8420fc94d776950d"}}
EOF
cat >"$validator47_config/node.env" <<EOF
NODE_ID=triad-testnet-validator-47
NODE_ROLE=storage
P2P_NODE_ROLE=full_storage
STATUS_BIND=0.0.0.0:6634
NODE_GOSSIP_BIND=0.0.0.0:6834
REPLICATION_NETWORK_LISTEN_ADDRS_CSV=/ip4/0.0.0.0/tcp/6834
CHECKPOINT_PROVIDER=1
FULL_STORAGE_PROVIDER=1
WORLD_ID=oasis7-public-testnet-governed-20260606
NETWORK_TIER_MANIFEST_PATH=config/public-testnet-governed-bootstrap-manifest-2026-06-06.json
GENESIS_VALIDATOR_REGISTRY_PATH=config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json
GENESIS_VALIDATOR_REGISTRY_SHA256=8bfb4411f3895ab5f1a2a3de1bcaa08ce97567202d4198444b323ef437a88f78
GENESIS_VALIDATOR_REGISTRY_SEMANTIC_SHA256=aa6f6f7f367470d3b2c7282d489422d3eef14370446aa1fe8420fc94d776950d
EXECUTION_WORLD_DIR=staged-world
DEPLOYMENT_INVENTORY_PATH=config/public-testnet-validator-triad-inventory.v1.json
DEPLOYMENT_INVENTORY_SHA256=$triad_inventory_sha256
BOOTSTRAP_PEER_PATH=config/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt
BOOTSTRAP_PEER_SHA256=c7d0b977937adb5d27733ed0ad3e2212ccd0f3ac1b2273214e8cc57df110e5d6
IDENTITY_KEY_PATH=config/node-keypair.toml
IDENTITY_RECEIPT_PATH=config/identity-receipt.json
IDENTITY_KEY_SHA256=$validator47_key_sha256
IDENTITY_RECEIPT_SHA256=$validator47_receipt_sha256
EOF

# A missing or drifted public replication listener must fail before any new
# stack root is materialized.  Keep the valid staged source for the happy path
# below and exercise each invalid variant in an isolated root.
validator47_node_env_valid="$TMP_DIR/validator47-node.env.valid"
cp "$validator47_config/node.env" "$validator47_node_env_valid"
export OASIS7_TEST_RUNTIME_PEER_ID="$validator47_peer_id"
sed -i.bak '/^REPLICATION_NETWORK_LISTEN_ADDRS_CSV=/d' "$validator47_config/node.env"
rm -f "$validator47_config/node.env.bak"
expect_fail "validator-47 node.env REPLICATION_NETWORK_LISTEN_ADDRS_CSV mismatch" env OASIS7_TEST_ONLY=1 "$BOOTSTRAP" \
  --allow-test-stack-root --test-root-prefix "$TMP_DIR" \
  --systemd-unit-dir "$TMP_DIR/validator47-missing-listener-systemd" \
  --stack-root "$TMP_DIR/validator47-missing-listener-opt/oasis7/p2p-testnet" \
  --package-deb "$package_deb" --ops-tools-tar "$ops_tools_tar" \
  --config-dir "$validator47_config" --world-dir "$world_dir" \
  --identity-dir "$validator47_identity" --node-id triad-testnet-validator-47 \
  --service-name oasis7-triad-validator-47.service \
  --receipt "$TMP_DIR/validator47-missing-listener-receipt.json"
test ! -e "$TMP_DIR/validator47-missing-listener-opt/oasis7/p2p-testnet"
cp "$validator47_node_env_valid" "$validator47_config/node.env"

sed -i.bak 's#^REPLICATION_NETWORK_LISTEN_ADDRS_CSV=.*#REPLICATION_NETWORK_LISTEN_ADDRS_CSV=/ip4/0.0.0.0/tcp/6835#' \
  "$validator47_config/node.env"
rm -f "$validator47_config/node.env.bak"
expect_fail "validator-47 node.env REPLICATION_NETWORK_LISTEN_ADDRS_CSV mismatch" env OASIS7_TEST_ONLY=1 "$BOOTSTRAP" \
  --allow-test-stack-root --test-root-prefix "$TMP_DIR" \
  --systemd-unit-dir "$TMP_DIR/validator47-drifted-listener-systemd" \
  --stack-root "$TMP_DIR/validator47-drifted-listener-opt/oasis7/p2p-testnet" \
  --package-deb "$package_deb" --ops-tools-tar "$ops_tools_tar" \
  --config-dir "$validator47_config" --world-dir "$world_dir" \
  --identity-dir "$validator47_identity" --node-id triad-testnet-validator-47 \
  --service-name oasis7-triad-validator-47.service \
  --receipt "$TMP_DIR/validator47-drifted-listener-receipt.json"
test ! -e "$TMP_DIR/validator47-drifted-listener-opt/oasis7/p2p-testnet"
cp "$validator47_node_env_valid" "$validator47_config/node.env"

baseline_runtime_commands="$(wc -l <"$OASIS7_TEST_RUNTIME_LOG" | tr -d ' ')"
export OASIS7_TEST_RUNTIME_PEER_ID="$validator47_peer_id"
OASIS7_TEST_ONLY=1 "$BOOTSTRAP" \
  --allow-test-stack-root \
  --test-root-prefix "$TMP_DIR" \
  --systemd-unit-dir "$validator47_systemd_dir" \
  --stack-root "$validator47_stack_root" \
  --package-deb "$package_deb" \
  --ops-tools-tar "$ops_tools_tar" \
  --config-dir "$validator47_config" \
  --world-dir "$world_dir" \
  --identity-dir "$validator47_identity" \
  --node-id triad-testnet-validator-47 \
  --service-name oasis7-triad-validator-47.service \
  --receipt "$validator47_receipt" >/dev/null
test -f "$validator47_stack_root/config/node-keypair.toml"
cmp -s "$validator47_identity/node-keypair.toml" "$validator47_stack_root/config/node-keypair.toml" \
  || fail "validator-47 imported key bytes changed"
test "$(python3 -c 'import os,stat,sys; print(oct(stat.S_IMODE(os.lstat(sys.argv[1]).st_mode))[2:])' "$validator47_stack_root/config/node-keypair.toml")" = 600
test "$(python3 -c 'import os,stat,sys; print(oct(stat.S_IMODE(os.lstat(sys.argv[1]).st_mode))[2:])' "$validator47_stack_root/config/identity-receipt.json")" = 600
jq -e \
  --arg key_sha "$validator47_key_sha256" \
  '.identity.mode == "imported"
   and .identity.ownership_valid == true
   and .identity.source_key_sha256 == $key_sha
   and .no_service_started == true
   and ((tostring | test("private_key|secret"; "i")) | not)' \
  "$validator47_receipt" >/dev/null
test "$(tail -n +$((baseline_runtime_commands + 1)) "$OASIS7_TEST_RUNTIME_LOG" | grep -c '^provision-identity$' || true)" = 0
test "$(tail -n +$((baseline_runtime_commands + 1)) "$OASIS7_TEST_RUNTIME_LOG" | grep -c '^identity-receipt$' || true)" = 2

# A public identity mismatch must fail before the new stack root is
# materialized; the already-running pair and the staged source remain intact.
cp "$validator47_public_receipt" "$TMP_DIR/validator47-public-receipt.valid"
sed "s/$validator47_finality_key/dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd/" \
  "$validator47_public_receipt" >"$TMP_DIR/validator47-public-receipt.bad"
mv "$TMP_DIR/validator47-public-receipt.bad" "$validator47_public_receipt"
chmod 0600 "$validator47_public_receipt"
validator47_bad_receipt_sha256="$(shasum -a 256 "$validator47_public_receipt" | awk '{print $1}')"
sed -i.bak "s/^IDENTITY_RECEIPT_SHA256=.*/IDENTITY_RECEIPT_SHA256=$validator47_bad_receipt_sha256/" \
  "$validator47_config/node.env"
rm -f "$validator47_config/node.env.bak"
expect_fail "does not match governed inventory" env OASIS7_TEST_ONLY=1 "$BOOTSTRAP" \
  --allow-test-stack-root --test-root-prefix "$TMP_DIR" \
  --systemd-unit-dir "$TMP_DIR/validator47-mismatch-systemd" \
  --stack-root "$TMP_DIR/validator47-mismatch-opt/oasis7/p2p-testnet" \
  --package-deb "$package_deb" --ops-tools-tar "$ops_tools_tar" \
  --config-dir "$validator47_config" --world-dir "$world_dir" \
  --identity-dir "$validator47_identity" --node-id triad-testnet-validator-47 \
  --service-name oasis7-triad-validator-47.service \
  --receipt "$TMP_DIR/validator47-mismatch-receipt.json"
test ! -e "$TMP_DIR/validator47-mismatch-opt/oasis7/p2p-testnet"
mv "$TMP_DIR/validator47-public-receipt.valid" "$validator47_public_receipt"
chmod 0600 "$validator47_public_receipt"
sed -i.bak "s/^IDENTITY_RECEIPT_SHA256=.*/IDENTITY_RECEIPT_SHA256=$validator47_receipt_sha256/" \
  "$validator47_config/node.env"
rm -f "$validator47_config/node.env.bak"

# The exact local paths required by existing pair-rebuild C1 preflight are
# present and the repair binary advertises the flag that C1 executes.
test -x "$stack_root_abs/current/bin/oasis7_chain_runtime"
test -x "$stack_root_abs/current/bin/oasis7_world_repair_rebuild"
test -x "$stack_root_abs/current/bin/oasis7_governance_registry_import"
test -x "$stack_root_abs/current/bin/oasis7_governance_registry_audit"
"$stack_root_abs/current/bin/oasis7_world_repair_rebuild" --help | grep -Fq -- '--generated-world-dir'

# Every archive member is classified before extraction. Symlinks, hardlinks,
# and special files must fail closed without creating an ops-tools path.
for member_kind in symlink hardlink fifo; do
  unsafe_tar="$TMP_DIR/unsafe-${member_kind}.tar.gz"
  python3 - "$unsafe_tar" "$member_kind" <<'PY'
import tarfile
import sys

archive_path, member_kind = sys.argv[1:]
member = tarfile.TarInfo("oasis7-linux-x64-ops-tools/bin/unsafe")
if member_kind == "symlink":
    member.type = tarfile.SYMTYPE
    member.linkname = "../../outside"
elif member_kind == "hardlink":
    member.type = tarfile.LNKTYPE
    member.linkname = "oasis7-linux-x64-ops-tools/bin/target"
else:
    member.type = tarfile.FIFOTYPE
with tarfile.open(archive_path, "w:gz") as archive:
    archive.addfile(member)
PY
  expect_fail "non-regular member" bootstrap_with \
    --stack-root "$TMP_DIR/unsafe-${member_kind}-root" \
    --package-deb "$package_deb" \
    --ops-tools-tar "$unsafe_tar" \
    --config-dir "$config_dir" \
    --world-dir "$world_dir" \
    --node-id triad-testnet-sequencer \
    --service-name oasis7-triad-sequencer.service \
    --receipt "$TMP_DIR/unsafe-${member_kind}-receipt.json"
  test ! -e "$TMP_DIR/unsafe-${member_kind}-root"
done

unsafe_tar="$TMP_DIR/unsafe-traversal.tar.gz"
python3 - "$unsafe_tar" <<'PY'
import io
import tarfile
import sys

member = tarfile.TarInfo("oasis7-linux-x64-ops-tools/../outside")
member.size = 1
with tarfile.open(sys.argv[1], "w:gz") as archive:
    archive.addfile(member, io.BytesIO(b"x"))
PY
expect_fail "unsafe member path" bootstrap_with \
  --stack-root "$TMP_DIR/unsafe-traversal-root" \
  --package-deb "$package_deb" \
  --ops-tools-tar "$unsafe_tar" \
  --config-dir "$config_dir" \
  --world-dir "$world_dir" \
  --node-id triad-testnet-sequencer \
  --service-name oasis7-triad-sequencer.service \
  --receipt "$TMP_DIR/unsafe-traversal-receipt.json"
test ! -e "$TMP_DIR/unsafe-traversal-root"

# The Debian player tree is validated in full before any checksum or ops-tool
# copy. A symlink hidden beside the runtime must therefore leave no bootstrap
# root behind.
ln -s "$TMP_DIR" "$bundle_dir/bin/unsafe-link"
expect_fail "symlink" bootstrap_with \
  --stack-root "$TMP_DIR/unsafe-deb-root" \
  --package-deb "$package_deb" \
  --ops-tools-tar "$ops_tools_tar" \
  --config-dir "$config_dir" \
  --world-dir "$world_dir" \
  --node-id triad-testnet-sequencer \
  --service-name oasis7-triad-sequencer.service \
  --receipt "$TMP_DIR/unsafe-deb-receipt.json"
rm "$bundle_dir/bin/unsafe-link"
test ! -e "$TMP_DIR/unsafe-deb-root"

# Fail closed before mutation for package integrity and unsafe filesystem input.
cp "$bundle_dir/BUILDINFO" "$TMP_DIR/valid-buildinfo"
rm "$bundle_dir/BUILDINFO"
expect_fail "BUILDINFO" bootstrap_with --stack-root "$TMP_DIR/absent-buildinfo-root" --package-deb "$package_deb" --ops-tools-tar "$ops_tools_tar" --config-dir "$config_dir" --world-dir "$world_dir" --node-id triad-testnet-sequencer --service-name oasis7-triad-sequencer.service --receipt "$TMP_DIR/absent-buildinfo-receipt.json"
mv "$TMP_DIR/valid-buildinfo" "$bundle_dir/BUILDINFO"

# Recreate the valid bundle and corrupt its declared runtime checksum.
printf 'commit=deadbeefdeadbeefdeadbeefdeadbeefdeadbeef\npackage_version=0.0.0+testnet.test.abcdef123456\nrun_id=2737\nplatform=linux-x64\n' >"$bundle_dir/BUILDINFO"
expect_fail "BUILDINFO" bootstrap_with --stack-root "$TMP_DIR/mismatched-buildinfo-root" --package-deb "$package_deb" --ops-tools-tar "$ops_tools_tar" --config-dir "$config_dir" --world-dir "$world_dir" --node-id triad-testnet-sequencer --service-name oasis7-triad-sequencer.service --receipt "$TMP_DIR/mismatched-buildinfo-receipt.json"

printf 'commit=abcdef1234567890abcdef1234567890abcdef12\npackage_version=0.0.0+testnet.test.abcdef123456\nrun_id=2737\nplatform=linux-x64\n' >"$bundle_dir/BUILDINFO"
printf '%064d  bin/oasis7_chain_runtime\n' 0 >"$bundle_dir/SHA256SUMS"
expect_fail "checksum" bootstrap_with --stack-root "$TMP_DIR/mismatched-checksum-root" --package-deb "$package_deb" --ops-tools-tar "$ops_tools_tar" --config-dir "$config_dir" --world-dir "$world_dir" --node-id triad-testnet-sequencer --service-name oasis7-triad-sequencer.service --receipt "$TMP_DIR/mismatched-checksum-receipt.json"
printf 'commit=abcdef1234567890abcdef1234567890abcdef12\npackage_version=0.0.0+testnet.test.abcdef123456\nrun_id=2737\nplatform=linux-x64\n' >"$bundle_dir/BUILDINFO"
(cd "$bundle_dir" && shasum -a 256 BUILDINFO bin/oasis7_chain_runtime >SHA256SUMS)

mkdir -p "$TMP_DIR/non-empty-root"
touch "$TMP_DIR/non-empty-root/sentinel"
expect_fail "empty" bootstrap_with --stack-root "$TMP_DIR/non-empty-root" --package-deb "$package_deb" --ops-tools-tar "$ops_tools_tar" --config-dir "$config_dir" --world-dir "$world_dir" --node-id triad-testnet-sequencer --service-name oasis7-triad-sequencer.service --receipt "$TMP_DIR/non-empty-receipt.json"

ln -s "$TMP_DIR/non-empty-root" "$TMP_DIR/symlink-root"
expect_fail "symlink" bootstrap_with --stack-root "$TMP_DIR/symlink-root" --package-deb "$package_deb" --ops-tools-tar "$ops_tools_tar" --config-dir "$config_dir" --world-dir "$world_dir" --node-id triad-testnet-sequencer --service-name oasis7-triad-sequencer.service --receipt "$TMP_DIR/symlink-receipt.json"

mkdir -p "$TMP_DIR/physical-root"
ln -s "$TMP_DIR/physical-root" "$TMP_DIR/symlink-parent"
expect_fail "symlink" bootstrap_with --stack-root "$TMP_DIR/symlink-parent/child" --package-deb "$package_deb" --ops-tools-tar "$ops_tools_tar" --config-dir "$config_dir" --world-dir "$world_dir" --node-id triad-testnet-sequencer --service-name oasis7-triad-sequencer.service --receipt "$TMP_DIR/symlink-component-receipt.json"

expect_fail "prefix" bootstrap_with --stack-root /tmp/unsafe-oasis7-root --package-deb "$package_deb" --ops-tools-tar "$ops_tools_tar" --config-dir "$config_dir" --world-dir "$world_dir" --node-id triad-testnet-sequencer --service-name oasis7-triad-sequencer.service --receipt "$TMP_DIR/unsafe-root-receipt.json"
expect_fail "descendant" bootstrap_with --stack-root "$TMP_DIR" --package-deb "$package_deb" --ops-tools-tar "$ops_tools_tar" --config-dir "$config_dir" --world-dir "$world_dir" --node-id triad-testnet-sequencer --service-name oasis7-triad-sequencer.service --receipt "$TMP_DIR/prefix-equality-receipt.json"
expect_fail "prefix" bootstrap_with --stack-root "$TMP_DIR/../oasis7-root-escape" --package-deb "$package_deb" --ops-tools-tar "$ops_tools_tar" --config-dir "$config_dir" --world-dir "$world_dir" --node-id triad-testnet-sequencer --service-name oasis7-triad-sequencer.service --receipt "$TMP_DIR/escape-receipt.json"
expect_fail "test" bootstrap_with --stack-root /opt/oasis7/p2p-testnet --package-deb "$package_deb" --ops-tools-tar "$ops_tools_tar" --config-dir "$config_dir" --world-dir "$world_dir" --node-id triad-testnet-sequencer --service-name oasis7-triad-sequencer.service --receipt "$TMP_DIR/production-root-in-test-receipt.json"

cp -a "$config_dir" "$TMP_DIR/malformed-config"
printf '{not-json\n' >"$TMP_DIR/malformed-config/public-testnet-governed-bootstrap-bundle-2026-06-06.json"
expect_fail "malformed" bootstrap_with --stack-root "$TMP_DIR/malformed-config-root" --package-deb "$package_deb" --ops-tools-tar "$ops_tools_tar" --config-dir "$TMP_DIR/malformed-config" --world-dir "$world_dir" --node-id triad-testnet-sequencer --service-name oasis7-triad-sequencer.service --receipt "$TMP_DIR/malformed-config-receipt.json"

mkdir -p "$TMP_DIR/missing-sidecar-world"
cp "$world_dir/snapshot.json" "$TMP_DIR/missing-sidecar-world/"
cp "$world_dir/world-generation-provenance.json" "$TMP_DIR/missing-sidecar-world/"
expect_fail "missing directory" bootstrap_with --stack-root "$TMP_DIR/missing-sidecar-root" --package-deb "$package_deb" --ops-tools-tar "$ops_tools_tar" --config-dir "$config_dir" --world-dir "$TMP_DIR/missing-sidecar-world" --node-id triad-testnet-sequencer --service-name oasis7-triad-sequencer.service --receipt "$TMP_DIR/missing-sidecar-receipt.json"

expect_fail "service" bootstrap_with --stack-root "$TMP_DIR/malformed-unit-root" --package-deb "$package_deb" --ops-tools-tar "$ops_tools_tar" --config-dir "$config_dir" --world-dir "$world_dir" --node-id triad-testnet-sequencer --service-name '../unsafe.service' --receipt "$TMP_DIR/malformed-unit-receipt.json"

expect_fail "OASIS7_TEST_ONLY" "$BOOTSTRAP" --allow-test-stack-root --test-root-prefix "$TMP_DIR" --systemd-unit-dir "$systemd_dir" --stack-root "$TMP_DIR/missing-test-gate-root" --package-deb "$package_deb" --ops-tools-tar "$ops_tools_tar" --config-dir "$config_dir" --world-dir "$world_dir" --node-id triad-testnet-sequencer --service-name oasis7-triad-sequencer.service --receipt "$TMP_DIR/missing-test-gate-receipt.json"

printf 'ok: fresh validator host bootstrap contract\n'
