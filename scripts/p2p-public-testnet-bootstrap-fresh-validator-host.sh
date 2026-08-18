#!/usr/bin/env bash
set -euo pipefail

# Provision a never-before-used public-testnet validator host.  This is
# deliberately a no-start operation: the service is rendered, but activation
# remains an explicit post-bootstrap operator action after topology review.

readonly PRODUCTION_ROOT=/opt/oasis7/p2p-testnet
readonly PRODUCTION_UNIT=/etc/systemd/system/oasis7-triad-sequencer.service
readonly SERVICE_NAME=oasis7-triad-sequencer.service
readonly BUNDLE_DIR_NAME=oasis7-linux-x64
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
  cat <<'EOF'
Usage:
  ./scripts/p2p-public-testnet-bootstrap-fresh-validator-host.sh \
    --package-deb <oasis7-linux-x64.deb> \
    --ops-tools-tar <oasis7-linux-x64-ops-tools.tar.gz> \
    --config-dir <governed stage config directory> \
    --world-dir <generated world directory> \
    --node-id triad-testnet-sequencer \
    --receipt <public receipt path>

Production uses exactly /opt/oasis7/p2p-testnet and
/etc/systemd/system/oasis7-triad-sequencer.service.  It renders and installs
the unit but never enables or starts it.

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
}

verify_bundle() {
  local bundle_root=$1 config=$2 runtime_sum expected_sum bundle_commit config_commit
  require_file "$bundle_root/BUILDINFO"
  require_file "$bundle_root/SHA256SUMS"
  require_file "$bundle_root/bin/oasis7_chain_runtime"
  (cd "$bundle_root" && shasum -a 256 -c SHA256SUMS) >/dev/null \
    || die "bundle checksum verification failed"
  for binary in oasis7_chain_runtime oasis7_world_repair_rebuild oasis7_governance_registry_import oasis7_governance_registry_audit; do
    [[ -x "$bundle_root/bin/$binary" ]] || die "bundle missing required executable: $binary"
  done
  grep -Eq '^commit=[0-9a-f]{40}$' "$bundle_root/BUILDINFO" || die "BUILDINFO missing valid commit"
  grep -Eq '^package_version=.+$' "$bundle_root/BUILDINFO" || die "BUILDINFO missing package_version"
  grep -Eq '^run_id=.+$' "$bundle_root/BUILDINFO" || die "BUILDINFO missing run_id"
  bundle_commit=$(sed -n 's/^commit=//p' "$bundle_root/BUILDINFO" | head -n1)
  config_commit=$(jq -r '.git_commit // empty' "$config/public-testnet-governed-bootstrap-bundle-2026-06-06.json")
  [[ "$bundle_commit" == "$config_commit" ]] || die "BUILDINFO commit does not match governed config"
  runtime_sum=$(shasum -a 256 "$bundle_root/bin/oasis7_chain_runtime" | awk '{print $1}')
  expected_sum=$(jq -r '.runtime_build.sha256 // empty' "$config/public-testnet-governed-bootstrap-bundle-2026-06-06.json")
  [[ "$runtime_sum" == "$expected_sum" ]] || die "BUILDINFO governed runtime checksum mismatch"
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
package_deb=""
ops_tools_tar=""
config_dir=""
world_dir=""
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
    --node-id) node_id=${2:-}; shift 2 ;;
    --service-name)
      [[ ${2:-} == "$SERVICE_NAME" ]] || die "service must be exactly $SERVICE_NAME"
      shift 2 ;;
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
  [[ "$systemd_unit_dir/$SERVICE_NAME" == "$PRODUCTION_UNIT" ]] || die "production unit must be $PRODUCTION_UNIT"
  assert_physical_path "$stack_root"
  assert_physical_path "$systemd_unit_dir"
  [[ "$receipt" == "$stack_root/evidence/fresh-validator-host-bootstrap-receipt.json" ]] \
    || die "production receipt must be exactly $stack_root/evidence/fresh-validator-host-bootstrap-receipt.json"
  require_command systemctl
  [[ ! -e "$PRODUCTION_UNIT" ]] || die "fresh host already has service unit: $PRODUCTION_UNIT"
  if systemctl is-active --quiet "$SERVICE_NAME"; then
    die "service is active; fresh bootstrap never stops or replaces an active service"
  fi
  ensure_service_account
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
for binary in oasis7_world_repair_rebuild oasis7_governance_registry_import oasis7_governance_registry_audit; do
  [[ -x "$ops_bundle_root/bin/$binary" ]] || die "ops-tools archive missing executable: $binary"
done
mkdir -p "$bundle_root/bin"
cp -a "$ops_bundle_root/bin/." "$bundle_root/bin/"
require_dir "$bundle_root"
verify_bundle "$bundle_root" "$config_dir"

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
install -d -m 0755 "$stack_root/bin"
install -m 0755 "$start_script" "$stack_root/bin/start-node.sh"
find "$stack_root/config" "$stack_root/staged-world" -type d -exec chmod 0700 {} +
find "$stack_root/config" "$stack_root/staged-world" -type f -exec chmod 0600 {} +

