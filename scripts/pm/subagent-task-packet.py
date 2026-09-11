#!/usr/bin/env python3
"""Create or validate a bounded, immutable subagent task packet."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path


SCHEMA = "oasis7-subagent-task-packet/v1"
TASK_UID_RE = re.compile(r"task_[0-9a-f]{32}\Z")
SLICE_ID_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}\Z")
MAX_SUMMARY_BYTES = 4096
DELIVERY_MODES = {"minimal_head_bound_task_packet", "full_history_escalation"}
ROLE_ACTIVATIONS = {"message_assigned_adapter_inactive", "named_role_adapter_backed"}
INCREMENTAL_CONTEXT_SCHEMA = "oasis7-review-context/v1"
SHA_RE = re.compile(r"[0-9a-f]{64}\Z")


class PacketError(RuntimeError):
    pass


def fail(message: str) -> None:
    raise PacketError(message)


def git(root: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(root), *args], text=True, capture_output=True
    )
    if result.returncode:
        fail(result.stderr.strip() or f"git {' '.join(args)} failed")
    return result.stdout.strip()


def git_bytes(root: Path, *args: str) -> bytes:
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True)
    if result.returncode:
        fail(result.stderr.decode(errors="replace").strip() or f"git {' '.join(args)} failed")
    return result.stdout


def binary_diff_digest(root: Path, old_head: str, new_head: str) -> str:
    """Hash a diff without repository-configured output filters."""
    return hashlib.sha256(git_bytes(
        root, "diff", "--binary", "--no-ext-diff", "--no-textconv", "--no-renames",
        old_head, new_head,
    )).hexdigest()


def repo_root() -> Path:
    return Path(git(Path.cwd(), "rev-parse", "--show-toplevel")).resolve()


def load_task(root: Path, task_uid: str) -> dict[str, object]:
    if not TASK_UID_RE.fullmatch(task_uid):
        fail(f"invalid task UID: {task_uid}")
    mapping = root / ".pm/github-project-sync/tasks.json"
    try:
        payload = json.loads(mapping.read_text(encoding="utf-8"))
        task = payload["tasks"][task_uid]
    except (OSError, json.JSONDecodeError, KeyError, TypeError) as exc:
        fail(f"task is not present in {mapping}: {task_uid} ({exc})")
    if not isinstance(task, dict) or task.get("task_uid") != task_uid:
        fail(f"mapping record does not match task UID: {task_uid}")
    return task


def current_facts(root: Path, task: dict[str, object], base: str,
                  frozen_base_oid: str | None = None) -> dict[str, object]:
    canonical = Path(str(task.get("canonical_worktree") or task.get("worktree_hint") or "")).resolve()
    if canonical != root:
        fail(f"wrong worktree: mapping requires {canonical}, current worktree is {root}")
    branch = git(root, "branch", "--show-current")
    expected_branch = str(task.get("task_branch") or "")
    if not branch or branch != expected_branch:
        fail(f"wrong branch: mapping requires {expected_branch}, current branch is {branch or '(detached)'}")
    if not base:
        fail("base ref is required")
    head = git(root, "rev-parse", "HEAD")
    if frozen_base_oid:
        resolved = git(root, "rev-parse", "--verify", f"{frozen_base_oid}^{{commit}}")
        if resolved != frozen_base_oid:
            fail(f"frozen base OID does not resolve exactly: {frozen_base_oid}")
        ancestor = subprocess.run(
            ["git", "-C", str(root), "merge-base", "--is-ancestor", resolved, head],
            text=True, capture_output=True,
        )
        if ancestor.returncode != 0:
            fail(f"frozen base OID is not an ancestor of current head: base={resolved}, head={head}")
        base_sha = resolved
    else:
        base_sha = git(root, "rev-parse", "--verify", f"{base}^{{commit}}")
    return {
        "worktree": str(root),
        "branch": branch,
        "base_ref": base,
        "base_sha": base_sha,
        "base_binding": "immutable_oid" if frozen_base_oid else "live_ref",
        "head": head,
    }


def bounded(value: str, name: str) -> str:
    value = value.strip()
    if not value:
        fail(f"missing mandatory field: {name}")
    if len(value.encode("utf-8")) > MAX_SUMMARY_BYTES:
        fail(f"field exceeds {MAX_SUMMARY_BYTES} bytes: {name}")
    return value


def repo_reference(root: Path, value: str, name: str) -> str:
    value = bounded(value, name)
    path = Path(value)
    if path.is_absolute() or ".." in path.parts:
        fail(f"{name} must be a repo-relative reference: {value}")
    if not (root / path).exists():
        fail(f"{name} does not exist: {value}")
    return path.as_posix()


def canonical_digest(packet: dict[str, object]) -> str:
    unsigned = {key: value for key, value in packet.items() if key != "packet_digest"}
    encoded = json.dumps(unsigned, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(encoded.encode("utf-8")).hexdigest()


def load_object(path: Path, name: str) -> dict[str, object]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        fail(f"cannot read {name} {path}: {exc}")
    if not isinstance(value, dict):
        fail(f"{name} must be a JSON object: {path}")
    return value


def resolve_path(root: Path, value: str) -> Path:
    path = Path(value)
    return (path if path.is_absolute() else root / path).resolve()


def resolve_collected_artifact(root: Path, ledger_path: Path, artifact: str) -> Path:
    path = Path(artifact)
    if path.is_absolute():
        return path
    root_path = root / path
    return root_path if root_path.exists() else ledger_path.parent / path


def validate_collected_ledger(root: Path, batch: dict[str, object], ledger_path: Path) -> str:
    """Revalidate collector output and every immutable artifact before reuse."""
    try:
        raw = ledger_path.read_bytes()
        decoded = raw.decode("utf-8")
    except (OSError, UnicodeDecodeError) as exc:
        fail(f"cannot read prior review ledger: {exc}")
    entries: list[dict[str, object]] = []
    for line_number, line in enumerate(decoded.splitlines(), 1):
        if not line.strip():
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError as exc:
            fail(f"invalid prior review ledger JSON on line {line_number}: {exc}")
        if not isinstance(value, dict):
            fail(f"prior review ledger line {line_number} is not an object")
        entries.append(value)

    expected_raw = batch.get("expected_slices")
    if not isinstance(expected_raw, list) or any(not isinstance(item, dict) for item in expected_raw):
        fail("prior review batch expected slices are invalid")
    expected = {(item.get("role"), item.get("slice_id")) for item in expected_raw}
    if len(expected) != len(expected_raw):
        fail("prior review batch contains duplicate expected slices")
    seen: set[tuple[object, object]] = set()
    seen_roles: set[object] = set()
    seen_ids: set[object] = set()
    for item in entries:
        role, slice_id = item.get("role"), item.get("slice_id")
        identity = (role, slice_id)
        if role in seen_roles:
            fail(f"duplicate prior review role: {role}")
        if slice_id in seen_ids:
            fail(f"duplicate prior review slice id: {slice_id}")
        seen_roles.add(role)
        seen_ids.add(slice_id)
        seen.add(identity)
        if item.get("task_uid") != batch.get("task_uid"):
            fail(f"prior review ledger task mismatch for role {role}")
        if item.get("head") != batch.get("frozen_head"):
            fail(f"prior review ledger head mismatch for role {role}")
        if item.get("epoch", item.get("review_epoch")) != batch.get("epoch"):
            fail(f"prior review ledger epoch mismatch for role {role}")
        if item.get("status") != "completed":
            fail(f"prior review ledger is not completed for role {role}")
        artifact_digest = item.get("artifact_digest")
        artifacts = item.get("artifacts")
        if not isinstance(artifact_digest, str) or not SHA_RE.fullmatch(artifact_digest):
            fail(f"invalid prior review artifact digest for role {role}")
        if not isinstance(artifacts, list) or len(artifacts) != 1 or not isinstance(artifacts[0], str):
            fail(f"prior review role {role} must bind exactly one artifact")
        artifact_path = resolve_collected_artifact(root, ledger_path, artifacts[0])
        try:
            artifact_bytes = artifact_path.read_bytes()
        except OSError as exc:
            fail(f"cannot read prior review artifact for role {role}: {artifact_path} ({exc})")
        if hashlib.sha256(artifact_bytes).hexdigest() != artifact_digest:
            fail(f"prior review artifact digest mismatch for role {role}")
        try:
            returned = json.loads(artifact_bytes)
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            fail(f"prior review artifact is not valid JSON for role {role}: {exc}")
        if not isinstance(returned, dict):
            fail(f"prior review artifact is not an object for role {role}")
        identity_fields = {
            "role": role, "slice_id": slice_id, "task_uid": batch.get("task_uid"),
            "head": batch.get("frozen_head"), "epoch": batch.get("epoch"), "status": "completed",
        }
        for field, expected_value in identity_fields.items():
            if returned.get(field) != expected_value:
                fail(f"prior review artifact {field} mismatch for role {role}")
        disposition = returned.get("disposition")
        findings = returned.get("findings")
        residual_risk = returned.get("residual_risk")
        if disposition not in {"findings", "no_findings"}:
            fail(f"prior review artifact disposition is invalid for role {role}")
        if not isinstance(findings, list) or (disposition == "findings" and not findings):
            fail(f"prior review artifact findings are invalid for role {role}")
        if disposition == "no_findings" and findings:
            fail(f"prior review no_findings artifact contains findings for role {role}")
        if not isinstance(residual_risk, str) or not residual_risk.strip():
            fail(f"prior review artifact residual_risk is missing for role {role}")
    missing = expected - seen
    unexpected = seen - expected
    if missing:
        fail(f"prior review ledger is missing expected returns: {sorted(missing)}")
    if unexpected:
        fail(f"prior review ledger has unexpected returns: {sorted(unexpected)}")
    return hashlib.sha256(raw).hexdigest()


def validate_incremental_context(root: Path, context: dict[str, object], task_uid: str,
                                 current_head: str) -> None:
    if context.get("schema") != INCREMENTAL_CONTEXT_SCHEMA or context.get("authority") != "context_only":
        fail("review context is not advisory oasis7-review-context/v1")
    if context.get("task_uid") != task_uid or context.get("current_head_oid") != current_head:
        fail("review context task or current head does not match packet")
    prior_head = str(context.get("prior_head_oid") or "")
    if not re.fullmatch(r"[0-9a-f]{40,64}", prior_head) or prior_head == current_head:
        fail("review context prior head is invalid")
    prior_path_raw = context.get("prior_plan_path")
    if not isinstance(prior_path_raw, str):
        fail("review context prior plan path is missing")
    prior_path = resolve_path(root, prior_path_raw)
    canonical_plan_dir = (root / ".pm" / "scratch" / task_uid / "review-plans").resolve()
    if prior_path.parent != canonical_plan_dir:
        fail("review context prior plan is outside the canonical task review plans")
    try:
        actual_plan_digest = hashlib.sha256(prior_path.read_bytes()).hexdigest()
    except OSError as exc:
        fail(f"cannot read review context prior plan: {exc}")
    if context.get("prior_plan_digest") != actual_plan_digest:
        fail("review context prior plan digest does not match its bytes")
    prior_plan = load_object(prior_path, "review context prior plan")
    if prior_plan.get("task_uid") != task_uid or prior_plan.get("frozen_head") != prior_head:
        fail("review context prior plan identity does not match its context")
    prior_epoch = str(context.get("prior_epoch") or "")
    if prior_plan.get("epoch") != prior_epoch or not re.fullmatch(r"[0-9a-f]{64}", prior_epoch):
        fail("review context prior epoch does not match its plan")
    batch_raw = prior_plan.get("batch_path")
    canonical_batch = (root / ".pm" / "scratch" / task_uid / "review-batches" / f"{prior_epoch}.json").resolve()
    if not isinstance(batch_raw, str) or resolve_path(root, batch_raw) != canonical_batch:
        fail("review context prior batch is not canonical")
    batch = load_object(canonical_batch, "review context prior batch")
    batch_identity = {key: batch.get(key) for key in
                      ("task_uid", "frozen_head", "relevant_evidence_digest", "expected_slices")}
    batch_epoch = hashlib.sha256(json.dumps(
        batch_identity, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")).hexdigest()
    plan_slices = prior_plan.get("expected_slices")
    batch_slices = batch.get("expected_slices")
    if not isinstance(plan_slices, list) or not isinstance(batch_slices, list):
        fail("review context prior batch slice identities are invalid")
    plan_identities = [(item.get("role"), item.get("slice_id")) for item in plan_slices if isinstance(item, dict)]
    batch_identities = [(item.get("role"), item.get("slice_id")) for item in batch_slices if isinstance(item, dict)]
    plan_roles = [item.get("role") for item in plan_slices if isinstance(item, dict)]
    batch_roles = [item.get("role") for item in batch_slices if isinstance(item, dict)]
    if (batch.get("schema") != "oasis7-review-batch/v1" or batch.get("epoch") != batch_epoch
            or batch.get("task_uid") != task_uid or batch.get("frozen_head") != prior_head
            or prior_plan.get("relevant_evidence_digest") != batch.get("relevant_evidence_digest")
            or sorted(plan_identities) != sorted(batch_identities)
            or len(plan_identities) != len(plan_slices) or len(batch_identities) != len(batch_slices)
            or len(set(plan_identities)) != len(plan_identities) or len(set(batch_identities)) != len(batch_identities)
            or prior_plan.get("roles") != plan_roles
            or len(set(plan_roles)) != len(plan_roles)
            or sorted(plan_roles) != sorted(batch_roles)):
        fail("review context prior plan does not match its immutable batch")
    if prior_plan.get("schema") == "oasis7-review-plan/v2":
        helper_path = Path(__file__).with_name("ci_ready_receipt_identity.py")
        spec = importlib.util.spec_from_file_location("ci_ready_receipt_identity_context", helper_path)
        if spec is None or spec.loader is None:
            fail("cannot load v2 review identity helper for prior context")
        helper = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(helper)
        try:
            if prior_plan.get("source_review_digest") != helper.source_review_digest(prior_plan.get("source_review_identity")):
                fail("review context prior v2 source digest is invalid")
            if prior_plan.get("integration_ci_digest") != helper.integration_ci_digest(prior_plan.get("integration_ci_identity")):
                fail("review context prior v2 integration digest is invalid")
        except (TypeError, ValueError) as exc:
            fail(f"review context prior v2 identity is invalid: {exc}")
        if prior_plan.get("relevant_evidence_digest") != prior_plan.get("source_review_digest"):
            fail("review context prior v2 evidence digest does not match source digest")
        if prior_plan.get("integration_ci_provenance") != {"live_validation": "ci-ready-receipt-live", "trusted_integration_artifact": True}:
            fail("review context prior v2 provenance is not trusted")
    source_digest = prior_plan.get("source_review_digest", prior_plan.get("relevant_evidence_digest"))
    if context.get("prior_source_review_digest") != source_digest:
        fail("review context prior source digest does not match its plan")
    if context.get("prior_integration_ci_digest") != prior_plan.get("integration_ci_digest"):
        fail("review context prior integration digest does not match its plan")
    prior_roles = context.get("prior_roles")
    if prior_roles != prior_plan.get("roles") or not isinstance(prior_roles, list) or not prior_roles:
        fail("review context prior roles do not match its plan")
    collection_path_raw = context.get("prior_collection_path")
    if not isinstance(collection_path_raw, str):
        fail("review context prior collection path is missing")
    collection_path = resolve_path(root, collection_path_raw)
    canonical_collection = (root / ".pm" / "scratch" / task_uid / "review-batches" / f"{prior_epoch}.collection.json").resolve()
    if collection_path != canonical_collection:
        fail("review context prior collection is not canonical")
    try:
        collection_bytes = collection_path.read_bytes()
    except OSError as exc:
        fail(f"cannot read review context prior collection: {exc}")
    if context.get("prior_collection_digest") != hashlib.sha256(collection_bytes).hexdigest():
        fail("review context prior collection digest does not match its bytes")
    collection = load_object(collection_path, "review context prior collection")
    if (collection.get("schema") != "oasis7-review-collection/v1" or collection.get("status") != "passed"
            or collection.get("task_uid") != task_uid or collection.get("epoch") != prior_epoch
            or collection.get("frozen_head") != prior_head):
        fail("review context prior collection is not a completed passed collection")
    ledger_raw = prior_plan.get("preflight", {}).get("ledger_path") if isinstance(prior_plan.get("preflight"), dict) else None
    if not isinstance(ledger_raw, str):
        fail("review context prior plan has no ledger")
    ledger_path = resolve_path(root, ledger_raw)
    try:
        ledger_path.relative_to(root.resolve())
    except ValueError:
        fail("review context prior ledger escapes the repository")
    ledger_digest = validate_collected_ledger(root, batch, ledger_path)
    if context.get("prior_collection_ledger_digest") != ledger_digest or collection.get("ledger_digest") != ledger_digest:
        fail("review context prior collection ledger digest does not match its bytes")
    if sorted(collection.get("roles", [])) != sorted(prior_roles):
        fail("review context prior collection roles do not match its plan")
    delta_paths = context.get("delta_paths")
    if not isinstance(delta_paths, list) or any(not isinstance(path, str) for path in delta_paths):
        fail("review context delta paths are invalid")
    if delta_paths != sorted(set(delta_paths)):
        fail("review context delta paths are not sorted and unique")
    actual_paths = [line for line in git(root, "diff", "--name-only", "--no-renames", prior_head, current_head).splitlines() if line]
    if actual_paths != delta_paths:
        fail("review context delta paths do not match the prior-head to current-head diff")
    expected_delta_digest = hashlib.sha256(json.dumps(
        sorted(delta_paths), ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")).hexdigest()
    if context.get("delta_paths_digest") != expected_delta_digest:
        fail("review context delta paths digest is invalid")
    expected_patch_digest = binary_diff_digest(root, prior_head, current_head)
    if context.get("delta_patch_digest") != expected_patch_digest:
        fail("review context patch digest does not match the prior-head to current-head diff")


def validate_packet(root: Path, packet: dict[str, object]) -> None:
    if packet.get("schema") != SCHEMA:
        fail(f"unsupported packet schema: {packet.get('schema')}")
    identity = packet.get("identity")
    slice_contract = packet.get("slice")
    context = packet.get("context")
    if not all(isinstance(value, dict) for value in (identity, slice_contract, context)):
        fail("packet is missing identity, slice, or context objects")
    assert isinstance(identity, dict) and isinstance(slice_contract, dict) and isinstance(context, dict)
    task_uid = bounded(str(identity.get("task_uid") or ""), "identity.task_uid")
    task = load_task(root, task_uid)
    base_binding = str(identity.get("base_binding") or "live_ref")
    frozen_base_oid = str(identity.get("base_sha")) if base_binding == "immutable_oid" else None
    facts = current_facts(root, task, str(identity.get("base_ref") or ""), frozen_base_oid)
    from loop_gate import admission
    try:
        admission(root, task, facts['base_sha'], facts['head'])
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as exc:
        fail(str(exc))
    if packet.get('loop_binding') != task.get('loop_binding'):
        fail('packet loop binding differs from canonical task')
    for field in ("worktree", "branch", "base_sha", "head"):
        if identity.get(field) != facts[field]:
            fail(f"stale or mismatched packet {field}: expected {facts[field]}, got {identity.get(field)}")
    if base_binding != facts["base_binding"]:
        fail(f"stale or mismatched packet base_binding: expected {facts['base_binding']}, got {base_binding}")
    if identity.get("issue_url") != task.get("issue_url"):
        fail("packet issue URL does not match task mapping")
    for field in ("repository", "project_item_id", "task_status"):
        mapping_field = "status" if field == "task_status" else field
        if identity.get(field) != task.get(mapping_field):
            fail(f"packet {field} does not match task mapping")
    bounded(str(identity.get("packet_producer") or ""), "identity.packet_producer")
    for field in ("slice_id", "role", "slice_type", "owner_role", "integration_owner", "integration_order", "context_delivery_mode", "intended_model_configuration", "actual_dispatched_model_reasoning", "actual_runtime_evidence_reason", "role_activation", "write_scope", "return_contract", "validation_command", "formal_sink"):
        bounded(str(slice_contract.get(field) or ""), f"slice.{field}")
    if slice_contract.get("owner_role") != task.get("owner_role"):
        fail("packet owner role does not match task mapping")
    if not SLICE_ID_RE.fullmatch(str(slice_contract["slice_id"])):
        fail("slice.slice_id must be a safe immutable identifier")
    if slice_contract.get("formal_sink") != task.get("issue_url"):
        fail("packet formal sink does not match task issue URL")
    delivery_mode = str(slice_contract.get("context_delivery_mode"))
    if delivery_mode not in DELIVERY_MODES:
        fail(f"unsupported context delivery mode: {delivery_mode}")
    role_activation = str(slice_contract.get("role_activation"))
    if role_activation not in ROLE_ACTIVATIONS:
        fail(f"unsupported role activation: {role_activation}")
    escalation_reason = str(slice_contract.get("full_history_escalation_reason") or "").strip()
    if delivery_mode == "full_history_escalation":
        bounded(escalation_reason, "slice.full_history_escalation_reason")
    elif escalation_reason:
        fail("full-history escalation reason is only valid with full_history_escalation mode")
    for field in ("user_intent", "work_item", "non_goals", "acceptance_target", "evidence_summary", "collaboration_boundary"):
        bounded(str(context.get(field) or ""), f"context.{field}")
    governance = context.get("governance_refs")
    scoped = context.get("scoped_refs")
    if not isinstance(governance, list) or not governance:
        fail("missing mandatory field: context.governance_refs")
    if not isinstance(scoped, list) or not scoped:
        fail("missing mandatory field: context.scoped_refs")
    required = {"AGENTS.md", "doc/engineering/workflow/source-of-truth.md", f".agents/roles/{slice_contract['role']}.md"}
    if not required.issubset(set(governance)):
        fail(f"governance refs must include: {', '.join(sorted(required))}")
    for value in governance + scoped:
        repo_reference(root, str(value), "packet reference")
    review_context = packet.get("review_context")
    if review_context is not None:
        if not isinstance(review_context, dict):
            fail("packet review_context must be an object")
        validate_incremental_context(root, review_context, task_uid, str(identity["head"]))
    if packet.get("packet_digest") != canonical_digest(packet):
        fail("packet digest mismatch")


def validate_bootstrap_snapshot(root: Path, snapshot: Path, task_uid: str) -> dict[str, object]:
    saved = load_object(snapshot, "bootstrap snapshot")
    request = saved.get("request")
    request_identity = request.get("identity") if isinstance(request, dict) else None
    if not isinstance(request_identity, str) or not request_identity:
        fail("bootstrap snapshot request identity is missing")
    helper = Path(__file__).with_name("bootstrap-task-snapshot.py")
    result = subprocess.run(
        [sys.executable, str(helper), "validate-epoch-identity", "--repo-root", str(root),
         "--task-uid", task_uid, "--request-identity", request_identity,
         "--snapshot", str(snapshot)],
        text=True, capture_output=True,
    )
    if result.returncode:
        fail(result.stderr.strip() or result.stdout.strip() or "bootstrap snapshot validation failed")
    return saved


def review_admission(root: Path, packet_path: Path, plan_path: Path,
                     snapshot_path: Path) -> dict[str, object]:
    packet = load_object(packet_path, "packet")
    validate_packet(root, packet)
    identity = packet["identity"]
    slice_contract = packet["slice"]
    assert isinstance(identity, dict) and isinstance(slice_contract, dict)
    task_uid = str(identity["task_uid"])

    plan = load_object(plan_path, "review plan")
    plan_schema = plan.get("schema")
    if plan_schema not in {"oasis7-review-plan/v1", "oasis7-review-plan/v2"}:
        fail(f"unsupported review plan schema: {plan.get('schema')}")
    if plan_schema == "oasis7-review-plan/v2":
        helper_path = Path(__file__).with_name("ci_ready_receipt_identity.py")
        spec = importlib.util.spec_from_file_location("ci_ready_receipt_identity_v2", helper_path)
        if spec is None or spec.loader is None:
            fail("cannot load v2 review identity helper")
        helper = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(helper)
        try:
            if plan.get("source_review_digest") != helper.source_review_digest(plan.get("source_review_identity")):
                fail("v2 source review digest does not match its identity")
            if plan.get("integration_ci_digest") != helper.integration_ci_digest(plan.get("integration_ci_identity")):
                fail("v2 integration CI digest does not match its identity")
        except (TypeError, ValueError) as exc:
            fail(f"invalid v2 review identity: {exc}")
    snapshot = validate_bootstrap_snapshot(root, snapshot_path, task_uid)
    if plan_schema == "oasis7-review-plan/v2":
        source_identity = plan.get("source_review_identity")
        snapshot_task = snapshot.get("task")
        if not isinstance(source_identity, dict) or not isinstance(snapshot_task, dict):
            fail("v2 review identity and bootstrap snapshot task must be objects")
        snapshot_epoch = snapshot_task.get("bootstrap_epoch")
        if type(snapshot_epoch) is not int or snapshot_epoch < 1:
            fail("bootstrap snapshot has an invalid bootstrap epoch")
        if source_identity.get("bootstrap_epoch") != snapshot_epoch:
            fail("v2 source review bootstrap epoch does not match bootstrap snapshot")

    plan_context = plan.get("incremental_review_context")
    packet_context = packet.get("review_context")
    if plan_context is None:
        if packet_context is not None:
            fail("packet carries review context absent from its review plan")
    else:
        if not isinstance(plan_context, dict) or packet_context != plan_context:
            fail("packet review context does not match its review plan")
        validate_incremental_context(root, plan_context, task_uid, str(identity["head"]))

    canonical_packet_dir = (root / ".pm" / "scratch" / task_uid / "slice-packets").resolve()
    if packet_path.parent != canonical_packet_dir:
        fail("review packet is outside the canonical task slice-packets directory")
    canonical_plan_dir = (root / ".pm" / "scratch" / task_uid / "review-plans").resolve()
    if plan_path.parent != canonical_plan_dir:
        fail("review plan is outside the canonical task review-plans directory")

    epoch = plan.get("epoch")
    if not isinstance(epoch, str) or not re.fullmatch(r"[0-9a-f]{64}", epoch):
        fail("review plan has an invalid batch epoch")
    canonical_batch = (root / ".pm" / "scratch" / task_uid / "review-batches" / f"{epoch}.json").resolve()
    planned_batch = plan.get("batch_path")
    if not isinstance(planned_batch, str) or resolve_path(root, planned_batch) != canonical_batch:
        fail("review plan does not reference its canonical review batch")
    batch = load_object(canonical_batch, "review batch")
    if batch.get("schema") != "oasis7-review-batch/v1":
        fail("invalid review batch schema")
    batch_identity = {key: batch.get(key) for key in
                      ("task_uid", "frozen_head", "relevant_evidence_digest", "expected_slices")}
    batch_epoch = hashlib.sha256(json.dumps(
        batch_identity, ensure_ascii=False, sort_keys=True, separators=(",", ":"),
    ).encode("utf-8")).hexdigest()
    if batch.get("epoch") != batch_epoch or epoch != batch_epoch:
        fail("review batch epoch does not match immutable batch contents")
    batch_digest = batch.get("relevant_evidence_digest")
    plan_digest = plan.get("relevant_evidence_digest", plan.get("source_review_digest"))
    if plan.get("task_uid") != batch.get("task_uid") or plan.get("frozen_head") != batch.get("frozen_head"):
        fail("review plan task/head does not match canonical batch")
    if plan_digest != batch_digest:
        fail("review plan evidence digest does not match canonical batch")

    expected = plan.get("expected_slices")
    refs = plan.get("packet_refs")
    roles = plan.get("roles")
    if not isinstance(expected, list) or not isinstance(refs, list) or not isinstance(roles, list):
        fail("review plan is missing roles, expected_slices, or packet_refs")
    if any(not isinstance(item, dict) for item in expected + refs):
        fail("review plan slice and packet references must be objects")
    canonical_expected = sorted(expected, key=lambda item: (str(item.get("role")), str(item.get("slice_id"))))
    if canonical_expected != batch.get("expected_slices"):
        fail("review plan expected slices do not match canonical batch")
    if roles != [item.get("role") for item in expected] or len(set(str(role) for role in roles)) != len(roles):
        fail("review plan roles do not match its expected slice order")
    expected_identities = {(item.get("role"), item.get("slice_id")) for item in expected}
    ref_identities = [(item.get("role"), item.get("slice_id")) for item in refs]
    if len(ref_identities) != len(expected_identities) or set(ref_identities) != expected_identities:
        fail("review plan packet refs do not cover the complete expected slice set")

    packet_role = str(slice_contract["role"])
    packet_slice = str(slice_contract["slice_id"])
    if plan.get("task_uid") != task_uid:
        fail("review plan task UID does not match packet")
    if plan.get("frozen_head") != identity.get("head"):
        fail("review plan frozen head does not match current packet head")
    if plan.get("comparison_ref") != identity.get("base_ref"):
        fail("review plan comparison ref does not match packet base ref")
    if plan.get("comparison_oid") != identity.get("base_sha"):
        fail("review plan comparison OID does not match packet base SHA")
    if str(identity.get("base_binding") or "live_ref") != "immutable_oid":
        resolved_comparison = git(root, "rev-parse", "--verify", f"{plan.get('comparison_ref')}^{{commit}}")
        if resolved_comparison != plan.get("comparison_oid"):
            fail("review plan comparison ref moved from its recorded OID")

    integration_base = plan.get("integration_base_oid")
    if integration_base is not None:
        if git(root, "merge-base", str(integration_base), str(identity.get("head"))) != identity.get("base_sha"):
            fail("review plan integration base does not derive packet scope base")

    expected_matches = [item for item in expected if isinstance(item, dict)
                        and item.get("role") == packet_role and item.get("slice_id") == packet_slice]
    ref_matches = [item for item in refs if isinstance(item, dict)
                   and item.get("role") == packet_role and item.get("slice_id") == packet_slice]
    if len(expected_matches) != 1 or len(ref_matches) != 1:
        fail("packet role and slice must occur exactly once in the review plan")
    planned_packet = ref_matches[0].get("packet_ref")
    if not isinstance(planned_packet, str) or resolve_path(root, planned_packet) != packet_path:
        fail("review packet path does not match the planned packet reference")

    snapshot_task = snapshot.get("task")
    snapshot_git = snapshot.get("git")
    if not isinstance(snapshot_task, dict) or not isinstance(snapshot_git, dict):
        fail("bootstrap snapshot is missing task or git identity")
    if snapshot_task.get("uid") != task_uid or snapshot.get("repository") != identity.get("repository"):
        fail("bootstrap snapshot task or repository does not match packet")
    if (snapshot_git.get("worktree") != identity.get("worktree")
            or snapshot_git.get("branch") != identity.get("branch")):
        fail("bootstrap snapshot git identity does not match packet")

    return {
        "status": "admitted",
        "task_uid": task_uid,
        "head": identity["head"],
        "comparison_ref": identity["base_ref"],
        "comparison_oid": identity["base_sha"],
        "integration_base_oid": integration_base,
        "review_plan_schema": plan_schema,
        "source_review_digest": plan.get("source_review_digest", plan.get("relevant_evidence_digest")),
        "incremental_review_context_digest": (
            hashlib.sha256(json.dumps(plan_context, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")).hexdigest()
            if isinstance(plan_context, dict) else None
        ),
        "role": packet_role,
        "slice_id": packet_slice,
        "packet_digest": packet["packet_digest"],
        "snapshot_digest": snapshot.get("digest"),
    }


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    sub = result.add_subparsers(dest="mode", required=True)
    create = sub.add_parser("create")
    create.add_argument("--task-uid", required=True)
    create.add_argument("--slice-id", required=True)
    create.add_argument("--role", required=True)
    create.add_argument("--slice-type", required=True)
    create.add_argument("--owner-role", required=True)
    create.add_argument("--integration-owner", required=True)
    create.add_argument("--integration-order", required=True)
    create.add_argument("--packet-producer", required=True)
    create.add_argument("--context-delivery-mode", choices=sorted(DELIVERY_MODES), required=True)
    create.add_argument("--full-history-escalation-reason", default="")
    create.add_argument("--intended-model-configuration", required=True)
    create.add_argument("--actual-dispatched-model-reasoning", required=True)
    create.add_argument("--actual-runtime-evidence-reason", required=True)
    create.add_argument("--role-activation", choices=sorted(ROLE_ACTIVATIONS), required=True)
    create.add_argument("--base", required=True)
    create.add_argument("--frozen-base-oid")
    create.add_argument("--user-intent", required=True)
    create.add_argument("--work-item", required=True)
    create.add_argument("--non-goals", required=True)
    create.add_argument("--acceptance-target", required=True)
    create.add_argument("--governance-ref", action="append", default=[])
    create.add_argument("--scoped-ref", action="append", default=[])
    create.add_argument("--evidence-summary", required=True)
    create.add_argument("--collaboration-boundary", required=True)
    create.add_argument("--write-scope", required=True)
    create.add_argument("--return-contract", required=True)
    create.add_argument("--validation-command", required=True)
    create.add_argument("--formal-sink", required=True)
    create.add_argument("--review-plan", help="canonical review plan whose advisory context is embedded in this packet")
    create.add_argument("--out")
    validate = sub.add_parser("validate")
    validate.add_argument("packet")
    admission = sub.add_parser("review-admission")
    admission.add_argument("--packet", required=True)
    admission.add_argument("--review-plan", required=True)
    admission.add_argument("--bootstrap-snapshot", required=True)
    return result


def main() -> int:
    args = parser().parse_args()
    root = repo_root()
    if args.mode == "validate":
        path = Path(args.packet)
        if not path.is_absolute():
            path = root / path
        try:
            packet = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            fail(f"cannot read packet {path}: {exc}")
        validate_packet(root, packet)
        print(f"subagent-task-packet: valid: {path}")
        return 0
    if args.mode == "review-admission":
        packet_path = resolve_path(root, args.packet)
        plan_path = resolve_path(root, args.review_plan)
        snapshot_path = resolve_path(root, args.bootstrap_snapshot)
        print(json.dumps(review_admission(root, packet_path, plan_path, snapshot_path),
                         ensure_ascii=False, sort_keys=True))
        return 0

    task = load_task(root, args.task_uid)
    facts = current_facts(root, task, args.base, args.frozen_base_oid)
    from loop_gate import admission
    loop_admission = admission(root, task, facts['base_sha'], facts['head'])
    governance = [repo_reference(root, item, "governance-ref") for item in args.governance_ref]
    scoped = [repo_reference(root, item, "scoped-ref") for item in args.scoped_ref]
    review_context: dict[str, object] | None = None
    if args.review_plan:
        review_plan_path = resolve_path(root, args.review_plan)
        canonical_plan_dir = (root / ".pm" / "scratch" / args.task_uid / "review-plans").resolve()
        if review_plan_path.parent != canonical_plan_dir:
            fail("--review-plan must be a canonical task review plan")
        review_plan = load_object(review_plan_path, "review plan")
        if review_plan.get("task_uid") != args.task_uid or review_plan.get("frozen_head") != facts["head"]:
            fail("--review-plan task or frozen head does not match packet")
        candidate_context = review_plan.get("incremental_review_context")
        if candidate_context is not None:
            if not isinstance(candidate_context, dict):
                fail("review plan incremental context must be an object")
            validate_incremental_context(root, candidate_context, args.task_uid, str(facts["head"]))
            review_context = candidate_context
    packet: dict[str, object] = {
        "schema": SCHEMA,
        "created_at": datetime.now(timezone.utc).isoformat(),
        "identity": {
            "task_uid": args.task_uid,
            "issue_url": task.get("issue_url"),
            "repository": task.get("repository"),
            "project_item_id": task.get("project_item_id"),
            "task_status": task.get("status"),
            "packet_producer": bounded(args.packet_producer, "identity.packet_producer"),
            **facts,
        },
        "slice": {
            key: bounded(str(getattr(args, key)), f"slice.{key}")
            for key in ("slice_id", "role", "slice_type", "owner_role", "integration_owner", "integration_order", "context_delivery_mode", "intended_model_configuration", "actual_dispatched_model_reasoning", "actual_runtime_evidence_reason", "role_activation", "write_scope", "return_contract", "validation_command", "formal_sink")
        },
        "context": {
            "user_intent": bounded(args.user_intent, "context.user_intent"),
            "work_item": bounded(args.work_item, "context.work_item"),
            "non_goals": bounded(args.non_goals, "context.non_goals"),
            "acceptance_target": bounded(args.acceptance_target, "context.acceptance_target"),
            "governance_refs": governance,
            "scoped_refs": scoped,
            "evidence_summary": bounded(args.evidence_summary, "context.evidence_summary"),
            "collaboration_boundary": bounded(args.collaboration_boundary, "context.collaboration_boundary"),
        },
    }
    if review_context is not None:
        packet["review_context"] = review_context
    packet["slice"]["full_history_escalation_reason"] = bounded(
        args.full_history_escalation_reason,
        "slice.full_history_escalation_reason",
    ) if args.context_delivery_mode == "full_history_escalation" else ""
    if loop_admission['status'] != 'legacy':
        packet['loop_binding'] = task['loop_binding']
    packet["packet_digest"] = canonical_digest(packet)
    validate_packet(root, packet)
    packet_dir = (root / f".pm/scratch/{args.task_uid}/slice-packets").resolve()
    path = Path(args.out) if args.out else packet_dir / f"{args.slice_id}.json"
    if not path.is_absolute():
        path = root / path
    path = path.resolve()
    if path.parent != packet_dir:
        fail(f"packet output must be an immutable file directly under {packet_dir}")
    if path.exists():
        fail(f"refusing to overwrite immutable packet: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(packet, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(path.relative_to(root) if path.is_relative_to(root) else path)
    print(packet["packet_digest"])
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except PacketError as exc:
        print(f"subagent-task-packet: {exc}", file=sys.stderr)
        raise SystemExit(1)
