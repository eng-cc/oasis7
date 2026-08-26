#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

workflow_toolchain_install=$(awk '
/^[[:space:]]*- name: Install pinned Rust toolchain$/ { in_step=1; next }
in_step && /^[[:space:]]*- name:/ { in_step=0 }
in_step && /rustup toolchain install/ { print; exit }
' .github/workflows/compile-metrics.yml)
if [[ "$workflow_toolchain_install" != *"--profile minimal"* ]]; then
  echo "compile metrics workflow must install Rust with the minimal profile" >&2
  exit 1
fi
if [[ "$workflow_toolchain_install" == *"--profile default"* ]]; then
  echo "compile metrics workflow must not install Rust with the default profile" >&2
  exit 1
fi

workflow_checkout_depth=$(awk '
/^[[:space:]]*- uses: actions\/checkout@v6$/ { in_step=1; next }
in_step && /^[[:space:]]*- name:/ { in_step=0 }
in_step && /fetch-depth:/ { print $2; exit }
' .github/workflows/compile-metrics.yml)
if [[ "$workflow_checkout_depth" != "1" ]]; then
  echo "compile metrics workflow must use a shallow current checkout" >&2
  exit 1
fi
if grep -Eq 'fetch-depth:[[:space:]]*0' .github/workflows/compile-metrics.yml; then
  echo "compile metrics workflow must not request a full-history checkout" >&2
  exit 1
fi
compile_metrics_timeout=$(awk '
/^  compile-metrics:/ { in_job=1; next }
in_job && /^  [^[:space:]]/ { exit }
in_job && /^[[:space:]]+timeout-minutes:/ { print $2; exit }
' .github/workflows/compile-metrics.yml)
if [[ "$compile_metrics_timeout" != "45" ]]; then
  echo "compile metrics matrix job must cap runner time at 45 minutes" >&2
  exit 1
fi
if ! grep -Eq '^[[:space:]]*CARGO_REGISTRIES_CRATES_IO_PROTOCOL:[[:space:]]*sparse[[:space:]]*$' .github/workflows/compile-metrics.yml; then
  echo "compile metrics workflow must pin the sparse crates.io protocol" >&2
  exit 1
fi
if grep -Eq 'COMPILE_METRICS_MAX_[A-Z0-9_]+' .github/workflows/compile-metrics.yml; then
  echo "compile metrics workflow must not define unused threshold environment defaults" >&2
  exit 1
fi
if ! grep -Eq '^[[:space:]]*- oasis7_default_features[[:space:]]*$' .github/workflows/compile-metrics.yml; then
  echo "compile metrics workflow must expose the oasis7 default-feature target" >&2
  exit 1
fi
workflow_oasis7_default_target=$(awk '
/^[[:space:]]*oasis7_default_features\)[[:space:]]*$/ { in_case=1; next }
in_case && /^[[:space:]]*;;/ { exit }
in_case { print }
' .github/workflows/compile-metrics.yml)
if [[ "$workflow_oasis7_default_target" != *'package="oasis7"'* ]]; then
  echo "oasis7 default-feature target must measure package oasis7" >&2
  exit 1
fi
if [[ "$workflow_oasis7_default_target" != *'extra_args+=(--check-only)'* ]]; then
  echo "oasis7 default-feature target must use check-only measurement" >&2
  exit 1
fi
if [[ "$workflow_oasis7_default_target" == *'--no-default-features'* ]]; then
  echo "oasis7 default-feature target must retain Cargo default features" >&2
  exit 1
fi
if ! grep -Eq 'git fetch --no-tags --depth=1 origin -- "\$\{BASELINE_REF\}"' .github/workflows/compile-metrics.yml; then
  echo "compile metrics workflow must fetch an optional baseline explicitly" >&2
  exit 1
fi
if ! grep -Eq "FETCH_HEAD\\^\\{commit\\}" .github/workflows/compile-metrics.yml; then
  echo "compile metrics workflow must resolve fetched baseline provenance" >&2
  exit 1
fi
workflow_baseline_fetch_step=$(awk '
/^[[:space:]]*- name: Fetch baseline ref$/ { in_step=1; next }
in_step && /^[[:space:]]*- name:/ { exit }
in_step { print }
' .github/workflows/compile-metrics.yml)
if [[ "$workflow_baseline_fetch_step" != *"if:"* || "$workflow_baseline_fetch_step" != *"baseline_ref !="* ]]; then
  echo "baseline fetch must be conditional on a requested baseline ref" >&2
  exit 1
fi
# These literals intentionally assert GitHub expressions instead of expanding
# them as Bash variables.
# shellcheck disable=SC2016
if [[ "$workflow_baseline_fetch_step" != *'BASELINE_REF: ${{ inputs.baseline_ref }}'* ]]; then
  echo "baseline fetch must pass the input through an environment variable" >&2
  exit 1
fi
workflow_baseline_resolver_step=$(awk '
/^[[:space:]]*- name: Resolve baseline ref$/ { in_step=1; next }
in_step && /^[[:space:]]*- name:/ { exit }
in_step { print }
' .github/workflows/compile-metrics.yml)
# shellcheck disable=SC2016
if [[ "$workflow_baseline_resolver_step" != *'BASELINE_REF: ${{ inputs.baseline_ref }}'* ]]; then
  echo "baseline resolver must pass the input through an environment variable" >&2
  exit 1
fi
# shellcheck disable=SC2016
if [[ "$workflow_baseline_resolver_step" != *'baseline_ref="${BASELINE_REF}"'* ]]; then
  echo "baseline resolver must read the environment variable literally" >&2
  exit 1
fi
# shellcheck disable=SC2016
if [[ "$workflow_baseline_resolver_step" == *'baseline_ref="${{ inputs.baseline_ref }}"'* ]]; then
  echo "baseline resolver must not interpolate workflow input into Bash" >&2
  exit 1
fi

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT

malicious_ref_marker="$tmp_dir/malicious-ref-marker"
malicious_ref="evil\$(touch ${malicious_ref_marker})"
resolved_malicious_ref=$(
  BASELINE_REF="$malicious_ref" \
    bash -c 'set -euo pipefail; baseline_ref="${BASELINE_REF}"; printf "%s" "$baseline_ref"'
)
if [[ "$resolved_malicious_ref" != "$malicious_ref" || -e "$malicious_ref_marker" ]]; then
  echo "baseline resolver must preserve malicious-looking refs literally" >&2
  exit 1
fi

option_ref_marker="$tmp_dir/option-ref-marker"
option_ref="--upload-pack=touch ${option_ref_marker}"
option_ref_repo="$tmp_dir/option-ref-repo"
option_ref_origin="$tmp_dir/option-ref-origin.git"
git init --bare --quiet "$option_ref_origin"
git init --quiet "$option_ref_repo"
git -C "$option_ref_repo" remote add origin "$option_ref_origin"
if git -C "$option_ref_repo" fetch --no-tags --depth=1 origin -- "$option_ref" 2>/dev/null; then
  echo "option-shaped baseline ref unexpectedly fetched" >&2
  exit 1
fi
if [[ -e "$option_ref_marker" ]]; then
  echo "baseline fetch must not execute option-shaped refs" >&2
  exit 1
fi

fake_bin="$tmp_dir/bin"
mkdir -p "$fake_bin"
host_target=$(rustc -vV | sed -n 's/^host: //p')
if [[ -z "$host_target" ]]; then
  echo "unable to resolve the Rust host target for the compile metrics contract" >&2
  exit 1
fi
cat >"$fake_bin/cargo" <<'FAKE_CARGO'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${FAKE_CARGO_LOG:?}"
printf '%s\n' "${CARGO_HOME:-<unset>}" >>"${FAKE_CARGO_HOME_LOG:?}"
printf '%s\n' "${CARGO_INCREMENTAL:-<unset>}" >>"${FAKE_CARGO_INCREMENTAL_LOG:?}"
printf '%s\n' "${CARGO_PROFILE_DEV_DEBUG:-<unset>}" >>"${FAKE_CARGO_PROFILE_DEV_DEBUG_LOG:?}"
printf '%s\n' "${CARGO_PROFILE_TEST_DEBUG:-<unset>}" >>"${FAKE_CARGO_PROFILE_TEST_DEBUG_LOG:?}"
printf '%s\n' "${RUSTC_WRAPPER:-<unset>}" >>"${FAKE_RUSTC_WRAPPER_LOG:?}"
if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
  printf '%s\n' "$CARGO_TARGET_DIR" >>"${FAKE_CARGO_TARGET_LOG:?}"
fi
case "${1:-}" in
  tree)
    if [[ "$*" != *"--offline"* || "$*" != *"--locked"* ]]; then
      echo "dependency-tree query must be locked and offline: $*" >&2
      exit 1
    fi
    if [[ "$*" == *"-i wasmtime"* || "$*" == *"-i oasis7_wasm_executor"* ]]; then
      exit 1
    fi
    case "${FAKE_CARGO_TREE_FIXTURE:-none}" in
      all)
        printf 'fake-package\nfake-dependency\nfake-dependency (*)\nwasmtime v99.0.0\noasis7_wasm_executor v0.1.0\n'
        ;;
      none)
        printf 'fake-package\nfake-dependency\n'
        ;;
      *)
        echo "unexpected dependency-tree fixture: ${FAKE_CARGO_TREE_FIXTURE}" >&2
        exit 1
        ;;
    esac
    ;;
  fetch)
    expected_target="${FAKE_HOST_TARGET:?}"
    if [[ "$*" != *"--locked"* || "$*" != *"--target ${expected_target}"* ]]; then
      echo "compile metrics fetch must be locked to the runner host target: $*" >&2
      exit 1
    fi
    ;;
  check)
    ;;
  build)
    mkdir -p "${CARGO_TARGET_DIR:?}/release"
    : >"${CARGO_TARGET_DIR}/release/fake_library"
    ;;
  *)
    echo "unexpected cargo command: $*" >&2
    exit 1
    ;;
