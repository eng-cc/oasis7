#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT

fake_bin="$tmp_dir/bin"
mkdir -p "$fake_bin"
cat >"$fake_bin/cargo" <<'FAKE_CARGO'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${FAKE_CARGO_LOG:?}"
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

out_dir="$tmp_dir/metrics"
FAKE_CARGO_LOG="$tmp_dir/cargo.log" PATH="$fake_bin:$PATH" \
  ./scripts/ci-compile-metrics.sh \
    --package fake_library \
    --out-dir "$out_dir" \
    --check-only \
    --no-default-features

python3 - "$out_dir/current.metrics.json" "$out_dir/comparison.json" "$out_dir/summary.md" <<'PY'
import json
from pathlib import Path
import sys

metrics = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
comparison = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
summary = Path(sys.argv[3]).read_text(encoding="utf-8")
assert metrics["package_count"] == 2
assert metrics["binary"] is None
assert metrics["check_only"] is True
assert metrics["no_default_features"] is True
assert metrics["cargo_build_release_seconds"] is None
assert metrics["release_binary_bytes"] is None
assert comparison["metric_rows"] == []
assert "not measured (check-only package)" in summary
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

echo "ci-compile-metrics-contract.test: OK"
