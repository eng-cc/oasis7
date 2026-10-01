#!/usr/bin/env python3
"""Create and validate immutable evidence for plan-owned review preflight returns."""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
import re
import stat
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any
from urllib.parse import quote


HANDOFF_SCHEMA_V1 = "oasis7-review-return-handoff/v1"
HANDOFF_SCHEMA_V2 = "oasis7-review-return-handoff/v2"
HANDOFF_SCHEMA = HANDOFF_SCHEMA_V2
DISPATCH_SCHEMA = "oasis7-review-dispatch/v1"
DISPATCH_MARKER = "<!-- oasis7-review-dispatch/v1 -->"
PM_TASK_MARKER = "<!-- oasis7-pm-task -->"
REPOSITORY = "eng-cc/oasis7"
TASK_RE = re.compile(r"task_[0-9a-f]{32}\Z")
HEAD_RE = re.compile(r"[0-9a-f]{40,64}\Z")
SHA_RE = re.compile(r"[0-9a-f]{64}\Z")
UUID_RE = re.compile(r"[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\Z", re.I)
CANONICAL_ROLES = {
    "producer_system_designer", "gameplay_designer", "game_visual_interaction_designer",
    "runtime_engineer", "blockchain_ops_engineer", "wasm_platform_engineer",
    "agent_engineer", "viewer_engineer", "qa_engineer",
    "repository_health_engineer", "liveops_community",
}
PLAN_KEYS = {
    "schema", "epoch", "task_uid", "frozen_head", "relevant_evidence_digest",
    "source_review_identity", "source_review_digest", "professional_review_applicability",
    "impact_projection", "impact_projection_schema", "impact_projection_digest",
    "impact_projection_test_profile", "impact_projection_declared_tests",
    "impact_projection_planner_digest", "integration_ci_status", "integration_ci_identity",
    "integration_ci_digest", "integration_ci_provenance", "integration_base_oid",
    "source_scope_oid", "comparison_ref", "comparison_oid", "roles", "expected_slices",
    "effective_mode", "batch_path", "collection_path", "packet_refs", "reused", "preflight",
    "loop_binding", "ci_validation_mode", "ci_ready_receipt_digest",
    "incremental_review_context",
}
HANDOFF_FIELDS_V1 = {
    "schema", "repository", "task_uid", "pr_number", "comparison_ref", "comparison_oid",
    "frozen_head", "source_review_identity", "source_review_digest", "epoch", "plan_path",
    "plan_sha256", "batch_path", "batch_sha256", "preflight_ledger_path",
    "preflight_ledger_sha256", "rows", "handoff_digest",
}
HANDOFF_FIELDS_V2 = HANDOFF_FIELDS_V1 | {"dispatch_evidence"}
HANDOFF_ROW_FIELDS_V1 = {"role", "slice_id", "artifact_path", "return_sha256", "findings_digest"}
HANDOFF_ROW_FIELDS_V2 = HANDOFF_ROW_FIELDS_V1 | {"packet_path", "packet_digest"}
DISPATCH_FIELDS = {
    "schema", "repository", "task_uid", "issue_number", "pr_number", "frozen_head", "epoch",
    "plan_path", "plan_sha256", "batch_path", "batch_sha256", "rows",
}
DISPATCH_ROW_FIELDS = {"role", "slice_id", "packet_path", "packet_digest"}
DISPATCH_EVIDENCE_FIELDS = {"issue_number", "issue_url", "comment_id", "author", "body_digest"}
SOURCE_IDENTITY_FIELDS = {
    "task_uid", "bootstrap_epoch", "repository", "pr_number", "source_head_oid",
    "source_scope_oid", "changed_paths_digest", "ordered_role_ids", "role_contract_digest",
    "review_policy_digest", "input_contract_digest",
}
PREFLIGHT_ROW_FIELDS = {
    "role", "slice_id", "task_uid", "head", "epoch", "status", "artifact_digest", "artifacts",
}
COMPLETED_ROW_FIELDS = {
    "role", "slice_id", "task_uid", "head", "epoch", "status", "activation",
    "context_delivery", "actual_runtime", "scope_verdict", "risk_verdict", "findings",
    "residual_risk", "artifact_digest", "artifacts",
}
PACKET_ACTIVATIONS = {
    "message_assigned_adapter_inactive": "message-assigned",
    "named_role_adapter_backed": "adapter-backed",
}
PACKET_CONTEXT_DELIVERY = {
    "minimal_head_bound_task_packet": "minimal-task-packet",
    "full_history_escalation": "full-history",
}


class ContractError(ValueError):
    """An immutable review-handoff contract violation."""


