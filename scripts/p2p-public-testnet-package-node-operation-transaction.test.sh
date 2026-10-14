#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPT="$ROOT_DIR/scripts/p2p-public-testnet-package-node-upgrade.sh"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-package-operation-transaction.XXXXXX")"
trap 'if [[ -n "${stage_pid:-}" ]]; then kill -KILL "$stage_pid" 2>/dev/null || true; fi; rm -rf "$TMP_DIR"' EXIT

# Operation-mode stage must pass the exact current-release snapshot captured
# by request_for into the immutable runtime-plan validator. Keep this wiring
# covered independently of the fake Linux fixtures below, which intentionally
# stop before a valid runtime descriptor can be admitted.
python3 - "$SCRIPT" <<'PY'
import ast
import sys
from pathlib import Path

source = Path(sys.argv[1]).read_text(encoding="utf-8")
marker = "  python3 - \"$@\" <<'PY'\n"
start = source.index(marker, source.index("operation_transaction_tool() {")) + len(marker)
end = source.index("\nPY\n}", start)
module = ast.parse(source[start:end])
functions = {
    node.name: node
    for node in module.body
    if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
}
validator = functions["validate_runtime_compatibility_plan"]
parameters = [argument.arg for argument in validator.args.args]
assert "current_binding" in parameters, "runtime-plan validator must receive the captured release binding"

requester = functions["request_for"]
calls = [
    node
    for node in ast.walk(requester)
    if isinstance(node, ast.Call)
    and isinstance(node.func, ast.Name)
    and node.func.id == "validate_runtime_compatibility_plan"
]
assert len(calls) == 1, "request_for must validate one immutable runtime plan"
binding_index = parameters.index("current_binding")
assert len(calls[0].args) > binding_index and isinstance(calls[0].args[binding_index], ast.Name) and calls[0].args[binding_index].id == "current_binding", (
    "request_for must pass its exact validated current_binding into the plan validator"
)
PY

FAKE_BIN="$TMP_DIR/fake-bin"
mkdir -p "$FAKE_BIN"
cat >"$FAKE_BIN/dpkg-deb" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  --build)
    source="${@: -2:1}"
    output="${@: -1}"
    tar -czf "$output" -C "$source" .
    ;;
  --extract)
    source="${2:?missing source}"
    destination="${3:?missing destination}"
    mkdir -p "$destination"
    tar -xzf "$source" -C "$destination"
    ;;
  *)
    echo "unsupported dpkg-deb fixture operation" >&2
    exit 2
    ;;
esac
SH
cat >"$FAKE_BIN/systemctl" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf 'systemctl %s\n' "$*" >>"${SYSTEMCTL_LOG:?}"
SH
chmod +x "$FAKE_BIN/dpkg-deb" "$FAKE_BIN/systemctl"

package_version="0.0.0+operation-transaction"
commit="abcdef1234567890abcdef1234567890abcdef12"
run_id="50123"
operation_id="op-node-17-upgrade-1"
action_id="action-17-1"
expected_instance_id="node-instance-17"
expected_generation="9"
node_root="$TMP_DIR/node"
bundle_dir="$TMP_DIR/package-source"
ops_dir="$TMP_DIR/ops-source/oasis7-linux-x64-ops-tools"
ops_tools_tar="$TMP_DIR/oasis7-linux-x64-ops-tools.tar.gz"
systemctl_log="$TMP_DIR/systemctl.log"
export SYSTEMCTL_LOG="$systemctl_log"

