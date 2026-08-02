#!/usr/bin/env python3
"""Validate the complete, snapshot-bound testing evidence inventory."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import sys


LIFECYCLES = {
    "CURRENT_NAVIGATION",
    "WINDOW_OBSERVATION",
    "HISTORICAL_PROVENANCE",
    "AMBIGUOUS_LIFECYCLE",
    "ARCHIVED_PROVENANCE",
    "SUPPORTING_ARTIFACT",
    "TEMPLATE_NOT_EVIDENCE",
}
ROLES = {
    "qa_engineer", "repository_health_engineer", "producer_system_designer",
    "gameplay_designer", "game_visual_interaction_designer", "runtime_engineer",
    "blockchain_ops_engineer", "wasm_platform_engineer", "agent_engineer",
    "viewer_engineer", "liveops_community",
}
DISPOSITIONS = {"retain", "needs_domain_decision", "delete_candidate"}
REQUIRED_FIELDS = {
    "path", "lifecycle", "semantic_role", "retention_owner", "domain_owner",
    "required_followup_roles", "authority", "backlink", "disposition", "rationale", "residual_risk",
}
EXPECTED_SNAPSHOT = "144e9f4bf1f4d18c27e2fb17823cc89343d97206"
EXPECTED_LIFECYCLE_COUNTS = {
    "AMBIGUOUS_LIFECYCLE": 0,
    "ARCHIVED_PROVENANCE": 7,
    "CURRENT_NAVIGATION": 1,
    "WINDOW_OBSERVATION": 23,
    "HISTORICAL_PROVENANCE": 87,
    "SUPPORTING_ARTIFACT": 6,
    "TEMPLATE_NOT_EVIDENCE": 1,
}


def classification(path: str) -> tuple[str, str, str, str, str]:
    """Return QA-conservative lifecycle, role, domain owner, authority, disposition."""
    name = Path(path).name
    if path == "doc/testing/evidence/README.md":
        return ("CURRENT_NAVIGATION", "hotspot_landing", "repository_health_engineer", "doc/testing/README.md", "retain")
    current_public_testnet_prefixes = (
        "doc/testing/evidence/public-testnet-api-viewer-projection-2026-07-05",
        "doc/testing/evidence/public-testnet-claims-boundary-review-2026-07-06.md",
        "doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03",
        "doc/testing/evidence/public-testnet-faucet-guard-ready-2026-07-05.md",
        "doc/testing/evidence/public-testnet-faucet-recovery-blocker-2026-07-04.md",
        "doc/testing/evidence/public-testnet-governed-reset-policy-announcement-2026-07-03.md",
        "doc/testing/evidence/public-testnet-node-deploy-2026-07-05.md",
        "doc/testing/evidence/public-testnet-provider-resource-provenance-2026-07-05.md",
        "doc/testing/evidence/public-testnet-public-surface-freshness-2026-07-03",
        "doc/testing/evidence/public-testnet-resource-delta-replay-2026-07-05.md",
        "doc/testing/evidence/public-testnet-runtime-world-resource-closure-2026-07-05.md",
        "doc/testing/evidence/public-testnet-same-world-hosted-entry-2026-07-05",
    )
    if path.startswith(current_public_testnet_prefixes):
        return ("WINDOW_OBSERVATION", "public_testnet_observation", "blockchain_ops_engineer", "doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.runbook.md", "retain")
    if path.startswith("doc/testing/evidence/public-testnet-governed-bootstrap-world-2026-06-06/"):
        return ("HISTORICAL_PROVENANCE", "public_testnet_governed_bootstrap_replay", "runtime_engineer", "doc/p2p/blockchain/public-testnet-governed-bootstrap.runbook.md", "retain")
    if "/archive/" in path:
        return ("ARCHIVED_PROVENANCE", "archive_manifest_or_asset", "viewer_engineer", "doc/testing/evidence/archive/visual-cleanup-2026-06-14/manifest.md", "retain")
    if path == "doc/testing/evidence/assets/manifest.md":
        return ("SUPPORTING_ARTIFACT", "supporting_visual_manifest", "game_visual_interaction_designer", "doc/world-simulator/viewer/viewer-visual-design-spec-2026-06-05.design.md", "retain")
    if "/assets/" in path:
        return ("SUPPORTING_ARTIFACT", "supporting_visual_asset", "game_visual_interaction_designer", "doc/testing/evidence/assets/manifest.md", "retain")
    if name == "token-genesis-allocation-audit-template.md":
        return ("TEMPLATE_NOT_EVIDENCE", "audit_template", "blockchain_ops_engineer", "doc/testing/governance/token-genesis-allocation-audit-checklist.prd.md", "retain")
    if path.startswith("doc/testing/evidence/shared-network-") or name.startswith("legacy-shared-devnet-"):
        return ("HISTORICAL_PROVENANCE", "legacy_network_rehearsal", "blockchain_ops_engineer", "doc/testing/evidence/legacy-shared-devnet-provenance-2026-07-26.md", "retain")
    if name.startswith("governance-registry-"):
        return ("HISTORICAL_PROVENANCE", "historical_governance_registry_drill", "blockchain_ops_engineer", "doc/p2p/blockchain/p2p-mainnet-security-governance-readiness.prd.md", "retain")
    if name.startswith("closed-beta-"):
        return ("HISTORICAL_PROVENANCE", "release_candidate_lineage", "qa_engineer", "doc/product/player-entry-distribution/access-modes-and-release-readiness.prd.md", "retain")
    if name.startswith("pure-api-"):
        return ("HISTORICAL_PROVENANCE", "pure_api_validation_history", "qa_engineer", "testing-manual.md", "retain")
    if name in {"release-evidence-bundle-task-game-018-2026-03-10.md", "testing-quality-trend-baseline-2026-03-11.md"}:
        return ("HISTORICAL_PROVENANCE", "release_candidate_lineage", "qa_engineer", "doc/product/player-entry-distribution/access-modes-and-release-readiness.prd.md", "retain")
    if name == "issue-160-first-capability-closeout-2026-05-17.md":
        return ("HISTORICAL_PROVENANCE", "historical_issue_closeout", "qa_engineer", "doc/product/world-rules-core-gameplay/first-session-and-continuation.prd.md", "retain")
    if name == "issue-162-industrial-chain-legibility-closeout-2026-05-04.md":
        return ("HISTORICAL_PROVENANCE", "historical_issue_closeout", "qa_engineer", "doc/world-simulator/m4/industrial-resource-flow-contract.prd.md", "retain")
    if name == "network-tier-signer-truth-binding-2026-06-05.md":
        return ("HISTORICAL_PROVENANCE", "signer_binding_history", "blockchain_ops_engineer", "doc/p2p/blockchain/public-testnet-governed-bootstrap.runbook.md", "retain")
    if name == "testnet-upgrade-preflight-drill-2026-06-01.md":
        return ("HISTORICAL_PROVENANCE", "testnet_preflight_history", "blockchain_ops_engineer", "doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.runbook.md", "retain")
    if name == "p2p-public-testnet-faucet-service-2026-05-19.md":
        return ("HISTORICAL_PROVENANCE", "public_testnet_faucet_transition", "blockchain_ops_engineer", "doc/p2p/blockchain/p2p-public-testnet-faucet-operator-runbook-2026-07-04.md", "retain")
    if name.startswith("p2p-"):
        if name == "p2p-user-mode-launcher-ux-2026-04-07.md":
            return ("HISTORICAL_PROVENANCE", "launcher_user_mode_ux", "viewer_engineer", "doc/p2p/network/mainnet-private-reachability-architecture.prd.md", "retain")
        if name.startswith(("p2p-mixed-topology-", "p2p-private-observer-")):
            return ("HISTORICAL_PROVENANCE", "p2p_mixed_topology_transition", "blockchain_ops_engineer", "doc/p2p/network/mainnet-private-reachability-architecture.prd.md", "retain")
        return ("HISTORICAL_PROVENANCE", "p2p_triad_transition", "blockchain_ops_engineer", "doc/p2p/node/node-triad-operations-observability.prd.md", "retain")
    if name.startswith("game-agent-"):
        return ("HISTORICAL_PROVENANCE", "agent_claim_validation", "agent_engineer", "doc/game/gameplay/gameplay-agent-claim-economy-contract.prd.md", "retain")
    if name.startswith("gameplay-"):
        return ("HISTORICAL_PROVENANCE", "gameplay_evidence", "gameplay_designer", "doc/game/prd.md", "retain")
    if name.startswith("hosted-world-"):
        return ("HISTORICAL_PROVENANCE", "hosted_access_validation", "blockchain_ops_engineer", "doc/p2p/blockchain/hosted-public-join-managed-identity-custody.prd.md", "retain")
    if name.startswith("mainchain-token-"):
        return ("HISTORICAL_PROVENANCE", "token_web_validation", "blockchain_ops_engineer", "doc/p2p/blockchain/p2p-mainnet-security-governance-readiness.prd.md", "retain")
    if name.startswith("public-testnet-"):
        governed_prefixes = (
            "public-testnet-governance-public-signers-2026-06-05",
            "public-testnet-liveops-public-signers-2026-06-05",
            "public-testnet-governed-bootstrap-",
            "public-testnet-five-node-inventory-2026-06-23",
        )
        if name.startswith(governed_prefixes):
            return ("HISTORICAL_PROVENANCE", "public_testnet_governed_bootstrap", "blockchain_ops_engineer", "doc/p2p/blockchain/public-testnet-governed-bootstrap.runbook.md", "retain")
        return ("HISTORICAL_PROVENANCE", "public_testnet_transition", "blockchain_ops_engineer", "doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.runbook.md", "retain")
    if name.startswith("provider-agent-"):
        return ("HISTORICAL_PROVENANCE", "provider_recertification", "agent_engineer", "doc/world-simulator/llm/provider-agent-dual-mode.prd.md", "retain")
    if name.startswith("post-onboarding-"):
        return ("HISTORICAL_PROVENANCE", "onboarding_protocol_smoke", "runtime_engineer", "doc/product/world-rules-core-gameplay/first-session-and-continuation.prd.md", "retain")
    if name == "software-safe-primary-web-entry-evidence-2026-04-07.md":
        return ("HISTORICAL_PROVENANCE", "viewer_entry_validation", "viewer_engineer", "doc/world-simulator/viewer/viewer-web-entry-compatibility.prd.md", "retain")
    if name == "software-safe-primary-entry-diagnostics-declutter-2026-04-28.md":
        return ("SUPPORTING_ARTIFACT", "viewer_ui_capture", "viewer_engineer", "doc/world-simulator/viewer/viewer-web-entry-compatibility.prd.md", "retain")
    if name.startswith("viewer-wasm-"):
        return ("SUPPORTING_ARTIFACT", "dated_viewer_wasm_proof", "viewer_engineer", "doc/world-simulator/viewer/README.md", "retain")
    return ("HISTORICAL_PROVENANCE", "dated_closeout_or_baseline", "qa_engineer", "doc/testing/README.md", "retain")


def generated_inventory(root: Path) -> dict[str, object]:
    paths = sorted(
        path.relative_to(root).as_posix()
        for path in (root / "doc/testing/evidence").rglob("*")
        if path.is_file() and path.name != "inventory.json"
    )
    entries = []
    for path in paths:
        lifecycle, semantic_role, domain_owner, authority, disposition = classification(path)
        followup_roles = [domain_owner]
        entry_name = Path(path).name
        if entry_name.startswith("game-agent-claim-"):
            followup_roles = ["agent_engineer", "runtime_engineer"]
        elif entry_name.startswith("gameplay-"):
            followup_roles = ["gameplay_designer", "producer_system_designer"]
        elif entry_name.startswith("governance-registry-"):
            followup_roles = ["blockchain_ops_engineer", "runtime_engineer"]
        elif entry_name.startswith("hosted-world-"):
            followup_roles = ["blockchain_ops_engineer", "viewer_engineer", "agent_engineer", "runtime_engineer"]
        elif entry_name.startswith("mainchain-token-"):
            followup_roles = ["blockchain_ops_engineer", "viewer_engineer", "runtime_engineer"]
        elif entry_name.startswith("provider-agent-"):
            followup_roles = ["agent_engineer", "runtime_engineer", "producer_system_designer"]
        elif entry_name.startswith("post-onboarding-"):
            followup_roles = ["runtime_engineer", "gameplay_designer", "viewer_engineer"]
        elif entry_name == "software-safe-primary-entry-diagnostics-declutter-2026-04-28.md":
            followup_roles = ["viewer_engineer", "game_visual_interaction_designer"]
        elif entry_name == "software-safe-primary-web-entry-evidence-2026-04-07.md":
            followup_roles = ["viewer_engineer", "runtime_engineer"]
        elif entry_name.startswith("viewer-wasm-"):
            followup_roles = ["viewer_engineer", "runtime_engineer", "game_visual_interaction_designer"]
        batch3_source = path.startswith("doc/testing/evidence/shared-network-")
        if batch3_source:
            followup_roles = ["blockchain_ops_engineer", "runtime_engineer", "liveops_community"]
        batch4_source = semantic_role in {
            "public_testnet_transition",
            "public_testnet_governed_bootstrap",
            "public_testnet_governed_bootstrap_replay",
            "public_testnet_faucet_transition",
        }
        if batch4_source:
            followup_roles = ["blockchain_ops_engineer", "runtime_engineer", "liveops_community"]
        batch5_source = semantic_role in {
            "historical_governance_registry_drill",
            "p2p_mixed_topology_transition",
            "p2p_triad_transition",
            "launcher_user_mode_ux",
        }
        if semantic_role == "historical_governance_registry_drill":
            followup_roles = ["blockchain_ops_engineer", "runtime_engineer", "liveops_community"]
        elif semantic_role in {"p2p_mixed_topology_transition", "p2p_triad_transition"}:
            followup_roles = ["blockchain_ops_engineer", "runtime_engineer"]
        elif semantic_role == "launcher_user_mode_ux":
            followup_roles = ["viewer_engineer", "game_visual_interaction_designer", "blockchain_ops_engineer", "runtime_engineer"]
        batch6_source = semantic_role in {
            "release_candidate_lineage",
            "pure_api_validation_history",
            "historical_issue_closeout",
            "signer_binding_history",
            "testnet_preflight_history",
            "archive_manifest_or_asset",
            "supporting_visual_manifest",
            "supporting_visual_asset",
            "audit_template",
        }
        if semantic_role in {"release_candidate_lineage", "pure_api_validation_history", "historical_issue_closeout"}:
            followup_roles = ["qa_engineer", "producer_system_designer"]
        elif semantic_role in {"signer_binding_history", "testnet_preflight_history"}:
            followup_roles = ["blockchain_ops_engineer", "runtime_engineer", "qa_engineer"]
        elif semantic_role in {"archive_manifest_or_asset", "supporting_visual_manifest", "supporting_visual_asset"}:
            followup_roles = ["game_visual_interaction_designer", "viewer_engineer", "qa_engineer"]
        elif semantic_role == "audit_template":
            followup_roles = ["blockchain_ops_engineer", "producer_system_designer", "qa_engineer"]
        batch7_source = lifecycle == "WINDOW_OBSERVATION"
        if batch7_source:
            followup_roles = ["blockchain_ops_engineer", "runtime_engineer", "viewer_engineer", "liveops_community", "qa_engineer"]
        batch2_reviewed = entry_name.startswith((
            "hosted-world-", "game-agent-claim-", "gameplay-", "provider-agent-",
            "post-onboarding-", "software-safe-", "viewer-wasm-", "mainchain-token-",
        ))
        entry = {
            "path": path,
            "lifecycle": lifecycle,
            "semantic_role": semantic_role,
            "retention_owner": "qa_engineer",
            "domain_owner": domain_owner,
            "required_followup_roles": followup_roles,
            "authority": authority,
            "backlink": "doc/testing/evidence/README.md",
            "disposition": disposition,
            "rationale": (
                "Batch-7 cross-role disposition: retained bounded observation-window evidence; never a continuously current endpoint, fleet, readiness, recovery, or release claim."
                if batch7_source
                else (
                    "Batch-6 cross-role disposition: retained historical candidate/validation/closeout, manifest-backed visual provenance, supporting asset, or explicit non-evidence template."
                    if batch6_source
                    else (
                        "Batch-5 cross-role disposition: retained dated governance, topology, triad, incident, or launcher diagnostic provenance; not current finality, security, network health, UX, or readiness."
                        if batch5_source
                        else (
                            "Batch-4 cross-role disposition: retained transition/bootstrap provenance; not current deployment, recovery, readiness, operator action, or public status."
                            if batch4_source
                            else (
                                "Batch-3 cross-role disposition: retained historical rehearsal, incident, and recovery provenance; not current network status, public availability, or release input."
                                if batch3_source
                                else (
                                    "Batch-2 cross-role reviewed classification; dated evidence is retained without current release claims."
                                    if batch2_reviewed
                                    else "QA-conservative batch-1 classification; dated naming alone is not a deletion decision."
                                )
                            )
                        )
                    )
                )
            ),
            "residual_risk": (
                "Dated pass, ready, running, endpoint, height, hash, peer, and faucet results can conflict across windows; current action requires a newly captured atomic window."
                if batch7_source
                else (
                    "Historical pass/closeout/visual/signer wording and captured assets are window-bound; templates are never evidence, and current claims require fresh candidate-bound validation."
                    if batch6_source
                    else (
                        "Dated pass, current-version, finality, signer, topology, height, recovery, and UX wording is window-bound; present claims require current authorities and fresh same-window evidence."
                        if batch5_source
                        else (
                            "Historical endpoints, signer and peer identities, runtime hashes, faucet state, and live-candidate wording are stale-window facts; current action requires formal runbooks and fresh same-window evidence."
                            if batch4_source
                            else (
                                "Historical pass, live, endpoint, operator-access, and rollback wording is window-bound and may be stale; current operator recovery, public status, and claims require the formal public-testnet runbook plus fresh lane and claims evidence."
                                if batch3_source
                                else "Topic validity and currentness require the listed domain owner and QA confirmation."
                            )
                        )
                    )
                )
            ),
        }
        if batch4_source:
            entry["evidence_window"] = "2026-06-governed-bootstrap" if "2026-06" in path else "2026-05-live-candidate-transition"
            entry["claim_boundary"] = "historical_provenance_only_not_current_readiness_operator_sop_or_public_claim"
            entry["content_sha256"] = hashlib.sha256((root / path).read_bytes()).hexdigest()
        if batch5_source:
            entry["evidence_window"] = "2026-03-governance-drill" if semantic_role == "historical_governance_registry_drill" else "2026-04-to-2026-07-p2p-transition"
            entry["claim_boundary"] = "historical_provenance_only_not_current_finality_security_network_health_ux_or_readiness"
            entry["content_sha256"] = hashlib.sha256((root / path).read_bytes()).hexdigest()
        if batch6_source:
            entry["evidence_window"] = "reusable-template" if semantic_role == "audit_template" else "dated-historical-or-supporting-evidence"
            entry["claim_boundary"] = "template_only_not_evidence" if semantic_role == "audit_template" else "historical_or_supporting_provenance_only_not_current_gate_or_visual_acceptance"
            entry["content_sha256"] = hashlib.sha256((root / path).read_bytes()).hexdigest()
        if batch7_source:
            if "api-viewer-projection" in path:
                atomic_group = "2026-07-05-api-viewer-projection"
            elif "same-world-hosted-entry" in path:
                atomic_group = "2026-07-05-same-world-hosted-entry"
            elif entry_name.startswith(("public-testnet-current-required-lanes-", "public-testnet-claims-boundary-review-")):
                atomic_group = "2026-07-03-to-06-lanes-and-claims"
            elif entry_name.startswith(("public-testnet-faucet-", "public-testnet-public-surface-freshness-")):
                atomic_group = "2026-07-03-to-05-public-surface-and-faucet"
            else:
                atomic_group = "2026-07-03-to-05-deployment-and-resources"
            entry["observation_window"] = atomic_group
            entry["atomic_group"] = atomic_group
            entry["claim_boundary"] = "bounded_window_only_not_current_endpoint_fleet_readiness_recovery_or_release"
            entry["content_sha256"] = hashlib.sha256((root / path).read_bytes()).hexdigest()
        entries.append(entry)
    grouped: dict[str, list[dict[str, object]]] = {}
    for entry in entries:
        if entry.get("lifecycle") == "WINDOW_OBSERVATION":
            grouped.setdefault(str(entry["atomic_group"]), []).append(entry)
    for members in grouped.values():
        digest_input = "".join(
            f"{member['content_sha256']}  {member['path']}\n"
            for member in sorted(members, key=lambda item: str(item["path"]))
        ).encode("utf-8")
        group_sha256 = hashlib.sha256(digest_input).hexdigest()
        for member in members:
            member["group_sha256"] = group_sha256
    return {
        "version": 1,
        "scope": "doc/testing/evidence/** excluding inventory.json",
        "snapshot": EXPECTED_SNAPSHOT,
        "freshness_boundary": "2026-08-02 batch-1 inventory; it records classification, not a release-readiness verdict.",
        "entries": entries,
    }


def check(root: Path, inventory_path: Path) -> list[str]:
    errors: list[str] = []
    try:
        data = json.loads(inventory_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return [f"inventory-load: {error}"]
    if data.get("version") != 1:
        errors.append("schema-version: expected 1")
    if data.get("snapshot") != EXPECTED_SNAPSHOT:
        errors.append(f"snapshot: expected {EXPECTED_SNAPSHOT}")
    entries = data.get("entries")
    if not isinstance(entries, list):
        return errors + ["schema-entries: expected list"]
    actual = {
        path.relative_to(root).as_posix()
        for path in (root / "doc/testing/evidence").rglob("*")
        if path.is_file() and path.name != "inventory.json"
    }
    seen: set[str] = set()
    for index, entry in enumerate(entries):
        if not isinstance(entry, dict):
            errors.append(f"entry-{index}: expected object")
            continue
        missing = REQUIRED_FIELDS - entry.keys()
        if missing:
            errors.append(f"entry-{index}: missing {sorted(missing)}")
            continue
        path = entry["path"]
        if path in seen:
            errors.append(f"duplicate-path: {path}")
        seen.add(path)
        if path not in actual:
            errors.append(f"missing-path: {path}")
        for reference_field in ("authority", "backlink"):
            reference = root / entry[reference_field]
            if not reference.exists():
                errors.append(f"missing-{reference_field}: {path} -> {entry[reference_field]}")
        if entry["lifecycle"] not in LIFECYCLES:
            errors.append(f"lifecycle: {path}")
        if entry["disposition"] not in DISPOSITIONS:
            errors.append(f"disposition: {path}")
        for field in ("retention_owner", "domain_owner"):
            if entry[field] not in ROLES:
                errors.append(f"owner-{field}: {path}")
        roles = entry["required_followup_roles"]
        if not isinstance(roles, list) or not roles or any(role not in ROLES for role in roles):
            errors.append(f"followup-roles: {path}")
        string_fields = REQUIRED_FIELDS - {"path", "required_followup_roles"}
        if not all(isinstance(entry[field], str) and entry[field].strip() for field in string_fields):
            errors.append(f"empty-field: {path}")
        if entry["disposition"] == "delete_candidate":
            for gate in ("semantic_absorption", "reference_repair", "owner_approval", "validation"):
                if not entry.get(gate):
                    errors.append(f"delete-gate-{gate}: {path}")
    if seen != actual:
        errors.append(f"path-coverage: inventory={len(seen)} filesystem={len(actual)}")
    lifecycle_counts = {
        lifecycle: sum(entry.get("lifecycle") == lifecycle for entry in entries if isinstance(entry, dict))
        for lifecycle in LIFECYCLES
    }
    if lifecycle_counts != EXPECTED_LIFECYCLE_COUNTS:
        errors.append(f"lifecycle-counts: {lifecycle_counts}")
    generated_entries = generated_inventory(root)["entries"]
    if entries != generated_entries:
        errors.append("classification-drift: committed entries differ from reviewed generator")
    legacy_sources = [
        entry["path"] for entry in entries
        if isinstance(entry, dict) and entry.get("semantic_role") == "legacy_network_rehearsal"
        and str(entry.get("path", "")).startswith("doc/testing/evidence/shared-network-")
    ]
    if len(legacy_sources) != 21:
        errors.append(f"legacy-source-count: expected 21, got {len(legacy_sources)}")
    legacy_authority = (root / "doc/testing/evidence/legacy-shared-devnet-provenance-2026-07-26.md").read_text(encoding="utf-8")
    missing_legacy_refs = [path for path in legacy_sources if Path(path).name not in legacy_authority]
    if missing_legacy_refs:
        errors.append(f"legacy-authority-coverage: {missing_legacy_refs}")
    batch4_entries = [
        entry for entry in entries if isinstance(entry, dict) and entry.get("semantic_role") in {
            "public_testnet_transition",
            "public_testnet_governed_bootstrap",
            "public_testnet_governed_bootstrap_replay",
            "public_testnet_faucet_transition",
        }
    ]
    if len(batch4_entries) != 27:
        errors.append(f"public-testnet-transition-count: expected 27, got {len(batch4_entries)}")
    for entry in batch4_entries:
        for field in ("evidence_window", "claim_boundary", "content_sha256"):
            if not isinstance(entry.get(field), str) or not entry[field].strip():
                errors.append(f"public-testnet-transition-{field}: {entry.get('path')}")
    batch5_entries = [
        entry for entry in entries if isinstance(entry, dict) and entry.get("semantic_role") in {
            "historical_governance_registry_drill",
            "p2p_mixed_topology_transition",
            "p2p_triad_transition",
            "launcher_user_mode_ux",
        }
    ]
    if len(batch5_entries) != 13:
        errors.append(f"p2p-governance-history-count: expected 13, got {len(batch5_entries)}")
    for entry in batch5_entries:
        for field in ("evidence_window", "claim_boundary", "content_sha256"):
            if not isinstance(entry.get(field), str) or not entry[field].strip():
                errors.append(f"p2p-governance-history-{field}: {entry.get('path')}")
    batch6_entries = [
        entry for entry in entries if isinstance(entry, dict) and entry.get("semantic_role") in {
            "release_candidate_lineage",
            "pure_api_validation_history",
            "historical_issue_closeout",
            "signer_binding_history",
            "testnet_preflight_history",
            "archive_manifest_or_asset",
            "supporting_visual_manifest",
            "supporting_visual_asset",
            "audit_template",
        }
    ]
    if len(batch6_entries) != 22:
        errors.append(f"dated-visual-template-count: expected 22, got {len(batch6_entries)}")
    for entry in batch6_entries:
        for field in ("evidence_window", "claim_boundary", "content_sha256"):
            if not isinstance(entry.get(field), str) or not entry[field].strip():
                errors.append(f"dated-visual-template-{field}: {entry.get('path')}")
    batch7_entries = [entry for entry in entries if isinstance(entry, dict) and entry.get("lifecycle") == "WINDOW_OBSERVATION"]
    expected_window_groups = {
        "2026-07-03-to-06-lanes-and-claims": 3,
        "2026-07-03-to-05-public-surface-and-faucet": 4,
        "2026-07-03-to-05-deployment-and-resources": 5,
        "2026-07-05-api-viewer-projection": 4,
        "2026-07-05-same-world-hosted-entry": 7,
    }
    observed_window_groups = {
        group: sum(entry.get("atomic_group") == group for entry in batch7_entries)
        for group in expected_window_groups
    }
    if len(batch7_entries) != 23 or observed_window_groups != expected_window_groups:
        errors.append(f"window-observation-groups: total={len(batch7_entries)} groups={observed_window_groups}")
    for entry in batch7_entries:
        for field in ("observation_window", "atomic_group", "group_sha256", "claim_boundary", "content_sha256"):
            if not isinstance(entry.get(field), str) or not entry[field].strip():
                errors.append(f"window-observation-{field}: {entry.get('path')}")
    readme = (root / "doc/testing/evidence/README.md").read_text(encoding="utf-8")
    required_navigation = (
        "../testing-manual.md",
        "formal-network-tiers-testnet-mechanism.runbook.md",
        "gameplay-agent-claim-economy-contract.prd.md",
        "非 evidence 模板",
        "不授权恢复操作",
        "窗口观测不能替代 fresh rerun",
    )
    if any(marker not in readme for marker in required_navigation):
        errors.append("readme-navigation: current authority or freshness boundary missing")
    forbidden_current_questions = (
        "当前 release evidence bundle 该从哪里开始看",
        "三节点现在跑的是哪一版 runtime",
    )
    if any(marker in readme for marker in forbidden_current_questions):
        errors.append("readme-navigation: historical evidence presented as a current entrypoint")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--print-generated", action="store_true")
    args = parser.parse_args()
    root = args.repo_root.resolve()
    if args.print_generated:
        print(json.dumps(generated_inventory(root), ensure_ascii=False, indent=2) + "\n")
        return 0
    errors = check(root, root / "doc/testing/evidence/inventory.json")
    if errors:
        print("doc-evidence-inventory-check: FAIL", file=sys.stderr)
        print("\n".join(errors), file=sys.stderr)
        return 1
    print("doc-evidence-inventory-check: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
