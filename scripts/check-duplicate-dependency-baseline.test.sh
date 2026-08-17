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
  "duplicate_dependency_cluster_count": 2,
  "duplicate_dependency_unique_crates": 2,
  "duplicate_dependency_entry_total": 10,
  "duplicate_dependency_tree_output_lines": 1903,
  "duplicate_dependency_crates": [
    {"crate": "hashbrown", "duplicate_entries": 4},
    {"crate": "windows-sys", "duplicate_entries": 6}
  ],
  "duplicate_dependency_top_crates": [
    {"crate": "windows-sys", "duplicate_entries": 6},
    {"crate": "hashbrown", "duplicate_entries": 4}
  ]
}
JSON

cat >"$baseline" <<'JSON'
{
  "schema_version": 1,
  "owner": "repository_health_engineer",
  "reviewed_at": "2026-06-25",
  "expires": "2026-09-30",
  "rationale": "test fixture",
  "update_policy": "test fixture",
  "maxima": {
    "duplicate_dependency_cluster_count": 2,
    "duplicate_dependency_unique_crates": 2,
    "duplicate_dependency_entry_total": 10
  },
  "crate_maxima": {
    "hashbrown": 4,
    "windows-sys": 6
  }
}
JSON

valid_out="$tmp_dir/valid.out"
OASIS7_DUPLICATE_DEP_BASELINE="$baseline" OASIS7_DUPLICATE_DEP_TODAY=2026-06-25 \
  ./scripts/check-duplicate-dependency-baseline.sh "$summary" >"$valid_out"
grep -q "ok: duplicate dependency baseline within budget" "$valid_out"

python3 - "$summary" "$baseline" "$tmp_dir/zero-summary.json" "$tmp_dir/zero-baseline.json" <<'PY'
from pathlib import Path
import json
import sys

summary = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
baseline = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
summary["duplicate_dependency_cluster_count"] = 0
summary["duplicate_dependency_unique_crates"] = 0
summary["duplicate_dependency_entry_total"] = 0
summary["duplicate_dependency_crates"] = []
summary["duplicate_dependency_top_crates"] = []
baseline["maxima"] = {
    "duplicate_dependency_cluster_count": 0,
    "duplicate_dependency_unique_crates": 0,
    "duplicate_dependency_entry_total": 0,
}
baseline["crate_maxima"] = {}
Path(sys.argv[3]).write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
Path(sys.argv[4]).write_text(json.dumps(baseline, indent=2) + "\n", encoding="utf-8")
PY
zero_out="$tmp_dir/zero.out"
OASIS7_DUPLICATE_DEP_BASELINE="$tmp_dir/zero-baseline.json" OASIS7_DUPLICATE_DEP_TODAY=2026-06-25 \
  ./scripts/check-duplicate-dependency-baseline.sh "$tmp_dir/zero-summary.json" >"$zero_out"
grep -q "ok: duplicate dependency baseline within budget" "$zero_out"

python3 - "$summary" "$tmp_dir/growth.json" <<'PY'
from pathlib import Path
import json
import sys

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
payload["duplicate_dependency_entry_total"] = 11
Path(sys.argv[2]).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY
if OASIS7_DUPLICATE_DEP_BASELINE="$baseline" OASIS7_DUPLICATE_DEP_TODAY=2026-06-25 \
  ./scripts/check-duplicate-dependency-baseline.sh "$tmp_dir/growth.json" >"$tmp_dir/growth.out" 2>&1
then
  echo "expected duplicate entry growth to fail" >&2
  exit 1
fi
grep -q "duplicate_dependency_entry_total grew beyond baseline" "$tmp_dir/growth.out"

python3 - "$summary" "$tmp_dir/negative-summary.json" <<'PY'
from pathlib import Path
import json
import sys

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
payload["duplicate_dependency_entry_total"] = -1
Path(sys.argv[2]).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY
if OASIS7_DUPLICATE_DEP_BASELINE="$baseline" OASIS7_DUPLICATE_DEP_TODAY=2026-06-25 \
  ./scripts/check-duplicate-dependency-baseline.sh "$tmp_dir/negative-summary.json" >"$tmp_dir/negative-summary.out" 2>&1
then
  echo "expected negative aggregate duplicate count to fail" >&2
  exit 1
fi
grep -q "summary duplicate_dependency_entry_total must be non-negative" "$tmp_dir/negative-summary.out"

python3 - "$summary" "$tmp_dir/negative-crate.json" <<'PY'
from pathlib import Path
import json
import sys

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
for key in ("duplicate_dependency_crates", "duplicate_dependency_top_crates"):
    for entry in payload[key]:
        if entry["crate"] == "hashbrown":
            entry["duplicate_entries"] = -1
