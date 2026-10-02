#!/usr/bin/env python3
"""Frozen and domain-specific checks for the testing evidence corpus.

The checker supplies both a CorpusView and immutable legacy baseline bytes;
this policy module has no implicit repository or filesystem lookup.
"""

from __future__ import annotations

import json
from typing import Any

import document_corpus as dc


EXPECTED_SNAPSHOT = "ddbc5a7d081cffd0c17697397ee89fbd69a98cd6"
EXPECTED_LIFECYCLE_COUNTS = {
    "AMBIGUOUS_LIFECYCLE": 0,
    "ARCHIVED_PROVENANCE": 7,
    "CURRENT_NAVIGATION": 1,
    "CURRENT_OPERATOR_INPUT": 2,
    "WINDOW_OBSERVATION": 23,
    "HISTORICAL_PROVENANCE": 88,
    "SUPPORTING_ARTIFACT": 6,
    "TEMPLATE_NOT_EVIDENCE": 1,
}
WINDOW_GROUPS = {
    "2026-07-03-to-06-lanes-and-claims": 3,
    "2026-07-03-to-05-public-surface-and-faucet": 4,
    "2026-07-03-to-05-deployment-and-resources": 5,
    "2026-07-05-api-viewer-projection": 4,
    "2026-07-05-same-world-hosted-entry": 7,
}
BATCH4_ROLES = {
    "public_testnet_transition", "public_testnet_governed_bootstrap",
    "public_testnet_governed_bootstrap_replay", "public_testnet_faucet_transition",
    "public_testnet_bootstrap_peer_input", "public_testnet_bootstrap_registry_input",
}
BATCH5_ROLES = {
    "historical_governance_registry_drill", "p2p_mixed_topology_transition",
    "p2p_triad_transition", "launcher_user_mode_ux",
}
BATCH6_ROLES = {
    "release_candidate_lineage", "pure_api_validation_history", "historical_issue_closeout",
    "signer_binding_history", "testnet_preflight_history", "archive_manifest_or_asset",
    "supporting_visual_manifest", "supporting_visual_asset", "audit_template",
}
BATCH_COUNTS = (("batch4", BATCH4_ROLES, 30), ("batch5", BATCH5_ROLES, 13), ("batch6", BATCH6_ROLES, 22))
TRIAD_PATHS = {
    "doc/testing/evidence/public-testnet-governed-bootstrap-validator-triad-bootstrap-peers-2026-09-15.txt": (
        "CURRENT_OPERATOR_INPUT", "public_testnet_bootstrap_peer_input", "current-committed-bootstrap-input",
        "current_operator_input_only_not_current_health_readiness_deployment_completion_or_public_status",
    ),
    "doc/testing/evidence/public-testnet-governed-bootstrap-validator-triad-registry-2026-09-15.json": (
        "CURRENT_OPERATOR_INPUT", "public_testnet_bootstrap_registry_input", "current-committed-bootstrap-input",
        "current_operator_input_only_not_current_health_readiness_deployment_completion_or_public_status",
    ),
    "doc/testing/evidence/public-testnet-validator-triad-authority-2026-09-15.json": (
        "HISTORICAL_PROVENANCE", "public_testnet_transition", "2026-09-15-validator-triad-candidate-staging",
        "historical_provenance_only_not_current_readiness_operator_sop_or_public_claim",
    ),
}
REQUIRED_README = (
    "../testing-manual.md",
    "formal-network-tiers-testnet-mechanism.runbook.md",
    "gameplay-agent-claim-economy-contract.prd.md",
    "非 evidence 模板",
    "不授权恢复操作",
    "窗口观测不能替代 fresh rerun",
)
FORBIDDEN_README = (
    "当前 release evidence bundle 该从哪里开始看",
    "三节点现在跑的是哪一版 runtime",
)
HASH_FIELDS = {"content_sha256", "group_sha256"}