esac
FAKE_CARGO
chmod +x "$fake_bin/cargo"

cat >"$fake_bin/git" <<'FAKE_GIT'
#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "-C" && "${3:-}" == "rev-parse" ]]; then
  printf '%s\n' "${FAKE_COMMIT_OID:?}"
  exit 0
fi
if [[ "${1:-}" == "worktree" && "${2:-}" == "add" ]]; then
  mkdir -p "${4:?}"
  exit 0
fi
echo "unexpected git command: $*" >&2
exit 1
FAKE_GIT
chmod +x "$fake_bin/git"

out_dir="$tmp_dir/metrics"
expected_commit_oid=$(git rev-parse HEAD)
FAKE_CARGO_LOG="$tmp_dir/cargo.log" \
FAKE_CARGO_HOME_LOG="$tmp_dir/cargo-home.log" \
FAKE_CARGO_INCREMENTAL_LOG="$tmp_dir/cargo-incremental.log" \
FAKE_CARGO_PROFILE_DEV_DEBUG_LOG="$tmp_dir/cargo-profile-dev-debug.log" \
FAKE_CARGO_PROFILE_TEST_DEBUG_LOG="$tmp_dir/cargo-profile-test-debug.log" \
FAKE_RUSTC_WRAPPER_LOG="$tmp_dir/cargo-rustc-wrapper.log" \
FAKE_CARGO_TARGET_LOG="$tmp_dir/cargo-target.log" \
FAKE_HOST_TARGET="$host_target" \
FAKE_COMMIT_OID="$expected_commit_oid" \
FAKE_CARGO_TREE_FIXTURE=all \
CARGO_INCREMENTAL=1 \
CARGO_PROFILE_DEV_DEBUG=2 \
CARGO_PROFILE_TEST_DEBUG=2 \
RUSTC_WRAPPER=hostile-wrapper \
PATH="$fake_bin:$PATH" \
  ./scripts/ci-compile-metrics.sh \
    --package fake_library \
    --out-dir "$out_dir" \
    --check-only \
    --no-default-features

