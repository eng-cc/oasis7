#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKFLOW="$ROOT_DIR/.github/workflows/release-packages.yml"

python3 - "$WORKFLOW" <<'PY'
from pathlib import Path
import re
import sys

workflow = Path(sys.argv[1]).read_text(encoding="utf-8")

env = re.search(
    r"^  TRUNK_VERSION: (?P<version>[0-9]+\.[0-9]+\.[0-9]+)$",
    workflow,
    re.MULTILINE,
)
assert env, "release workflow must pin TRUNK_VERSION"
version = env.group("version")


def step(name: str) -> str:
    match = re.search(
        rf"^      - name: {re.escape(name)}\n(?P<body>.*?)(?=^      - name: |\Z)",
        workflow,
        re.MULTILINE | re.DOTALL,
    )
    assert match, f"missing release workflow step: {name}"
    return match.group("body")


cache = step("Restore pinned trunk binary")
assert "uses: actions/cache@v4" in cache
assert "path: ~/.cargo/bin/trunk" in cache
for key_part in (
    "runner.os",
    "runner.arch",
    "env.RUST_TOOLCHAIN",
    "env.TRUNK_VERSION",
):
    assert key_part in cache, f"trunk cache key must include {key_part}"

install = step("Install pinned trunk")
assert 'if: steps.trunk-cache.outputs.cache-hit != \'true\'' in install
assert 'cargo install trunk --locked --version "${TRUNK_VERSION}"' in install

verify = step("Verify pinned trunk")
assert 'trunk --version | grep -Fqx "trunk ${TRUNK_VERSION}"' in verify

build = step("Build web dists")
assert "trunk build --release" in build
assert workflow.index("Restore pinned trunk binary") < workflow.index("Build web dists")

assert f'cargo install trunk --locked --version "${{TRUNK_VERSION}}"' in workflow
assert "cargo install trunk --locked\n" not in workflow
print(f"ok: release workflow caches and verifies pinned trunk {version} before web build")
PY
