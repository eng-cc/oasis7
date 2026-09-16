#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

python3 - "$ROOT_DIR" <<'PY'
from pathlib import Path
import re
import sys

root = Path(sys.argv[1])
workflow_paths = [
    root / ".github/workflows/testnet-packages.yml",
    root / ".github/workflows/mainnet-packages.yml",
    root / ".github/workflows/release-packages.yml",
]


def package_native_body(workflow: str, path: Path) -> str:
    match = re.search(
        r"^  package-native:\n(?P<body>.*?)(?=^  [A-Za-z0-9_-]+:\n|\Z)",
        workflow,
        re.MULTILINE | re.DOTALL,
    )
    assert match, f"{path.name} must define package-native"
    return match.group("body")


def cache_steps(workflow: str, path: Path) -> list[str]:
    body = package_native_body(workflow, path)
    steps = re.findall(
        r"^      - uses: Swatinem/rust-cache@v2\n(?P<body>.*?)(?=^      - |\Z)",
        body,
        re.MULTILINE | re.DOTALL,
    )
    assert len(steps) == 2, (
        f"{path.name} package-native must keep separate Linux and non-Linux cache steps"
    )
    return steps


def linux_cache_guard(workflow: str, path: Path) -> str:
    body = package_native_body(workflow, path)
    cache_marker = "      - uses: Swatinem/rust-cache@v2\n        if: runner.os == 'Linux'"
    cache_offset = body.find(cache_marker)
    assert cache_offset >= 0, f"{path.name} must define a Linux package cache step"
    pre_cache = body[:cache_offset]
    guards = re.findall(
        r"^      - name: Compute Linux package cache save eligibility\n"
        r"(?P<body>.*?)(?=^      - |\Z)",
        pre_cache,
        re.MULTILINE | re.DOTALL,
    )
    assert len(guards) == 1, (
        f"{path.name} must compute Linux cache save eligibility after checkout"
    )
    return guards[0]


linux_keys: list[str] = []
expected_non_linux_keys = {
    "testnet-packages-${{ runner.os }}-${{ matrix.target_triple }}-v1",
    "mainnet-packages-${{ runner.os }}-${{ matrix.target_triple }}-v1",
    "release-packages-package-native-${{ runner.os }}-${{ matrix.target_triple }}-v2",
}
observed_non_linux_keys: set[str] = set()

for path in workflow_paths:
    workflow = path.read_text(encoding="utf-8")
    guard = linux_cache_guard(workflow, path)
    assert "id: linux-package-cache-write" in guard
    assert 'live_head="$(git rev-parse HEAD)"' in guard
    assert (
        '[[ "${GITHUB_REF}" == "refs/heads/main" && '
        '"${live_head}" == "${GITHUB_SHA}" ]]'
    ) in guard, (
        f"{path.name} must reject main-dispatch builds checked out at a tag/SHA"
    )
    assert 'echo "save=${cache_write}" >> "${GITHUB_OUTPUT}"' in guard
    steps = cache_steps(workflow, path)
    linux = [step for step in steps if "if: runner.os == 'Linux'" in step]
    non_linux = [step for step in steps if "if: runner.os != 'Linux'" in step]
    assert len(linux) == 1, f"{path.name} must gate its shared cache to Linux"
    assert len(non_linux) == 1, f"{path.name} must preserve a non-Linux cache step"

    linux_key = re.search(r"^          shared-key: (?P<key>.+)$", linux[0], re.MULTILINE)
    assert linux_key, f"{path.name} Linux cache must declare shared-key"
    linux_keys.append(linux_key.group("key"))
    assert "${{ runner.os }}" in linux_key.group("key")
    assert "${{ matrix.target_triple }}" in linux_key.group("key")
    assert "${{ inputs.build_profile }}" in linux_key.group("key") or "packaging" in linux_key.group("key")
    assert "github.ref_name" not in linux_key.group("key")
    assert "GITHUB_REF_NAME" not in linux_key.group("key")
    assert "cache-on-failure: false" in linux[0], f"{path.name} Linux cache must save successful jobs only"
    assert (
        "save-if: ${{ steps.linux-package-cache-write.outputs.save == 'true' }}"
        in linux[0]
    ), (
        f"{path.name} Linux cache must only save when checkout matches the trusted branch"
    )

    non_linux_key = re.search(r"^          shared-key: (?P<key>.+)$", non_linux[0], re.MULTILINE)
    assert non_linux_key, f"{path.name} non-Linux cache must declare shared-key"
    observed_non_linux_keys.add(non_linux_key.group("key"))
    assert "cache-on-failure: true" in non_linux[0], (
        f"{path.name} non-Linux cache behavior must remain unchanged"
    )

normalized_linux_keys = {
    key.replace("${{ inputs.build_profile }}", "packaging") for key in linux_keys
}
assert len(normalized_linux_keys) == 1, (
    "Testnet packaging profile=packaging, Mainnet, and Release Linux caches must share one key"
)
assert observed_non_linux_keys == expected_non_linux_keys, (
    "macOS/Windows package cache identities must remain unchanged"
)

print("ok: Linux package caches share trusted profile-aware identity; non-Linux cache scope is unchanged")
PY