Path(sys.argv[2]).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY
if OASIS7_DUPLICATE_DEP_BASELINE="$baseline" OASIS7_DUPLICATE_DEP_TODAY=2026-06-25 \
  ./scripts/check-duplicate-dependency-baseline.sh "$tmp_dir/negative-crate.json" >"$tmp_dir/negative-crate.out" 2>&1
then
  echo "expected negative crate duplicate count to fail" >&2
  exit 1
fi
grep -q "summary duplicate_dependency_crates duplicate_entries for hashbrown must be non-negative" "$tmp_dir/negative-crate.out"

python3 - "$baseline" "$tmp_dir/negative-maxima-baseline.json" <<'PY'
from pathlib import Path
import json
import sys

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
payload["maxima"]["duplicate_dependency_entry_total"] = -1
Path(sys.argv[2]).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY
if OASIS7_DUPLICATE_DEP_BASELINE="$tmp_dir/negative-maxima-baseline.json" OASIS7_DUPLICATE_DEP_TODAY=2026-06-25 \
  ./scripts/check-duplicate-dependency-baseline.sh "$summary" >"$tmp_dir/negative-maxima.out" 2>&1
then
  echo "expected negative duplicate baseline maximum to fail" >&2
  exit 1
fi
grep -q "baseline maxima duplicate_dependency_entry_total must be non-negative" "$tmp_dir/negative-maxima.out"

python3 - "$summary" "$tmp_dir/top-growth.json" <<'PY'
from pathlib import Path
import json
import sys

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
for key in ("duplicate_dependency_crates", "duplicate_dependency_top_crates"):
    payload[key][0]["duplicate_entries"] = 7
Path(sys.argv[2]).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY
if OASIS7_DUPLICATE_DEP_BASELINE="$baseline" OASIS7_DUPLICATE_DEP_TODAY=2026-06-25 \
  ./scripts/check-duplicate-dependency-baseline.sh "$tmp_dir/top-growth.json" >"$tmp_dir/top-growth.out" 2>&1
then
  echo "expected top crate growth to fail" >&2
  exit 1
fi
grep -q 'duplicate crate `hashbrown` grew beyond baseline' "$tmp_dir/top-growth.out"

python3 - "$summary" "$tmp_dir/top-summary-mismatch.json" <<'PY'
from pathlib import Path
import json
import sys

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
payload["duplicate_dependency_top_crates"] = [
    {"crate": "hashbrown", "duplicate_entries": 4}
]
Path(sys.argv[2]).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY
if OASIS7_DUPLICATE_DEP_BASELINE="$baseline" OASIS7_DUPLICATE_DEP_TODAY=2026-06-25 \
  ./scripts/check-duplicate-dependency-baseline.sh "$tmp_dir/top-summary-mismatch.json" >"$tmp_dir/top-summary-mismatch.out" 2>&1
then
  echo "expected top summary mismatch to fail" >&2
  exit 1
fi
grep -q "duplicate_dependency_top_crates must match the top 20 entries" "$tmp_dir/top-summary-mismatch.out"

python3 - "$summary" "$tmp_dir/new-crate.json" <<'PY'
from pathlib import Path
import json
import sys

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
payload["duplicate_dependency_crates"] = [
    {"crate": "hashbrown", "duplicate_entries": 4},
    {"crate": "new_duplicate_surface", "duplicate_entries": 6},
]
payload["duplicate_dependency_top_crates"] = [
    {"crate": "new_duplicate_surface", "duplicate_entries": 6},
    {"crate": "hashbrown", "duplicate_entries": 4},
]
Path(sys.argv[2]).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY
if OASIS7_DUPLICATE_DEP_BASELINE="$baseline" OASIS7_DUPLICATE_DEP_TODAY=2026-06-25 \
  ./scripts/check-duplicate-dependency-baseline.sh "$tmp_dir/new-crate.json" >"$tmp_dir/new-crate.out" 2>&1
then
  echo "expected new duplicate crate identity to fail" >&2
  exit 1
fi
grep -q 'new duplicate crate `new_duplicate_surface` is not approved in baseline' "$tmp_dir/new-crate.out"

python3 - "$baseline" "$tmp_dir/stale-baseline.json" <<'PY'
from pathlib import Path
import json
import sys

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
payload["crate_maxima"]["stale_duplicate_surface"] = 2
Path(sys.argv[2]).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY
if OASIS7_DUPLICATE_DEP_BASELINE="$tmp_dir/stale-baseline.json" OASIS7_DUPLICATE_DEP_TODAY=2026-06-25 \
  ./scripts/check-duplicate-dependency-baseline.sh "$summary" >"$tmp_dir/stale.out" 2>&1
then
  echo "expected stale duplicate crate baseline entry to fail" >&2
  exit 1
fi
grep -q 'duplicate crate baseline entry `stale_duplicate_surface` is stale' "$tmp_dir/stale.out"

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
payload["duplicate_dependency_crates"] = []
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
