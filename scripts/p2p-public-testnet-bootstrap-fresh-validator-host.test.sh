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
  cat >"$bundle_dir/bin/$binary" <<'SH'
#!/usr/bin/env bash
if [[ "${1:-}" == "--help" ]]; then
  printf '%s\n' '--generated-world-dir'
fi
SH
  chmod +x "$bundle_dir/bin/$binary"
done
cat >"$bundle_dir/bin/oasis7_chain_runtime" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

[[ "${1:-}" == "provision-identity" ]] || exit 64
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
mkdir -p "$config_dir"
key_path="$config_dir/node-keypair.toml"
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
cp -a "$bundle_dir/bin/oasis7_world_repair_rebuild" "$ops_bundle_dir/bin/"
cp -a "$bundle_dir/bin/oasis7_governance_registry_import" "$ops_bundle_dir/bin/"
cp -a "$bundle_dir/bin/oasis7_governance_registry_audit" "$ops_bundle_dir/bin/"
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
