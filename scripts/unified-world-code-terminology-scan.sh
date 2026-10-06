#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

python3 - "$repo_root" <<'PY'
from __future__ import annotations

import hashlib
import json
import pathlib
import re
import stat
import sys

repo_root = pathlib.Path(sys.argv[1]).resolve()
scan_paths = [
    "crates",
    "scripts",
    "doc/testing/templates",
    "doc/engineering/governance/environment-lanes-and-inventory-2026-05-29.md",
    "doc/p2p/prd.md",
    "doc/p2p/prd.index.md",
    "doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.prd.md",
    "doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.design.md",
    "doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.runbook.md",
    "testing-manual.md",
]
pattern = r"shared_devnet|shared_network|shared-devnet|shared-network"
boundary_files = {
    "doc/engineering/governance/environment-lanes-and-inventory-2026-05-29.md",
    "doc/p2p/prd.md",
    "doc/p2p/prd.index.md",
    "doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.prd.md",
    "doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.design.md",
    "doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.runbook.md",
}
boundary_markers = (
    "legacy",
    "旧",
    "历史",
    "不再",
    "不是",
    "不能",
    "不得",
    "不等于",
    "不替代",
    "not a target",
    "no longer",
    "does not replace",
    "not public_testnet",
    "not mainnet",
)
legacy_fixture_root_rel = "scripts/fixtures/document-corpus-v3/legacy"
legacy_manifest_rel = f"{legacy_fixture_root_rel}/manifest.json"
legacy_manifest_sha256 = "10e09fe2204fc972fd1d6994766fd1b5345e1b4d805056bb5ee00ba5ac0a1d0a"
legacy_manifest_schema = "oasis7.document-corpus-v3-legacy-fixture/v1"
legacy_source_revision = "49a5a207e549f34b97fe265ab094160e7b402ae3"
legacy_payloads = {
    "document-corpus-inventory.json": (
        389326,
        "0a1c829e3e94021d15e41d23fdddab4d1d732a5a9510ddc124207edc699cfc95",
    ),
    "document-semantic-review-overrides.json": (
        50363,
        "a46e488df3c54e0dc6ad6309417b0ca3dad2929a0388b4aa719c6b589006527c",
    ),
    "inventory.json": (
        152689,
        "e4a2c74f954a0613033bf4cf245023becc38dbd88edefcf085b90ce9b56f10b3",
    ),
}
policy_compatibility_rel = "scripts/document_evidence_policy.py"
policy_compatibility_lines = (
    '    legacy_sources = [path for path, item in old_by_path.items() if item.get("semantic_role") == "legacy_network_rehearsal" and path.startswith("doc/testing/evidence/shared-network-")]',
    '    authority_path = "doc/testing/evidence/legacy-shared-devnet-provenance-2026-07-26.md"',
)
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
    "scripts/legacy-shared-devnet-provenance-smoke.sh": (
        "authority=doc/testing/evidence/legacy-shared-devnet-provenance-2026-07-26.md",
        '"shared_devnet"',
        '"shared-devnet-live-reset-20260523-01"',
        'shared_devnet-',
        'shared-network-shared-devnet-',
        'active generated shared_devnet capture reference remains',
        'legacy-shared-devnet-provenance-smoke',
    ),
    "scripts/doc-evidence-inventory-check.py": (
        'doc/testing/evidence/shared-network-',
        'legacy-shared-devnet-provenance-2026-07-26.md',
    ),
    "scripts/doc-evidence-inventory-check.test.py": (
        'legacy-shared-devnet-provenance-2026-07-26.md',
        'shared-network-ecs-triad-node-inventory-2026-03-30.md',
    ),
    "scripts/shared-network-track-gate.sh": (
        "shared-network-track-gate.sh is a legacy compatibility wrapper",
    ),
    "scripts/shared-network-track-gate-smoke.sh": (
        'shared_network_track_gate_smoke',
    ),
    "doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.prd.md": (
        "doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.prd.md",
        "scripts/shared-network-track-gate.sh",
    ),
    "doc/p2p/prd.md": (
        "doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.prd.md",
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
    "scripts/legacy-shared-devnet-provenance-smoke.sh",
    "scripts/shared-network-track-gate-smoke.sh",
    "scripts/shared-network-track-gate.sh",
}

def iter_scan_files() -> list[pathlib.Path]:
    files: list[pathlib.Path] = []
    for rel in scan_paths:
        path = repo_root / rel
        if path.is_file():
            files.append(path)
            continue
        if path.is_dir():
            files.extend(p for p in path.rglob("*") if p.is_file())
    return sorted(files)

def no_duplicate_json_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result

