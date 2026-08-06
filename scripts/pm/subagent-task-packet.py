#!/usr/bin/env python3
"""Create or validate a bounded, immutable subagent task packet."""

from __future__ import annotations

import argparse
import hashlib
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


def current_facts(root: Path, task: dict[str, object], base: str) -> dict[str, str]:
    canonical = Path(str(task.get("canonical_worktree") or task.get("worktree_hint") or "")).resolve()
    if canonical != root:
        fail(f"wrong worktree: mapping requires {canonical}, current worktree is {root}")
    branch = git(root, "branch", "--show-current")
    expected_branch = str(task.get("task_branch") or "")
    if not branch or branch != expected_branch:
        fail(f"wrong branch: mapping requires {expected_branch}, current branch is {branch or '(detached)'}")
    if not base:
        fail("base ref is required")
    return {
        "worktree": str(root),
        "branch": branch,
        "base_ref": base,
        "base_sha": git(root, "rev-parse", "--verify", f"{base}^{{commit}}"),
        "head": git(root, "rev-parse", "HEAD"),
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
    facts = current_facts(root, task, str(identity.get("base_ref") or ""))
    for field in ("worktree", "branch", "base_sha", "head"):
        if identity.get(field) != facts[field]:
            fail(f"stale or mismatched packet {field}: expected {facts[field]}, got {identity.get(field)}")
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
    if plan.get("schema") != "oasis7-review-plan/v1":
        fail(f"unsupported review plan schema: {plan.get('schema')}")
    snapshot = validate_bootstrap_snapshot(root, snapshot_path, task_uid)

    canonical_packet_dir = (root / ".pm" / "scratch" / task_uid / "slice-packets").resolve()
    if packet_path.parent != canonical_packet_dir:
        fail("review packet is outside the canonical task slice-packets directory")
    canonical_plan_dir = (root / ".pm" / "scratch" / task_uid / "review-plans").resolve()
    if plan_path.parent != canonical_plan_dir:
        fail("review plan is outside the canonical task review-plans directory")

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
    resolved_comparison = git(root, "rev-parse", "--verify", f"{plan.get('comparison_ref')}^{{commit}}")
    if resolved_comparison != plan.get("comparison_oid"):
        fail("review plan comparison ref moved from its recorded OID")

    expected = plan.get("expected_slices")
    refs = plan.get("packet_refs")
    if not isinstance(expected, list) or not isinstance(refs, list):
        fail("review plan is missing expected_slices or packet_refs")
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
    facts = current_facts(root, task, args.base)
    governance = [repo_reference(root, item, "governance-ref") for item in args.governance_ref]
    scoped = [repo_reference(root, item, "scoped-ref") for item in args.scoped_ref]
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
    packet["slice"]["full_history_escalation_reason"] = bounded(
        args.full_history_escalation_reason,
        "slice.full_history_escalation_reason",
    ) if args.context_delivery_mode == "full_history_escalation" else ""
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
