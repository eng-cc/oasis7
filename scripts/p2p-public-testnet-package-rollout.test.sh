#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-package-rollout-test.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

package_dir="$TMP_DIR/package"
bundle_src="$TMP_DIR/bundle/oasis7-linux-x64"
node_root="$TMP_DIR/node"
out_dir="$TMP_DIR/out"
package_version="0.0.0+testnet.89.419e119bc897"
commit="419e119bc897efaa34750bee04c63470d1156699"
run_id="27605906795"

mkdir -p "$package_dir/windows" "$bundle_src/bin" "$node_root/releases/old/bin" "$node_root/config/doc/testing/evidence"
printf 'runtime-v2\n' >"$bundle_src/bin/oasis7_chain_runtime"
chmod +x "$bundle_src/bin/oasis7_chain_runtime"
tar -czf "$package_dir/oasis7-linux-x64-bundle.tar.gz" -C "$TMP_DIR/bundle" oasis7-linux-x64
printf 'fake windows installer\n' >"$package_dir/windows/oasis7-windows-x64.exe"

cat >"$package_dir/linux-x64-BUILDINFO" <<EOF
workflow=Testnet Packages
run_id=$run_id
run_number=89
repository=eng-cc/oasis7
requested_ref=$commit
commit=$commit
build_profile=release
package_scope=all_existing
platform=linux-x64
package_version=$package_version
published=false
EOF

cat >"$package_dir/windows/windows-x64-BUILDINFO" <<EOF
workflow=Testnet Packages
run_id=$run_id
run_number=89
repository=eng-cc/oasis7
requested_ref=$commit
commit=$commit
build_profile=release
package_scope=all_existing
platform=windows-x64
package_version=$package_version
published=false
EOF

(
  cd "$package_dir"
  shasum -a 256 oasis7-linux-x64-bundle.tar.gz linux-x64-BUILDINFO >linux-x64-SHA256SUMS
  cd "$package_dir/windows"
  shasum -a 256 oasis7-windows-x64.exe windows-x64-BUILDINFO >windows-x64-SHA256SUMS
)

printf 'runtime-v1\n' >"$node_root/releases/old/bin/oasis7_chain_runtime"
chmod +x "$node_root/releases/old/bin/oasis7_chain_runtime"
ln -s "$node_root/releases/old" "$node_root/current"
cat >"$node_root/config/doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json" <<'EOF'
{
  "schema_version": "oasis7.release_candidate_bundle.v1",
  "runtime_build": {
    "path": "old",
    "ref": "old",
    "resolved_path": "old",
    "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
    "size_bytes": 1
  }
}
EOF

cat >"$TMP_DIR/manifest.json" <<EOF
{
  "nodes": [
    {
      "name": "local-linux",
      "platform": "linux-x64",
      "node_root": "$node_root",
      "restart": false,
      "status_url": "http://127.0.0.1:6632/v1/chain/status"
    },
    {
      "name": "remote-linux",
      "platform": "linux-x64",
      "host": "198.51.100.44",
      "user": "root",
      "node_root": "/opt/oasis7/p2p-testnet",
      "remote_bundle": "/tmp/oasis7-linux-x64-bundle.tar.gz",
      "remote_script": "/opt/oasis7/oasis7/scripts/p2p-public-testnet-package-node-upgrade.sh",
      "restart": true,
      "systemd_service": "oasis7-testnet-storage.service",
      "status_url": "http://127.0.0.1:6632/v1/chain/status"
    },
    {
      "name": "windows-observer",
      "platform": "windows-x64",
      "host": "192.0.2.33",
      "user": "Administrator",
      "deploy_root": "C:\\\\oasis7-deploy",
      "scheduled_task": "Oasis7Observer",
      "status_url": "http://127.0.0.1:5121/v1/chain/status"
    }
  ]
}
EOF

"$ROOT_DIR/scripts/p2p-public-testnet-package-rollout.py" \
  --manifest "$TMP_DIR/manifest.json" \
  --package-dir "$package_dir" \
  --out-dir "$TMP_DIR/plan-only-out" \
  --json >"$TMP_DIR/plan-only.json"

node_root_abs=$(python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).resolve())' "$node_root")
plan_current_target=$(python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).resolve())' "$(readlink "$node_root/current")")
test "$plan_current_target" = "$node_root_abs/releases/old"
jq -e '
  (.nodes[] | select(.name == "local-linux") | .applied == false)
  and (.nodes[] | select(.name == "remote-linux") | .commands[0] | startswith("scp "))
  and (.nodes[] | select(.name == "remote-linux") | .commands[1] | startswith("ssh root@198.51.100.44 "))
  and (.nodes[] | select(.name == "remote-linux") | .commands[1] | contains("--bundle-tar /tmp/oasis7-linux-x64-bundle.tar.gz"))
  and (.nodes[] | select(.name == "windows-observer") | .governed_bundle_path | endswith("public-testnet-governed-bootstrap-bundle-2026-06-06.windows.json"))' \
  "$TMP_DIR/plan-only.json" >/dev/null

"$ROOT_DIR/scripts/p2p-public-testnet-package-rollout.py" \
  --manifest "$TMP_DIR/manifest.json" \
  --package-dir "$package_dir" \
  --out-dir "$out_dir" \
  --apply-local \
  --json >"$TMP_DIR/plan.json"

