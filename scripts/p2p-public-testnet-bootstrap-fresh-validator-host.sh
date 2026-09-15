#!/usr/bin/env bash
set -euo pipefail

# Provision a never-before-used public-testnet validator host.  This is
# deliberately a no-start operation: the service is rendered, but activation
# remains an explicit post-bootstrap operator action after topology review.

readonly PRODUCTION_ROOT=/opt/oasis7/p2p-testnet
readonly PRODUCTION_UNIT=/etc/systemd/system/oasis7-triad-sequencer.service
readonly SERVICE_NAME=oasis7-triad-sequencer.service
readonly VALIDATOR_47_PRODUCTION_UNIT=/etc/systemd/system/oasis7-triad-validator-47.service
readonly VALIDATOR_47_SERVICE_NAME=oasis7-triad-validator-47.service
readonly TRIAD_INVENTORY_RELATIVE=scripts/public-testnet-validator-triad-inventory.v1.json
readonly TRIAD_INVENTORY_FILE=public-testnet-validator-triad-inventory.v1.json
readonly VALIDATOR_47_NODE_ID=triad-testnet-validator-47
# Inventory role is validator.  The runtime role is storage and its independent
# P2P provider role is full_storage; PoS identity comes from consensus truth.
readonly VALIDATOR_47_INVENTORY_ROLE=validator # inventory NODE_ROLE=validator
readonly VALIDATOR_47_RUNTIME_NODE_ROLE=storage
readonly VALIDATOR_47_P2P_NODE_ROLE=full_storage
readonly VALIDATOR_47_STATUS_BIND=0.0.0.0:6634
readonly VALIDATOR_47_NODE_GOSSIP_BIND=0.0.0.0:6834
readonly VALIDATOR_47_WORLD_ID=oasis7-public-testnet-governed-20260606
readonly VALIDATOR_47_MANIFEST_PATH=config/public-testnet-governed-bootstrap-manifest-2026-06-06.json
readonly VALIDATOR_47_REGISTRY_PATH=config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json
readonly VALIDATOR_47_EXECUTION_WORLD_DIR=staged-world
readonly VALIDATOR_47_IDENTITY_KEY_FILE=node-keypair.toml
readonly VALIDATOR_47_IDENTITY_RECEIPT_FILE=identity-receipt.json
readonly BUNDLE_DIR_NAME=oasis7-linux-x64
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SCRIPT_DIR

usage() {
  cat <<'EOF'
Usage:
  ./scripts/p2p-public-testnet-bootstrap-fresh-validator-host.sh \
    --package-deb <oasis7-linux-x64.deb> \
    --ops-tools-tar <oasis7-linux-x64-ops-tools.tar.gz> \
    --config-dir <governed stage config directory> \
    --world-dir <generated world directory> \
    [--identity-dir <already-staged validator-47 identity directory>] \
    --node-id triad-testnet-sequencer \
    --receipt <public receipt path>

Production uses exactly /opt/oasis7/p2p-testnet and
/etc/systemd/system/oasis7-triad-sequencer.service for the existing pair, or
/etc/systemd/system/oasis7-triad-validator-47.service for validator-47.  It
renders and installs the unit but never enables or starts it.

Test-only override (both gates are required):
  OASIS7_TEST_ONLY=1 ... --allow-test-stack-root --test-root-prefix <absolute>
  [--stack-root <strict descendant>] [--systemd-unit-dir <strict descendant>]
EOF
}

die() { printf 'error: %s\n' "$*" >&2; exit 1; }

require_command() { command -v "$1" >/dev/null 2>&1 || die "missing command: $1"; }
require_file() { [[ -f "$1" ]] || die "missing file: $1"; }
require_dir() { [[ -d "$1" ]] || die "missing directory: $1"; }
sha256_file() { shasum -a 256 "$1" | awk '{print $1}'; }
file_size() { wc -c <"$1" | tr -d ' '; }
public_file_json() {
  local path=$1
  if [[ ! -f "$path" ]]; then
    printf 'null\n'
    return
  fi
  jq -n --arg path "$path" --arg sha256 "$(sha256_file "$path")" \
    --argjson size "$(file_size "$path")" '{path:$path,sha256:$sha256,size_bytes:$size}'
}

key_owner_json() {
  python3 - "$1" <<'PY'
import hashlib
import json
import os
import stat
import sys
p = sys.argv[1]
s = os.stat(p)
print(json.dumps({"path": p, "mode": f"{stat.S_IMODE(s.st_mode):04o}", "uid": s.st_uid, "gid": s.st_gid}))
PY
}

safe_cleanup_root=0
bootstrap_complete=0
installed_unit_path=""
cleanup() {
  local status=$?
  if [[ $safe_cleanup_root -eq 1 && $bootstrap_complete -eq 0 ]]; then
    # This is set only after the exact empty target was created in this run.
    rm -rf -- "$stack_root"
    if [[ -n "$installed_unit_path" && -f "$installed_unit_path" ]]; then
      rm -f -- "$installed_unit_path"
      if [[ ${OASIS7_TEST_ONLY:-} != 1 ]] && command -v systemctl >/dev/null 2>&1; then
        systemctl daemon-reload >/dev/null 2>&1 || true
      fi
    fi
  fi
  [[ -z "${work_dir:-}" ]] || rm -rf -- "$work_dir"
  exit "$status"
}

absolute_path() {
  python3 - "$1" <<'PY'
import os
import sys
print(os.path.abspath(os.path.expanduser(sys.argv[1])))
PY
}

