#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

python3 - "$repo_root" <<'PY'
from __future__ import annotations

import pathlib
import re
import subprocess
import sys

repo_root = pathlib.Path(sys.argv[1]).resolve()
scan_paths = [
    "crates",
    "scripts",
    "doc/testing/templates",
    "testing-manual.md",
]
pattern = r"shared_devnet|shared_network|shared-devnet|shared-network"
allowed_snippets = {
    "testing-manual.md": (
        "summary.json.evidence_contract.claim_readiness.shared_network_pass_blockers",
        "evidence_contract.claim_readiness.shared_network_pass_blockers",
    ),
    "scripts/network-tier-manifest.sh": (
        '"$tier" == "shared_devnet"',
        "shared_devnet is no longer a manifest tier",
    ),
    "scripts/p2p-mixed-topology-matrix.sh": (
        "shared_network_pass_inputs_ready:",
        "shared_network_pass_blockers:",
        "same_window_shared_network_evidence_ref",
        'shared_network_pass_inputs_ready',
        'shared_network_pass_blockers',
    ),
    "scripts/p2p-mixed-topology-matrix-smoke.sh": (
        'shared_network_pass_blockers',
    ),
    "scripts/public-testnet-rehearsal.sh": (
        "run_capture shared_network_gate",
        'shared_network_gate.rc',
    ),
    "scripts/release-candidate-bundle.sh": (
        "shared_devnet|shared_network)",
        '{"shared_devnet", "shared_network"}',
    ),
    "scripts/shared-devnet-blocker-packet.sh": (
        "shared-devnet-blocker-packet.sh is a legacy compatibility wrapper",
    ),
    "scripts/shared-devnet-rehearsal.sh": (
        "shared-devnet-rehearsal.sh is a legacy compatibility wrapper",
    ),
    "scripts/shared-network-track-gate.sh": (
        "shared-network-track-gate.sh is a legacy compatibility wrapper",
    ),
    "scripts/shared-network-track-gate-smoke.sh": (
        'shared_network_track_gate_smoke',
    ),
}

cmd = [
    "rg",
    "-n",
    "--no-heading",
    "-g",
    "!scripts/unified-world-code-terminology-scan.sh",
    pattern,
    *scan_paths,
]
proc = subprocess.run(cmd, cwd=repo_root, text=True, capture_output=True)
if proc.returncode not in {0, 1}:
    sys.stderr.write(proc.stderr)
    raise SystemExit(proc.returncode)

unexpected: list[str] = []
allowed_hits: list[str] = []
for line in proc.stdout.splitlines():
    rel, _, rest = line.partition(":")
    if not rest:
        unexpected.append(line)
        continue
    _, _, text = rest.partition(":")
    snippets = allowed_snippets.get(rel, ())
    if any(snippet in text for snippet in snippets):
        allowed_hits.append(line)
    else:
        unexpected.append(line)

if unexpected:
    print("unified-world-code-terminology-scan: FAIL")
    print("Unexpected legacy shared world terminology in active code/template surfaces:")
    for line in unexpected:
        print(f"  {line}")
    print()
    print("Allowed compatibility snippets are limited to:")
    for rel, snippets in sorted(allowed_snippets.items()):
        for snippet in snippets:
            print(f"  {rel}: {snippet}")
    raise SystemExit(1)

print("unified-world-code-terminology-scan: OK")
print(f"allowed compatibility hits: {len(allowed_hits)}")
PY
