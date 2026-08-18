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

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT

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
    printf 'fake-package\nfake-dependency\n'
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
assert metrics["package_count"] == 2
assert metrics["commit_oid"] == expected_commit_oid
assert metrics["binary"] is None
assert metrics["check_only"] is True
assert metrics["no_default_features"] is True
assert metrics["cargo_build_release_seconds"] is None
assert metrics["release_binary_bytes"] is None
assert comparison["metric_rows"] == []
assert comparison["current_commit_oid"] == expected_commit_oid
assert comparison["baseline_commit_oid"] is None
assert "not measured (check-only package)" in summary
assert expected_commit_oid in summary
PY

if rg -q 'build' "$tmp_dir/cargo.log"; then
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
PY

baseline_out_dir="$tmp_dir/metrics-with-baseline"
baseline_cargo_home="$tmp_dir/shared-cargo-home"
FAKE_CARGO_LOG="$tmp_dir/baseline-cargo.log" \
FAKE_CARGO_HOME_LOG="$tmp_dir/baseline-cargo-home.log" \
FAKE_CARGO_TARGET_LOG="$tmp_dir/baseline-cargo-target.log" \
FAKE_COMMIT_OID="$expected_commit_oid" \
CARGO_HOME="$baseline_cargo_home" PATH="$fake_bin:$PATH" \
  ./scripts/ci-compile-metrics.sh \
    --package fake_library \
    --out-dir "$baseline_out_dir" \
    --check-only \
    --baseline-ref HEAD

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
