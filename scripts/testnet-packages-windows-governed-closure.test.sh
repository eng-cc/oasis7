#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKFLOW="$ROOT_DIR/.github/workflows/testnet-packages.yml"

python3 - "$WORKFLOW" <<'PY'
from pathlib import Path
import re
import sys


workflow = Path(sys.argv[1]).read_text(encoding="utf-8")


def step_body(name: str) -> str:
    match = re.search(
        rf"^      - name: {re.escape(name)}\n(?P<body>.*?)(?=^      - name: |\Z)",
        workflow,
        re.MULTILINE | re.DOTALL,
    )
    assert match, f"missing Testnet Packages workflow step: {name}"
    return match.group("body")


stage = step_body("Stage Windows governed deployment closure")
assert "matrix.platform == 'windows-x64'" in stage, (
    "Windows governed deployment closure must be staged only for the Windows package"
)
assert "windows-governed-closure" in stage, (
    "Windows governed deployment closure must have an explicit staged root"
)
assert '--bash-executable "${BASH}"' in stage, (
    "Windows governed deployment closure must pass the active Git Bash executable to its helper"
)
for required_source in (
    "public-testnet-governed-bootstrap-bundle-2026-06-06.windows.json",
    "public-testnet-governed-bootstrap-genesis-2026-06-06.windows.json",
    "public-testnet-governed-bootstrap-manifest-2026-06-06.windows.json",
    "public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.windows.txt",
    "generated-world",
    "doc/testing/evidence",
    "oasis7-windows-x64-ops-tools.tar.gz",
):
    assert required_source in stage, (
        "Windows governed deployment closure stage omits required source: "
        f"{required_source}"
    )

checksums = step_body("Generate checksums")
assert "windows-governed-closure" in checksums, (
    "windows-x64-SHA256SUMS must cover the staged governed closure"
)
assert re.search(
    r"find\s+windows-governed-closure\s+-type\s+f",
    checksums,
), (
    "windows-x64-SHA256SUMS must recursively cover every governed evidence/world sidecar/provenance file"
)

ops_checksums = step_body("Generate ops package checksum")
assert "windows-governed-closure" in ops_checksums, (
    "Windows ops-tools checksum must be generated inside the governed package closure"
)
assert 'cp "${{ matrix.platform }}-ops-tools-SHA256SUMS" ../' in ops_checksums, (
    "Windows ops-tools checksum sidecar must remain discoverable at the artifact root"
)

upload = step_body("Upload testnet package artifacts")
assert "windows-governed-closure" in upload, (
    "uploaded Windows package artifact must include the governed closure"
)
PY
