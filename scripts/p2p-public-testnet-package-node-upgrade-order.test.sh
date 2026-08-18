#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-package-node-upgrade-order.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

if ! command -v dpkg-deb >/dev/null 2>&1; then
  mkdir -p "$TMP_DIR/fake-bin"
  cat >"$TMP_DIR/fake-bin/dpkg-deb" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  --build)
    source="${@: -2:1}"
    output="${@: -1}"
    mkdir -p "${output}.root"
    cp -a "$source/." "${output}.root/"
    : >"$output"
    ;;
  --extract)
    source="${2:?missing source}"
    destination="${3:?missing destination}"
    mkdir -p "$destination"
    cp -a "${source}.root/." "$destination/"
    ;;
  *)
    echo "unsupported dpkg-deb fixture operation" >&2
    exit 2
    ;;
esac
SH
  chmod +x "$TMP_DIR/fake-bin/dpkg-deb"
  export PATH="$TMP_DIR/fake-bin:$PATH"
fi

make_node_fixture() {
  local root=$1
  mkdir -p \
    "$root/releases/old/bin" \
    "$root/config/doc/testing/evidence"
  printf 'runtime-v1\n' >"$root/releases/old/bin/oasis7_chain_runtime"
  chmod +x "$root/releases/old/bin/oasis7_chain_runtime"
  ln -s "$root/releases/old" "$root/current"
  cat >"$root/config/doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json" <<'EOF'
{
  "schema_version": "oasis7.release_candidate_bundle.v1",
  "git_commit": "old",
  "runtime_build": {
    "path": "old",
    "ref": "old",
    "resolved_path": "old",
    "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
    "size_bytes": 1
  }
}
EOF
}

