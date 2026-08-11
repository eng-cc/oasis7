#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-package-node-upgrade-health-test.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

node_root="$TMP_DIR/node"
bundle_root="$TMP_DIR/bundle/oasis7-linux-x64"
package_version="0.0.0+testnet.health.abcdef123456"
commit="abcdef1234567890abcdef1234567890abcdef12"
run_id="12345"
health_url="http://127.0.0.1:6631/healthz"
status_url="http://127.0.0.1:6631/v1/chain/status"

mkdir -p \
  "$bundle_root/bin" \
  "$node_root/config/doc/testing/evidence" \
  "$node_root/releases/old/bin" \
  "$TMP_DIR/fake-bin"
printf 'runtime-v2\n' >"$bundle_root/bin/oasis7_chain_runtime"
chmod +x "$bundle_root/bin/oasis7_chain_runtime"
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
tar -czf "$TMP_DIR/oasis7-linux-x64-bundle.tar.gz" \
  -C "$TMP_DIR/bundle" oasis7-linux-x64
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
  --bundle-tar "$TMP_DIR/oasis7-linux-x64-bundle.tar.gz" \
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
  --bundle-tar "$TMP_DIR/oasis7-linux-x64-bundle.tar.gz" \
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