identity_json=$("$stack_root/current/bin/oasis7_chain_runtime" provision-identity \
  --config-dir "$stack_root/config" --node-id "$node_id") \
  || die "identity provisioning failed"
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
  || die "identity provisioning returned invalid public receipt"
key_path="$stack_root/config/node-keypair.toml"
require_file "$key_path"
chmod 0600 "$key_path"

# The unit deliberately runs unprivileged while config remains 0700/0600.
# Production bootstrap therefore requires the managed service account before
# installing the unit; test mode is render-only and has no host account.
if [[ ${OASIS7_TEST_ONLY:-} != 1 ]]; then
  chown -R oasis7-testnet:oasis7-testnet "$stack_root"
fi

mkdir -p "$systemd_unit_dir"
installed_unit_path="$systemd_unit_dir/$SERVICE_NAME"
render_unit "$template" "$installed_unit_path" "$stack_root"
if [[ ${OASIS7_TEST_ONLY:-} != 1 ]]; then
  systemctl daemon-reload
  if systemctl is-active --quiet "$SERVICE_NAME"; then
    die "service is active; fresh bootstrap never stops or replaces an active service"
  fi
  systemctl disable "$SERVICE_NAME" >/dev/null 2>&1 || true
  service_active=$(systemctl is-active "$SERVICE_NAME" 2>&1 || true)
  service_enabled=$(systemctl is-enabled "$SERVICE_NAME" 2>&1 || true)
  [[ "$service_active" == inactive ]] || die "service must be inactive after install"
  [[ "$service_enabled" == disabled ]] || die "service must be disabled after install"
else
  service_active=rendered_test_only
  service_enabled=rendered_test_only
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
for binary in oasis7_chain_runtime oasis7_world_repair_rebuild oasis7_governance_registry_import oasis7_governance_registry_audit; do
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
receipt_parent=$(dirname "$receipt")
mkdir -p "$receipt_parent"
jq -n \
  --arg time "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" --arg root "$stack_root" \
  --arg unit "$installed_unit_path" --arg unit_sha "$unit_sha" --arg active "$service_active" --arg enabled "$service_enabled" \
  --arg commit "$build_commit" --arg package_version "$build_version" --arg run_id "$build_run_id" \
  --arg package_deb "$package_deb" --arg package_sha "$package_sha" --argjson package_size "$package_size" \
  --arg ops_tools "$ops_tools_tar" --arg ops_tools_sha "$ops_tools_sha" --argjson ops_tools_size "$ops_tools_size" \
  --arg runtime "$runtime_path" --arg runtime_sha "$runtime_sha" --argjson runtime_size "$runtime_size" \
  --argjson binaries "$binaries_json" --argjson node "$identity_json" --argjson key "$key_json" \
  --argjson key_owner_valid "$key_owner_valid" --argjson service_uid "$service_uid" --argjson service_gid "$service_gid" \
  --argjson config_bundle "$config_bundle_json" --argjson config_manifest "$config_manifest_json" \
  --argjson config_genesis "$config_genesis_json" --argjson config_registry "$config_registry_json" \
  --argjson config_peers "$config_peers_json" --argjson config_node_env "$config_node_env_json" \
  --argjson world_snapshot "$world_snapshot_json" --argjson world_provenance "$world_provenance_json" \
  --argjson buildinfo "$buildinfo_json" --argjson checksums "$checksums_json" \
  '{schema_version:"oasis7.fresh_validator_host_bootstrap.v1",generated_at:$time,
    stack_root:$root, no_service_started:true,
    package:{deb:{path:$package_deb,sha256:$package_sha,size_bytes:$package_size},ops_tools:{path:$ops_tools,sha256:$ops_tools_sha,size_bytes:$ops_tools_size},buildinfo:($buildinfo + {commit:$commit,package_version:$package_version,run_id:$run_id}),sha256sums:$checksums},
    runtime:{path:$runtime,sha256:$runtime_sha,size_bytes:$runtime_size,required_binaries:$binaries},
    node:{node_id:$node.node_id,public_key:$node.root_public_key,finality_public_key:$node.finality_public_key,libp2p_peer_id:$node.libp2p_peer_id,key:($key + {owner_valid:$key_owner_valid})},
    config:{node_env:$config_node_env,bundle:$config_bundle,manifest:$config_manifest,genesis:$config_genesis,validator_registry:$config_registry,bootstrap_peers:$config_peers},
    world:{snapshot:$world_snapshot,provenance:$world_provenance},
    service:{name:"oasis7-triad-sequencer.service",unit_path:$unit,unit_sha256:$unit_sha,active:$active,enabled:$enabled,account:{uid:$service_uid,gid:$service_gid}}}' \
  >"$receipt"
chmod 0600 "$receipt"
bootstrap_complete=1
printf 'fresh_validator_host_bootstrap=complete root=%s service=%s state=not_started\n' "$stack_root" "$SERVICE_NAME"