python3 - "$out_dir/current.metrics.json" "$out_dir/comparison.json" "$out_dir/summary.md" "$expected_commit_oid" <<'PY'
import json
from pathlib import Path
import sys

metrics = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
comparison = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
summary = Path(sys.argv[3]).read_text(encoding="utf-8")
expected_commit_oid = sys.argv[4]
assert metrics["package_count"] == 4
assert metrics["commit_oid"] == expected_commit_oid
assert metrics["binary"] is None
assert metrics["check_only"] is True
assert metrics["no_default_features"] is True
assert metrics["wasmtime_present"] is True
assert metrics["wasm_executor_present"] is True
assert metrics["cargo_check_seconds"] >= 0
assert metrics["cargo_build_release_seconds"] is None
assert metrics["release_binary_bytes"] is None
assert comparison["metric_rows"] == []
assert comparison["current_commit_oid"] == expected_commit_oid
assert comparison["baseline_commit_oid"] is None
assert "not measured (check-only package)" in summary
assert expected_commit_oid in summary
PY

if grep -Eq 'build' "$tmp_dir/cargo.log"; then
  echo "check-only compile metrics unexpectedly invoked cargo build" >&2
  exit 1
fi

python3 - \
  "$tmp_dir/cargo-incremental.log" \
  "$tmp_dir/cargo-profile-dev-debug.log" \
  "$tmp_dir/cargo-profile-test-debug.log" \
  "$tmp_dir/cargo-rustc-wrapper.log" <<'PY'
