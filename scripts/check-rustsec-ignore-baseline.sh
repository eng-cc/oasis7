#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

deny_toml="${OASIS7_RUSTSEC_DENY_TOML:-deny.toml}"
direct_baseline="${OASIS7_RUSTSEC_DIRECT_DEP_BASELINE:-scripts/rustsec-ignore-direct-dependency-baseline.json}"

python3 - "$deny_toml" "$direct_baseline" <<'PY'
from __future__ import annotations

from datetime import date
import json
from pathlib import Path
import os
import re
import shlex
import subprocess
import sys

deny_path = Path(sys.argv[1])
direct_baseline_path = Path(sys.argv[2])
today = os.environ.get("OASIS7_RUSTSEC_TODAY") or date.today().isoformat()

approved = [
    "RUSTSEC-2021-0127",
    "RUSTSEC-2024-0436",
    "RUSTSEC-2026-0192",
]
required_keys = ("owner", "crate", "scope", "reason", "expiry", "validation", "local_crates")
required_direct_baseline_keys = (
    "schema_version",
    "owner",
    "reviewed_at",
    "expires",
    "rationale",
    "ignored_direct_manifests",
)
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
table_re = re.compile(r"^\s*\[(?P<table>[^\]]+)\]\s*$")
dep_key_re = re.compile(
    r"^\s*(?P<key>(?:[A-Za-z0-9_-]+|'[^']+'|\"[^\"]+\")"
    r"(?:\s*\.\s*(?:[A-Za-z0-9_-]+|'[^']+'|\"[^\"]+\"))*)\s*="
)
package_re = re.compile(r"\bpackage\s*=\s*(['\"])(?P<package>[^'\"]+)\1")
quoted_scalar_re = re.compile(r"^\s*(['\"])(?P<value>[^'\"]+)\1\s*$")

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

def inverse_package_name(args: list[str], advisory_id: str, line_no: int) -> str | None:
    if "-i" not in args:
        failures.append(f"line {line_no}: `{advisory_id}` validation is missing `-i` package")
        return None
    inverse_index = args.index("-i")
    if inverse_index + 1 >= len(args):
        failures.append(f"line {line_no}: `{advisory_id}` validation is missing `-i` package")
        return None
    package_spec = args[inverse_index + 1]
    return re.split(r"[@:=]", package_spec, maxsplit=1)[0]

def with_prefix_none(args: list[str]) -> list[str]:
    if "--prefix" in args:
        return args
    return [*args, "--prefix", "none"]

def csv_set(value: str) -> set[str]:
    return {part.strip() for part in value.split(",") if part.strip()}