make_bundle() {
  local bundle_dir=$1
  mkdir -p "$bundle_dir/oasis7-linux-x64/bin"
  printf 'runtime-v2\n' >"$bundle_dir/oasis7-linux-x64/bin/oasis7_chain_runtime"
  chmod +x "$bundle_dir/oasis7-linux-x64/bin/oasis7_chain_runtime"
  printf 'commit=abcdef1234567890abcdef1234567890abcdef12\npackage_version=0.0.0+order-test\nrun_id=3191-order\nplatform=linux-x64\n' \
    >"$bundle_dir/oasis7-linux-x64/BUILDINFO"
  (cd "$bundle_dir/oasis7-linux-x64" && shasum -a 256 BUILDINFO bin/oasis7_chain_runtime >SHA256SUMS)
  mkdir -p "$bundle_dir/oasis7-linux-x64-ops-tools/bin"
  for binary in oasis7_world_repair_rebuild oasis7_governance_registry_import oasis7_governance_registry_audit; do
    printf '#!/usr/bin/env bash\n' >"$bundle_dir/oasis7-linux-x64-ops-tools/bin/$binary"
    chmod +x "$bundle_dir/oasis7-linux-x64-ops-tools/bin/$binary"
  done
  printf '{"opsToolsSchemaVersion":1}\n' >"$bundle_dir/oasis7-linux-x64-ops-tools/.oasis7-ops-tools-manifest.json"
  (cd "$bundle_dir/oasis7-linux-x64-ops-tools" && shasum -a 256 .oasis7-ops-tools-manifest.json bin/* >SHA256SUMS)
  tar -czf "$bundle_dir/oasis7-linux-x64-ops-tools.tar.gz" \
    -C "$bundle_dir" oasis7-linux-x64-ops-tools
  package_root="$bundle_dir/deb-root"
  mkdir -p "$package_root/DEBIAN" "$package_root/opt/oasis7"
  cp -a "$bundle_dir/oasis7-linux-x64/." "$package_root/opt/oasis7/"
  cat >"$package_root/DEBIAN/control" <<'EOF'
Package: oasis7
Version: 0.0.0
Section: games
Priority: optional
Architecture: amd64
Description: package node upgrade ordering fixture
EOF
  dpkg-deb --build --root-owner-group "$package_root" "$bundle_dir/oasis7-linux-x64.deb" >/dev/null
}

cat >"$TMP_DIR/fake-bin-systemctl" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

event_log=${FAKE_SYSTEMCTL_LOG:?}
node_root=${FAKE_NODE_ROOT:?}
printf 'systemctl %s\n' "$*" >>"$event_log"

if [[ "${1:-}" == "stop" ]]; then
  snapshot_count=$(find "$node_root/package-upgrade-rollback" -name transaction.json -type f 2>/dev/null | wc -l | tr -d ' ')
  if [[ "$snapshot_count" -lt 1 ]]; then
    printf 'stop observed before active transaction snapshot\n' >>"$event_log"
    exit 97
  fi
fi
exit 0
SH
cat >"$TMP_DIR/fake-bin-ps" <<'SH'
#!/usr/bin/env bash
printf '777 1 harmless-process\n'
SH
mkdir -p "$TMP_DIR/fake-bin"
mv "$TMP_DIR/fake-bin-systemctl" "$TMP_DIR/fake-bin/systemctl"
mv "$TMP_DIR/fake-bin-ps" "$TMP_DIR/fake-bin/ps"
chmod +x "$TMP_DIR/fake-bin/systemctl" "$TMP_DIR/fake-bin/ps"

bundle_dir="$TMP_DIR/bundle"
make_bundle "$bundle_dir"
failures=0

# Build a separate malicious Debian fixture from the package root.  Real
# dpkg-deb does not create the fallback `<package>.root` directory used by the
# macOS shim, so mutating that path after the build is not portable.  Rebuild a
# dedicated artifact with the symlink in its archive instead.
unsafe_deb_package="$bundle_dir/oasis7-linux-x64-unsafe-link.deb"
ln -s "$TMP_DIR" "$bundle_dir/deb-root/opt/oasis7/unsafe-link"
dpkg-deb --build --root-owner-group "$bundle_dir/deb-root" "$unsafe_deb_package" >/dev/null
rm "$bundle_dir/deb-root/opt/oasis7/unsafe-link"

# The node upgrader must reject non-regular ops-tools members before creating a
# release or touching systemd, just like fresh-host bootstrap.
unsafe_ops_tar="$TMP_DIR/unsafe-ops-tools.tar.gz"
python3 - "$unsafe_ops_tar" <<'PY'
import tarfile
import sys

member = tarfile.TarInfo("oasis7-linux-x64-ops-tools/bin/unsafe")
member.type = tarfile.SYMTYPE
member.linkname = "../../outside"
with tarfile.open(sys.argv[1], "w:gz") as archive:
    archive.addfile(member)
PY
unsafe_node="$TMP_DIR/unsafe-node"
make_node_fixture "$unsafe_node"
set +e
PATH="$TMP_DIR/fake-bin:$PATH" \
  "$ROOT_DIR/scripts/p2p-public-testnet-package-node-upgrade.sh" \
  --node-root "$unsafe_node" \
  --package-deb "$bundle_dir/oasis7-linux-x64.deb" \
  --ops-tools-tar "$unsafe_ops_tar" \
  --package-version 0.0.0+order-test \
  --commit abcdef1234567890abcdef1234567890abcdef12 \
  --run-id 3191-order \
  >"$TMP_DIR/unsafe.out" 2>&1
unsafe_status=$?
set -e
test "$unsafe_status" -ne 0
grep -q 'non-regular member' "$TMP_DIR/unsafe.out"
test "$(readlink "$unsafe_node/current")" = "$unsafe_node/releases/old"
test ! -e "$unsafe_node/releases/0.0.0+unsafe-test"

# A package extraction containing a symlink must fail before any bundle hash,
# ops-tool copy, release promotion, or service-manager call.
unsafe_deb_node="$TMP_DIR/unsafe-deb-node"
make_node_fixture "$unsafe_deb_node"
set +e
PATH="$TMP_DIR/fake-bin:$PATH" \
  "$ROOT_DIR/scripts/p2p-public-testnet-package-node-upgrade.sh" \
  --node-root "$unsafe_deb_node" \
  --package-deb "$unsafe_deb_package" \
  --ops-tools-tar "$bundle_dir/oasis7-linux-x64-ops-tools.tar.gz" \
  --package-version 0.0.0+order-test \
  --commit abcdef1234567890abcdef1234567890abcdef12 \
  --run-id 3191-order \
  >"$TMP_DIR/unsafe-deb.out" 2>&1
unsafe_deb_status=$?
set -e
test "$unsafe_deb_status" -ne 0
grep -q 'symlink' "$TMP_DIR/unsafe-deb.out"
test ! -e "$unsafe_deb_node/releases/0.0.0+order-test"
test ! -e "$unsafe_deb_node/package-upgrade-rollback"

# RED 1: a service stop must only happen after a durable transaction snapshot
# exists.  The current upgrader calls systemctl stop before create_transaction_snapshot.
ordered_node="$TMP_DIR/ordered-node"
make_node_fixture "$ordered_node"
set +e
FAKE_NODE_ROOT="$ordered_node" \
FAKE_SYSTEMCTL_LOG="$TMP_DIR/ordered-systemctl.log" \
PATH="$TMP_DIR/fake-bin:$PATH" \
  "$ROOT_DIR/scripts/p2p-public-testnet-package-node-upgrade.sh" \
  --node-root "$ordered_node" \
  --package-deb "$bundle_dir/oasis7-linux-x64.deb" \
  --ops-tools-tar "$bundle_dir/oasis7-linux-x64-ops-tools.tar.gz" \
  --package-version 0.0.0+order-test \
  --commit abcdef1234567890abcdef1234567890abcdef12 \
  --run-id 3191-order \
  --systemd-service oasis7-testnet-order.service \
  --restart-service \
  >"$TMP_DIR/ordered.out" 2>&1
ordered_status=$?
set -e
if [[ "$ordered_status" -ne 0 ]]; then
  printf '%s\n' 'package upgrader stopped service before activating transaction snapshot' >&2
  cat "$TMP_DIR/ordered-systemctl.log" >&2
  cat "$TMP_DIR/ordered.out" >&2
  failures=1
fi

# RED 2: any snapshot/preflight failure must happen before touching systemd.
# Make the rollback parent a file so snapshot creation fails deterministically.
preflight_node="$TMP_DIR/preflight-node"
make_node_fixture "$preflight_node"
printf 'not-a-directory\n' >"$preflight_node/package-upgrade-rollback"
set +e
FAKE_NODE_ROOT="$preflight_node" \
FAKE_SYSTEMCTL_LOG="$TMP_DIR/preflight-systemctl.log" \
PATH="$TMP_DIR/fake-bin:$PATH" \
  "$ROOT_DIR/scripts/p2p-public-testnet-package-node-upgrade.sh" \
  --node-root "$preflight_node" \
  --package-deb "$bundle_dir/oasis7-linux-x64.deb" \
  --ops-tools-tar "$bundle_dir/oasis7-linux-x64-ops-tools.tar.gz" \
  --package-version 0.0.0+preflight-test \
  --commit abcdef1234567890abcdef1234567890abcdef12 \
  --run-id 3191-preflight \
  --systemd-service oasis7-testnet-preflight.service \
  --restart-service \
  >"$TMP_DIR/preflight.out" 2>&1
preflight_status=$?
set -e
test "$preflight_status" -ne 0
if [[ -s "$TMP_DIR/preflight-systemctl.log" ]]; then
  printf '%s\n' 'package upgrader touched systemd before snapshot/preflight failure' >&2
  cat "$TMP_DIR/preflight-systemctl.log" >&2
  cat "$TMP_DIR/preflight.out" >&2
  failures=1
fi

if [[ "$failures" -ne 0 ]]; then
  exit 1
fi

printf '%s\n' 'ok: package upgrader snapshots before stop and leaves service untouched on preflight failure'