make_node() {
  mkdir -p "$node_root/releases/old/bin" "$node_root/config/doc/testing/evidence"
  printf 'runtime-before\n' >"$node_root/releases/old/bin/oasis7_chain_runtime"
  chmod +x "$node_root/releases/old/bin/oasis7_chain_runtime"
  ln -s "$node_root/releases/old" "$node_root/current"
  printf 'old-version\n' >"$node_root/CURRENT_VERSION"
  printf 'old-buildinfo\n' >"$node_root/DEPLOYED_BUILDINFO"
  cat >"$node_root/config/doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json" <<'EOF'
{
  "schema_version": "oasis7.release_candidate_bundle.v1",
  "git_commit": "old",
  "runtime_build": {"path": "old", "ref": "old", "resolved_path": "old", "sha256": "0000000000000000000000000000000000000000000000000000000000000000", "size_bytes": 1}
}
EOF
}

make_bundle() {
  local root=$1 runtime_contents=$2
  mkdir -p "$root/oasis7-linux-x64/bin"
  printf '%s\n' "$runtime_contents" >"$root/oasis7-linux-x64/bin/oasis7_chain_runtime"
  chmod +x "$root/oasis7-linux-x64/bin/oasis7_chain_runtime"
  cat >"$root/oasis7-linux-x64/BUILDINFO" <<EOF
workflow=Testnet Packages
commit=$commit
package_version=$package_version
run_id=$run_id
platform=linux-x64
EOF
  (cd "$root/oasis7-linux-x64" && shasum -a 256 BUILDINFO bin/oasis7_chain_runtime >SHA256SUMS)
  mkdir -p "$root/deb-root/DEBIAN" "$root/deb-root/opt/oasis7"
  cp -a "$root/oasis7-linux-x64/." "$root/deb-root/opt/oasis7/"
  cat >"$root/deb-root/DEBIAN/control" <<'EOF'
Package: oasis7
Version: 0.0.0
Section: games
Priority: optional
Architecture: amd64
Description: package operation transaction fixture
EOF
  PATH="$FAKE_BIN:$PATH" dpkg-deb --build --root-owner-group "$root/deb-root" "$root/oasis7-linux-x64.deb" >/dev/null
}

