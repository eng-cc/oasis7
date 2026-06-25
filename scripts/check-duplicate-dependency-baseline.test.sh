#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT

summary="$tmp_dir/summary.json"
baseline="$tmp_dir/baseline.json"

cat >"$summary" <<'JSON'
{
  "cargo_deny_rc": 0,
  "duplicate_dependency_cluster_count": 88,
  "duplicate_dependency_unique_crates": 88,
  "duplicate_dependency_entry_total": 213,
  "duplicate_dependency_tree_output_lines": 1903,
  "duplicate_dependency_top_crates": [
    {"crate": "windows-sys", "duplicate_entries": 6},
    {"crate": "hashbrown", "duplicate_entries": 4}
  ]
}
JSON

cp scripts/rust-duplicate-dependency-baseline.json "$baseline"

valid_out="$tmp_dir/valid.out"
OASIS7_DUPLICATE_DEP_BASELINE="$baseline" OASIS7_DUPLICATE_DEP_TODAY=2026-06-25 \
  ./scripts/check-duplicate-dependency-baseline.sh "$summary" >"$valid_out"
grep -q "ok: duplicate dependency baseline within budget" "$valid_out"

python3 - "$summary" "$tmp_dir/growth.json" <<'PY'
from pathlib import Path
import json
import sys

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
payload["duplicate_dependency_entry_total"] = 214
Path(sys.argv[2]).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY
if OASIS7_DUPLICATE_DEP_BASELINE="$baseline" OASIS7_DUPLICATE_DEP_TODAY=2026-06-25 \
  ./scripts/check-duplicate-dependency-baseline.sh "$tmp_dir/growth.json" >"$tmp_dir/growth.out" 2>&1
then
  echo "expected duplicate entry growth to fail" >&2
  exit 1
fi
grep -q "duplicate_dependency_entry_total grew beyond baseline" "$tmp_dir/growth.out"

python3 - "$summary" "$tmp_dir/top-growth.json" <<'PY'
from pathlib import Path
import json
import sys

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
payload["duplicate_dependency_top_crates"][0]["duplicate_entries"] = 7
Path(sys.argv[2]).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY
if OASIS7_DUPLICATE_DEP_BASELINE="$baseline" OASIS7_DUPLICATE_DEP_TODAY=2026-06-25 \
  ./scripts/check-duplicate-dependency-baseline.sh "$tmp_dir/top-growth.json" >"$tmp_dir/top-growth.out" 2>&1
then
  echo "expected top crate growth to fail" >&2
  exit 1
fi
grep -q 'top duplicate crate `windows-sys` grew beyond baseline' "$tmp_dir/top-growth.out"

python3 - "$baseline" "$tmp_dir/expired-baseline.json" <<'PY'
from pathlib import Path
import json
import sys

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
payload["expires"] = "2026-01-01"
Path(sys.argv[2]).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY
if OASIS7_DUPLICATE_DEP_BASELINE="$tmp_dir/expired-baseline.json" OASIS7_DUPLICATE_DEP_TODAY=2026-06-25 \
  ./scripts/check-duplicate-dependency-baseline.sh "$summary" >"$tmp_dir/expired.out" 2>&1
then
  echo "expected expired baseline to fail" >&2
  exit 1
fi
grep -q "duplicate dependency baseline expired" "$tmp_dir/expired.out"

python3 - "$summary" "$tmp_dir/no-cargo-deny.json" <<'PY'
from pathlib import Path
import json
import sys

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
payload["cargo_deny_rc"] = 127
payload["duplicate_dependency_cluster_count"] = 0
payload["duplicate_dependency_unique_crates"] = 0
payload["duplicate_dependency_entry_total"] = 0
payload["duplicate_dependency_top_crates"] = []
Path(sys.argv[2]).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY
if OASIS7_DUPLICATE_DEP_BASELINE="$baseline" OASIS7_DUPLICATE_DEP_TODAY=2026-06-25 \
  ./scripts/check-duplicate-dependency-baseline.sh "$tmp_dir/no-cargo-deny.json" >"$tmp_dir/no-cargo-deny.out" 2>&1
then
  echo "expected missing cargo-deny duplicate data to fail" >&2
  exit 1
fi
grep -q "requires cargo-deny duplicate data" "$tmp_dir/no-cargo-deny.out"

echo "check-duplicate-dependency-baseline.test: OK"
