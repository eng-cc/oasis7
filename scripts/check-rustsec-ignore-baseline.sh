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
import shlex
import subprocess
import sys

deny_path = Path(sys.argv[1])
today = os.environ.get("OASIS7_RUSTSEC_TODAY") or date.today().isoformat()

approved = [
    "RUSTSEC-2021-0127",
    "RUSTSEC-2024-0384",
    "RUSTSEC-2024-0436",
    "RUSTSEC-2025-0012",
    "RUSTSEC-2026-0118",
    "RUSTSEC-2026-0119",
]
required_keys = ("owner", "scope", "reason", "expiry", "validation")
allowed_libp2p_local_crates = {
    "oasis7_net",
    "oasis7_node",
    "oasis7",
    "oasis7_client_launcher",
}

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

def cargo_tree_args(validation: str, advisory_id: str, line_no: int) -> list[str] | None:
    try:
        parts = shlex.split(validation)
    except ValueError as err:
        failures.append(f"line {line_no}: `{advisory_id}` validation is not shell-parseable: {err}")
        return None
    if len(parts) < 4 or parts[0:2] != ["cargo", "tree"] or "-i" not in parts:
        failures.append(
            f"line {line_no}: `{advisory_id}` validation must be a `cargo tree -i ...` command"
        )
        return None
    return parts[2:]

def with_prefix_none(args: list[str]) -> list[str]:
    if "--prefix" in args:
        return args
    return [*args, "--prefix", "none"]

def run_cargo_tree(args: list[str], advisory_id: str, label: str) -> list[str] | None:
    cmd = ["env", "-u", "RUSTC_WRAPPER", "cargo", "tree", *with_prefix_none(args)]
    result = subprocess.run(cmd, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if result.returncode != 0:
        failures.append(
            f"`{advisory_id}` {label} validation failed: {' '.join(cmd)}\n"
            + result.stderr.strip()
        )
        return None
    return [line.strip() for line in result.stdout.splitlines() if line.strip()]

def package_name(line: str) -> str | None:
    match = re.match(r"^(?P<name>[A-Za-z0-9_-]+)\s+v\S+", line)
    if match:
        return match.group("name")
    return None

def is_local_crate(line: str) -> bool:
    return " (/ " in line or " (" in line and "/crates/" in line

def validate_dependency_scope(advisory_id: str, metadata: dict[str, str], line_no: int) -> None:
    args = cargo_tree_args(metadata.get("validation", ""), advisory_id, line_no)
    if args is None:
        return
    scoped_lines = run_cargo_tree(args, advisory_id, "scoped cargo tree")
    if scoped_lines is None:
        return
    if "-p" not in args:
        return

    inverse_index = args.index("-i")
    if inverse_index + 1 >= len(args):
        failures.append(f"line {line_no}: `{advisory_id}` validation is missing `-i` package")
        return
    package_spec = args[inverse_index + 1]
    workspace_lines = run_cargo_tree(["-i", package_spec], advisory_id, "workspace cargo tree")
    if workspace_lines is None:
        return

    scoped_nonlocal = {
        line.removesuffix(" (*)")
        for line in scoped_lines
        if package_name(line) and not is_local_crate(line)
    }
    workspace_nonlocal = {
        line.removesuffix(" (*)")
        for line in workspace_lines
        if package_name(line) and not is_local_crate(line)
    }
    extra_nonlocal = sorted(workspace_nonlocal - scoped_nonlocal)
    if extra_nonlocal:
        failures.append(
            f"`{advisory_id}` dependency closure escaped approved validation scope; "
            + "extra third-party path line(s): "
            + "; ".join(extra_nonlocal)
        )

    local_crates = {
        name
        for line in workspace_lines
        if is_local_crate(line)
        for name in [package_name(line)]
        if name
    }
    if metadata.get("scope", "").startswith("oasis7_net libp2p"):
        extra_local = sorted(local_crates - allowed_libp2p_local_crates)
        if extra_local:
            failures.append(
                f"`{advisory_id}` appears in unapproved local crate scope(s): "
                + ", ".join(extra_local)
            )

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
    if line.lstrip().startswith("#"):
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

if not failures:
    for advisory_id, metadata in seen.items():
        validate_dependency_scope(advisory_id, metadata, 0)

if failures:
    for failure in failures:
        print(f"error: {failure}", file=sys.stderr)
    raise SystemExit(1)

print(
    "ok: RustSec ignore baseline is reviewed, metadata-complete, and unexpired "
    f"({len(seen)} advisories)"
)
PY
