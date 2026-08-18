#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-package-node-upgrade-health-test.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

node_root="$TMP_DIR/node"
bundle_root="$TMP_DIR/bundle/oasis7-linux-x64"
package_deb="$TMP_DIR/oasis7-linux-x64.deb"
ops_bundle_root="$TMP_DIR/bundle/oasis7-linux-x64-ops-tools"
ops_tools_tar="$TMP_DIR/oasis7-linux-x64-ops-tools.tar.gz"
package_version="0.0.0+testnet.health.abcdef123456"
commit="abcdef1234567890abcdef1234567890abcdef12"
run_id="12345"
health_url="http://127.0.0.1:6631/healthz"
status_url="http://127.0.0.1:6631/v1/chain/status"

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

mkdir -p \
  "$bundle_root/bin" \
  "$node_root/config/doc/testing/evidence" \
  "$node_root/releases/old/bin" \
  "$TMP_DIR/fake-bin"
printf 'runtime-v2\n' >"$bundle_root/bin/oasis7_chain_runtime"
chmod +x "$bundle_root/bin/oasis7_chain_runtime"
printf 'commit=%s\npackage_version=%s\nrun_id=%s\nplatform=linux-x64\n' "$commit" "$package_version" "$run_id" >"$bundle_root/BUILDINFO"
(cd "$bundle_root" && shasum -a 256 BUILDINFO bin/oasis7_chain_runtime >SHA256SUMS)
printf 'runtime-v1\n' >"$node_root/releases/old/bin/oasis7_chain_runtime"
chmod +x "$node_root/releases/old/bin/oasis7_chain_runtime"
ln -s "$node_root/releases/old" "$node_root/current"
cat >"$node_root/config/doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json" <<'EOF'
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
mkdir -p "$ops_bundle_root/bin"
for binary in oasis7_world_repair_rebuild oasis7_governance_registry_import oasis7_governance_registry_audit; do
  printf '#!/usr/bin/env bash\n' >"$ops_bundle_root/bin/$binary"
  chmod +x "$ops_bundle_root/bin/$binary"
done
printf '{"opsToolsSchemaVersion":1}\n' >"$ops_bundle_root/.oasis7-ops-tools-manifest.json"
(cd "$ops_bundle_root" && shasum -a 256 .oasis7-ops-tools-manifest.json bin/* >SHA256SUMS)
tar -czf "$ops_tools_tar" -C "$TMP_DIR/bundle" oasis7-linux-x64-ops-tools
package_root="$TMP_DIR/deb-root"
mkdir -p "$package_root/DEBIAN" "$package_root/opt/oasis7"
cp -a "$bundle_root/." "$package_root/opt/oasis7/"
cat >"$package_root/DEBIAN/control" <<'EOF'
Package: oasis7
Version: 0.0.0
Section: games
Priority: optional
Architecture: amd64
Description: package node upgrade health fixture
EOF
dpkg-deb --build --root-owner-group "$package_root" "$package_deb" >/dev/null
node_root_abs=$(python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).resolve())' "$node_root")

cat >"$TMP_DIR/fake-bin/systemctl" <<'SH'
#!/usr/bin/env bash
printf 'systemctl %s\n' "$*" >>"$FAKE_SYSTEMCTL_LOG"
exit 0
SH
cat >"$TMP_DIR/fake-bin/ps" <<'SH'
#!/usr/bin/env bash
printf '777 1 harmless-process\n'
SH
cat >"$TMP_DIR/fake-bin/curl" <<'SH'
#!/usr/bin/env bash
url="${!#}"
printf '%s\n' "$url" >>"$FAKE_CURL_LOG"
case "$url" in
  */healthz)
    printf '{"ok":true}\n'
    ;;
  */v1/chain/status)
    printf 'full status serialization is forbidden in this constrained-node gate\n' >&2
    exit 97
    ;;
  *)
    printf 'unexpected probe URL: %s\n' "$url" >&2
    exit 98
    ;;
esac
SH
chmod +x "$TMP_DIR/fake-bin/systemctl" "$TMP_DIR/fake-bin/ps" "$TMP_DIR/fake-bin/curl"

set +e
FAKE_SYSTEMCTL_LOG="$TMP_DIR/systemctl.log" \
FAKE_CURL_LOG="$TMP_DIR/curl.log" \
PATH="$TMP_DIR/fake-bin:$PATH" \
"$ROOT_DIR/scripts/p2p-public-testnet-package-node-upgrade.sh" \
  --node-root "$node_root" \
  --package-deb "$package_deb" \
  --ops-tools-tar "$ops_tools_tar" \
  --package-version "$package_version" \
  --commit "$commit" \
  --run-id "$run_id" \
  --systemd-service oasis7-testnet-health.service \
  --restart-service \
  --post-restart-health-url "$health_url" \
  --post-restart-timeout-secs 1 \
  >"$TMP_DIR/health.out" 2>&1
health_status=$?
set -e
if [[ "$health_status" -ne 0 ]]; then
  echo "healthz readiness gate failed before constrained-node upgrade could complete" >&2
  cat "$TMP_DIR/health.out" >&2
fi
test "$health_status" -eq 0
grep -q 'post_restart_health=ok' "$TMP_DIR/health.out"
test "$(cat "$TMP_DIR/curl.log")" = "$health_url"
! grep -q '/v1/chain/status' "$TMP_DIR/curl.log"
test "$(readlink "$node_root_abs/current")" = "$node_root_abs/releases/$package_version"

set +e
FAKE_SYSTEMCTL_LOG="$TMP_DIR/systemctl-conflict.log" \
FAKE_CURL_LOG="$TMP_DIR/curl-conflict.log" \
PATH="$TMP_DIR/fake-bin:$PATH" \
"$ROOT_DIR/scripts/p2p-public-testnet-package-node-upgrade.sh" \
  --node-root "$node_root" \
  --package-deb "$package_deb" \
  --ops-tools-tar "$ops_tools_tar" \
  --package-version "$package_version-conflict" \
  --commit "$commit" \
  --run-id "$run_id" \
  --systemd-service oasis7-testnet-health.service \
  --restart-service \
  --post-restart-status-url "$status_url" \
  --post-restart-health-url "$health_url" \
  >"$TMP_DIR/conflict.out" 2>&1
conflict_status=$?
set -e
test "$conflict_status" -ne 0
grep -q 'mutually exclusive' "$TMP_DIR/conflict.out"
test ! -s "$TMP_DIR/curl-conflict.log"

echo 'ok: constrained package upgrade readiness uses healthz without full status serialization'
