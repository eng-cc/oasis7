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
allowed_path_names = {
    "doc/testing/templates/shared-network-exit-decision-template.md",
    "doc/testing/templates/shared-network-incident-review-template.md",
    "doc/testing/templates/shared-network-incident-template.md",
    "doc/testing/templates/shared-network-mixed-topology-gate-template.md",
    "doc/testing/templates/shared-network-promotion-record-template.md",
    "doc/testing/templates/shared-network-rollback-target-template.md",
    "doc/testing/templates/shared-network-shared-access-check-template.md",
    "doc/testing/templates/shared-network-track-gate-lanes.canary.template.tsv",
    "doc/testing/templates/shared-network-track-gate-lanes.shared_devnet.template.tsv",
    "doc/testing/templates/shared-network-track-gate-lanes.staging.template.tsv",
    "doc/testing/templates/shared-network-track-gate-template.md",
    "scripts/shared-devnet-blocker-packet-smoke.sh",
    "scripts/shared-devnet-blocker-packet.sh",
    "scripts/shared-devnet-rehearsal-smoke.sh",
    "scripts/shared-devnet-rehearsal.sh",
    "scripts/shared-network-track-gate-smoke.sh",
    "scripts/shared-network-track-gate.sh",
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

files_cmd = ["rg", "--files", *scan_paths]
files_proc = subprocess.run(files_cmd, cwd=repo_root, text=True, capture_output=True)
if files_proc.returncode != 0:
    sys.stderr.write(files_proc.stderr)
    raise SystemExit(files_proc.returncode)

path_pattern = re.compile(pattern)
allowed_path_hits: list[str] = []
for rel in files_proc.stdout.splitlines():
    if rel == "scripts/unified-world-code-terminology-scan.sh":
        continue
    if not path_pattern.search(rel):
        continue
    if rel in allowed_path_names:
        allowed_path_hits.append(rel)
    else:
        unexpected.append(f"{rel}: legacy terminology in path name")

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
print(f"allowed compatibility content hits: {len(allowed_hits)}")
print(f"allowed compatibility path hits: {len(allowed_path_hits)}")
PY
