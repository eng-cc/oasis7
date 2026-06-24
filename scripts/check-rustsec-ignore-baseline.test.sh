#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT

run_check() {
  OASIS7_RUSTSEC_DENY_TOML="$1" OASIS7_RUSTSEC_TODAY="2026-06-24" \
    ./scripts/check-rustsec-ignore-baseline.sh
}

valid_out="$tmp_dir/valid.out"
run_check deny.toml >"$valid_out"
if ! grep -q "ok: RustSec ignore baseline" "$valid_out"; then
  echo "expected valid baseline to pass" >&2
  cat "$valid_out" >&2
  exit 1
fi

missing_metadata="$tmp_dir/missing-metadata.toml"
python3 - deny.toml "$missing_metadata" <<'PY'
from pathlib import Path
import sys

source = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
out = []
skip_previous = False
for line in source:
    if '"RUSTSEC-2025-0009"' in line and out and "rustsec-ignore:" in out[-1]:
        out.pop()
    out.append(line)
Path(sys.argv[2]).write_text("\n".join(out) + "\n", encoding="utf-8")
PY
if run_check "$missing_metadata" >"$tmp_dir/missing.out" 2>&1; then
  echo "expected missing metadata case to fail" >&2
  exit 1
fi
grep -q "missing rustsec-ignore metadata" "$tmp_dir/missing.out"

extra_id="$tmp_dir/extra-id.toml"
python3 - deny.toml "$extra_id" <<'PY'
from pathlib import Path
import sys

source = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
out = []
for line in source:
    out.append(line)
    if '"RUSTSEC-2026-0119"' in line:
        out.append("  # rustsec-ignore: owner=repository_health_engineer; scope=test; reason=test; expiry=2026-08-31; validation=test")
        out.append('  "RUSTSEC-2099-0001", # test-only unapproved advisory')
Path(sys.argv[2]).write_text("\n".join(out) + "\n", encoding="utf-8")
PY
if run_check "$extra_id" >"$tmp_dir/extra.out" 2>&1; then
  echo "expected unapproved advisory case to fail" >&2
  exit 1
fi
grep -q "unapproved RustSec ignore id" "$tmp_dir/extra.out"

expired="$tmp_dir/expired.toml"
python3 - deny.toml "$expired" <<'PY'
from pathlib import Path
import sys

text = Path(sys.argv[1]).read_text(encoding="utf-8")
Path(sys.argv[2]).write_text(text.replace("expiry=2026-08-31", "expiry=2026-01-01"), encoding="utf-8")
PY
if run_check "$expired" >"$tmp_dir/expired.out" 2>&1; then
  echo "expected expired metadata case to fail" >&2
  exit 1
fi
grep -q "metadata expired" "$tmp_dir/expired.out"

echo "check-rustsec-ignore-baseline.test: OK"
