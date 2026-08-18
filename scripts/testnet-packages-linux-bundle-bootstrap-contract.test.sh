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
_, outer_checksums = step("Generate checksums")
assert '"platform":"linux-x64"' in workflow
assert '"asset_name":"oasis7-linux-x64.deb"' in workflow, (
    "Linux package must publish the Debian installer as the sole primary asset"
)
assert "AppImage" not in workflow, "Linux AppImage must not remain a published artifact"
assert "Archive raw Linux bundle" not in workflow, "raw Linux tar must not remain a release artifact"
assert "Package secondary Linux .deb" not in workflow, "duplicate Linux deb step must be removed"
assert '"output/testnet-packages/assets/${{ matrix.platform }}-BUILDINFO"' in buildinfo, (
    "external Linux BUILDINFO artifact must remain available for outer checksums"
)
assert '"${{ matrix.platform }}-BUILDINFO"' in outer_checksums, (
    "outer artifact checksums must continue to cover BUILDINFO"
)
assert "ops package" in workflow.lower(), "Linux workflow must publish a separate ops package"
assert "oasis7-${{ matrix.platform }}-ops-tools.tar.gz" in workflow, (
    "Linux workflow must upload the checksummed ops-tools archive"
)
PY

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-linux-bundle-contract.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT
bundle="$TMP_DIR/oasis7-linux-x64-ops-tools"
mkdir -p "$bundle/bin"
printf 'repair\n' >"$bundle/bin/oasis7_world_repair_rebuild"
printf 'registry-import\n' >"$bundle/bin/oasis7_governance_registry_import"
printf 'registry-audit\n' >"$bundle/bin/oasis7_governance_registry_audit"
printf '{"opsToolsSchemaVersion":1}\n' >"$bundle/.oasis7-ops-tools-manifest.json"
(
  cd "$bundle"
  files=()
  while IFS= read -r file; do
    files+=("$file")
  done < <(find . -type f ! -name SHA256SUMS -print | sort)
  shasum -a 256 "${files[@]}" > SHA256SUMS
  shasum -a 256 -c SHA256SUMS >/dev/null
)
tar -C "$TMP_DIR" -czf "$TMP_DIR/oasis7-linux-x64-ops-tools.tar.gz" oasis7-linux-x64-ops-tools
tar -tzf "$TMP_DIR/oasis7-linux-x64-ops-tools.tar.gz" | grep -Fxq 'oasis7-linux-x64-ops-tools/.oasis7-ops-tools-manifest.json'
tar -tzf "$TMP_DIR/oasis7-linux-x64-ops-tools.tar.gz" | grep -Fxq 'oasis7-linux-x64-ops-tools/SHA256SUMS'

echo "ok: Linux publishes deb-only player package plus checksummed ops package"