make_ops_tools() {
  mkdir -p "$ops_dir/bin"
  for binary in oasis7_world_repair_rebuild oasis7_governance_registry_import oasis7_governance_registry_audit service-readback; do
    printf '#!/usr/bin/env bash\nexit 0\n' >"$ops_dir/bin/$binary"
    chmod +x "$ops_dir/bin/$binary"
  done
  printf '{"opsToolsSchemaVersion":1}\n' >"$ops_dir/.oasis7-ops-tools-manifest.json"
  (cd "$ops_dir" && shasum -a 256 .oasis7-ops-tools-manifest.json bin/* >SHA256SUMS)
  tar -czf "$ops_tools_tar" -C "$(dirname "$ops_dir")" oasis7-linux-x64-ops-tools
}

make_node
mkdir -p "$bundle_dir" "$(dirname "$ops_dir")"
make_bundle "$bundle_dir" runtime-staged-v1
make_ops_tools

stage_args=(
  --operation-mode stage
  --operation-id "$operation_id"
  --action-id "$action_id"
  --expected-instance-id "$expected_instance_id"
  --expected-generation "$expected_generation"
  --node-root "$node_root"
  --package-deb "$bundle_dir/oasis7-linux-x64.deb"
  --ops-tools-tar "$ops_tools_tar"
  --package-version "$package_version"
  --commit "$commit"
  --run-id "$run_id"
)

current_before="$(readlink "$node_root/current")"
config_path="$node_root/config/doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json"
config_before="$(shasum -a 256 "$config_path" | awk '{print $1}')"
version_before="$(shasum -a 256 "$node_root/CURRENT_VERSION" | awk '{print $1}')"

# Stop the staging process after its atomic transaction-directory rename and
# parent-directory fsync, but before it can print a result.  This models a
# SIGKILL in the receipt-to-caller gap without a production test hook.
REAL_PYTHON3="$(command -v python3)"
export REAL_PYTHON3
export KILL_MARKER="$TMP_DIR/stage-committed.marker"
cat >"$FAKE_BIN/python3" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
python_source="$(mktemp "${TMPDIR:-/tmp}/oasis7-python-wrapper.XXXXXX")"
cat >"$python_source"
if [[ "${1:-}" == "-" && "${2:-}" == "persist" && ! -e "$KILL_MARKER" ]]; then
  python_output="$(mktemp "${TMPDIR:-/tmp}/oasis7-python-output.XXXXXX")"
  wrapper_parent=$PPID
  "$REAL_PYTHON3" "$@" <"$python_source" >"$python_output"
  : >"$KILL_MARKER"
  kill -STOP "$wrapper_parent"
  rm -f "$python_output"
else
  "$REAL_PYTHON3" "$@" <"$python_source"
fi
rm -f "$python_source"
SH
chmod +x "$FAKE_BIN/python3"
export PATH="$FAKE_BIN:$PATH"
"$SCRIPT" "${stage_args[@]}" >"$TMP_DIR/stage-first.out" 2>"$TMP_DIR/stage-first.err" &
stage_pid=$!
for _ in $(seq 1 500); do
  [[ -f "$KILL_MARKER" ]] && break
  kill -0 "$stage_pid" 2>/dev/null || break
  sleep 0.02
done
test -f "$KILL_MARKER" || {
  cat "$TMP_DIR/stage-first.err" >&2
  printf 'staging did not reach the durable post-commit boundary\n' >&2
  exit 1
}
kill -KILL "$stage_pid"
wait "$stage_pid" 2>/dev/null || true
stage_pid=""
test ! -s "$TMP_DIR/stage-first.out"
test -L "$node_root/current"
test "$(readlink "$node_root/current")" = "$current_before"
test "$(shasum -a 256 "$config_path" | awk '{print $1}')" = "$config_before"
test "$(shasum -a 256 "$node_root/CURRENT_VERSION" | awk '{print $1}')" = "$version_before"
test ! -e "$systemctl_log"

operation_record="$node_root/package-upgrade-operations/$operation_id"
test -d "$operation_record"
operation_mode="$(python3 - "$operation_record" <<'PY'
import stat
import sys
from pathlib import Path
print(stat.S_IMODE(Path(sys.argv[1]).stat().st_mode))
PY
)"
test "$operation_mode" = "448" # 0700
operation_digest="$(python3 - "$operation_record/operation.json" <<'PY'
import json
import sys
from pathlib import Path
print(json.loads(Path(sys.argv[1]).read_bytes())["request_digest"])
PY
)"
receipt_after_kill="$TMP_DIR/receipt-after-kill.json"
"$SCRIPT" \
  --operation-mode query \
  --operation-id "$operation_id" \
  --action-id "$action_id" \
  --operation-request-digest "$operation_digest" \
  --node-root "$node_root" >"$receipt_after_kill"
cmp "$operation_record/receipt.json" "$receipt_after_kill"
python3 - "$receipt_after_kill" "$operation_digest" <<'PY'
import json
import sys
from pathlib import Path
receipt = json.loads(Path(sys.argv[1]).read_bytes())
assert receipt["phase"] == "staged"
assert receipt["outcome"] == "staged"
assert receipt["request_digest"] == sys.argv[2]
assert receipt["expected_instance_id"] == "node-instance-17"
assert receipt["expected_generation"] == 9
PY

if "$SCRIPT" --operation-mode query --operation-id "$operation_id" --action-id "$action_id" --operation-request-digest "0000000000000000000000000000000000000000000000000000000000000000" --node-root "$node_root" >"$TMP_DIR/wrong-digest.out" 2>"$TMP_DIR/wrong-digest.err"; then
  printf 'query unexpectedly accepted a different immutable request digest\n' >&2
  exit 1
fi
grep -q 'request digest does not match' "$TMP_DIR/wrong-digest.err"

# Repeating the same operation/action and immutable request returns the exact
# original receipt and does not copy, promote, or start anything again.
"$SCRIPT" "${stage_args[@]}" --operation-request-digest "$operation_digest" >"$TMP_DIR/stage-retry.out"
cmp "$receipt_after_kill" "$TMP_DIR/stage-retry.out"

# A new package under the same operation identity is a conflict.
alternate_bundle="$TMP_DIR/alternate-package"
mkdir -p "$alternate_bundle"
make_bundle "$alternate_bundle" runtime-staged-v2
if "$SCRIPT" \
  --operation-mode stage --operation-id "$operation_id" --action-id "$action_id" \
  --expected-instance-id "$expected_instance_id" --expected-generation "$expected_generation" \
  --node-root "$node_root" --package-deb "$alternate_bundle/oasis7-linux-x64.deb" \
  --ops-tools-tar "$ops_tools_tar" --package-version "$package_version" \
  --commit "$commit" --run-id "$run_id" \
  >"$TMP_DIR/changed-package.out" 2>"$TMP_DIR/changed-package.err"; then
  printf 'operation ID reuse with a different package unexpectedly succeeded\n' >&2
  exit 1
fi
grep -q 'different package or ops-tools inputs' "$TMP_DIR/changed-package.err" || {
  cat "$TMP_DIR/changed-package.err" >&2
  exit 1
}

# Live configuration is part of the preimage.  Drift blocks both query and a
# retry; restoring the exact bytes makes the original receipt readable again.
cp "$config_path" "$TMP_DIR/config.backup"
printf '\n' >>"$config_path"
if "$SCRIPT" --operation-mode query --operation-id "$operation_id" --action-id "$action_id" --operation-request-digest "$operation_digest" --node-root "$node_root" >"$TMP_DIR/drift-query.out" 2>"$TMP_DIR/drift-query.err"; then
  printf 'query unexpectedly accepted a changed live configuration preimage\n' >&2
  exit 1
fi
grep -q 'preimage drifted' "$TMP_DIR/drift-query.err"
if "$SCRIPT" "${stage_args[@]}" >"$TMP_DIR/drift-stage.out" 2>"$TMP_DIR/drift-stage.err"; then
  printf 'stage retry unexpectedly accepted a changed live preimage\n' >&2
  exit 1
fi
grep -q 'preimage drifted' "$TMP_DIR/drift-stage.err"
cp "$TMP_DIR/config.backup" "$config_path"

if "$SCRIPT" \
  --operation-mode promote \
  --operation-id "$operation_id" \
  --action-id "$action_id" \
  --node-root "$node_root" >"$TMP_DIR/promote.out" 2>"$TMP_DIR/promote.err"; then
  printf 'unsupported promotion unexpectedly succeeded\n' >&2
  exit 1
fi
grep -q 'exact stopped-instance evidence is not available' "$TMP_DIR/promote.err"
test "$(readlink "$node_root/current")" = "$current_before"
test ! -e "$systemctl_log"

# A damaged receipt is never treated as a status-shaped success.
printf 'corruption\n' >>"$operation_record/receipt.json"
if "$SCRIPT" --operation-mode query --operation-id "$operation_id" --action-id "$action_id" --operation-request-digest "$operation_digest" --node-root "$node_root" >"$TMP_DIR/corrupt.out" 2>"$TMP_DIR/corrupt.err"; then
  printf 'query unexpectedly accepted a corrupted receipt\n' >&2
  exit 1
fi
grep -q 'receipt is corrupt' "$TMP_DIR/corrupt.err"

# This generic Linux fixture has no executable-derived runtime compatibility
# plan. It verifies that an unsupported stage is rejected before any effects.
linux_node_root="$TMP_DIR/linux-node"
linux_operation_id="op-linux-upgrade-2"
linux_action_id="pkg-$linux_operation_id-stage"
linux_instance="linux-incarnation-17"
linux_config_digest="$(printf 'linux config v1' | shasum -a 256 | awk '{print $1}')"
linux_operation_record_digest="$(printf 'immutable OperationRecord v1' | shasum -a 256 | awk '{print $1}')"
linux_lifecycle_data_dir="$TMP_DIR/linux-lifecycle-data"
mkdir -p "$linux_node_root/releases/old/bin" "$linux_node_root/config/doc/testing/evidence" "$linux_lifecycle_data_dir/actions"
chmod 700 "$linux_lifecycle_data_dir" "$linux_lifecycle_data_dir/actions"
linux_node_root="$(cd "$linux_node_root" && pwd -P)"
printf 'old-linux-runtime\n' >"$linux_node_root/releases/old/bin/oasis7_chain_runtime"
chmod +x "$linux_node_root/releases/old/bin/oasis7_chain_runtime"
old_linux_runtime_sha256="$(shasum -a 256 "$linux_node_root/releases/old/bin/oasis7_chain_runtime" | awk '{print $1}')"
cat >"$linux_node_root/releases/old/BUILDINFO" <<EOF
workflow=Testnet Packages
commit=abcdefabcdefabcdefabcdefabcdefabcdefabcd
package_version=old-linux-release
run_id=old-run
platform=linux-x64
runtime_sha256=$old_linux_runtime_sha256
EOF
ln -s "$linux_node_root/releases/old" "$linux_node_root/current"
printf 'old-version\n' >"$linux_node_root/CURRENT_VERSION"
printf 'old-buildinfo\n' >"$linux_node_root/DEPLOYED_BUILDINFO"
cp "$config_path" "$linux_node_root/config/doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json"
python3 - "$linux_lifecycle_data_dir" "$linux_instance" "$linux_config_digest" "$linux_operation_id" <<'PY'
import hashlib
import json
import os
import sys
from pathlib import Path

data = Path(sys.argv[1])
incarnation = sys.argv[2]
config_digest = sys.argv[3]
operation_id = sys.argv[4]
start_id = "start-linux-17"
stop_id = f"op:{operation_id}:stop"
instance = {
    "api_version": "oasis7.local-node-instance/v1",
    "action_id": start_id,
    "generation": 9,
    "incarnation": incarnation,
    "config_digest": config_digest,
    "supervisor_pid": 91,
    "runtime_pid": 92,
    "started_unix_ms": 100,
    "state": "running",
    "exit_code": None,
}
(data / "instance.json").write_text(json.dumps(instance, indent=2) + "\n")
os.chmod(data / "instance.json", 0o600)
stop_payload = ["stop", incarnation, 9, config_digest]
target = dict(instance)
target.update({"runtime_pid": None, "state": "stopped", "exit_code": 0})
stop = {
    "api_version": "oasis7.local-node-action/v1",
    "action_id": stop_id,
    "payload_digest": hashlib.sha256(json.dumps(stop_payload, separators=(",", ":")).encode()).hexdigest(),
    "kind": "stop",
    "expected_generation": 9,
    "target": target,
}
action_name = hashlib.sha256(json.dumps(stop_id, separators=(",", ":")).encode()).hexdigest() + ".json"
(data / "actions" / action_name).write_text(json.dumps(stop, indent=2) + "\n")
os.chmod(data / "actions" / action_name, 0o600)
instance.update({"runtime_pid": None, "state": "stopped", "exit_code": 0})
(data / "instance.json").write_text(json.dumps(instance, indent=2) + "\n")
os.chmod(data / "instance.json", 0o600)
PY
# Keep the independent owner live to verify that this unsupported request
# does not stop it or append an action receipt.
python3 - "$linux_lifecycle_data_dir/instance.json" <<'PY'
import json, sys
from pathlib import Path
path = Path(sys.argv[1])
value = json.loads(path.read_bytes())
value.update({"runtime_pid": 92, "state": "running", "exit_code": None})
path.write_text(json.dumps(value, indent=2) + "\n")
PY
chmod 600 "$linux_lifecycle_data_dir/instance.json"
linux_invalid_node_root="$TMP_DIR/linux-invalid-current-directory"
linux_invalid_operation_id="op-linux-invalid-current-directory"
linux_invalid_action_id="pkg-$linux_invalid_operation_id-stage"
mkdir -p "$linux_invalid_node_root/current/bin" "$linux_invalid_node_root/config/doc/testing/evidence"
cp "$linux_node_root/releases/old/bin/oasis7_chain_runtime" "$linux_invalid_node_root/current/bin/oasis7_chain_runtime"
cp "$linux_node_root/releases/old/BUILDINFO" "$linux_invalid_node_root/current/BUILDINFO"
cp "$config_path" "$linux_invalid_node_root/config/doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json"
printf 'old-version\n' >"$linux_invalid_node_root/CURRENT_VERSION"
printf 'old-buildinfo\n' >"$linux_invalid_node_root/DEPLOYED_BUILDINFO"
linux_invalid_current_runtime_sha256="$(shasum -a 256 "$linux_invalid_node_root/current/bin/oasis7_chain_runtime" | awk '{print $1}')"
linux_invalid_config_sha256="$(shasum -a 256 "$linux_invalid_node_root/config/doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json" | awk '{print $1}')"
printf '{}\n' >"$TMP_DIR/runtime-compatibility-plan.json"
runtime_compatibility_plan_sha256="$(shasum -a 256 "$TMP_DIR/runtime-compatibility-plan.json" | awk '{print $1}')"
linux_instance_sha256_before="$(shasum -a 256 "$linux_lifecycle_data_dir/instance.json" | awk '{print $1}')"
linux_actions_before="$(find "$linux_lifecycle_data_dir/actions" -type f -print | sort | shasum -a 256 | awk '{print $1}')"
if "$SCRIPT" \
  --operation-mode stage --operation-id "$linux_invalid_operation_id" --action-id "$linux_invalid_action_id" \
  --operation-record-digest "$(printf '%s' "$linux_invalid_operation_id" | shasum -a 256 | awk '{print $1}')" \
  --operation-profile public_testnet_linux_package_v1 --lifecycle-data-dir "$linux_lifecycle_data_dir" \
  --expected-instance-id "$linux_instance" --expected-incarnation "$linux_instance" --expected-generation 9 \
  --expected-config-digest "$linux_config_digest" --expected-instance-action-id start-linux-17 \
  --node-root "$linux_invalid_node_root" --package-deb "$bundle_dir/oasis7-linux-x64.deb" \
  --ops-tools-tar "$ops_tools_tar" --package-version "$package_version" --commit "$commit" --run-id "$run_id" \
  --runtime-compatibility-plan "$TMP_DIR/runtime-compatibility-plan.json" \
  --runtime-compatibility-plan-sha256 "$runtime_compatibility_plan_sha256" \
  >"$TMP_DIR/linux-invalid-current.out" 2>"$TMP_DIR/linux-invalid-current.err"; then
  printf 'Linux package stage unexpectedly accepted directory current\n' >&2
  exit 1
fi
if ! grep -q 'package current must be a symlink to a valid prior release' "$TMP_DIR/linux-invalid-current.err"; then
  cat "$TMP_DIR/linux-invalid-current.err" >&2
  printf 'Linux invalid-current fixture was rejected at an unexpected boundary\n' >&2
  exit 1
fi
test -d "$linux_invalid_node_root/current"
test ! -L "$linux_invalid_node_root/current"
test "$(shasum -a 256 "$linux_invalid_node_root/current/bin/oasis7_chain_runtime" | awk '{print $1}')" = "$linux_invalid_current_runtime_sha256"
test "$(shasum -a 256 "$linux_invalid_node_root/config/doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json" | awk '{print $1}')" = "$linux_invalid_config_sha256"
test "$(cat "$linux_invalid_node_root/CURRENT_VERSION")" = "old-version"
test "$(cat "$linux_invalid_node_root/DEPLOYED_BUILDINFO")" = "old-buildinfo"
test ! -e "$linux_invalid_node_root/package-upgrade-operations"
test "$(shasum -a 256 "$linux_lifecycle_data_dir/instance.json" | awk '{print $1}')" = "$linux_instance_sha256_before"
test "$(find "$linux_lifecycle_data_dir/actions" -type f -print | sort | shasum -a 256 | awk '{print $1}')" = "$linux_actions_before"
test ! -e "$systemctl_log"

linux_invalid_runtime_node_root="$TMP_DIR/linux-invalid-runtime-symlink"
mkdir -p "$linux_invalid_runtime_node_root/releases/old/bin" "$linux_invalid_runtime_node_root/config/doc/testing/evidence"
linux_invalid_runtime_node_root="$(cd "$linux_invalid_runtime_node_root" && pwd -P)"
printf 'external linux runtime\n' >"$TMP_DIR/external-linux-runtime"
chmod +x "$TMP_DIR/external-linux-runtime"
linux_external_runtime_sha256="$(shasum -a 256 "$TMP_DIR/external-linux-runtime" | awk '{print $1}')"
ln -s "$TMP_DIR/external-linux-runtime" "$linux_invalid_runtime_node_root/releases/old/bin/oasis7_chain_runtime"
cat >"$linux_invalid_runtime_node_root/releases/old/BUILDINFO" <<EOF
workflow=Testnet Packages
commit=abcdefabcdefabcdefabcdefabcdefabcdefabcd
package_version=old-linux-release
run_id=old-run
platform=linux-x64
runtime_sha256=$linux_external_runtime_sha256
EOF
ln -s "$linux_invalid_runtime_node_root/releases/old" "$linux_invalid_runtime_node_root/current"
cp "$config_path" "$linux_invalid_runtime_node_root/config/doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json"
if "$SCRIPT" \
  --operation-mode stage --operation-id op-linux-invalid-runtime-symlink --action-id pkg-linux-invalid-runtime-symlink-stage \
  --operation-record-digest "$(printf '%s' op-linux-invalid-runtime-symlink | shasum -a 256 | awk '{print $1}')" \
  --operation-profile public_testnet_linux_package_v1 --lifecycle-data-dir "$linux_lifecycle_data_dir" \
  --expected-instance-id "$linux_instance" --expected-incarnation "$linux_instance" --expected-generation 9 \
  --expected-config-digest "$linux_config_digest" --expected-instance-action-id start-linux-17 \
  --node-root "$linux_invalid_runtime_node_root" --package-deb "$bundle_dir/oasis7-linux-x64.deb" \
  --ops-tools-tar "$ops_tools_tar" --package-version "$package_version" --commit "$commit" --run-id "$run_id" \
  --runtime-compatibility-plan "$TMP_DIR/runtime-compatibility-plan.json" \
  --runtime-compatibility-plan-sha256 "$runtime_compatibility_plan_sha256" \
  >"$TMP_DIR/linux-invalid-runtime.out" 2>"$TMP_DIR/linux-invalid-runtime.err"; then
  printf 'Linux package stage followed a symlink inside the selected release\n' >&2
  exit 1
fi
if ! grep -q 'package runtime is unavailable without following symlinks' "$TMP_DIR/linux-invalid-runtime.err"; then
  cat "$TMP_DIR/linux-invalid-runtime.err" >&2
  printf 'Linux invalid-runtime fixture was rejected at an unexpected boundary\n' >&2
  exit 1
fi
test -L "$linux_invalid_runtime_node_root/releases/old/bin/oasis7_chain_runtime"
test ! -e "$linux_invalid_runtime_node_root/package-upgrade-operations"
test ! -e "$linux_invalid_runtime_node_root/CURRENT_VERSION"
test ! -e "$systemctl_log"

printf 'SKIP: Linux promotion and rollback require a real executable compatibility plan; this fixture verified fail-closed admission only.\n'