from pathlib import Path
import sys

for path_arg, variable in zip(
    sys.argv[1:],
    (
        "CARGO_INCREMENTAL",
        "CARGO_PROFILE_DEV_DEBUG",
        "CARGO_PROFILE_TEST_DEBUG",
        "RUSTC_WRAPPER",
    ),
):
    values = [line for line in Path(path_arg).read_text(encoding="utf-8").splitlines() if line]
    expected = "<unset>" if variable == "RUSTC_WRAPPER" else "0"
    if not values or set(values) != {expected}:
        raise SystemExit(f"{variable} was not normalized for every Cargo call: {values}")
print("compile environment (current-only): all Cargo calls normalized")
PY

release_out_dir="$tmp_dir/release-metrics"
FAKE_CARGO_LOG="$tmp_dir/release-cargo.log" \
FAKE_CARGO_HOME_LOG="$tmp_dir/release-cargo-home.log" \
FAKE_CARGO_INCREMENTAL_LOG="$tmp_dir/release-cargo-incremental.log" \
FAKE_CARGO_PROFILE_DEV_DEBUG_LOG="$tmp_dir/release-cargo-profile-dev-debug.log" \
FAKE_CARGO_PROFILE_TEST_DEBUG_LOG="$tmp_dir/release-cargo-profile-test-debug.log" \
FAKE_RUSTC_WRAPPER_LOG="$tmp_dir/release-cargo-rustc-wrapper.log" \
FAKE_CARGO_TARGET_LOG="$tmp_dir/release-cargo-target.log" \
FAKE_HOST_TARGET="$host_target" \
FAKE_COMMIT_OID="$expected_commit_oid" \
FAKE_CARGO_TREE_FIXTURE=all \
CARGO_INCREMENTAL=1 \
CARGO_PROFILE_DEV_DEBUG=2 \
CARGO_PROFILE_TEST_DEBUG=2 \
RUSTC_WRAPPER=hostile-wrapper \
PATH="$fake_bin:$PATH" \
  ./scripts/ci-compile-metrics.sh \
    --package fake_library \
    --out-dir "$release_out_dir" \
    --binary fake_library

if ! grep -Eq '^build ' "$tmp_dir/release-cargo.log"; then
  echo "release compile metrics did not invoke cargo build" >&2
  exit 1
fi

python3 - \
  "$tmp_dir/release-cargo-incremental.log" \
  "$tmp_dir/release-cargo-profile-dev-debug.log" \
  "$tmp_dir/release-cargo-profile-test-debug.log" \
  "$tmp_dir/release-cargo-rustc-wrapper.log" <<'PY'
from pathlib import Path
import sys

