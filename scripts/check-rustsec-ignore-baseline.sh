#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

deny_toml="${OASIS7_RUSTSEC_DENY_TOML:-deny.toml}"

python3 - "$deny_toml" <<'PY'
from __future__ import annotations

from datetime import date
from pathlib import Path
import os
import re
import sys

deny_path = Path(sys.argv[1])
today = os.environ.get("OASIS7_RUSTSEC_TODAY") or date.today().isoformat()

approved = [
    "RUSTSEC-2021-0127",
    "RUSTSEC-2024-0384",
    "RUSTSEC-2024-0436",
    "RUSTSEC-2025-0009",
    "RUSTSEC-2025-0010",
    "RUSTSEC-2025-0012",
    "RUSTSEC-2025-0052",
    "RUSTSEC-2026-0098",
    "RUSTSEC-2026-0099",
    "RUSTSEC-2026-0104",
    "RUSTSEC-2026-0119",
]
required_keys = ("owner", "scope", "reason", "expiry", "validation")

if not deny_path.exists():
    print(f"error: deny.toml not found: {deny_path}", file=sys.stderr)
    raise SystemExit(1)

in_ignore = False
pending_metadata: dict[str, str] | None = None
seen: dict[str, dict[str, str]] = {}
failures: list[str] = []

metadata_re = re.compile(r"^\s*#\s*rustsec-ignore:\s*(?P<body>.+)$")
id_re = re.compile(r'"(?P<id>RUSTSEC-\d{4}-\d{4})"')

def parse_metadata(body: str, line_no: int) -> dict[str, str]:
    metadata: dict[str, str] = {}
    for chunk in body.split(";"):
        chunk = chunk.strip()
        if not chunk:
            continue
        if "=" not in chunk:
            failures.append(f"line {line_no}: malformed metadata chunk `{chunk}`")
            continue
        key, value = chunk.split("=", 1)
        metadata[key.strip()] = value.strip()
    return metadata

for line_no, line in enumerate(deny_path.read_text(encoding="utf-8").splitlines(), 1):
    if not in_ignore:
        if re.match(r"\s*ignore\s*=\s*\[", line):
            in_ignore = True
        continue
    if re.match(r"\s*\]", line):
        in_ignore = False
        continue

    metadata_match = metadata_re.match(line)
    if metadata_match:
        pending_metadata = parse_metadata(metadata_match.group("body"), line_no)
        continue

    id_match = id_re.search(line)
    if not id_match:
        continue
    advisory_id = id_match.group("id")
    if advisory_id in seen:
        failures.append(f"line {line_no}: duplicate ignore id `{advisory_id}`")
    metadata = pending_metadata
    pending_metadata = None
    if metadata is None:
        failures.append(f"line {line_no}: `{advisory_id}` missing rustsec-ignore metadata")
        metadata = {}
    missing_keys = [key for key in required_keys if not metadata.get(key)]
    if missing_keys:
        failures.append(
            f"line {line_no}: `{advisory_id}` metadata missing keys: {', '.join(missing_keys)}"
        )
    expiry = metadata.get("expiry", "")
    if expiry and not re.match(r"^\d{4}-\d{2}-\d{2}$", expiry):
        failures.append(f"line {line_no}: `{advisory_id}` expiry must be YYYY-MM-DD")
    elif expiry and expiry < today:
        failures.append(f"line {line_no}: `{advisory_id}` metadata expired on {expiry}")
    seen[advisory_id] = metadata

approved_set = set(approved)
seen_set = set(seen)
extra = sorted(seen_set - approved_set)
missing = sorted(approved_set - seen_set)
if extra:
    failures.append(
        "unapproved RustSec ignore id(s): "
        + ", ".join(extra)
        + "; update the approved baseline only with repository-health review evidence"
    )
if missing:
    failures.append(
        "approved RustSec ignore id(s) missing from deny.toml: "
        + ", ".join(missing)
        + "; update the baseline in the same reviewed patch when debt is removed"
    )

if failures:
    for failure in failures:
        print(f"error: {failure}", file=sys.stderr)
    raise SystemExit(1)

print(
    "ok: RustSec ignore baseline is reviewed, metadata-complete, and unexpired "
    f"({len(seen)} advisories)"
)
PY