def canonical_bytes(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_digest(value: object) -> str:
    return sha256_bytes(canonical_bytes(value))


def _reject_duplicate_pairs(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            raise ContractError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def parse_json(raw: bytes, label: str) -> object:
    try:
        return json.loads(raw.decode("utf-8"), object_pairs_hook=_reject_duplicate_pairs)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ContractError(f"{label} is not valid UTF-8 JSON: {exc}") from exc


def read_json(path: Path, label: str) -> tuple[object, bytes]:
    try:
        raw = path.read_bytes()
    except OSError as exc:
        raise ContractError(f"cannot read {label}: {path}: {exc}") from exc
    return parse_json(raw, label), raw


def require_string(value: object, label: str, pattern: re.Pattern[str] | None = None) -> str:
    if not isinstance(value, str) or not value.strip() or (pattern is not None and pattern.fullmatch(value) is None):
        raise ContractError(f"{label} is invalid")
    return value


def resolve_repo_file(root: Path, raw_path: object, label: str) -> Path:
    if not isinstance(raw_path, str) or not raw_path.strip():
        raise ContractError(f"{label} path is invalid")
    value = Path(raw_path)
    candidate = value if value.is_absolute() else root / value
    try:
        resolved = candidate.resolve(strict=True)
        resolved.relative_to(root.resolve(strict=True))
    except (OSError, ValueError) as exc:
        raise ContractError(f"{label} path is missing or escapes the repository: {raw_path}") from exc
    if not resolved.is_file():
        raise ContractError(f"{label} is not a regular file: {raw_path}")
    return resolved


def resolve_repo_destination(root: Path, raw_path: object, label: str) -> Path:
    if not isinstance(raw_path, str) or not raw_path.strip():
        raise ContractError(f"{label} path is invalid")
    value = Path(raw_path)
    candidate = value if value.is_absolute() else root / value
    try:
        resolved = candidate.resolve(strict=False)
        resolved.relative_to(root.resolve(strict=True))
    except (OSError, ValueError) as exc:
        raise ContractError(f"{label} path escapes the repository: {raw_path}") from exc
    return resolved


def gh_json(arguments: list[str], label: str) -> object:
    try:
        result = subprocess.run(
            ["gh", "api", *arguments], text=True, capture_output=True, check=False,
        )
    except OSError as exc:
        raise ContractError(f"cannot read live {label}: {exc}") from exc
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or f"gh exited {result.returncode}"
        raise ContractError(f"cannot read live {label}: {detail}")
    return parse_json(result.stdout.encode("utf-8"), f"live {label}")


def canonical_task_issue_number(root: Path, task_uid: str) -> int:
    mapping_path = root / ".pm" / "github-project-sync" / "tasks.json"
    mapping, _ = read_json(mapping_path, "canonical task mapping")
    if not isinstance(mapping, dict):
        raise ContractError("canonical task mapping is not an object")
    project = mapping.get("project")
    if isinstance(project, dict) and project.get("repo") not in (None, REPOSITORY):
        raise ContractError("canonical task mapping repository is not eng-cc/oasis7")
    tasks = mapping.get("tasks")
    record = tasks.get(task_uid) if isinstance(tasks, dict) else None
    issue_number = record.get("issue_number") if isinstance(record, dict) else None
    if not isinstance(issue_number, int) or isinstance(issue_number, bool) or issue_number <= 0:
        raise ContractError(f"canonical task Issue mapping is invalid for {task_uid}")
    return issue_number


def canonical_issue_api_url(issue_number: int) -> str:
    return f"https://api.github.com/repos/{REPOSITORY}/issues/{issue_number}"


def validate_live_task_issue(issue_number: int, task_uid: str, pr_number: int) -> dict[str, object]:
    issue_value = gh_json([f"repos/{REPOSITORY}/issues/{issue_number}"], "canonical task Issue")
    live_issue_number = issue_value.get("number") if isinstance(issue_value, dict) else None
    if (not isinstance(live_issue_number, int) or isinstance(live_issue_number, bool)
            or live_issue_number != issue_number):
        raise ContractError("live canonical task Issue number mismatch")
    if issue_value.get("html_url") != f"https://github.com/{REPOSITORY}/issues/{issue_number}":
        raise ContractError("live canonical task Issue URL mismatch")
    body = issue_value.get("body")
    if not isinstance(body, str):
        raise ContractError("live canonical task Issue body is missing")
    body = body.replace("\r\n", "\n")
    if body.count(PM_TASK_MARKER) != 1:
        raise ContractError("live canonical task Issue marker is missing or ambiguous")
    uid_lines = re.findall(r"^[ \t]*(?:[-*][ \t]*)?task_uid\b.*$", body, re.MULTILINE)
    if uid_lines != [f"task_uid: {task_uid}"]:
        raise ContractError("live canonical task Issue UID mismatch or ambiguity")
    expected_pr_url = f"https://github.com/{REPOSITORY}/pull/{pr_number}"
    pr_url_lines = re.findall(r"^[ \t]*-[ \t]*pr_url\b.*$", body, re.MULTILINE)
    pr_number_lines = re.findall(r"^[ \t]*-[ \t]*pr_number\b.*$", body, re.MULTILINE)
    if (pr_url_lines != [f"- pr_url: `{expected_pr_url}`"]
            or pr_number_lines != [f"- pr_number: `{pr_number}`"]):
        raise ContractError("live canonical task Issue reciprocal PR binding is missing or ambiguous")
    return issue_value


def flatten_comment_pages(value: object) -> list[dict[str, object]]:
    if not isinstance(value, list):
        raise ContractError("paginated dispatch comment response is malformed")
    comments: list[dict[str, object]] = []
    for page in value:
        if not isinstance(page, list):
            raise ContractError("paginated dispatch comment page is malformed")
        for comment in page:
            if not isinstance(comment, dict) or not isinstance(comment.get("body"), str):
                raise ContractError("paginated dispatch comment entry is malformed")
            comments.append(comment)
    return comments


def dispatch_body(payload: dict[str, object]) -> bytes:
    return f"{DISPATCH_MARKER}\n```json\n".encode("utf-8") + canonical_bytes(payload) + b"\n```"


def parse_dispatch_body(body: str) -> dict[str, object]:
    prefix = f"{DISPATCH_MARKER}\n```json\n"
    suffix = "\n```"
    if not body.startswith(prefix) or not body.endswith(suffix) or body.count(DISPATCH_MARKER) != 1:
        raise ContractError("review dispatch comment framing is invalid")
    payload_raw = body[len(prefix):-len(suffix)].encode("utf-8")
    payload_value = parse_json(payload_raw, "review dispatch payload")
    if not isinstance(payload_value, dict) or set(payload_value) != DISPATCH_FIELDS:
        raise ContractError("review dispatch payload fields are invalid")
    if payload_raw != canonical_bytes(payload_value):
        raise ContractError("review dispatch payload is not canonical JSON")
    return payload_value


def matching_dispatch_comments(comments: list[dict[str, object]], expected: dict[str, object]) -> list[dict[str, object]]:
    matching: list[dict[str, object]] = []
    identity_fields = ("repository", "task_uid", "pr_number", "frozen_head", "epoch")
    for comment in comments:
        body = comment.get("body")
        assert isinstance(body, str)
        if DISPATCH_MARKER not in body:
            continue
        payload = parse_dispatch_body(body)
        if all(payload.get(key) == expected[key] for key in identity_fields):
            matching.append(comment)
    return matching


def repo_relative(root: Path, path: Path, label: str) -> str:
    try:
        return path.resolve(strict=True).relative_to(root.resolve(strict=True)).as_posix()
    except (OSError, ValueError) as exc:
        raise ContractError(f"{label} path escapes the repository: {path}") from exc


def parse_ledger(raw: bytes, label: str) -> list[dict[str, object]]:
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise ContractError(f"{label} is not UTF-8") from exc
    rows: list[dict[str, object]] = []
    for line_number, line in enumerate(text.splitlines(), 1):
        if not line.strip():
            continue
        item = parse_json(line.encode("utf-8"), f"{label} line {line_number}")
        if not isinstance(item, dict):
            raise ContractError(f"{label} line {line_number} is not an object")
        rows.append(item)
    return rows


def validate_batch(batch_value: object, batch_path: Path, task_uid: str, head: str,
                   source_digest: str, epoch: str) -> list[dict[str, str]]:
    if not isinstance(batch_value, dict) or set(batch_value) != {
        "schema", "epoch", "task_uid", "frozen_head", "relevant_evidence_digest", "expected_slices",
    }:
        raise ContractError("batch closed schema is invalid")
    batch = batch_value
    if (batch.get("schema") != "oasis7-review-batch/v1" or batch.get("task_uid") != task_uid
            or batch.get("frozen_head") != head or batch.get("relevant_evidence_digest") != source_digest
            or batch.get("epoch") != epoch):
        raise ContractError("batch identity does not match the immutable plan")
    slices = batch.get("expected_slices")
    if not isinstance(slices, list) or not slices:
        raise ContractError("batch expected slices are invalid")
    if any(not isinstance(item, dict) or set(item) != {"role", "slice_id"} for item in slices):
        raise ContractError("batch expected slice fields are invalid")
    normalized: list[dict[str, str]] = []
    seen_roles: set[str] = set()
    seen_ids: set[str] = set()
    for item in slices:
        role = require_string(item.get("role"), "batch role")
        slice_id = require_string(item.get("slice_id"), "batch slice id", UUID_RE)
        if role not in CANONICAL_ROLES or role in seen_roles or slice_id in seen_ids:
            raise ContractError("batch contains a noncanonical or duplicate role/slice")
        seen_roles.add(role)
        seen_ids.add(slice_id)
        normalized.append({"role": role, "slice_id": slice_id})
    if normalized != sorted(normalized, key=lambda item: (item["role"].encode(), item["slice_id"].encode())):
        raise ContractError("batch expected slices are not deterministically sorted")
    identity = {key: batch[key] for key in ("task_uid", "frozen_head", "relevant_evidence_digest", "expected_slices")}
    if canonical_digest(identity) != epoch:
        raise ContractError("batch epoch does not match its immutable identity")
    return normalized


def validate_source_identity(plan: dict[str, object], task_uid: str, head: str) -> tuple[dict[str, object], str]:
    identity_value = plan.get("source_review_identity")
    if not isinstance(identity_value, dict) or set(identity_value) != SOURCE_IDENTITY_FIELDS:
        raise ContractError("plan source-review identity fields are invalid")
    identity = identity_value
    if (identity.get("task_uid") != task_uid or identity.get("repository") != REPOSITORY
            or identity.get("source_head_oid") != head):
        raise ContractError("plan source-review identity does not match task, repository, or head")
    bootstrap = identity.get("bootstrap_epoch")
    pr_number = identity.get("pr_number")
    if not isinstance(bootstrap, int) or isinstance(bootstrap, bool) or bootstrap <= 0:
        raise ContractError("plan bootstrap epoch is invalid")
    if not isinstance(pr_number, int) or isinstance(pr_number, bool) or pr_number <= 0:
        raise ContractError("plan PR number is invalid")
    require_string(identity.get("source_scope_oid"), "source scope OID", HEAD_RE)
    roles = identity.get("ordered_role_ids")
    if (not isinstance(roles, list) or not roles or any(not isinstance(role, str) for role in roles)
            or any(role not in CANONICAL_ROLES for role in roles) or len(roles) != len(set(roles))):
        raise ContractError("source-review ordered role IDs are invalid")
    for key in ("changed_paths_digest", "role_contract_digest", "review_policy_digest", "input_contract_digest"):
        require_string(identity.get(key), f"source-review {key}", SHA_RE)
    source_digest = require_string(plan.get("source_review_digest"), "source-review digest", SHA_RE)
    if canonical_digest(identity) != source_digest:
        raise ContractError("source-review identity digest mismatch")
    if plan.get("relevant_evidence_digest") != source_digest:
        raise ContractError("plan evidence digest does not match source-review identity")
    if plan.get("source_scope_oid") != identity.get("source_scope_oid"):
        raise ContractError("plan source-scope identity mismatch")
    return identity, source_digest


def validate_plan(plan_value: object, raw: bytes) -> tuple[dict[str, object], dict[str, object], str]:
    if not isinstance(plan_value, dict):
        raise ContractError("review plan must be an object")
    plan = plan_value
    unknown = set(plan) - PLAN_KEYS
    if unknown:
        raise ContractError(f"review plan contains unknown fields: {sorted(unknown)}")
    if plan.get("schema") != "oasis7-review-plan/v2":
        raise ContractError("review plan schema must be v2")
    task_uid = require_string(plan.get("task_uid"), "plan task UID", TASK_RE)
    head = require_string(plan.get("frozen_head"), "plan frozen head", HEAD_RE)
    epoch = require_string(plan.get("epoch"), "plan epoch", SHA_RE)
    comparison_ref = require_string(plan.get("comparison_ref"), "plan comparison ref")
    comparison_oid = require_string(plan.get("comparison_oid"), "plan comparison OID", HEAD_RE)
    source_identity, source_digest = validate_source_identity(plan, task_uid, head)
    if comparison_oid != source_identity.get("source_scope_oid"):
        raise ContractError("plan comparison OID does not match source-review scope OID")
    applicability = plan.get("professional_review_applicability")
    if not isinstance(applicability, dict) or set(applicability) != {"identity", "identity_digest", "verified"}:
        raise ContractError("plan professional review applicability is invalid")
    applicability_identity = applicability.get("identity")
    if not isinstance(applicability_identity, dict) or set(applicability_identity) != {
        "changed_paths_digest", "input_contract_digest", "ordered_role_ids", "role_contract_digest", "review_policy_digest",
    }:
        raise ContractError("plan applicability identity is invalid")
    expected_applicability = {key: source_identity[key] for key in applicability_identity}
    if (applicability_identity != expected_applicability or applicability.get("verified") is not True
            or applicability.get("identity_digest") != canonical_digest(applicability_identity)):
        raise ContractError("plan applicability identity digest or source binding mismatch")
    if "impact_projection" not in plan:
        raise ContractError("plan impact projection is required")
    projection = plan["impact_projection"]
    if not isinstance(projection, dict):
        raise ContractError("plan impact projection is invalid")
    for field in ("schema", "projection_digest", "test_profile", "declared_tests", "planner_digest"):
        if plan.get(f"impact_projection_{field.removeprefix('projection_')}") != projection.get(field):
            raise ContractError(f"plan impact projection {field} binding mismatch")
    return plan, source_identity, source_digest


def validate_plan_impact_projection(
    root: Path, plan: dict[str, object], source_identity: dict[str, object],
) -> dict[str, Any]:
    """Verify the embedded projection and bind it to this frozen review identity."""
    projection = plan.get("impact_projection")
    if not isinstance(projection, dict):
        raise ContractError("plan impact projection is invalid")
    code_root = Path(__file__).resolve().parents[2]
    try:
        module = _load_pm_module(code_root, "workflow_impact_projection", "workflow-impact-projection.py")
        verified = module.validate_projection_value(
            projection,
            expected={
                "task_uid": plan["task_uid"],
                "source_head_oid": plan["frozen_head"],
                "scope_base_oid": source_identity["source_scope_oid"],
                "changed_paths_digest": f"sha256:{source_identity['changed_paths_digest']}",
                "ordered_role_ids": source_identity["ordered_role_ids"],
                "test_profile": plan["impact_projection_test_profile"],
            },
            repo_root=root,
        )
    except (OSError, TypeError, ValueError) as exc:
        raise ContractError(f"plan impact projection verification failed: {exc}") from exc

    projected_paths_digest = str(verified["changed_paths_digest"]).removeprefix("sha256:")
    if projected_paths_digest != source_identity["changed_paths_digest"]:
        raise ContractError("plan impact projection changed-path digest does not match source-review identity")
    projected_input_digest = str(verified["projection_digest"]).removeprefix("sha256:")
    if projected_input_digest != source_identity["input_contract_digest"]:
        raise ContractError("plan impact projection digest does not match source-review input contract")
    return verified


def frozen_changed_paths(root: Path, comparison_oid: str, head: str) -> list[str]:
    """Require the plan's base to precede its frozen head, then derive review paths."""
    try:
        ancestry = subprocess.run(
            ["git", "-C", str(root), "merge-base", "--is-ancestor", comparison_oid, head],
            text=True, capture_output=True, check=False,
        )
    except (OSError, UnicodeError) as exc:
        raise ContractError(f"cannot validate comparison ancestry: {exc}") from exc
    if ancestry.returncode == 1:
        raise ContractError(
            "comparison OID is not an ancestor of frozen head: "
            f"comparison={comparison_oid}, head={head}"
        )
    if ancestry.returncode != 0:
        detail = ancestry.stderr.strip() or ancestry.stdout.strip() or f"git exited {ancestry.returncode}"
        raise ContractError(f"cannot validate comparison ancestry: {detail}")

    try:
        result = subprocess.run(
            ["git", "-C", str(root), "diff", "--name-only", "--no-renames", comparison_oid, head],
            text=True, capture_output=True, check=False,
        )
    except (OSError, UnicodeError) as exc:
        raise ContractError(f"cannot derive frozen base..head changed paths: {exc}") from exc
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or f"git exited {result.returncode}"
        raise ContractError(f"cannot derive frozen base..head changed paths: {detail}")
    paths = [line for line in result.stdout.splitlines() if line]
    if len(paths) != len(set(paths)):
        raise ContractError("frozen base..head changed path set contains duplicates")
    return sorted(paths)


def validate_packet_refs(root: Path, plan: dict[str, object], expected_slices: list[dict[str, str]],
                         source_identity: dict[str, object]) -> dict[tuple[str, str], dict[str, str]]:
    """Validate plan-owned packet bindings and return the metadata each return must echo."""
    refs = plan.get("packet_refs")
    if not isinstance(refs, list) or len(refs) != len(expected_slices):
        raise ContractError("plan packet refs do not cover every expected role/slice")

    expected = {(item["role"], item["slice_id"]) for item in expected_slices}
    metadata: dict[tuple[str, str], dict[str, str]] = {}
    packet_paths: set[Path] = set()
    for ref in refs:
        if not isinstance(ref, dict) or set(ref) != {"role", "slice_id", "packet_ref"}:
            raise ContractError("plan packet ref fields are invalid")
        role = require_string(ref.get("role"), "plan packet role")
        slice_id = require_string(ref.get("slice_id"), "plan packet slice ID", UUID_RE)
        identity = (role, slice_id)
        if identity not in expected or identity in metadata:
            raise ContractError("plan packet refs contain duplicate or unexpected role/slice")
        packet_ref = ref.get("packet_ref")
        expected_ref = f".pm/scratch/{plan['task_uid']}/slice-packets/{slice_id}.json"
        if packet_ref != expected_ref:
            raise ContractError(f"plan packet ref is not canonical for {role}")
        packet_path = resolve_repo_file(root, packet_ref, "slice packet")
        if packet_path in packet_paths:
            raise ContractError("plan packet refs contain a duplicate packet path")
        packet_paths.add(packet_path)
        packet, _ = read_json(packet_path, f"slice packet for {role}")
        if not isinstance(packet, dict) or packet.get("schema") != "oasis7-subagent-task-packet/v1":
            raise ContractError(f"slice packet schema is invalid for {role}")
        packet_digest = require_string(packet.get("packet_digest"), f"slice packet digest for {role}", SHA_RE)
        unsigned_packet = {key: value for key, value in packet.items() if key != "packet_digest"}
        if canonical_digest(unsigned_packet) != packet_digest:
            raise ContractError(f"slice packet digest mismatch for {role}")

        packet_identity = packet.get("identity")
        if not isinstance(packet_identity, dict):
            raise ContractError(f"slice packet identity is invalid for {role}")
        for field, wanted in (
            ("task_uid", plan["task_uid"]), ("head", plan["frozen_head"]),
            ("base_sha", source_identity["source_scope_oid"]),
        ):
            if packet_identity.get(field) != wanted:
                raise ContractError(f"slice packet {field} mismatch for {role}")

        packet_slice = packet.get("slice")
        if not isinstance(packet_slice, dict):
            raise ContractError(f"slice packet metadata is invalid for {role}")
        for field, wanted in (("role", role), ("slice_id", slice_id)):
            if packet_slice.get(field) != wanted:
                raise ContractError(f"slice packet {field} mismatch for {role}")
        activation_mode = packet_slice.get("role_activation")
        context_mode = packet_slice.get("context_delivery_mode")
        if (not isinstance(activation_mode, str) or activation_mode not in PACKET_ACTIVATIONS
                or not isinstance(context_mode, str) or context_mode not in PACKET_CONTEXT_DELIVERY):
            raise ContractError(f"slice packet activation/context mode is invalid for {role}")
        if context_mode == "full_history_escalation":
            require_string(packet_slice.get("full_history_escalation_reason"),
                           f"slice packet full-history escalation reason for {role}")
        model_reasoning = require_string(packet_slice.get("actual_dispatched_model_reasoning"),
                                         f"slice packet actual dispatched model/reasoning for {role}")
        runtime_reason = require_string(packet_slice.get("actual_runtime_evidence_reason"),
                                        f"slice packet runtime evidence reason for {role}")
        metadata[identity] = {
            "activation": PACKET_ACTIVATIONS[activation_mode],
            "context_delivery": PACKET_CONTEXT_DELIVERY[context_mode],
            "actual_runtime": f"{model_reasoning}: {runtime_reason}",
            "packet_path": repo_relative(root, packet_path, "slice packet"),
            "packet_digest": packet_digest,
        }
    if set(metadata) != expected:
        raise ContractError("plan packet refs do not cover every expected role/slice")
    return metadata


def validate_return(return_value: object, *, role: str, slice_id: str, task_uid: str,
                    head: str, epoch: str,
                    packet_metadata: dict[str, str] | None = None) -> list[object]:
    if not isinstance(return_value, dict):
        raise ContractError(f"review return is not an object for role {role}")
    identity = {"task_uid": task_uid, "role": role, "slice_id": slice_id,
                "head": head, "epoch": epoch, "status": "completed"}
    for key, expected in identity.items():
        if return_value.get(key) != expected:
            raise ContractError(f"review return {key} mismatch for role {role}")
    for key in ("activation", "context_delivery", "actual_runtime", "scope_verdict", "risk_verdict"):
        require_string(return_value.get(key), f"review return {key} for role {role}")
    if packet_metadata is not None:
        for key in ("activation", "context_delivery", "actual_runtime"):
            if return_value.get(key) != packet_metadata[key]:
                raise ContractError(f"review return {key} conflicts with packet metadata for role {role}")
        if "packet_digest" in packet_metadata:
            admitted_digest = require_string(
                return_value.get("admitted_packet_digest"),
                f"review return admitted packet digest for role {role}", SHA_RE,
            )
            if admitted_digest != packet_metadata["packet_digest"]:
                raise ContractError(f"review return admitted packet digest conflicts with dispatch packet for role {role}")
    disposition = return_value.get("disposition")
    findings = return_value.get("findings")
    if disposition not in {"findings", "no_findings"} or not isinstance(findings, list):
        raise ContractError(f"review return disposition/findings are invalid for role {role}")
    if (disposition == "findings" and not findings) or (disposition == "no_findings" and findings):
        raise ContractError(f"review return findings do not match disposition for role {role}")
    residual = require_string(return_value.get("residual_risk"), f"review return residual risk for role {role}")
    del residual
    for finding in findings:
        if not isinstance(finding, dict):
            raise ContractError(f"review finding is not an object for role {role}")
        triage = finding.get("triage")
        if (not isinstance(triage, dict) or set(triage) != {"classification", "basis"}
                or triage.get("classification") not in {"blocking", "nonblocking"}
                or not isinstance(triage.get("basis"), str) or not triage["basis"].strip()):
            raise ContractError(f"review finding triage is missing or invalid for role {role}")
    return findings


def validate_plan_inputs(
    root: Path, plan_path: Path, *, allow_promoted_ledger: bool = False,
) -> tuple[dict[str, object], dict[str, object], Path, Path, Path, list[dict[str, str]], bytes, bytes, bytes]:
    root = root.resolve(strict=True)
    resolved_plan = resolve_repo_file(root, str(plan_path), "review plan")
    plan_value, plan_raw = read_json(resolved_plan, "review plan")
    plan, source_identity, source_digest = validate_plan(plan_value, plan_raw)
    projection = validate_plan_impact_projection(root, plan, source_identity)
    derived_paths = frozen_changed_paths(
        root, str(source_identity["source_scope_oid"]), str(plan["frozen_head"]),
    )
    if projection["changed_paths"] != derived_paths:
        projected_paths = projection["changed_paths"]
        missing = sorted(set(derived_paths) - set(projected_paths))
        extra = sorted(set(projected_paths) - set(derived_paths))
        details = []
        if missing:
            details.append("missing=" + ",".join(missing))
        if extra:
            details.append("extra=" + ",".join(extra))
        raise ContractError(
            "impact projection changed paths do not match frozen base..head: "
            + ("; ".join(details) if details else "ordering differs")
        )
    batch_path = resolve_repo_file(root, plan.get("batch_path"), "review batch")
    if plan.get("collection_path") is not None:
        collection_path_value = resolve_repo_destination(root, plan.get("collection_path"), "collection receipt")
        expected_collection_path = batch_path.with_name(f"{batch_path.stem}.collection.json").resolve(strict=False)
        if collection_path_value != expected_collection_path:
            raise ContractError("plan collection path does not match its batch")
    batch_value, batch_raw = read_json(batch_path, "review batch")
    batch_slices = validate_batch(
        batch_value, batch_path, str(plan["task_uid"]), str(plan["frozen_head"]), source_digest, str(plan["epoch"])
    )
    validate_packet_refs(root, plan, batch_slices, source_identity)
    plan_slices = plan.get("expected_slices")
    if (plan.get("epoch") != batch_value.get("epoch") or not isinstance(plan_slices, list)
            or any(not isinstance(item, dict) or set(item) != {"role", "slice_id"} for item in plan_slices)
            or len(plan_slices) != len(batch_slices)
            or sorted(plan_slices, key=lambda item: (item["role"], item["slice_id"])) != batch_slices):
        raise ContractError("plan expected slices or epoch do not match immutable batch")
    roles = plan.get("roles")
    source_roles = source_identity.get("ordered_role_ids")
    expected_roles = [item["role"] for item in batch_slices]
    plan_slice_roles = [item["role"] for item in plan_slices]
    if (not isinstance(roles, list) or roles != source_roles or plan_slice_roles != roles
            or set(roles) != set(expected_roles) or len(roles) != len(expected_roles)):
        raise ContractError("plan role set does not match source identity and batch")
    preflight = plan.get("preflight")
    if not isinstance(preflight, dict) or preflight.get("status") != "incomplete":
        raise ContractError("plan has no incomplete preflight")
    allowed_preflight = {"status", "epoch", "ledger_path", "artifact_paths", "reused"}
    if set(preflight) - allowed_preflight:
        raise ContractError("plan preflight contains unknown fields")
    if "epoch" in preflight and preflight["epoch"] != plan["epoch"]:
        raise ContractError("plan preflight epoch mismatch")
    if "reused" in preflight and type(preflight["reused"]) is not bool:
        raise ContractError("plan preflight reused flag is invalid")
    ledger_path = resolve_repo_file(root, preflight.get("ledger_path"), "preflight ledger")
    ledger_raw = ledger_path.read_bytes()
    rows = parse_ledger(ledger_raw, "preflight ledger")
    expected = {(item["role"], item["slice_id"]) for item in batch_slices}
    if len(rows) != len(expected) or len(rows) == 0:
        raise ContractError("preflight ledger role/slice coverage is incomplete")
    statuses = {row.get("status") for row in rows}
    if statuses == {"incomplete"}:
        promoted = False
    elif statuses == {"completed"} and allow_promoted_ledger:
        promoted = True
    else:
        raise ContractError("preflight ledger status is not an authorized incomplete or promoted state")
    seen: set[tuple[str, str]] = set()
    artifact_paths: dict[tuple[str, str], Path] = {}
    ordered_identities: list[tuple[str, str]] = []
    for row in rows:
        required_fields = COMPLETED_ROW_FIELDS if promoted else PREFLIGHT_ROW_FIELDS
        if set(row) != required_fields:
            raise ContractError("completed ledger row fields are invalid" if promoted else "preflight ledger row fields are invalid")
        role = require_string(row.get("role"), "preflight ledger role")
        slice_id = require_string(row.get("slice_id"), "preflight ledger slice id", UUID_RE)
        identity = (role, slice_id)
        if identity in seen or identity not in expected:
            raise ContractError("preflight ledger has duplicate or unexpected role/slice")
        seen.add(identity)
        ordered_identities.append(identity)
        if any(row.get(key) != expected_value for key, expected_value in (
            ("task_uid", plan["task_uid"]), ("head", plan["frozen_head"]),
            ("epoch", plan["epoch"]), ("status", "completed" if promoted else "incomplete"),
        )):
            raise ContractError("completed ledger identity or status mismatch" if promoted else "preflight ledger identity or status mismatch")
        require_string(row.get("artifact_digest"), "completed return digest" if promoted else "preflight skeleton digest", SHA_RE)
        artifacts = row.get("artifacts")
        if not isinstance(artifacts, list) or len(artifacts) != 1:
            raise ContractError("completed ledger must bind exactly one artifact" if promoted else "preflight ledger must bind exactly one artifact")
        artifact_paths[identity] = resolve_repo_file(root, artifacts[0], "completed return" if promoted else "preflight return")
        if promoted:
            for field in ("activation", "context_delivery", "actual_runtime", "scope_verdict", "risk_verdict", "residual_risk"):
                require_string(row.get(field), f"completed ledger {field} for {role}")
            if row.get("findings") not in {"findings", "no_findings"}:
                raise ContractError(f"completed ledger findings disposition is invalid for {role}")
    if seen != expected:
        raise ContractError("preflight ledger role/slice coverage is incomplete")
    if ordered_identities != [(item["role"], item["slice_id"]) for item in batch_slices]:
        raise ContractError("preflight ledger rows are not in canonical batch order")
    if "artifact_paths" in preflight:
        listed = preflight["artifact_paths"]
        if not isinstance(listed, list) or len(listed) != len(batch_slices):
            raise ContractError("plan preflight artifact path list is invalid")
        resolved_listed = [resolve_repo_file(root, item, "plan preflight return") for item in listed]
        if set(resolved_listed) != set(artifact_paths.values()):
            raise ContractError("plan preflight artifact path list does not match ledger")
    collection_path = batch_path.with_name(f"{batch_path.stem}.collection.json")
    if not allow_promoted_ledger and (collection_path.exists() or collection_path.is_symlink()):
        raise ContractError("handoff cannot be created after a collection receipt exists")
    return plan, source_identity, resolved_plan, batch_path, ledger_path, batch_slices, plan_raw, batch_raw, ledger_raw


def dispatch_payload_for_plan(
    root: Path, plan_path: Path, *, allow_promoted_ledger: bool = False,
) -> dict[str, object]:
    root = root.resolve(strict=True)
    (plan, source_identity, resolved_plan, batch_path, _ledger_path, expected_slices,
     plan_raw, batch_raw, _ledger_raw) = validate_plan_inputs(
        root, plan_path, allow_promoted_ledger=allow_promoted_ledger,
    )
    packet_metadata = validate_packet_refs(root, plan, expected_slices, source_identity)
    task_uid = str(plan["task_uid"])
    issue_number = canonical_task_issue_number(root, task_uid)
    rows = [
        {
            "role": item["role"], "slice_id": item["slice_id"],
            "packet_path": packet_metadata[(item["role"], item["slice_id"])]["packet_path"],
            "packet_digest": packet_metadata[(item["role"], item["slice_id"])]["packet_digest"],
        }
        for item in expected_slices
    ]
    rows.sort(key=lambda item: (item["role"].encode(), item["slice_id"].encode()))
    return {
        "schema": DISPATCH_SCHEMA, "repository": REPOSITORY,
        "task_uid": task_uid, "issue_number": issue_number,
        "pr_number": source_identity["pr_number"], "frozen_head": plan["frozen_head"],
        "epoch": plan["epoch"], "plan_path": repo_relative(root, resolved_plan, "plan"),
        "plan_sha256": sha256_bytes(plan_raw), "batch_path": repo_relative(root, batch_path, "batch"),
        "batch_sha256": sha256_bytes(batch_raw), "rows": rows,
    }


def validate_dispatch_payload(payload: object) -> dict[str, object]:
    if not isinstance(payload, dict) or set(payload) != DISPATCH_FIELDS:
        raise ContractError("review dispatch payload fields are invalid")
    if payload.get("schema") != DISPATCH_SCHEMA or payload.get("repository") != REPOSITORY:
        raise ContractError("review dispatch schema or repository is invalid")
    require_string(payload.get("task_uid"), "dispatch task UID", TASK_RE)
    require_string(payload.get("frozen_head"), "dispatch frozen head", HEAD_RE)
    require_string(payload.get("epoch"), "dispatch epoch", SHA_RE)
    require_string(payload.get("plan_path"), "dispatch plan path")
    require_string(payload.get("batch_path"), "dispatch batch path")
    for key in ("plan_sha256", "batch_sha256"):
        require_string(payload.get(key), f"dispatch {key}", SHA_RE)
    for key in ("issue_number", "pr_number"):
        value = payload.get(key)
        if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
            raise ContractError(f"dispatch {key} is invalid")
    rows = payload.get("rows")
    if not isinstance(rows, list) or not rows:
        raise ContractError("dispatch rows are invalid")
    seen: set[tuple[str, str]] = set()
    for row in rows:
        if not isinstance(row, dict) or set(row) != DISPATCH_ROW_FIELDS:
            raise ContractError("dispatch row fields are invalid")
        role = require_string(row.get("role"), "dispatch role")
        slice_id = require_string(row.get("slice_id"), "dispatch slice ID", UUID_RE)
        require_string(row.get("packet_path"), "dispatch packet path")
        require_string(row.get("packet_digest"), "dispatch packet digest", SHA_RE)
        identity = (role, slice_id)
        if identity in seen:
            raise ContractError("dispatch contains a duplicate role/slice row")
        seen.add(identity)
    if rows != sorted(rows, key=lambda item: (item["role"].encode(), item["slice_id"].encode())):
        raise ContractError("dispatch rows are not deterministically sorted")
    return payload


def paginated_task_issue_comments(issue_number: int) -> list[dict[str, object]]:
    pages = gh_json([
        f"repos/{REPOSITORY}/issues/{issue_number}/comments?per_page=100", "--paginate", "--slurp",
    ], "paginated task Issue comments")
    return flatten_comment_pages(pages)


def live_dispatch_readback(root: Path, expected_payload: dict[str, object], comment_id: object) -> dict[str, object]:
    expected_payload = validate_dispatch_payload(expected_payload)
    issue_number = int(expected_payload["issue_number"])
    task_uid = str(expected_payload["task_uid"])
    pr_number = int(expected_payload["pr_number"])
    if not isinstance(comment_id, int) or isinstance(comment_id, bool) or comment_id <= 0:
        raise ContractError("dispatch comment ID is invalid")
    if canonical_task_issue_number(root, task_uid) != issue_number:
        raise ContractError("dispatch Issue number does not match canonical task mapping")
    validate_live_task_issue(issue_number, task_uid, pr_number)
    comments = paginated_task_issue_comments(issue_number)
    matches = matching_dispatch_comments(comments, expected_payload)
    if len(matches) != 1:
        raise ContractError("live task Issue does not contain exactly one matching dispatch marker")
    listed_comment = matches[0]
    listed_id = listed_comment.get("id")
    if not isinstance(listed_id, int) or isinstance(listed_id, bool) or listed_id != comment_id:
        raise ContractError("dispatch comment ID does not match the unique live marker")
    expected_body = dispatch_body(expected_payload)
    listed_body = listed_comment.get("body")
    if not isinstance(listed_body, str) or listed_body.encode("utf-8") != expected_body:
        raise ContractError("live dispatch comment body differs from the exact plan and packet payload")

    live_comment = gh_json(
        [f"repos/{REPOSITORY}/issues/comments/{comment_id}"], "live dispatch comment",
    )
    live_comment_id = live_comment.get("id") if isinstance(live_comment, dict) else None
    if (not isinstance(live_comment_id, int) or isinstance(live_comment_id, bool)
            or live_comment_id != comment_id):
        raise ContractError("live dispatch comment ID mismatch")
    if live_comment.get("issue_url") != canonical_issue_api_url(issue_number):
        raise ContractError("live dispatch comment is not attached to the canonical task Issue")
    body = live_comment.get("body")
    if not isinstance(body, str) or body.encode("utf-8") != expected_body:
        raise ContractError("live dispatch comment readback body mismatch")
    listed_user = listed_comment.get("user")
    live_user = live_comment.get("user")
    listed_author = listed_user.get("login") if isinstance(listed_user, dict) else None
    author = live_user.get("login") if isinstance(live_user, dict) else None
    author = require_string(author, "dispatch comment author", re.compile(r"[A-Za-z0-9](?:[A-Za-z0-9-]{0,37}[A-Za-z0-9])?\Z"))
    if listed_author != author:
        raise ContractError("paginated and direct dispatch comment author readbacks differ")
    permission = gh_json(
        [f"repos/{REPOSITORY}/collaborators/{quote(author, safe='')}/permission"],
        "dispatch author repository permission",
    )
    if not isinstance(permission, dict) or permission.get("permission") != "admin":
        raise ContractError("dispatch comment author is not a current repository admin")
    return {
        "issue_number": issue_number, "issue_url": canonical_issue_api_url(issue_number),
        "comment_id": comment_id, "author": author,
        "body_digest": sha256_bytes(body.encode("utf-8")),
        "payload": expected_payload, "body": body,
    }


def publish_dispatch(root: Path, plan_path: Path) -> dict[str, object]:
    root = root.resolve(strict=True)
    payload = validate_dispatch_payload(dispatch_payload_for_plan(root, plan_path))
    issue_number = int(payload["issue_number"])
    validate_live_task_issue(issue_number, str(payload["task_uid"]), int(payload["pr_number"]))
    comments = paginated_task_issue_comments(issue_number)
    matches = matching_dispatch_comments(comments, payload)
    if len(matches) > 1:
        raise ContractError("duplicate live task Issue dispatch markers make publication ambiguous")
    if matches:
        comment_id = matches[0].get("id")
    else:
        body = dispatch_body(payload).decode("utf-8")
        created = gh_json([
            f"repos/{REPOSITORY}/issues/{issue_number}/comments", "--method", "POST",
            "--field", f"body={body}",
        ], "dispatch comment publication")
        comment_id = created.get("id") if isinstance(created, dict) else None
    readback = live_dispatch_readback(root, payload, comment_id)
    return {
        "status": "published", "task_uid": payload["task_uid"],
        "issue_number": readback["issue_number"], "pr_number": payload["pr_number"],
        "head": payload["frozen_head"], "epoch": payload["epoch"],
        "dispatch_comment_id": readback["comment_id"], "author": readback["author"],
        "issue_url": readback["issue_url"], "body_digest": readback["body_digest"],
        "rows": payload["rows"],
    }


def create_handoff(root: Path, plan_path: Path, dispatch_comment_id: int) -> dict[str, object]:
    root = root.resolve(strict=True)
    (plan, source_identity, resolved_plan, batch_path, ledger_path, expected_slices,
     plan_raw, batch_raw, ledger_raw) = validate_plan_inputs(root, plan_path)
    packet_metadata = validate_packet_refs(root, plan, expected_slices, source_identity)
    expected_dispatch = dispatch_payload_for_plan(root, resolved_plan)
    snapshot_rows = [
        {
            "role": item["role"], "slice_id": item["slice_id"],
            "packet_path": packet_metadata[(item["role"], item["slice_id"])]["packet_path"],
            "packet_digest": packet_metadata[(item["role"], item["slice_id"])]["packet_digest"],
        }
        for item in expected_slices
    ]
    snapshot_rows.sort(key=lambda item: (item["role"].encode(), item["slice_id"].encode()))
    snapshot_dispatch = validate_dispatch_payload({
        "schema": DISPATCH_SCHEMA, "repository": REPOSITORY,
        "task_uid": plan["task_uid"],
        "issue_number": canonical_task_issue_number(root, str(plan["task_uid"])),
        "pr_number": source_identity["pr_number"], "frozen_head": plan["frozen_head"],
        "epoch": plan["epoch"], "plan_path": repo_relative(root, resolved_plan, "plan"),
        "plan_sha256": sha256_bytes(plan_raw),
        "batch_path": repo_relative(root, batch_path, "batch"),
        "batch_sha256": sha256_bytes(batch_raw), "rows": snapshot_rows,
    })
    if expected_dispatch != snapshot_dispatch:
        raise ContractError("plan, batch, or packet bytes changed while validating dispatch evidence")
    dispatch_readback = live_dispatch_readback(root, expected_dispatch, dispatch_comment_id)
    rows: list[dict[str, object]] = []
    preflight_rows = { (row["role"], row["slice_id"]): row for row in parse_ledger(ledger_raw, "preflight ledger") }
    for expected in expected_slices:
        identity = (expected["role"], expected["slice_id"])
        preflight_row = preflight_rows[identity]
        artifact_path = resolve_repo_file(root, preflight_row["artifacts"][0], "completed review return")
        return_value, return_raw = read_json(artifact_path, f"review return for {expected['role']}")
        findings = validate_return(
            return_value, role=expected["role"], slice_id=expected["slice_id"],
            task_uid=str(plan["task_uid"]), head=str(plan["frozen_head"]), epoch=str(plan["epoch"]),
            packet_metadata=packet_metadata[identity],
        )
        rows.append({
            "role": expected["role"], "slice_id": expected["slice_id"],
            "packet_path": packet_metadata[identity]["packet_path"],
            "packet_digest": packet_metadata[identity]["packet_digest"],
            "artifact_path": repo_relative(root, artifact_path, "review return"),
            "return_sha256": sha256_bytes(return_raw), "findings_digest": canonical_digest(findings),
        })
    rows.sort(key=lambda row: (str(row["role"]).encode(), str(row["slice_id"]).encode()))
    payload: dict[str, object] = {
        "schema": HANDOFF_SCHEMA, "repository": REPOSITORY,
        "task_uid": plan["task_uid"], "pr_number": source_identity["pr_number"],
        "comparison_ref": plan["comparison_ref"], "comparison_oid": plan["comparison_oid"],
        "frozen_head": plan["frozen_head"], "source_review_identity": source_identity,
        "source_review_digest": plan["source_review_digest"], "epoch": plan["epoch"],
        "plan_path": repo_relative(root, resolved_plan, "plan"), "plan_sha256": sha256_bytes(plan_raw),
        "batch_path": repo_relative(root, batch_path, "batch"), "batch_sha256": sha256_bytes(batch_raw),
        "preflight_ledger_path": repo_relative(root, ledger_path, "preflight ledger"),
        "preflight_ledger_sha256": sha256_bytes(ledger_raw),
        "dispatch_evidence": {
            key: dispatch_readback[key]
            for key in ("issue_number", "issue_url", "comment_id", "author", "body_digest")
        },
        "rows": rows,
    }
    handoff = {**payload, "handoff_digest": canonical_digest(payload)}
    output = root / ".pm" / "scratch" / str(plan["task_uid"]) / "review-handoffs" / f"{plan['epoch']}.json"
    output = resolve_repo_destination(root, str(output), "review handoff output")
    output.parent.mkdir(parents=True, exist_ok=True)
    serialized = json.dumps(handoff, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    try:
        with output.open("x", encoding="utf-8") as handle:
            handle.write(serialized)
    except FileExistsError as exc:
        raise ContractError(f"refusing to replace immutable handoff: {output}") from exc
    return {"status": "created", "handoff_path": str(output), "handoff_digest": handoff["handoff_digest"],
            "epoch": plan["epoch"]}


def promoted_ledger_bytes(root: Path, plan: dict[str, object],
                          expected_slices: list[dict[str, str]],
                          returns: dict[tuple[str, str], tuple[Path, bytes, dict[str, object]]]) -> bytes:
    rows: list[dict[str, object]] = []
    for expected in expected_slices:
        identity = (expected["role"], expected["slice_id"])
        try:
            artifact_path, raw, returned = returns[identity]
        except KeyError as exc:
            raise ContractError("handoff return coverage is incomplete") from exc
        row: dict[str, object] = {
            "role": identity[0], "slice_id": identity[1],
            "task_uid": plan["task_uid"], "head": plan["frozen_head"],
            "epoch": plan["epoch"], "status": "completed",
            "activation": returned["activation"],
            "context_delivery": returned["context_delivery"],
            "actual_runtime": returned["actual_runtime"],
            "scope_verdict": returned["scope_verdict"],
            "risk_verdict": returned["risk_verdict"],
            "findings": returned["disposition"],
            "residual_risk": returned["residual_risk"],
            "artifact_digest": sha256_bytes(raw),
            "artifacts": [str(artifact_path.resolve(strict=True))],
        }
        if set(row) != COMPLETED_ROW_FIELDS:
            raise ContractError("derived completed ledger row fields are invalid")
        rows.append(row)
    return b"".join(
        (json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n").encode("utf-8")
        for row in rows
    )


def validate_handoff(root: Path, handoff_path: Path, *, expected_plan_path: Path | None = None) -> dict[str, object]:
    """Reopen and independently validate a stored handoff against all bound bytes."""
    root = root.resolve(strict=True)
    resolved_handoff = resolve_repo_file(root, str(handoff_path), "review handoff")
    handoff_value, raw = read_json(resolved_handoff, "review handoff")
    if not isinstance(handoff_value, dict):
        raise ContractError("review handoff fields are invalid")
    handoff = handoff_value
    schema = handoff.get("schema")
    if schema == HANDOFF_SCHEMA_V1:
        handoff_fields = HANDOFF_FIELDS_V1
        row_fields = HANDOFF_ROW_FIELDS_V1
    elif schema == HANDOFF_SCHEMA_V2:
        handoff_fields = HANDOFF_FIELDS_V2
        row_fields = HANDOFF_ROW_FIELDS_V2
    else:
        raise ContractError("review handoff schema is invalid")
    if set(handoff) != handoff_fields or handoff.get("repository") != REPOSITORY:
        raise ContractError("review handoff schema or repository is invalid")
    if canonical_digest({key: value for key, value in handoff.items() if key != "handoff_digest"}) != handoff.get("handoff_digest"):
        raise ContractError("review handoff digest mismatch")
    plan_path = resolve_repo_file(root, handoff.get("plan_path"), "handoff plan")
    if expected_plan_path is not None and plan_path != resolve_repo_file(root, str(expected_plan_path), "expected plan"):
        raise ContractError("review handoff plan path mismatch")
    (plan, source_identity, validated_plan, batch_path, ledger_path, expected_slices,
     plan_raw, batch_raw, original_ledger_raw) = validate_plan_inputs(
        root, plan_path, allow_promoted_ledger=True
    )
    packet_metadata = validate_packet_refs(root, plan, expected_slices, source_identity)
    dispatch_rows: dict[tuple[str, str], dict[str, object]] = {}
    if schema == HANDOFF_SCHEMA_V2:
        expected_dispatch = validate_dispatch_payload(dispatch_payload_for_plan(
            root, validated_plan, allow_promoted_ledger=True,
        ))
        live_dispatch = live_dispatch_readback(
            root, expected_dispatch,
            handoff.get("dispatch_evidence", {}).get("comment_id")
            if isinstance(handoff.get("dispatch_evidence"), dict) else None,
        )
        evidence = handoff.get("dispatch_evidence")
        if not isinstance(evidence, dict) or set(evidence) != DISPATCH_EVIDENCE_FIELDS:
            raise ContractError("review handoff dispatch evidence fields are invalid")
        for key in ("issue_number", "comment_id"):
            value = evidence.get(key)
            if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
                raise ContractError(f"review handoff dispatch {key} is invalid")
        require_string(evidence.get("issue_url"), "handoff dispatch Issue URL")
        require_string(evidence.get("author"), "handoff dispatch author")
        require_string(evidence.get("body_digest"), "handoff dispatch body digest", SHA_RE)
        for key in DISPATCH_EVIDENCE_FIELDS:
            if evidence.get(key) != live_dispatch.get(key):
                raise ContractError(f"review handoff dispatch {key} does not match live readback")
        dispatch_rows = {
            (str(row["role"]), str(row["slice_id"])): row
            for row in expected_dispatch["rows"]
            if isinstance(row, dict)
        }
    if repo_relative(root, validated_plan, "plan") != handoff.get("plan_path"):
        raise ContractError("review handoff plan path is not canonical")
    ledger_rows = parse_ledger(original_ledger_raw, "preflight ledger")
    ledger_statuses = {row.get("status") for row in ledger_rows}
    if ledger_statuses == {"incomplete"}:
        ledger_state = "preflight"
        if sha256_bytes(original_ledger_raw) != handoff.get("preflight_ledger_sha256"):
            raise ContractError("review handoff preflight ledger digest binding mismatch")
    elif ledger_statuses == {"completed"}:
        ledger_state = "promoted"
        require_string(handoff.get("preflight_ledger_sha256"), "handoff original preflight ledger digest", SHA_RE)
    else:
        raise ContractError("review handoff ledger is neither exact preflight nor completed promotion state")
    for field, expected in (
        ("task_uid", plan["task_uid"]), ("pr_number", source_identity["pr_number"]),
        ("comparison_ref", plan["comparison_ref"]), ("comparison_oid", plan["comparison_oid"]),
        ("frozen_head", plan["frozen_head"]), ("source_review_identity", source_identity),
        ("source_review_digest", plan["source_review_digest"]), ("epoch", plan["epoch"]),
        ("plan_sha256", sha256_bytes(plan_raw)), ("batch_path", repo_relative(root, batch_path, "batch")),
        ("batch_sha256", sha256_bytes(batch_raw)),
        ("preflight_ledger_path", repo_relative(root, ledger_path, "preflight ledger")),
    ):
        if handoff.get(field) != expected:
            raise ContractError(f"review handoff {field} binding mismatch")
    rows = handoff.get("rows")
    if not isinstance(rows, list) or len(rows) != len(expected_slices):
        raise ContractError("review handoff rows are invalid")
    if any(not isinstance(row, dict) or set(row) != row_fields for row in rows):
        raise ContractError("review handoff row fields are invalid")
    expected_rows = {(item["role"], item["slice_id"]): item for item in expected_slices}
    seen: set[tuple[str, str]] = set()
    preflight_rows = { (row["role"], row["slice_id"]): row for row in ledger_rows }
    returns: dict[tuple[str, str], tuple[Path, bytes, dict[str, object]]] = {}
    for row in rows:
        role = require_string(row.get("role"), "handoff role")
        slice_id = require_string(row.get("slice_id"), "handoff slice ID", UUID_RE)
        identity = (role, slice_id)
        if identity in seen or identity not in expected_rows:
            raise ContractError("review handoff has duplicate or unexpected role/slice row")
        seen.add(identity)
        return_packet_metadata = packet_metadata[identity]
        if schema == HANDOFF_SCHEMA_V2:
            dispatch_row = dispatch_rows.get(identity)
            if dispatch_row is None:
                raise ContractError(f"dispatch comment lacks packet row for {role}")
            for field in ("packet_path", "packet_digest"):
                if row.get(field) != dispatch_row.get(field) or row.get(field) != packet_metadata[identity][field]:
                    raise ContractError(f"review handoff {field} does not match dispatch packet for {role}")
        else:
            # Historical v1 handoffs predate dispatch evidence and remain
            # content-only audit records; do not reinterpret their returns.
            return_packet_metadata = {
                key: value for key, value in packet_metadata[identity].items()
                if key not in {"packet_path", "packet_digest"}
            }
        artifact_path = resolve_repo_file(root, row.get("artifact_path"), "handoff return")
        if repo_relative(root, artifact_path, "handoff return") != row.get("artifact_path"):
            raise ContractError("review handoff return path is not canonical")
        preflight_path = resolve_repo_file(root, preflight_rows[identity]["artifacts"][0], "preflight return")
        if artifact_path != preflight_path:
            raise ContractError("review handoff return path does not match preflight ledger")
        return_value, return_raw = read_json(artifact_path, f"review return for {role}")
        findings = validate_return(return_value, role=role, slice_id=slice_id, task_uid=str(plan["task_uid"]),
                                   head=str(plan["frozen_head"]), epoch=str(plan["epoch"]),
                                   packet_metadata=return_packet_metadata)
        if row.get("return_sha256") != sha256_bytes(return_raw) or row.get("findings_digest") != canonical_digest(findings):
            raise ContractError(f"review handoff return digest mismatch for {role}")
        returns[identity] = (artifact_path, return_raw, return_value)
    if seen != set(expected_rows):
        raise ContractError("review handoff does not cover every expected role/slice")
    if rows != sorted(rows, key=lambda row: (str(row["role"]).encode(), str(row["slice_id"]).encode())):
        raise ContractError("review handoff rows are not deterministically sorted")
    expected_promoted_raw = promoted_ledger_bytes(root, plan, expected_slices, returns)
    if ledger_state == "promoted":
        if original_ledger_raw != expected_promoted_raw:
            raise ContractError("completed ledger does not equal the handoff-derived promotion bytes")
        collection_path = batch_path.with_name(f"{batch_path.stem}.collection.json")
        if collection_path.exists() or collection_path.is_symlink():
            collection_value, _ = read_json(collection_path, "review collection")
            expected_collection = {
                "schema": "oasis7-review-collection/v1", "status": "passed",
                "epoch": plan["epoch"], "task_uid": plan["task_uid"],
                "frozen_head": plan["frozen_head"],
                "ledger_digest": sha256_bytes(expected_promoted_raw),
                "roles": sorted(str(item["role"]) for item in expected_slices),
            }
            if collection_value != expected_collection:
                raise ContractError("existing collection does not match the handoff-derived promoted ledger")
    else:
        collection_path = batch_path.with_name(f"{batch_path.stem}.collection.json")
        if collection_path.exists() or collection_path.is_symlink():
            raise ContractError("handoff preflight state cannot coexist with a collection receipt")
    return {"handoff": handoff, "handoff_path": str(resolved_handoff), "handoff_raw": raw,
            "plan": plan, "plan_path": str(validated_plan), "batch_path": str(batch_path),
            "ledger_path": str(ledger_path), "original_ledger_raw": original_ledger_raw,
            "expected_slices": expected_slices, "ledger_state": ledger_state,
            "returns": returns, "promoted_ledger_raw": expected_promoted_raw,
            "plan_raw": plan_raw, "batch_raw": batch_raw}


def promotion_lock_path(root: Path, task_uid: str, epoch: str) -> Path:
    require_string(task_uid, "promotion task UID", TASK_RE)
    require_string(epoch, "promotion epoch", SHA_RE)
    return root / ".pm" / "scratch" / task_uid / "review-reservations" / f"{epoch}.lock"


def _load_pm_module(root: Path, module_name: str, filename: str) -> Any:
    module_path = root / "scripts" / "pm" / filename
    spec = importlib.util.spec_from_file_location(module_name, module_path)
    if spec is None or spec.loader is None:
        raise ContractError(f"cannot load repository-owned PM helper: {filename}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


def _atomic_compare_and_swap(ledger_path: Path, original_raw: bytes, expected_raw: bytes) -> str:
    """Atomically replace exactly the handoff-bound bytes, resolving uncertain outcomes by bytes."""
    try:
        current_raw = ledger_path.read_bytes()
    except OSError as exc:
        raise ContractError(f"cannot read preflight ledger before compare-and-swap: {exc}") from exc
    if current_raw != original_raw:
        raise ContractError("preflight ledger compare-and-swap conflict; current bytes are not the handoff-bound original")

    mode = stat.S_IMODE(ledger_path.stat().st_mode)
    fd, temporary_name = tempfile.mkstemp(prefix=f".{ledger_path.name}.promotion.", suffix=".tmp", dir=ledger_path.parent)
    temporary_path = Path(temporary_name)
    try:
        os.fchmod(fd, mode)
        with os.fdopen(fd, "wb") as handle:
            fd = -1
            handle.write(expected_raw)
            handle.flush()
            os.fsync(handle.fileno())
        # Recheck immediately before the single atomic replacement. All supported
        # writers for this task/epoch share the reservation held by the caller.
        try:
            current_raw = ledger_path.read_bytes()
        except OSError as exc:
            raise ContractError(f"cannot reread preflight ledger before compare-and-swap: {exc}") from exc
        if current_raw != original_raw:
            raise ContractError("preflight ledger compare-and-swap conflict; current bytes changed before replacement")
        try:
            os.replace(temporary_path, ledger_path)
        except OSError as exc:
            try:
                observed = ledger_path.read_bytes()
            except OSError as read_exc:
                raise ContractError(f"uncertain ledger replacement outcome cannot be read: {read_exc}") from exc
            if observed == expected_raw:
                # The one replacement completed but its caller observed an error.
                return "applied_after_uncertain_error"
            if observed == original_raw:
                raise ContractError(f"ledger replacement did not occur; original bytes remain: {exc}") from exc
            raise ContractError("uncertain ledger replacement left a third state; refusing to overwrite it") from exc
        try:
            directory_fd = os.open(ledger_path.parent, os.O_RDONLY)
        except OSError:
            directory_fd = None
        if directory_fd is not None:
            try:
                os.fsync(directory_fd)
            finally:
                os.close(directory_fd)
        observed = ledger_path.read_bytes()
        if observed != expected_raw:
            raise ContractError("atomic ledger replacement did not leave the exact expected bytes")
        return "applied"
    finally:
        if fd >= 0:
            os.close(fd)
        try:
            temporary_path.unlink()
        except FileNotFoundError:
            pass


def promote_handoff(root: Path, plan_path: Path, manifest_path: Path,
                    task_uid: str, frozen_head: str, epoch: str) -> dict[str, object]:
    """Validate v2 authority under a same-epoch reservation and promote once, then collect."""
    root = root.resolve(strict=True)
    plan_path = resolve_repo_file(root, str(plan_path), "review plan")
    manifest_path = resolve_repo_file(root, str(manifest_path), "finding resolution manifest")
    if not TASK_RE.fullmatch(task_uid) or not HEAD_RE.fullmatch(frozen_head) or not SHA_RE.fullmatch(epoch):
        raise ContractError("promotion task, head, or epoch identity is invalid")
    handoff_path = root / ".pm" / "scratch" / task_uid / "review-handoffs" / f"{epoch}.json"
    reservation_path = promotion_lock_path(root, task_uid, epoch)
    reservation_path.parent.mkdir(parents=True, exist_ok=True)
    lock_fd = os.open(reservation_path, os.O_CREAT | os.O_RDWR, 0o600)
    outcome = "already_promoted"
    try:
        fcntl.flock(lock_fd, fcntl.LOCK_EX)

        # The validator is the authority gate: it reopens the plan, batch,
        # handoff, original/current ledger, all returns, manifest/readback and
        # repeats the live issue/comment/author/admin reads while reserved.
        review_findings_resolution = _load_pm_module(
            root, "review_findings_resolution", "review-findings-resolution.py"
        )

        manifest_value, _ = read_json(manifest_path, "v2 resolution manifest")
        if not isinstance(manifest_value, dict) or manifest_value.get("schema") != "oasis7-review-resolution/v2":
            raise ContractError("plan-owned preflight promotion requires an oasis7-review-resolution/v2 manifest")
        if manifest_value.get("task_uid") != task_uid or manifest_value.get("head") != frozen_head or manifest_value.get("epoch") != epoch:
            raise ContractError("v2 resolution manifest identity does not match the requested promotion")
        readback_path = manifest_path.with_name(f"{manifest_path.stem}.readback.json")
        manifest_before = manifest_path.read_bytes()
        readback_before = readback_path.read_bytes()
        validated = validate_handoff(root, handoff_path, expected_plan_path=plan_path)
        if validated["handoff"].get("schema") != HANDOFF_SCHEMA_V2:
            raise ContractError("plan-owned preflight promotion requires a v2 dispatch-bound handoff")
        resolution = review_findings_resolution.validate_manifest(
            root, manifest_path, Path(str(validated["ledger_path"])),
            task_uid, frozen_head,
        )
        # Use the immutable plan-bound path returned by independent H validation;
        # the canonical epoch filename above is never treated as ledger authority.
        validated = validate_handoff(root, handoff_path, expected_plan_path=plan_path)
        if validated["handoff"].get("schema") != HANDOFF_SCHEMA_V2:
            raise ContractError("plan-owned preflight promotion requires a v2 dispatch-bound handoff")
        if (validated["plan"].get("task_uid") != task_uid
                or validated["plan"].get("frozen_head") != frozen_head
                or validated["plan"].get("epoch") != epoch):
            raise ContractError("validated handoff identity does not match the requested promotion")
        ledger_path = Path(str(validated["ledger_path"])).resolve(strict=True)
        if ledger_path != Path(str(validated["ledger_path"])):
            raise ContractError("validated preflight ledger path is not canonical")
        if (resolution.get("schema") not in (None, "oasis7-review-resolution/v2")
                or resolution.get("task_uid") != task_uid
                or resolution.get("head") != frozen_head
                or resolution.get("epoch") != epoch):
            raise ContractError("v2 resolution validation result identity mismatch")
        if manifest_path.read_bytes() != manifest_before or readback_path.read_bytes() != readback_before:
            raise ContractError("v2 manifest or readback changed during reserved validation")

        expected_raw = bytes(validated["promoted_ledger_raw"])
        ledger_state = str(validated["ledger_state"])
        if ledger_state == "preflight":
            original_raw = bytes(validated["original_ledger_raw"])
            outcome = _atomic_compare_and_swap(ledger_path, original_raw, expected_raw)
        elif ledger_state != "promoted":
            raise ContractError("validated handoff has an unsupported ledger state")

        # Read-only post-CAS validation: rebind the exact promoted bytes and all
        # handoff-owned files before the collection writer is allowed to run.
        promoted = validate_handoff(root, handoff_path, expected_plan_path=plan_path)
        if promoted["ledger_state"] != "promoted" or bytes(promoted["promoted_ledger_raw"]) != expected_raw:
            raise ContractError("post-CAS handoff validation did not observe the exact expected promoted bytes")
        if (promoted["plan_raw"] != validated["plan_raw"]
                or promoted["batch_raw"] != validated["batch_raw"]
                or promoted["handoff_raw"] != validated["handoff_raw"]
                or {key: value[1] for key, value in promoted["returns"].items()}
                    != {key: value[1] for key, value in validated["returns"].items()}
                or manifest_path.read_bytes() != manifest_before
                or readback_path.read_bytes() != readback_before):
            raise ContractError("a handoff-bound input changed during preflight promotion")

        review_batch_epoch = _load_pm_module(root, "review_batch_epoch", "review-batch-epoch.py")
        collection = review_batch_epoch.validate(argparse.Namespace(
            root=str(root), batch=str(validated["batch_path"]), ledger=str(ledger_path),
        ))
        return {"status": "passed", "promotion": outcome, "task_uid": task_uid,
                "head": frozen_head, "epoch": epoch, "resolution": resolution,
                "collection": collection, "ledger_path": str(ledger_path),
                "reservation_path": str(reservation_path)}
    finally:
        try:
            fcntl.flock(lock_fd, fcntl.LOCK_UN)
        finally:
            os.close(lock_fd)


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    sub = result.add_subparsers(dest="command", required=True)
    promote = sub.add_parser("promote")
    promote.add_argument("--root", required=True)
    promote.add_argument("--plan", required=True)
    promote.add_argument("--manifest", required=True)
    promote.add_argument("--task-uid", required=True)
    promote.add_argument("--head", required=True)
    promote.add_argument("--epoch", required=True)
    return result


def main() -> int:
    args = parser().parse_args()
    try:
        result = promote_handoff(Path(args.root), Path(args.plan), Path(args.manifest),
                                 args.task_uid, args.head, args.epoch)
    except ContractError as exc:
        print(f"review-preflight-handoff: {exc}", file=__import__("sys").stderr)
        return 2
    print(json.dumps(result, ensure_ascii=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())


__all__ = ["ContractError", "canonical_bytes", "canonical_digest", "create_handoff",
           "promote_handoff", "promoted_ledger_bytes", "sha256_bytes", "validate_handoff"]
