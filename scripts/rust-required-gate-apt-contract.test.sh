#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKFLOW="$ROOT_DIR/.github/workflows/rust.yml"

python3 - "$WORKFLOW" <<'PY'
from pathlib import Path
import re
import sys

workflow = Path(sys.argv[1]).read_text(encoding="utf-8")
match = re.search(
    r"^      - name: Install system deps\n(?P<body>.*?)(?=^      - name: |\Z)",
    workflow,
    re.MULTILINE | re.DOTALL,
)
assert match, "required-gate Install system deps step is missing"
body = match.group("body")

assert "steps.scope.outputs.needs_system_deps" in body, (
    "contract must inspect the required-gate system-deps step, not a full-tier job"
)
assert "max_attempts=3" in body, "apt operations must use exactly three bounded attempts"
assert "timeout --foreground" in body, "each apt attempt must have a foreground timeout"
assert "Acquire::Retries=3" in body, "apt must retry downloads within each bounded attempt"
assert "Acquire::http::Timeout=30" in body, "apt HTTP downloads need a per-attempt timeout"
assert "Acquire::https::Timeout=30" in body, "apt HTTPS downloads need a per-attempt timeout"
assert "apt-get" in body and "update" in body and "install" in body
assert "run_apt update update" in body
assert "run_apt install install -y" in body
assert "sudo apt-get" not in body, "required-gate apt calls must use the bounded wrapper"
assert "failed after" in body and "return 1" in body, (
    "persistent apt failures must remain explicit"
)
assert "pkg-config" in body and "libudev-dev" in body, (
    "the existing required-gate package set must remain intact"
)
print("ok: required-gate apt retry/timeout/failure contract")
PY
