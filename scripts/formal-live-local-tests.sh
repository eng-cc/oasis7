#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=${1:-$(cd "$script_dir/.." && pwd)}
cd "$repo_root"
case "$(uname -s)" in
  Linux|Darwin) ;;
  *) echo "formal native process tests require Linux or Darwin" >&2; exit 2 ;;
esac
formal_python=$(bash "$script_dir/find-python-with-module.sh" tomllib)
endpoint_manifest=$(mktemp)
trap 'rm -f "$endpoint_manifest"' EXIT

# Native closure verification uses the optimized child artifact. The Rust test
# harness remains the ordinary debug/test artifact, with its original deadlines.
env -u RUSTC_WRAPPER cargo build -p oasis7_distfs --bin oasis7_durability_endpoint --release --message-format=json > "$endpoint_manifest"
endpoint_binary=$("$formal_python" - "$endpoint_manifest" <<'PY_ARTIFACT'
import json, pathlib, sys
artifacts = []
for line in pathlib.Path(sys.argv[1]).read_text().splitlines():
    if not line.startswith('{'):
        continue
    artifact = json.loads(line)
    if (artifact.get('reason') == 'compiler-artifact'
        and artifact.get('target', {}).get('name') == 'oasis7_durability_endpoint'
        and artifact.get('profile', {}).get('test') is False
        and artifact.get('profile', {}).get('opt_level') == '3'
        and artifact.get('profile', {}).get('debug_assertions') is False
        and artifact.get('executable')
        and pathlib.Path(artifact['executable']).parent.name == 'release'):
        artifacts.append(artifact['executable'])
if len(artifacts) != 1 or not pathlib.Path(artifacts[0]).is_file():
    raise SystemExit(f'expected exactly one built endpoint executable: {artifacts}')
print(artifacts[0])
PY_ARTIFACT
)
export OASIS7_FORMAL_ENDPOINT_TEST_BIN="$endpoint_binary"
# The ordinary lifecycle must also run against real production and fixture binaries.
for formal_build_kind in production fixture; do
  if [[ "$formal_build_kind" == production ]]; then
    env -u RUSTC_WRAPPER cargo build -p oasis7 --bin oasis7_chain_runtime --features test_tier_required --message-format=json > "$endpoint_manifest"
  else
    env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime --features test_tier_required --no-run --message-format=json > "$endpoint_manifest"
  fi
  formal_runtime_binary=$("$formal_python" - "$endpoint_manifest" "$formal_build_kind" <<'PY_RUNTIME'
import json, pathlib, sys
wanted_test = sys.argv[2] == 'fixture'
artifacts = []
for line in pathlib.Path(sys.argv[1]).read_text().splitlines():
    if not line.startswith('{'):
        continue
    value = json.loads(line)
    if (value.get('reason') == 'compiler-artifact'
        and value.get('target', {}).get('name') == 'oasis7_chain_runtime'
        and value.get('profile', {}).get('test') == wanted_test
        and value.get('executable')):
        artifacts.append(value['executable'])
if len(artifacts) != 1 or not pathlib.Path(artifacts[0]).is_file():
    raise SystemExit(f'expected exactly one runtime executable: {artifacts}')
print(artifacts[0])
PY_RUNTIME
)
  if [[ "$formal_build_kind" == production ]]; then
    export OASIS7_CHAIN_RUNTIME_BIN="$formal_runtime_binary"
  else
    export OASIS7_NODE_LIFECYCLE_TEST_BIN="$formal_runtime_binary"
  fi
done
export OASIS7_FORMAL_NODE_TEST_BIN="$OASIS7_CHAIN_RUNTIME_BIN"

# Build/test serially: the hardened protocol hashes the loaded executable image.
# Never replace a live child executable while its qualification is running.
env -u RUSTC_WRAPPER cargo test -p oasis7 --lib --features test_tier_required formal -- --test-threads=1
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime --features test_tier_required formal -- --test-threads=1
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime --features test_tier_required protected_endpoint_children -- --ignored --nocapture --test-threads=1
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime --features test_tier_required formal_protected_healthy_durable_set_ -- --ignored --test-threads=1
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime --features test_tier_required healthy_durable_set_owner_ -- --ignored --test-threads=1
env -u RUSTC_WRAPPER cargo test -p oasis7_distfs --lib formal -- --test-threads=1
env -u RUSTC_WRAPPER cargo test -p oasis7_distfs --lib formal_endpoint_child_process_dual_qualification_sigkill_reopen_original -- --ignored --test-threads=1
env -u RUSTC_WRAPPER cargo test -p oasis7_distfs --lib recovered_endpoint_children -- --ignored --test-threads=1
env -u RUSTC_WRAPPER cargo test -p oasis7_distfs --lib native_profile2_c10_over_64_records_and_16mib_reopens_and_retries_original_once -- --ignored --test-threads=1

env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime --features test_tier_required formal_world_initialization_cli_ -- --ignored --test-threads=1
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime --features test_tier_required formal_full_state_sync_cli -- --ignored --test-threads=1
bash "$script_dir/local-node-lifecycle.test.sh"
bash "$script_dir/legacy-observer-migration.test.sh"