test "$(readlink "$node_root_abs/current")" = "$node_root_abs/releases/$package_version"
grep -q "^package_version=$package_version$" "$node_root_abs/DEPLOYED_BUILDINFO"
jq -e \
  --arg commit "$commit" \
  --arg version "$package_version" \
  '.commit == $commit
    and .package_version == $version
    and .readiness_policy == "rpc-running"
    and (.nodes[] | select(.name == "local-linux") | .applied == true)
    and (.nodes[] | select(.name == "windows-observer") | .windows_script | endswith("windows-observer-windows-upgrade.ps1"))' \
  "$TMP_DIR/plan.json" >/dev/null

windows_script="$out_dir/windows-observer-windows-upgrade.ps1"
test -f "$windows_script"
python3 - "$windows_script" <<'PY'
from pathlib import Path
import sys

data = Path(sys.argv[1]).read_bytes()
assert not data.startswith(b"\xef\xbb\xbf"), "PowerShell script must be UTF-8 without BOM"
text = data.decode("utf-8")
assert "Set-JsonProperty $json.runtime_build 'sha256' $hash" in text
assert 'throw "governed bundle missing runtime_build' in text
assert "[System.Text.UTF8Encoding]::new($false)" in text
assert "Start-ScheduledTask -TaskName $taskName" in text
assert "public-testnet-governed-bootstrap-bundle-2026-06-06.windows.json" in text
assert "-Filter '*bundle*.json'" not in text
PY

"$ROOT_DIR/scripts/p2p-public-testnet-package-rollout.py" --help >/tmp/oasis7-package-rollout-help.out
grep -q "mode is plan-only" /tmp/oasis7-package-rollout-help.out
grep -q "Mutation requires" /tmp/oasis7-package-rollout-help.out
grep -q "never reads or stores credentials" /tmp/oasis7-package-rollout-help.out
grep -q -- "--readiness-policy" /tmp/oasis7-package-rollout-help.out

bad_package_dir="$TMP_DIR/bad-package"
cp -R "$package_dir" "$bad_package_dir"
sed -i.bak "s/^commit=.*/commit=0000000000000000000000000000000000000000/" \
  "$bad_package_dir/windows/windows-x64-BUILDINFO"
rm "$bad_package_dir/windows/windows-x64-BUILDINFO.bak"
(
  cd "$bad_package_dir/windows"
  shasum -a 256 oasis7-windows-x64.exe windows-x64-BUILDINFO >windows-x64-SHA256SUMS
)
if "$ROOT_DIR/scripts/p2p-public-testnet-package-rollout.py" \
  --manifest "$TMP_DIR/manifest.json" \
  --package-dir "$bad_package_dir" \
  --out-dir "$TMP_DIR/bad-out" \
  >"$TMP_DIR/bad.out" 2>"$TMP_DIR/bad.err"; then
  echo "expected mismatched BUILDINFO to fail" >&2
  exit 1
fi
grep -q "does not match" "$TMP_DIR/bad.err"

bad_sums_dir="$TMP_DIR/bad-sums-package"
cp -R "$package_dir" "$bad_sums_dir"
(
  cd "$bad_sums_dir"
  shasum -a 256 linux-x64-BUILDINFO >linux-x64-SHA256SUMS
)
if "$ROOT_DIR/scripts/p2p-public-testnet-package-rollout.py" \
  --manifest "$TMP_DIR/manifest.json" \
  --package-dir "$bad_sums_dir" \
  --out-dir "$TMP_DIR/bad-sums-out" \
  >"$TMP_DIR/bad-sums.out" 2>"$TMP_DIR/bad-sums.err"; then
  echo "expected missing asset checksum coverage to fail" >&2
  exit 1
fi
grep -q "checksum file does not cover required artifact: oasis7-linux-x64-bundle.tar.gz" "$TMP_DIR/bad-sums.err"

strict_node_root="$TMP_DIR/strict-node"
mkdir -p "$strict_node_root/releases/old/bin" "$strict_node_root/config/doc/testing/evidence"
printf 'runtime-v1\n' >"$strict_node_root/releases/old/bin/oasis7_chain_runtime"
chmod +x "$strict_node_root/releases/old/bin/oasis7_chain_runtime"
ln -s "$strict_node_root/releases/old" "$strict_node_root/current"
cp \
  "$node_root/config/doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json" \
  "$strict_node_root/config/doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json"
python3 - "$TMP_DIR/manifest.json" "$strict_node_root" <<'PY'
from pathlib import Path
import json
import sys

manifest = json.loads(Path(sys.argv[1]).read_text())
manifest["nodes"][0]["node_root"] = sys.argv[2]
manifest["nodes"][0]["restart"] = True
manifest["nodes"][0]["systemd_service"] = "oasis7-testnet-storage.service"
Path(sys.argv[1]).write_text(json.dumps(manifest, indent=2) + "\n")
PY
"$ROOT_DIR/scripts/p2p-public-testnet-package-rollout.py" \
  --manifest "$TMP_DIR/manifest.json" \
  --package-dir "$package_dir" \
  --out-dir "$TMP_DIR/strict-out" \
  --readiness-policy strict-ready \
  --json >"$TMP_DIR/strict-plan.json"
jq -e '
  .readiness_policy == "strict-ready"
  and (.nodes[] | select(.name == "local-linux") | .commands[0] | contains("--post-restart-status-url"))' \
  "$TMP_DIR/strict-plan.json" >/dev/null

echo "ok: package rollout helper validates artifacts and standardizes linux/windows replacement plans"
