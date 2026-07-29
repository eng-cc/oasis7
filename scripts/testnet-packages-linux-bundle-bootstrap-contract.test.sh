#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKFLOW="$ROOT_DIR/.github/workflows/testnet-packages.yml"

python3 - "$WORKFLOW" <<'PY'
from pathlib import Path
import re
import sys

workflow = Path(sys.argv[1]).read_text(encoding="utf-8")


def step(name: str) -> tuple[int, str]:
    match = re.search(
        rf"^      - name: {re.escape(name)}\n(?P<body>.*?)(?=^      - name: |\Z)",
        workflow,
        re.MULTILINE | re.DOTALL,
    )
    assert match, f"missing Testnet Packages workflow step: {name}"
    return match.start(), match.group("body")


buildinfo_offset, buildinfo = step("Write BUILDINFO")
archive_offset, archive = step("Archive raw Linux bundle")
_, outer_checksums = step("Generate checksums")
assert buildinfo_offset < archive_offset, (
    "Linux raw bundle must be archived after BUILDINFO is written"
)
assert '"output/testnet-packages/assets/${{ matrix.platform }}-BUILDINFO"' in buildinfo, (
    "external Linux BUILDINFO artifact must remain available for outer checksums"
)
assert '"${{ matrix.platform }}-BUILDINFO"' in outer_checksums, (
    "outer artifact checksums must continue to cover BUILDINFO"
)
assert 'cp "output/testnet-packages/assets/linux-x64-BUILDINFO"' in archive, (
    "Linux bundle must embed the external BUILDINFO before archive"
)
assert '"$bundle_dir/BUILDINFO"' in archive, (
    "Linux bundle must place BUILDINFO at its root"
)
assert "find . -type f ! -name SHA256SUMS" in archive, (
    "inner bundle checksums must exclude SHA256SUMS to avoid recursive self-checks"
)
assert "> SHA256SUMS" in archive, "Linux bundle must write root SHA256SUMS"
assert archive.index("> SHA256SUMS") < archive.index("tar -C"), (
    "Linux bundle checksums must be created before archive"
)
PY

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-linux-bundle-contract.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT
bundle="$TMP_DIR/oasis7-linux-x64"
mkdir -p "$bundle/bin"
printf 'runtime\n' >"$bundle/bin/oasis7_chain_runtime"
printf 'repair\n' >"$bundle/bin/oasis7_world_repair_rebuild"
printf 'registry\n' >"$bundle/bin/oasis7_governance_registry_import"
printf 'commit=abcdef1234567890abcdef1234567890abcdef12\npackage_version=0.0.0+test\nrun_id=1\n' >"$bundle/BUILDINFO"
(
  cd "$bundle"
  files=()
  while IFS= read -r file; do
    files+=("$file")
  done < <(find . -type f ! -name SHA256SUMS -print | sort)
  shasum -a 256 "${files[@]}" > SHA256SUMS
  shasum -a 256 -c SHA256SUMS >/dev/null
)
tar -C "$TMP_DIR" -czf "$TMP_DIR/oasis7-linux-x64-bundle.tar.gz" oasis7-linux-x64
tar -tzf "$TMP_DIR/oasis7-linux-x64-bundle.tar.gz" | grep -Fxq 'oasis7-linux-x64/BUILDINFO'
tar -tzf "$TMP_DIR/oasis7-linux-x64-bundle.tar.gz" | grep -Fxq 'oasis7-linux-x64/SHA256SUMS'

echo "ok: Linux raw bundle embeds non-recursive bootstrap metadata"
