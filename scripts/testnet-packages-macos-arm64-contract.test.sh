#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKFLOW="$ROOT_DIR/.github/workflows/testnet-packages.yml"

python3 - "$WORKFLOW" <<'PY'
from pathlib import Path
import json
import re
import sys

workflow = Path(sys.argv[1]).read_text(encoding="utf-8")
plan = re.search(
    r"case \"\$\{\{ inputs\.package_scope \}\}\" in(?P<body>.*?)^          esac",
    workflow,
    re.MULTILINE | re.DOTALL,
)
assert plan, "missing Testnet Packages package-scope matrix"
matrix = plan.group("body")


def scope_entries(scope: str) -> list[dict[str, str]]:
    entry = re.search(
        rf"^            {re.escape(scope)}\)\n"
        r"              matrix='(?P<matrix>[^']+)'",
        matrix,
        re.MULTILINE,
    )
    assert entry, f"missing package scope: {scope}"
    return json.loads(entry.group("matrix"))["include"]


assert scope_entries("linux_macos_x64") == [
    {
        "os": "ubuntu-24.04",
        "platform": "linux-x64",
        "asset_name": "oasis7-linux-x64.deb",
        "target_triple": "native",
    },
    {
        "os": "macos-14",
        "platform": "macos-x64",
        "asset_name": "oasis7-macos-x64.dmg",
        "target_triple": "x86_64-apple-darwin",
    },
], "linux_macos_x64 must remain the exact existing x64 contract"
assert scope_entries("all_existing") == [
    {
        "os": "ubuntu-24.04",
        "platform": "linux-x64",
        "asset_name": "oasis7-linux-x64.deb",
        "target_triple": "native",
    },
    {
        "os": "macos-14",
        "platform": "macos-x64",
        "asset_name": "oasis7-macos-x64.dmg",
        "target_triple": "x86_64-apple-darwin",
    },
    {
        "os": "windows-2022",
        "platform": "windows-x64",
        "asset_name": "oasis7-windows-x64.exe",
        "target_triple": "native",
    },
], "all_existing must remain the exact existing x64/Windows contract"

assert scope_entries("linux_macos_arm64") == [
    {
        "os": "ubuntu-24.04",
        "platform": "linux-x64",
        "asset_name": "oasis7-linux-x64.deb",
        "target_triple": "native",
    },
    {
        "os": "macos-14",
        "platform": "macos-arm64",
        "asset_name": "oasis7-macos-arm64.dmg",
        "target_triple": "aarch64-apple-darwin",
    },
], "linux_macos_arm64 must add the native Apple Silicon package contract"

validation = re.search(
    r"^      - name: Validate bundle entrypoints\n(?P<body>.*?)(?=^      - name: |\Z)",
    workflow,
    re.MULTILINE | re.DOTALL,
)
assert validation, "missing bundle entrypoint validation step"
body = validation.group("body")
assert "file" in body and "arm64" in body, (
    "macos-arm64 package validation must assert Mach-O arm64 architecture"
)
PY

echo "ok: Testnet Packages keeps macos-x64 and requires additive macos-arm64 native artifacts"