def load_direct_baseline() -> dict:
    if not direct_baseline_path.exists():
        failures.append(f"RustSec direct dependency baseline not found: {direct_baseline_path}")
        return {}
    try:
        payload = json.loads(direct_baseline_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as err:
        failures.append(f"RustSec direct dependency baseline is not valid JSON: {err}")
        return {}
    if not isinstance(payload, dict):
        failures.append("RustSec direct dependency baseline must be a JSON object")
        return {}
    for key in required_direct_baseline_keys:
        if not payload.get(key):
            failures.append(f"RustSec direct dependency baseline missing key: {key}")
    if payload.get("schema_version") != 1:
        failures.append("RustSec direct dependency baseline schema_version must be 1")
    for key in ("reviewed_at", "expires"):
        value = str(payload.get(key, ""))
        if value and not re.match(r"^\d{4}-\d{2}-\d{2}$", value):
            failures.append(f"RustSec direct dependency baseline {key} must be YYYY-MM-DD")
    expires = str(payload.get("expires", ""))
    if expires and re.match(r"^\d{4}-\d{2}-\d{2}$", expires) and expires < today:
        failures.append(f"RustSec direct dependency baseline expired on {expires}")
    manifests = payload.get("ignored_direct_manifests", {})
    if manifests is not None and not isinstance(manifests, dict):
        failures.append("RustSec direct dependency baseline ignored_direct_manifests must be an object")
    return payload

def tracked_cargo_manifests() -> list[Path]:
    result = subprocess.run(
        ["git", "ls-files", "Cargo.toml", "**/Cargo.toml"],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode != 0:
        failures.append("failed to list tracked Cargo.toml files: " + result.stderr.strip())
        return []
    return [Path(line.strip()) for line in result.stdout.splitlines() if line.strip()]

def normalize_table_name(table: str) -> str:
    return table.replace('"', "").replace("'", "")

def split_toml_key(key: str) -> list[str]:
    parts: list[str] = []
    current: list[str] = []
    quote: str | None = None
    for char in key:
        if quote:
            current.append(char)
            if char == quote:
                quote = None
            continue
        if char in ("'", '"'):
            quote = char
            current.append(char)
            continue
        if char == ".":
            parts.append("".join(current).strip())
            current = []
            continue
        current.append(char)
    parts.append("".join(current).strip())
    return [unquote_toml_key(part) for part in parts if part]

def unquote_toml_key(key: str) -> str:
    key = key.strip()
    if len(key) >= 2 and key[0] == key[-1] and key[0] in ("'", '"'):
        return key[1:-1]
    return key

def quoted_scalar(value: str) -> str | None:
    match = quoted_scalar_re.match(value)
    if match:
        return match.group("value")
    return None

def direct_dependency_manifests(crate_name: str) -> set[str]:
    manifests: set[str] = set()
    for manifest in tracked_cargo_manifests():
        dependency_table = False
        dependency_subtable = False
        subtable_matches = False
        try:
            lines = manifest.read_text(encoding="utf-8").splitlines()
        except UnicodeDecodeError as err:
            failures.append(f"failed to read Cargo manifest {manifest}: {err}")
            continue
        for line in lines:
            table_match = table_re.match(line)
            if table_match:
                table = normalize_table_name(table_match.group("table"))
                dependency_table = bool(
                    re.search(
                        r"(^|\.)(workspace\.)?(dependencies|dev-dependencies|build-dependencies)$",
                        table,
                    )
                )
                subtable_match = re.search(
                    r"(^|\.)(dependencies|dev-dependencies|build-dependencies)\.([A-Za-z0-9_-]+)$",
                    table,
                )
                dependency_subtable = bool(subtable_match)
                subtable_matches = bool(subtable_match and subtable_match.group(3) == crate_name)
                if subtable_matches:
                    manifests.add(str(manifest))
                continue
            stripped = line.split("#", 1)[0].strip()
            if not stripped:
                continue
            if dependency_table:
                key_match = dep_key_re.match(stripped)
                if key_match:
                    key_parts = split_toml_key(key_match.group("key"))
                    key = key_parts[0] if key_parts else ""
                    value = stripped.split("=", 1)[1]
                    package_match = package_re.search(value or "")
                    dotted_package = (
                        len(key_parts) >= 2
                        and key_parts[-1] == "package"
                        and quoted_scalar(value) == crate_name
                    )
                    if (
                        key == crate_name
                        or package_match and package_match.group("package") == crate_name
                        or dotted_package
                    ):
                        manifests.add(str(manifest))
            elif dependency_subtable:
                package_match = package_re.search(stripped)
                if subtable_matches or package_match and package_match.group("package") == crate_name:
                    manifests.add(str(manifest))
    return manifests

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
    package_spec = inverse_package_name(args, advisory_id, line_no)
    if package_spec is None:
        return
    if metadata.get("crate") != package_spec:
        failures.append(
            f"line {line_no}: `{advisory_id}` crate metadata `{metadata.get('crate')}` "
            f"must match validation package `{package_spec}`"
        )
        return
    scoped_lines = run_cargo_tree(args, advisory_id, "scoped cargo tree")
    if scoped_lines is None:
        return
    if "-p" in args:
        workspace_args = ["-i", package_spec]
    else:
        workspace_args = args
    workspace_lines = run_cargo_tree(workspace_args, advisory_id, "workspace cargo tree")
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
    expected_local = csv_set(metadata.get("local_crates", ""))
    if expected_local:
        extra_local = sorted(local_crates - expected_local)
        missing_local = sorted(expected_local - local_crates)
        if extra_local:
            failures.append(
                f"`{advisory_id}` appears in unapproved local crate scope(s): "
                + ", ".join(extra_local)
            )
        if missing_local:
            failures.append(
                f"`{advisory_id}` approved local crate scope(s) missing from validation: "
                + ", ".join(missing_local)
            )
    if metadata.get("scope", "").startswith("oasis7_net libp2p"):
        extra_local = sorted(local_crates - allowed_libp2p_local_crates)
        if extra_local:
            failures.append(
                f"`{advisory_id}` appears in unapproved local crate scope(s): "
                + ", ".join(extra_local)
            )

def validate_direct_manifest_baseline() -> None:
    baseline = load_direct_baseline()
    if failures:
        return
    expected_by_package = baseline.get("ignored_direct_manifests", {})
    if not isinstance(expected_by_package, dict):
        return
    for advisory_id, metadata in seen.items():
        args = cargo_tree_args(metadata.get("validation", ""), advisory_id, 0)
        if args is None:
            continue
        validation_package_name = inverse_package_name(args, advisory_id, 0)
        if validation_package_name is None:
            continue
        package_name_value = metadata.get("crate", validation_package_name)
        if package_name_value != validation_package_name:
            failures.append(
                f"`{advisory_id}` crate metadata `{package_name_value}` "
                f"must match validation package `{validation_package_name}`"
            )
            continue
        actual = direct_dependency_manifests(package_name_value)
        expected_payload = expected_by_package.get(package_name_value, [])
        if not isinstance(expected_payload, list):
            failures.append(
                f"RustSec direct dependency baseline for `{package_name_value}` must be a list"
            )
            continue
        expected = {str(path) for path in expected_payload}
        extra = sorted(actual - expected)
        missing = sorted(expected - actual)
        if extra:
            failures.append(
                f"`{package_name_value}` direct dependency manifest scope grew beyond RustSec baseline: "
                + ", ".join(extra)
            )
        if missing:
            failures.append(
                f"`{package_name_value}` direct dependency manifest baseline is stale; "
                + "remove resolved path(s) from the baseline in the same reviewed patch: "
                + ", ".join(missing)
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
if not failures:
    validate_direct_manifest_baseline()

if failures:
    for failure in failures:
        print(f"error: {failure}", file=sys.stderr)
    raise SystemExit(1)

print(
    "ok: RustSec ignore baseline is reviewed, metadata-complete, and unexpired "
    f"({len(seen)} advisories)"
)
PY
