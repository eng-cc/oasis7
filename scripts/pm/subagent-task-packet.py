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
from urllib.parse import urlsplit


SCHEMA = "oasis7-subagent-task-packet/v1"
PRIMARY_PACKAGE_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9_-]*\Z")
TASK_UID_RE = re.compile(r"task_[0-9a-f]{32}\Z")
SLICE_ID_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}\Z")
MAX_SUMMARY_BYTES = 4096
DELIVERY_MODES = {"minimal_head_bound_task_packet", "full_history_escalation"}
ROLE_ACTIVATIONS = {"message_assigned_adapter_inactive", "named_role_adapter_backed"}
INCREMENTAL_CONTEXT_SCHEMA = "oasis7-review-context/v1"
SHA_RE = re.compile(r"[0-9a-f]{64}\Z")
INCREMENTAL_REVIEW_SCOPES = {"full", "scoped"}
INCREMENTAL_REVIEW_MODES = {"full_review", "impact_confirmation"}
INCREMENTAL_ESCALATION_REASONS = {
    "unknown_impact", "new_required_role", "authority_drift", "policy_drift",
    "uncovered_finding", "invalid_prior_evidence",
}
INCREMENTAL_OBLIGATION_FIELDS = {
    "mode", "prior_head_oid", "current_head_oid", "delta_paths_digest", "scope_digest",
}
ARCHIVED_ORIGIN_SCHEMA = "oasis7-publication-helper-review-origin/v1"


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


def validate_live_project_review_admission(root: Path, task: dict[str, object],
                                           task_uid: str) -> None:
    mapping_path = root / ".pm/github-project-sync/tasks.json"
    try:
        mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
        mapped_task = mapping["tasks"][task_uid]
        project = mapping["project"]
        if mapped_task != task or not isinstance(project, dict):
            fail("task mapping changed or lacks Project identity during review admission")
        project_id = project.get("id")
        project_owner = project.get("owner")
        project_number = project.get("number")
        project_repo = project.get("repo")
        item_id = task.get("project_item_id")
        repository = task.get("repository")
        issue_number = task.get("issue_number")
        issue_url = task.get("issue_url")
        if (not all(isinstance(value, str) and value for value in
                    (project_id, project_owner, project_repo, item_id, repository, issue_url))
                or type(project_number) is not int or project_number < 1
                or project_repo != repository
                or type(issue_number) is not int or issue_number < 1):
            fail("task mapping lacks exact Project or Issue identity for review admission")
        parsed_url = urlsplit(issue_url)
        if (parsed_url.scheme != "https" or not parsed_url.netloc or parsed_url.query or parsed_url.fragment
                or parsed_url.path != f"/{repository}/issues/{issue_number}"):
            fail("task Issue URL does not match its repository and number")
        helper_path = Path(__file__).with_name("github-project-workflow.py")
        spec = importlib.util.spec_from_file_location("github_project_workflow_readback", helper_path)
        if spec is None or spec.loader is None:
            fail("live Project review-admission reader is unavailable")
        helper = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(helper)
        item = helper.fetch_project_items_by_ids([item_id]).get(item_id)
    except PacketError:
        raise
    except Exception as exc:
        fail(f"live Project review-admission read failed: {exc}")
    if not isinstance(item, dict) or item.get("id") != item_id:
        fail("live Project review-admission read did not return the mapped item")
    if (item.get("_project_id") != project_id
            or item.get("_project_owner") != project_owner
            or type(item.get("_project_number")) is not int
            or item.get("_project_number") != project_number):
        fail("live Project identity does not match the task mapping")
    if item.get("_field_values_has_next_page") is not False:
        fail("live Project review-admission fields are incomplete")
    content = item.get("content")
    if (not isinstance(content, dict) or type(content.get("number")) is not int
            or content.get("number") != issue_number or content.get("url") != issue_url):
        fail("live Project item Issue identity does not match the task mapping")
    body = str(content.get("body") or "")
    body_task_uids = re.findall(r"task_[0-9a-f]{32}", body)
    project_task_uid = helper.field_value(item, "Task UID")
    if (not body_task_uids or set(body_task_uids) != {task_uid}
            or project_task_uid not in ("", task_uid) or helper.item_task_uid(item) != task_uid):
        fail("live Project item Task UID does not match the task mapping")
    live_status = helper.field_value(item, "PM Status")
    live_phase = helper.field_value(item, "Workflow Phase")
    live_lane = helper.field_value(item, "Status")
    if (live_status != "committed" or live_phase != "verification"
            or live_lane != "In Progress"):
        fail("live Project state does not confirm committed/verification review admission")


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


def validate_primary_package(value: object, name: str = "primary_package") -> str:
    package = str(value or "").strip()
    if not PRIMARY_PACKAGE_RE.fullmatch(package):
        fail(f"{name} must be one valid declared Cargo package name")
    return package


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


def _repository_root_for_artifacts(artifact_root: Path) -> Path:
    """Select the Git object database for a live checkout or trusted archive consumer.

    Archive members may live below the repository's common Git directory rather
    than in a checkout. In that case the effective helper's own repository is
    the only allowed object database; a caller-provided target checkout is not
    an authority source.
    """
    candidate = artifact_root.resolve()
    probe = subprocess.run(
        ["git", "-C", str(candidate), "rev-parse", "--show-toplevel"],
        text=True, capture_output=True, check=False,
    )
    if probe.returncode == 0:
        return Path(probe.stdout.strip()).resolve()
    trusted_root = Path(__file__).resolve().parents[2]
    probe = subprocess.run(
        ["git", "-C", str(trusted_root), "rev-parse", "--show-toplevel"],
        text=True, capture_output=True, check=False,
    )
    if probe.returncode != 0:
        fail("trusted helper repository cannot resolve its Git object database")
    return Path(probe.stdout.strip()).resolve()