for path_arg, variable in zip(
    sys.argv[1:],
    (
        "CARGO_INCREMENTAL",
        "CARGO_PROFILE_DEV_DEBUG",
        "CARGO_PROFILE_TEST_DEBUG",
        "RUSTC_WRAPPER",
    ),
):
    values = [line for line in Path(path_arg).read_text(encoding="utf-8").splitlines() if line]
    expected = "<unset>" if variable == "RUSTC_WRAPPER" else "0"
    if not values or set(values) != {expected}:
        raise SystemExit(f"{variable} was not normalized for every release Cargo call: {values}")
print("compile environment (release): all Cargo calls normalized")
PY

python3 - "$tmp_dir/cargo.log" <<'PY'
from pathlib import Path
import sys

commands = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
measured = [command for command in commands if command.split(maxsplit=1)[0] in {"tree", "fetch", "check", "build"}]
if not measured:
    raise SystemExit("no cargo invocations recorded")
unlocked = [command for command in measured if "--locked" not in command.split()]
if unlocked:
    raise SystemExit(f"unlocked cargo invocations: {unlocked}")
timed = [command for command in measured if command.split(maxsplit=1)[0] in {"check", "build"}]
offline_missing = [command for command in timed if "--offline" not in command.split()]
if offline_missing:
    raise SystemExit(f"timed cargo invocations must be offline: {offline_missing}")
tree_commands = [command for command in measured if command.split(maxsplit=1)[0] == "tree"]
if len(tree_commands) != 1:
    raise SystemExit(f"expected one dependency-tree query for a check-only run: {tree_commands}")
tree_offline_missing = [command for command in tree_commands if "--offline" not in command.split()]
if tree_offline_missing:
    raise SystemExit(f"dependency-tree queries must be offline: {tree_offline_missing}")
fetch_positions = [index for index, command in enumerate(measured) if command.split(maxsplit=1)[0] == "fetch"]
tree_positions = [index for index, command in enumerate(measured) if command.split(maxsplit=1)[0] == "tree"]
if not fetch_positions or not tree_positions or fetch_positions[0] > tree_positions[0]:
    raise SystemExit(f"host-target fetch must precede dependency-tree query: {measured}")
if any("-i" in command.split() for command in tree_commands):
    raise SystemExit(f"dependency-tree query unexpectedly used inverse traversal: {tree_commands}")
print("dependency-tree queries (current-only): 1")
PY

if ! grep -Fq 'cargo build --offline --locked' scripts/ci-compile-metrics.sh; then
  echo "release compile metrics must run cargo build offline" >&2
  exit 1
fi

baseline_out_dir="$tmp_dir/metrics-with-baseline"
baseline_cargo_home="$tmp_dir/shared-cargo-home"
FAKE_CARGO_LOG="$tmp_dir/baseline-cargo.log" \
FAKE_CARGO_HOME_LOG="$tmp_dir/baseline-cargo-home.log" \
FAKE_CARGO_INCREMENTAL_LOG="$tmp_dir/baseline-cargo-incremental.log" \
FAKE_CARGO_PROFILE_DEV_DEBUG_LOG="$tmp_dir/baseline-cargo-profile-dev-debug.log" \
FAKE_CARGO_PROFILE_TEST_DEBUG_LOG="$tmp_dir/baseline-cargo-profile-test-debug.log" \
FAKE_RUSTC_WRAPPER_LOG="$tmp_dir/baseline-cargo-rustc-wrapper.log" \
FAKE_CARGO_TARGET_LOG="$tmp_dir/baseline-cargo-target.log" \
FAKE_HOST_TARGET="$host_target" \
FAKE_COMMIT_OID="$expected_commit_oid" \
FAKE_CARGO_TREE_FIXTURE=none \
CARGO_INCREMENTAL=1 \
CARGO_PROFILE_DEV_DEBUG=2 \
CARGO_PROFILE_TEST_DEBUG=2 \
RUSTC_WRAPPER=hostile-wrapper \
CARGO_HOME="$baseline_cargo_home" PATH="$fake_bin:$PATH" \
  ./scripts/ci-compile-metrics.sh \
    --package fake_library \
    --out-dir "$baseline_out_dir" \
    --check-only \
    --baseline-ref HEAD