def verify_legacy_snapshot() -> tuple[set[str], str | None]:
    fixture_root = repo_root / legacy_fixture_root_rel
    parents = (
        repo_root / "scripts",
        repo_root / "scripts/fixtures",
        repo_root / "scripts/fixtures/document-corpus-v3",
        fixture_root,
    )
    try:
        for parent in parents:
            if not stat.S_ISDIR(parent.lstat().st_mode):
                return set(), f"fixture directory is missing or linked: {parent.relative_to(repo_root)}"
        expected_names = set(legacy_payloads) | {"manifest.json"}
        children = list(fixture_root.iterdir())
        if {child.name for child in children} != expected_names or len(children) != len(expected_names):
            return set(), "fixture file set differs from the fixed manifest"
        for child in children:
            if not stat.S_ISREG(child.lstat().st_mode):
                return set(), f"fixture path is not a regular file: {child.relative_to(repo_root)}"

        manifest_path = fixture_root / "manifest.json"
        manifest_bytes = manifest_path.read_bytes()
        if hashlib.sha256(manifest_bytes).hexdigest() != legacy_manifest_sha256:
            return set(), "fixed manifest SHA-256 mismatch"
        manifest = json.loads(
            manifest_bytes.decode("utf-8"), object_pairs_hook=no_duplicate_json_keys
        )
        if type(manifest) is not dict or set(manifest) != {"files", "schema", "source_revision"}:
            return set(), "manifest identity fields differ from the fixed schema"
        if manifest["schema"] != legacy_manifest_schema or manifest["source_revision"] != legacy_source_revision:
            return set(), "manifest schema or source revision differs from the fixed identity"
        entries = manifest["files"]
        if type(entries) is not list or len(entries) != len(legacy_payloads):
            return set(), "manifest file list differs from the fixed file set"
        expected_paths = list(legacy_payloads)
        if [entry.get("path") if type(entry) is dict else None for entry in entries] != expected_paths:
            return set(), "manifest file paths or order differ from the fixed file set"

        trusted_paths = {legacy_manifest_rel}
        for entry in entries:
            if type(entry) is not dict or set(entry) != {"bytes", "path", "sha256"}:
                return set(), "manifest file entry has an unexpected shape"
            rel = entry["path"]
            expected_bytes, expected_sha256 = legacy_payloads[rel]
            if (
                type(entry["bytes"]) is not int
                or entry["bytes"] != expected_bytes
                or entry["sha256"] != expected_sha256
            ):
                return set(), f"manifest payload declaration differs from fixed bytes: {rel}"
            payload = fixture_root / rel
            payload_bytes = payload.read_bytes()
            if len(payload_bytes) != expected_bytes or hashlib.sha256(payload_bytes).hexdigest() != expected_sha256:
                return set(), f"frozen payload byte count or SHA-256 mismatch: {rel}"
            trusted_paths.add(f"{legacy_fixture_root_rel}/{rel}")
        return trusted_paths, None
    except (OSError, UnicodeDecodeError, ValueError, TypeError, KeyError) as exc:
        return set(), f"snapshot verification error: {exc}"


unexpected: list[str] = []
allowed_hits: list[str] = []
policy_reference_counts = {line: 0 for line in policy_compatibility_lines}
scan_re = re.compile(pattern)
scan_files = iter_scan_files()
legacy_snapshot_paths, legacy_snapshot_error = verify_legacy_snapshot()
if legacy_snapshot_error is not None:
    unexpected.append(f"{legacy_manifest_rel}: frozen snapshot verification failed: {legacy_snapshot_error}")
for path in scan_files:
    rel = path.relative_to(repo_root).as_posix()
    if rel == "scripts/unified-world-code-terminology-scan.sh":
        continue
    try:
        text = path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        continue
    if rel in legacy_snapshot_paths:
        for line_no, text_line in enumerate(text.splitlines(), start=1):
            if scan_re.search(text_line):
                allowed_hits.append(f"{rel}:{line_no}:{text_line}")
        continue
    snippets = allowed_snippets.get(rel, ())
    for line_no, text_line in enumerate(text.splitlines(), start=1):
        if not scan_re.search(text_line):
            continue
        rendered = f"{rel}:{line_no}:{text_line}"
        if rel == policy_compatibility_rel and text_line in policy_compatibility_lines:
            policy_reference_counts[text_line] += 1
            allowed_hits.append(rendered)
        elif any(snippet in text_line for snippet in snippets):
            allowed_hits.append(rendered)
        elif rel in boundary_files and any(marker in text_line for marker in boundary_markers):
            allowed_hits.append(rendered)
        else:
            unexpected.append(rendered)

for compatibility_line, count in policy_reference_counts.items():
    if count != 1:
        unexpected.append(
            f"{policy_compatibility_rel}: exact compatibility reference must occur once; found {count}"
        )

path_pattern = re.compile(pattern)
allowed_path_hits: list[str] = []
for path in scan_files:
    rel = path.relative_to(repo_root).as_posix()
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