def validate_evidence_policy(view: dc.CorpusView, model: dc.CorpusModel, baseline_bytes: bytes) -> list[dc.Diagnostic]:
    errors: list[dc.Diagnostic] = []
    try:
        baseline = dc.strict_json(baseline_bytes, "scripts/fixtures/document-corpus-v3/legacy/inventory.json")
    except dc.CorpusError as exc:
        return [exc.diagnostic]
    if not isinstance(baseline, dict) or baseline.get("version") != 1 or baseline.get("snapshot") != EXPECTED_SNAPSHOT or not isinstance(baseline.get("entries"), list):
        return [dc.Diagnostic("frozen-baseline-invalid", detail="legacy evidence baseline fixture identity/schema mismatch")]
    old_entries = baseline["entries"]
    old_by_path = {item["path"]: item for item in old_entries if isinstance(item, dict) and isinstance(item.get("path"), str)}
    if len(old_by_path) != len(old_entries) or len(old_entries) != 128:
        errors.append(dc.Diagnostic("frozen-baseline-invalid", detail=f"expected 128 unique frozen evidence rows, got {len(old_entries)}"))
    for name, roles, expected_count in BATCH_COUNTS:
        rows = [entry for entry in old_entries if isinstance(entry, dict) and entry.get("semantic_role") in roles]
        if len(rows) != expected_count:
            errors.append(dc.Diagnostic("frozen-baseline-invalid", detail=f"{name} expected {expected_count} frozen entries, got {len(rows)}"))
        for row in rows:
            for key in ("evidence_window", "claim_boundary", "content_sha256"):
                if not isinstance(row.get(key), str) or not row[key].strip():
                    errors.append(dc.Diagnostic("frozen-baseline-invalid", source_path=row.get("path"), detail=f"{name} requires {key}"))
    actual: dict[str, dict[str, Any]] = {path: dict(entry) for path, entry in model.evidence_entries_by_path.items()}
    group_of: dict[str, str] = {}
    for group_id, group in model.evidence_groups_by_id.items():
        for member in group["entries"]:
            if isinstance(member, dict) and isinstance(member.get("path"), str):
                row = dict(member)
                row["atomic_group"] = group_id
                row["group_sha256"] = group["group_sha256"]
                actual[row["path"]] = row
                group_of[row["path"]] = group_id
    for path, expected in old_by_path.items():
        current = actual.get(path)
        if current is None:
            errors.append(dc.Diagnostic("frozen-cohort-drift", source_path=path, detail="frozen evidence member was removed"))
            continue
        expected_group = expected.get("atomic_group")
        observed_group = group_of.get(path)
        if observed_group != expected_group:
            errors.append(dc.Diagnostic("frozen-cohort-drift", source_path=path, detail=f"expected atomic_group={expected_group!r}, got {observed_group!r}"))
        # Hashes are checked against source bytes and the group algorithm by core;
        # all semantic classification and review fields remain byte-for-byte values.
        expected_fields = {key: value for key, value in expected.items() if key not in HASH_FIELDS}
        current_fields = {key: value for key, value in current.items() if key not in HASH_FIELDS}
        if current_fields != expected_fields:
            errors.append(dc.Diagnostic("frozen-cohort-drift", source_path=path, detail="frozen evidence review fields changed"))
        if expected.get("semantic_role") in BATCH4_ROLES | BATCH5_ROLES | BATCH6_ROLES:
            for key in ("evidence_window", "claim_boundary", "content_sha256"):
                if not isinstance(current.get(key), str) or not current[key].strip():
                    errors.append(dc.Diagnostic("frozen-cohort-drift", source_path=path, detail=f"frozen batch evidence requires {key}"))
    for group_id, expected_size in WINDOW_GROUPS.items():
        expected_members = {path for path, item in old_by_path.items() if item.get("atomic_group") == group_id}
        current_members = {path for path, actual_id in group_of.items() if actual_id == group_id}
        if len(expected_members) != expected_size or current_members != expected_members:
            errors.append(dc.Diagnostic("frozen-cohort-drift", record_path=dc.group_record_path(group_id), detail=f"frozen window group expected {expected_size} exact members"))
    lifecycle_counts = {value: sum(item.get("lifecycle") == value for item in old_entries if isinstance(item, dict)) for value in EXPECTED_LIFECYCLE_COUNTS}
    if lifecycle_counts != EXPECTED_LIFECYCLE_COUNTS:
        errors.append(dc.Diagnostic("frozen-baseline-invalid", detail=f"legacy lifecycle counts differ: {lifecycle_counts}"))
    current_lifecycle_counts = {value: sum(actual.get(path, {}).get("lifecycle") == value for path in old_by_path) for value in EXPECTED_LIFECYCLE_COUNTS}
    if current_lifecycle_counts != EXPECTED_LIFECYCLE_COUNTS:
        errors.append(dc.Diagnostic("frozen-cohort-drift", detail=f"frozen lifecycle counts differ: {current_lifecycle_counts}"))
    # Keep the historical cross-file authority coverage invariant.
    legacy_sources = [path for path, item in old_by_path.items() if item.get("semantic_role") == "legacy_network_rehearsal" and path.startswith("doc/testing/evidence/shared-network-")]
    if len(legacy_sources) != 21:
        errors.append(dc.Diagnostic("frozen-baseline-invalid", detail=f"expected 21 legacy sources, got {len(legacy_sources)}"))
    authority_path = "doc/testing/evidence/legacy-shared-devnet-provenance-2026-07-26.md"
    try:
        authority = view.read_bytes(authority_path).decode("utf-8")
        missing = [path for path in legacy_sources if path.rsplit("/", 1)[-1] not in authority]
        if missing:
            errors.append(dc.Diagnostic("legacy-authority-coverage", source_path=authority_path, detail=f"missing references: {missing}"))
    except (dc.CorpusError, UnicodeDecodeError) as exc:
        errors.append(dc.Diagnostic("legacy-authority-coverage", source_path=authority_path, detail=str(exc)))
    for path, expected in TRIAD_PATHS.items():
        item = actual.get(path)
        fields = ("lifecycle", "semantic_role", "evidence_window", "claim_boundary")
        if item is None or tuple(item.get(field) for field in fields) != expected:
            errors.append(dc.Diagnostic("current-operator-input-boundary", source_path=path, detail="expected exact path-bound triad classification"))
    window_rows = [item for item in actual.values() if item.get("lifecycle") == "WINDOW_OBSERVATION" and item.get("path") in old_by_path]
    observed = {group: sum(item.get("atomic_group") == group for item in window_rows) for group in WINDOW_GROUPS}
    if len(window_rows) != 23 or observed != WINDOW_GROUPS:
        errors.append(dc.Diagnostic("window-observation-groups", detail=f"total={len(window_rows)} groups={observed}"))
    try:
        readme = view.read_bytes("doc/testing/evidence/README.md").decode("utf-8")
        if any(text not in readme for text in REQUIRED_README) or any(text in readme for text in FORBIDDEN_README):
            errors.append(dc.Diagnostic("readme-navigation", source_path="doc/testing/evidence/README.md", detail="current authority or freshness boundary missing/invalid"))
    except (dc.CorpusError, UnicodeDecodeError) as exc:
        errors.append(dc.Diagnostic("readme-navigation", detail=str(exc)))
    # New independent evidence is permitted, but it cannot join a frozen group
    # or masquerade as one of the exact frozen paths.
    frozen_groups = set(WINDOW_GROUPS)
    for path, group_id in group_of.items():
        if path not in old_by_path and group_id in frozen_groups:
            errors.append(dc.Diagnostic("frozen-cohort-drift", source_path=path, record_path=dc.group_record_path(group_id), detail="new evidence cannot join a frozen atomic group"))
    return errors