python3 - \
  "$tmp_dir/baseline-cargo-incremental.log" \
  "$tmp_dir/baseline-cargo-profile-dev-debug.log" \
  "$tmp_dir/baseline-cargo-profile-test-debug.log" \
  "$tmp_dir/baseline-cargo-rustc-wrapper.log" <<'PY'
from pathlib import Path
import sys

for path_arg, variable in zip(
    sys.argv[1:],
    (
        "CARGO_INCREMENTAL",
        "CARGO_PROFILE_DEV_DEBUG",
        "CARGO_PROFILE_TEST_DEBUG",
        "RUSTC_WRAPPER",
    ),
):
    values = [line for line in Path(path_arg).read_text(encoding="utf-8").splitlines() if line]
    expected = "<unset>" if variable == "RUSTC_WRAPPER" else "0"
    if not values or set(values) != {expected}:
        raise SystemExit(f"{variable} was not normalized for every current/baseline Cargo call: {values}")
print("compile environment (current+baseline): all Cargo calls normalized")
PY

python3 - "$baseline_out_dir/baseline.metrics.json" "$baseline_out_dir/comparison.json" <<'PY'
import json
from pathlib import Path
import sys

metrics = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
assert metrics["wasmtime_present"] is False
assert metrics["wasm_executor_present"] is False

comparison = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
expected_identity = {
    "package": "fake_library",
    "binary": None,
    "check_only": True,
    "no_default_features": False,
}
assert comparison["measurement_identity"] == expected_identity
assert {
    field: comparison["current"][field] for field in expected_identity
} == expected_identity
assert {
    field: comparison["baseline"][field] for field in expected_identity
} == expected_identity
print("measurement identity matching contract: OK")
PY

python3 - "$tmp_dir/baseline-cargo-home.log" "$baseline_cargo_home" "$tmp_dir/baseline-cargo-target.log" <<'PY'
from pathlib import Path
import sys

homes = [line for line in Path(sys.argv[1]).read_text(encoding="utf-8").splitlines() if line]
expected = str(Path(sys.argv[2]).expanduser().resolve())
if not homes:
    raise SystemExit("baseline run did not record any Cargo home paths")
if set(homes) != {expected}:
    raise SystemExit(f"compile metrics did not reuse caller Cargo home: {homes}")

targets = [line for line in Path(sys.argv[3]).read_text(encoding="utf-8").splitlines() if line]
if len(set(targets)) != 2:
    raise SystemExit(f"current and baseline target directories were not isolated: {targets}")
PY

python3 - "$tmp_dir/baseline-cargo.log" <<'PY'
from pathlib import Path
import sys

commands = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
tree_positions = [index for index, command in enumerate(commands) if command.split(maxsplit=1)[0] == "tree"]
tree_commands = [commands[index] for index in tree_positions]
if len(tree_commands) != 2:
    raise SystemExit(
        "expected one dependency-tree query per checkout in a baseline run: "
        f"{tree_commands}"
    )
if any("-i" in command.split() for command in tree_commands):
    raise SystemExit(f"dependency-tree queries unexpectedly used inverse traversal: {tree_commands}")
if any("--offline" not in command.split() for command in tree_commands):
    raise SystemExit(f"dependency-tree queries must be offline: {tree_commands}")
fetch_positions = [index for index, command in enumerate(commands) if command.split(maxsplit=1)[0] == "fetch"]
if len(fetch_positions) != len(tree_commands):
    raise SystemExit(
        "expected one host-target fetch per checkout in a baseline run: "
        f"fetches={fetch_positions}, trees={tree_commands}"
    )
for checkout_index, tree_position in enumerate(tree_positions):
    segment_start = 0 if checkout_index == 0 else tree_positions[checkout_index - 1] + 1
    segment = commands[segment_start : tree_position + 1]
    segment_fetches = [
        index
        for index, command in enumerate(segment, start=segment_start)
        if command.split(maxsplit=1)[0] == "fetch"
    ]
    if len(segment_fetches) != 1 or segment_fetches[0] >= tree_position:
        raise SystemExit(
            "each checkout segment must fetch before its dependency-tree query: "
            f"segment={segment}"
        )
print("dependency-tree queries (current+baseline): 2")
PY