def _reference_at_tree(artifact_root: Path, value: str, name: str, *,
                       reference_tree_oid: str, git_root: Path,
                       archived: bool) -> str:
    """Validate a repository-relative reference against the reviewed source tree.

    Generated task evidence can exist only in the immutable archive, so archived
    task-scratch references are resolved there. Other references must exist in
    the reviewed Git tree. Live validation retains its historical allowance for
    an existing untracked working-tree reference.
    """
    value = bounded(value, name)
    path = Path(value)
    if (path.is_absolute() or ".." in path.parts or not path.parts
            or path.as_posix() != value):
        fail(f"{name} must be a normalized repo-relative reference: {value}")
    root = artifact_root.resolve()
    candidate = root.joinpath(*path.parts)
    current = root
    for part in path.parts:
        current = current / part
        if current.is_symlink():
            fail(f"{name} must not traverse a symlink: {value}")
    try:
        candidate.resolve().relative_to(root)
    except (OSError, ValueError):
        fail(f"{name} escapes the artifact root: {value}")

    in_task_archive = len(path.parts) >= 3 and path.parts[:2] == (".pm", "scratch")
    if archived and in_task_archive and candidate.exists():
        return path.as_posix()
    if not archived and candidate.exists():
        return path.as_posix()

    result = subprocess.run(
        ["git", "-C", str(git_root), "cat-file", "-e", f"{reference_tree_oid}:{path.as_posix()}"],
        text=True, capture_output=True, check=False,
    )
    if result.returncode == 0:
        return path.as_posix()
    fail(f"{name} does not exist in the reviewed source tree: {value}")


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
        resolved = path.resolve()
    else:
        root_path = root / path
        candidate = root_path if root_path.exists() else ledger_path.parent / path
        resolved = candidate.resolve()
    try:
        resolved.relative_to(root.resolve())
    except ValueError:
        fail(f"prior review artifact escapes the repository: {artifact}")
    return resolved


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
                                 current_head: str, packet_role: str | None = None,
                                 required_roles: list[str] | None = None,
                                 enforce_semantics: bool = True,
                                 git_root: Path | None = None) -> None:
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
    comparison_root = git_root or root
    actual_paths = [line for line in git(comparison_root, "diff", "--name-only", "--no-renames", prior_head, current_head).splitlines() if line]
    if actual_paths != delta_paths:
        fail("review context delta paths do not match the prior-head to current-head diff")
    expected_delta_digest = hashlib.sha256(json.dumps(
        sorted(delta_paths), ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")).hexdigest()
    if context.get("delta_paths_digest") != expected_delta_digest:
        fail("review context delta paths digest is invalid")
    expected_patch_digest = binary_diff_digest(comparison_root, prior_head, current_head)
    if context.get("delta_patch_digest") != expected_patch_digest:
        fail("review context patch digest does not match the prior-head to current-head diff")

    semantic_fields = {
        "review_scope", "escalation_reasons", "role_review_modes",
        "role_review_modes_digest", "scope_digest", "role_review_obligations",
    }
    present_semantic_fields = semantic_fields.intersection(context)
    if not enforce_semantics or not present_semantic_fields:
        # Contexts produced before impact-scoped review was introduced remain
        # valid advisory inputs.  New contexts must be complete as a unit.
        return
    if present_semantic_fields != semantic_fields:
        missing = sorted(semantic_fields - present_semantic_fields)
        fail(f"review context incremental scope schema is incomplete; missing: {', '.join(missing)}")

    review_scope = context.get("review_scope")
    if review_scope not in INCREMENTAL_REVIEW_SCOPES:
        fail("review context review_scope is invalid")
    escalation_reasons = context.get("escalation_reasons")
    if not isinstance(escalation_reasons, list) or any(
        not isinstance(reason, str) or not reason.strip() for reason in escalation_reasons
    ) or len(set(escalation_reasons)) != len(escalation_reasons):
        fail("review context escalation_reasons are invalid")
    if any(reason not in INCREMENTAL_ESCALATION_REASONS for reason in escalation_reasons):
        fail("review context escalation_reasons contain an unknown reason")

    role_modes = context.get("role_review_modes")
    if not isinstance(role_modes, dict) or not role_modes or any(
        not isinstance(role, str) or not role or mode not in INCREMENTAL_REVIEW_MODES
        for role, mode in role_modes.items()
    ):
        fail("review context role_review_modes are invalid")
    mode_roles = set(role_modes)
    if required_roles is not None:
        if any(not isinstance(role, str) for role in required_roles):
            fail("review context required roles are invalid")
        if mode_roles != set(required_roles):
            fail("review context role_review_modes do not cover the required roles")
    if packet_role is not None and packet_role not in mode_roles:
        fail("review context has no role_review_mode for the current packet role")
    if review_scope == "full":
        if any(mode == "impact_confirmation" for mode in role_modes.values()):
            fail("review context full scope cannot contain impact_confirmation")
        if not escalation_reasons:
            fail("review context full scope requires an escalation reason")
    elif not any(mode == "full_review" for mode in role_modes.values()):
        fail("review context scoped review requires a full_review role")
    if review_scope == "scoped" and "unknown_impact" in escalation_reasons:
        fail("review context scoped review cannot use unknown_impact escalation")

    expected_modes_digest = hashlib.sha256(json.dumps(
        role_modes, ensure_ascii=False, sort_keys=True, separators=(",", ":"),
    ).encode("utf-8")).hexdigest()
    if context.get("role_review_modes_digest") != expected_modes_digest:
        fail("review context role_review_modes_digest is invalid")
    expected_scope_digest = hashlib.sha256(json.dumps({
        "prior_head_oid": prior_head,
        "current_head_oid": current_head,
        "delta_paths_digest": context.get("delta_paths_digest"),
        "role_review_modes_digest": expected_modes_digest,
    }, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")).hexdigest()
    scope_digest = context.get("scope_digest")
    if not isinstance(scope_digest, str) or not SHA_RE.fullmatch(scope_digest):
        fail("review context scope_digest is invalid")
    if scope_digest != expected_scope_digest:
        fail("review context scope_digest does not match its bound scope")

    obligations = context.get("role_review_obligations")
    if not isinstance(obligations, dict) or set(obligations) != mode_roles:
        fail("review context role_review_obligations do not match role_review_modes")
    for role, obligation in obligations.items():
        if not isinstance(obligation, dict) or set(obligation) != INCREMENTAL_OBLIGATION_FIELDS:
            fail(f"review context role obligation schema is invalid for role {role}")
        if obligation.get("mode") != role_modes[role]:
            fail(f"review context role obligation mode does not match role mode for {role}")
        if obligation.get("prior_head_oid") != prior_head:
            fail(f"review context role obligation prior head does not match for {role}")
        if obligation.get("current_head_oid") != current_head:
            fail(f"review context role obligation current head does not match for {role}")
        if obligation.get("delta_paths_digest") != context.get("delta_paths_digest"):
            fail(f"review context role obligation delta digest does not match for {role}")
        if obligation.get("scope_digest") != scope_digest:
            fail(f"review context role obligation scope digest does not match for {role}")
    if packet_role is not None:
        packet_obligation = obligations.get(packet_role)
        if not isinstance(packet_obligation, dict):
            fail("review context has no obligation for the current packet role")
        if packet_obligation.get("mode") != role_modes[packet_role]:
            fail("review context current packet role obligation does not match its mode")


def validate_packet_contract(packet: dict[str, object], task_snapshot: dict[str, object],
                             facts_snapshot: dict[str, object], *, artifact_root: Path,
                             reference_tree_oid: str,
                             enforce_incremental_semantics: bool = True,
                             allow_ready_review_commit: bool = False,
                             archived: bool = False,
                             git_root: Path | None = None) -> None:
    """Validate immutable packet semantics against supplied, already-authenticated facts.

    The live wrapper supplies canonical task mapping and Git facts. The archive
    wrapper derives the same shape from its validated origin bundle and trusted
    source objects. This function performs no task lookup or network read.
    """
    if packet.get("schema") != SCHEMA:
        fail(f"unsupported packet schema: {packet.get('schema')}")
    identity = packet.get("identity")
    slice_contract = packet.get("slice")
    context = packet.get("context")
    if not all(isinstance(value, dict) for value in (identity, slice_contract, context)):
        fail("packet is missing identity, slice, or context objects")
    assert isinstance(identity, dict) and isinstance(slice_contract, dict) and isinstance(context, dict)
    task_uid = bounded(str(identity.get("task_uid") or ""), "identity.task_uid")
    if not TASK_UID_RE.fullmatch(task_uid):
        fail("identity.task_uid is invalid")
    task = task_snapshot
    facts = facts_snapshot
    root = artifact_root.resolve()
    if not isinstance(task, dict) or not isinstance(facts, dict):
        fail("packet task and Git fact snapshots must be objects")
    if task.get("task_uid") != task_uid:
        fail("packet task UID does not match the authenticated task snapshot")
    for field in ("worktree", "branch", "base_ref", "base_sha", "base_binding", "head"):
        value = facts.get(field)
        if not isinstance(value, str) or not value:
            fail(f"packet Git fact snapshot is missing {field}")
    if not re.fullmatch(r"[0-9a-f]{40,64}", reference_tree_oid):
        fail("reference tree OID is invalid")
    trusted_git_root = (git_root or _repository_root_for_artifacts(root)).resolve()
    try:
        exact_tree = git(trusted_git_root, "rev-parse", "--verify", f"{reference_tree_oid}^{{tree}}")
    except PacketError as exc:
        fail(f"reference tree is unavailable: {exc}")
    if exact_tree != reference_tree_oid:
        fail("reference_tree_oid must identify an exact Git tree object")

    if packet.get('loop_binding') != task.get('loop_binding'):
        fail('packet loop binding differs from canonical task')
    for field in ("worktree", "branch", "base_sha", "head"):
        if identity.get(field) != facts[field]:
            fail(f"stale or mismatched packet {field}: expected {facts[field]}, got {identity.get(field)}")
    base_binding = str(identity.get("base_binding") or "live_ref")
    if base_binding != facts["base_binding"]:
        fail(f"stale or mismatched packet base_binding: expected {facts['base_binding']}, got {base_binding}")
    if identity.get("issue_url") != task.get("issue_url"):
        fail("packet issue URL does not match task mapping")
    task_package = task.get("primary_package")
    packet_package = identity.get("primary_package")
    if task_package not in (None, ""):
        validate_primary_package(task_package, "task primary_package")
        if packet_package != task_package:
            fail("packet primary_package does not match task mapping")
    elif packet_package not in (None, ""):
        fail("legacy task cannot carry packet primary_package")
    for field in ("repository", "project_item_id", "task_status"):
        mapping_field = "status" if field == "task_status" else field
        if identity.get(field) != task.get(mapping_field):
            ready_review_commit = (
                allow_ready_review_commit and field == "task_status"
                and identity.get("task_status") == "ready"
                and task.get("status") == "committed"
                and task.get("workflow_phase") == "verification"
                and slice_contract.get("slice_type") == "professional_review"
            )
            if ready_review_commit:
                # The live wrapper has already established this exception by
                # reading the exact current Project item. The archive wrapper
                # never enables it: archived facts are constructed per packet.
                pass
            else:
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
        _reference_at_tree(root, str(value), "packet reference",
                           reference_tree_oid=reference_tree_oid,
                           git_root=trusted_git_root, archived=archived)
    review_context = packet.get("review_context")
    if review_context is not None:
        if not isinstance(review_context, dict):
            fail("packet review_context must be an object")
        validate_incremental_context(
            root, review_context, task_uid, str(identity["head"]), str(slice_contract["role"]),
            enforce_semantics=enforce_incremental_semantics, git_root=trusted_git_root,
        )
    if packet.get("packet_digest") != canonical_digest(packet):
        fail("packet digest mismatch")


def validate_packet(root: Path, packet: dict[str, object],
                    enforce_incremental_semantics: bool = True,
                    allow_ready_review_commit: bool = False) -> None:
    """Validate a packet against current canonical task and worktree facts."""
    if packet.get("schema") != SCHEMA:
        fail(f"unsupported packet schema: {packet.get('schema')}")
    identity = packet.get("identity")
    slice_contract = packet.get("slice")
    if not isinstance(identity, dict) or not isinstance(slice_contract, dict):
        fail("packet is missing identity, slice, or context objects")
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

    ready_review_commit = (
        allow_ready_review_commit
        and identity.get("task_status") == "ready"
        and task.get("status") == "committed"
        and task.get("workflow_phase") == "verification"
        and slice_contract.get("slice_type") == "professional_review"
    )
    if identity.get("task_status") != task.get("status") and ready_review_commit:
        validate_live_project_review_admission(root, task, task_uid)

    reference_tree_oid = git(root, "rev-parse", "--verify", f"{facts['head']}^{{tree}}")
    validate_packet_contract(
        packet, task, facts, artifact_root=root, reference_tree_oid=reference_tree_oid,
        enforce_incremental_semantics=enforce_incremental_semantics,
        allow_ready_review_commit=ready_review_commit,
        archived=False, git_root=root,
    )


def _strict_json_object(raw: bytes, label: str) -> dict[str, object]:
    def reject_duplicates(pairs: list[tuple[str, object]]) -> dict[str, object]:
        result: dict[str, object] = {}
        for key, value in pairs:
            if key in result:
                fail(f"{label} contains a duplicate JSON key: {key}")
            result[key] = value
        return result

    try:
        value = json.loads(raw.decode("utf-8"), object_pairs_hook=reject_duplicates)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        fail(f"{label} is not valid UTF-8 JSON: {exc}")
    if not isinstance(value, dict):
        fail(f"{label} must be a JSON object")
    return value


def _archive_member(root: Path, relative: str, label: str) -> tuple[Path, bytes, dict[str, object]]:
    path = Path(relative)
    if (path.is_absolute() or ".." in path.parts or not path.parts
            or path.as_posix() != relative):
        fail(f"{label} path is not a normalized archive-relative path")
    base = root.resolve(strict=True)
    candidate = base.joinpath(*path.parts)
    current = base
    for part in path.parts:
        current = current / part
        if current.is_symlink():
            fail(f"{label} must not traverse a symlink")
    try:
        candidate.resolve(strict=True).relative_to(base)
        raw = candidate.read_bytes()
    except (OSError, ValueError) as exc:
        fail(f"cannot read archived {label}: {exc}")
    return candidate, raw, _strict_json_object(raw, f"archived {label}")


def _json_file_bytes(value: dict[str, object], *, indent: int | None = None) -> bytes:
    text = json.dumps(value, ensure_ascii=False, sort_keys=True, indent=indent)
    return (text + "\n").encode("utf-8")


def _bootstrap_task_record(snapshot: dict[str, object], packet_status: object | None) -> dict[str, object]:
    if snapshot.get("schema") != "oasis7.bootstrap-task-snapshot/v1":
        fail("archived bootstrap snapshot schema is invalid")
    task = snapshot.get("task")
    repository = snapshot.get("repository")
    git_identity = snapshot.get("git")
    if not isinstance(task, dict) or not isinstance(repository, str) or not isinstance(git_identity, dict):
        fail("archived bootstrap snapshot identity is incomplete")
    issue = task.get("issue")
    project = task.get("project")
    base = git_identity.get("base")
    if not isinstance(issue, dict) or not isinstance(project, dict) or not isinstance(base, dict):
        fail("archived bootstrap snapshot is missing Issue, Project, or Git identity")
    if (not isinstance(task.get("uid"), str) or not TASK_UID_RE.fullmatch(str(task.get("uid")))
            or type(issue.get("number")) is not int or issue["number"] < 1
            or not isinstance(issue.get("url"), str) or not issue["url"]
            or not isinstance(project.get("item_id"), str) or not project["item_id"]
            or not isinstance(task.get("owner_role"), str) or not task["owner_role"]
            or not isinstance(task.get("acceptance"), list) or not task["acceptance"]
            or type(task.get("bootstrap_epoch")) is not int or task["bootstrap_epoch"] < 1
            or not isinstance(git_identity.get("worktree"), str) or not git_identity["worktree"]
            or not isinstance(git_identity.get("branch"), str) or not git_identity["branch"]
            or not isinstance(base.get("branch"), str) or not base["branch"]
            or not re.fullmatch(r"[0-9a-f]{40,64}", str(base.get("oid") or ""))
            or not re.fullmatch(r"[0-9a-f]{40,64}", str(git_identity.get("head") or ""))
            or (packet_status is not None and (not isinstance(packet_status, str) or not packet_status))):
        fail("archived bootstrap snapshot stable fields are malformed")
    record: dict[str, object] = {
        "task_uid": task["uid"], "issue_number": issue["number"], "issue_url": issue["url"],
        "project_item_id": project["item_id"],
        "owner_role": task["owner_role"], "acceptance": task["acceptance"],
        "bootstrap_epoch": task["bootstrap_epoch"], "repository": repository,
        "canonical_worktree": git_identity["worktree"], "task_branch": git_identity["branch"],
    }
    if packet_status is not None:
        record["status"] = packet_status
    if "primary_package" in task:
        record["primary_package"] = task["primary_package"]
    if "loop_binding" in task:
        record["loop_binding"] = task["loop_binding"]
    return record


def _validate_origin_snapshot(origin_context: dict[str, object], archive_root: Path,
                              task_uid: str) -> dict[str, object]:
    wrapper = origin_context.get("bootstrap_snapshot")
    if (not isinstance(wrapper, dict) or set(wrapper) != {"value", "raw_sha256"}
            or not isinstance(wrapper.get("value"), dict)
            or not isinstance(wrapper.get("raw_sha256"), str)
            or not SHA_RE.fullmatch(wrapper["raw_sha256"])):
        fail("archived bootstrap snapshot wrapper is invalid")
    snapshot = wrapper["value"]
    unsigned = {key: value for key, value in snapshot.items() if key != "digest"}
    digest = "sha256:" + hashlib.sha256(json.dumps(
        unsigned, ensure_ascii=False, sort_keys=True, separators=(",", ":"),
    ).encode("utf-8")).hexdigest()
    if snapshot.get("digest") != digest:
        fail("archived bootstrap snapshot digest mismatch")
    expected_file = f".pm/scratch/{task_uid}/bootstrap-task-snapshot.json"
    raw = _json_file_bytes(snapshot, indent=2)
    archived_path = archive_root / expected_file
    if archived_path.exists() or archived_path.is_symlink():
        _path, raw, saved = _archive_member(archive_root, expected_file, "bootstrap snapshot")
        if saved != snapshot:
            fail("archived bootstrap snapshot bytes differ from the origin context")
    if hashlib.sha256(raw).hexdigest() != wrapper["raw_sha256"]:
        fail("archived bootstrap snapshot file digest mismatch")
    task = snapshot.get("task")
    if not isinstance(task, dict) or task.get("uid") != task_uid:
        fail("archived bootstrap snapshot Task UID mismatch")
    # Validate stable bootstrap identity without synthesizing Task status:
    # lifecycle status is temporal and is supplied by the packet-origin facts.
    _bootstrap_task_record(snapshot, None)
    return snapshot


def _normalized_archived_batch_path(batch_path_raw: object, original_worktree: object,
                                    canonical_batch_path: str) -> str:
    if not isinstance(original_worktree, str) or not original_worktree:
        fail("authenticated original worktree path is missing")
    origin_root = Path(original_worktree)
    if (not origin_root.is_absolute() or ".." in origin_root.parts
            or str(origin_root) != original_worktree):
        fail("authenticated original worktree path is not canonical")
    if not isinstance(batch_path_raw, str):
        fail("archived review plan batch path is missing")
    batch_path = Path(batch_path_raw)
    if batch_path.is_absolute():
        try:
            # Interpret the old absolute reference against the authenticated
            # bootstrap identity, without resolving or accessing that worktree.
            batch_path = batch_path.relative_to(origin_root)
        except ValueError:
            fail("archived review batch path escapes the authenticated original worktree")
    batch_relative = batch_path.as_posix()
    if batch_relative != canonical_batch_path:
        fail("archived review batch path is not canonical")
    return batch_relative


def _github_issue_identity(url: object, repository: object, issue_number: object) -> tuple[str, int]:
    if (not isinstance(repository, str)
            or not re.fullmatch(r"[A-Za-z0-9._-]+/[A-Za-z0-9._-]+", repository)
            or any(not part or part in {".", ".."} or any(ch.isspace() for ch in part)
                   for part in repository.split("/"))
            or type(issue_number) is not int or issue_number < 1):
        fail("canonical repository or Issue number is malformed")
    if (not isinstance(url, str) or not url or url != url.strip()
            or any(ord(ch) <= 0x20 or ord(ch) == 0x7f for ch in url)
            or any(ch in url for ch in "?#\\")):
        fail("GitHub Issue URL is malformed")
    try:
        parsed = urlsplit(url)
    except ValueError as exc:
        fail(f"GitHub Issue URL is malformed: {exc}")
    issue_path = f"/{repository}/issues/{issue_number}"
    expected_paths = {
        "github.com": issue_path,
        "api.github.com": f"/repos{issue_path}",
    }
    if (parsed.scheme != "https" or parsed.netloc not in expected_paths
            or parsed.path != expected_paths.get(parsed.netloc)
            or parsed.query or parsed.fragment):
        fail("GitHub Issue URL does not identify the canonical repository and Issue")
    return repository, issue_number


def _validate_origin_plan_batch(origin_context: dict[str, object], archive_root: Path,
                                task_uid: str, origin_snapshot: dict[str, object]) -> tuple[dict[str, object], dict[str, object],
                                                       str, str, str, str]:
    plan_wrapper = origin_context.get("review_plan")
    batch_wrapper = origin_context.get("review_batch")
    if (not isinstance(plan_wrapper, dict) or set(plan_wrapper) != {"value", "raw_sha256"}
            or not isinstance(batch_wrapper, dict) or set(batch_wrapper) != {"value", "raw_sha256"}
            or not isinstance(plan_wrapper.get("value"), dict)
            or not isinstance(batch_wrapper.get("value"), dict)
            or not isinstance(plan_wrapper.get("raw_sha256"), str)
            or not SHA_RE.fullmatch(plan_wrapper["raw_sha256"])
            or not isinstance(batch_wrapper.get("raw_sha256"), str)
            or not SHA_RE.fullmatch(batch_wrapper["raw_sha256"])):
        fail("archived review plan or batch wrapper is invalid")
    plan = plan_wrapper["value"]
    batch = batch_wrapper["value"]
    epoch = plan.get("epoch")
    if (plan.get("schema") not in {"oasis7-review-plan/v1", "oasis7-review-plan/v2"}
            or plan.get("task_uid") != task_uid
            or not isinstance(epoch, str) or not re.fullmatch(r"[0-9a-f]{64}", epoch)):
        fail("archived review plan schema or identity is invalid")
    plan_path = plan.get("plan_path")
    dispatch = origin_context.get("dispatch_readback")
    payload = dispatch.get("payload") if isinstance(dispatch, dict) else None
    if not isinstance(payload, dict):
        fail("archived review dispatch payload is missing")
    plan_path = payload.get("plan_path")
    expected_plan_parent = f".pm/scratch/{task_uid}/review-plans"
    if (not isinstance(plan_path, str) or Path(plan_path).parent.as_posix() != expected_plan_parent
            or Path(plan_path).name in {"", ".", ".."}):
        fail("dispatch plan path is not canonical for the archived Task")
    plan_file, plan_raw, archived_plan = _archive_member(archive_root, plan_path, "review plan")
    if archived_plan != plan or hashlib.sha256(plan_raw).hexdigest() != plan_wrapper["raw_sha256"]:
        fail("archived review plan bytes or digest differ from the origin context")
    batch_path_raw = plan.get("batch_path")
    canonical_batch_path = f".pm/scratch/{task_uid}/review-batches/{epoch}.json"
    snapshot_git = origin_snapshot.get("git")
    if not isinstance(snapshot_git, dict):
        fail("authenticated bootstrap snapshot lacks Git identity")
    batch_relative = _normalized_archived_batch_path(
        batch_path_raw, snapshot_git.get("worktree"), canonical_batch_path,
    )
    batch_file, batch_raw, archived_batch = _archive_member(archive_root, batch_relative, "review batch")
    if archived_batch != batch or hashlib.sha256(batch_raw).hexdigest() != batch_wrapper["raw_sha256"]:
        fail("archived review batch bytes or digest differ from the origin context")
    batch_identity = {key: batch.get(key) for key in
                      ("task_uid", "frozen_head", "relevant_evidence_digest", "expected_slices")}
    expected_epoch = hashlib.sha256(json.dumps(
        batch_identity, ensure_ascii=False, sort_keys=True, separators=(",", ":"),
    ).encode("utf-8")).hexdigest()
    def normalized_slice_identities(value: object) -> list[tuple[str, str]] | None:
        if not isinstance(value, list) or not value:
            return None
        identities: list[tuple[str, str]] = []
        for item in value:
            if not isinstance(item, dict) or set(item) != {"role", "slice_id"}:
                return None
            role, slice_id = item.get("role"), item.get("slice_id")
            if (not isinstance(role, str) or not role
                    or not isinstance(slice_id, str) or not slice_id):
                return None
            identities.append((role, slice_id))
        if (len(set(identities)) != len(identities)
                or len({role for role, _slice_id in identities}) != len(identities)):
            return None
        return sorted(identities)

    plan_slice_identities = normalized_slice_identities(plan.get("expected_slices"))
    batch_slice_identities = normalized_slice_identities(batch.get("expected_slices"))
    if (batch.get("schema") != "oasis7-review-batch/v1" or batch.get("epoch") != expected_epoch
            or expected_epoch != epoch or batch.get("task_uid") != task_uid
            or batch.get("frozen_head") != plan.get("frozen_head")
            or plan_slice_identities is None or batch_slice_identities is None
            or plan_slice_identities != batch_slice_identities
            or plan.get("relevant_evidence_digest") != batch.get("relevant_evidence_digest")):
        fail("review plan does not match its immutable review batch")
    plan_sha = hashlib.sha256(plan_raw).hexdigest()
    batch_sha = hashlib.sha256(batch_raw).hexdigest()
    return plan, batch, plan_sha, batch_sha, plan_path, batch_relative


def _validate_review_dispatch(origin_context: dict[str, object], plan: dict[str, object],
                              batch: dict[str, object], plan_sha: str, batch_sha: str,
                              plan_path: str, batch_path: str, packet_rows: list[dict[str, object]],
                              live_task_identity: dict[str, object]) -> dict[str, object]:
    readback = origin_context.get("dispatch_readback")
    fields = {"issue_number", "issue_url", "comment_id", "author", "body_digest", "payload"}
    if not isinstance(readback, dict) or set(readback) != fields:
        fail("original review dispatch readback is malformed")
    payload = readback.get("payload")
    if not isinstance(payload, dict):
        fail("original review dispatch payload is malformed")
    expected_rows = [
        {"role": str(row["role"]), "slice_id": str(row["slice_id"]),
         "packet_path": str(row["repo_path"]), "packet_digest": str(row["value"]["packet_digest"])}
        for row in packet_rows
    ]
    expected_rows.sort(key=lambda row: (row["role"].encode("utf-8"), row["slice_id"].encode("utf-8")))
    source_identity = plan.get("source_review_identity")
    plan_repo = source_identity.get("repository") if isinstance(source_identity, dict) else None
    if not isinstance(plan_repo, str) or not plan_repo:
        plan_repo = live_task_identity.get("repository")
    pr_number = source_identity.get("pr_number") if isinstance(source_identity, dict) else live_task_identity.get("pr_number")
    expected_payload = {
        "schema": "oasis7-review-dispatch/v1", "repository": plan_repo,
        "task_uid": origin_context["task_uid"], "issue_number": readback.get("issue_number"),
        "pr_number": pr_number, "frozen_head": plan.get("frozen_head"),
        "epoch": plan.get("epoch"), "plan_path": plan_path, "plan_sha256": plan_sha,
        "batch_path": batch_path, "batch_sha256": batch_sha, "rows": expected_rows,
    }
    if payload != expected_payload:
        fail("authenticated original dispatch does not bind the exact archived review inputs")
    issue_number = live_task_identity.get("issue_number")
    issue_url = live_task_identity.get("issue_url")
    repository = live_task_identity.get("repository")
    if (type(readback.get("issue_number")) is not int or readback["issue_number"] < 1
            or type(readback.get("comment_id")) is not int or readback["comment_id"] < 1
            or not isinstance(readback.get("author"), str) or not readback["author"].strip()
            or not isinstance(readback.get("body_digest"), str)
            or not SHA_RE.fullmatch(readback["body_digest"])
            or readback.get("issue_number") != issue_number or not isinstance(repository, str)):
        fail("original dispatch readback does not match the canonical live Task Issue")
    comment_identity = _github_issue_identity(readback.get("issue_url"), repository, issue_number)
    task_identity = _github_issue_identity(issue_url, repository, issue_number)
    if comment_identity != task_identity:
        fail("original dispatch readback does not match the canonical live Task Issue")
    canonical_payload = json.dumps(payload, ensure_ascii=False, sort_keys=True,
                                   separators=(",", ":")).encode("utf-8")
    expected_body = ("<!-- oasis7-review-dispatch/v1 -->\n```json\n".encode("utf-8")
                     + canonical_payload + b"\n```")
    if hashlib.sha256(expected_body).hexdigest() != readback["body_digest"]:
        fail("original dispatch body digest does not match its exact canonical payload")
    return payload


def validate_archived_packet(packet: dict[str, object], origin_context: dict[str, object], *,
                             archive_root: Path, live_task_identity: dict[str, object],
                             reviewed_source_oid: str) -> None:
    """Validate archived review-origin bytes after cleanup, without replaying live checkout gates.

    The canonical recovery caller first verifies the UID-derived archive manifest,
    current Task/Issue/Project/PR and helper closure, and performs a fresh live
    dispatch readback. This internal validator cross-binds those readbacks with
    the preserved origin artifacts and delegates packet semantics to the shared
    contract validator above.
    """
    required_keys = {
        "schema", "task_uid", "bootstrap_snapshot", "review_plan", "review_batch",
        "dispatch_readback", "packets", "reviewed_git",
    }
    if not isinstance(origin_context, dict) or set(origin_context) != required_keys:
        fail("archived packet origin context fields are invalid")
    if origin_context.get("schema") != ARCHIVED_ORIGIN_SCHEMA:
        fail("archived packet origin context schema is invalid")
    task_uid = origin_context.get("task_uid")
    if not isinstance(task_uid, str) or not TASK_UID_RE.fullmatch(task_uid):
        fail("archived packet origin Task UID is invalid")
    if not isinstance(live_task_identity, dict):
        fail("fresh live Task identity is unavailable")
    archive_root = archive_root.resolve(strict=True)
    snapshot = _validate_origin_snapshot(origin_context, archive_root, task_uid)
    plan, batch, plan_sha, batch_sha, plan_path, batch_path = _validate_origin_plan_batch(
        origin_context, archive_root, task_uid, snapshot,
    )
    refs = plan.get("packet_refs")
    expected_slices = plan.get("expected_slices")
    roles = plan.get("roles")
    rows = origin_context.get("packets")
    if (not isinstance(refs, list) or not isinstance(expected_slices, list)
            or not isinstance(roles, list) or not isinstance(rows, list)
            or len(refs) != len(expected_slices) or len(rows) != len(expected_slices)):
        fail("archived review plan packet coverage is incomplete")
    expected_by_id: dict[tuple[str, str], dict[str, object]] = {}
    for expected in expected_slices:
        if not isinstance(expected, dict) or set(expected) != {"role", "slice_id"}:
            fail("archived review expected slice entry is malformed")
        identity = (expected["role"], expected["slice_id"])
        if (not all(isinstance(item, str) and item for item in identity)
                or identity in expected_by_id):
            fail("archived review expected slices contain duplicate or invalid identities")
        expected_by_id[identity] = expected
    if roles != [item.get("role") for item in expected_slices] or len(set(roles)) != len(roles):
        fail("archived review plan role order does not match its expected slices")
    refs_by_id: dict[tuple[str, str], dict[str, object]] = {}
    for ref in refs:
        if not isinstance(ref, dict) or set(ref) != {"role", "slice_id", "packet_ref"}:
            fail("archived review packet reference is malformed")
        identity = (ref.get("role"), ref.get("slice_id"))
        if identity not in expected_by_id or identity in refs_by_id:
            fail("archived review packet references contain duplicate or unexpected identities")
        if ref.get("packet_ref") != f".pm/scratch/{task_uid}/slice-packets/{identity[1]}.json":
            fail("archived review packet reference is not canonical")
        refs_by_id[identity] = ref
    if set(refs_by_id) != set(expected_by_id):
        fail("archived review packet references do not cover every planned slice")

    packets_by_id: dict[tuple[str, str], dict[str, object]] = {}
    for row in rows:
        if (not isinstance(row, dict)
                or set(row) != {"role", "slice_id", "repo_path", "raw_sha256", "value"}
                or not isinstance(row.get("value"), dict)
                or not isinstance(row.get("raw_sha256"), str)
                or not SHA_RE.fullmatch(row["raw_sha256"])):
            fail("archived packet origin row is malformed")
        identity = (row["role"], row["slice_id"])
        if identity not in expected_by_id or identity in packets_by_id:
            fail("archived packet origin rows contain duplicate or unexpected identities")
        expected_path = refs_by_id[identity]["packet_ref"]
        if row.get("repo_path") != expected_path:
            fail("archived packet origin path differs from its immutable review plan")
        _path, raw, archived_packet = _archive_member(archive_root, str(expected_path), "review packet")
        if (hashlib.sha256(raw).hexdigest() != row["raw_sha256"]
                or archived_packet != row["value"]):
            fail("archived packet bytes differ from the dispatch-bound origin")
        packets_by_id[identity] = row
    if set(packets_by_id) != set(expected_by_id):
        fail("archived packet origins do not cover every planned slice")

    dispatch_payload = _validate_review_dispatch(
        origin_context, plan, batch, plan_sha, batch_sha, plan_path, batch_path, rows, live_task_identity,
    )
    snapshot_task = snapshot.get("task")
    snapshot_git = snapshot.get("git")
    if not isinstance(snapshot_task, dict) or not isinstance(snapshot_git, dict):
        fail("archived bootstrap snapshot lacks stable Task/Git identity")
    snapshot_issue = snapshot_task.get("issue")
    snapshot_project = snapshot_task.get("project")
    if not isinstance(snapshot_issue, dict) or not isinstance(snapshot_project, dict):
        fail("archived bootstrap snapshot lacks stable Issue/Project identity")
    stable_pairs = (
        ("task_uid", task_uid), ("issue_number", snapshot_issue.get("number")),
        ("issue_url", snapshot_issue.get("url")),
        ("project_item_id", snapshot_project.get("item_id")),
        ("repository", snapshot.get("repository")),
        ("canonical_worktree", snapshot_git.get("worktree")),
        ("task_branch", snapshot_git.get("branch")),
        ("owner_role", snapshot_task.get("owner_role")),
    )
    for field, expected in stable_pairs:
        if live_task_identity.get(field) != expected:
            fail(f"fresh live Task identity {field} differs from the archived bootstrap origin")
    if "primary_package" in snapshot_task and live_task_identity.get("primary_package") != snapshot_task["primary_package"]:
        fail("fresh live Task primary package differs from the archived bootstrap origin")
    if "loop_binding" in snapshot_task and live_task_identity.get("loop_binding") != snapshot_task["loop_binding"]:
        fail("fresh live Task loop binding differs from the archived bootstrap origin")

    reviewed_git = origin_context.get("reviewed_git")
    if (not isinstance(reviewed_git, dict)
            or set(reviewed_git) != {"comparison_oid", "source_scope_oid", "head_oid"}):
        fail("archived reviewed Git identity is malformed")
    source_identity = plan.get("source_review_identity")
    source_scope_oid = (source_identity.get("source_scope_oid")
                        if isinstance(source_identity, dict) else plan.get("source_scope_oid"))
    if source_scope_oid is None:
        source_scope_oid = plan.get("comparison_oid")
    if (reviewed_git.get("comparison_oid") != plan.get("comparison_oid")
            or reviewed_git.get("source_scope_oid") != source_scope_oid
            or reviewed_git.get("head_oid") != reviewed_source_oid
            or plan.get("frozen_head") != reviewed_source_oid
            or batch.get("frozen_head") != reviewed_source_oid):
        fail("archived reviewed Git objects do not match plan, batch, and current source OID")
    if isinstance(source_identity, dict):
        if (source_identity.get("task_uid") != task_uid
                or source_identity.get("repository") != snapshot.get("repository")
                or source_identity.get("source_head_oid") != reviewed_source_oid
                or source_identity.get("pr_number") != dispatch_payload.get("pr_number")):
            fail("archived v2 source review identity conflicts with its dispatch")
        identity_module_path = Path(__file__).with_name("ci_ready_receipt_identity.py")
        spec = importlib.util.spec_from_file_location("ci_ready_receipt_identity_archived_packet", identity_module_path)
        if spec is None or spec.loader is None:
            fail("cannot load source review identity validator")
        identity_module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(identity_module)
        try:
            if plan.get("source_review_digest") != identity_module.source_review_digest(source_identity):
                fail("archived v2 source review digest is invalid")
            identity_module.validate_review_applicability(
                source_identity, plan.get("professional_review_applicability"),
            )
        except (TypeError, ValueError) as exc:
            fail(f"archived v2 source review applicability is invalid: {exc}")
    elif plan.get("schema") == "oasis7-review-plan/v2":
        fail("archived v2 review plan is missing its source review identity")
    comparison_oid = str(plan.get("comparison_oid") or "")
    if not re.fullmatch(r"[0-9a-f]{40,64}", comparison_oid):
        fail("archived review comparison OID is invalid")
    git_root = _repository_root_for_artifacts(archive_root)
    try:
        for oid in (comparison_oid, reviewed_source_oid,
                    str(snapshot_git["head"]), str(snapshot_git["base"]["oid"])):
            if git(git_root, "rev-parse", "--verify", f"{oid}^{{commit}}") != oid:
                fail("archived review/bootstrap Git identity does not resolve exactly")
    except (KeyError, PacketError) as exc:
        fail(f"archived review/bootstrap Git object is unavailable: {exc}")
    ancestry = subprocess.run(
        ["git", "-C", str(git_root), "merge-base", "--is-ancestor", comparison_oid, reviewed_source_oid],
        text=True, capture_output=True, check=False,
    )
    if ancestry.returncode != 0:
        fail("archived review comparison OID is not an ancestor of its frozen source head")
    tree_oid = git(git_root, "rev-parse", "--verify", f"{reviewed_source_oid}^{{tree}}")

    for identity, row in packets_by_id.items():
        archived_packet = row["value"]
        packet_identity = archived_packet.get("identity") if isinstance(archived_packet, dict) else None
        if not isinstance(packet_identity, dict):
            fail("archived packet identity is missing")
        if (packet_identity.get("base_ref") != plan.get("comparison_ref")
                or packet_identity.get("base_sha") != plan.get("comparison_oid")
                or packet_identity.get("head") != reviewed_source_oid):
            fail("archived packet comparison/head facts differ from the frozen review")
        if (packet_identity.get("task_uid") != task_uid
                or packet_identity.get("repository") != snapshot.get("repository")
                or packet_identity.get("project_item_id") != snapshot_project.get("item_id")
                or packet_identity.get("issue_url") != snapshot_issue.get("url")):
            fail("archived packet stable identity differs from the bootstrap snapshot")
        task_snapshot = _bootstrap_task_record(snapshot, packet_identity.get("task_status"))
        facts_snapshot = {key: packet_identity.get(key) for key in
                          ("worktree", "branch", "base_ref", "base_sha", "base_binding", "head")}
        validate_packet_contract(
            archived_packet, task_snapshot, facts_snapshot, artifact_root=archive_root,
            reference_tree_oid=tree_oid, archived=True, git_root=git_root,
        )

    target_identity = packet.get("identity") if isinstance(packet, dict) else None
    target_slice = packet.get("slice") if isinstance(packet, dict) else None
    if not isinstance(target_identity, dict) or not isinstance(target_slice, dict):
        fail("requested archived packet identity is malformed")
    target_key = (target_slice.get("role"), target_slice.get("slice_id"))
    matching = packets_by_id.get(target_key) if isinstance(target_key[0], str) and isinstance(target_key[1], str) else None
    if (matching is None or matching.get("value") != packet
            or target_identity.get("task_uid") != task_uid):
        fail("requested packet is not the exact packet in the authenticated review origin")


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
    validate_packet(root, packet, allow_ready_review_commit=True)
    identity = packet["identity"]
    slice_contract = packet["slice"]
    assert isinstance(identity, dict) and isinstance(slice_contract, dict)
    task_uid = str(identity["task_uid"])
    packet_role = str(slice_contract["role"])

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
            integration_identity = plan.get("integration_ci_identity")
            if integration_identity is None:
                if plan.get("integration_ci_digest") is not None or plan.get("integration_ci_provenance") is not None:
                    fail("v2 source-only plan has unexpected integration CI fields")
                if plan.get("integration_ci_status") != "pending":
                    fail("v2 source-only plan must record pending integration CI")
            elif plan.get("integration_ci_digest") != helper.integration_ci_digest(integration_identity):
                fail("v2 integration CI digest does not match its identity")
            helper.validate_review_applicability(plan.get("source_review_identity"), plan.get("professional_review_applicability"))
            if not re.fullmatch(r"sha256:[0-9a-f]{64}", str(plan.get("impact_projection_digest", ""))):
                fail("v2 review plan lacks a verified impact projection digest")
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
        validate_incremental_context(
            root, plan_context, task_uid, str(identity["head"]), str(slice_contract["role"])
        )

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
    if isinstance(plan_context, dict):
        validate_incremental_context(
            root, plan_context, task_uid, str(identity["head"]), packet_role, [str(role) for role in roles]
        )
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
    create.add_argument("--primary-package", type=validate_primary_package)
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
            validate_incremental_context(
                root, candidate_context, args.task_uid, str(facts["head"]), enforce_semantics=False
            )
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
    if args.primary_package is not None:
        packet["identity"]["primary_package"] = validate_primary_package(args.primary_package)
    if review_context is not None:
        packet["review_context"] = review_context
    packet["slice"]["full_history_escalation_reason"] = bounded(
        args.full_history_escalation_reason,
        "slice.full_history_escalation_reason",
    ) if args.context_delivery_mode == "full_history_escalation" else ""
    if loop_admission['status'] != 'legacy':
        packet['loop_binding'] = task['loop_binding']
    packet["packet_digest"] = canonical_digest(packet)
    validate_packet(root, packet, enforce_incremental_semantics=False)
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