# Reject every symlink component, including one above a path that does not yet
# exist.  resolve() is intentionally not used as a safety decision.
assert_physical_path() {
  local candidate=$1 component parent
  [[ "$candidate" == /* ]] || die "path must be absolute: $candidate"
  component=/
  IFS=/ read -r -a parts <<<"${candidate#/}"
  for parent in "${parts[@]}"; do
    [[ -n "$parent" ]] || continue
    component="${component%/}/$parent"
    [[ ! -L "$component" ]] || die "symlink path component is forbidden: $component"
  done
}

assert_strict_descendant() {
  local candidate=$1 prefix=$2 label=$3
  [[ "$prefix" == /* && "$candidate" == /* ]] || die "$label must be absolute"
  [[ "$candidate" == "$prefix"/* ]] || die "$label must be a strict descendant of prefix: $prefix"
}

assert_no_symlink_below_prefix() {
  local candidate=$1 prefix=$2 component
  assert_strict_descendant "$candidate" "$prefix" "path"
  component="$prefix"
  local suffix=${candidate#"$prefix"/}
  IFS=/ read -r -a parts <<<"$suffix"
  for part in "${parts[@]}"; do
    [[ -n "$part" ]] || continue
    component="$component/$part"
    [[ ! -L "$component" ]] || die "symlink path component is forbidden: $component"
  done
}

safe_tar_extract() {
  local archive=$1 destination=$2
  python3 "$SCRIPT_DIR/p2p-safe-extract-tar.py" "$archive" "$destination" \
    || die "cannot safely extract ops-tools archive"
}

safe_deb_extract() {
  local package=$1 destination=$2
  require_command dpkg-deb
  dpkg-deb --extract "$package" "$destination" || die "cannot extract Debian package"
  python3 "$SCRIPT_DIR/p2p-safe-validate-deb-tree.py" "$destination" "opt/oasis7" \
    || die "extracted Debian package failed the symlink/non-regular/path containment checks"
}

verify_bundle() {
  local bundle_root=$1 config=$2 runtime_sum expected_sum bundle_commit config_commit
  require_file "$bundle_root/BUILDINFO"
  require_file "$bundle_root/SHA256SUMS"
  require_file "$bundle_root/bin/oasis7_chain_runtime"
  local build_version build_run_id
  build_version=$(sed -n 's/^package_version=//p' "$bundle_root/BUILDINFO" | head -n1)
  build_run_id=$(sed -n 's/^run_id=//p' "$bundle_root/BUILDINFO" | head -n1)
  bundle_commit=$(sed -n 's/^commit=//p' "$bundle_root/BUILDINFO" | head -n1)
  python3 "$SCRIPT_DIR/p2p-verify-linux-package-bundle.py" \
    "$bundle_root" "$build_version" "$bundle_commit" "$build_run_id" \
    || die "bundle checksum verification failed (BUILDINFO/SHA256SUMS)"
  [[ -x "$bundle_root/bin/oasis7_chain_runtime" ]] || die "bundle missing required executable: oasis7_chain_runtime"
  grep -Eq '^commit=[0-9a-f]{40}$' "$bundle_root/BUILDINFO" || die "BUILDINFO missing valid commit"
  grep -Eq '^package_version=.+$' "$bundle_root/BUILDINFO" || die "BUILDINFO missing package_version"
  grep -Eq '^run_id=.+$' "$bundle_root/BUILDINFO" || die "BUILDINFO missing run_id"
  config_commit=$(jq -r '.git_commit // empty' "$config/public-testnet-governed-bootstrap-bundle-2026-06-06.json")
  [[ "$bundle_commit" == "$config_commit" ]] || die "BUILDINFO commit does not match governed config"
  runtime_sum=$(shasum -a 256 "$bundle_root/bin/oasis7_chain_runtime" | awk '{print $1}')
  expected_sum=$(jq -r '.runtime_build.sha256 // empty' "$config/public-testnet-governed-bootstrap-bundle-2026-06-06.json")
  [[ "$runtime_sum" == "$expected_sum" ]] || die "BUILDINFO governed runtime checksum mismatch"
}

validate_validator_47_stage() {
  local inventory_path="$config_dir/$TRIAD_INVENTORY_FILE"
  require_file "$inventory_path"
  [[ ! -L "$inventory_path" ]] || die "triad inventory authority must not be a symlink"
  triad_inventory_sha256=$(sha256_file "$inventory_path")
  python3 - "$inventory_path" "$config_dir/node.env" \
    "$config_dir/public-testnet-governed-bootstrap-manifest-2026-06-06.json" \
    "$config_dir/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json" \
    "$triad_inventory_sha256" <<'PY'
import hashlib
import json
import pathlib
import sys

inventory_path = pathlib.Path(sys.argv[1])
env_path = pathlib.Path(sys.argv[2])
manifest_path = pathlib.Path(sys.argv[3])
registry_path = pathlib.Path(sys.argv[4])
expected_inventory_sha256 = sys.argv[5]
actual_inventory_sha256 = hashlib.sha256(inventory_path.read_bytes()).hexdigest()
if actual_inventory_sha256 != expected_inventory_sha256:
    raise SystemExit("triad inventory digest mismatch")

inventory = json.loads(inventory_path.read_text(encoding="utf-8"))
if inventory.get("schema_version") != "oasis7.public_testnet_validator_triad_inventory.v1":
    raise SystemExit("triad inventory schema mismatch")
if inventory.get("network_tier") != "public_testnet" or inventory.get("topology") != "three_equal_validator":
    raise SystemExit("triad inventory network/topology mismatch")
target = inventory.get("nodes", {}).get("validator-47")
if not isinstance(target, dict):
    raise SystemExit("triad inventory validator-47 binding missing")
expected_target = {
    "node_id": "triad-testnet-validator-47",
    "roles": ["validator", "checkpoint_provider", "full_storage_provider"],
    "service": "oasis7-triad-validator-47.service",
    "ports": ["6634", "6834"],
}
for key, expected in expected_target.items():
    if target.get(key) != expected:
        raise SystemExit(f"validator-47 inventory {key} mismatch")

values = {}
for line_number, raw_line in enumerate(env_path.read_text(encoding="utf-8").splitlines(), 1):
    line = raw_line.strip()
    if not line or line.startswith("#"):
        continue
    if "=" not in line:
        raise SystemExit(f"validator-47 node.env malformed line {line_number}")
    key, value = line.split("=", 1)
    if not key or key in values:
        raise SystemExit(f"validator-47 node.env duplicate or empty key at line {line_number}")
    values[key] = value
expected_env = {
    "NODE_ID": "triad-testnet-validator-47",
    "NODE_ROLE": "storage",
    "P2P_NODE_ROLE": "full_storage",
    "STATUS_BIND": "0.0.0.0:6634",
    "NODE_GOSSIP_BIND": "0.0.0.0:6834",
    "CHECKPOINT_PROVIDER": "1",
    "FULL_STORAGE_PROVIDER": "1",
    "WORLD_ID": "oasis7-public-testnet-governed-20260606",
    "NETWORK_TIER_MANIFEST_PATH": "config/public-testnet-governed-bootstrap-manifest-2026-06-06.json",
    "GENESIS_VALIDATOR_REGISTRY_PATH": "config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json",
    "EXECUTION_WORLD_DIR": "staged-world",
    "DEPLOYMENT_INVENTORY_PATH": "config/public-testnet-validator-triad-inventory.v1.json",
    "DEPLOYMENT_INVENTORY_SHA256": actual_inventory_sha256,
}
for key, expected in expected_env.items():
    if values.get(key) != expected:
        raise SystemExit(f"validator-47 node.env {key} mismatch")

manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
if manifest.get("network_id") != "oasis7-public-testnet-governed-20260606":
    raise SystemExit("validator-47 manifest network identity mismatch")
if manifest.get("chain_id") != "oasis7-public-testnet-governed-20260606":
    raise SystemExit("validator-47 manifest chain identity mismatch")
if manifest.get("tier") != "public_testnet":
    raise SystemExit("validator-47 manifest network tier mismatch")
inventory_binding = manifest.get("deployment_inventory")
if inventory_binding != {"ref": "scripts/public-testnet-validator-triad-inventory.v1.json", "sha256": actual_inventory_sha256}:
    raise SystemExit("validator-47 manifest inventory binding mismatch")

registry = json.loads(registry_path.read_text(encoding="utf-8"))
validators = registry.get("validators")
if not isinstance(validators, list):
    raise SystemExit("validator-47 registry validators missing")
if len(validators) != 3:
    raise SystemExit("validator-47 registry validator count mismatch")
if sorted(item.get("node_id") for item in validators) != sorted(
    [inventory["nodes"][name]["node_id"] for name in ("sequencer-204", "storage-205", "validator-47")]
):
    raise SystemExit("validator-47 registry identity mismatch")
if any(item.get("stake") != 100 for item in validators):
    raise SystemExit("validator-47 registry stake mismatch")
PY
}

validate_validator_47_identity_source() {
  [[ "$node_id" == "$VALIDATOR_47_NODE_ID" ]] || return 0
  [[ -n "$identity_dir" ]] || die "validator-47 requires --identity-dir for an already-staged identity"
  [[ ! -e "$config_dir/$VALIDATOR_47_IDENTITY_KEY_FILE" ]] \
    || die "validator-47 config stage contains a stale node-keypair.toml; use --identity-dir"
  python3 - "$identity_dir" "$config_dir/node.env" "$config_dir/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json" <<'PY'
import hashlib
import json
import os
import stat
import sys
from pathlib import Path

identity_dir = Path(sys.argv[1])
node_env_path = Path(sys.argv[2])
registry_path = Path(sys.argv[3])
current = Path(identity_dir.anchor or "/")
for component in identity_dir.parts[1:]:
    current /= component
    if current.is_symlink():
        # macOS exposes /var and /tmp as stable aliases below /private; these
        # platform-owned aliases are safe, but caller-controlled redirects are
        # not accepted.
        resolved = os.path.realpath(current)
        if str(current) not in {"/var", "/tmp"} or not resolved.startswith("/private/"):
            raise SystemExit(f"validator-47 identity source path contains a symlink: {current}")
if not identity_dir.is_dir():
    raise SystemExit("validator-47 identity source must be a directory")
directory_metadata = os.lstat(identity_dir)
if stat.S_IMODE(directory_metadata.st_mode) != 0o700:
    raise SystemExit("validator-47 identity source directory must have mode 0700")

def secure_regular(path: Path, label: str) -> os.stat_result:
    metadata = os.lstat(path)
    if not stat.S_ISREG(metadata.st_mode):
        raise SystemExit(f"validator-47 {label} must be a regular file")
    if stat.S_IMODE(metadata.st_mode) != 0o600:
        raise SystemExit(f"validator-47 {label} must have mode 0600")
    if (metadata.st_uid, metadata.st_gid) != (directory_metadata.st_uid, directory_metadata.st_gid):
        raise SystemExit(f"validator-47 {label} ownership differs from identity directory")
    return metadata

key_path = identity_dir / "node-keypair.toml"
receipt_path = identity_dir / "identity-receipt.json"
key_metadata = secure_regular(key_path, "identity source key")
receipt_metadata = secure_regular(receipt_path, "identity source receipt")
try:
    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    registry = json.loads(registry_path.read_text(encoding="utf-8"))
except (OSError, UnicodeError, ValueError) as error:
    raise SystemExit(f"validator-47 identity or registry JSON is malformed: {error.__class__.__name__}")
if receipt.get("schema_version") != "oasis7.identity_provision.v1":
    raise SystemExit("validator-47 staged identity receipt schema mismatch")
if receipt.get("node_id") != "triad-testnet-validator-47":
    raise SystemExit("validator-47 staged identity receipt node mismatch")
for field in ("root_public_key", "finality_public_key", "libp2p_peer_id"):
    if not isinstance(receipt.get(field), str) or not receipt[field].strip():
        raise SystemExit(f"validator-47 staged identity receipt missing {field}")
if "private_key" in json.dumps(receipt, ensure_ascii=False).lower():
    raise SystemExit("validator-47 public identity receipt must not contain private key material")
node_env_values = {}
for line_number, raw_line in enumerate(node_env_path.read_text(encoding="utf-8").splitlines(), 1):
    line = raw_line.strip()
    if not line or line.startswith("#"):
        continue
    if "=" not in line:
        raise SystemExit(f"validator-47 node.env malformed line {line_number}")
    key, value = line.split("=", 1)
    if not key or key in node_env_values:
        raise SystemExit(f"validator-47 node.env duplicate or empty key at line {line_number}")
    node_env_values[key] = value
if node_env_values.get("IDENTITY_KEY_PATH") != "config/node-keypair.toml":
    raise SystemExit("validator-47 node.env identity key path mismatch")
if node_env_values.get("IDENTITY_RECEIPT_PATH") != "config/identity-receipt.json":
    raise SystemExit("validator-47 node.env identity receipt path mismatch")
if node_env_values.get("IDENTITY_KEY_SHA256") != hashlib.sha256(key_path.read_bytes()).hexdigest():
    raise SystemExit("validator-47 node.env identity key digest mismatch")
if node_env_values.get("IDENTITY_RECEIPT_SHA256") != hashlib.sha256(receipt_path.read_bytes()).hexdigest():
    raise SystemExit("validator-47 node.env identity receipt digest mismatch")
matches = [item for item in registry.get("validators", []) if item.get("node_id") == receipt["node_id"]]
if len(matches) != 1:
    raise SystemExit("validator-47 staged identity node is absent or duplicated in governed registry")
if matches[0].get("finality_signer_public_key", "").lower() != receipt["finality_public_key"].lower():
    raise SystemExit("validator-47 staged public identity does not match governed registry")
PY
}

render_unit() {
  local template=$1 output=$2 stack_root=$3
  sed "s|@STACK_ROOT@|$stack_root|g" "$template" >"$output"
  chmod 0644 "$output"
}

ensure_service_account() {
  [[ ${EUID:-$(id -u)} -eq 0 ]] || die "production bootstrap must run as root"
  require_command getent
  require_command useradd
  require_command groupadd
  if ! getent group oasis7-testnet >/dev/null; then
    groupadd --system oasis7-testnet
  fi
  if ! getent passwd oasis7-testnet >/dev/null; then
    useradd --system --gid oasis7-testnet --no-create-home \
      --home-dir /nonexistent --shell /usr/sbin/nologin oasis7-testnet
  fi
  local passwd_entry shell home primary_gid group_gid service_uid
  passwd_entry=$(getent passwd oasis7-testnet)
  shell=$(cut -d: -f7 <<<"$passwd_entry")
  home=$(cut -d: -f6 <<<"$passwd_entry")
  primary_gid=$(id -g oasis7-testnet)
  service_uid=$(id -u oasis7-testnet)
  group_gid=$(getent group oasis7-testnet | cut -d: -f3)
  [[ "$home" == /nonexistent && ( "$shell" == /usr/sbin/nologin || "$shell" == /sbin/nologin ) ]] \
    || die "oasis7-testnet must be no-home and no-login"
  [[ "$primary_gid" == "$group_gid" ]] || die "oasis7-testnet must use its fixed primary group"
  [[ "$service_uid" -lt 1000 ]] || die "oasis7-testnet must be a system account"
}

stack_root=$PRODUCTION_ROOT
systemd_unit_dir=/etc/systemd/system
service_name=$SERVICE_NAME
package_deb=""
ops_tools_tar=""
config_dir=""
world_dir=""
identity_dir=""
node_id=""
receipt=""
allow_test_stack_root=0
test_root_prefix=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --stack-root) stack_root=${2:-}; shift 2 ;;
    --package-deb) package_deb=${2:-}; shift 2 ;;
    --ops-tools-tar) ops_tools_tar=${2:-}; shift 2 ;;
    --config-dir) config_dir=${2:-}; shift 2 ;;
    --world-dir) world_dir=${2:-}; shift 2 ;;
    --identity-dir) identity_dir=${2:-}; shift 2 ;;
    --node-id) node_id=${2:-}; shift 2 ;;
    --service-name) service_name=${2:-}; shift 2 ;;
    --receipt) receipt=${2:-}; shift 2 ;;
    --allow-test-stack-root) allow_test_stack_root=1; shift ;;
    --test-root-prefix) test_root_prefix=${2:-}; shift 2 ;;
    --systemd-unit-dir) systemd_unit_dir=${2:-}; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown option: $1" ;;
  esac
done

for required in package_deb ops_tools_tar config_dir world_dir node_id receipt; do
  [[ -n ${!required} ]] || die "missing required option: --${required//_/-}"
done
[[ "$node_id" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]] || die "invalid node-id"
for command in tar shasum jq python3 install find; do require_command "$command"; done

stack_root=$(absolute_path "$stack_root")
systemd_unit_dir=$(absolute_path "$systemd_unit_dir")
package_deb=$(absolute_path "$package_deb")
ops_tools_tar=$(absolute_path "$ops_tools_tar")
config_dir=$(absolute_path "$config_dir")
world_dir=$(absolute_path "$world_dir")
if [[ -n "$identity_dir" ]]; then
  identity_dir=$(python3 -c 'import os,sys; print(os.path.abspath(os.path.expanduser(sys.argv[1])))' "$identity_dir")
fi
receipt=$(absolute_path "$receipt")
if [[ ${OASIS7_TEST_ONLY:-} == 1 ]]; then
  [[ $allow_test_stack_root -eq 1 && -n "$test_root_prefix" ]] \
    || die "test mode requires --allow-test-stack-root and --test-root-prefix"
  test_root_prefix=$(absolute_path "$test_root_prefix")
  # A macOS /tmp may itself traverse the platform-owned /var symlink.  The
  # supplied test prefix is the trust boundary; every component below it must
  # still be physical, so a caller cannot redirect either writable target.
  assert_no_symlink_below_prefix "$stack_root" "$test_root_prefix"
  assert_no_symlink_below_prefix "$systemd_unit_dir" "$test_root_prefix"
  [[ "$stack_root" != "$PRODUCTION_ROOT" ]] || die "test mode cannot use production root"
  assert_strict_descendant "$receipt" "$test_root_prefix" "test receipt"
else
  [[ $allow_test_stack_root -eq 0 && -z "$test_root_prefix" ]] || die "test-only flags require OASIS7_TEST_ONLY=1"
  [[ "$stack_root" == "$PRODUCTION_ROOT" ]] || die "production stack root must be $PRODUCTION_ROOT"
  if [[ "$node_id" == triad-testnet-validator-47 ]]; then
    [[ "$service_name" == "$VALIDATOR_47_SERVICE_NAME" ]] || die "validator-47 service must be exactly $VALIDATOR_47_SERVICE_NAME"
    production_unit=$VALIDATOR_47_PRODUCTION_UNIT
  else
    [[ "$service_name" == "$SERVICE_NAME" ]] || die "service must be exactly $SERVICE_NAME"
    production_unit=$PRODUCTION_UNIT
  fi
  [[ "$systemd_unit_dir/$service_name" == "$production_unit" ]] || die "production unit must be $production_unit"
  assert_physical_path "$stack_root"
  assert_physical_path "$systemd_unit_dir"
  [[ "$receipt" == "$stack_root/evidence/fresh-validator-host-bootstrap-receipt.json" ]] \
    || die "production receipt must be exactly $stack_root/evidence/fresh-validator-host-bootstrap-receipt.json"
  require_command systemctl
  [[ ! -e "$production_unit" ]] || die "fresh host already has service unit: $production_unit"
  if systemctl is-active --quiet "$service_name"; then
    die "service is active; fresh bootstrap never stops or replaces an active service"
  fi
  ensure_service_account
fi

if [[ "$node_id" == triad-testnet-validator-47 ]]; then
  [[ "$service_name" == "$VALIDATOR_47_SERVICE_NAME" ]] || die "validator-47 service must be exactly $VALIDATOR_47_SERVICE_NAME"
else
  [[ "$service_name" == "$SERVICE_NAME" ]] || die "service must be exactly $SERVICE_NAME"
fi

require_file "$package_deb"; require_file "$ops_tools_tar"; require_dir "$config_dir"; require_dir "$world_dir"
for source in \
  public-testnet-governed-bootstrap-bundle-2026-06-06.json \
  public-testnet-governed-bootstrap-genesis-2026-06-06.json \
  public-testnet-governed-bootstrap-validator-registry-2026-06-06.json \
  node.env; do require_file "$config_dir/$source"; done
for source in snapshot.json world-generation-provenance.json; do require_file "$world_dir/$source"; done
require_dir "$world_dir/generated-scenario-world"
jq -e . "$config_dir/public-testnet-governed-bootstrap-bundle-2026-06-06.json" >/dev/null \
  || die "config bundle JSON is malformed"
jq -e . "$config_dir/public-testnet-governed-bootstrap-genesis-2026-06-06.json" >/dev/null \
  || die "config genesis JSON is malformed"
jq -e . "$config_dir/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json" >/dev/null \
  || die "config validator registry JSON is malformed"
if [[ ${OASIS7_TEST_ONLY:-} != 1 ]]; then
  require_file "$config_dir/public-testnet-governed-bootstrap-manifest-2026-06-06.json"
  require_file "$config_dir/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt"
  jq -e . "$config_dir/public-testnet-governed-bootstrap-manifest-2026-06-06.json" >/dev/null \
    || die "config manifest JSON is malformed"
fi

if [[ "$node_id" == "$VALIDATOR_47_NODE_ID" ]]; then
  [[ "$service_name" == "$VALIDATOR_47_SERVICE_NAME" ]] \
    || die "validator-47 service binding mismatch"
  validate_validator_47_stage
  validate_validator_47_identity_source
else
  [[ -z "$identity_dir" ]] || die "--identity-dir is only valid for validator-47"
  triad_inventory_sha256=""
fi

[[ ! -e "$stack_root" ]] || [[ -d "$stack_root" && -z "$(find "$stack_root" -mindepth 1 -print -quit)" ]] \
  || die "stack root must be empty: $stack_root"

template_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
template="$template_dir/p2p-public-testnet-triad-sequencer.service"
start_script="$template_dir/p2p-triad-node-start.sh"
require_file "$template"
require_file "$start_script"
work_dir=$(mktemp -d "${TMPDIR:-/tmp}/oasis7-fresh-validator.XXXXXX")
trap cleanup EXIT
safe_deb_extract "$package_deb" "$work_dir/deb-root"
bundle_root="$work_dir/deb-root/opt/oasis7"
[[ -d "$bundle_root" ]] || die "Debian package missing /opt/oasis7 player bundle"
safe_tar_extract "$ops_tools_tar" "$work_dir"
ops_bundle_root="$work_dir/${BUNDLE_DIR_NAME}-ops-tools"
require_file "$ops_bundle_root/.oasis7-ops-tools-manifest.json"
require_file "$ops_bundle_root/SHA256SUMS"
(
  cd "$ops_bundle_root"
  shasum -a 256 -c SHA256SUMS >/dev/null || die "ops-tools checksum verification failed"
)
for binary in oasis7_world_repair_rebuild oasis7_governance_registry_import oasis7_governance_registry_audit service-readback; do
  [[ -x "$ops_bundle_root/bin/$binary" ]] || die "ops-tools archive missing executable: $binary"
done
require_dir "$bundle_root"
verify_bundle "$bundle_root" "$config_dir"
identity_import_mode="generated"
identity_source_key_sha256=""
identity_source_receipt_sha256=""
staged_identity_receipt_json='null'
if [[ "$node_id" == "$VALIDATOR_47_NODE_ID" ]]; then
  identity_import_mode="imported"
  identity_source_key_sha256=$(sha256_file "$identity_dir/$VALIDATOR_47_IDENTITY_KEY_FILE")
  identity_source_receipt_sha256=$(sha256_file "$identity_dir/$VALIDATOR_47_IDENTITY_RECEIPT_FILE")
  [[ -x "$bundle_root/bin/oasis7_chain_runtime" ]] \
    || die "validator-47 identity readback runtime is not executable"
  staged_identity_receipt_json=$("$bundle_root/bin/oasis7_chain_runtime" identity-receipt \
    --config-dir "$identity_dir" --node-id "$node_id") \
    || die "validator-47 staged identity readback failed before materialization"
  python3 - "$identity_dir" "$staged_identity_receipt_json" "$identity_source_key_sha256" \
    "$config_dir/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json" <<'PY'
import json
import sys
from pathlib import Path

identity_dir = Path(sys.argv[1])
runtime_receipt = json.loads(sys.argv[2])
expected_key_sha = sys.argv[3]
registry_path = Path(sys.argv[4])
if runtime_receipt.get("schema_version") != "oasis7.identity_receipt.v1":
    raise SystemExit("validator-47 runtime identity readback schema mismatch")
if runtime_receipt.get("node_id") != "triad-testnet-validator-47":
    raise SystemExit("validator-47 runtime identity readback node mismatch")
if runtime_receipt.get("key_sha256") != expected_key_sha:
    raise SystemExit("validator-47 runtime identity key digest mismatch")
if runtime_receipt.get("key_mode") not in (0o600, 384):
    raise SystemExit("validator-47 runtime identity key mode mismatch")
receipt = json.loads((identity_dir / "identity-receipt.json").read_text(encoding="utf-8"))
if runtime_receipt.get("peer_id") != receipt.get("libp2p_peer_id"):
    raise SystemExit("validator-47 runtime identity peer id mismatch")
registry = json.loads(registry_path.read_text(encoding="utf-8"))
matches = [item for item in registry.get("validators", []) if item.get("node_id") == receipt.get("node_id")]
if len(matches) != 1 or matches[0].get("finality_signer_public_key", "").lower() != receipt.get("finality_public_key", "").lower():
    raise SystemExit("validator-47 staged public identity does not match governed registry")
PY
fi
mkdir -p "$bundle_root/bin"
cp -a "$ops_bundle_root/bin/." "$bundle_root/bin/"
python3 "$SCRIPT_DIR/p2p-rebuild-linux-bundle-checksums.py" "$bundle_root" \
  || die "deployed player/ops checksum closure rebuild failed"
require_dir "$bundle_root"
for binary in oasis7_world_repair_rebuild oasis7_governance_registry_import oasis7_governance_registry_audit service-readback; do
  [[ -x "$bundle_root/bin/$binary" ]] || die "bundle missing required executable: $binary"
done

# All failure-prone validation above occurs before the fresh root is created.
mkdir -p "$stack_root/releases"
safe_cleanup_root=1
install -d -m 0700 "$stack_root/config" "$stack_root/staged-world" "$stack_root/data" "$stack_root/logs"
release_dir="$stack_root/releases/$(sed -n 's/^package_version=//p' "$bundle_root/BUILDINFO" | head -n1)"
[[ ! -e "$release_dir" ]] || die "release destination unexpectedly exists"
mv "$bundle_root" "$release_dir"
ln -s "releases/$(basename "$release_dir")" "$stack_root/current"
cp -a "$config_dir/." "$stack_root/config/"
cp -a "$world_dir/." "$stack_root/staged-world/"
if [[ "$node_id" == "$VALIDATOR_47_NODE_ID" ]]; then
  # Import the already-staged identity byte-for-byte.  This path is deliberately
  # separate from the governed config stage so a stale pair key cannot be
  # mistaken for validator-47.  No runtime provisioning command is used here.
  identity_key_path="$stack_root/config/$VALIDATOR_47_IDENTITY_KEY_FILE"
  identity_receipt_path="$stack_root/config/$VALIDATOR_47_IDENTITY_RECEIPT_FILE"
  install -m 0600 "$identity_dir/$VALIDATOR_47_IDENTITY_KEY_FILE" "$identity_key_path"
  install -m 0600 "$identity_dir/$VALIDATOR_47_IDENTITY_RECEIPT_FILE" "$identity_receipt_path"
  python3 - "$identity_dir" "$identity_key_path" "$identity_receipt_path" <<'PY'
import os
import stat
import sys
from pathlib import Path

source_dir = Path(sys.argv[1])
destination_key = Path(sys.argv[2])
destination_receipt = Path(sys.argv[3])
source_directory_metadata = os.lstat(source_dir)
for name, destination in (("node-keypair.toml", destination_key), ("identity-receipt.json", destination_receipt)):
    source = source_dir / name
    source_metadata = os.lstat(source)
    destination_metadata = os.lstat(destination)
    if not stat.S_ISREG(destination_metadata.st_mode) or stat.S_IMODE(destination_metadata.st_mode) != 0o600:
        raise SystemExit(f"validator-47 imported {name} is not a regular file with mode 0600")
    if (source_metadata.st_uid, source_metadata.st_gid) != (destination_metadata.st_uid, destination_metadata.st_gid):
        raise SystemExit(f"validator-47 imported {name} ownership mismatch")
    if source.read_bytes() != destination.read_bytes():
        raise SystemExit(f"validator-47 imported {name} bytes changed")
if (destination_key.stat().st_uid, destination_key.stat().st_gid) != (destination_receipt.stat().st_uid, destination_receipt.stat().st_gid):
    raise SystemExit("validator-47 imported identity key/receipt ownership mismatch")
PY
fi
install -d -m 0755 "$stack_root/bin"
install -m 0755 "$start_script" "$stack_root/bin/start-node.sh"
find "$stack_root/config" "$stack_root/staged-world" -type d -exec chmod 0700 {} +
find "$stack_root/config" "$stack_root/staged-world" -type f -exec chmod 0600 {} +

if [[ "$node_id" == "$VALIDATOR_47_NODE_ID" ]]; then
  identity_json=$("$stack_root/current/bin/oasis7_chain_runtime" identity-receipt \
    --config-dir "$stack_root/config" --node-id "$node_id") \
    || die "validator-47 imported identity readback failed"
  identity_public_receipt_json=$(jq -c . "$stack_root/config/$VALIDATOR_47_IDENTITY_RECEIPT_FILE")
  identity_json=$(jq -cn \
    --argjson readback "$identity_json" --argjson public "$identity_public_receipt_json" \
    '$readback + {root_public_key:$public.root_public_key,finality_public_key:$public.finality_public_key,libp2p_peer_id:$public.libp2p_peer_id,node_keypair_config_path:$readback.key_path,node_keypair_config_exists:true,node_keypair_config_mode:"0600"}')
else
  identity_json=$("$stack_root/current/bin/oasis7_chain_runtime" provision-identity \
    --config-dir "$stack_root/config" --node-id "$node_id") \
    || die "identity provisioning failed"
fi
config_dir_physical=$(python3 - "$stack_root/config" <<'PY'
import pathlib
import sys
print(pathlib.Path(sys.argv[1]).resolve())
PY
)
printf '%s' "$identity_json" | jq -e \
  --arg config "$stack_root/config" --arg physical_config "$config_dir_physical" --arg node "$node_id" '
    type == "object" and .node_id == $node
    and (.node_keypair_config_path == ($config + "/node-keypair.toml")
         or .node_keypair_config_path == ($physical_config + "/node-keypair.toml"))
    and .node_keypair_config_exists == true and .node_keypair_config_mode == "0600"
    and (.root_public_key | type == "string" and length > 0)
    and (.finality_public_key | type == "string" and length > 0)
    and (.libp2p_peer_id | type == "string" and length > 0)' >/dev/null \
  || die "identity readback returned invalid public receipt"
key_path="$stack_root/config/node-keypair.toml"
require_file "$key_path"
chmod 0600 "$key_path"
if [[ "$node_id" == "$VALIDATOR_47_NODE_ID" ]]; then
  [[ "$(sha256_file "$key_path")" == "$identity_source_key_sha256" ]] \
    || die "validator-47 imported identity key digest changed"
  [[ "$(sha256_file "$stack_root/config/$VALIDATOR_47_IDENTITY_RECEIPT_FILE")" == "$identity_source_receipt_sha256" ]] \
    || die "validator-47 imported identity receipt digest changed"
fi

# The unit deliberately runs unprivileged while config remains 0700/0600.
# Production bootstrap therefore requires the managed service account before
# installing the unit; test mode is render-only and has no host account.
if [[ ${OASIS7_TEST_ONLY:-} != 1 ]]; then
  chown -R oasis7-testnet:oasis7-testnet "$stack_root"
fi

mkdir -p "$systemd_unit_dir"
installed_unit_path="$systemd_unit_dir/$service_name"
render_unit "$template" "$installed_unit_path" "$stack_root"
if [[ ${OASIS7_TEST_ONLY:-} != 1 ]]; then
  systemctl daemon-reload
  if systemctl is-active --quiet "$service_name"; then
    die "service is active; fresh bootstrap never stops or replaces an active service"
  fi
  systemctl disable "$service_name" >/dev/null 2>&1 || true
  service_active=$(systemctl is-active "$service_name" 2>&1 || true)
  service_enabled=$(systemctl is-enabled "$service_name" 2>&1 || true)
  [[ "$service_active" == inactive ]] || die "service must be inactive after install"
  [[ "$service_enabled" == disabled ]] || die "service must be disabled after install"
  service_unit_file_state=$(systemctl show --no-page --property=UnitFileState "$service_name" | sed -n 's/^UnitFileState=//p')
  [[ "$service_unit_file_state" == disabled ]] || die "UnitFileState must be disabled after install"
else
  service_active=rendered_test_only
  service_enabled=rendered_test_only
  # Test-only mode renders a unit without a service manager; the explicit
  # no-start receipt still records the state required of production staging.
  service_unit_file_state=disabled
fi

# Keep service-manager state independent in the receipt.  The packaged
# service-readback command is the independent process/listener observation.
no_process=true
no_listener=true
listeners_json='[]'
if [[ ${OASIS7_TEST_ONLY:-} != 1 ]]; then
  no_process=null
  no_listener=null
fi

runtime_path="$stack_root/current/bin/oasis7_chain_runtime"
runtime_sha=$(sha256_file "$runtime_path")
runtime_size=$(file_size "$runtime_path")
package_sha=$(sha256_file "$package_deb")
package_size=$(file_size "$package_deb")
ops_tools_sha=$(sha256_file "$ops_tools_tar")
ops_tools_size=$(file_size "$ops_tools_tar")
unit_sha=$(sha256_file "$installed_unit_path")
build_commit=$(sed -n 's/^commit=//p' "$release_dir/BUILDINFO" | head -n1)
build_version=$(sed -n 's/^package_version=//p' "$release_dir/BUILDINFO" | head -n1)
build_run_id=$(sed -n 's/^run_id=//p' "$release_dir/BUILDINFO" | head -n1)
binaries_json='[]'
for binary in oasis7_chain_runtime oasis7_world_repair_rebuild oasis7_governance_registry_import oasis7_governance_registry_audit service-readback; do
  binary_path="$stack_root/current/bin/$binary"
  binaries_json=$(jq -c --arg path "$binary_path" --arg sha256 "$(sha256_file "$binary_path")" \
    --argjson size "$(file_size "$binary_path")" --argjson executable true \
    '. + [{path:$path,sha256:$sha256,size_bytes:$size,executable:$executable}]' <<<"$binaries_json")
done
config_bundle_json=$(public_file_json "$stack_root/config/public-testnet-governed-bootstrap-bundle-2026-06-06.json")
config_manifest_json=$(public_file_json "$stack_root/config/public-testnet-governed-bootstrap-manifest-2026-06-06.json")
config_genesis_json=$(public_file_json "$stack_root/config/public-testnet-governed-bootstrap-genesis-2026-06-06.json")
config_registry_json=$(public_file_json "$stack_root/config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json")
config_peers_json=$(public_file_json "$stack_root/config/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt")
config_node_env_json=$(public_file_json "$stack_root/config/node.env")
config_inventory_json=$(public_file_json "$stack_root/config/$TRIAD_INVENTORY_FILE")
world_snapshot_json=$(public_file_json "$stack_root/staged-world/snapshot.json")
world_provenance_json=$(public_file_json "$stack_root/staged-world/world-generation-provenance.json")
buildinfo_json=$(public_file_json "$release_dir/BUILDINFO")
checksums_json=$(public_file_json "$release_dir/SHA256SUMS")
key_json=$(key_owner_json "$key_path")
if [[ ${OASIS7_TEST_ONLY:-} != 1 ]]; then
  service_uid=$(id -u oasis7-testnet)
  service_gid=$(id -g oasis7-testnet)
  key_owner_valid=$(jq -e --argjson uid "$service_uid" --argjson gid "$service_gid" '.uid == $uid and .gid == $gid and .mode == "0600"' <<<"$key_json" >/dev/null && printf true || printf false)
else
  service_uid=null
  service_gid=null
  key_owner_valid=$(jq -e '.mode == "0600"' <<<"$key_json" >/dev/null && printf true || printf false)
fi
identity_ownership_valid=true
if [[ "$node_id" == "$VALIDATOR_47_NODE_ID" ]]; then
  identity_ownership_valid=$(python3 - "$stack_root/config" "$key_path" "$stack_root/config/$VALIDATOR_47_IDENTITY_RECEIPT_FILE" <<'PY'
import os
import stat
import sys
from pathlib import Path

config_dir, key_path, receipt_path = map(Path, sys.argv[1:])
config_metadata = os.lstat(config_dir)
result = True
for path in (key_path, receipt_path):
    metadata = os.lstat(path)
    result = result and stat.S_ISREG(metadata.st_mode)
    result = result and stat.S_IMODE(metadata.st_mode) == 0o600
    result = result and (metadata.st_uid, metadata.st_gid) == (config_metadata.st_uid, config_metadata.st_gid)
print("true" if result else "false")
PY
  )
  [[ "$identity_ownership_valid" == true ]] || die "validator-47 imported identity ownership contract failed"
fi
receipt_parent=$(dirname "$receipt")
mkdir -p "$receipt_parent"
jq -n \
  --arg time "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" --arg root "$stack_root" \
  --arg service_name "$service_name" \
  --arg unit "$installed_unit_path" --arg unit_sha "$unit_sha" --arg active "$service_active" --arg enabled "$service_enabled" \
  --arg unit_file_state "$service_unit_file_state" --argjson no_process "$no_process" --argjson no_listener "$no_listener" --argjson listeners "$listeners_json" \
  --arg inventory_ref "$TRIAD_INVENTORY_RELATIVE" --arg inventory_sha256 "$triad_inventory_sha256" \
  --arg commit "$build_commit" --arg package_version "$build_version" --arg run_id "$build_run_id" \
  --arg package_deb "$package_deb" --arg package_sha "$package_sha" --argjson package_size "$package_size" \
  --arg ops_tools "$ops_tools_tar" --arg ops_tools_sha "$ops_tools_sha" --argjson ops_tools_size "$ops_tools_size" \
  --arg runtime "$runtime_path" --arg runtime_sha "$runtime_sha" --argjson runtime_size "$runtime_size" \
  --argjson binaries "$binaries_json" --argjson node "$identity_json" --argjson key "$key_json" \
  --argjson key_owner_valid "$key_owner_valid" --argjson service_uid "$service_uid" --argjson service_gid "$service_gid" \
  --arg identity_import_mode "$identity_import_mode" --arg identity_source_key_sha256 "$identity_source_key_sha256" \
  --arg identity_source_receipt_sha256 "$identity_source_receipt_sha256" \
  --argjson identity_ownership_valid "$identity_ownership_valid" \
  --argjson staged_identity_receipt "$staged_identity_receipt_json" \
  --argjson config_bundle "$config_bundle_json" --argjson config_manifest "$config_manifest_json" \
  --argjson config_genesis "$config_genesis_json" --argjson config_registry "$config_registry_json" \
  --argjson config_peers "$config_peers_json" --argjson config_node_env "$config_node_env_json" --argjson config_inventory "$config_inventory_json" \
  --argjson world_snapshot "$world_snapshot_json" --argjson world_provenance "$world_provenance_json" \
  --argjson buildinfo "$buildinfo_json" --argjson checksums "$checksums_json" \
  '{schema_version:"oasis7.fresh_validator_host_bootstrap.v1",generated_at:$time,
    stack_root:$root, no_service_started:true,
    package:{deb:{path:$package_deb,sha256:$package_sha,size_bytes:$package_size},ops_tools:{path:$ops_tools,sha256:$ops_tools_sha,size_bytes:$ops_tools_size},buildinfo:($buildinfo + {commit:$commit,package_version:$package_version,run_id:$run_id}),sha256sums:$checksums},
    runtime:{path:$runtime,sha256:$runtime_sha,size_bytes:$runtime_size,required_binaries:$binaries},
    node:{node_id:$node.node_id,public_key:$node.root_public_key,finality_public_key:$node.finality_public_key,libp2p_peer_id:$node.libp2p_peer_id,key:($key + {owner_valid:$key_owner_valid})},
    identity:{mode:$identity_import_mode,source_key_sha256:$identity_source_key_sha256,source_receipt_sha256:$identity_source_receipt_sha256,ownership_valid:$identity_ownership_valid,readback:$staged_identity_receipt},
    config:{node_env:$config_node_env,bundle:$config_bundle,manifest:$config_manifest,genesis:$config_genesis,validator_registry:$config_registry,bootstrap_peers:$config_peers,inventory:$config_inventory,inventory_ref:$inventory_ref,inventory_sha256:$inventory_sha256},
    world:{snapshot:$world_snapshot,provenance:$world_provenance},
    service:{name:$service_name,unit_path:$unit,unit_sha256:$unit_sha,active:$active,enabled:$enabled,unit_file_state:$unit_file_state,no_process:$no_process,no_listener:$no_listener,listeners:$listeners,account:{uid:$service_uid,gid:$service_gid}}}' \
  >"$receipt"
chmod 0600 "$receipt"
bootstrap_complete=1
printf 'fresh_validator_host_bootstrap=complete root=%s service=%s state=not_started\n' "$stack_root" "$service_name"