python3 - "$tmp_dir/comparison.json" <<'PY'
import json
from copy import deepcopy
from pathlib import Path
import subprocess
import sys

comparison_path = Path(sys.argv[1])
matching_comparison = {
    "measurement_identity": {
        "package": "fake_library",
        "binary": None,
        "check_only": True,
        "no_default_features": False,
    },
    "current": {
        "package": "fake_library",
        "binary": None,
        "check_only": True,
        "no_default_features": False,
        "wasmtime_present": False,
    },
    "baseline": {
        "package": "fake_library",
        "binary": None,
        "check_only": True,
        "no_default_features": False,
        "package_count": 10,
    },
    "metric_rows": [
        {
            "metric": "package_count",
            "baseline": 10,
            "current": 20,
            "delta": 10,
            "percent": 100.0,
        }
    ],
}


def run_gate(payload):
    comparison_path.write_text(json.dumps(payload), encoding="utf-8")
    return subprocess.run(
        [
            "python3",
            "scripts/ci-compile-metrics-gate.py",
            "--comparison",
            str(comparison_path),
            "--max-package-count-regression-pct",
            "100",
        ],
        capture_output=True,
        text=True,
        check=False,
    )


comparison_path.write_text(json.dumps(matching_comparison), encoding="utf-8")

for invalid in ("NaN", "inf", "-1"):
    result = subprocess.run(
        [
            "python3",
            "scripts/ci-compile-metrics-gate.py",
            "--comparison",
            str(comparison_path),
            "--max-package-count-regression-pct",
            invalid,
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode == 0:
        raise SystemExit(f"invalid threshold accepted: {invalid}")
    if "finite and non-negative" not in result.stderr:
        raise SystemExit(
            f"invalid threshold error omitted contract wording for {invalid}: "
            f"{result.stderr!r}"
        )

passing = run_gate(matching_comparison)
if passing.returncode != 0:
    raise SystemExit(f"valid threshold unexpectedly failed: {passing.stdout}{passing.stderr}")

missing_identity = dict(matching_comparison)
missing_identity.pop("measurement_identity")
missing = run_gate(missing_identity)
if (
    missing.returncode == 0
    or "comparison is missing measurement_identity" not in missing.stdout
):
    raise SystemExit(
        "missing measurement identity unexpectedly passed or used wrong error: "
        f"{missing.stdout}{missing.stderr}"
    )

mismatched_identity = json.loads(json.dumps(matching_comparison))
mismatched_identity["baseline"]["no_default_features"] = True
mismatched = run_gate(mismatched_identity)
if (
    mismatched.returncode == 0
    or "current/baseline measurement identity mismatch" not in mismatched.stdout
):
    raise SystemExit(
        "mismatched measurement identity unexpectedly passed or used wrong error: "
        f"{mismatched.stdout}{mismatched.stderr}"
    )

invalid_identity_cases = (
    (
        "boolean-string",
        {"check_only": "true"},
        "check_only must be a boolean",
    ),
    (
        "empty-package",
        {"package": ""},
        "package must be a non-empty string",
    ),
    (
        "integer-binary",
        {"binary": 7},
        "binary must be null or a non-empty string",
    ),
    (
        "check-only-with-binary",
        {"check_only": True, "binary": "fake_binary"},
        "check_only requires binary to be null",
    ),
    (
        "release-without-binary",
        {"check_only": False, "binary": None},
        "release-build requires binary to be a non-empty string",
    ),
)
for case_name, updates, expected_error in invalid_identity_cases:
    invalid = deepcopy(matching_comparison)
    for section in ("measurement_identity", "current", "baseline"):
        invalid[section].update(updates)
    result = run_gate(invalid)
    if result.returncode == 0 or expected_error not in result.stdout:
        raise SystemExit(
            f"invalid {case_name} measurement identity unexpectedly passed or "
            f"used wrong error: {result.stdout}{result.stderr}"
        )

print("ci-compile-metrics-gate threshold contract: OK")
print("measurement identity missing/mismatch contract: OK")
print("measurement identity semantic validation contract: OK")
PY

echo "ci-compile-metrics-contract.test: OK"
