#!/usr/bin/env python3
"""Validate complete document-corpus coverage without making semantic decisions."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from typing import Any


INVENTORY = "doc/.governance/document-corpus-inventory.json"
SEMANTIC_OVERLAY = "doc/.governance/document-semantic-review-overrides.json"
REGISTRY = "doc/.governance/top-level-directory-registry.json"
NESTED_ROOT = "doc/testing/evidence"
NESTED_INVENTORY = f"{NESTED_ROOT}/inventory.json"
NESTED_CHECKER = "scripts/doc-evidence-inventory-check.py"
CONTROL_PATHS = (INVENTORY, SEMANTIC_OVERLAY, NESTED_INVENTORY)
PRODUCT_MODULES = ("agents-world-simulation", "player-entry-distribution", "world-infrastructure", "world-rules-core-gameplay")
LIFECYCLE_ROUTES = {"active_candidate", "historical_candidate", "review_required"}
REVIEWED_DISPOSITIONS = {
    "retain_current_authority", "retain_active_template", "retain_repeatable_procedure",
    "retain_historical_evidence_linked", "controlled_exception_empty_pool",
    "retain_current_design_supplement", "retain_comparison_target", "retain_supporting_artifact",
    "retain_current_procedure", "retain_dated_window_evidence",
    "retain_dated_window_evidence_asset", "retain_historical_governance_decision",
    "retain_current_target_contract",
    "retain_outstanding_fulfillment_obligation", "retain_historical_campaign_material",
    "retain_explanatory_overview", "retain_unpublished_external_draft",
    "retain_historical_visual_asset", "retain_current_companion_authority",
    "retain_non_authoritative_ideation",
    "retain_mixed_contract_snapshot", "retain_historical_decision_background",
    "retain_historical_benchmark", "retain_mixed_runbook_observation",
    "retain_legacy_rehearsal_provenance",
    "retain_current_controlled_runbook",
    "retain_active_template_or_example", "retain_executable_test_plan",
    "retain_current_governance_control", "retain_current_channel_material",
    "retain_unpublished_external_support_bundle",
    "retain_current_design_companion", "retain_mixed_implementation_record",
    "retain_orphan_draft_design", "retain_historical_validation_index",
    "retain_current_interface_authority",
}
SEMANTIC_OWNERS = {"qa_engineer", "gameplay_designer", "game_visual_interaction_designer", "repository_health_engineer", "viewer_engineer", "blockchain_ops_engineer", "runtime_engineer", "liveops_community", "producer_system_designer", "agent_engineer", "wasm_platform_engineer"}


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def object_kind(path: str) -> str:
    name = Path(path).name
    for suffix, kind in ((".prd.md", "prd"), (".design.md", "design"), (".manual.md", "manual"), (".runbook.md", "runbook"), (".migration.md", "migration")):
        if name.endswith(suffix): return kind
    if name == "README.md": return "landing"
    if name == "prd.index.md": return "prd_index"
    return "markdown_record" if name.endswith(".md") else "supporting_artifact"


def lifecycle(path: str, kind: str) -> str:
    name = Path(path).name
    if any(value in name for value in ("archive", "legacy", "historical", "-2026-", "-2025-", "-2024-")):
        return "historical_candidate"
    if path.startswith("doc/product/") or kind in {"prd", "design", "manual", "runbook", "landing", "prd_index"}:
        return "active_candidate"
    return "review_required"


def registry(root: Path) -> dict[str, dict[str, str]]:
    data = json.loads((root / REGISTRY).read_text(encoding="utf-8"))
    return {entry["name"]: entry for entry in data["directories"]}


def authority_layer(registry_type: str) -> str:
    if registry_type == "product_overlay": return "product_overlay"
    if registry_type == "evidence_domain": return "evidence_domain"
    if registry_type in {"retired_archive", "ephemeral_evidence_pool"}: return "controlled_exception"
    return "professional_domain"


def root_contract(root: Path, top: str, entries: dict[str, dict[str, str]]) -> dict[str, str]:
    if top == "":
        return {"registry_type": "doc_root", "structural_owner": "repository_health_engineer", "authority_entry": "doc/README.md", "authority_layer": "professional_domain"}
    if top == ".governance":
        return {"registry_type": "governance_control", "structural_owner": "repository_health_engineer", "authority_entry": "doc/engineering/doc-governance/README.md", "authority_layer": "controlled_exception"}
    entry = entries[top]
    return {"registry_type": entry["type"], "structural_owner": entry["owner"], "authority_entry": entry["entry"], "authority_layer": authority_layer(entry["type"])}


def direct_object(root: Path, path: str, entries: dict[str, dict[str, str]]) -> dict[str, str]:
    parts = path.split("/")
    top = parts[1] if len(parts) > 2 else ""
    contract = root_contract(root, top, entries)
    kind = object_kind(path)
    return {
        "path": path, "content_sha256": digest((root / path).read_bytes()), **contract,
        "semantic_decision_owner": contract["structural_owner"], "object_kind": kind,
        "lifecycle_candidate": lifecycle(path, kind), "inventory_disposition": "retain_for_owner_review",
        "routing_batch": f"{top or 'doc-root'}-corpus-review",
        "routing_note": "heuristic routing only; no lifecycle, authority, migration, or deletion decision is authorized by this inventory",
    }


def nested_delegate(root: Path) -> dict[str, Any]:
    data = json.loads((root / NESTED_INVENTORY).read_text(encoding="utf-8"))
    return {"path": NESTED_INVENTORY, "exact_scope": "doc/testing/evidence/** excluding inventory.json", "checker_command": "python3 scripts/doc-evidence-inventory-check.py --repo-root <repo-root>", "expected_version": 1, "sha256": digest((root / NESTED_INVENTORY).read_bytes()), "object_count": len(data.get("entries", []))}


def generated(root: Path) -> dict[str, Any]:
    entries = registry(root)
    objects = []
    for candidate in sorted((root / "doc").rglob("*")):
        if not candidate.is_file(): continue
        path = candidate.relative_to(root).as_posix()
        if path in CONTROL_PATHS or path.startswith(f"{NESTED_ROOT}/"): continue
        objects.append(direct_object(root, path, entries))
    controls = [{"path": INVENTORY, "content_sha256": "self-referential-control"}, {"path": SEMANTIC_OVERLAY, "content_sha256": digest((root / SEMANTIC_OVERLAY).read_bytes())}, {"path": NESTED_INVENTORY, "content_sha256": digest((root / NESTED_INVENTORY).read_bytes())}]
    return {"version": 2, "scope": "all doc files through direct objects, delegated evidence objects, and controls", "decision_boundary": "routing is conservative heuristic input only", "product_modules": list(PRODUCT_MODULES), "controls": controls, "delegates": [nested_delegate(root)], "objects": objects}


def nested_paths(root: Path) -> tuple[list[str], list[str]]:
    try:
        data = json.loads((root / NESTED_INVENTORY).read_text(encoding="utf-8"))
        paths = [entry.get("path") for entry in data["entries"] if isinstance(entry, dict)]
    except (OSError, KeyError, TypeError, json.JSONDecodeError):
        return [], ["missing-delegate"]
    errors = []
    actual = sorted(path.relative_to(root).as_posix() for path in (root / NESTED_ROOT).rglob("*") if path.is_file() and path.relative_to(root).as_posix() != NESTED_INVENTORY)
    if len(paths) != len(set(paths)) or set(paths) != set(actual): errors.append("delegated-coverage")
    return [path for path in paths if isinstance(path, str)], errors


def run_delegate_checker(root: Path) -> list[str]:
    checker = Path(__file__).resolve().parent / "doc-evidence-inventory-check.py"
    if not checker.is_file(): return ["delegate-check-failed: checker missing"]
    result = subprocess.run(["python3", str(checker), "--repo-root", str(root)], text=True, capture_output=True)
    return [] if result.returncode == 0 else ["delegate-check-failed"]


def check_semantic_overlay(root: Path) -> list[str]:
    try: data = json.loads((root / SEMANTIC_OVERLAY).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError): return ["semantic-overlay-load"]
    entries = data.get("entries"); bundles = data.get("bundles", [])
    if data.get("version") != 1 or not isinstance(entries, list) or not isinstance(bundles, list): return ["semantic-overlay-schema"]
    errors: list[str] = []
    paths = [entry.get("path") for entry in entries if isinstance(entry, dict)]
    if len(paths) != len(set(paths)): errors.append("semantic-overlay-duplicate-path")
    for entry in entries:
        if not isinstance(entry, dict): errors.append("semantic-overlay-schema"); continue
        path = entry.get("path"); authority = entry.get("current_authority")
        if entry.get("disposition") not in REVIEWED_DISPOSITIONS: errors.append("semantic-overlay-disposition")
        if entry.get("disposition") == "retain_outstanding_fulfillment_obligation" and not isinstance(entry.get("retirement_gate"), str): errors.append("semantic-overlay-obligation-gate")
        if entry.get("decision_owner") not in SEMANTIC_OWNERS: errors.append("semantic-overlay-owner")
        if not isinstance(path, str) or not (root / path).is_file(): errors.append("semantic-overlay-path"); continue
        if entry.get("content_sha256") != digest((root / path).read_bytes()): errors.append("semantic-overlay-content-drift")
        if not isinstance(authority, str) or not (root / authority).is_file(): errors.append("semantic-overlay-authority")
    bundle_paths: list[str] = []
    for bundle in bundles:
        if not isinstance(bundle, dict) or not isinstance(bundle.get("paths"), list): errors.append("semantic-overlay-bundle-schema"); continue
        members = bundle["paths"]
        if members != sorted(members) or len(members) != len(set(members)): errors.append("semantic-overlay-bundle-paths")
        bundle_paths.extend(path for path in members if isinstance(path, str))
        if bundle.get("disposition") not in REVIEWED_DISPOSITIONS: errors.append("semantic-overlay-disposition")
        if bundle.get("decision_owner") not in SEMANTIC_OWNERS: errors.append("semantic-overlay-owner")
        authority = bundle.get("current_authority")
        if not isinstance(authority, str) or not (root / authority).is_file(): errors.append("semantic-overlay-authority")
        if any(not isinstance(path, str) or not (root / path).is_file() for path in members): errors.append("semantic-overlay-path"); continue
        material = b"".join(path.encode() + b"\0" + digest((root / path).read_bytes()).encode() + b"\n" for path in members)
        if bundle.get("content_set_sha256") != digest(material): errors.append("semantic-overlay-bundle-content-drift")
    if len(paths + bundle_paths) != len(set(paths + bundle_paths)): errors.append("semantic-overlay-duplicate-path")
    return errors


def check(root: Path) -> list[str]:
    try: committed = json.loads((root / INVENTORY).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error: return [f"inventory-load: {error}"]
    errors: list[str] = check_semantic_overlay(root)
    if committed.get("version") != 2: errors.append("schema-version")
    objects = committed.get("objects")
    if not isinstance(objects, list): return errors + ["objects: expected list"]
    direct_paths = [entry.get("path") for entry in objects if isinstance(entry, dict)]
    if len(direct_paths) != len(set(direct_paths)): errors.append("duplicate-path")
    try: expected = generated(root)
    except (OSError, KeyError, TypeError, json.JSONDecodeError): return errors + ["missing-delegate"]
    if committed.get("product_modules") != list(PRODUCT_MODULES): errors.append("inventory-product-four-module-boundary")
    actual_product = sorted(path.name for path in (root / "doc/product").iterdir() if path.is_dir())
    if actual_product != list(PRODUCT_MODULES): errors.append("product-four-module-boundary")
    delegated_paths, delegate_errors = nested_paths(root)
    errors.extend(delegate_errors + run_delegate_checker(root))
    if set(direct_paths) & set(delegated_paths): errors.append("delegate-overlap")
    coverage = set(path for path in direct_paths if isinstance(path, str)) | set(delegated_paths) | set(control["path"] for control in committed.get("controls", []) if isinstance(control, dict))
    actual = {path.relative_to(root).as_posix() for path in (root / "doc").rglob("*") if path.is_file()}
    if coverage != actual: errors.append("full-corpus-coverage")
    if committed.get("delegates") != expected["delegates"]: errors.append("delegate-digest-drift")
    if committed.get("controls") != expected["controls"]: errors.append("control-drift")
    if objects != expected["objects"]:
        expected_paths = {entry["path"] for entry in expected["objects"]}
        current_paths = set(path for path in direct_paths if isinstance(path, str))
        if expected_paths - current_paths: errors.append("new-path-drift")
        if current_paths - expected_paths: errors.append("missing-path-drift")
        if any(isinstance(entry, dict) and entry.get("structural_owner") != next((item["structural_owner"] for item in expected["objects"] if item["path"] == entry.get("path")), None) for entry in objects): errors.append("registry-structural-owner-drift")
        errors.append("object-digest-or-routing-drift")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(); parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parent.parent); parser.add_argument("--print-generated", action="store_true")
    args = parser.parse_args(); root = args.repo_root.resolve()
    if args.print_generated:
        print(json.dumps(generated(root), ensure_ascii=False, indent=2) + "\n"); return 0
    errors = check(root)
    print("document-corpus-inventory-check: " + ("OK" if not errors else "FAIL\n" + "\n".join(errors)))
    return 0 if not errors else 1


if __name__ == "__main__": raise SystemExit(main())
