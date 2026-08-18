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
if grep -Eq 'COMPILE_METRICS_MAX_[A-Z0-9_]+' .github/workflows/compile-metrics.yml; then
  echo "compile metrics workflow must not define unused threshold environment defaults" >&2
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
cat >"$fake_bin/cargo" <<'FAKE_CARGO'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${FAKE_CARGO_LOG:?}"
printf '%s\n' "${CARGO_HOME:-<unset>}" >>"${FAKE_CARGO_HOME_LOG:?}"
if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
  printf '%s\n' "$CARGO_TARGET_DIR" >>"${FAKE_CARGO_TARGET_LOG:?}"
fi
case "${1:-}" in
  tree)
    if [[ "$*" == *"-i wasmtime"* || "$*" == *"-i oasis7_wasm_executor"* ]]; then
      exit 1
    fi
    case "${FAKE_CARGO_TREE_FIXTURE:-none}" in
      all)
        printf 'fake-package\nfake-dependency\nwasmtime v99.0.0\noasis7_wasm_executor v0.1.0\n'
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
  fetch|check)
    ;;
  build)
    echo "unexpected release build" >&2
    exit 1
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
FAKE_CARGO_TARGET_LOG="$tmp_dir/cargo-target.log" \
FAKE_COMMIT_OID="$expected_commit_oid" \
FAKE_CARGO_TREE_FIXTURE=all \
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
tree_commands = [command for command in measured if command.split(maxsplit=1)[0] == "tree"]
if len(tree_commands) != 1:
    raise SystemExit(f"expected one dependency-tree query for a check-only run: {tree_commands}")
if any("-i" in command.split() for command in tree_commands):
    raise SystemExit(f"dependency-tree query unexpectedly used inverse traversal: {tree_commands}")
print("dependency-tree queries (current-only): 1")
PY

baseline_out_dir="$tmp_dir/metrics-with-baseline"
baseline_cargo_home="$tmp_dir/shared-cargo-home"
FAKE_CARGO_LOG="$tmp_dir/baseline-cargo.log" \
FAKE_CARGO_HOME_LOG="$tmp_dir/baseline-cargo-home.log" \
FAKE_CARGO_TARGET_LOG="$tmp_dir/baseline-cargo-target.log" \
FAKE_COMMIT_OID="$expected_commit_oid" \
FAKE_CARGO_TREE_FIXTURE=none \
CARGO_HOME="$baseline_cargo_home" PATH="$fake_bin:$PATH" \
  ./scripts/ci-compile-metrics.sh \
    --package fake_library \
    --out-dir "$baseline_out_dir" \
    --check-only \
    --baseline-ref HEAD

python3 - "$baseline_out_dir/baseline.metrics.json" <<'PY'
import json
from pathlib import Path
import sys

metrics = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
assert metrics["wasmtime_present"] is False
assert metrics["wasm_executor_present"] is False
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
tree_commands = [command for command in commands if command.split(maxsplit=1)[0] == "tree"]
if len(tree_commands) != 2:
    raise SystemExit(
        "expected one dependency-tree query per checkout in a baseline run: "
        f"{tree_commands}"
    )
if any("-i" in command.split() for command in tree_commands):
    raise SystemExit(f"dependency-tree queries unexpectedly used inverse traversal: {tree_commands}")
print("dependency-tree queries (current+baseline): 2")
PY

python3 - "$tmp_dir/comparison.json" <<'PY'
import json
from pathlib import Path
import subprocess
import sys

comparison_path = Path(sys.argv[1])
comparison_path.write_text(
    json.dumps(
        {
            "current": {"wasmtime_present": False},
            "baseline": {"package_count": 10},
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
    ),
    encoding="utf-8",
)

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

passing = subprocess.run(
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
if passing.returncode != 0:
    raise SystemExit(f"valid threshold unexpectedly failed: {passing.stdout}{passing.stderr}")

print("ci-compile-metrics-gate threshold contract: OK")
PY

echo "ci-compile-metrics-contract.test: OK"
