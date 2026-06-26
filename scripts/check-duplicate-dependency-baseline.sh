#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

summary_path="${1:-output/rust-governance/summary.json}"
baseline_path="${OASIS7_DUPLICATE_DEP_BASELINE:-scripts/rust-duplicate-dependency-baseline.json}"

python3 - "$summary_path" "$baseline_path" <<'PY'
from __future__ import annotations

from datetime import date
from pathlib import Path
import json
import os
import re
import sys

summary_path = Path(sys.argv[1])
baseline_path = Path(sys.argv[2])
today = os.environ.get("OASIS7_DUPLICATE_DEP_TODAY") or date.today().isoformat()

required_summary_keys = (
    "cargo_deny_rc",
    "duplicate_dependency_cluster_count",
    "duplicate_dependency_unique_crates",
    "duplicate_dependency_entry_total",
    "duplicate_dependency_tree_output_lines",
    "duplicate_dependency_crates",
    "duplicate_dependency_top_crates",
)
required_baseline_keys = (
    "schema_version",
    "owner",
    "reviewed_at",
    "expires",
    "rationale",
    "update_policy",
    "maxima",
    "crate_maxima",
)
budget_keys = (
    "duplicate_dependency_cluster_count",
    "duplicate_dependency_unique_crates",
    "duplicate_dependency_entry_total",
)

failures: list[str] = []

def load_json(path: Path, label: str) -> dict:
    if not path.exists():
        failures.append(f"{label} not found: {path}")
        return {}
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as err:
        failures.append(f"{label} is not valid JSON: {path}: {err}")
        return {}
    if not isinstance(payload, dict):
        failures.append(f"{label} must be a JSON object: {path}")
        return {}
    return payload

summary = load_json(summary_path, "duplicate dependency summary")
baseline = load_json(baseline_path, "duplicate dependency baseline")

for key in required_summary_keys:
    if key not in summary:
        failures.append(f"summary missing key: {key}")

try:
    cargo_deny_rc = int(summary.get("cargo_deny_rc", -1))
except (TypeError, ValueError):
    cargo_deny_rc = -1
if cargo_deny_rc != 0:
    failures.append(
        "duplicate dependency baseline requires cargo-deny duplicate data "
        f"(cargo_deny_rc={summary.get('cargo_deny_rc')})"
    )

for key in required_baseline_keys:
    if key not in baseline or baseline[key] is None:
        failures.append(f"baseline missing key: {key}")

if baseline.get("schema_version") != 1:
    failures.append("baseline schema_version must be 1")

for key in ("reviewed_at", "expires"):
    value = str(baseline.get(key, ""))
    if value and not re.match(r"^\d{4}-\d{2}-\d{2}$", value):
        failures.append(f"baseline {key} must be YYYY-MM-DD")

expires = str(baseline.get("expires", ""))
if expires and re.match(r"^\d{4}-\d{2}-\d{2}$", expires) and expires < today:
    failures.append(f"duplicate dependency baseline expired on {expires}")

maxima = baseline.get("maxima", {})
if not isinstance(maxima, dict):
    failures.append("baseline maxima must be an object")
    maxima = {}

for key in budget_keys:
    if key not in maxima:
        failures.append(f"baseline maxima missing key: {key}")
        continue
    try:
        actual = int(summary.get(key, -1))
        maximum = int(maxima[key])
    except (TypeError, ValueError):
        failures.append(f"summary/baseline value must be integer for {key}")
        continue
    if actual > maximum:
        failures.append(f"{key} grew beyond baseline: actual={actual} maximum={maximum}")

crate_maxima = baseline.get("crate_maxima", {})
if not isinstance(crate_maxima, dict):
    failures.append("baseline crate_maxima must be an object")
    crate_maxima = {}

duplicate_crates = summary.get("duplicate_dependency_crates", [])
if not isinstance(duplicate_crates, list):
    failures.append("summary duplicate_dependency_crates must be a list")
    duplicate_crates = []

seen_crates: set[str] = set()
for entry in duplicate_crates:
    if not isinstance(entry, dict):
        failures.append("summary duplicate_dependency_crates entries must be objects")
        continue
    crate_name = entry.get("crate")
    if not crate_name:
        failures.append("summary duplicate crate entry missing crate")
        continue
    if crate_name in seen_crates:
        failures.append(f"summary duplicate_dependency_crates repeats crate `{crate_name}`")
        continue
    seen_crates.add(crate_name)
    if crate_name not in crate_maxima:
        failures.append(
            f"new duplicate crate `{crate_name}` is not approved in baseline; "
            "update crate_maxima only with repository-health review evidence"
        )
        continue
    try:
        actual = int(entry.get("duplicate_entries", -1))
        maximum = int(crate_maxima[crate_name])
    except (TypeError, ValueError):
        failures.append(f"duplicate crate value must be integer for {crate_name}")
        continue
    if actual > maximum:
        failures.append(
            f"duplicate crate `{crate_name}` grew beyond baseline: "
            f"actual={actual} maximum={maximum}"
        )

for crate_name in sorted(set(crate_maxima) - seen_crates):
    failures.append(
        f"duplicate crate baseline entry `{crate_name}` is stale; "
        "remove or lower it in the same reviewed dependency-governance patch"
    )

if failures:
    for failure in failures:
        print(f"error: {failure}", file=sys.stderr)
    raise SystemExit(1)

print(
    "ok: duplicate dependency baseline within budget "
    f"(clusters={summary['duplicate_dependency_cluster_count']}/"
    f"{maxima['duplicate_dependency_cluster_count']}, "
    f"unique={summary['duplicate_dependency_unique_crates']}/"
    f"{maxima['duplicate_dependency_unique_crates']}, "
    f"entries={summary['duplicate_dependency_entry_total']}/"
    f"{maxima['duplicate_dependency_entry_total']}, "
    f"tree_lines_observed={summary['duplicate_dependency_tree_output_lines']})"
)
PY
