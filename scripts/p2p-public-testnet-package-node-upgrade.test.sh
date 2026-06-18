#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-package-node-upgrade-test.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

node_root="$TMP_DIR/node"
bundle_root="$TMP_DIR/bundle/oasis7-linux-x64"
package_version="0.0.0+testnet.test.abcdef123456"
commit="abcdef1234567890abcdef1234567890abcdef12"
run_id="12345"

mkdir -p \
  "$bundle_root/bin" \
  "$node_root/config/doc/testing/evidence" \
  "$node_root/releases/old/bin" \
  "$node_root/data" \
  "$node_root/backups"
printf 'runtime-v2\n' >"$bundle_root/bin/oasis7_chain_runtime"
chmod +x "$bundle_root/bin/oasis7_chain_runtime"
printf 'runtime-v1\n' >"$node_root/releases/old/bin/oasis7_chain_runtime"
chmod +x "$node_root/releases/old/bin/oasis7_chain_runtime"
touch -t 202001010000 "$node_root/releases/old"
ln -s "$node_root/releases/old" "$node_root/current"
printf 'keep-data\n' >"$node_root/data/sentinel.txt"
printf 'keep-config\n' >"$node_root/config/sentinel.txt"
printf 'keep-backups\n' >"$node_root/backups/sentinel.txt"

for index in 1 2 3 4 5; do
  mkdir -p "$node_root/releases/retention-candidate-$index/bin"
  printf 'runtime-retention-%s\n' "$index" \
    >"$node_root/releases/retention-candidate-$index/bin/oasis7_chain_runtime"
  chmod +x "$node_root/releases/retention-candidate-$index/bin/oasis7_chain_runtime"
  touch -t "20260601010${index}" "$node_root/releases/retention-candidate-$index"
done

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

touch -t 202001010000 "$bundle_root"
tar -czf "$TMP_DIR/oasis7-linux-x64-bundle.tar.gz" -C "$TMP_DIR/bundle" oasis7-linux-x64

"$ROOT_DIR/scripts/p2p-public-testnet-package-node-upgrade.sh" \
  --node-root "$node_root" \
  --bundle-tar "$TMP_DIR/oasis7-linux-x64-bundle.tar.gz" \
  --package-version "$package_version" \
  --commit "$commit" \
  --run-id "$run_id" \
  --artifact-ref "testnet-package-linux-x64-$package_version/oasis7-linux-x64-bundle.tar.gz!/bin/oasis7_chain_runtime" \
  >/dev/null

node_root_abs=$(python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).resolve())' "$node_root")
expected_sha=$(shasum -a 256 "$node_root_abs/current/bin/oasis7_chain_runtime" | awk '{print $1}')

test "$(readlink "$node_root_abs/current")" = "$node_root_abs/releases/$package_version"
test -x "$node_root_abs/releases/$package_version/bin/oasis7_chain_runtime"
test -f "$node_root_abs/CURRENT_VERSION"
test -f "$node_root_abs/DEPLOYED_BUILDINFO"
grep -q "^package_version=$package_version$" "$node_root_abs/DEPLOYED_BUILDINFO"
grep -q "^commit=$commit$" "$node_root_abs/DEPLOYED_BUILDINFO"
release_count=$(find "$node_root_abs/releases" -mindepth 1 -maxdepth 1 -type d ! -name '.*' | wc -l | tr -d ' ')
test "$release_count" = "5"
test -d "$node_root_abs/releases/$package_version"
test -d "$node_root_abs/releases/old"
test -d "$node_root_abs/releases/retention-candidate-3"
test -d "$node_root_abs/releases/retention-candidate-4"
test -d "$node_root_abs/releases/retention-candidate-5"
test ! -e "$node_root_abs/releases/retention-candidate-1"
test ! -e "$node_root_abs/releases/retention-candidate-2"
test -f "$node_root_abs/data/sentinel.txt"
test -f "$node_root_abs/config/sentinel.txt"
test -f "$node_root_abs/backups/sentinel.txt"

jq -e \
  --arg expected "$expected_sha" \
  --arg commit "$commit" \
  --arg runtime "$node_root_abs/current/bin/oasis7_chain_runtime" \
  '.git_commit == $commit
    and .runtime_build.git_commit == $commit
    and .runtime_build.sha256 == $expected
    and .runtime_build.path == $runtime
    and .runtime_build.resolved_path == $runtime
    and (.runtime_build.ref | contains("testnet-package-linux-x64-"))' \
  "$node_root_abs/config/doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json" >/dev/null

directory_current_node="$TMP_DIR/directory-current-node"
mkdir -p "$directory_current_node/current/bin" "$directory_current_node/config/doc/testing/evidence"
printf 'runtime-v1\n' >"$directory_current_node/current/bin/oasis7_chain_runtime"
chmod +x "$directory_current_node/current/bin/oasis7_chain_runtime"
cp \
  "$node_root_abs/config/doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json" \
  "$directory_current_node/config/doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json"

"$ROOT_DIR/scripts/p2p-public-testnet-package-node-upgrade.sh" \
  --node-root "$directory_current_node" \
  --bundle-tar "$TMP_DIR/oasis7-linux-x64-bundle.tar.gz" \
  --package-version "$package_version" \
  --commit "$commit" \
  --run-id "$run_id" \
  --artifact-ref "testnet-package-linux-x64-$package_version/oasis7-linux-x64-bundle.tar.gz!/bin/oasis7_chain_runtime" \
  >/dev/null

directory_current_node_abs=$(python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).resolve())' "$directory_current_node")
test -L "$directory_current_node_abs/current"
test "$(readlink "$directory_current_node_abs/current")" = "$directory_current_node_abs/releases/$package_version"
test -x "$directory_current_node_abs/current/bin/oasis7_chain_runtime"
test "$(find "$directory_current_node_abs" -maxdepth 1 -type d -name 'current-pre-*.dir' | wc -l | tr -d ' ')" = "1"
test ! -e "$directory_current_node_abs/current/oasis7-linux-x64"

missing_bundle_node="$TMP_DIR/missing-bundle-node"
mkdir -p "$missing_bundle_node/releases/old/bin" "$missing_bundle_node/config"
printf 'runtime-v1\n' >"$missing_bundle_node/releases/old/bin/oasis7_chain_runtime"
chmod +x "$missing_bundle_node/releases/old/bin/oasis7_chain_runtime"
ln -s "$missing_bundle_node/releases/old" "$missing_bundle_node/current"
missing_bundle_node_abs=$(python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).resolve())' "$missing_bundle_node")
set +e
"$ROOT_DIR/scripts/p2p-public-testnet-package-node-upgrade.sh" \
  --node-root "$missing_bundle_node" \
  --bundle-tar "$TMP_DIR/oasis7-linux-x64-bundle.tar.gz" \
  --package-version "$package_version" \
  --commit "$commit" \
  --run-id "$run_id" \
  >/tmp/oasis7-package-node-upgrade-negative.out 2>&1
negative_status=$?
set -e
test "$negative_status" -ne 0
grep -q "no governed bootstrap bundle found" /tmp/oasis7-package-node-upgrade-negative.out
negative_current=$(python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).resolve())' "$missing_bundle_node_abs/current")
test "$negative_current" = "$missing_bundle_node_abs/releases/old"

echo "ok: package node upgrade pins current runtime hash into governed bootstrap bundle"
