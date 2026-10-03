#!/usr/bin/env python3
from __future__ import annotations

import argparse
import base64
import contextlib
import hashlib
import importlib.util
import io
import json
import os
import pathlib
import re
import shlex
import shutil
import subprocess
import sys
import tempfile
import uuid
from collections import OrderedDict
from datetime import datetime
from typing import Any

from loop_leaf_result import verification_projection_errors


ALL_STATUSES = ("candidate", "committed", "blocked", "ready", "pr_watch", "done", "deferred")
GATE_OWNED_STATUSES = {"ready", "pr_watch"}
TERMINAL_WORKFLOW_PHASES = {"task_done", "main_sync", "post_merge_done", "closed_without_merge"}
# Issue bodies carry the fine-grained task record. Project fields are only a
# lifecycle projection and local evidence paths are retained only after their
# task/worktree identity and digest have been verified.
issue_authoritative_keys = frozenset(
    {
        "task_uid", "title", "issue_number", "issue_url", "owner_role", "module",
        "status", "workflow_phase", "priority", "worktree_hint", "source_signal",
        "source_type", "severity", "pr_url", "pr_number", "merge_hold",
        "primary_package",
        "loop_binding", "bootstrap_base_oid", "completion_mode",
        "aggregate_plan_comment_id", "aggregate_plan_sha256",
        "aggregate_completion_receipt_sha256",
        "traceability_mode", "coordination_ref", "traceability_record",
        "coordination_record", "traceability_candidate", "aggregate_candidate",
        "non_pr_completion_evidence", "non_pr_completion_evidence_sha256",
        "source_refs", "doc_refs", "related_prd", "acceptance",
        "last_closed_at", "claim_verifications",
    }
)
traceability_context_keys = frozenset(
    {
        "traceability_mode", "coordination_ref", "traceability_record",
        "coordination_record", "traceability_candidate", "aggregate_candidate",
    }
)
traceability_issue_keys = frozenset({"loop_binding", *traceability_context_keys})
project_lifecycle_keys = frozenset({"status", "workflow_phase"})
ISSUE_ROUTE_FIELDS = (
    "status", "workflow_phase", "completion_mode", "aggregate_plan_comment_id",
    "aggregate_plan_sha256", "aggregate_completion_receipt_sha256", "pr_number", "pr_url",
)
LIVE_ROUTE_CACHE_FIELDS = ISSUE_ROUTE_FIELDS
identity_bound_cache_keys = frozenset(
    {
        "repository", "canonical_worktree", "task_branch", "default_branch",
        "non_pr_completion_evidence_file", "non_pr_completion_evidence_sha256",
    }
)
DEFAULT_REPO = "eng-cc/oasis7"
DEFAULT_PROJECT_OWNER = "eng-cc"
DEFAULT_PROJECT_NUMBER = 1
ALLOWED_PHASE_TRANSITIONS = {"task_done": {"main_sync"}, "main_sync": {"main_sync"}}
RECEIPT_SCHEMAS = {"main_sync": ("oasis7_main_sync", "post-merge-main-sync")}
NON_MERGE_INTENT_FIELD = "closed_without_merge_intent"
NON_MERGE_RECEIPT_NAME = "closed-without-merge-receipt.json"
PR_HEAD_REQUIRED_FIELDS = ("headRefOid", "headRefName")
PR_HEAD_OPTIONAL_FIELDS = ("headRepositoryOwner", "headRepositoryName")
PR_HEAD_FIELDS = PR_HEAD_REQUIRED_FIELDS + PR_HEAD_OPTIONAL_FIELDS
NON_MERGE_RECEIPT_SCHEMA_VERSION = 1
PRIMARY_PACKAGE_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9_-]*\Z")

_store_path = pathlib.Path(__file__).with_name("workflow-durable-store.py")
if not _store_path.exists(): _store_path = pathlib.Path.cwd()/"scripts/pm/workflow-durable-store.py"
_store_spec = importlib.util.spec_from_file_location("workflow_durable_store", _store_path)
assert _store_spec and _store_spec.loader
durable_store = importlib.util.module_from_spec(_store_spec); _store_spec.loader.exec_module(durable_store)
_candidate_guard_path = pathlib.Path(__file__).with_name("closed_duplicate_candidate_guard.py")
_candidate_guard_module: Any | None = None


def candidate_admission_guard_module() -> Any:
    global _candidate_guard_module
    if _candidate_guard_module is not None:
        return _candidate_guard_module
    if not _candidate_guard_path.is_file():
        raise RuntimeError(f"candidate admission guard is unavailable beside this task entrypoint: {_candidate_guard_path}")
    spec = importlib.util.spec_from_file_location("closed_duplicate_candidate_guard", _candidate_guard_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load candidate admission guard at {_candidate_guard_path}")
    _candidate_guard_module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(_candidate_guard_module)
    return _candidate_guard_module


class _CommandExit(SystemExit):
    """Keep the CLI's numeric failure status while exposing the diagnostic to callers."""

    def __init__(self, message: str) -> None:
        self.message = f"github-project-task: {message}"
        super().__init__(1)

    def __str__(self) -> str:
        return self.message


def die(message: str) -> None:
    print(f"github-project-task: {message}", file=sys.stderr)
    raise _CommandExit(message)


def validate_primary_package(value: str) -> str:
    value = str(value or "").strip()
    if not PRIMARY_PACKAGE_RE.fullmatch(value):
        raise ValueError("primary_package must be one valid declared Cargo package name")
    return value


def now() -> str:
    return datetime.now().astimezone().isoformat(timespec="seconds")


def load_sync_module() -> Any:
    path = pathlib.Path(__file__).with_name("github-project-sync.py")
    spec = importlib.util.spec_from_file_location("github_project_sync_impl", path)
    if spec is None or spec.loader is None:
        die(f"cannot import {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_non_merge_finalizer_module() -> Any:
    path = pathlib.Path(__file__).with_name("non-merge-finalize.py")
    spec = importlib.util.spec_from_file_location("non_merge_finalizer_impl", path)
    if spec is None or spec.loader is None:
        die(f"cannot import {path}")
    module = importlib.util.module_from_spec(spec)
    script_dir = str(path.parent)
    added = script_dir not in sys.path
    if added:
        sys.path.insert(0, script_dir)
    try:
        spec.loader.exec_module(module)
    finally:
        if added:
            sys.path.remove(script_dir)
    return module


def load_mapping(path: pathlib.Path) -> dict[str, Any]:
    return durable_store.read_mapping(path, {"version": 1, "tasks": {}})


save_mapping = durable_store.replace_json


def merge_task_mapping(
    path: pathlib.Path,
    task_uid: str,
    record: dict[str, Any],
    project: dict[str, Any] | None = None,
    clear_keys: frozenset[str] = frozenset(),
) -> None:
    """Reload under lock and merge only one task, preventing lost updates."""
    def update(latest: dict[str, Any]) -> None:
        if project:
            latest_project = latest.get("project")
            latest_project = latest_project if isinstance(latest_project, dict) else {}
            for key, value in project.items():
                if value not in (None, "") and latest_project.get(key) in (None, ""):
                    latest_project[key] = value
            latest["project"] = latest_project
        latest_record = dict((latest.setdefault("tasks", {}).get(task_uid) or {}))
        for key in clear_keys:
            if key not in traceability_context_keys:
                die(f"merge_task_mapping: refusing to clear non-context key {key}")
            latest_record.pop(key, None)
        for key, value in record.items():
            if key in {"claim_verifications", "evidence_comments"}:
                merged = list(latest_record.get(key) or [])
                for item in value or []:
                    if item not in merged:
                        merged.append(item)
                latest_record[key] = merged
            else:
                latest_record[key] = value
        latest["tasks"][task_uid] = latest_record
    durable_store.transact_json(path, update, {"version": 1, "tasks": {}})


def synchronize_live_issue_traceability(
    repo: str,
    task_uid: str,
    record: dict[str, Any],
    *,
    live: dict[str, Any] | None = None,
    explicit_updates: frozenset[str] = frozenset(),
) -> frozenset[str]:
    """Overlay Issue-authoritative traceability and report deleted context keys."""
    authoritative = live if live is not None else github_issue_record(repo, task_uid)
    if not authoritative:
        die(f"traceability sync: authoritative GitHub issue not found for {task_uid}")
    if authoritative.get("trace_projection_error"):
        trace_projection_loss(task_uid, str(authoritative["trace_projection_error"]))
    if "loop_binding" not in explicit_updates:
        cached_binding = record.get("loop_binding")
        live_binding = authoritative.get("loop_binding")
        if cached_binding is not None and live_binding is None:
            die("traceability sync: live loop binding disappeared; explicit reconciliation required")
        if cached_binding is not None and live_binding != cached_binding:
            die("traceability sync: live loop binding differs from cached immutable binding")
    clear_keys = frozenset(
        key for key in traceability_context_keys
        if key not in explicit_updates and key not in authoritative
    )
    for key in traceability_issue_keys:
        if key in explicit_updates:
            continue
        if key in authoritative:
            record[key] = authoritative[key]
        elif key in traceability_context_keys:
            record.pop(key, None)
    return clear_keys


def atomic_json(path: pathlib.Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    encoded = (json.dumps(payload, indent=2, sort_keys=True, ensure_ascii=False) + "\n").encode()
    fd, temporary = tempfile.mkstemp(prefix=path.name + ".", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as handle:
            handle.write(encoded)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def atomic_text(path: pathlib.Path, value: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix=path.name + ".", dir=path.parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            handle.write(value)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def merge_project_mapping(path: pathlib.Path, project: dict[str, Any]) -> None:
    def update(latest: dict[str, Any]) -> None:
        latest_project = dict(latest.get("project") or {})
        latest_project.update(project)
        latest["project"] = latest_project
    durable_store.transact_json(path, update, {"version": 1, "tasks": {}})


def mapping_path_for(root: pathlib.Path, value: str) -> pathlib.Path:
    path = pathlib.Path(value)
    return path if path.is_absolute() else root / path


def pending_non_merge_phase(root: pathlib.Path, task_uid: str,
                            existing: dict[str, Any], repository: str,
                            project: dict[str, Any]) -> str | None:
    """Validate a pre-terminal intent/receipt pair before Project refresh.

    A Project `done` value is a coarse projection and cannot replace a
    receipt-bound pre-terminal phase while remote terminal effects are still
    resumable.  The pair is deliberately strict: a partial or mismatched
    authority must stop refresh rather than rewrite task truth.
    """
    intent = existing.get(NON_MERGE_INTENT_FIELD)
    if intent is None:
        return None
    if not isinstance(intent, dict):
        die("refresh-task: pending non-merge intent is malformed")
    if intent.get("schema") != "oasis7_non_merge_closeout_intent_v1":
        die("refresh-task: pending non-merge intent schema is unsupported")
    if str(intent.get("task_uid") or "") != task_uid:
        die("refresh-task: pending non-merge intent Task UID mismatch")
    if str(intent.get("repository") or "") != str(repository or ""):
        die("refresh-task: pending non-merge intent repository mismatch")
    project_identity = {
        key: str(project.get(key) or "") for key in ("owner", "number", "id")
    }
    if intent.get("project_identity") != project_identity:
        die("refresh-task: pending non-merge intent Project identity mismatch")
    for key, current_key in (
        ("previous_status", "status"),
        ("previous_workflow_phase", "workflow_phase"),
    ):
        if key in intent and intent.get(key) != existing.get(current_key):
            die("refresh-task: pending non-merge intent predecessor disagrees")
    try:
        finalizer = load_non_merge_finalizer_module()
        canonical_path, receipt_path, receipt, _ = finalizer.resolve_non_merge_receipt(
            root, task_uid,
        )
    except (OSError, ValueError, SystemExit) as exc:
        die(f"refresh-task: non-merge receipt resolution failed: {exc}")
    if receipt is None or not receipt_path.is_file():
        die("refresh-task: pending non-merge intent has no receipt")
    migrated = receipt_path != canonical_path
    if "canonical_receipt_sha256" in intent:
        canonical_digest = intent.get("canonical_receipt_sha256")
        if (not isinstance(canonical_digest, str)
                or not re.fullmatch(r"[0-9a-f]{64}", canonical_digest)):
            die("refresh-task: canonical non-merge receipt digest authority is malformed")
        actual_digest = hashlib.sha256(canonical_path.read_bytes()).hexdigest()
        if canonical_digest != actual_digest:
            die("refresh-task: canonical non-merge receipt digest authority mismatch")
    if migrated:
        receipt_digest = hashlib.sha256(receipt_path.read_bytes()).hexdigest()
        if intent.get("migrated_receipt_sha256") != receipt_digest:
            die("refresh-task: migrated non-merge receipt digest authority mismatch")
    if receipt.get("receipt_type") != "oasis7_closed_without_merge":
        die("refresh-task: pending non-merge receipt type mismatch")
    if receipt.get("schema_version") != NON_MERGE_RECEIPT_SCHEMA_VERSION or receipt.get("issuer") != "non-merge-finalize":
        die("refresh-task: pending non-merge receipt authority mismatch")
    authority_fields = (
        "task_uid", "repository", "issue_number", "project_item_id",
        "project_identity", "reason", "evidence_sha256", "pr_number", "pr_url",
        "previous_status", "previous_workflow_phase",
    ) + PR_HEAD_FIELDS
    pr_bound = bool(intent.get("pr_number") or intent.get("pr_url"))
    if migrated:
        # A migrated sidecar is the modern authority.  Project identity and,
        # for PR-bound tasks, the required ref identity must be present.  The
        # optional head repository fields remain optional, while a no-PR task
        # must not acquire a fabricated PR head requirement.
        legacy_optional = set(PR_HEAD_OPTIONAL_FIELDS)
        if not pr_bound:
            legacy_optional.update(PR_HEAD_REQUIRED_FIELDS)
        if "evidence" not in receipt:
            die("refresh-task: migrated non-merge receipt lacks evidence authority")
    else:
        # The canonical receipt may still be a legacy pre-migration payload.
        legacy_optional = {
            "project_identity", "previous_status", "previous_workflow_phase",
            *PR_HEAD_FIELDS,
        }
    for key in authority_fields:
        if key not in receipt:
            if key not in legacy_optional:
                die("refresh-task: pending non-merge intent and receipt disagree")
            continue
        if receipt.get(key) != intent.get(key):
            die("refresh-task: pending non-merge intent and receipt disagree")
    if pr_bound:
        if migrated and any(not intent.get(key) or not receipt.get(key)
                            for key in PR_HEAD_REQUIRED_FIELDS):
            die("refresh-task: pending PR-bound non-merge authority lacks PR head identity")
    if (migrated or "previous_status" in receipt) and receipt.get("previous_status") != existing.get("status"):
        die("refresh-task: pending non-merge receipt status snapshot disagrees")
    if (migrated or "previous_workflow_phase" in receipt) and receipt.get("previous_workflow_phase") != existing.get("workflow_phase"):
        die("refresh-task: pending non-merge receipt phase snapshot disagrees")
    phase = str(existing.get("workflow_phase") or "")
    if phase in {"done", "closed_without_merge", "post_" + "merge_done"}:
        die("refresh-task: pending non-merge intent is not pre-terminal")
    return phase


def run_text(cmd: list[str]) -> str:
    result = subprocess.run(
        cmd,
        check=True,
        text=True,
        encoding="utf-8",
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=180,
    )
    return result.stdout.strip()


def authoritative_repository_identity(root: pathlib.Path, repository: str, worktree_hint: str) -> dict[str, str]:
    """Resolve the task/repository identity from the registered git worktree."""
    root = root.resolve(strict=True)
    requested = pathlib.Path(worktree_hint or root).expanduser()
    # A supplied hint is an identity claim, not a preference. Falling back to
    # the command root when it is absent can silently bind a new task to an
    # unrelated coordination worktree.
    if worktree_hint and not requested.exists():
        die(f"canonical worktree hint does not exist: {requested}")
    canonical = requested.resolve(strict=True)
    def resolved_common_dir(worktree: pathlib.Path) -> pathlib.Path:
        value = pathlib.Path(run_text(["git", "-C", str(worktree), "rev-parse", "--git-common-dir"]))
        return value.resolve() if value.is_absolute() else (worktree / value).resolve()
    root_common_dir = resolved_common_dir(root)
    candidate_common_dir = resolved_common_dir(canonical)
    if candidate_common_dir != root_common_dir:
        die(f"worktree hint belongs to a different git common dir: {canonical}")
    # Membership comes only from the command root's repository registry.  A
    # foreign worktree cannot authorize itself by listing its own family.
    registered = run_text(["git", "-C", str(root), "worktree", "list", "--porcelain"])
    worktrees: list[tuple[str, str]] = []
    current_path = ""
    for line in registered.splitlines() + [""]:
        if line.startswith("worktree "):
            current_path = str(pathlib.Path(line.removeprefix("worktree ")).resolve())
        elif line.startswith("branch refs/heads/") and current_path:
            worktrees.append((current_path, line.removeprefix("branch refs/heads/")))
        elif not line:
            current_path = ""
    canonical_text = str(canonical)
    task_branch = next((branch for path, branch in worktrees if path == canonical_text), "")
    if not task_branch:
        die(f"canonical worktree is detached or unregistered: {canonical}")
    try:
        default_branch = run_text(["git", "-C", str(canonical), "symbolic-ref", "--short", "refs/remotes/origin/HEAD"]).removeprefix("origin/")
    except subprocess.CalledProcessError:
        # `git worktree list` emits the primary worktree first; for a local
        # repository without origin this is the only authoritative default
        # branch fact available.
        default_branch = worktrees[0][1] if worktrees else ""
    normalized_repository = repository.strip().strip("/")
    if not re.fullmatch(r"[^/\s]+/[^/\s]+", normalized_repository):
        die(f"invalid repository identity: {repository!r}")
    if not default_branch:
        die("cannot resolve repository default branch from git facts")
    return {
        "repository": normalized_repository,
        "canonical_worktree": canonical_text,
        "task_branch": task_branch,
        "default_branch": default_branch,
    }


def issue_number_from_url(issue_url: str) -> int:
    try:
        return int(issue_url.rstrip("/").rsplit("/", 1)[-1])
    except ValueError as exc:
        raise RuntimeError(f"cannot parse issue number from {issue_url}") from exc


def validate_issue_identity(repo: str, issue_number: Any, issue_url: Any, *, source: str) -> tuple[int, str]:
    """Require a complete Issue handle bound to the requested repository."""
    number_text = str(issue_number or "").strip()
    url_text = str(issue_url or "").strip()
    if not re.fullmatch(r"[1-9]\d*", number_text) or not url_text:
        die(f"classify-non-pr-task: {source} Issue identity is incomplete")
    match = re.fullmatch(
        r"https://github\.com/([^/\s]+/[^/\s]+)/issues/(\d+)/?",
        url_text,
    )
    if not match or match.group(1) != repo or int(match.group(2)) != int(number_text):
        die(f"classify-non-pr-task: {source} Issue identity is invalid or repository-bound incorrectly")
    return int(number_text), url_text


def pr_number_from_url(pr_url: str) -> int | None:
    match = re.search(r"/pull/(\d+)(?:$|[?#])", pr_url)
    return int(match.group(1)) if match else None


def same_pr_number(left: Any, right: Any) -> bool:
    """Compare canonical numeric PR identities across Issue and mapping encodings."""
    def canonical(value: Any) -> str | None:
        if type(value) is int and value > 0:
            return str(value)
        if isinstance(value, str) and re.fullmatch(r"[1-9][0-9]*", value):
            return value
        return None

    left_empty = left is None or (isinstance(left, str) and left == "")
    right_empty = right is None or (isinstance(right, str) and right == "")
    if left_empty or right_empty:
        return left_empty and right_empty
    left_number = canonical(left)
    right_number = canonical(right)
    return left_number is not None and left_number == right_number


ISSUE_LIST_SECTIONS = ("Source refs:", "Doc refs:", "Related PRD:", "Acceptance:")


def issue_section_rows(body: str, header: str, *, references: bool) -> list[str] | None:
    """Read one Issue list without allowing adjacent sections to bleed into it."""
    lines = body.replace("\r\n", "\n").splitlines()
    try:
        start = next(index for index, line in enumerate(lines) if line.strip() == header) + 1
    except StopIteration:
        return None
    values: list[str] = []
    for line in lines[start:]:
        stripped = line.strip()
        if stripped in ISSUE_LIST_SECTIONS:
            break
        if not stripped:
            if values:
                break
            continue
        if references:
            match = re.fullmatch(r"- `([^`]+)`", line)
        else:
            match = re.fullmatch(r"-\s+(?:\[[ xX]\]\s*)?(.*\S)\s*", line)
        if not match:
            break
        values.append(match.group(1).strip())
    return values


def strict_issue_scalar_fields(body: str, keys: tuple[str, ...]) -> dict[str, str]:
    """Read safety-sensitive Issue scalars without accepting first-match ambiguity."""
    fields: dict[str, str] = {}
    for key in keys:
        lines = re.findall(
            rf"^[ \t]*(?:-[ \t]+)?{re.escape(key)}\b[^\n]*$",
            body,
            re.MULTILINE,
        )
        if not lines:
            continue
        if len(lines) != 1:
            die(f"task Issue {key} field is duplicated")
        match = re.fullmatch(rf"[ \t]*-[ \t]+{re.escape(key)}: `([^`\n]*)`[ \t]*", lines[0])
        if not match:
            die(f"task Issue {key} field is malformed")
        fields[key] = match.group(1)
    return fields


def issue_task_fields(body: str) -> dict[str, Any]:
    body = body.replace("\r\n", "\n")
    fields: dict[str, Any] = {}
    binding_matches = re.findall(r"^- loop_binding_b64: `([^`]+)`$", body, re.MULTILINE)
    if "loop_binding_b64:" in body:
        if len(binding_matches) != 1:
            die("loop binding is malformed or duplicated")
        try:
            encoded = binding_matches[0]
            binding = json.loads(base64.b64decode(encoded + "=" * (-len(encoded) % 4), altchars=b"-_", validate=True))
            if not isinstance(binding, dict):
                raise ValueError("binding must be an object")
        except (ValueError, UnicodeError) as exc:
            die(f"invalid loop binding: {exc}")
        fields["loop_binding"] = binding
    package_lines = re.findall(r"^- primary_package:.*$", body, re.MULTILINE)
    package_matches = re.findall(r"^- primary_package: `([^`]+)`$", body, re.MULTILINE)
    if "primary_package:" in body:
        if len(package_lines) != 1 or len(package_matches) != 1:
            die("primary package is malformed or duplicated")
        try:
            fields["primary_package"] = validate_primary_package(package_matches[0])
        except ValueError as exc:
            die(str(exc))
    context_matches = re.findall(r"^- traceability_context_b64: `([^`]+)`$", body, re.MULTILINE)
    if "traceability_context_b64:" in body:
        if len(context_matches) != 1:
            die("traceability context is malformed or duplicated")
        try:
            encoded = context_matches[0]
            context = json.loads(base64.b64decode(
                encoded + "=" * (-len(encoded) % 4), altchars=b"-_", validate=True,
            ).decode("utf-8"))
            if not isinstance(context, dict):
                raise ValueError("traceability context must be an object")
            allowed = {
                "traceability_mode", "coordination_ref", "traceability_record",
                "coordination_record", "traceability_candidate", "aggregate_candidate",
            }
            if set(context) - allowed:
                raise ValueError("traceability context contains unknown fields")
        except (ValueError, UnicodeDecodeError, json.JSONDecodeError) as exc:
            die(f"invalid traceability context: {exc}")
        fields.update(context)
    fields.update(strict_issue_scalar_fields(body, ISSUE_ROUTE_FIELDS))
    if "pr_number" in fields:
        if not re.fullmatch(r"[1-9][0-9]*", fields["pr_number"]):
            die("task Issue pr_number field is malformed")
        fields["pr_number"] = int(fields["pr_number"])
    for key in ("owner_role", "module", "priority", "worktree_hint", "source_signal", "source_type", "severity", "bootstrap_base_oid", "non_pr_completion_evidence_sha256", "last_closed_at"):
        match = re.search(rf"^- {re.escape(key)}: `([^`]+)`$", body, re.MULTILINE)
        if match:
            fields[key] = match.group(1)
    claim_matches = re.findall(r"^- claim_verifications_b64: `([^`]+)`$", body, re.MULTILINE)
    if "claim_verifications_b64:" in body:
        if len(claim_matches) != 1:
            fields["trace_projection_error"] = "claim verification projection is malformed or duplicated"
        else:
            try:
                padding = "=" * (-len(claim_matches[0]) % 4)
                claims = json.loads(base64.b64decode(
                    claim_matches[0] + padding, altchars=b"-_", validate=True,
                ).decode("utf-8"))
                if not isinstance(claims, list) or any(not isinstance(claim, dict) for claim in claims):
                    raise ValueError("claim verifications must be an array of objects")
                fields["claim_verifications"] = claims
            except (ValueError, UnicodeDecodeError, json.JSONDecodeError) as exc:
                fields["trace_projection_error"] = f"malformed claim verification projection: {exc}"
    evidence_match = re.search(r"^- non_pr_completion_evidence_b64: `([^`]+)`$", body, re.MULTILINE)
    if evidence_match:
        encoded = evidence_match.group(1)
        try:
            padding = "=" * (-len(encoded) % 4)
            fields["non_pr_completion_evidence"] = base64.b64decode(
                encoded + padding, altchars=b"-_", validate=True
            ).decode("utf-8")
        except (ValueError, UnicodeDecodeError):
            fields["trace_projection_error"] = "malformed non-PR completion evidence encoding"
    hold_values: dict[str, Any] = {}
    for key in ("kind", "requester", "reason", "resume_authority", "active"):
        match = re.search(rf"^- merge_hold_{key}: `([^`]+)`$", body, re.MULTILINE)
        if match:
            hold_values[key] = match.group(1)
    if hold_values:
        hold_values["active"] = str(hold_values.get("active", "false")).lower() == "true"
        fields["merge_hold"] = hold_values
    for key, header in (("source_refs", "Source refs:"), ("doc_refs", "Doc refs:"), ("related_prd", "Related PRD:")):
        values = issue_section_rows(body, header, references=True)
        if values is not None:
            fields[key] = values
    fields["acceptance"] = issue_section_rows(body, "Acceptance:", references=False) or []
    return fields


def require_supplied_uid_absent(repo: str, task_uid: str) -> None:
    """Search indexing cannot prove absence for a predetermined task identity."""
    seen_ids, seen_numbers = set(), set()
    for page in range(1, 101):
        issues = json.loads(run_text(['gh', 'api',
            f'repos/{repo}/issues?state=all&sort=created&direction=asc&per_page=100&page={page}']))
        if not isinstance(issues, list) or len(issues) > 100:
            die('repository Issue enumeration incomplete or malformed')
        for issue in issues:
            if (not isinstance(issue, dict) or type(issue.get('id')) is not int or issue['id'] < 1
                    or type(issue.get('number')) is not int or issue['number'] < 1
                    or 'body' not in issue or (issue['body'] is not None and not isinstance(issue['body'], str))):
                die('repository Issue enumeration missing canonical identity/body')
            if issue['id'] in seen_ids or issue['number'] in seen_numbers:
                die('repository Issue enumeration pagination ambiguous')
            seen_ids.add(issue['id']); seen_numbers.add(issue['number'])
            if 'pull_request' in issue:
                if not isinstance(issue['pull_request'], dict):
                    die('repository Issue enumeration malformed pull request record')
                continue
            body = (issue['body'] or '').replace('\r\n', '\n')
            uids = re.findall(r'^task_uid:\s*(task_[0-9a-f]{32})$', body, re.MULTILINE)
            if task_uid in uids:
                die('manual task UID already exists in repository Issues; use explicit existing-task resume')
        if len(issues) < 100:
            return
    die('repository Issue enumeration limit exhausted; absence unproven')


def github_issue_record(repo: str, task_uid: str) -> dict[str, Any] | None:
    search_payload = run_text(
        [
            "gh",
            "issue",
            "list",
            "-R",
            repo,
            "--state",
            "all",
            "--search",
            f"{task_uid} in:body",
            "--json",
            "number,url,title,state",
            "--limit",
            "5",
        ]
    )
    hits = json.loads(search_payload)
    if not isinstance(hits, list) or len(hits) >= 5:
        die("task Issue discovery incomplete; cannot establish canonical identity")
    matches = []
    for hit in hits:
        number = int(hit.get("number") or 0)
        if not number:
            die("task Issue discovery returned invalid identity")
        candidate = json.loads(run_text(["gh", "issue", "view", str(number), "-R", repo,
                                         "--json", "body,number,title,url,state,stateReason,updatedAt"]))
        candidate_body = str(candidate.get("body") or "").replace("\r\n", "\n")
        fields = re.findall(r"^task_uid:[^\n]*$", candidate_body, re.MULTILINE)
        uids = re.findall(r"^task_uid:\s*(task_[0-9a-f]{32})$", candidate_body, re.MULTILINE)
        if task_uid in uids:
            if fields != ["task_uid: " + task_uid] or uids != [task_uid]:
                die("task Issue has ambiguous canonical UID")
            matches.append((hit, candidate))
    if len(matches) > 1:
        die("multiple canonical task Issues; reconcile before creation")
    if not matches:
        return None
    hit, issue = matches[0]
    hits = [hit]
    issue_number = int(hit.get("number") or 0)
    if not issue_number:
        return None
    body = str(issue.get("body") or "").replace("\r\n", "\n")
    if re.findall(r"^task_uid:[^\n]*$", body, re.MULTILINE) != ["task_uid: " + task_uid]:
        return None
    record = issue_task_fields(body)
    title = str(issue.get("title") or hits[0].get("title") or "")
    if title.startswith("[PM] "):
        title = title[5:]
    record.update(
        {
            "task_uid": task_uid,
            "title": title,
            "issue_number": int(issue.get("number") or issue_number),
            "issue_url": str(issue.get("url") or hits[0].get("url") or ""),
            "issue_state": str(issue.get("state") or hits[0].get("state") or ""),
            "issue_state_reason": str(issue.get("stateReason") or ""),
            "updated_at": str(issue.get("updatedAt") or ""),
            "_github_source": "issue_search",
        }
    )
    return record


def github_pull_request(repo: str, pr_number: int) -> dict[str, Any]:
    """Read the canonical PR object directly from the requested repository."""
    payload = json.loads(run_text(["gh", "api", f"repos/{repo}/pulls/{pr_number}"]))
    if not isinstance(payload, dict):
        die("record-pr: live PR readback is malformed")
    return payload


def has_exact_task_pr_linkage(body: Any, task_uid: str, issue_number: int) -> bool:
    """Require one canonical whole-line Task marker and non-closing Refs line."""
    if not isinstance(body, str):
        return False
    lines = body.splitlines()
    task_lines = [line for line in lines if line.startswith("Task:")]
    refs_lines = [line for line in lines if line.startswith("Refs")]
    return task_lines == [f"Task: {task_uid}"] and refs_lines == [f"Refs #{issue_number}"]


def validate_record_pr_live_identity(
    args: argparse.Namespace,
    record: dict[str, Any],
    pr_number: int,
    *,
    allow_exact_publication_poststate: bool = False,
    publication_intent: dict[str, Any] | None = None,
    publication_module: Any | None = None,
) -> dict[str, Any]:
    """Bind record-pr to the live task Issue, registered worktree and live PR head."""
    try:
        live_issue = github_issue_record(args.repo, args.task_uid)
    except (OSError, subprocess.CalledProcessError, json.JSONDecodeError, RuntimeError) as exc:
        die(f"record-pr: live task Issue readback failed: {exc}")
    if not live_issue:
        die("record-pr: authoritative task Issue was not found")
    if live_issue.get("task_uid") != args.task_uid:
        die("record-pr: live task Issue UID mismatch")
    if str(live_issue.get("issue_state") or "").strip().upper() != "OPEN":
        die("record-pr: live task Issue is not OPEN")
    expected_status = "committed" if bool(getattr(args, "draft_candidate", False)) else "pr_watch"
    expected_phase = "verification" if bool(getattr(args, "draft_candidate", False)) else "pr_watch"
    if bool(getattr(args, "existing_ready_update", False)):
        expected_status, expected_phase = record.get("status"), record.get("workflow_phase")
    expected_url = f"https://github.com/{args.repo}/pull/{pr_number}"
    exact_publication_poststate = allow_exact_publication_poststate and all(
        live_issue.get(key) == value
        for key, value in {
            "status": expected_status,
            "workflow_phase": expected_phase,
            "pr_url": expected_url,
        }.items()
    ) and same_pr_number(live_issue.get("pr_number"), pr_number)
    cached_projection_matches = all(
        live_issue.get(key) == record.get(key)
        for key in ("status", "workflow_phase", "pr_url")
    ) and same_pr_number(live_issue.get("pr_number"), record.get("pr_number"))
    if allow_exact_publication_poststate and not exact_publication_poststate and not cached_projection_matches:
        die("record-pr: live Task Issue is neither cached truth nor the exact publication poststate")
    for key in (
        "issue_number", "issue_url", "owner_role", "module", "priority",
        "status", "workflow_phase", "worktree_hint",
    ):
        if live_issue.get(key) != record.get(key):
            if exact_publication_poststate and key in {"status", "workflow_phase"}:
                continue
            die(f"record-pr: live task Issue {key} differs from cached task truth")
    for key in ("pr_url", "pr_number"):
        matches = (
            same_pr_number(live_issue.get(key), record.get(key))
            if key == "pr_number"
            else live_issue.get(key) == record.get(key)
        )
        if not matches:
            if exact_publication_poststate:
                continue
            die(f"record-pr: live task Issue {key} differs from cached PR binding")

    if record.get("task_uid") != args.task_uid or str(record.get("repository") or "") != args.repo:
        die("record-pr: cached task repository identity is missing or mismatched")
    worktree = str(record.get("canonical_worktree") or "")
    if not worktree or str(record.get("worktree_hint") or "") != worktree:
        die("record-pr: canonical task worktree identity is missing or mismatched")
    try:
        identity = authoritative_repository_identity(args.root.resolve(), args.repo, worktree)
    except (OSError, subprocess.CalledProcessError, RuntimeError, ValueError) as exc:
        die(f"record-pr: canonical task repository identity readback failed: {exc}")
    for key in ("repository", "canonical_worktree", "task_branch", "default_branch"):
        if not record.get(key) or str(record.get(key)) != identity.get(key):
            label = "branch" if key == "task_branch" else key
            die(f"record-pr: canonical task {label} identity mismatch")
    try:
        canonical_head = run_text([
            "git", "-C", identity["canonical_worktree"], "rev-parse", "--verify", "HEAD^{commit}",
        ])
    except (OSError, subprocess.CalledProcessError, RuntimeError) as exc:
        die(f"record-pr: canonical task HEAD readback failed: {exc}")
    if not re.fullmatch(r"[0-9a-fA-F]{40,64}", canonical_head):
        die("record-pr: canonical task HEAD identity is malformed")
    if bool(getattr(args, "existing_ready_update", False)) and publication_intent is not None:
        if publication_module is None:
            die("record-pr: existing ready update requires validated C1 publication evidence")
        if (
            publication_intent.get("repository") != args.repo
            or publication_intent.get("task_uid") != args.task_uid
            or publication_intent.get("source_repository_id") != publication_intent.get("repository_id")
            or publication_intent.get("source_ref") != identity["task_branch"]
            or publication_intent.get("target_ref") != identity["default_branch"]
            or str(publication_intent.get("source_head_oid") or "").casefold() != canonical_head.casefold()
        ):
            die("record-pr: C1 publication intent differs from current canonical task identity")

    try:
        live_pr = github_pull_request(args.repo, pr_number)
    except (OSError, subprocess.CalledProcessError, json.JSONDecodeError, RuntimeError) as exc:
        die(f"record-pr: live PR readback failed: {exc}")
    expected_url = f"https://github.com/{args.repo}/pull/{pr_number}"
    if type(live_pr.get("number")) is not int or live_pr.get("number") != pr_number:
        die("record-pr: live PR number does not match requested PR")
    if str(live_pr.get("html_url") or "").casefold() != expected_url.casefold():
        die("record-pr: live PR URL does not match requested repository and number")
    if str(live_pr.get("state") or "").casefold() != "open" or live_pr.get("merged_at") is not None:
        die("record-pr: live PR is stale, closed, or merged")
    head = live_pr.get("head")
    base = live_pr.get("base")
    if not isinstance(head, dict) or not isinstance(base, dict):
        die("record-pr: live PR head/base identity is malformed")
    head_repo = head.get("repo")
    base_repo = base.get("repo")
    if (not isinstance(head_repo, dict)
            or str(head_repo.get("full_name") or "").casefold() != args.repo.casefold()):
        die("record-pr: live PR head repository does not match task repository")
    if (not isinstance(base_repo, dict)
            or str(base_repo.get("full_name") or "").casefold() != args.repo.casefold()):
        die("record-pr: live PR base repository does not match task repository")
    if str(head.get("ref") or "") != identity["task_branch"]:
        die("record-pr: live PR branch does not match canonical task branch")
    if str(base.get("ref") or "") != identity["default_branch"]:
        die("record-pr: live PR base branch does not match canonical repository default branch")
    if str(head.get("sha") or "").casefold() != canonical_head.casefold():
        die("record-pr: live PR head does not match canonical task HEAD")
    if type(live_pr.get("draft")) is not bool or live_pr.get("draft") != bool(getattr(args, "draft_candidate", False)):
        die("record-pr: live PR draft state does not match requested task transition")
    if bool(getattr(args, "existing_ready_update", False)):
        if not isinstance(publication_intent, dict) or publication_module is None:
            die("record-pr: existing ready update requires validated C1 publication evidence")
        if not has_exact_task_pr_linkage(
            live_pr.get("body"), args.task_uid, int(record.get("issue_number") or 0),
        ):
            die("record-pr: live ready-update PR lacks exact unique Task/Refs identity")
        try:
            live_projection = publication_module.decode_marker(live_pr.get("body"))
        except (TypeError, ValueError) as exc:
            die(f"record-pr: live ready-update PR projection marker is invalid: {exc}")
        if (
            live_projection.get("task_uid") != publication_intent.get("task_uid")
            or str(live_projection.get("source_head_oid") or "").casefold()
                != str(publication_intent.get("source_head_oid") or "").casefold()
            or live_projection.get("scope_base_oid") != publication_intent.get("source_scope_oid")
            or live_projection.get("projection_digest") != publication_intent.get("projection_digest")
        ):
            die("record-pr: live ready-update PR projection differs from C1 publication intent")
    return live_issue


def task_from_record(uid: str, record: dict[str, Any]) -> OrderedDict[str, Any]:
    task = OrderedDict(
        [
            ("task_uid", uid),
            ("title", record.get("title") or ""),
            ("owner_role", record.get("owner_role") or ""),
            ("module", record.get("module") or ""),
            ("worktree_hint", record.get("worktree_hint") or ""),
            ("status", record.get("status") or "candidate"),
            ("workflow_phase", record.get("workflow_phase") or ""),
            ("priority", record.get("priority") or "P2"),
            ("source_signal", record.get("source_signal") or ""),
            ("source_type", record.get("source_type") or ""),
            ("severity", record.get("severity") or ""),
            ("pr_url", record.get("pr_url") or record.get("pull_request_url") or ""),
            ("pr_number", record.get("pr_number") or ""),
            ("merge_hold", record.get("merge_hold") or {}),
            ("loop_binding", record.get("loop_binding")),
            ("bootstrap_base_oid", record.get("bootstrap_base_oid")),
            ("completion_mode", record.get("completion_mode") or ""),
            ("aggregate_plan_comment_id", record.get("aggregate_plan_comment_id") or ""),
            ("aggregate_plan_sha256", record.get("aggregate_plan_sha256") or ""),
            ("aggregate_completion_receipt_sha256", record.get("aggregate_completion_receipt_sha256") or ""),
            ("traceability_mode", record.get("traceability_mode")),
            ("coordination_ref", record.get("coordination_ref")),
            ("traceability_record", record.get("traceability_record")),
            ("coordination_record", record.get("coordination_record")),
            ("traceability_candidate", record.get("traceability_candidate")),
            ("aggregate_candidate", record.get("aggregate_candidate")),
            ("non_pr_completion_evidence", record.get("non_pr_completion_evidence") or ""),
            ("non_pr_completion_evidence_file", record.get("non_pr_completion_evidence_file") or ""),
            ("non_pr_completion_evidence_sha256", record.get("non_pr_completion_evidence_sha256") or ""),
            ("last_closed_at", record.get("last_closed_at") or ""),
            ("claim_verifications", record.get("claim_verifications") or []),
            ("source_refs", record.get("source_refs") or []),
            ("doc_refs", record.get("doc_refs") or []),
            ("related_prd", record.get("related_prd") or []),
            ("acceptance", record.get("acceptance") or []),
            ("updated_at", record.get("updated_at") or now()),
        ]
    )
    if record.get("primary_package") not in (None, ""):
        try:
            task["primary_package"] = validate_primary_package(str(record["primary_package"]))
        except ValueError as exc:
            die(str(exc))
    return task


def issue_body(task: OrderedDict[str, Any]) -> str:
    lines = [
        "<!-- oasis7-pm-task -->",
        f"task_uid: {task['task_uid']}",
        "",
        "GitHub-backed oasis7 PM task.",
        "",
        "Task metadata:",
        f"- owner_role: `{task.get('owner_role')}`",
        f"- module: `{task.get('module') or ''}`",
        f"- status: `{task.get('status')}`",
        f"- workflow_phase: `{task.get('workflow_phase') or ''}`",
        f"- priority: `{task.get('priority')}`",
        f"- worktree_hint: `{task.get('worktree_hint') or ''}`",
    ]
    if task.get("primary_package") not in (None, ""):
        lines.append(f"- primary_package: `{validate_primary_package(str(task['primary_package']))}`")
    if task.get("source_signal") or task.get("source_type") or task.get("severity"):
        lines.extend(
            [
                f"- source_signal: `{task.get('source_signal') or ''}`",
                f"- source_type: `{task.get('source_type') or ''}`",
                f"- severity: `{task.get('severity') or ''}`",
            ]
        )
    if task.get("loop_binding") is not None:
        encoded = base64.urlsafe_b64encode(json.dumps(task["loop_binding"], sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()).decode().rstrip("=")
        lines.append(f"- loop_binding_b64: `{encoded}`")
        if task.get("bootstrap_base_oid"):
            lines.append(f"- bootstrap_base_oid: `{task['bootstrap_base_oid']}`")
    traceability_context = {
        key: task[key]
        for key in (
            "traceability_mode", "coordination_ref", "traceability_record",
            "coordination_record", "traceability_candidate", "aggregate_candidate",
        )
        if task.get(key) is not None
    }
    if traceability_context:
        encoded = base64.urlsafe_b64encode(json.dumps(
            traceability_context, sort_keys=True, separators=(",", ":"), ensure_ascii=False,
        ).encode("utf-8")).decode("ascii").rstrip("=")
        lines.append(f"- traceability_context_b64: `{encoded}`")
    if task.get("pr_url"):
        lines.append(f"- pr_url: `{task.get('pr_url')}`")
    if task.get("pr_number"):
        lines.append(f"- pr_number: `{task.get('pr_number')}`")
    if task.get("completion_mode"):
        lines.append(f"- completion_mode: `{task.get('completion_mode')}`")
        if task.get("aggregate_plan_comment_id"):
            lines.append(f"- aggregate_plan_comment_id: `{task.get('aggregate_plan_comment_id')}`")
        if task.get("aggregate_plan_sha256"):
            lines.append(f"- aggregate_plan_sha256: `{task.get('aggregate_plan_sha256')}`")
        if task.get("aggregate_completion_receipt_sha256"):
            lines.append(f"- aggregate_completion_receipt_sha256: `{task.get('aggregate_completion_receipt_sha256')}`")
        evidence = str(task.get("non_pr_completion_evidence") or "").encode("utf-8")
        encoded = base64.urlsafe_b64encode(evidence).decode("ascii").rstrip("=")
        lines.append(f"- non_pr_completion_evidence_b64: `{encoded}`")
        if task.get("non_pr_completion_evidence_sha256"):
            lines.append(f"- non_pr_completion_evidence_sha256: `{task.get('non_pr_completion_evidence_sha256')}`")
    if task.get("last_closed_at"):
        lines.append(f"- last_closed_at: `{task.get('last_closed_at')}`")
    claims = task.get("claim_verifications") or []
    if claims:
        encoded_claims = base64.urlsafe_b64encode(
            json.dumps(claims, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
        ).decode("ascii").rstrip("=")
        lines.append(f"- claim_verifications_b64: `{encoded_claims}`")
    if task.get("merge_hold"):
        hold = task["merge_hold"]
        for key in ("kind", "requester", "reason", "resume_authority", "active"):
            value = str(hold.get(key, "")).lower() if key == "active" else hold.get(key, "")
            lines.append(f"- merge_hold_{key}: `{value}`")
    source_refs = task.get("source_refs") or []
    if source_refs:
        lines.append("")
        lines.append("Source refs:")
        for ref in source_refs:
            lines.append(f"- `{ref}`")
    doc_refs = sorted({str(ref) for ref in (task.get("doc_refs") or [])})
    if doc_refs:
        lines.append("")
        lines.append("Doc refs:")
        for ref in doc_refs:
            lines.append(f"- `{ref}`")
    related_prd = sorted({str(ref) for ref in (task.get("related_prd") or [])})
    if related_prd:
        lines.append("")
        lines.append("Related PRD:")
        for ref in related_prd:
            lines.append(f"- `{ref}`")
    acceptance = task.get("acceptance") or []
    if acceptance:
        lines.append("")
        lines.append("Acceptance:")
        for item in acceptance:
            lines.append(f"- {item}")
    return "\n".join(lines) + "\n"


def create_issue(repo: str, task: OrderedDict[str, Any], before_write=None) -> str:
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as handle:
        handle.write(issue_body(task))
        body_path = handle.name
    try:
        if before_write is not None:
            before_write()
        return run_text(["gh", "issue", "create", "-R", repo, "--title", f"[PM] {task['title']}", "--body-file", body_path])
    finally:
        pathlib.Path(body_path).unlink(missing_ok=True)


def update_issue_body(repo: str, issue_number: int, task: OrderedDict[str, Any]) -> None:
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as handle:
        handle.write(issue_body(task))
        body_path = handle.name
    try:
        run_text(["gh", "issue", "edit", str(issue_number), "-R", repo, "--body-file", body_path])
    finally:
        pathlib.Path(body_path).unlink(missing_ok=True)


def issue_comment(repo: str, issue_number: int, body: str) -> str:
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as handle:
        handle.write(body)
        body_path = handle.name
    try:
        return run_text(["gh", "issue", "comment", str(issue_number), "-R", repo, "--body-file", body_path])
    finally:
        pathlib.Path(body_path).unlink(missing_ok=True)


def verified_issue_comment(repo: str, issue_number: int, body: str) -> str:
    """Write an Issue comment and prove GitHub returned the exact body."""
    comment_url = issue_comment(repo, issue_number, body)
    match = re.search(r"issuecomment-(\d+)(?:$|[?#])", comment_url)
    if not match:
        die("classify-non-pr-task: issue comment URL has no comment identity")
    try:
        readback = json.loads(run_text(["gh", "api", f"repos/{repo}/issues/comments/{match.group(1)}"]))
    except (subprocess.SubprocessError, json.JSONDecodeError) as exc:
        die(f"classify-non-pr-task: issue comment readback failed: {exc}")
    if str(readback.get("body") or "") != body:
        die("classify-non-pr-task: issue comment readback mismatch")
    return comment_url


def evidence_body(task_uid: str, role: str, phase: str, fields: dict[str, Any]) -> str:
    lines = [
        "<!-- oasis7-pm-evidence -->",
        f"Task UID: {task_uid}",
        f"Evidence Phase: {phase}",
        f"Role: {role}",
        f"Recorded At: {now()}",
        "",
    ]
    for key, value in fields.items():
        if isinstance(value, list):
            value = ", ".join(str(item) for item in value)
        lines.append(f"{key}: {value}")
    return "\n".join(lines) + "\n"


def update_project_fields(
    args: argparse.Namespace,
    task: OrderedDict[str, Any],
    project_item_id: str,
    *,
    require_lifecycle_projection: bool = False,
) -> int:
    sync = load_sync_module()
    project_id, fields = sync.project_context(args.project_owner, args.project_number)
    if task.get("loop_binding"):
        values = sync.project_field_values(task)
        for name in ("Loop", "Change ID"):
            field = fields.get(name)
            if not field or (name in sync.SINGLE_SELECT_FIELDS and values[name] not in field.get("options_by_name", {})):
                die(f"loop Project projection unavailable: provision {name} field/options through the authorized Project setup path, then retry")
    if require_lifecycle_projection:
        values = sync.project_field_values(task)
        missing: list[str] = []
        for field_name in ("Status", "PM Status", "Workflow Phase"):
            value = str(values.get(field_name) or "")
            field = fields.get(field_name)
            if not field:
                missing.append(f"{field_name}:missing_field")
            elif not value:
                missing.append(f"{field_name}:empty_value")
            elif field_name in sync.SINGLE_SELECT_FIELDS and value not in (field.get("options_by_name") or {}):
                missing.append(f"{field_name}:missing_option:{value}")
        if missing:
            die(
                "project lifecycle projection is unavailable; refusing to leave local task truth ahead of GitHub Project: "
                + ", ".join(missing)
                + "; add the missing Project field/options, then rerun "
                + f"./scripts/pm/refresh-task-cache.sh --task-uid {args.task_uid} --json and the same lifecycle command"
            )
    updated, skipped = sync.update_fields(project_id, project_item_id, task, fields)
    if task.get("loop_binding") and any(item.split(":", 1)[0] in {"Loop", "Change ID"} and not item.endswith(":unchanged") for item in skipped):
        die("loop Project projection incomplete; reconcile before retry")
    # The shell integration harness uses a fake GitHub transport. Production
    # task records always require authoritative Project readback.
    if task.get("loop_binding") and not os.environ.get("OASIS7_PM_FAKE_GITHUB"):
        expected = sync.project_field_values(task)
        try:
            live_values = sync.read_project_item_field_values(project_id, project_item_id)
        except Exception as exc:
            die("start outcome uncertain: loop Project projection readback unavailable; reconcile before retry: " + str(exc))
        mismatches = [
            f"{name}={live_values.get(name)!r}, expected={value!r}"
            for name, value in expected.items()
            if name in {"Loop", "Change ID"} and live_values.get(name) != value
        ]
        if mismatches:
            die("loop Project projection readback mismatch; reconcile before retry: " + "; ".join(mismatches))
    if skipped:
        print(f"github-project-task: skipped fields: {', '.join(skipped)}", file=sys.stderr)
    if require_lifecycle_projection:
        unresolved = [
            item for item in skipped
            if item.split(":", 1)[0] in {"Status", "PM Status", "Workflow Phase"}
            and not item.endswith(":unchanged")
        ]
        if unresolved:
            die(
                "project lifecycle projection was not fully updated; refusing local/Project phase drift: "
                + ", ".join(unresolved)
                + "; rerun the same lifecycle command after the Project fields are available"
            )
    return int(updated)


def update_pr_project_field(args: argparse.Namespace, task: OrderedDict[str, Any], project_item_id: str) -> int:
    sync = load_sync_module()
    project_id, fields = sync.project_context(args.project_owner, args.project_number)
    updated, skipped = sync.update_fields(project_id, project_item_id, task, fields, only_fields={"PR"})
    if skipped:
        print(f"github-project-task: skipped draft PR field: {', '.join(skipped)}", file=sys.stderr)
    if int(updated) != 1:
        die(f"record-pr: refusing draft candidate because Project PR field was not updated: updated={updated}/1")
    return int(updated)


def update_done_project_fields(args: argparse.Namespace, task: OrderedDict[str, Any], project_item_id: str) -> int:
    sync = load_sync_module()
    project_id, fields = sync.project_context(args.project_owner, args.project_number)
    required_fields = {"Status", "PM Status", "Workflow Phase"}
    values = sync.project_field_values(task)
    missing: list[str] = []
    for field_name in sorted(required_fields):
        value = str(values.get(field_name) or "")
        field = fields.get(field_name)
        if not field:
            missing.append(f"{field_name}:missing_field")
            continue
        if not value:
            missing.append(f"{field_name}:empty_value")
            continue
        if field_name in sync.SINGLE_SELECT_FIELDS and value not in (field.get("options_by_name") or {}):
            missing.append(f"{field_name}:missing_option:{value}")
    if missing:
        die(
            "move-task: refusing done because required GitHub Project fields are unavailable: "
            + ", ".join(missing)
        )
    updated, skipped = sync.update_fields(project_id, project_item_id, task, fields, only_fields=required_fields)
    if skipped:
        print(f"github-project-task: skipped done fields: {', '.join(skipped)}", file=sys.stderr)
    if int(updated) != len(required_fields):
        die(
            "move-task: refusing done because required GitHub Project fields were not updated: "
            f"updated={updated}/{len(required_fields)}"
        )
    return int(updated)


def add_project_item(args: argparse.Namespace, issue_url: str) -> str:
    payload = json.loads(
        run_text(
            [
                "gh",
                "project",
                "item-add",
                str(args.project_number),
                "--owner",
                args.project_owner,
                "--url",
                issue_url,
                "--format",
                "json",
            ]
        )
    )
    item_id = str(payload.get("id") or "")
    if not item_id:
        die("gh project item-add returned no item id")
    return item_id


def validate_loop_binding(binding: Any) -> dict[str, Any]:
    path = pathlib.Path(__file__).with_name("loop_policy.py")
    spec = importlib.util.spec_from_file_location("loop_policy", path)
    if spec is None or spec.loader is None:
        die("loop policy unavailable")
    policy = importlib.util.module_from_spec(spec)
    sys.path.insert(0, str(path.parent))
    try:
        spec.loader.exec_module(policy)
        verdict = policy.validate_binding(binding)
    finally:
        sys.path.pop(0)
    if verdict.get("status") != "passed":
        die("invalid loop binding: " + str(verdict.get("blockers")))
    return binding


def loop_lineage_path(root: pathlib.Path, task_uid: str) -> pathlib.Path:
    if not re.fullmatch(r"task_[0-9a-f]{32}", task_uid):
        die("invalid task UID for loop lineage")
    common = pathlib.Path(run_text(["git", "-C", str(root), "rev-parse", "--git-common-dir"]))
    return (root / common).resolve() / "oasis7-loop-lineage" / (task_uid + ".json")


def validate_loop_inputs(root: pathlib.Path, binding: dict[str, Any], repository: str, purpose: str) -> None:
    """Admission of selected manual inputs; never scan unrelated tasks."""
    tool_root = pathlib.Path(__file__).resolve().parents[2]
    trusted_import_files = (
        "scripts/document_corpus.py",
        "scripts/product-doc-content-check.py",
        "scripts/product_doc_markdown.py",
    )
    commit = binding.get("policy_commit", "")
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        die("manual admission requires immutable policy commit")
    if run_text(["git", "-C", str(tool_root), "rev-parse", "HEAD"]) != commit:
        die("manual admission helper is not running from pinned effective policy")
    run_text(["git", "-C", str(tool_root), "diff", "--no-ext-diff", "--no-textconv", "--exit-code", commit, "--", "scripts/pm", *trusted_import_files])
    shadows = run_text(["git", "-C", str(tool_root), "ls-files", "--others", "--", "scripts/pm", *trusted_import_files, ":(exclude)**/__pycache__/**"])
    if any(path.endswith((".py", ".sh", ".json")) for path in shadows.splitlines()):
        die("untracked executable authority in manual helper root")
    if run_text(["git", "-C", str(tool_root), "rev-parse", "--path-format=absolute", "--git-common-dir"]) != run_text(["git", "-C", str(root), "rev-parse", "--path-format=absolute", "--git-common-dir"]):
        die("manual helper root belongs to another repository")

    def load_pinned_module(name: str):
        relative = f"scripts/pm/{name}.py"
        entries = run_text(["git", "-C", str(tool_root), "ls-tree", commit, "--", relative]).splitlines()
        if len(entries) != 1 or "\t" not in entries[0]:
            die("effective helper module missing or ambiguous: " + relative)
        metadata, recorded_path = entries[0].split("\t", 1)
        mode, object_type, _oid = metadata.split()
        path = tool_root / relative
        if (recorded_path != relative or mode != "100644" or object_type != "blob"
                or path.is_symlink() or not path.resolve().is_relative_to(tool_root.resolve())
                or path.read_bytes() != subprocess.check_output(["git", "-C", str(tool_root), "show", commit + ":" + relative])):
            die("effective helper bytes or mode differ: " + relative)
        if run_text(["git", "-C", str(tool_root), "ls-files", "--others", "--", relative]):
            die("untracked effective helper shadow: " + relative)
        spec = importlib.util.spec_from_file_location(name, path)
        if spec is None or spec.loader is None:
            die("effective helper module unavailable: " + relative)
        module = importlib.util.module_from_spec(spec)
        sys.modules[name] = module
        spec.loader.exec_module(module)
        return module

    sys.dont_write_bytecode = True
    # loop_policy imports loop_contracts by name; replace any preloaded module
    # with the exact pinned helper before executing the policy bytes.
    loop_contracts = load_pinned_module("loop_contracts")
    loop_policy = load_pinned_module("loop_policy")
    for result in (loop_policy.validate_tool_root(tool_root, root, binding),
                   loop_contracts.validate_contracts(tool_root, root, binding, purpose=purpose)):
        if result.get("status") != "passed":
            die("manual input admission blocked: " + str(result.get("blockers")))
    bindings = {binding["task_uid"]: binding}
    pending = list(binding["dependencies"])
    while pending:
        uid = pending.pop()
        if uid in bindings:
            continue
        if len(bindings) >= 64:
            die("selected dependency closure exceeds 64 tasks; narrow the dependency contract")
        live = github_issue_record(repository, uid)
        require_loop_dependency_ready(live or {}, uid, repository, root)
        dependency = (live or {}).get("loop_binding")
        if not isinstance(dependency, dict):
            die("selected dependency binding unavailable: " + uid)
        binding_result = loop_policy.validate_binding(dependency)
        if binding_result.get("status") != "passed":
            die("invalid selected dependency binding: " + str(binding_result.get("blockers")))
        bindings[uid] = dependency
        pending.extend(dependency["dependencies"])
    result = loop_policy.validate_dependencies(binding, bindings)
    if result.get("status") != "passed":
        die("manual dependency admission blocked: " + str(result.get("blockers")))


def require_loop_dependency_ready(live: dict[str, Any], task_uid: str, repository: str = DEFAULT_REPO,
                                  repo_root: pathlib.Path | None = None) -> None:
    """A delivery dependency needs the canonical merged terminal, not mere closure."""
    from loop_terminal import validate_terminal_delivery
    if repo_root is None:
        result = validate_terminal_delivery(repository, task_uid, live.get("issue_number"))
    else:
        result = validate_terminal_delivery(repository, task_uid, live.get("issue_number"), repo_root=repo_root)
    if result.get("status") != "passed":
        die("selected dependency has not completed merged delivery: " + task_uid + ": " + str(result.get("blockers")))


def record_loop_lineage(root: pathlib.Path, binding: dict[str, Any], *, migrate: bool = False) -> None:
    path = loop_lineage_path(root, binding["task_uid"])
    with durable_store.locked_json(path, {}) as lineage:
        previous = lineage.get("loop_binding")
        if previous and previous != binding:
            if not migrate or binding["bootstrap_epoch"] != previous["bootstrap_epoch"] + 1:
                die("loop lineage drift; explicit epoch reconciliation required")
            lineage.setdefault("previous_epochs", []).append(previous)
        lineage["loop_binding"] = binding


def ensure_loop_history(repository: str, issue_number: int, binding: dict[str, Any]) -> None:
    digest = hashlib.sha256(json.dumps(binding, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
    marker = f"<!-- oasis7-loop-binding-history task_uid={binding['task_uid']} epoch={binding['bootstrap_epoch']} digest={digest} -->"
    body = marker + "\nImmutable loop binding history; removing current metadata does not restore legacy admission.\n"
    pages = json.loads(run_text(["gh", "api", f"repos/{repository}/issues/{issue_number}/comments", "--paginate", "--slurp"]))
    comments = [item for page in pages for item in (page if isinstance(page, list) else [page])]
    matching = [item for item in comments if marker in str(item.get("body") or "")]
    if matching:
        if len(matching) != 1 or matching[0].get("body") != body:
            die("loop binding history is ambiguous; reconcile selected task")
        return
    url = issue_comment(repository, issue_number, body)
    match = re.search(r"#issuecomment-(\d+)$", url)
    if not match:
        die("loop binding history comment identity uncertain; reconcile before retry")
    readback = json.loads(run_text(["gh", "api", f"repos/{repository}/issues/comments/{match.group(1)}"]))
    if readback.get("body") != body:
        die("loop binding history readback mismatch")


def check_loop_binding_update(record: dict[str, Any], binding: dict[str, Any], migrate_epoch: int | None) -> None:
    previous = record.get("loop_binding")
    if previous == binding:
        return
    current_epoch = (previous or {}).get("bootstrap_epoch", record.get("bootstrap_epoch", 1))
    if migrate_epoch != current_epoch + 1 or binding.get("bootstrap_epoch") != migrate_epoch:
        die("loop/input/scope binding change requires explicit next --migrate-epoch")
    if record.get("owner_role") and record["owner_role"] != binding.get("owner_role"):
        die("loop binding owner must match the existing task owner")


def command_bind_loop(args: argparse.Namespace) -> int:
    mapping_path, _, record = require_record(args)
    binding = json.loads(pathlib.Path(args.loop_binding).read_text())
    if not isinstance(binding, dict):
        die("loop binding must be an object")
    if binding["task_uid"] != args.task_uid:
        die("loop binding task UID mismatch")
    if not args.manual_request_ref.strip():
        die("manual request reference required")
    live = github_issue_record(args.repo, args.task_uid)
    if not live:
        die("cannot bind without live task Issue readback")
    for field in ("issue_number", "owner_role", "worktree_hint"):
        if live.get(field) != record.get(field):
            die(f"loop bind live identity drift: {field}")
    check_loop_binding_update(live, binding, args.migrate_epoch)
    validate_loop_inputs(args.root.resolve(), binding, args.repo, "in_flight" if live.get("loop_binding") == binding else "new_tasks")
    if live.get("status") in {"done", "deferred"} or live.get("workflow_phase") in TERMINAL_WORKFLOW_PHASES:
        die("terminal task cannot be rebound")
    updated = {**record, **live, "loop_binding": binding, "bootstrap_epoch": binding["bootstrap_epoch"]}
    if args.migrate_epoch:
        snapshot_path = pathlib.Path(record["canonical_worktree"]) / ".pm/scratch" / args.task_uid / "bootstrap-task-snapshot.json"
        source = pathlib.Path(__file__).with_name("bootstrap-task-snapshot.py")
        spec = importlib.util.spec_from_file_location("bootstrap_snapshot", source)
        snapshot = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(snapshot)
        try:
            saved = json.loads(snapshot_path.read_text())
            if saved.get("digest") != snapshot.digest(saved):
                die("cannot migrate corrupt bootstrap snapshot")
            old_epoch = saved["task"].get("bootstrap_epoch", 1)
            if old_epoch not in (binding["bootstrap_epoch"] - 1, binding["bootstrap_epoch"]):
                die("bootstrap snapshot epoch gap; reconcile existing task")
            if saved["task"]["uid"] != args.task_uid:
                die("bootstrap snapshot task identity mismatch")
            if old_epoch == binding["bootstrap_epoch"] and (live.get("loop_binding") != binding or saved["task"].get("loop_binding") != binding):
                die("bootstrap snapshot binding does not match migrated live task")
            saved["request"]["identity"]
            snapshot_base = saved["git"]["base"]["oid"]
            if updated.get("bootstrap_base_oid") and updated["bootstrap_base_oid"] != snapshot_base:
                die("bootstrap snapshot base differs from cached immutable base")
            archive = snapshot_path.with_name(f"bootstrap-task-snapshot.epoch-{old_epoch}.json")
            if old_epoch != binding["bootstrap_epoch"] and archive.exists() and json.loads(archive.read_text()) != saved:
                die("bootstrap snapshot epoch archive mismatch")
        except (OSError, ValueError, KeyError, TypeError) as error:
            die("bootstrap snapshot migration preflight failed: " + str(error))
    if not updated.get("bootstrap_base_oid"):
        snapshot_path = pathlib.Path(record["canonical_worktree"]) / ".pm/scratch" / args.task_uid / "bootstrap-task-snapshot.json"
        try:
            updated["bootstrap_base_oid"] = json.loads(snapshot_path.read_text())["git"]["base"]["oid"]
        except (OSError, ValueError, KeyError):
            die("existing task binding needs its immutable bootstrap snapshot base")
    cleared_traceability = synchronize_live_issue_traceability(
        args.repo, args.task_uid, updated, live=live,
        explicit_updates=frozenset({"loop_binding"}),
    )
    task = task_from_record(args.task_uid, updated)
    # The facade's inherited OS reservation covers preflight and this writer.
    # Register only at the mutation boundary; a preflight error is not an
    # uncertain remote operation. Recovery already has its original intent.
    action_json = getattr(args, "loop_action_json", None)
    action = json.loads(action_json) if action_json else None
    if action is not None and (action.get("kind") != "bind_loop" or action.get("expected") != json.dumps(binding, sort_keys=True)):
        die("bind mutation intent differs from the requested binding")
    intent_written = False
    def before_write():
        nonlocal intent_written
        if action is not None and not intent_written:
            from loop_recovery import common_dir, record_action
            record_action(common_dir(args.root.resolve()), args.task_uid, action)
            intent_written = True
    if live.get("loop_binding") != binding:
        before_write()
        update_issue_body(args.repo, int(record["issue_number"]), task)
    readback = github_issue_record(args.repo, args.task_uid)
    if not readback or readback.get("loop_binding") != binding:
        die("loop binding Issue write/readback uncertain; reconcile before retry")
    before_write()
    update_project_fields(args, task, str(record["project_item_id"]))
    ensure_loop_history(args.repo, int(record["issue_number"]), binding)
    merge_task_mapping(mapping_path, args.task_uid, updated, clear_keys=cleared_traceability)
    if args.migrate_epoch:
        snapshot_path = pathlib.Path(record["canonical_worktree"]) / ".pm/scratch" / args.task_uid / "bootstrap-task-snapshot.json"
        saved = json.loads(snapshot_path.read_text())
        old_epoch = saved.get("task", {}).get("bootstrap_epoch", 1)
        if old_epoch != binding["bootstrap_epoch"]:
            if old_epoch + 1 != binding["bootstrap_epoch"]:
                die("bootstrap snapshot epoch gap; reconcile existing task")
            source = pathlib.Path(__file__).with_name("bootstrap-task-snapshot.py")
            spec = importlib.util.spec_from_file_location("bootstrap_snapshot", source)
            snapshot = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(snapshot)
            if saved.get("digest") != snapshot.digest(saved):
                die("cannot migrate corrupt bootstrap snapshot")
            archive = snapshot_path.with_name(f"bootstrap-task-snapshot.epoch-{old_epoch}.json")
            if archive.exists() and json.loads(archive.read_text()) != saved:
                die("bootstrap snapshot epoch archive mismatch")
            atomic_json(archive, saved)
            replacement = snapshot.live_payload(pathlib.Path(record["canonical_worktree"]), mapping_path, args.task_uid, saved["request"]["identity"])
            replacement.update(producer="scripts/pm/github-project-task.py bind-loop", created_at=now())
            replacement["digest"] = snapshot.digest(replacement)
            atomic_json(snapshot_path, replacement)
    record_loop_lineage(args.root.resolve(), binding, migrate=bool(args.migrate_epoch))
    payload = {"status": "bound", "task_uid": args.task_uid, "loop_binding": binding,
               "manual_request_ref": args.manual_request_ref, "issue_url": record["issue_url"]}
    print(json.dumps(payload, sort_keys=True) if args.json else f"bind-loop: {args.task_uid}")
    return 0


def command_new_task(args: argparse.Namespace) -> int:
    request_key = getattr(args, "request_key", None)
    if not request_key:
        return _command_new_task(args)
    common = pathlib.Path(run_text(["git", "-C", str(args.root), "rev-parse", "--git-common-dir"]))
    common = (args.root / common).resolve()
    key = hashlib.sha256(request_key.encode()).hexdigest()
    with durable_store.locked_json(common / "oasis7-bootstrap-writers" / (key + ".json"), {}):
        return _command_new_task(args)


def _command_new_task(args: argparse.Namespace) -> int:
    root = args.root.resolve()
    mapping_path = mapping_path_for(root, args.mapping)
    repository_identity = authoritative_repository_identity(root, args.repo, args.worktree_hint or str(root))
    primary_package = getattr(args, "primary_package", None)
    if primary_package not in (None, ""):
        try:
            primary_package = validate_primary_package(primary_package)
        except ValueError as exc:
            die(str(exc))
    immutable_request = OrderedDict(
        [
            ("repo", args.repo),
            ("title", args.title),
            ("owner_role", args.owner_role),
            ("worktree_hint", args.worktree_hint or ""),
            ("module", args.module or ""),
            ("priority", args.priority),
            ("source_refs", sorted(args.source_ref or [])),
            ("acceptance", list(args.acceptance or [])),
            ("source_signal", args.source_signal or ""),
            ("source_type", args.source_type or ""),
            ("severity", args.severity or ""),
            ("doc_refs", sorted(args.doc_ref or [])),
            ("related_prd", sorted(args.related_prd or [])),
            ("handoff_to", sorted(args.handoff_to or [])),
        ]
    )
    if primary_package:
        immutable_request["primary_package"] = primary_package
    binding_path = getattr(args, "loop_binding", None)
    binding = json.loads(pathlib.Path(binding_path).read_text()) if binding_path else None
    if binding_path and (not isinstance(binding, dict) or not binding):
        die("loop binding must be an object")
    request_key = getattr(args, "request_key", None)
    if binding:
        if binding["owner_role"] != args.owner_role or binding["request_key"] != request_key:
            die("loop binding owner/request key mismatch")
        immutable_request["loop_binding"] = binding
        immutable_request["request_key"] = request_key
        base_oid = getattr(args, "bootstrap_base_oid", None)
        if not base_oid or not re.fullmatch(r"[0-9a-f]{40}", base_oid):
            die("manual loop bootstrap requires --bootstrap-base-oid fixed commit")
        immutable_request["bootstrap_base_oid"] = base_oid
    immutable_digest = hashlib.sha256(
        json.dumps(immutable_request, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
    ).hexdigest()
    journal_key = hashlib.sha256(
        ("\0".join((args.repo, "manual-request", request_key)) if request_key else
         "\0".join((args.repo, args.title, args.owner_role, args.worktree_hint or ""))).encode()
    ).hexdigest()
    test_scratch = os.environ.get("OASIS7_PM_TEST_SCRATCH", "")
    if test_scratch:
        scratch_root = pathlib.Path(test_scratch)
        if not scratch_root.is_absolute():
            die("OASIS7_PM_TEST_SCRATCH must be absolute")
        journal_root = scratch_root / "bootstrap-journal"
    else:
        journal_root = root / ".pm/scratch/bootstrap-journal"
        if request_key:
            common = pathlib.Path(run_text(["git", "-C", str(root), "rev-parse", "--git-common-dir"]))
            journal_root = (common if common.is_absolute() else root / common).resolve() / "oasis7-bootstrap-journal"
    journal_path = journal_root / f"{journal_key}.json"
    journal_existed = journal_path.exists()
    journal = json.loads(journal_path.read_text(encoding="utf-8")) if journal_existed else {}
    if journal:
        recorded_request = journal.get("immutable_request") or journal.get("request")
        if not isinstance(recorded_request, dict):
            die("bootstrap journal is missing immutable request; cannot resume safely")
        normalized_recorded = OrderedDict(
            (key, sorted(recorded_request.get(key) or []) if key in {"source_refs", "doc_refs", "related_prd", "handoff_to"} else
             list(recorded_request.get(key) or []) if key == "acceptance" else
             recorded_request.get(key) if key == "loop_binding" else
             str(recorded_request.get(key) or ""))
            for key in immutable_request
        )
        if normalized_recorded != immutable_request:
            changed = [key for key in immutable_request if normalized_recorded.get(key) != immutable_request.get(key)]
            die("bootstrap immutable request drift; start a new task bootstrap (mismatch: " + ",".join(changed) + ")")
        recorded_digest = str(journal.get("immutable_request_digest") or "")
        if recorded_digest and recorded_digest != immutable_digest:
            die("bootstrap immutable request digest mismatch; journal may be corrupt")
    task_uid = str(journal.get("task_uid") or (binding or {}).get("task_uid") or f"task_{uuid.uuid4().hex}")
    if binding and journal.get("state") == "completed":
        validate_loop_inputs(root, binding, args.repo, "in_flight")
        live = github_issue_record(args.repo, task_uid)
        if not live or live.get("loop_binding") != binding:
            die("completed manual bootstrap live binding mismatch; reconcile selected task")
        payload = {**live, "task_uid": task_uid, "task_path": live["issue_url"],
                   "execution_log_path": live["issue_url"], "resumed_existing_task": True}
        print(json.dumps(payload, sort_keys=True) if args.json else f"new-task: reused {task_uid}")
        return 0
    if binding and not journal:
        existing_uid = github_issue_record(args.repo, task_uid)
        if existing_uid:
            die("manual task UID already exists; use explicit existing-task resume instead")
    if binding:
        validate_loop_inputs(root, binding, args.repo, "new_tasks")
    if not journal:
        journal = {"version": 2, "task_uid": task_uid, "state": "planned", "creation_outcome": "never_attempted", "next_action": "create_issue",
                   "immutable_request": immutable_request, "immutable_request_digest": immutable_digest,
                   "updated_at": now()}
        atomic_json(journal_path, journal)
    task = OrderedDict(
        [
            ("task_uid", task_uid),
            ("title", args.title),
            ("owner_role", args.owner_role),
            ("module", args.module or ""),
            ("worktree_hint", args.worktree_hint or ""),
            ("status", "candidate"),
            ("workflow_phase", "bootstrap"),
            ("priority", args.priority),
            ("source_signal", args.source_signal or ""),
            ("source_type", args.source_type or ""),
            ("severity", args.severity or ""),
            ("source_refs", args.source_ref or []),
            ("doc_refs", args.doc_ref or []),
            ("related_prd", args.related_prd or []),
            ("acceptance", args.acceptance or []),
            ("handoff_to", args.handoff_to or []),
            ("updated_at", now()),
        ]
    )
    if primary_package:
        task["primary_package"] = primary_package
    if binding:
        task["loop_binding"] = binding
        task["bootstrap_base_oid"] = base_oid
    issue_url = str(journal.get("issue_url") or "")
    if not issue_url and journal_existed:
        try:
            recovered = github_issue_record(args.repo, task_uid)
        except (subprocess.CalledProcessError, json.JSONDecodeError, RuntimeError):
            if binding:
                die("manual bootstrap Issue reconciliation uncertain; retry the same request after readback recovers")
            recovered = None
        issue_url = str((recovered or {}).get("issue_url") or "")
        if issue_url and binding and ((recovered or {}).get("loop_binding") != binding or
                                      (recovered or {}).get("worktree_hint") != task["worktree_hint"]):
            die("manual bootstrap recovered Issue binding/worktree mismatch")
    if not issue_url:
        if journal_existed and journal.get("creation_outcome") not in {"never_attempted", "confirmed_no_write"}:
            die("bootstrap Issue creation outcome uncertain; retain pending intent and reconcile exact live identity before retry")
        if binding:
            require_supplied_uid_absent(args.repo, task_uid)
        # Persist before the non-idempotent remote attempt. Neither an empty
        # search nor an exception proves that GitHub rejected the write.
        attempted = False
        def before_create():
            nonlocal attempted
            journal.update({"creation_outcome": "uncertain", "next_action": "reconcile_issue", "updated_at": now()})
            atomic_json(journal_path, journal)
            attempted = True
        try:
            issue_url = create_issue(args.repo, task, before_write=before_create)
        except Exception:
            if not attempted:
                journal.update({"creation_outcome": "confirmed_no_write", "next_action": "create_issue", "updated_at": now()})
                atomic_json(journal_path, journal)
            raise
    journal.update({"issue_url": issue_url, "creation_outcome": "completed", "state": "issue_created", "next_action": "add_project_item", "updated_at": now()})
    atomic_json(journal_path, journal)
    issue_number = issue_number_from_url(issue_url)
    if binding:
        ensure_loop_history(args.repo, issue_number, binding)
    item_id = str(journal.get("project_item_id") or "")
    if not item_id:
        item_id = add_project_item(args, issue_url)
    journal.update({"project_item_id": item_id, "state": "project_item_added", "next_action": "update_project_fields", "updated_at": now()})
    atomic_json(journal_path, journal)
    updated_fields = update_project_fields(args, task, item_id)
    record = {
        "task_uid": task_uid,
        "title": args.title,
        "owner_role": args.owner_role,
        "module": args.module or "",
        "worktree_hint": args.worktree_hint or "",
        "status": "candidate",
        "priority": args.priority,
        "source_signal": args.source_signal or "",
        "source_type": args.source_type or "",
        "severity": args.severity or "",
        **({"primary_package": primary_package} if primary_package else {}),
        "source_refs": args.source_ref or [],
        "doc_refs": args.doc_ref or [],
        "related_prd": args.related_prd or [],
        "acceptance": args.acceptance or [],
        "handoff_to": args.handoff_to or [],
        "issue_url": issue_url,
        "issue_number": issue_number,
        "project_item_id": item_id,
        "created_at": now(),
        "updated_at": now(),
        "evidence_sink": issue_url,
        **repository_identity,
    }
    if binding:
        record.update(loop_binding=binding, bootstrap_epoch=binding["bootstrap_epoch"], bootstrap_base_oid=base_oid)
    merge_task_mapping(mapping_path, task_uid, record)
    if binding:
        record_loop_lineage(root, binding)
    merge_project_mapping(mapping_path, {"owner": args.project_owner, "number": args.project_number, "repo": args.repo})
    journal.update({"state": "completed", "next_action": "none", "mapping_path": str(mapping_path), "updated_at": now()})
    atomic_json(journal_path, journal)
    payload = dict(record)
    payload.update(
        {
            "task_path": issue_url,
            "execution_log_path": issue_url,
            "updated_field_values": updated_fields,
            "mapping_path": str(mapping_path),
            "bootstrap_journal": str(journal_path),
            "resumed_existing_task": False,
        }
    )
    if args.json:
        print(json.dumps(payload, indent=2, sort_keys=True))
    else:
        print(f"new-task: created {task_uid} ({issue_url})")
    return 0


def require_record(args: argparse.Namespace) -> tuple[pathlib.Path, dict[str, Any], dict[str, Any]]:
    root = args.root.resolve()
    mapping_path = mapping_path_for(root, args.mapping)
    mapping = load_mapping(mapping_path)
    try:
        retired = durable_store.retired_task(mapping, args.task_uid)
    except ValueError as exc:
        die(f"task UID retirement ledger is invalid: {exc}")
    if retired is not None:
        die(f"task UID is retired and cannot be read or mutated: {args.task_uid}")
    record = mapping.get("tasks", {}).get(args.task_uid)
    if not record:
        try:
            record = github_issue_record(args.repo, args.task_uid)
        except (subprocess.CalledProcessError, json.JSONDecodeError, RuntimeError) as exc:
            die(f"task_uid not found in mapping and GitHub issue lookup failed: {args.task_uid}: {exc}")
        if not record:
            die(f"task_uid not found in mapping or GitHub issue body: {args.task_uid}")
        mapping.setdefault("tasks", {})[args.task_uid] = record
    return mapping_path, mapping, record


def require_live_issue_route_matches_cache(repo: str, task_uid: str, record: dict[str, Any]) -> dict[str, Any]:
    """Require the canonical Issue route and lifecycle to match the local projection."""
    try:
        live = github_issue_record(repo, task_uid)
    except (OSError, subprocess.CalledProcessError, json.JSONDecodeError, RuntimeError) as exc:
        die(f"done transition: canonical live Issue route lookup failed closed: {exc}")
    if not isinstance(live, dict):
        die("done transition: canonical live Issue route is unavailable")
    if (live.get("task_uid") != task_uid or type(live.get("issue_number")) is not int
            or type(record.get("issue_number")) is not int
            or live.get("issue_number") != record.get("issue_number")):
        die("done transition: canonical live Issue identity differs from task mapping")

    mismatched = []
    for key in LIVE_ROUTE_CACHE_FIELDS:
        cached_value = record.get(key)
        live_value = live.get(key)
        if key == "pr_number":
            cached_value = None if cached_value in (None, "") else str(cached_value)
            live_value = None if live_value in (None, "") else str(live_value)
        else:
            cached_value = None if cached_value in (None, "") else cached_value
            live_value = None if live_value in (None, "") else live_value
        if cached_value != live_value:
            mismatched.append(key)
    if mismatched:
        aggregate_route = (
            record.get("completion_mode") == "ordered_delivery_aggregate"
            or live.get("completion_mode") == "ordered_delivery_aggregate"
            or any(key.startswith("aggregate_") for key in mismatched)
        )
        route_name = "aggregate route/pointers" if aggregate_route else "Issue route/lifecycle"
        die(f"done transition: canonical live {route_name} differ from task mapping ({', '.join(mismatched)})")
    return live


def validate_aggregate_task_complete_claim(
    repo: str,
    root: pathlib.Path,
    task_uid: str,
    claim: Any,
    live_issue: dict[str, Any],
) -> None:
    """Validate the local claim-ready projection consumed by aggregate closeout.

    This checks the repository-owned claim shape and its exact local source
    identity. It does not claim trusted runtime attestation; that is outside the
    current human-operated closeout contract.
    """
    if not isinstance(claim, dict):
        die("closeout-task: aggregate completion requires canonical task_complete claim evidence")
    if (claim.get("claim_type") != "task_complete" or claim.get("status") != "verified"
            or claim.get("allowed_to_claim") is not True
            or type(claim.get("verification_exit_code")) is not int
            or claim.get("verification_exit_code") != 0
            or claim.get("task_uid") != task_uid):
        die("closeout-task: aggregate task_complete claim identity or result is invalid")

    profile = claim.get("verification_profile")
    if not isinstance(profile, str) or profile == "fixture_repository_state":
        die("closeout-task: aggregate task_complete claim requires a production verification profile")
    profile_commands = {
        # Keep these command identities aligned with claim-ready.sh's
        # repository-owned verification-profile switch. Profile/mode support
        # itself is shared with loop_leaf_result.py.
        "codex_subagent_role_fit": (
            "./scripts/pm/verify-codex-subagent-role-fit.sh --task-uid " + shlex.quote(task_uid)
        ),
        "workflow_behavior": "./scripts/pm/workflow-behavior-eval.sh",
        "repository_required": "true",
    }
    if profile not in profile_commands or claim.get("verify_command") != profile_commands[profile]:
        die("closeout-task: aggregate task_complete claim profile/command is not repository-owned")

    verification = {
        "profile": profile,
        "mode": claim.get("verification_mode"),
        "frozen_source_head": claim.get("frozen_source_head"),
        "frozen_source_tree": claim.get("frozen_source_tree"),
        "repository_fingerprint_before": claim.get("repository_fingerprint_before"),
        "repository_fingerprint_after": claim.get("repository_fingerprint_after"),
        "verification_epoch_stable": claim.get("verification_epoch_stable"),
        "verification_exit_code": claim.get("verification_exit_code"),
    }
    if verification_projection_errors(verification):
        die("closeout-task: aggregate task_complete verification projection is incomplete or unsupported")
    if claim.get("verification_mode") != "detached_frozen_tree":
        die("closeout-task: aggregate task_complete claim must use detached frozen-tree verification")

    try:
        verified_at = datetime.fromisoformat(str(claim.get("verified_at") or "").replace("Z", "+00:00"))
        current_time = datetime.now().astimezone()
    except (TypeError, ValueError):
        die("closeout-task: aggregate task_complete claim timestamp is invalid")
    if verified_at.tzinfo is None or verified_at > current_time:
        die("closeout-task: aggregate task_complete claim timestamp is not a valid current-round time")

    validate_latest_task_complete_comment(repo, live_issue, task_uid, claim, profile_commands[profile], verified_at)

    root = root.resolve()
    fingerprint_tool = root / "scripts/pm/repo-state-fingerprint.py"
    try:
        fingerprint = json.loads(subprocess.check_output(
            [sys.executable, str(fingerprint_tool), str(root)],
            text=True,
            stderr=subprocess.PIPE,
        ))
        head = str(fingerprint.get("head") or "")
        tree = subprocess.check_output(
            ["git", "-C", str(root), "rev-parse", "HEAD^{tree}"],
            text=True,
            stderr=subprocess.PIPE,
        ).strip()
    except (OSError, subprocess.CalledProcessError, json.JSONDecodeError) as exc:
        die(f"closeout-task: aggregate task_complete source identity read failed: {exc}")

    expected_head = str(claim.get("frozen_source_head") or "")
    expected_tree = str(claim.get("frozen_source_tree") or "")
    if (not re.fullmatch(r"[0-9a-f]{40}", head)
            or claim.get("repository_head") != head
            or expected_head != head
            or expected_tree != tree
            or claim.get("repository_index_sha256") != fingerprint.get("index_sha256")
            or claim.get("repository_fingerprint_before") != fingerprint.get("sha256")
            or claim.get("repository_fingerprint_after") != fingerprint.get("sha256")):
        die("closeout-task: aggregate task_complete claim does not bind current HEAD/tree/index/fingerprint")


def validate_latest_task_complete_comment(
    repo: str,
    live_issue: dict[str, Any],
    task_uid: str,
    claim: dict[str, Any],
    verify_command: str,
    verified_at: datetime,
) -> None:
    """Require the latest Issue update to be the exact claim-ready readback."""
    issue_number = live_issue.get("issue_number")
    issue_url = f"https://github.com/{repo}/issues/{issue_number}"
    if (type(issue_number) is not int or issue_number <= 0
            or live_issue.get("issue_url") != issue_url
            or str(live_issue.get("issue_state") or "").upper() != "OPEN"):
        die("closeout-task: aggregate claim verification requires the exact open task Issue")
    try:
        comments = json.loads(run_text([
            "gh", "api",
            f"repos/{repo}/issues/{issue_number}/comments?per_page=1&sort=created&direction=desc",
        ]))
    except (OSError, subprocess.CalledProcessError, json.JSONDecodeError, RuntimeError, subprocess.TimeoutExpired) as exc:
        die(f"closeout-task: latest task claim comment readback failed closed: {exc}")
    if not isinstance(comments, list) or len(comments) != 1 or not isinstance(comments[0], dict):
        die("closeout-task: latest task claim comment readback is missing or malformed")
    comment = comments[0]
    expected_body = "\n".join((
        "<!-- oasis7-pm-claim-verification -->",
        f"Task UID: {task_uid}",
        f"Claim Type: {claim['claim_type']}",
        f"Verified At: {claim['verified_at']}",
        f"Verification Exit Code: {claim['verification_exit_code']}",
        f"Verification Status: {claim['status']}",
        f"Verify Command: {verify_command}",
        f"Claim Message: {claim.get('claim_message') or ''}",
        "",
    ))
    comment_id = comment.get("id")
    expected_comment_url = (
        f"https://github.com/{repo}/issues/{issue_number}#issuecomment-{comment_id}"
    )
    expected_api_issue_url = f"https://api.github.com/repos/{repo}/issues/{issue_number}"
    if (comment.get("body") != expected_body
            or type(comment_id) is not int or comment_id <= 0
            or comment.get("html_url") != expected_comment_url
            or comment.get("issue_url") != expected_api_issue_url):
        die("closeout-task: latest task comment does not exactly bind the task_complete claim")
    try:
        created_at = datetime.fromisoformat(str(comment.get("created_at") or "").replace("Z", "+00:00"))
        comment_updated_at = datetime.fromisoformat(str(comment.get("updated_at") or "").replace("Z", "+00:00"))
        issue_updated_at = datetime.fromisoformat(str(live_issue.get("updated_at") or "").replace("Z", "+00:00"))
        current_time = datetime.now().astimezone()
    except (TypeError, ValueError):
        die("closeout-task: latest task claim comment timestamps are invalid")
    if (created_at.tzinfo is None or comment_updated_at.tzinfo is None or issue_updated_at.tzinfo is None
            or created_at < verified_at or created_at > current_time
            or comment_updated_at != created_at or issue_updated_at != comment_updated_at):
        die("closeout-task: task claim comment is stale, edited, or superseded on the live Issue")


def validate_live_aggregate_lifecycle(
    repo: str,
    task_uid: str,
    record: dict[str, Any],
    live_issue: dict[str, Any],
    *,
    expected_state: str,
) -> dict[str, Any]:
    """Require the live coordinator Issue to retain its task_done projection.

    Aggregate terminal completion advances the Project/mapping phase and then
    closes the Issue. The Issue body remains at its verified `done/task_done`
    closeout projection, so every terminal consumer checks that projection
    against the bound aggregate route and receipt instead of trusting cache.
    """
    expected_state = expected_state.upper()
    if expected_state not in {"OPEN", "CLOSED"}:
        die("aggregate coordinator lifecycle expected Issue state is invalid")
    if not isinstance(live_issue, dict):
        die("aggregate coordinator lifecycle live Issue is unavailable")

    body = live_issue.get("body")
    if isinstance(body, str):
        body = body.replace("\r\n", "\n")
        uid_lines = re.findall(r"(?m)^[ \t]*(?:-[ \t]+)?task_uid[ \t]*:[^\n]*$", body)
        if uid_lines != [f"task_uid: {task_uid}"]:
            die("aggregate coordinator lifecycle Task UID is missing, malformed, or ambiguous")
        try:
            live_fields = issue_task_fields(body)
        except SystemExit as exc:
            die(f"aggregate coordinator lifecycle fields are malformed: {exc}")
        if live_fields.get("trace_projection_error"):
            die(f"aggregate coordinator lifecycle fields are malformed: {live_fields['trace_projection_error']}")
        issue_url = str(live_issue.get("url") or "")
        issue_state = str(live_issue.get("state") or "")
    else:
        # github_issue_record() is also a live source: it verifies the unique
        # canonical UID in the Issue body and returns its strict parsed fields.
        if live_issue.get("task_uid") != task_uid:
            die("aggregate coordinator lifecycle Task UID is missing or ambiguous")
        live_fields = live_issue
        issue_url = str(live_issue.get("issue_url") or "")
        issue_state = str(live_issue.get("issue_state") or "")

    issue_number = live_issue.get("number", live_issue.get("issue_number"))
    expected_url = f"https://github.com/{repo}/issues/{record.get('issue_number')}"
    if (record.get("task_uid") != task_uid or record.get("repository") != repo
            or type(record.get("issue_number")) is not int
            or issue_number != record.get("issue_number") or issue_url != expected_url):
        die("aggregate coordinator lifecycle Issue identity differs from task truth")
    if issue_state.upper() != expected_state:
        die(f"aggregate coordinator lifecycle Issue must remain {expected_state.lower()}")

    if (record.get("status") != "done"
            or record.get("workflow_phase") not in {"task_done", "post_merge_done"}
            or record.get("completion_mode") != "ordered_delivery_aggregate"
            or record.get("pr_number") or record.get("pr_url")
            or not record.get("aggregate_plan_comment_id")
            or not record.get("aggregate_plan_sha256")
            or not re.fullmatch(r"[0-9a-f]{64}", str(record.get("aggregate_completion_receipt_sha256") or ""))):
        die("aggregate coordinator lifecycle task truth is not terminalizable")

    expected_fields = {
        "status": "done",
        "workflow_phase": "task_done",
        "completion_mode": "ordered_delivery_aggregate",
        "aggregate_plan_comment_id": str(record["aggregate_plan_comment_id"]),
        "aggregate_plan_sha256": record["aggregate_plan_sha256"],
        "aggregate_completion_receipt_sha256": record["aggregate_completion_receipt_sha256"],
    }
    if (any(live_fields.get(key) != value for key, value in expected_fields.items())
            or live_fields.get("pr_number") not in (None, "")
            or live_fields.get("pr_url") not in (None, "")):
        die("aggregate coordinator lifecycle differs from the verified done/task_done route")
    return live_fields


def require_live_aggregate_lifecycle(
    repo: str,
    task_uid: str,
    record: dict[str, Any],
    *,
    expected_state: str,
) -> dict[str, Any]:
    """Read and validate the live coordinator lifecycle before terminal effects."""
    try:
        live_issue = github_issue_record(repo, task_uid)
    except (OSError, subprocess.CalledProcessError, json.JSONDecodeError, RuntimeError, SystemExit) as exc:
        die(f"aggregate coordinator lifecycle live Issue read failed closed: {exc}")
    if not isinstance(live_issue, dict):
        die("aggregate coordinator lifecycle live Issue is unavailable")
    validate_live_aggregate_lifecycle(
        repo, task_uid, record, live_issue, expected_state=expected_state,
    )
    return live_issue


def recover_missing_project_item(args: argparse.Namespace, record: dict[str, Any]) -> None:
    if record.get("project_item_id"):
        return
    try:
        recovered = load_sync_module().recover_project_mapping(args.project_owner, args.project_number)
    except Exception:
        return
    record.update(recovered.get(args.task_uid) or {})


def has_verified_task_complete(record: dict[str, Any]) -> bool:
    for item in record.get("claim_verifications") or []:
        if not isinstance(item, dict):
            continue
        if str(item.get("claim_type") or "") != "task_complete":
            continue
        if str(item.get("status") or "") != "verified":
            continue
        if str(item.get("verification_exit_code") or "") in {"", "0"}:
            return True
    return (
        str(record.get("last_claim_type") or "") == "task_complete"
        and str(record.get("last_verification_status") or "") == "verified"
        and str(record.get("last_verification_exit_code") or "") in {"", "0"}
    )


def validate_authoritative_project_fields(
    args: argparse.Namespace,
    mapping: dict[str, Any],
    record: dict[str, Any],
    live_issue: dict[str, Any],
) -> None:
    """Read the live Project item and reject Issue/Project lifecycle drift."""
    sync = load_sync_module()
    try:
        project_id, _project_fields = sync.project_context(args.project_owner, args.project_number)
        cached_project_id = str((mapping.get("project") or {}).get("id") or "")
        if cached_project_id and cached_project_id != project_id:
            die("classify-non-pr-task: cached/live Project identity drift")
        recovered = sync.recover_project_mapping_for_task_uids(
            args.project_owner,
            args.project_number,
            args.repo,
            [args.task_uid],
            project_id,
        )
    except Exception as exc:
        die(f"classify-non-pr-task: live GitHub Project fields unavailable: {exc}")
    project_record = recovered.get(args.task_uid)
    if not isinstance(project_record, dict):
        die(f"classify-non-pr-task: live GitHub Project item not found for {args.task_uid}")
    cached_item_id = str(record.get("project_item_id") or "")
    live_item_id = str(project_record.get("project_item_id") or "")
    if not cached_item_id or not live_item_id or cached_item_id != live_item_id:
        die("classify-non-pr-task: cached/live Project item identity drift")
    values = project_record.get("project_field_values")
    if not isinstance(values, dict):
        die("classify-non-pr-task: live GitHub Project field values are missing")
    status = str(live_issue.get("status") or "")
    issue_phase = str(live_issue.get("workflow_phase") or "")
    project_phase = issue_phase
    if issue_phase in {"", "bootstrap"}:
        project_phase = sync.workflow_phase_for(status)
    elif issue_phase in {"task_done", "main_sync", "closed_without_merge", "post_" + "merge_done"}:
        project_phase = "done"
    expected = {
        "Task UID": args.task_uid,
        "Status": sync.project_status_for(status),
        "PM Status": status,
        "Workflow Phase": project_phase,
        "Owner Role": str(live_issue.get("owner_role") or ""),
        "Module": str(live_issue.get("module") or ""),
        "Priority": str(live_issue.get("priority") or ""),
        "Canonical Worktree": str(live_issue.get("worktree_hint") or ""),
    }
    for field_name, expected_value in expected.items():
        if not expected_value:
            die(f"classify-non-pr-task: live Issue field {field_name} is missing")
        actual_value = str(values.get(field_name) or "")
        if not actual_value:
            die(f"classify-non-pr-task: live GitHub Project field {field_name} is missing")
        if expected_value and actual_value != expected_value:
            die(
                "classify-non-pr-task: Issue/Project lifecycle authority drift "
                f"({field_name}: Issue={expected_value!r} Project={actual_value!r})"
            )


def command_append_evidence(args: argparse.Namespace) -> int:
    mapping_path, mapping, record = require_record(args)
    issue_number = int(record["issue_number"])
    fields = {
        "Completed": args.completed,
        "Pending": args.pending,
        "Action": args.action,
        "Validation Command": args.validation_command,
        "Expected Result": args.expected_result,
        "Actual Result": args.actual_result,
        "Blocker / Next Action": args.blocker_next_action,
    }
    comment_url = issue_comment(args.repo, issue_number, evidence_body(args.task_uid, args.role, "execution", fields))
    record["last_evidence_at"] = now()
    record["updated_at"] = now()
    record.setdefault("evidence_comments", []).append(comment_url)
    merge_task_mapping(mapping_path, args.task_uid, record)
    payload = {
        "task_uid": args.task_uid,
        "issue_url": record.get("issue_url"),
        "comment_url": comment_url,
        "execution_log_path": record.get("issue_url"),
        "status": "ok",
    }
    print(json.dumps(payload, indent=2, sort_keys=True) if args.json else f"append-execution-log: appended evidence to {comment_url}")
    return 0


def command_classify_non_pr_task(args: argparse.Namespace) -> int:
    mapping_path, mapping, record = require_record(args)
    if str(record.get("task_uid") or "") != args.task_uid:
        die("classify-non-pr-task: canonical mapping embedded Task UID does not match its key")
    required_identity = (
        "repository",
        "canonical_worktree",
        "task_branch",
        "default_branch",
        "issue_number",
        "issue_url",
    )
    if any(record.get(key) in (None, "") for key in required_identity):
        die("classify-non-pr-task: canonical repository/worktree/Issue identity is incomplete")
    if str(record["repository"]) != args.repo:
        die("classify-non-pr-task: canonical repository is not bound to --repo")
    cached_issue_number, cached_issue_url = validate_issue_identity(
        args.repo, record["issue_number"], record["issue_url"], source="cached"
    )
    repository_identity = authoritative_repository_identity(
        args.root.resolve(), str(record["repository"]), str(record["canonical_worktree"]),
    )
    for key in ("repository", "canonical_worktree", "task_branch", "default_branch"):
        if key == "canonical_worktree":
            matches = pathlib.Path(str(record.get(key) or "")).expanduser().resolve() == pathlib.Path(repository_identity[key]).resolve()
        else:
            matches = str(record.get(key) or "") == repository_identity[key]
        if not matches:
            die(f"classify-non-pr-task: canonical {key} identity drift")
    evidence = str(args.evidence or "").strip()
    if not evidence:
        die("classify-non-pr-task: --evidence must be non-empty")
    live = github_issue_record(args.repo, args.task_uid)
    if not live:
        die(f"classify-non-pr-task: live GitHub issue not found for {args.task_uid}")
    if str(live.get("task_uid") or "") != args.task_uid:
        die("classify-non-pr-task: live Issue Task UID mismatch")
    live_issue_number, live_issue_url = validate_issue_identity(
        args.repo, live.get("issue_number"), live.get("issue_url"), source="live"
    )
    if cached_issue_number != live_issue_number:
        die("classify-non-pr-task: cached/live Issue identity drift (issue_number)")
    if cached_issue_url != live_issue_url:
        die("classify-non-pr-task: cached/live Issue identity drift (issue_url)")
    live_state = str(live.get("issue_state") or "").upper()
    live_state_reason = str(live.get("issue_state_reason") or "")
    if live_state == "CLOSED":
        die(
            "classify-non-pr-task: CLOSED Issue is immutable; "
            f"stateReason={live_state_reason or 'missing'}"
        )
    if live_state != "OPEN":
        die("classify-non-pr-task: authoritative live Issue state is missing or unsupported")
    validate_authoritative_project_fields(args, mapping, record, live)
    status = str(live.get("status") or "")
    phase = str(live.get("workflow_phase") or "")
    if not status or not phase:
        die("classify-non-pr-task: live Issue status and workflow phase are required")
    for key, current in (("status", status), ("workflow_phase", phase)):
        cached = str(record.get(key) or "")
        if cached and cached != current:
            die(f"classify-non-pr-task: cached/live lifecycle authority drift ({key})")
    if status in {"done", "deferred"} or phase in {"task_done", "closed_without_merge", "post_" + "merge_done"}:
        die("classify-non-pr-task: terminal tasks cannot be classified")
    if any(record.get(key) not in (None, "", 0, False, {}) for key in
           ("pr_number", "pr_url", "pull_request_url", "merge_receipt")):
        die("classify-non-pr-task: PR-bound tasks cannot be classified")
    if any(live.get(key) not in (None, "", 0, False, {}) for key in
           ("pr_number", "pr_url", "pull_request_url", "merge_receipt")):
        die("classify-non-pr-task: live Issue is PR-bound")

    updated = json.loads(json.dumps(record))
    updated["status"] = status
    updated["workflow_phase"] = phase
    for key in ("pr_number", "pr_url", "pull_request_url"):
        if live.get(key) not in (None, "", 0, False, {}):
            updated[key] = live[key]
    updated["completion_mode"] = "non_pr_task"
    updated["non_pr_completion_evidence"] = evidence
    evidence_file = args.root.resolve() / ".pm" / "scratch" / args.task_uid / "non-pr-completion-evidence.txt"
    atomic_text(evidence_file, evidence)
    updated["non_pr_completion_evidence_file"] = str(evidence_file)
    updated["non_pr_completion_evidence_sha256"] = hashlib.sha256(
        evidence_file.read_bytes()
    ).hexdigest()
    updated["updated_at"] = now()
    cleared_traceability = synchronize_live_issue_traceability(
        args.repo, args.task_uid, updated, live=live,
    )
    task = task_from_record(args.task_uid, updated)
    update_issue_body(args.repo, int(live["issue_number"]), task)
    comment_url = verified_issue_comment(
        args.repo,
        int(live["issue_number"]),
        evidence_body(
            args.task_uid,
            args.role,
            "non_pr_classification",
            {
                "Classification": "non_pr_task",
                "Evidence": evidence,
                "Validation Command": args.validation_command,
                "Expected Result": "Task is explicitly classified for non-PR completion.",
                "Actual Result": "Issue body updated and exact comment readback verified.",
            },
        ),
    )
    updated["last_evidence_at"] = now()
    updated.setdefault("evidence_comments", []).append(comment_url)
    merge_task_mapping(mapping_path, args.task_uid, updated, clear_keys=cleared_traceability)
    payload = {
        "status": "ok",
        "task_uid": args.task_uid,
        "completion_mode": "non_pr_task",
        "non_pr_completion_evidence": evidence,
        "non_pr_completion_evidence_file": str(evidence_file),
        "non_pr_completion_evidence_sha256": updated["non_pr_completion_evidence_sha256"],
        "issue_url": live.get("issue_url"),
        "comment_url": comment_url,
        "comment_readback_verified": True,
    }
    print(json.dumps(payload, indent=2, sort_keys=True) if args.json else f"classify-non-pr-task: classified {args.task_uid}")
    return 0


def command_workflow_report(args: argparse.Namespace) -> int:
    if args.phase in {"start", "close"} and not args.task_uid:
        die("workflow-report: --task-uid is required for start and close")
    if args.phase == "review" and not args.task_uid:
        payload = {"phase": "review", "role": args.role, "status": "ok", "task_source": "github_project"}
        print(json.dumps(payload, indent=2, sort_keys=True) if args.json else "workflow-report review: GitHub Project is authoritative")
        return 0
    mapping_path, mapping, record = require_record(args)
    fields = {
        "Workflow Phase": args.phase,
        "Task Status": record.get("status"),
        "Issue": record.get("issue_url"),
        "Worktree": record.get("worktree_hint") or "",
    }
    comment_url = issue_comment(args.repo, int(record["issue_number"]), evidence_body(args.task_uid, args.role, args.phase, fields))
    timestamp_key = {
        "start": "last_started_at",
        "close": "last_workflow_report_close_at",
    }.get(args.phase)
    if timestamp_key:
        record[timestamp_key] = now()
    record["last_evidence_at"] = now()
    record["updated_at"] = now()
    record.setdefault("evidence_comments", []).append(comment_url)
    merge_task_mapping(mapping_path, args.task_uid, record)
    payload = {
        "task_uid": args.task_uid,
        "role": args.role,
        "phase": args.phase,
        "status": "ok",
        "issue_url": record.get("issue_url"),
        "execution_log_path": record.get("issue_url"),
        "comment_url": comment_url,
    }
    if timestamp_key:
        payload[timestamp_key] = record[timestamp_key]
    print(json.dumps(payload, indent=2, sort_keys=True) if args.json else f"workflow-report {args.phase}: recorded {comment_url}")
    return 0


def command_move_task(args: argparse.Namespace) -> int:
    mapping_path, mapping, record = require_record(args)
    previous = str(record.get("status") or "")
    previous_phase = str(record.get("workflow_phase") or "")
    if args.to_status == "done" and record.get("completion_mode") == "ordered_delivery_aggregate":
        die(
            "move-task: ordered aggregate completion requires the exact aggregate receipt, plan, candidate, "
            "and evidence through task-closeout.sh; generic move-task cannot publish aggregate task_done"
        )
    if args.to_status == "done":
        require_live_issue_route_matches_cache(args.repo, args.task_uid, record)
    if args.to_status in GATE_OWNED_STATUSES:
        canonical_writer = (
            "task-closeout.sh with canonical review/CI evidence"
            if args.to_status == "ready"
            else "prepare-task-pr.sh --promote-draft with canonical CI/promotion evidence"
        )
        die(
            f"move-task: {args.to_status} is owned by the canonical {canonical_writer}; "
            "generic move-task cannot publish a readiness-gated status"
        )
    if previous == "done":
        if args.to_status == "done":
            # Terminal finalizers own the fine-grained phase.  A repeated
            # generic move is a read-only idempotent acknowledgment, never a
            # projection back to the intermediate task_done phase.
            payload = {
                "task_uid": args.task_uid,
                "previous_status": previous,
                "status": "done",
                "workflow_phase": previous_phase,
                "issue_url": record.get("issue_url"),
                "project_item_id": record.get("project_item_id"),
                "updated_field_values": 0,
                "idempotent": True,
            }
            print(json.dumps(payload, indent=2, sort_keys=True) if args.json else f"move-task: {args.task_uid} already done")
            return 0
        die("move-task: terminal task cannot be reclassified; use its canonical finalizer or terminal runbook")
    if previous_phase in TERMINAL_WORKFLOW_PHASES:
        die("move-task: terminal workflow phase cannot be reclassified by generic move-task")
    if args.to_status == "done" and not (record.get("last_closed_at") and has_verified_task_complete(record)):
        die(
            "move-task: refusing done without closeout and verified task_complete evidence; "
            "run ./scripts/pm/task-closeout.sh --role <owner_role> --task-uid "
            f"{args.task_uid} --verify-command '<cmd>'"
        )
    if args.to_status == "done":
        recover_missing_project_item(args, record)
    if args.to_status == "done" and not record.get("project_item_id"):
        die(
            "move-task: refusing done because GitHub Project item mapping could not be recovered; "
            f"task_uid={args.task_uid}"
        )
    # Status and Workflow Phase are one canonical Project projection.  Derive
    # both from the requested status before producing any Issue/Project write,
    # so a stale cached phase cannot publish an invalid combination such as
    # Done/deferred/execution.
    record["status"] = args.to_status
    record["workflow_phase"] = load_sync_module().workflow_phase_for(args.to_status)
    record["updated_at"] = now()
    cleared_traceability = synchronize_live_issue_traceability(args.repo, args.task_uid, record)
    task = task_from_record(args.task_uid, record)
    updated_fields = 0
    if args.to_status != "done" and record.get("project_item_id"):
        updated_fields = update_project_fields(args, task, str(record["project_item_id"]))
    update_issue_body(args.repo, int(record["issue_number"]), task)
    merge_task_mapping(mapping_path, args.task_uid, record, clear_keys=cleared_traceability)
    payload = {
        "task_uid": args.task_uid,
        "previous_status": previous,
        "status": args.to_status,
        "issue_url": record.get("issue_url"),
        "project_item_id": record.get("project_item_id"),
        "updated_field_values": updated_fields,
    }
    print(json.dumps(payload, indent=2, sort_keys=True) if args.json else f"move-task: moved {args.task_uid} {previous} -> {args.to_status}")
    return 0


def command_closeout_task(args: argparse.Namespace) -> int:
    mapping_path, mapping, original = require_record(args)
    previous = str(original.get("status") or "")
    claim = json.loads(args.claim_json)
    if args.to_status == "done" and original.get("completion_mode") == "ordered_delivery_aggregate" and not args.aggregate_receipt:
        die("closeout-task: ordered aggregate completion requires an aggregate receipt")
    if args.aggregate_receipt and original.get("completion_mode") != "ordered_delivery_aggregate":
        die("closeout-task: aggregate receipt requires ordered aggregate task truth")
    live_issue = None
    if args.to_status == "done":
        live_issue = require_live_issue_route_matches_cache(
            getattr(args, "repo", DEFAULT_REPO), args.task_uid, original,
        )
    if args.to_status != "deferred":
        if claim.get("status") != "verified" or not claim.get("allowed_to_claim"):
            die("closeout-task: verified immutable claim evidence is required")
    if args.aggregate_receipt:
        if args.to_status != "done" or args.pr_receipt:
            die("closeout-task: aggregate receipt is done-only and excludes singular PR receipt")
        if original.get("completion_mode") != "ordered_delivery_aggregate" or original.get("pr_number") or original.get("pr_url"):
            die("closeout-task: coordinator mode/PR identity is invalid")
        if not all((args.aggregate_plan, args.aggregate_candidate, args.aggregate_evidence)):
            die("closeout-task: aggregate receipt requires plan, candidate and evidence")
        validate_aggregate_task_complete_claim(
            getattr(args, "repo", DEFAULT_REPO), args.root, args.task_uid, claim, live_issue or {},
        )
        validation = subprocess.run([
            sys.executable, str(args.root.resolve() / "scripts/pm/aggregate-task-completion.py"), "validate",
            "--repo-root", str(args.root.resolve()), "--task-uid", args.task_uid,
            "--record", args.aggregate_plan, "--candidate", args.aggregate_candidate,
            "--evidence", args.aggregate_evidence, "--receipt", args.aggregate_receipt, "--json",
        ], text=True, capture_output=True)
        if validation.returncode:
            die("closeout-task: aggregate receipt live validation failed: " + (validation.stderr.strip() or validation.stdout.strip()))
    record = json.loads(json.dumps(original))
    closed_at = now()
    record.setdefault("claim_verifications", []).append(claim)
    record["last_claim_verification_at"] = claim.get("verified_at")
    record["last_closed_at"] = closed_at
    record["last_evidence_at"] = closed_at
    record["status"] = args.to_status
    if args.to_status == "done" and args.pr_receipt:
        receipt = json.loads(pathlib.Path(args.pr_receipt).read_text(encoding="utf-8"))
        record["merge_receipt"] = receipt
        record["merge_receipt_sha256"] = hashlib.sha256(pathlib.Path(args.pr_receipt).read_bytes()).hexdigest()
    if args.to_status == "done" and args.aggregate_receipt:
        aggregate_path = pathlib.Path(args.aggregate_receipt)
        receipt = json.loads(aggregate_path.read_text(encoding="utf-8"))
        record["aggregate_completion_receipt"] = receipt
        record["aggregate_completion_receipt_sha256"] = hashlib.sha256(aggregate_path.read_bytes()).hexdigest()
    if args.to_status == "done":
        recover_missing_project_item(args, record)
        if not record.get("project_item_id"):
            die("closeout-task: done requires a recoverable GitHub Project item")
    task = task_from_record(args.task_uid, record)
    terminal_phase = (
        "task_done" if args.to_status == "done"
        else "blocked" if args.to_status == "deferred"
        else "pre_pr_ready"
    )
    record["workflow_phase"] = terminal_phase
    cleared_traceability = synchronize_live_issue_traceability(args.repo, args.task_uid, record)
    task = task_from_record(args.task_uid, record)
    evidence_fields = {
        "Workflow Phase": terminal_phase,
        "Task Status": args.to_status,
        "Immutable Verification Head": claim.get("frozen_source_head") or "n/a",
        "Immutable Verification Tree": claim.get("frozen_source_tree") or "n/a",
    }
    if record.get("merge_receipt"):
        receipt = record["merge_receipt"]
        evidence_fields.update({
            "Merge Receipt Issuer": receipt.get("issuer"),
            "Merge Receipt Repository": receipt.get("repository"),
            "Merge Receipt PR": receipt.get("pr_url"),
            "Merge Receipt Head": receipt.get("head_oid"),
            "Merge Receipt Observed At": receipt.get("observed_at"),
        })
    if record.get("aggregate_completion_receipt"):
        evidence_fields.update({
            "Aggregate Receipt Type": "oasis7_aggregate_task_complete",
            "Aggregate Receipt SHA256": record["aggregate_completion_receipt_sha256"],
            "Aggregate Plan Comment ID": record["aggregate_completion_receipt"].get("plan_comment_id"),
        })
    comment_url = issue_comment(
        args.repo,
        int(record["issue_number"]),
        evidence_body(
            args.task_uid,
            args.role,
            terminal_phase,
            evidence_fields,
        ),
    )
    updated_fields = 0
    if args.to_status == "done":
        updated_fields = update_done_project_fields(args, task, str(record["project_item_id"]))
    elif record.get("project_item_id"):
        updated_fields = update_project_fields(
            args,
            task,
            str(record["project_item_id"]),
            require_lifecycle_projection=True,
        )
    update_issue_body(args.repo, int(record["issue_number"]), task)
    record.setdefault("evidence_comments", []).append(comment_url)
    if record.get("_github_source") != "issue_search" or mapping_path.exists():
        cache_patch = {
            "status": record["status"],
            "last_closed_at": record["last_closed_at"],
            "last_evidence_at": record["last_evidence_at"],
            "last_claim_verification_at": record.get("last_claim_verification_at"),
            "claim_verifications": [claim],
            "evidence_comments": [comment_url],
            "workflow_phase": terminal_phase,
        }
        if record.get("merge_receipt"):
            cache_patch["merge_receipt"] = record["merge_receipt"]
            cache_patch["merge_receipt_sha256"] = record["merge_receipt_sha256"]
        if record.get("aggregate_completion_receipt"):
            cache_patch["aggregate_completion_receipt"] = record["aggregate_completion_receipt"]
            cache_patch["aggregate_completion_receipt_sha256"] = record["aggregate_completion_receipt_sha256"]
        if record.get("project_item_id"):
            cache_patch["project_item_id"] = record["project_item_id"]
        for key in traceability_issue_keys:
            if key in record:
                cache_patch[key] = record[key]
        merge_task_mapping(
            mapping_path, args.task_uid, cache_patch,
            clear_keys=cleared_traceability,
        )
    payload = {
        "task_uid": args.task_uid,
        "previous_status": previous,
        "status": args.to_status,
        "issue_url": record.get("issue_url"),
        "comment_url": comment_url,
        "last_closed_at": closed_at,
        "updated_field_values": updated_fields,
    }
    print(json.dumps(payload, indent=2, sort_keys=True) if args.json else f"closeout-task: {args.task_uid} -> {args.to_status}")
    return 0


def command_bind_aggregate_plan(args: argparse.Namespace) -> int:
    """Bind an immutable linked-delivery plan before any declared PR merges."""
    mapping_path, _mapping, original = require_record(args)
    if original.get("pr_number") or original.get("pr_url"):
        die("bind-aggregate-plan: coordinator cannot have a singular PR")
    if original.get("completion_mode") not in {None, "", "ordered_delivery_aggregate"}:
        die("bind-aggregate-plan: coordinator already uses a different completion route")
    if original.get("status") in {"done", "deferred"} or original.get("workflow_phase") in TERMINAL_WORKFLOW_PHASES:
        die("bind-aggregate-plan: terminal coordinator cannot be rebound")
    plan_path = pathlib.Path(args.plan).resolve(strict=True)
    plan = json.loads(plan_path.read_text(encoding="utf-8"))
    aggregate_path = args.root.resolve() / "scripts/pm/aggregate-task-completion.py"
    aggregate_spec = importlib.util.spec_from_file_location("aggregate_task_completion", aggregate_path)
    if not aggregate_spec or not aggregate_spec.loader:
        die("bind-aggregate-plan: aggregate plan validator is unavailable")
    aggregate_module = importlib.util.module_from_spec(aggregate_spec)
    aggregate_spec.loader.exec_module(aggregate_module)
    try:
        aggregate_module.validate_plan(plan, args.task_uid)
    except aggregate_module.ReceiptError as exc:
        die(f"bind-aggregate-plan: invalid versioned plan: {exc}")
    if plan.get("task_uid") != args.task_uid or plan.get("repository") != args.repo or plan.get("issue_number") != original.get("issue_number"):
        die("bind-aggregate-plan: plan/coordinator identity mismatch")
    canonical = json.dumps(plan, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    body = "<!-- oasis7-aggregate-delivery-plan/v1 -->\n" + canonical
    expected_sha = "sha256:" + hashlib.sha256(body.encode("utf-8")).hexdigest()
    comment = json.loads(run_text(["gh", "api", f"repos/{args.repo}/issues/comments/{args.comment_id}"]))
    if (comment.get("id") != args.comment_id or comment.get("body") != body or
            str(comment.get("issue_url") or "").rstrip("/").split("/")[-1] != str(original.get("issue_number"))):
        die("bind-aggregate-plan: live plan comment body/Issue identity mismatch")
    author = str((comment.get("user") or {}).get("login") or "")
    if not author:
        die("bind-aggregate-plan: live plan author is unavailable")
    permission = json.loads(run_text(["gh", "api", f"repos/{args.repo}/collaborators/{author}/permission"]))
    if permission.get("permission") != "admin":
        die("bind-aggregate-plan: plan author lacks repository admin authority")
    issue = json.loads(run_text(["gh", "api", f"repos/{args.repo}/issues/{original['issue_number']}"]))
    issue_body = str(issue.get("body") or "").replace("\r\n", "\n")
    issue_uid_fields = re.findall(r"(?m)^[ \t]*(?:-[ \t]+)?task_uid\b[^\n]*$", issue_body)
    if issue.get("state") != "open" or issue_uid_fields != [f"task_uid: {args.task_uid}"]:
        die("bind-aggregate-plan: live coordinator Issue identity/state mismatch")
    live_fields = issue_task_fields(issue_body)
    if live_fields.get("completion_mode") not in {None, "", "ordered_delivery_aggregate"}:
        die("bind-aggregate-plan: live coordinator already uses a different completion route")
    live_pointer = (live_fields.get("aggregate_plan_comment_id"), live_fields.get("aggregate_plan_sha256"))
    if any(live_pointer) and live_pointer != (str(args.comment_id), expected_sha):
        die("bind-aggregate-plan: live immutable plan pointer differs")
    for child in plan.get("required_deliveries") or []:
        number = child.get("pr_number") if isinstance(child, dict) else None
        if not isinstance(number, int) or number <= 0:
            die("bind-aggregate-plan: required delivery PR identity is invalid")
        pr = json.loads(run_text(["gh", "pr", "view", str(number), "-R", args.repo, "--json", "state,isDraft,number,url"]))
        if pr.get("state") != "OPEN" or pr.get("isDraft") is not True or pr.get("number") != number or pr.get("url") != child.get("pr_url"):
            die("bind-aggregate-plan: all required PRs must be live drafts at binding")
    old = (original.get("aggregate_plan_comment_id"), original.get("aggregate_plan_sha256"))
    if any(old) and old != (str(args.comment_id), expected_sha):
        die("bind-aggregate-plan: an immutable coordinator plan is already bound")
    record = json.loads(json.dumps(original))
    record.update(completion_mode="ordered_delivery_aggregate",
                  aggregate_plan_comment_id=str(args.comment_id), aggregate_plan_sha256=expected_sha)
    update_issue_body(args.repo, int(record["issue_number"]), task_from_record(args.task_uid, record))
    reread = json.loads(run_text(["gh", "api", f"repos/{args.repo}/issues/{record['issue_number']}"]))
    fields = issue_task_fields(str(reread.get("body") or ""))
    if any(fields.get(key) != value for key, value in (("completion_mode", "ordered_delivery_aggregate"),
                                                       ("aggregate_plan_comment_id", str(args.comment_id)),
                                                       ("aggregate_plan_sha256", expected_sha))):
        die("bind-aggregate-plan: live Issue pointer readback mismatch")
    merge_task_mapping(mapping_path, args.task_uid, {
        "completion_mode": "ordered_delivery_aggregate",
        "aggregate_plan_comment_id": str(args.comment_id), "aggregate_plan_sha256": expected_sha,
    })
    print(json.dumps({"status": "bound", "task_uid": args.task_uid, "comment_id": args.comment_id,
                      "plan_body_sha256": expected_sha}, sort_keys=True))
    return 0


def validate_canonical_aggregate_terminal_receipt(
    args: argparse.Namespace,
    record: dict[str, Any],
    receipt: Any,
    receipt_bytes: bytes,
) -> None:
    """Accept only the finalizer's exact, canonical durable aggregate receipt."""
    root = args.root.resolve()
    common = pathlib.Path(run_text(["git", "-C", str(root), "rev-parse", "--git-common-dir"]).strip())
    if not common.is_absolute():
        common = (root / common).resolve()
    durable_root = common / "oasis7-workflow-receipts"
    task_root = durable_root / args.task_uid
    terminal_path = task_root / "aggregate-terminal-receipt.json"
    supplied_path = pathlib.Path(args.receipt_json)
    try:
        supplied_resolved = supplied_path.resolve(strict=True)
        terminal_resolved = terminal_path.resolve(strict=True)
        canonical_bytes = terminal_path.read_bytes()
    except OSError:
        die("set-phase: canonical aggregate terminal receipt is not durably present")
    if (durable_root.is_symlink() or task_root.is_symlink() or terminal_path.is_symlink()
            or supplied_resolved != terminal_resolved or canonical_bytes != receipt_bytes):
        die("set-phase: aggregate terminal receipt is not at its canonical durable identity")

    expected_keys = {
        "schema", "receipt_type", "issuer", "task_uid", "repository", "issue_number",
        "aggregate_completion_receipt_sha256", "plan_comment_id", "observed_at", "receipt_sha256",
    }
    if not isinstance(receipt, dict) or set(receipt) != expected_keys:
        die("set-phase: canonical aggregate terminal receipt schema is incomplete or ambiguous")
    expected_storage = (json.dumps(receipt, sort_keys=True, indent=2) + "\n").encode("utf-8")
    if receipt_bytes != expected_storage:
        die("set-phase: aggregate terminal receipt bytes are not canonical finalizer output")
    payload = {key: value for key, value in receipt.items() if key != "receipt_sha256"}
    canonical = json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")
    completion_sha = str(record.get("aggregate_completion_receipt_sha256") or "")
    plan_comment_id = str(record.get("aggregate_plan_comment_id") or "")
    plan_sha = str(record.get("aggregate_plan_sha256") or "")
    expected_repository = str(args.repo or "")
    cached_repository = str(record.get("repository") or expected_repository)
    issue_number = record.get("issue_number")
    try:
        observed = datetime.fromisoformat(str(receipt.get("observed_at") or "").replace("Z", "+00:00"))
        observed_valid = observed.tzinfo is not None
    except ValueError:
        observed_valid = False
    if (
        receipt.get("schema") != "oasis7.aggregate-terminal/v1"
        or receipt.get("receipt_type") != "oasis7_aggregate_terminal"
        or receipt.get("issuer") != "aggregate-task-finalizer"
        or receipt.get("task_uid") != args.task_uid
        or cached_repository != expected_repository
        or receipt.get("repository") != expected_repository
        or type(issue_number) is not int
        or type(receipt.get("issue_number")) is not int
        or receipt.get("issue_number") != issue_number
        or not re.fullmatch(r"[0-9a-f]{64}", completion_sha)
        or receipt.get("aggregate_completion_receipt_sha256") != completion_sha
        or not plan_comment_id
        or not plan_comment_id.isdecimal()
        or int(plan_comment_id) <= 0
        or type(receipt.get("plan_comment_id")) is not int
        or receipt.get("plan_comment_id") <= 0
        or not re.fullmatch(r"sha256:[0-9a-f]{64}", plan_sha)
        or str(receipt.get("plan_comment_id")) != plan_comment_id
        or not observed_valid
        or receipt.get("receipt_sha256") != hashlib.sha256(canonical).hexdigest()
    ):
        die("set-phase: canonical aggregate terminal receipt identity or digest mismatch")


def command_set_phase(args: argparse.Namespace) -> int:
    mapping_path, _mapping, original = require_record(args)
    receipt_bytes = pathlib.Path(args.receipt_json).read_bytes()
    receipt = json.loads(receipt_bytes.decode("utf-8"))
    current = str(original.get("workflow_phase") or "")
    aggregate_terminal = args.phase == "post_merge_done" and original.get("completion_mode") == "ordered_delivery_aggregate"
    allowed_transition = args.phase in ALLOWED_PHASE_TRANSITIONS.get(current, set()) or (aggregate_terminal and current == "task_done")
    if not allowed_transition:
        die(f"set-phase: transition {current!r} -> {args.phase!r} is not allowed")
    receipt_schema = ("oasis7_aggregate_terminal", "aggregate-task-finalizer") if aggregate_terminal else RECEIPT_SCHEMAS.get(args.phase)
    if not receipt_schema or (receipt.get("receipt_type"), receipt.get("issuer")) != receipt_schema:
        die("set-phase: receipt schema or issuer mismatch")
    if receipt.get("task_uid") != args.task_uid:
        die("set-phase: receipt task_uid mismatch")
    if aggregate_terminal:
        if original.get("pr_number") or original.get("pr_url") or original.get("status") != "done":
            die("set-phase: aggregate terminal coordinator identity/status is invalid")
        digest = str(original.get("aggregate_completion_receipt_sha256") or "")
        if not digest or receipt.get("aggregate_completion_receipt_sha256") != digest:
            die("set-phase: aggregate terminal receipt chain mismatch")
        if not all((args.aggregate_plan, args.aggregate_candidate, args.aggregate_evidence, args.aggregate_receipt)):
            die("set-phase: aggregate terminal requires the exact plan/candidate/evidence/completion receipt")
        validation = subprocess.run([
            sys.executable, str(args.root.resolve() / "scripts/pm/aggregate-task-completion.py"), "validate",
            "--repo-root", str(args.root.resolve()), "--task-uid", args.task_uid,
            "--record", args.aggregate_plan, "--candidate", args.aggregate_candidate,
            "--evidence", args.aggregate_evidence, "--receipt", args.aggregate_receipt, "--json",
        ], text=True, capture_output=True)
        if validation.returncode or hashlib.sha256(pathlib.Path(args.aggregate_receipt).read_bytes()).hexdigest() != digest:
            die("set-phase: aggregate completion receipt no longer verifies live")
        validate_canonical_aggregate_terminal_receipt(args, original, receipt, receipt_bytes)
        require_live_aggregate_lifecycle(
            args.repo, args.task_uid, original, expected_state="OPEN",
        )
    record = json.loads(json.dumps(original))
    record["workflow_phase"] = args.phase
    record.setdefault("phase_receipts", {})[args.phase] = receipt
    record.setdefault("phase_receipt_sha256", {})[args.phase] = hashlib.sha256(receipt_bytes).hexdigest()
    comment_url = issue_comment(args.repo, int(record["issue_number"]), evidence_body(
        args.task_uid, args.role, args.phase,
        {"Workflow Phase": args.phase, "Receipt Type": receipt.get("receipt_type"),
         "Receipt Observed At": receipt.get("observed_at")},
    ))
    record.setdefault("evidence_comments", []).append(comment_url)
    task = task_from_record(args.task_uid, record)
    if record.get("project_item_id"):
        update_project_fields(args, task, str(record["project_item_id"]))
    merge_task_mapping(mapping_path, args.task_uid, {
        "workflow_phase": args.phase,
        "phase_receipts": record["phase_receipts"],
        "phase_receipt_sha256": record["phase_receipt_sha256"],
        "evidence_comments": [comment_url],
        "last_evidence_at": now(),
    })
    print(json.dumps({"status":"ok","task_uid":args.task_uid,"workflow_phase":args.phase,
                      "comment_url":comment_url}, sort_keys=True))
    return 0


def project_refresh_graphql(
    query: str,
    variables: dict[str, Any],
    *,
    operation: str,
    task_uid: str,
) -> dict[str, Any]:
    """Keep selected refresh on one named shared-client GraphQL read."""
    sync = load_sync_module()
    data = sync.graphql_request(
        None,
        query,
        variables,
        operation=operation,
        context={"script": "github-project-task.py", "task_uid": task_uid},
    )
    # Existing refresh decoders consume the gh-compatible data envelope.
    return {"data": data}


def refresh_project_identity(
    node: dict[str, Any],
    canonical_owner: str,
    canonical_number: int,
    canonical_project_id: str,
) -> dict[str, Any]:
    project = node.get("project")
    if not isinstance(project, dict):
        die("refresh-task: live Project item is missing Project identity")
    project_id = str(project.get("id") or "")
    owner = project.get("owner")
    owner = owner if isinstance(owner, dict) else {}
    project_owner = str(owner.get("login") or "")
    try:
        project_number = int(project.get("number"))
    except (TypeError, ValueError):
        die("refresh-task: live Project identity has an invalid number")
    if not project_id or not project_owner:
        die("refresh-task: live Project identity is incomplete")
    if project_owner != canonical_owner:
        die(
            "refresh-task: live Project owner does not match canonical owner: "
            f"{project_owner!r} != {canonical_owner!r}"
        )
    if project_number != canonical_number:
        die(
            "refresh-task: live Project number does not match canonical Project: "
            f"{project_number!r} != {canonical_number!r}"
        )
    if canonical_project_id and project_id != canonical_project_id:
        die(
            "refresh-task: live Project id does not match canonical Project: "
            f"{project_id!r} != {canonical_project_id!r}"
        )
    field_values = node.get("fieldValues")
    field_values = field_values if isinstance(field_values, dict) else {}
    page_info = field_values.get("pageInfo")
    page_info = page_info if isinstance(page_info, dict) else {}
    if page_info.get("hasNextPage") is not False:
        die("refresh-task: live Project item fieldValues pagination is incomplete or unknown")
    return {"id": project_id, "number": project_number, "owner": project_owner}


def trace_projection_loss(task_uid: str, reason: str) -> None:
    die(f"trace-projection-loss: {task_uid}: {reason}")


def recover_non_pr_task_worktree_authority(
    task_uid: str,
    repository: str,
    mapping: str,
    live: dict[str, Any],
    repository_identity: dict[str, str],
    existing: dict[str, Any],
) -> dict[str, str] | None:
    """Recover non-PR evidence from the registered task worktree only."""
    mode = str(live.get("completion_mode") or existing.get("completion_mode") or "")
    if mode != "non_pr_task":
        return None

    canonical = pathlib.Path(repository_identity["canonical_worktree"]).resolve()
    task_mapping_path = mapping_path_for(canonical, mapping)
    if not task_mapping_path.is_file():
        trace_projection_loss(task_uid, "canonical task-worktree mapping is unavailable")
    try:
        task_mapping = load_mapping(task_mapping_path)
    except (OSError, ValueError, json.JSONDecodeError):
        trace_projection_loss(task_uid, "canonical task-worktree mapping cannot be read")
    task_record = (task_mapping.get("tasks") or {}).get(task_uid)
    if not isinstance(task_record, dict):
        trace_projection_loss(task_uid, "canonical task-worktree task record is unavailable")

    expected_identity = {
        "task_uid": task_uid,
        "repository": repository,
        "canonical_worktree": str(canonical),
        "task_branch": repository_identity["task_branch"],
        "default_branch": repository_identity["default_branch"],
        "issue_number": str(live.get("issue_number") or ""),
        "issue_url": str(live.get("issue_url") or ""),
    }
    if not expected_identity["issue_number"] or not expected_identity["issue_url"]:
        trace_projection_loss(task_uid, "live Issue identity is incomplete")
    for key, expected in expected_identity.items():
        actual = task_record.get(key)
        if key == "canonical_worktree":
            matches = bool(actual) and pathlib.Path(str(actual)).expanduser().resolve() == canonical
        else:
            matches = str(actual or "") == expected
        if not matches:
            trace_projection_loss(task_uid, f"canonical task-worktree {key} identity drift")

    if str(task_record.get("completion_mode") or "") != "non_pr_task":
        trace_projection_loss(task_uid, "canonical task-worktree completion mode is not non-PR")
    evidence = task_record.get("non_pr_completion_evidence")
    issue_evidence = live.get("non_pr_completion_evidence")
    if evidence is None or issue_evidence is None or str(evidence) != str(issue_evidence):
        trace_projection_loss(task_uid, "task-worktree evidence differs from Issue evidence")

    evidence_file = str(task_record.get("non_pr_completion_evidence_file") or "")
    expected_file = canonical / ".pm" / "scratch" / task_uid / "non-pr-completion-evidence.txt"
    try:
        actual_file = pathlib.Path(evidence_file).expanduser()
        if not actual_file.is_absolute() or actual_file.resolve() != expected_file:
            trace_projection_loss(task_uid, "task-worktree non-PR evidence path is not canonical")
        if not actual_file.is_file():
            trace_projection_loss(task_uid, "identity-bound non-PR evidence file is unavailable")
        actual_text = actual_file.read_text(encoding="utf-8")
        expected_text = str(evidence).rstrip("\n") + "\n"
        if actual_text != expected_text:
            trace_projection_loss(task_uid, "identity-bound non-PR evidence file differs from Issue evidence")
        digest = str(task_record.get("non_pr_completion_evidence_sha256") or "")
        if not re.fullmatch(r"[0-9a-f]{64}", digest):
            trace_projection_loss(task_uid, "task-worktree non-PR evidence digest is malformed")
        if hashlib.sha256(actual_file.read_bytes()).hexdigest() != digest:
            trace_projection_loss(task_uid, "identity-bound non-PR evidence file digest mismatch")
    except (OSError, UnicodeError):
        trace_projection_loss(task_uid, "identity-bound non-PR evidence file cannot be read")
    return {
        "non_pr_completion_evidence_file": str(expected_file),
        "non_pr_completion_evidence_sha256": digest,
    }


def preserve_identity_bound_cache(
    task_uid: str,
    existing: dict[str, Any],
    live: dict[str, Any],
    record: dict[str, Any],
    repository_identity: dict[str, str],
    task_worktree_authority: dict[str, str] | None = None,
) -> None:
    """Retain local non-PR evidence only after checking its full binding."""
    cached_identity = {
        key: existing.get(key) for key in identity_bound_cache_keys
    }
    for key in sorted(identity_bound_cache_keys - {
        "non_pr_completion_evidence_file",
        "non_pr_completion_evidence_sha256",
    }):
        cached = str(existing.get(key) or "")
        if cached and cached != repository_identity[key]:
            trace_projection_loss(task_uid, f"cached {key} identity drift")

    existing_mode = str(existing.get("completion_mode") or "")
    live_mode = str(live.get("completion_mode") or "")
    if existing_mode and live_mode and existing_mode != live_mode:
        trace_projection_loss(task_uid, "completion_mode cannot be reconstructed consistently")
    mode = live_mode or existing_mode
    if mode:
        record["completion_mode"] = mode

    # Closeout claims and its timestamp are Issue-authoritative projections,
    # not recoverable from a stale local cache.  If a previously refreshed
    # cache had them but the live Issue no longer carries either field, stop
    # before merge_task_mapping can silently retain the old claim.
    for key in ("last_closed_at", "claim_verifications"):
        if key in existing and existing.get(key) not in (None, "", [], {}):
            if key not in live:
                trace_projection_loss(
                    task_uid,
                    f"live Issue omitted previously projected {key}",
                )
            if key == "claim_verifications" and existing.get(key) != live.get(key):
                trace_projection_loss(
                    task_uid,
                    "live Issue claim_verifications changed from the refreshed cache",
                )
    if "last_closed_at" in live:
        record["last_closed_at"] = live["last_closed_at"]
    if "claim_verifications" in live:
        record["claim_verifications"] = live["claim_verifications"]

    evidence = live.get("non_pr_completion_evidence")
    cached_evidence = existing.get("non_pr_completion_evidence")
    if evidence is None:
        evidence = cached_evidence
    if evidence is not None:
        evidence = str(evidence)
        record["non_pr_completion_evidence"] = evidence
    live_digest = str(live.get("non_pr_completion_evidence_sha256") or "")
    cached_digest = str(cached_identity.get("non_pr_completion_evidence_sha256") or "")
    if live_digest and cached_digest and live_digest != cached_digest:
        trace_projection_loss(task_uid, "non-PR evidence digest disagrees between Issue and cache")
    authority = task_worktree_authority or {}
    authority_file = str(authority.get("non_pr_completion_evidence_file") or "")
    authority_digest = str(authority.get("non_pr_completion_evidence_sha256") or "")
    cached_file = str(cached_identity.get("non_pr_completion_evidence_file") or "")
    if cached_digest and not cached_file:
        trace_projection_loss(task_uid, "cached identity-bound non-PR evidence path is missing")
    if cached_file and not cached_digest:
        trace_projection_loss(task_uid, "cached identity-bound non-PR evidence digest is missing")
    if cached_file and authority_file and pathlib.Path(cached_file).expanduser().resolve() != pathlib.Path(authority_file).resolve():
        trace_projection_loss(task_uid, "cached and task-worktree non-PR evidence paths disagree")
    if cached_digest and authority_digest and cached_digest != authority_digest:
        trace_projection_loss(task_uid, "cached and task-worktree non-PR evidence digests disagree")
    digest = live_digest or cached_digest or authority_digest
    if evidence and digest:
        record["non_pr_completion_evidence_sha256"] = digest

    evidence_file = cached_file or authority_file
    requires_evidence = mode == "non_pr_task" and (
        str(record.get("status") or "") in {"done", "deferred"}
        or str(record.get("workflow_phase") or "") in TERMINAL_WORKFLOW_PHASES
        or bool(evidence)
    )
    if mode == "non_pr_task" and (requires_evidence or evidence_file or digest):
        if not evidence_file or not digest:
            trace_projection_loss(task_uid, "identity-bound non-PR evidence path or digest is missing")
        canonical = pathlib.Path(repository_identity["canonical_worktree"]).resolve()
        expected_file = canonical / ".pm" / "scratch" / task_uid / "non-pr-completion-evidence.txt"
        try:
            actual_file = pathlib.Path(evidence_file).expanduser()
            if not actual_file.is_absolute() or actual_file.resolve() != expected_file:
                trace_projection_loss(task_uid, "identity-bound non-PR evidence path is not canonical")
            if not actual_file.is_file():
                trace_projection_loss(task_uid, "identity-bound non-PR evidence file is unavailable")
            if hashlib.sha256(actual_file.read_bytes()).hexdigest() != digest:
                trace_projection_loss(task_uid, "identity-bound non-PR evidence file digest mismatch")
            if evidence is not None and actual_file.read_text(encoding="utf-8").rstrip("\n") != str(evidence).rstrip("\n"):
                trace_projection_loss(task_uid, "identity-bound non-PR evidence file differs from Issue evidence")
        except (OSError, UnicodeError):
            trace_projection_loss(task_uid, "identity-bound non-PR evidence file cannot be read")
        record["non_pr_completion_evidence_file"] = str(expected_file)
        record["non_pr_completion_evidence_sha256"] = digest


def command_refresh_task(args: argparse.Namespace) -> int:
    mapping_path = mapping_path_for(args.root.resolve(), args.mapping)
    latest = load_mapping(mapping_path)
    try:
        retired = durable_store.retired_task(latest, args.task_uid)
    except ValueError as exc:
        die(f"refresh-task: retirement ledger is invalid: {exc}")
    if retired is not None:
        die(f"refresh-task: Task UID is retired and cannot be refreshed: {args.task_uid}")
    existing = dict((latest.get("tasks") or {}).get(args.task_uid) or {})
    root = args.root.resolve()
    if existing:
        try:
            candidate_admission_guard_module().guard_candidate_issue(latest, mapping_path, args.task_uid, existing)
        except ValueError as exc:
            die(f"refresh-task: {exc}")
        except RuntimeError as exc:
            die(f"refresh-task: {exc}")
    project = latest.get("project") or {}
    project = project if isinstance(project, dict) else {}
    canonical_owner = str(project.get("owner") or args.project_owner or "")
    try:
        canonical_number = int(project.get("number") or args.project_number)
    except (TypeError, ValueError):
        die("refresh-task: canonical Project number is invalid")
    canonical_project_id = str(project.get("id") or "")
    if canonical_owner != str(args.project_owner or "") or canonical_number != int(args.project_number):
        die("refresh-task: configured Project identity does not match canonical mapping")
    pending_phase = pending_non_merge_phase(
        root, args.task_uid, existing, args.repo, project,
    )
    live = github_issue_record(args.repo, args.task_uid)
    if not live:
        die(f"refresh-task: authoritative GitHub issue not found for {args.task_uid}")
    if live.get("trace_projection_error"):
        trace_projection_loss(args.task_uid, str(live["trace_projection_error"]))
    if existing.get("task_uid") not in (None, "", args.task_uid):
        trace_projection_loss(args.task_uid, "cached task UID identity drift")
    if existing.get("repository") not in (None, "", args.repo):
        trace_projection_loss(args.task_uid, "cached repository identity drift")
    for key in ("issue_number", "issue_url"):
        cached_value = str(existing.get(key) or "")
        live_value = str(live.get(key) or "")
        if cached_value and live_value and cached_value != live_value:
            trace_projection_loss(args.task_uid, f"cached and live Issue {key} identities disagree")
    if existing.get("loop_binding") is not None and live.get("loop_binding") is None:
        die("refresh-task: live loop binding disappeared; explicit reconciliation required")
    lineage_path = loop_lineage_path(root, args.task_uid)
    if lineage_path.exists():
        lineage = json.loads(lineage_path.read_text())
        if lineage.get("loop_binding") != live.get("loop_binding"):
            die("refresh-task: live loop binding differs from immutable lineage; reconcile explicitly")
    snapshot_root = pathlib.Path(existing.get("canonical_worktree") or live.get("worktree_hint") or root)
    snapshot_path = snapshot_root / ".pm/scratch" / args.task_uid / "bootstrap-task-snapshot.json"
    if snapshot_path.exists():
        snapshot_binding = json.loads(snapshot_path.read_text()).get("task", {}).get("loop_binding")
        if snapshot_binding is not None and snapshot_binding != live.get("loop_binding"):
            die("refresh-task: live loop binding differs from bootstrap snapshot; reconcile explicitly")
    # The command root is execution context, not task identity.  Terminal
    # refreshes intentionally run from the default worktree, so rebinding the
    # task to that root would destroy the canonical task-worktree/branch pair.
    # Resolve registered identity from task truth instead and fail closed when
    # live and cached task identities disagree.
    identity_candidates: list[dict[str, str]] = []
    for label, hint in (
        ("cached", str(existing.get("canonical_worktree") or "")),
        ("live", str(live.get("worktree_hint") or "")),
    ):
        if not hint:
            continue
        if not pathlib.Path(hint).expanduser().exists():
            if label == "cached":
                trace_projection_loss(args.task_uid, "cached canonical task worktree is unavailable")
            continue
        candidate = authoritative_repository_identity(root, args.repo, hint)
        if not any(
            item["canonical_worktree"] == candidate["canonical_worktree"]
            for item in identity_candidates
        ):
            identity_candidates.append(candidate)
    if len(identity_candidates) > 1:
        die("refresh-task: cached and live canonical worktree identities disagree")
    if not identity_candidates:
        die("refresh-task: no registered canonical task worktree identity is available")
    repository_identity = identity_candidates[0]
    live["worktree_hint"] = repository_identity["canonical_worktree"]
    recovered: dict[str, Any] = {}
    item_id = str(existing.get("project_item_id") or "")
    project_fields: dict[str, str] = {}
    selected_node: dict[str, Any] | None = None
    live_project_identity: dict[str, Any] | None = None
    if item_id:
        query = """
        query($ids: [ID!]!) {
          nodes(ids: $ids) {
            ... on ProjectV2Item {
              id
              project {
                id
                number
                owner {
                  ... on Organization { login }
                  ... on User { login }
                }
              }
              fieldValues(first: 100) {
                pageInfo { hasNextPage }
                nodes {
                  ... on ProjectV2ItemFieldTextValue { text field { ... on ProjectV2FieldCommon { name } } }
                  ... on ProjectV2ItemFieldSingleSelectValue { name field { ... on ProjectV2FieldCommon { name } } }
                }
              }
            }
          }
        }
        """
        payload = project_refresh_graphql(
            query,
            {"ids": [item_id]},
            operation="project_task_refresh_bound_item",
            task_uid=args.task_uid,
        )
        nodes = ((payload.get("data") or {}).get("nodes") or [])
        if nodes:
            selected_node = nodes[0]
            if not isinstance(selected_node, dict):
                die("refresh-task: live Project item readback is malformed")
            live_project_identity = refresh_project_identity(
                selected_node,
                canonical_owner,
                canonical_number,
                canonical_project_id,
            )
        if selected_node is None:
            die("refresh-task: bound Project item is unavailable; refusing to refresh without Project binding")
    else:
        query = """
        query($q: String!) {
          search(query: $q, type: ISSUE, first: 2) {
            nodes { ... on Issue { number url body projectItems(first: 20) { nodes {
              id
              project {
                id
                number
                owner {
                  ... on Organization { login }
                  ... on User { login }
                }
              }
              fieldValues(first: 100) {
                pageInfo { hasNextPage }
                nodes {
                ... on ProjectV2ItemFieldTextValue { text field { ... on ProjectV2FieldCommon { name } } }
                ... on ProjectV2ItemFieldSingleSelectValue { name field { ... on ProjectV2FieldCommon { name } } }
                }
              }
            } } } }
          }
        }
        """
        payload = project_refresh_graphql(
            query,
            {"q": f"repo:{args.repo} {args.task_uid} in:body"},
            operation="project_task_refresh_issue_search",
            task_uid=args.task_uid,
        )
        issues = (((payload.get("data") or {}).get("search") or {}).get("nodes") or [])
        matches = [issue for issue in issues if re.search(
            rf"^task_uid:\s*{re.escape(args.task_uid)}$", str(issue.get("body") or ""), re.MULTILINE)]
        nodes = []
        if len(matches) == 1:
            for node in ((matches[0].get("projectItems") or {}).get("nodes") or []):
                project_node = node.get("project") if isinstance(node, dict) else None
                project_number = project_node.get("number") if isinstance(project_node, dict) else None
                try:
                    is_canonical_number = int(project_number) == canonical_number
                except (TypeError, ValueError):
                    is_canonical_number = False
                if not is_canonical_number:
                    continue
                if not isinstance(node, dict):
                    die("refresh-task: live Project item readback is malformed")
                live_project_identity = refresh_project_identity(
                    node,
                    canonical_owner,
                    canonical_number,
                    canonical_project_id,
                )
                nodes = [node]
                selected_node = node
                item_id = str(node.get("id") or "")
                if not item_id:
                    die("refresh-task: live Project item identity is missing")
                recovered = {"project_item_id": item_id, "issue_url": matches[0].get("url"),
                             "issue_number": matches[0].get("number")}
                break
        # The issue is still authoritative when its project item is temporarily
        # unavailable to this refresh query.  Preserve the refreshed issue and
        # repository identity; a later refresh can recover the project fields.
    if item_id:
        if selected_node:
            for value in (((selected_node.get("fieldValues") or {}).get("nodes")) or []):
                field_name = str(((value.get("field") or {}).get("name") or ""))
                field_value = str(value.get("name") or value.get("text") or "")
                if field_name and field_value:
                    project_fields[field_name] = field_value
    authoritative_keys = issue_authoritative_keys
    record: dict[str, Any] = {}
    for key in authoritative_keys:
        if key in live:
            record[key] = live[key]
        elif key == "acceptance":
            record[key] = []
        elif key in {"source_refs", "doc_refs", "related_prd"} and key in existing:
            # An older Issue may not yet have an optional section. Retain the
            # identity-bound cache value until an explicit Issue edit removes
            # it; Project refresh must never erase it by omission.
            record[key] = existing[key]
    record.update({key: value for key, value in recovered.items() if value not in (None, "")})
    if record.get("loop_binding") is not None:
        validate_loop_binding(record["loop_binding"])
        if record["loop_binding"]["task_uid"] != args.task_uid:
            die("live loop binding UID mismatch")
        record["bootstrap_epoch"] = record["loop_binding"]["bootstrap_epoch"]
        for name, expected in (("Loop", record["loop_binding"]["loop"]), ("Change ID", record["loop_binding"]["change_id"])):
            if project_fields.get(name) != expected:
                die(f"refresh-task: live Project {name} differs from frozen Issue binding")
    elif project_fields.get("Loop") or project_fields.get("Change ID"):
        die("refresh-task: live Project loop lineage exists but Issue binding is missing")
    project_lifecycle = {
        "status": project_fields.get("PM Status", ""),
        "workflow_phase": project_fields.get("Workflow Phase", ""),
    }
    project_status = project_lifecycle["status"] if "status" in project_lifecycle_keys else ""
    lifecycle_rank = {
        "candidate": 0, "committed": 1, "blocked": 2, "ready": 3,
        "pr_watch": 4, "done": 5, "deferred": 5,
    }
    issue_status = str(record.get("status") or "candidate")
    if project_status in lifecycle_rank and lifecycle_rank[project_status] >= lifecycle_rank.get(issue_status, -1):
        record["status"] = project_status
        record["project_status"] = project_fields.get("Status", "")
        project_phase = (
            project_lifecycle["workflow_phase"]
            if "workflow_phase" in project_lifecycle_keys
            else ""
        )
        existing_phase = str(existing.get("workflow_phase") or "")
        issue_phase = str(record.get("workflow_phase") or "")
        fine_terminal_phases = {
            "task_done",
            "closed_without_" + "merge",
            "post_" + "merge_done",
        }
        if pending_phase is not None:
            record["workflow_phase"] = pending_phase
            pending_intent = existing.get("closed_without_merge_intent") or {}
            if isinstance(pending_intent, dict) and pending_intent.get("previous_status"):
                # Coarse Project `done` is an in-flight terminal side effect,
                # not authority to rewrite the predecessor bound by intent.
                record["status"] = pending_intent["previous_status"]
        elif project_status == "done":
            # Project exposes terminal phases as coarse `done`; Issue remains
            # the fine-grained authority, with cache as a recovery fallback.
            if issue_phase in fine_terminal_phases:
                record["workflow_phase"] = issue_phase
            elif issue_phase:
                trace_projection_loss(
                    args.task_uid,
                    "live Issue workflow phase conflicts with cached terminal phase",
                )
            elif existing_phase in fine_terminal_phases:
                record["workflow_phase"] = existing_phase
            else:
                trace_projection_loss(
                    args.task_uid,
                    "Project done cannot be classified without a fine terminal Issue/cache phase",
                )
        else:
            record["workflow_phase"] = project_phase
        record["reconciled_from_project"] = project_status != issue_status
    if pending_phase is not None:
        # Pending intent authority wins independently of Project/Issue rank:
        # either remote sink may have advanced first when a run crashed.
        pending_intent = existing.get("closed_without_merge_intent") or {}
        record["workflow_phase"] = pending_phase
        if isinstance(pending_intent, dict) and pending_intent.get("previous_status"):
            record["status"] = pending_intent["previous_status"]
    # A default-worktree cache that already carries the complete identity-bound
    # evidence binding is authoritative for this refresh.  Recovery from the
    # registered task worktree is only needed when that projection is genuinely
    # stale/incomplete; requiring the task-worktree mapping unconditionally
    # would reject older but complete closeout caches (and fixtures that model
    # that lifecycle).  preserve_identity_bound_cache still rechecks the
    # cached path, content, and digest below.
    cached_non_pr_binding_complete = all(
        existing.get(key) not in (None, "")
        for key in (
            "completion_mode",
            "non_pr_completion_evidence",
            "non_pr_completion_evidence_file",
            "non_pr_completion_evidence_sha256",
        )
    ) and str(existing.get("completion_mode") or "") == "non_pr_task"
    task_worktree_authority = None
    if not cached_non_pr_binding_complete:
        task_worktree_authority = recover_non_pr_task_worktree_authority(
            args.task_uid,
            args.repo,
            args.mapping,
            live,
            repository_identity,
            existing,
        )
    preserve_identity_bound_cache(
        args.task_uid, existing, live, record, repository_identity,
        task_worktree_authority,
    )
    record["cache_refreshed_at"] = now()
    # Local cache identity is never accepted from stale issue/project/cache
    # values.  Every refresh overwrites it from current registered git facts.
    record.update(repository_identity)
    project_patch = None
    if live_project_identity:
        project_patch = {
            "owner": live_project_identity["owner"],
            "number": live_project_identity["number"],
            "repo": args.repo,
            "id": live_project_identity["id"],
        }
    cleared_traceability = frozenset(key for key in traceability_context_keys if key not in live)
    merge_task_mapping(
        mapping_path, args.task_uid, record, project=project_patch,
        clear_keys=cleared_traceability,
    )
    committed = (load_mapping(mapping_path).get("tasks") or {}).get(args.task_uid) or record
    payload = {
        "status": "refreshed",
        "task_uid": args.task_uid,
        "issue_url": committed.get("issue_url"),
        "project_item_id": committed.get("project_item_id"),
        "task_status": committed.get("status"),
        "acceptance": committed.get("acceptance") or [],
        "cache_refreshed_at": committed["cache_refreshed_at"],
        "loop_binding": committed.get("loop_binding"),
    }
    print(json.dumps(payload, indent=2, sort_keys=True) if args.json else f"refresh-task: refreshed {args.task_uid}")
    return 0


class PublicationRecoveryAuthority:
    """Read-only admission for one current, professionally reviewed recovery.

    The live TPM envelope locates evidence; each evidence surface is checked
    independently. Neither the envelope nor an old journal grants authority.
    """

    marker = "<!-- oasis7-publication-recovery-admission/v1 -->"
    roles = ("runtime_engineer", "repository_health_engineer", "qa_engineer")
    issue_owned = {"status", "workflow_phase", "pr_number", "pr_url"}
    project_owned = {"Status", "PM Status", "Workflow Phase", "PR"}

    @staticmethod
    def encoded(value: Any) -> bytes:
        return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()

    @staticmethod
    def sha(raw: bytes) -> str:
        return hashlib.sha256(raw).hexdigest()

    def __init__(self, args: argparse.Namespace, record: dict[str, Any], binding: dict[str, Any],
                 intent: dict[str, Any], module: Any, comments: list[dict[str, Any]]):
        self.args, self.record, self.binding, self.intent, self.module = args, dict(record), binding, intent, module
        self.root = args.root.resolve()
        project_id, _ = load_sync_module().project_context(args.project_owner, args.project_number)
        self.record["project_id"] = project_id
        selected = [c for c in comments if self.marker in c["body"]]
        if len(selected) != 1:
            raise ValueError("recovery requires one unique current TPM admission")
        body = selected[0]["body"]
        match = re.fullmatch(re.escape(self.marker) + r"\s*```json\s*\n(.*?)\n```\s*", body, re.S)
        if not match:
            raise ValueError("malformed recovery admission")
        self.envelope = json.loads(match[1], object_pairs_hook=module._unique_object)
        self.comment = selected[0]
        self.envelope_body = body
        if self.marker == "<!-- oasis7-publication-recovery-admission/v2 -->":
            envelope_keys = {"schema", "identity", "operation", "recovery_request", "plan_scope",
                             "current_action", "helper_review", "unrelated_snapshot", "admission_author_login"}
            supported_schema = "oasis7-publication-recovery-admission/v2"
        else:
            envelope_keys = {"schema", "identity", "operation", "scope_evidence", "current_action",
                             "predecessor", "helper_review", "unrelated_snapshot"}
            supported_schema = "oasis7-publication-recovery-admission/v1"
        self._closed(self.envelope, envelope_keys)
        if (self.envelope["schema"] != supported_schema
                or self.envelope["operation"] != "record_pr_publication_recovery"):
            raise ValueError("unsupported recovery operation")
        if (self.marker == "<!-- oasis7-publication-recovery-admission/v2 -->"
                and (not isinstance(self.envelope["admission_author_login"], str)
                     or self.envelope["admission_author_login"] != (self.comment.get("user") or {}).get("login"))):
            raise ValueError("v2 admission author is not bound to its authenticated server comment")
        identity = self.envelope["identity"]
        expected = {"repository": args.repo, "task_uid": args.task_uid, "issue_number": record["issue_number"],
                    "issue_url": record["issue_url"], "pr_number": binding["pr_number"], "pr_url": binding["pr_url"],
                    "project_id": project_id, "project_item_id": record["project_item_id"],
                    "canonical_worktree": str(self.root), "source_ref": record["task_branch"],
                    "target_ref": record["default_branch"]}
        if identity != expected:
            raise ValueError("recovery admission task identity mismatch")
        self._scope(comments)
        self._helpers()
        self._lineage(comments)
        self.issue_baseline: dict[str, Any] | None = None
        self.project_baseline: dict[str, Any] | None = None
        self.check()

    @staticmethod
    def _closed(value: Any, keys: set[str]) -> None:
        if not isinstance(value, dict) or set(value) != keys:
            raise ValueError("recovery evidence fields are incomplete or unsupported")

    def _comment_identity(self, comment: dict[str, Any], actor: str) -> None:
        if (type(comment.get("id")) is not int
                or comment.get("html_url") != self.record["issue_url"] + "#issuecomment-" + str(comment["id"])
                or comment.get("issue_url") != f"https://api.github.com/repos/{self.args.repo}/issues/{self.record['issue_number']}"
                or (comment.get("user") or {}).get("login") != actor):
            raise ValueError("unauthenticated recovery comment")

    def _scope(self, comments: list[dict[str, Any]]) -> None:
        refs = self.envelope["scope_evidence"]
        if not isinstance(refs, list) or len(refs) != 5 or len({r.get("comment_id") for r in refs}) != 5:
            raise ValueError("incomplete recovery scope chain")
        bodies = []
        for ref in refs:
            self._closed(ref, {"comment_id", "comment_url", "body_sha256", "author_login"})
            matches = [c for c in comments if c.get("id") == ref["comment_id"]]
            if len(matches) != 1:
                raise ValueError("scope comment missing or duplicated")
            c = matches[0]
            self._comment_identity(c, ref["author_login"])
            if c["html_url"] != ref["comment_url"] or self.sha(c["body"].encode()) != ref["body_sha256"]:
                raise ValueError("scope comment digest mismatch")
            bodies.append(c["body"])
        plans = [b for b in bodies if "Plan-Gap Evidence:" in b]
        if len(plans) != 1:
            raise ValueError("unique structured user Plan-Gap evidence required")
        rows, end = json.JSONDecoder(object_pairs_hook=self.module._unique_object).raw_decode(
            plans[0].split("Plan-Gap Evidence:", 1)[1].strip())
        if not isinstance(rows, list):
            raise ValueError("Plan-Gap evidence must be an array")
        keys = {"step_id", "acceptance_refs", "dependencies", "verification_command", "verification_evidence",
                "write_scope", "out_of_scope", "required_role_slices"}
        for row in rows:
            self._closed(row, keys)
            if not all(isinstance(v, str) and v.strip() for v in row.values()):
                raise ValueError("incomplete Plan-Gap row")
        by_step = {row["step_id"]: row for row in rows}
        if len(by_step) != len(rows) or set(by_step) != {"REC-SPEC", "REC-RED", "REC-GREEN", "REC-RESTORE", "REC-DELIVER"}:
            raise ValueError("recovery step scope incomplete")
        paths = set(re.findall(r"scripts/pm/[\w.-]+", by_step["REC-GREEN"]["write_scope"]))
        if paths != {"scripts/pm/github-project-task.py", "scripts/pm/pr_projection_publish.py"}:
            raise ValueError("unapproved recovery helper scope")
        if set(re.findall(r"scripts/pm/[\w.-]+", by_step["REC-RED"]["write_scope"])) != {
                "scripts/pm/github-project-task.test.sh", "scripts/pm/pr_projection_publication.test.py"}:
            raise ValueError("unapproved recovery test scope")
        if ("approved helpers" not in by_step["REC-RESTORE"]["write_scope"]
                or not re.search(r"seven|7", by_step["REC-DELIVER"]["write_scope"], re.I)):
            raise ValueError("recovery restoration/delivery bounds missing")
        if set(re.findall(r"(?:doc|scripts)/[\w./-]+", by_step["REC-SPEC"]["write_scope"])) != {
                "doc/engineering/workflow/source-of-truth.md"}:
            raise ValueError("unapproved recovery specification scope")
        history = "\n".join(bodies)
        if not all(path in history for path in ("scripts/pm/ci-reuse-validation.py", "scripts/pm/ci-reuse-validation.test.py")):
            raise ValueError("original approved CLI path scope missing")
        admissions = [b for b in bodies if re.search(r"REC-GREEN|GREEN ADMITTED", b)
                      and "scripts/pm/github-project-task.py" in b and "Plan-Gap Evidence:" not in b]
        if len(admissions) != 1 or not re.search(r"[0-9a-f]{64}", admissions[0]):
            raise ValueError("explicit frozen RED / helper GREEN admission missing")

    def _local(self, relative: str) -> pathlib.Path:
        path = (self.root / relative).resolve()
        if not path.is_relative_to(self.root / ".pm" / "scratch" / self.args.task_uid):
            raise ValueError("review artifact outside selected task scratch")
        return path

    def _helpers(self) -> None:
        review = self.envelope["helper_review"]
        self._closed(review, {"helper_source_oid", "helper_closure_sha256", "closure_manifest",
                              "ledger_path", "ledger_sha256", "role_returns"})
        head = run_text(["git", "-C", str(self.root), "rev-parse", "HEAD"])
        if review["helper_source_oid"] != head or self.intent["source_head_oid"] != head:
            raise ValueError("reviewed helper source is not current HEAD")
        manifest = review["closure_manifest"]
        if not isinstance(manifest, list) or self.sha(self.encoded(manifest)) != review["helper_closure_sha256"]:
            raise ValueError("helper closure digest mismatch")
        paths = []
        tree = {}
        for line in run_text(["git", "-C", str(self.root), "ls-tree", "-r", head]).splitlines():
            meta, path = line.split("\t", 1); mode, kind, oid = meta.split()
            tree[path] = (mode, kind, oid)
        for row in manifest:
            self._closed(row, {"path", "mode", "blob_oid"})
            path = row["path"]
            if (not isinstance(path, str) or not path.isascii() or pathlib.PurePosixPath(path).is_absolute()
                    or ".." in pathlib.PurePosixPath(path).parts or row["mode"] not in {"100644", "100755"}
                    or tree.get(path) != (row["mode"], "blob", row["blob_oid"])):
                raise ValueError("helper closure tree identity mismatch")
            local = self.root / path
            if local.is_symlink() or not local.is_file():
                raise ValueError("helper closure source unavailable")
            if bool(local.stat().st_mode & 0o111) != (row["mode"] == "100755"):
                raise ValueError("helper closure executable mode mismatch")
            raw = local.read_bytes()
            oid = hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()
            if oid != row["blob_oid"]:
                raise ValueError("helper closure working bytes differ from reviewed source")
            paths.append(path)
        if paths != sorted(set(paths)):
            raise ValueError("helper closure must be sorted and unique")
        # All repository-owned PM code is included, conservatively closing
        # dynamic module and subprocess selection before any metadata effect.
        required = {p for p, (_, kind, _) in tree.items() if p.startswith("scripts/pm/") and kind == "blob"}
        if not required.issubset(paths) or not pathlib.Path(__file__).resolve().is_relative_to(self.root / "scripts/pm"):
            raise ValueError("helper execution closure incomplete")
        ledger = self._local(review["ledger_path"])
        if self.sha(ledger.read_bytes()) != review["ledger_sha256"]:
            raise ValueError("helper review ledger digest mismatch")
        returns = review["role_returns"]
        if not isinstance(returns, list) or sorted(r.get("role", "") for r in returns) != sorted(self.roles):
            raise ValueError("helper review roles incomplete")
        environment = dict(os.environ)
        environment.pop("OASIS7_TEST_ALLOW_UNATTESTED_DISPATCH_RECEIPTS", None)
        for returned in returns:
            self._closed(returned, {"role", "slice_id", "packet_sha256", "source_head_oid", "return_path", "return_sha256", "verdict"})
            packet = self._local(f".pm/scratch/{self.args.task_uid}/slice-packets/{returned['slice_id']}.json")
            artifact = self._local(returned["return_path"])
            if (returned["source_head_oid"] != head or returned["verdict"] != "approved"
                    or self.sha(packet.read_bytes()) != returned["packet_sha256"]
                    or self.sha(artifact.read_bytes()) != returned["return_sha256"]):
                raise ValueError("role return binding mismatch")
            value = json.loads(artifact.read_text())
            for key, expected in {"task_uid": self.args.task_uid, "role": returned["role"],
                    "slice_id": returned["slice_id"], "head": head, "status": "completed",
                    "scope_verdict": "approved", "risk_verdict": "approved", "findings": "no_findings",
                    "helper_source_oid": head, "helper_closure_sha256": review["helper_closure_sha256"]}.items():
                if value.get(key) != expected:
                    raise ValueError("role return does not approve exact helper closure")
            if not getattr(self, "validated_reviews", False):
                subprocess.run([sys.executable, str(self.root / "scripts/pm/subagent-task-packet.py"), "validate", str(packet)],
                               cwd=self.root, env=environment, capture_output=True, text=True, check=True, timeout=30)
        if not getattr(self, "validated_reviews", False):
            subprocess.run([sys.executable, str(self.root / "scripts/pm/validate-review-provenance.py"),
                  "--mode", "human-operated", "--root", str(self.root), "--task-uid", self.args.task_uid,
                  "--ledger", str(ledger), "--roles", ",".join(self.roles), "--source-head", head],
                           cwd=self.root, env=environment, capture_output=True, text=True, check=True, timeout=30)
            self.validated_reviews = True

    def _lineage(self, comments: list[dict[str, Any]]) -> None:
        # A current-only envelope is the separately versioned first-attempt
        # route.  Keep the v1 two-lineage branch below intact; the v2 class
        # uses this narrow journal proof as one input to its larger live
        # admission/closure checks.
        if "predecessor" not in self.envelope:
            self._single_current_lineage(comments)
            return
        from pr_projection_journal import publication_paths
        common = pathlib.Path(run_text(["git", "-C", str(self.root), "rev-parse", "--path-format=absolute", "--git-common-dir"]))
        found = []
        for name in ("current_action", "predecessor"):
            evidence = self.envelope[name]
            self._closed(evidence, {"publication_id", "action_id", "journal_sha256", "H", "B", "S", "D",
                                    "intent_comment_id", "intent_body_sha256"})
            candidates = [c for c in comments if c.get("id") == evidence["intent_comment_id"]]
            if len(candidates) != 1 or self.sha(candidates[0]["body"].encode()) != evidence["intent_body_sha256"]:
                raise ValueError("publication intent digest/identity mismatch")
            self._comment_identity(candidates[0], (self.comment.get("user") or {}).get("login"))
            intent = self.module.parse_publication_comment(candidates[0]["body"])
            unique = [c for c in comments if "<!-- oasis7-ci-publication/v1 -->" in c["body"]
                      and self.module.parse_publication_comment(c["body"])["publication_id"] == intent["publication_id"]]
            if len(unique) != 1:
                raise ValueError("publication lineage intent is not unique")
            for key, field in {"publication_id": "publication_id", "H": "source_head_oid", "B": "planner_authority_oid",
                               "S": "source_scope_oid", "D": "projection_digest"}.items():
                if intent[field] != evidence[key]:
                    raise ValueError("publication lineage tuple mismatch")
            for key in ("repository", "repository_id", "source_repository_id", "task_uid", "source_ref", "target_ref"):
                if intent[key] != self.intent[key]:
                    raise ValueError("publication lineage repository/task/ref mismatch")
            for oid in (evidence["H"], evidence["B"], evidence["S"]):
                if run_text(["git", "-C", str(self.root), "rev-parse", "--verify", oid + "^{commit}"]) != oid:
                    raise ValueError("publication lineage Git identity mismatch")
            path, _ = publication_paths(common, self.args.repo, intent["source_ref"], intent["publication_id"])
            raw = path.read_bytes(); journal = json.loads(raw)
            if raw != self.encoded(journal) + b"\n":
                raise ValueError("publication journal bytes are not canonical producer output")
            if self.sha(raw) != evidence["journal_sha256"]:
                # A confirmed retry preserves the core's observed journal.
                # Prove the exact known observation delta against the admitted
                # raw preimage; arbitrary edits never become lineage evidence.
                # The only additional migration is the legacy root-tail anchor
                # written after exact Task-comment readback. It reconstructs
                # the admitted bytes by removing that one field and validates
                # the complete anchored READ_MATCH/RESOLVED sidecar.
                observed = name == "current_action" and self._observed_preimage(journal, evidence)
                anchored_readback = (
                    name == "current_action"
                    and self._legacy_exact_readback_anchor(
                        path, common, journal, evidence, intent, unique[0],
                    )
                )
                if not (observed or anchored_readback):
                    raise ValueError("raw publication journal hash mismatch")
            expected_identity = dict(repository=self.args.repo, branch=intent["source_ref"], publication_id=intent["publication_id"],
                task_uid=self.args.task_uid, source_head_oid=evidence["H"], scope_base_oid=evidence["S"], projection_digest=evidence["D"])
            if journal.get("schema") != "oasis7-pr-publication-journal/v1" or journal.get("identity") != expected_identity:
                raise ValueError("publication journal identity mismatch")
            actions = [a for a in journal["actions"] if a.get("action_id") == evidence["action_id"]]
            if (len(actions) != 1 or evidence["action_id"] != "record-pr:" + intent["publication_id"]
                    or actions[0].get("kind") != "record_pr" or actions[0].get("expected") != {
                        "publication_id": intent["publication_id"], "task_uid": self.args.task_uid, "pr_number": self.binding["pr_number"]}
                    or actions[0].get("state") not in ({"uncertain"} if name == "predecessor" else {"intent", "uncertain", "observed"})):
                raise ValueError("publication lineage action is not authentic")
            action = actions[0]
            if (set(action) - {"action_id", "kind", "expected", "state", "observed"}
                    or (action["state"] == "intent" and "observed" in action)
                    or (action["state"] == "observed" and "observed" not in action)
                    or ("observed" in action and action["observed"] != {"pr_number": self.binding["pr_number"]})):
                raise ValueError("publication record action observation is malformed")
            if name == "current_action":
                bindings = self._binding_actions(journal)
                if (journal.get("phase") not in {"PREPARED", "HEAD_CONFIRMED", "METADATA_CONFIRMED"}
                        or journal.get("disposition") not in {None, "NETWORK_UNCERTAIN"}):
                    raise ValueError("current publication phase/disposition is malformed")
                if bindings and action.get("observed") != {"pr_number": self.binding["pr_number"]}:
                    raise ValueError("binding action precedes confirmed record-pr observation")
            found.append(intent)
        if found[0] != self.intent or found[0]["publication_id"] == found[1]["publication_id"]:
            raise ValueError("current/predecessor publication identity conflict")
        run_text(["git", "-C", str(self.root), "merge-base", "--is-ancestor", found[1]["source_head_oid"], found[0]["source_head_oid"]])

    def _single_current_lineage(self, comments: list[dict[str, Any]]) -> None:
        """Validate one exact current uncertain record-pr action without history."""
        from pr_projection_journal import publication_paths

        evidence = self.envelope["current_action"]
        evidence_keys = {"publication_id", "action_id", "journal_sha256", "H", "B", "S", "D",
                         "intent_comment_id", "intent_body_sha256"}
        if self.marker == "<!-- oasis7-publication-recovery-admission/v2 -->":
            evidence_keys.add("intent_author_login")
        self._closed(evidence, evidence_keys)
        if evidence["action_id"] != "record-pr:" + evidence["publication_id"]:
            raise ValueError("single recovery action selector is malformed")
        candidates = [c for c in comments if c.get("id") == evidence["intent_comment_id"]]
        if len(candidates) != 1 or self.sha(candidates[0]["body"].encode()) != evidence["intent_body_sha256"]:
            raise ValueError("single publication intent digest/identity mismatch")
        candidate = candidates[0]
        actor = (evidence.get("intent_author_login")
                 or (self.comment.get("user") or {}).get("login"))
        self._comment_identity(candidate, actor)
        intent = self.module.parse_publication_comment(candidate["body"])
        if intent != self.intent or intent["publication_id"] != evidence["publication_id"]:
            raise ValueError("single publication intent differs from the current binding")
        if any(c.get("id") != candidate.get("id")
               and "<!-- oasis7-ci-publication/v1 -->" in str(c.get("body") or "")
               and self.module.parse_publication_comment(c["body"]).get("publication_id") == intent["publication_id"]
               for c in comments):
            raise ValueError("single publication intent is duplicated")
        for key, field in {"publication_id": "publication_id", "H": "source_head_oid",
                           "B": "planner_authority_oid", "S": "source_scope_oid",
                           "D": "projection_digest"}.items():
            if intent[field] != evidence[key]:
                raise ValueError("single publication H/B/S/D tuple mismatch")
        for key in ("repository", "repository_id", "source_repository_id", "task_uid", "source_ref", "target_ref"):
            if intent[key] != self.intent[key]:
                raise ValueError("single publication repository/task/ref mismatch")
        for oid in (evidence["H"], evidence["B"], evidence["S"]):
            if run_text(["git", "-C", str(self.root), "rev-parse", "--verify", oid + "^{commit}"]) != oid:
                raise ValueError("single publication Git identity mismatch")
        common = pathlib.Path(run_text(["git", "-C", str(self.root), "rev-parse", "--path-format=absolute", "--git-common-dir"]))
        path, _ = publication_paths(common, self.args.repo, intent["source_ref"], intent["publication_id"])
        raw = path.read_bytes()
        journal = json.loads(raw)
        if raw != self.encoded(journal) + b"\n" or self.sha(raw) != evidence["journal_sha256"]:
            # Only the existing exact legacy task-post anchor can explain a
            # post-intent tail write; all other journal bytes stay immutable.
            if not self._legacy_exact_readback_anchor(path, common, journal, evidence, intent, candidate):
                raise ValueError("single publication raw journal digest mismatch")
        expected_identity = {
            "repository": self.args.repo, "branch": intent["source_ref"],
            "publication_id": intent["publication_id"], "task_uid": self.args.task_uid,
            "source_head_oid": evidence["H"], "scope_base_oid": evidence["S"],
            "projection_digest": evidence["D"],
        }
        if journal.get("schema") != "oasis7-pr-publication-journal/v1" or journal.get("identity") != expected_identity:
            raise ValueError("single publication journal identity mismatch")
        actions = [a for a in journal.get("actions", []) if a.get("action_id") == evidence["action_id"]]
        record_actions = [a for a in journal.get("actions", []) if a.get("kind") == "record_pr"]
        if len(actions) != 1 or len(record_actions) != 1:
            raise ValueError("single publication requires exactly one current record-pr action")
        action = actions[0]
        if (set(action) - {"action_id", "kind", "expected", "state", "observed"}
                or action.get("kind") != "record_pr"
                or action.get("expected") != {"publication_id": intent["publication_id"],
                                               "task_uid": self.args.task_uid,
                                               "pr_number": self.binding["pr_number"]}
                or action.get("state") != "uncertain"
                or (action["state"] == "intent" and "observed" in action)
                or ("observed" in action and action["observed"] != {"pr_number": self.binding["pr_number"]})):
            raise ValueError("single current record-pr action is not authentic and uncertain")
        binds = self._binding_actions(journal)
        if (journal.get("phase") not in {"PREPARED", "HEAD_CONFIRMED", "METADATA_CONFIRMED"}
                or journal.get("disposition") not in {None, "NETWORK_UNCERTAIN"}
                or (binds and action.get("observed") != {"pr_number": self.binding["pr_number"]})):
            raise ValueError("single publication phase/disposition is malformed")

    def _legacy_exact_readback_anchor(self, path: pathlib.Path, common: pathlib.Path,
                                      journal: dict[str, Any], evidence: dict[str, Any],
                                      intent: dict[str, Any], task_comment: dict[str, Any]) -> bool:
        """Recognize only the exact legacy-tail addition proven by Task readback."""
        from pr_projection_journal import (
            JournalError, PublicationJournal, TASK_POST_EVENTS_FILE, TASK_POST_TAIL_SCHEMA,
        )

        try:
            tail = journal.get("task_post_tail")
            if (not isinstance(tail, dict) or set(tail) != {"schema", "sequence", "digest"}
                    or tail.get("schema") != TASK_POST_TAIL_SCHEMA
                    or type(tail.get("sequence")) is not int or tail["sequence"] < 0
                    or not isinstance(tail.get("digest"), str)
                    or re.fullmatch(r"sha256:[0-9a-f]{64}", tail["digest"]) is None):
                return False

            # Removing exactly the anchor must recreate the admitted canonical
            # root bytes. Any changed action, phase, identity, or other field
            # therefore remains outside this compatibility exception.
            preimage = dict(journal)
            del preimage["task_post_tail"]
            if self.sha(self.encoded(preimage) + b"\n") != evidence["journal_sha256"]:
                return False

            events_path = path.with_name(TASK_POST_EVENTS_FILE)
            exists = events_path.exists()
            event_raw = events_path.read_bytes() if exists else b""
            if not exists or not event_raw.endswith(b"\n"):
                return False
            reader = PublicationJournal(path, path.parent / "publisher.lock", journal["identity"],
                                        common_dir=common, canonical_worktree=self.root)
            events = reader._decode_task_events(event_raw, exists=exists)
            # The sidecar is producer-canonical, not merely semantically
            # equivalent JSON. This also rejects duplicate keys and byte drift.
            for line, event in zip(event_raw.splitlines(keepends=True), events):
                if line != self.encoded(event) + b"\n":
                    return False
            if (tail["sequence"] != len(events)
                    or tail["digest"] != "sha256:" + self.sha(event_raw)
                    or len(events) < 2):
                return False

            publication_id = intent["publication_id"]
            action_id = "task-intent:" + publication_id
            read_match, resolved = events[-2:]
            if (read_match.get("event") != "READ_MATCH"
                    or resolved.get("event") != "RESOLVED"
                    or read_match.get("action_id") != action_id
                    or resolved.get("action_id") != action_id
                    or read_match.get("identity") != resolved.get("identity")):
                return False

            actor = (self.comment.get("user") or {}).get("login")
            self._comment_identity(task_comment, actor)
            if task_comment.get("body") != self.module.publication_comment(intent):
                return False
            identity = read_match.get("identity")
            payload = task_comment["body"]
            expected = {
                "schema": "oasis7-pr-task-post-action/v1",
                "repository": intent["repository"],
                "repository_id": intent["repository_id"],
                "task_issue_number": self.record["issue_number"],
                "task_uid": self.args.task_uid,
                "bootstrap_epoch": intent["bootstrap_epoch"],
                "publication": intent,
                "canonical_worktree": str(self.root.resolve()),
                "git_common_dir": str(common.resolve()),
                "source_ref": intent["source_ref"],
                "target_ref": intent["target_ref"],
                "source_head_oid": intent["source_head_oid"],
                "scope_base_oid": intent["source_scope_oid"],
                "planner_authority_oid": intent["planner_authority_oid"],
                "planner_config_sha256": intent["planner_config_sha256"],
                "policy_digest": intent["policy_digest"],
                "projection_digest": intent["projection_digest"],
                "publisher_login": actor,
                "payload_utf8": payload,
                "payload_sha256": "sha256:" + self.sha(payload.encode("utf-8")),
            }
            if (not isinstance(identity, dict)
                    or set(identity) != set(expected) | {"pr_binding"}
                    or any(identity.get(key) != value for key, value in expected.items())):
                return False
            pr_binding = identity.get("pr_binding")
            if not isinstance(pr_binding, dict):
                return False
            # Accept only exact producer encodings. The create producer emits
            # state=unbound or state=bound/pr_number; the update producer also
            # pins its unbound candidate PR. Retain the older task_uid/pr_number
            # shape for journals written before these producer encodings.
            producer_binding = (
                pr_binding == {"state": "unbound"}
                or (set(pr_binding) == {"state", "pr_number"}
                    and pr_binding.get("state") == "bound"
                    and type(pr_binding.get("pr_number")) is int
                    and pr_binding["pr_number"] == self.binding["pr_number"])
                or (set(pr_binding) == {"state", "candidate_pr_number"}
                    and pr_binding.get("state") == "unbound"
                    and type(pr_binding.get("candidate_pr_number")) is int
                    and pr_binding["candidate_pr_number"] == self.binding["pr_number"])
            )
            legacy_binding = (
                set(pr_binding) == {"task_uid", "pr_number"}
                and pr_binding.get("task_uid") == self.args.task_uid
                and (pr_binding.get("pr_number") is None
                     or (type(pr_binding.get("pr_number")) is int
                         and pr_binding["pr_number"] == self.binding["pr_number"]))
            )
            if not producer_binding and not legacy_binding:
                return False
            if (read_match.get("evidence") != {
                    "phase": "prewrite", "publication_id": publication_id,
                    "author_login": actor, "payload_sha256": expected["payload_sha256"],
                }
                    or resolved.get("evidence") != {"publication_id": publication_id}):
                return False
            return True
        except (JournalError, OSError, ValueError, KeyError, TypeError, UnicodeError):
            return False


    def _binding_actions(self, journal: dict[str, Any]) -> list[dict[str, Any]]:
        binding_id = "reciprocal-binding:" + self.binding["publication_id"]
        binds = [a for a in journal["actions"] if a.get("action_id") == binding_id]
        if len(binds) > 1:
            raise ValueError("duplicate reciprocal binding journal action")
        for action in binds:
            if (set(action) - {"action_id", "kind", "expected", "state", "observed"}
                    or action.get("kind") != "publish_reciprocal_binding"
                    or action.get("expected") != {"publication_id": self.binding["publication_id"],
                        "pr_number": self.binding["pr_number"], "binding_digest": self.binding["binding_digest"]}
                    or action.get("state") not in {"intent", "uncertain", "observed"}
                    or (action.get("state") == "intent" and "observed" in action)
                    or (action.get("state") == "observed" and "observed" not in action)
                    or ("observed" in action and action["observed"] != {"binding_digest": self.binding["binding_digest"]})):
                raise ValueError("reciprocal binding journal action is malformed")
        return binds

    def _observed_preimage(self, journal: dict[str, Any], evidence: dict[str, Any]) -> bool:
        binds = self._binding_actions(journal)
        if (journal.get("phase") not in {"PREPARED", "HEAD_CONFIRMED", "METADATA_CONFIRMED"}
                or journal.get("disposition") not in {None, "NETWORK_UNCERTAIN"}):
            return False
        copy = json.loads(json.dumps(journal))
        # _intent and _push reset the global fields independently of retained
        # metadata observations. Prove that exact reset without changing any
        # action, rather than treating an aggregate phase as action authority.
        for phase in ("PREPARED", "HEAD_CONFIRMED", "METADATA_CONFIRMED"):
            for disposition in (None, "NETWORK_UNCERTAIN"):
                copy["phase"], copy["disposition"] = phase, disposition
                if self.sha(self.encoded(copy) + b"\n") == evidence["journal_sha256"]:
                    return True
        actions = copy.get("actions", [])
        current = [a for a in actions if a.get("action_id") == evidence["action_id"]]
        if len(current) != 1 or current[0].get("state") not in {"observed", "uncertain"} or current[0].get("observed") != {"pr_number": self.binding["pr_number"]}:
            return False
        binding_id = "reciprocal-binding:" + evidence["publication_id"]
        binding_variants = [json.loads(json.dumps(binds)), []]
        if binds:
            for state in ("intent", "uncertain"):
                pending = json.loads(json.dumps(binds))
                pending[0]["state"] = state
                pending[0].pop("observed", None)
                binding_variants.append(pending)
        non_binding = [a for a in actions if a.get("action_id") != binding_id]
        binding_position = next((i for i, a in enumerate(actions) if a.get("action_id") == binding_id), len(actions))
        current[0].pop("observed")
        # These are the only prior states admitted for the current action.
        # Hash equality proves every other byte/value of the original journal.
        for binding_variant in binding_variants:
            copy["actions"] = non_binding[:binding_position] + binding_variant + non_binding[binding_position:]
            for state, disposition in (("uncertain", "NETWORK_UNCERTAIN"), ("uncertain", None), ("intent", None)):
                current[0]["state"] = state
                copy["disposition"] = disposition
                for phase in ("PREPARED", "HEAD_CONFIRMED", "METADATA_CONFIRMED"):
                    copy["phase"] = phase
                    if self.sha(self.encoded(copy) + b"\n") == evidence["journal_sha256"]:
                        return True
        return False

    def check(self, *, final: bool = False, pre_admission: bool = False) -> tuple[dict[str, Any], dict[str, str]]:
        args = self.args
        actor = json.loads(run_text(["gh", "api", "user"])).get("login")
        if not pre_admission:
            self._comment_identity(self.comment, actor)
        if run_text(["git", "-C", str(self.root), "rev-parse", "HEAD"]) != self.intent["source_head_oid"]:
            raise ValueError("current HEAD drift during recovery")
        owner, repo = args.repo.split("/")
        query = '''query($owner:String!,$repo:String!,$issue:Int!,$ids:[ID!]!) {
          repository(owner:$owner,name:$repo) { issue(number:$issue) { number url body state viewerCanUpdate } }
          nodes(ids:$ids) { ... on ProjectV2Item { id project { id number viewerCanUpdate owner { ... on User { login } ... on Organization { login } } }
            content { __typename ... on Issue { number url repository { nameWithOwner } } }
            fieldValues(first:100) { pageInfo { hasNextPage } nodes {
              __typename
              ... on ProjectV2ItemFieldTextValue { text field { ... on ProjectV2FieldCommon { name } } }
              ... on ProjectV2ItemFieldSingleSelectValue { name field { ... on ProjectV2FieldCommon { name } } }
              ... on ProjectV2ItemFieldRepositoryValue { repository { id nameWithOwner } field { ... on ProjectV2FieldCommon { name } } }
            } } } }
        }'''
        data = project_refresh_graphql(
            query,
            {"owner": owner, "repo": repo, "issue": self.record["issue_number"],
             "ids": [self.record["project_item_id"]]},
            operation="project_task_publication_recovery_readback",
            task_uid=args.task_uid,
        )
        if data.get("errors"):
            raise ValueError("permission/readback GraphQL uncertainty")
        data = data.get("data") or {}; issue_permission = (data.get("repository") or {}).get("issue") or {}
        nodes = data.get("nodes") or []
        if len(nodes) != 1 or nodes[0].get("id") != self.record["project_item_id"]:
            raise ValueError("selected Project item identity mismatch")
        node = nodes[0]
        refresh_project_identity(node, args.project_owner, args.project_number, self.record["project_id"])
        field_values = node.get("fieldValues")
        if (not isinstance(field_values, dict)
                or not isinstance(field_values.get("pageInfo"), dict)
                or field_values["pageInfo"].get("hasNextPage") is not False
                or not isinstance(field_values.get("nodes"), list)):
            raise ValueError("Project field readback is incomplete")
        content = node.get("content")
        if (not isinstance(content, dict) or content.get("__typename") != "Issue"
                or type(content.get("number")) is not int
                or content["number"] != self.record["issue_number"]
                or content.get("url") != self.record["issue_url"]
                or not isinstance(content.get("repository"), dict)
                or content["repository"].get("nameWithOwner") != args.repo):
            raise ValueError("selected Project item content does not match canonical Task Issue")
        if (issue_permission.get("number") != self.record["issue_number"]
                or issue_permission.get("url") != self.record["issue_url"]
                or issue_permission.get("state") != "OPEN" or issue_permission.get("viewerCanUpdate") is not True
                or node["project"].get("viewerCanUpdate") is not True):
            raise ValueError("fresh Issue/Project writer permissions unavailable")
        values = {}
        for row in node["fieldValues"]["nodes"]:
            name = (row.get("field") or {}).get("name")
            if not name or name in values:
                raise ValueError("ambiguous Project field")
            if row.get("__typename") == "ProjectV2ItemFieldRepositoryValue":
                repository = row.get("repository")
                if (not isinstance(repository, dict) or set(repository) != {"id", "nameWithOwner"}
                        or not isinstance(repository["id"], str) or not repository["id"].strip()
                        or not isinstance(repository["nameWithOwner"], str)
                        or re.fullmatch(r"[^/\s]+/[^/\s]+", repository["nameWithOwner"]) is None):
                    raise ValueError("malformed Project Repository field")
                values[name] = {"type": "repository", "id": repository["id"],
                                "name_with_owner": repository["nameWithOwner"]}
            elif row.get("__typename") not in (None, "ProjectV2ItemFieldTextValue", "ProjectV2ItemFieldSingleSelectValue"):
                raise ValueError("unsupported Project field value type")
            else:
                values[name] = row.get("name", row.get("text", ""))
        raw_issue = (json.loads(run_text(["gh", "api", f"repos/{args.repo}/issues/{self.record['issue_number']}"]))
                     if final else {"body": issue_permission.get("body"), "state": issue_permission.get("state", "").lower()})
        raw_body = raw_issue.get("body")
        if not isinstance(raw_body, str):
            raise ValueError("malformed authoritative Issue body")
        for key in self.issue_owned:
            lines = re.findall(rf"^- {key}:.*$", raw_body, re.M)
            if len(lines) > 1 or any(not re.fullmatch(rf"- {key}: `[^`\n]+`", line) for line in lines):
                raise ValueError("ambiguous Issue publication metadata")
        unrelated_body = re.sub(r"^- (?:status|workflow_phase|pr_number|pr_url):.*\n?", "", raw_body, flags=re.M)
        if hasattr(self, "unrelated_body") and self.unrelated_body != unrelated_body:
            raise ValueError("unrelated Issue body changed during recovery")
        self.unrelated_body = unrelated_body
        self.live_body = raw_body
        live = github_issue_record(args.repo, args.task_uid)
        if not live or raw_issue.get("state") != "open":
            raise ValueError("fresh Task Issue is not OPEN")
        adjusted = dict(self.record)
        for key in self.issue_owned:
            adjusted[key] = live.get(key)
        validate_record_pr_live_identity(args, adjusted, self.binding["pr_number"])
        pr = github_pull_request(args.repo, self.binding["pr_number"])
        if not has_exact_task_pr_linkage(pr.get("body"), args.task_uid, self.record["issue_number"]):
            raise ValueError("live PR Task/Refs drift")
        projection = self.module.decode_marker(pr.get("body"))
        if any(projection.get(k) != v for k, v in {"task_uid": args.task_uid,
                "source_head_oid": self.intent["source_head_oid"], "scope_base_oid": self.intent["source_scope_oid"],
                "projection_digest": self.intent["projection_digest"]}.items()):
            raise ValueError("live PR projection drift")
        issue_post = {"status": "committed", "workflow_phase": "verification",
                      "pr_number": self.binding["pr_number"], "pr_url": self.binding["pr_url"]}
        issue_pre = {"status": "committed", "workflow_phase": "execution", "pr_number": None, "pr_url": None}
        for key, post in issue_post.items():
            actual = live.get(key) or None
            if key == "pr_number" and actual is not None:
                actual = int(actual)
            if actual != post and (final or actual != issue_pre[key]):
                raise ValueError("Issue field outside trusted pre/post transition: " + key)
        project_post = {"Status": "In Progress", "PM Status": "committed", "Workflow Phase": "verification", "PR": self.binding["pr_url"]}
        project_pre = {**project_post, "Workflow Phase": "execution", "PR": ""}
        for key, post in project_post.items():
            actual = values.get(key) or ""
            if actual != post and (final or actual != project_pre[key]):
                raise ValueError("Project field outside trusted pre/post transition: " + key)
        unrelated_issue = {k: live.get(k) for k in ("task_uid", "owner_role", "module", "priority", "worktree_hint", "primary_package")}
        unrelated_project = {k: v for k, v in values.items() if k not in self.project_owned}
        snapshots = {"issue_sha256": self.sha(self.encoded(unrelated_issue)),
                     "project_sha256": self.sha(self.encoded(unrelated_project))}
        if pre_admission:
            self.envelope["unrelated_snapshot"] = snapshots
        else:
            expected = self.envelope["unrelated_snapshot"]
            self._closed(expected, {"issue_sha256", "project_sha256"})
            if expected != snapshots:
                raise ValueError("unrelated Issue/Project snapshot drift")
        # Preserve every non-owned Issue field, beyond the compact admission
        # snapshot. Observation timestamps are not task fields.
        baseline = {k: v for k, v in live.items() if k not in self.issue_owned | {"updated_at"}}
        if self.issue_baseline is None:
            self.issue_baseline = baseline
        elif baseline != self.issue_baseline:
            raise ValueError("unrelated Issue content changed during recovery")
        current_comments = github_issue_comments(args.repo, self.record["issue_number"])
        if not pre_admission:
            admitted = [c for c in current_comments if self.marker in c["body"]]
            if len(admitted) != 1 or admitted[0]["body"] != self.envelope_body:
                raise ValueError("current recovery action admission drift")
            self._comment_identity(admitted[0], actor)
        self._scope(current_comments)
        self._lineage(current_comments)
        self._helpers()
        if final:
            bindings = [self.module.parse_publication_binding_comment(c["body"]) for c in current_comments
                        if "<!-- oasis7-ci-publication-binding/v1 -->" in c["body"]]
            matches = [b for b in bindings if b["publication_id"] == self.binding["publication_id"]]
            if matches != [self.binding]:
                raise ValueError("exact unique final reciprocal binding missing")
        return live, values


class SinglePublicationRecoveryAuthority(PublicationRecoveryAuthority):
    """Independent consumer for one exact current uncertain record-pr action."""

    marker = "<!-- oasis7-publication-recovery-admission/v2 -->"
    roles = ("runtime_engineer", "repository_health_engineer", "qa_engineer")

    def _scope(self, comments: list[dict[str, Any]]) -> None:
        request_ref = self.envelope["recovery_request"]
        plan_ref = self.envelope["plan_scope"]
        self._closed(request_ref, {"comment_id", "comment_url", "body_sha256", "author_login"})
        self._closed(plan_ref, {"comment_id", "comment_url", "body_sha256", "author_login"})
        if request_ref["author_login"] != plan_ref["author_login"]:
            raise ValueError("recovery request author is not the approved Plan-Gap author")
        if request_ref["comment_id"] == plan_ref["comment_id"]:
            raise ValueError("recovery request cannot authorize itself as Plan-Gap scope")
        request_matches = [c for c in comments if c.get("id") == request_ref["comment_id"]]
        plan_matches = [c for c in comments if c.get("id") == plan_ref["comment_id"]]
        if len(request_matches) != 1 or len(plan_matches) != 1:
            raise ValueError("recovery request or approved Plan-Gap comment is missing/duplicated")
        request_comment, plan_comment = request_matches[0], plan_matches[0]
        for comment, ref in ((request_comment, request_ref), (plan_comment, plan_ref)):
            self._comment_identity(comment, ref["author_login"])
            if (comment.get("html_url") != ref["comment_url"]
                    or self.sha(comment["body"].encode("utf-8")) != ref["body_sha256"]):
                raise ValueError("recovery scope comment readback mismatch")
        self.request_comment = request_comment
        self.plan_comment = plan_comment
        marker = "<!-- oasis7-publication-recovery-request/v1 -->"
        all_requests = [c for c in comments if marker in c["body"]]
        if len(all_requests) != 1 or all_requests[0] != request_comment:
            raise ValueError("exactly one independent user recovery request is required")
        match = re.fullmatch(re.escape(marker) + r"\s*```json\s*\n(.*?)\n```\s*", request_comment["body"], re.S)
        if not match:
            raise ValueError("malformed user recovery request")
        request = json.loads(match[1], object_pairs_hook=self.module._unique_object)
        self._closed(request, {"schema", "operation", "repository", "task_uid", "issue_number", "issue_url",
                               "pr_number", "pr_url", "publication_id", "action_id"})
        canonical_request = self.encoded(request).decode("utf-8")
        if (request_comment["body"] != marker + "\n```json\n" + canonical_request + "\n```"
                or request.get("schema") != "oasis7-publication-recovery-request/v1"
                or request.get("operation") != "resume_uncertain_record_pr"):
            raise ValueError("user recovery request is not canonical or supported")
        action = self.envelope["current_action"]
        expected_request = {"repository": self.args.repo, "task_uid": self.args.task_uid,
                            "issue_number": self.record["issue_number"], "issue_url": self.record["issue_url"],
                            "pr_number": self.binding["pr_number"], "pr_url": self.binding["pr_url"],
                            "publication_id": action["publication_id"], "action_id": action["action_id"]}
        if any(request.get(key) != value for key, value in expected_request.items()):
            raise ValueError("user recovery request identity/action mismatch")

        plan_marker = "Plan-Gap Evidence:"
        plans = [c for c in comments if plan_marker in c["body"]]
        if len(plans) != 1 or plans[0] != plan_comment:
            raise ValueError("unique approved live Plan-Gap scope is required")
        plan_payload = plan_comment["body"].split(plan_marker, 1)[1].strip()
        rows, _ = json.JSONDecoder(object_pairs_hook=self.module._unique_object).raw_decode(plan_payload)
        if not isinstance(rows, list) or not rows or any(not isinstance(row, dict) for row in rows):
            raise ValueError("approved Plan-Gap scope is malformed")
        scope_paths = set()
        for row in rows:
            scope_paths.update(re.findall(r"scripts/pm/[\w.-]+", str(row.get("write_scope") or "")))
        required_paths = {"scripts/pm/github-project-task.py", "scripts/pm/pr_projection_publish.py",
                          "scripts/pm/pr_projection_publication.py"}
        if not required_paths.issubset(scope_paths):
            raise ValueError("approved live Plan-Gap scope does not include this recovery action")
        refs = [c for c in comments if self.marker in c["body"]]
        if len(refs) > 1:
            raise ValueError("multiple current v2 recovery admissions are ambiguous")

    def _helpers(self) -> None:
        self.validated_reviews = True
        review = self.envelope["helper_review"]
        self._closed(review, {"helper_task_uid", "archive_manifest_sha256", "helper_source_oid",
                              "helper_closure_sha256", "closure_manifest", "plan_path", "plan_sha256",
                              "ledger_path", "ledger_sha256", "role_returns"})
        validate_helper_review_archive(self, review)

    def _lineage(self, comments: list[dict[str, Any]]) -> None:
        if "predecessor" in self.envelope:
            raise ValueError("v2 single-publication recovery cannot carry predecessor evidence")
        super()._single_current_lineage(comments)

    def check(self, *, final: bool = False, pre_admission: bool = False) -> tuple[dict[str, Any], dict[str, str]]:
        validate_recovery_pr(self)
        return super().check(final=final, pre_admission=pre_admission)


HELPER_REVIEW_ARCHIVE = "publication-helper-review-v1"
HELPER_REVIEW_ARCHIVE_SCHEMA = "oasis7-publication-helper-review-archive/v1"


def effective_helper_root() -> tuple[pathlib.Path, str, list[dict[str, str]], str]:
    """Resolve and hash the canonical default-branch helper execution closure."""
    script = pathlib.Path(__file__).resolve(strict=True)
    root = script.parents[2].resolve(strict=True)
    if pathlib.Path(run_text(["git", "-C", str(root), "rev-parse", "--show-toplevel"])).resolve() != root:
        raise ValueError("effective recovery helper is outside its Git worktree")
    branch = run_text(["git", "-C", str(root), "branch", "--show-current"])
    default_ref = run_text(["git", "-C", str(root), "symbolic-ref", "refs/remotes/origin/HEAD"])
    default_branch = default_ref.removeprefix("refs/remotes/origin/")
    if not branch or branch != default_branch:
        raise ValueError("effective recovery helper is not running from the canonical default branch")
    if run_text(["git", "-C", str(root), "status", "--porcelain=v1", "--untracked-files=all", "--", "scripts/pm/"]):
        raise ValueError("effective scripts/pm helper tree is not clean")
    head = run_text(["git", "-C", str(root), "rev-parse", "HEAD"])
    manifest = pm_closure_manifest(root, head, verify_worktree=True)
    return root, head, manifest, hashlib.sha256(PublicationRecoveryAuthority.encoded(manifest)).hexdigest()


def pm_closure_manifest(root: pathlib.Path, commit: str, *, verify_worktree: bool) -> list[dict[str, str]]:
    tree: dict[str, tuple[str, str, str]] = {}
    for line in run_text(["git", "-C", str(root), "ls-tree", "-r", commit]).splitlines():
        meta, path = line.split("\t", 1)
        mode, kind, oid = meta.split()
        if path.startswith("scripts/pm/"):
            if mode not in {"100644", "100755"} or kind != "blob":
                raise ValueError("effective scripts/pm closure contains a non-regular path")
            tree[path] = (mode, kind, oid)
    if not tree:
        raise ValueError("effective scripts/pm closure is empty")
    result = []
    for path, (mode, _kind, oid) in sorted(tree.items()):
        if verify_worktree:
            local = root / path
            if local.is_symlink() or not local.is_file():
                raise ValueError("effective helper closure member is unavailable")
            if bool(local.stat().st_mode & 0o111) != (mode == "100755"):
                raise ValueError("effective helper closure mode differs from its tracked source")
            raw = local.read_bytes()
            actual_oid = hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()
            if actual_oid != oid:
                raise ValueError("effective helper closure bytes differ from its tracked source")
        result.append({"path": path, "mode": mode, "blob_oid": oid})
    return result


def unique_json_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            raise ValueError("duplicate JSON key")
        value[key] = item
    return value


def strict_json(path: pathlib.Path) -> tuple[dict[str, Any], bytes]:
    if path.is_symlink() or not path.is_file():
        raise ValueError(f"JSON evidence is missing or uses symlink substitution: {path}")
    raw = path.read_bytes()
    value = json.loads(raw, object_pairs_hook=unique_json_object)
    if not isinstance(value, dict):
        raise ValueError(f"expected JSON object: {path}")
    return value, raw


def canonical_receipt_root(root: pathlib.Path, task_uid: str) -> pathlib.Path:
    helper = root / "scripts/pm/canonical-receipt-root.py"
    raw = run_text([sys.executable, str(helper), "--default-worktree", str(root), "--task-uid", task_uid, "--json"])
    value = json.loads(raw, object_pairs_hook=unique_json_object)
    if not isinstance(value, dict) or value.get("task_uid") != task_uid:
        raise ValueError("canonical receipt root identity is malformed")
    common = pathlib.Path(run_text([
        "git", "-C", str(root), "rev-parse", "--path-format=absolute", "--git-common-dir",
    ])).resolve()
    receipts = common / "oasis7-workflow-receipts"
    expected = receipts / task_uid
    if receipts.is_symlink() or expected.is_symlink():
        raise ValueError("canonical receipt root must not use symlink substitution")
    path = pathlib.Path(str(value.get("receipt_root") or "")).resolve(strict=True)
    if path != expected.resolve(strict=True) or path.parent != receipts.resolve() or path.name != task_uid:
        raise ValueError("receipt root is outside the canonical task layout")
    return path


def validate_recovery_pr(authority: SinglePublicationRecoveryAuthority) -> dict[str, Any]:
    """Re-read the exact open draft PR before every recovery effect."""
    intent, binding = authority.intent, authority.binding
    pr = github_pull_request(authority.args.repo, binding["pr_number"])
    base, head = pr.get("base") or {}, pr.get("head") or {}
    repository = head.get("repo") or {}
    if (type(pr.get("number")) is not int or pr["number"] != binding["pr_number"]
            or pr.get("html_url") != binding["pr_url"] or pr.get("state") != "open"
            or pr.get("merged") is not False or pr.get("draft") is not True
            or repository.get("full_name") != authority.args.repo
            or head.get("ref") != intent["source_ref"] or head.get("sha") != intent["source_head_oid"]
            or base.get("ref") != intent["target_ref"]
            or (base.get("repo") or {}).get("full_name") != authority.args.repo):
        raise ValueError("single recovery requires the exact open unmerged draft PR")
    if not has_exact_task_pr_linkage(pr.get("body"), authority.args.task_uid, authority.record["issue_number"]):
        raise ValueError("single recovery live PR Task/Refs identity mismatch")
    projection = authority.module.decode_marker(pr.get("body"))
    if any(projection.get(k) != v for k, v in {
            "task_uid": authority.args.task_uid,
            "source_head_oid": intent["source_head_oid"],
            "scope_base_oid": intent["source_scope_oid"],
            "projection_digest": intent["projection_digest"]}.items()):
        raise ValueError("single recovery live PR projection differs from the publication intent")
    return pr


def _project_task_identity(root: pathlib.Path, task_uid: str, record: dict[str, Any], repo: str) -> None:
    mapping = load_mapping(root / ".pm/github-project-sync/tasks.json")
    project = mapping.get("project") or {}
    project_id = str(project.get("id") or "")
    owner = str(project.get("owner") or "")
    number = project.get("number")
    item_id = str(record.get("project_item_id") or "")
    if (not project_id or not owner or type(number) is not int or number < 1 or not item_id
            or record.get("task_uid") != task_uid or record.get("repository") != repo):
        raise ValueError("canonical Project-backed Task identity is incomplete")
    query = """query($ids:[ID!]!) { nodes(ids:$ids) { ... on ProjectV2Item { id
      project { id number owner { ... on User { login } ... on Organization { login } } }
      content { __typename ... on Issue { number url repository { nameWithOwner } } }
      fieldValues(first:100) { pageInfo { hasNextPage } nodes { __typename field { ... on ProjectV2FieldCommon { name } }
        ... on ProjectV2ItemFieldTextValue { text } ... on ProjectV2ItemFieldSingleSelectValue { name } } } } } }"""
    data = project_refresh_graphql(query, {"ids": [item_id]}, operation="publication_recovery_task_identity",
                                   task_uid=task_uid).get("data") or {}
    nodes = data.get("nodes") or []
    if len(nodes) != 1 or not isinstance(nodes[0], dict):
        raise ValueError("live Project item is missing or ambiguous")
    item = nodes[0]
    live_project = item.get("project") or {}
    content = item.get("content") or {}
    owner_value = live_project.get("owner") or {}
    if (item.get("id") != item_id or live_project.get("id") != project_id
            or live_project.get("number") != number or owner_value.get("login") != owner
            or content.get("__typename") != "Issue" or content.get("number") != record.get("issue_number")
            or content.get("url") != record.get("issue_url")
            or (content.get("repository") or {}).get("nameWithOwner") != repo
            or ((item.get("fieldValues") or {}).get("pageInfo") or {}).get("hasNextPage") is not False):
        raise ValueError("live Project item does not match the canonical Task")


def _task_record(root: pathlib.Path, task_uid: str, repo: str) -> dict[str, Any]:
    mapping = load_mapping(root / ".pm/github-project-sync/tasks.json")
    try:
        retired = durable_store.retired_task(mapping, task_uid)
    except ValueError as exc:
        raise ValueError(f"Task retirement ledger is invalid: {exc}") from exc
    if retired is not None:
        raise ValueError("Task UID is retired")
    record = mapping.get("tasks", {}).get(task_uid)
    if not isinstance(record, dict) or record.get("task_uid") != task_uid or record.get("repository") != repo:
        raise ValueError("canonical task mapping does not contain the selected UID/repository")
    live = github_issue_record(repo, task_uid)
    if (not isinstance(live, dict) or live.get("task_uid") != task_uid
            or live.get("issue_number") != record.get("issue_number")
            or live.get("issue_url") != record.get("issue_url")):
        raise ValueError("fresh live Task Issue does not match canonical mapping")
    _project_task_identity(root, task_uid, record, repo)
    return record


def _verify_merged_task(root: pathlib.Path, task_uid: str, repo: str,
                        record: dict[str, Any]) -> tuple[pathlib.Path, dict[str, Any], dict[str, Any], str]:
    if record.get("status") != "done" or record.get("workflow_phase") not in {"main_sync", "post_merge_done"}:
        raise ValueError("helper Task has not completed merged main-sync")
    default_ref = run_text(["git", "-C", str(root), "symbolic-ref", "refs/remotes/origin/HEAD"])
    if default_ref != "refs/remotes/origin/" + str(record.get("default_branch") or ""):
        raise ValueError("helper Task default branch differs from current repository authority")
    pr_number = record.get("pr_number")
    pr_url = record.get("pr_url")
    if type(pr_number) is not int or pr_number < 1 or pr_url != f"https://github.com/{repo}/pull/{pr_number}":
        raise ValueError("helper Task has no exact canonical PR binding")
    pr = github_pull_request(repo, pr_number)
    head = pr.get("head") or {}
    base = pr.get("base") or {}
    source_oid = str(head.get("sha") or "")
    if (pr.get("number") != pr_number or pr.get("html_url") != pr_url or pr.get("state") != "closed"
            or pr.get("merged") is not True or not source_oid
            or (head.get("repo") or {}).get("full_name") != repo
            or head.get("ref") != record.get("task_branch")
            or base.get("ref") != record.get("default_branch")
            or (base.get("repo") or {}).get("full_name") != repo):
        raise ValueError("live helper PR is not the exact merged source PR")
    comments = github_issue_comments(repo, int(record["issue_number"]))
    publication_module = load_pr_projection_publication_module()
    bindings = []
    for comment in comments:
        if "<!-- oasis7-ci-publication-binding/v1 -->" in comment["body"]:
            binding = publication_module.parse_publication_binding_comment(comment["body"])
            if (binding.get("task_uid") == task_uid and binding.get("pr_number") == pr_number
                    and binding.get("pr_url") == pr_url):
                bindings.append((comment, binding))
    if len(bindings) != 1:
        raise ValueError("helper Task lacks one unique reciprocal publication binding")
    comment, binding = bindings[0]
    if (type(comment.get("id")) is not int
            or comment.get("issue_url") != f"https://api.github.com/repos/{repo}/issues/{record['issue_number']}"
            or comment.get("html_url") != record["issue_url"] + "#issuecomment-" + str(comment["id"])
            or binding.get("task_uid") != task_uid):
        raise ValueError("helper reciprocal binding server identity is malformed")
    receipt_root = canonical_receipt_root(root, task_uid)
    merge_path = receipt_root / "merge-receipt.json"
    sync_path = receipt_root / "main-sync-receipt.json"
    merge, _merge_raw = strict_json(merge_path)
    main_sync, _sync_raw = strict_json(sync_path)
    if (merge.get("receipt_type") != "oasis7_pr_merge" or merge.get("issuer") != "github_live_query"
            or merge.get("evidence_mode") != "production" or merge.get("state") != "MERGED"
            or merge.get("repository") != repo or merge.get("pr_number") != pr_number
            or merge.get("pr_url") != pr_url or merge.get("head_oid") != source_oid
            or merge.get("base_ref") != record.get("default_branch")
            or merge.get("default_branch") != record.get("default_branch")
            or not merge.get("merged_at") or not merge.get("observed_at")):
        raise ValueError("canonical merge receipt does not match the live helper PR")
    main_commit = str(main_sync.get("main_commit") or "")
    if (main_sync.get("receipt_type") != "oasis7_main_sync" or main_sync.get("issuer") != "post-merge-main-sync"
            or main_sync.get("task_uid") != task_uid or main_sync.get("repository") != repo
            or main_sync.get("default_branch") != record.get("default_branch")
            or main_sync.get("merge_receipt_sha256") != hashlib.sha256(merge_path.read_bytes()).hexdigest()
            or not main_sync.get("observed_at")
            or not re.fullmatch(r"[0-9a-f]{40,64}", main_commit)):
        raise ValueError("canonical main-sync receipt does not match the helper merge receipt")
    current_main = run_text(["git", "-C", str(root), "rev-parse", f"refs/heads/{record['default_branch']}"])
    if (main_sync.get("remote_main_commit") != main_commit
            or subprocess.run(["git", "-C", str(root), "merge-base", "--is-ancestor", main_commit, current_main],
                              capture_output=True).returncode != 0):
        raise ValueError("default branch does not contain the exact main-sync receipt commit")
    patch_path = receipt_root / "patch-equivalence-receipt.json"
    if main_sync.get("integration_mode") == "ancestry":
        if any(key in main_sync for key in (
                "patch_equivalence_receipt_sha256", "patch_id", "projected_tree_oid", "main_tree_oid",
                "integration_commit", "integration_parent")):
            raise ValueError("ancestry main-sync receipt contains patch-equivalence fields")
        run_text(["git", "-C", str(root), "merge-base", "--is-ancestor", source_oid, main_commit])
        if patch_path.exists() or patch_path.is_symlink():
            raise ValueError("unexpected patch-equivalence receipt for ancestry integration")
    elif main_sync.get("integration_mode") == "patch_equivalence":
        patch, _patch_raw = strict_json(patch_path)
        integration_commit = str(main_sync.get("integration_commit") or "")
        integration_parent = str(main_sync.get("integration_parent") or "")
        projected_tree = str(main_sync.get("projected_tree_oid") or "")
        main_tree = str(main_sync.get("main_tree_oid") or "")
        if (patch.get("receipt_type") != "oasis7_patch_equivalence"
                or patch.get("schema_version") != 2
                or patch.get("issuer") != "oasis7_patch_equivalence_helper"
                or patch.get("branch_tip") != source_oid or patch.get("main_commit") != integration_commit
                or patch.get("main_parent") != integration_parent
                or patch.get("patch_id") != main_sync.get("patch_id")
                or patch.get("projected_tree_oid") != projected_tree
                or patch.get("main_tree_oid") != main_tree
                or not re.fullmatch(r"[0-9a-f]{40,64}", integration_commit)
                or not re.fullmatch(r"[0-9a-f]{40,64}", integration_parent)
                or not re.fullmatch(r"[0-9a-f]{40,64}", projected_tree)
                or not re.fullmatch(r"[0-9a-f]{40,64}", main_tree)
                or projected_tree != main_tree
                or run_text(["git", "-C", str(root), "rev-parse", f"{integration_commit}^{{tree}}"])
                    != main_tree
                or subprocess.run(["git", "-C", str(root), "merge-base", "--is-ancestor",
                                   integration_commit, current_main], capture_output=True).returncode != 0
                or main_sync.get("patch_equivalence_receipt_sha256") != hashlib.sha256(patch_path.read_bytes()).hexdigest()):
            raise ValueError("patch-equivalence receipt does not bind the merged helper source")
    else:
        raise ValueError("main-sync receipt integration mode is unsupported")
    return receipt_root, merge, main_sync, source_oid


def _review_inputs(task_root: pathlib.Path, task_uid: str, source_oid: str,
                   effective_root: pathlib.Path, live_task_record: dict[str, Any]) -> tuple[
                       dict[str, Any], list[dict[str, Any]], list[dict[str, Any]], pathlib.Path, pathlib.Path,
                   ]:
    scratch = task_root / ".pm" / "scratch" / task_uid
    if scratch.is_symlink() or not scratch.is_dir():
        raise ValueError("helper task review scratch is unavailable")
    plan_dir = scratch / "review-plans"
    plans = []
    for path in sorted(plan_dir.glob("*.json")):
        value, _raw = strict_json(path)
        if (value.get("schema") in {"oasis7-review-plan/v1", "oasis7-review-plan/v2"}
                and value.get("task_uid") == task_uid and value.get("frozen_head") == source_oid):
            plans.append((path, value))
    if len(plans) != 1:
        raise ValueError("helper source requires one exact frozen review plan")
    plan_path, plan = plans[0]
    roles = plan.get("roles")
    slices = plan.get("expected_slices")
    preflight = plan.get("preflight")
    if (not isinstance(roles, list) or not roles or len(roles) != len(set(roles))
            or not isinstance(slices, list) or not isinstance(preflight, dict)
            or not isinstance(preflight.get("ledger_path"), str)):
        raise ValueError("review plan role/ledger contract is incomplete")
    epoch = str(plan.get("epoch") or "")
    canonical_batch = scratch / "review-batches" / f"{epoch}.json"
    batch_path = pathlib.Path(str(plan.get("batch_path"))).expanduser()
    if not batch_path.is_absolute():
        batch_path = task_root / batch_path
    if batch_path.is_symlink() or batch_path.resolve() != canonical_batch.resolve():
        raise ValueError("review plan batch path is not canonical")
    batch, _batch_raw = strict_json(canonical_batch)
    def normalized_slice_identities(value: Any) -> list[tuple[str, str]] | None:
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

    plan_slice_identities = normalized_slice_identities(slices)
    batch_slices = batch.get("expected_slices")
    batch_slice_identities = normalized_slice_identities(batch_slices)
    if (batch.get("schema") != "oasis7-review-batch/v1" or batch.get("task_uid") != task_uid
            or batch.get("frozen_head") != source_oid or batch.get("epoch") != epoch
            or plan_slice_identities is None or batch_slice_identities is None
            or plan_slice_identities != batch_slice_identities):
        raise ValueError("review plan batch identity mismatch")
    snapshot_path = scratch / "bootstrap-task-snapshot.json"
    if snapshot_path.is_symlink() or not snapshot_path.is_file():
        raise ValueError("helper bootstrap snapshot is unavailable for archived packet origin")
    packet_helper_path = effective_root / "scripts/pm/subagent-task-packet.py"
    packet_spec = importlib.util.spec_from_file_location("subagent_task_packet_archive_origin", packet_helper_path)
    if packet_spec is None or packet_spec.loader is None:
        raise ValueError("cannot load the effective packet validator for archive origin")
    packet_helper = importlib.util.module_from_spec(packet_spec)
    packet_spec.loader.exec_module(packet_helper)
    snapshot = packet_helper.validate_bootstrap_snapshot(task_root, snapshot_path, task_uid)
    ledger_path = pathlib.Path(preflight["ledger_path"]).expanduser()
    if not ledger_path.is_absolute():
        ledger_path = task_root / ledger_path
    if ledger_path.is_symlink():
        raise ValueError("role-return ledger must not be a symlink")
    ledger_path = ledger_path.resolve(strict=True)
    if not ledger_path.is_relative_to(scratch.resolve()):
        raise ValueError("role-return ledger escapes helper task scratch")
    ledger_rows = [json.loads(line, object_pairs_hook=unique_json_object)
                   for line in ledger_path.read_text(encoding="utf-8").splitlines() if line.strip()]
    if not ledger_rows or any(not isinstance(row, dict) for row in ledger_rows):
        raise ValueError("role-return ledger is empty or malformed")
    by_role = {str(row.get("role") or ""): row for row in ledger_rows}
    if len(by_role) != len(ledger_rows) or set(by_role) != set(roles):
        raise ValueError("role-return ledger does not exactly cover the frozen role set")
    slices_by_role = {str(row.get("role") or ""): row for row in slices if isinstance(row, dict)}
    if len(slices_by_role) != len(slices) or set(slices_by_role) != set(roles):
        raise ValueError("review plan slice set differs from its required roles")
    closure_manifest = pm_closure_manifest(effective_root, source_oid, verify_worktree=False)
    closure_digest = hashlib.sha256(PublicationRecoveryAuthority.encoded(closure_manifest)).hexdigest()
    handoff_script = effective_root / "scripts/pm/review_preflight_handoff.py"
    handoff_spec = importlib.util.spec_from_file_location("review_preflight_handoff_archive_origin", handoff_script)
    if handoff_spec is None or handoff_spec.loader is None:
        raise ValueError("cannot load the effective review handoff validator")
    handoff_helper = importlib.util.module_from_spec(handoff_spec)
    handoff_spec.loader.exec_module(handoff_helper)
    returns = []
    for role in roles:
        ledger_row = by_role[role]
        slice_id = str(slices_by_role[role].get("slice_id") or "")
        packet_path = scratch / "slice-packets" / f"{slice_id}.json"
        artifacts = ledger_row.get("artifacts")
        if ledger_row.get("status") != "completed" or ledger_row.get("slice_id") != slice_id:
            raise ValueError("review role is incomplete or bound to another slice")
        if not packet_path.is_file() or packet_path.is_symlink():
            raise ValueError("review role packet is missing or not a regular file")
        packet_raw = packet_path.read_bytes()
        if not isinstance(artifacts, list) or len(artifacts) != 1 or not isinstance(artifacts[0], str):
            raise ValueError("review role ledger does not name exactly one return artifact")
        artifact_path = pathlib.Path(artifacts[0]).expanduser()
        if not artifact_path.is_absolute():
            artifact_path = task_root / artifact_path
        if artifact_path.is_symlink():
            raise ValueError("review return artifact must not be a symlink")
        artifact_path = artifact_path.resolve(strict=True)
        if not artifact_path.is_relative_to(task_root.resolve()):
            raise ValueError("review return artifact escapes helper task worktree")
        artifact_raw = artifact_path.read_bytes()
        value = json.loads(artifact_raw, object_pairs_hook=unique_json_object)
        handoff_helper.validate_return(
            value, role=role, slice_id=slice_id, task_uid=task_uid, head=source_oid, epoch=epoch,
        )
        if (value.get("scope_verdict") != "approved" or value.get("risk_verdict") != "approved"
                or value.get("disposition") != "no_findings" or value.get("findings") != []
                or value.get("helper_source_oid") != source_oid
                or value.get("helper_closure_sha256") != closure_digest):
            raise ValueError("independent role return does not approve the exact helper closure")
        returns.append({
            "role": role, "slice_id": slice_id, "packet_sha256": hashlib.sha256(packet_raw).hexdigest(),
            "source_head_oid": source_oid, "packet_path": str(packet_path.relative_to(task_root)),
            "return_path": str(artifact_path.relative_to(task_root)),
            "return_sha256": hashlib.sha256(artifact_raw).hexdigest(), "verdict": "approved",
        })
    env = {k: v for k, v in os.environ.items() if k != "OASIS7_TEST_ALLOW_UNATTESTED_DISPATCH_RECEIPTS"}
    subprocess.run([sys.executable, str(effective_root / "scripts/pm/validate-review-provenance.py"),
                    "--mode", "human-operated", "--root", str(task_root), "--task-uid", task_uid,
                    "--ledger", str(ledger_path), "--roles", ",".join(roles), "--source-head", source_oid],
                   cwd=task_root, capture_output=True, text=True, check=True, timeout=60, env=env)

    # A v2 handoff contains an independently reread authenticated dispatch
    # comment.  Revalidate that handoff now, while the original checkout and
    # all reviewed bytes still exist, then preserve the exact inputs needed by
    # the post-cleanup packet-contract validator.
    handoff_path = scratch / "review-handoffs" / f"{epoch}.json"
    validated_handoff = handoff_helper.validate_handoff(
        task_root, handoff_path, expected_plan_path=plan_path,
    )
    handoff = validated_handoff.get("handoff")
    dispatch_evidence = handoff.get("dispatch_evidence") if isinstance(handoff, dict) else None
    if (not isinstance(handoff, dict) or handoff.get("schema") != handoff_helper.HANDOFF_SCHEMA_V2
            or not isinstance(dispatch_evidence, dict)
            or set(dispatch_evidence) != handoff_helper.DISPATCH_EVIDENCE_FIELDS):
        raise ValueError("archived helper review requires one authenticated v2 dispatch handoff")
    dispatch_payload = handoff_helper.dispatch_payload_for_plan(
        task_root, plan_path, allow_promoted_ledger=True,
    )
    plan_raw = plan_path.read_bytes()
    batch_raw = canonical_batch.read_bytes()
    snapshot_raw = snapshot_path.read_bytes()
    origin_packets = []
    for expected in slices:
        role = str(expected["role"])
        slice_id = str(expected["slice_id"])
        packet_path = scratch / "slice-packets" / f"{slice_id}.json"
        if packet_path.is_symlink() or not packet_path.is_file():
            raise ValueError("authenticated review packet is unavailable for archive origin")
        packet_raw = packet_path.read_bytes()
        packet_value = json.loads(packet_raw, object_pairs_hook=unique_json_object)
        origin_packets.append({
            "role": role, "slice_id": slice_id,
            "repo_path": str(packet_path.relative_to(task_root)),
            "raw_sha256": hashlib.sha256(packet_raw).hexdigest(), "value": packet_value,
        })
    source_identity = plan.get("source_review_identity")
    source_scope_oid = (source_identity.get("source_scope_oid")
                        if isinstance(source_identity, dict) else plan.get("source_scope_oid"))
    if source_scope_oid is None:
        source_scope_oid = plan.get("comparison_oid")
    origin_context = {
        "schema": "oasis7-publication-helper-review-origin/v1", "task_uid": task_uid,
        "bootstrap_snapshot": {"value": snapshot, "raw_sha256": hashlib.sha256(snapshot_raw).hexdigest()},
        "review_plan": {"value": plan, "raw_sha256": hashlib.sha256(plan_raw).hexdigest()},
        "review_batch": {"value": batch, "raw_sha256": hashlib.sha256(batch_raw).hexdigest()},
        "dispatch_readback": {**dispatch_evidence, "payload": dispatch_payload},
        "packets": origin_packets,
        "reviewed_git": {
            "comparison_oid": plan.get("comparison_oid"),
            "source_scope_oid": source_scope_oid,
            "head_oid": plan.get("frozen_head"),
        },
    }
    live_task_identity = {
        "task_uid": task_uid, "issue_number": live_task_record.get("issue_number"),
        "issue_url": live_task_record.get("issue_url"),
        "project_item_id": live_task_record.get("project_item_id"),
        "repository": live_task_record.get("repository"),
        "canonical_worktree": live_task_record.get("canonical_worktree"),
        "task_branch": live_task_record.get("task_branch"),
        "owner_role": live_task_record.get("owner_role"),
        "pr_number": live_task_record.get("pr_number"), "pr_url": live_task_record.get("pr_url"),
    }
    for optional in ("primary_package", "loop_binding"):
        if optional in live_task_record:
            live_task_identity[optional] = live_task_record[optional]
    for packet_origin in origin_packets:
        packet_helper.validate_archived_packet(
            packet_origin["value"], origin_context, archive_root=task_root,
            live_task_identity=live_task_identity, reviewed_source_oid=source_oid,
        )
    origin_path = scratch / "publication-helper-review-origin.json"
    origin_raw = (json.dumps(origin_context, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode("utf-8")
    if origin_path.exists() or origin_path.is_symlink():
        if origin_path.is_symlink() or not origin_path.is_file() or origin_path.read_bytes() != origin_raw:
            raise ValueError("publication helper review origin is create-once and contains different evidence")
    else:
        with origin_path.open("xb") as handle:
            handle.write(origin_raw)
            handle.flush()
            os.fsync(handle.fileno())
    files = []
    total_bytes = 0
    for path in sorted(scratch.rglob("*")):
        if path.is_symlink():
            raise ValueError("helper review archive refuses symlinks")
        if path.is_dir():
            continue
        if not path.is_file():
            raise ValueError("helper review archive contains a non-regular member")
        raw = path.read_bytes()
        total_bytes += len(raw)
        if total_bytes > 50 * 1024 * 1024 or len(files) >= 512:
            raise ValueError("helper review archive exceeds bounded size")
        files.append({"path": str(path.relative_to(task_root)), "sha256": hashlib.sha256(raw).hexdigest(),
                      "size": len(raw)})
    return plan, returns, files, plan_path, ledger_path


def _archive_dir(root: pathlib.Path, task_uid: str) -> pathlib.Path:
    return canonical_receipt_root(root, task_uid) / HELPER_REVIEW_ARCHIVE


def _unique_helper_archive(root: pathlib.Path, closure_digest: str,
                           closure_manifest: list[dict[str, str]]) -> tuple[str, pathlib.Path, dict[str, Any], bytes]:
    common = pathlib.Path(run_text([
        "git", "-C", str(root), "rev-parse", "--path-format=absolute", "--git-common-dir",
    ])).resolve()
    receipts = common / "oasis7-workflow-receipts"
    matches = []
    if receipts.is_dir() and not receipts.is_symlink():
        for candidate in sorted(receipts.glob(f"task_*/{HELPER_REVIEW_ARCHIVE}/manifest.json")):
            uid = candidate.parent.parent.name
            if not re.fullmatch(r"task_[0-9a-f]{32}", uid):
                continue
            expected_root = canonical_receipt_root(root, uid)
            expected_dir = expected_root / HELPER_REVIEW_ARCHIVE
            if (candidate.parent != expected_dir or expected_dir.is_symlink()
                    or candidate.is_symlink() or not candidate.is_file()):
                raise ValueError("helper review archive path is not canonical")
            try:
                manifest, raw = strict_json(candidate)
            except (OSError, ValueError, json.JSONDecodeError):
                continue
            if (manifest.get("task_uid") == uid
                    and manifest.get("helper_closure_sha256") == closure_digest
                    and manifest.get("closure_manifest") == closure_manifest):
                matches.append((uid, candidate.parent, manifest, raw))
    if len(matches) != 1:
        raise ValueError("one unique merged helper review archive must match the effective closure")
    return matches[0]


def _validate_archive_files(archive_dir: pathlib.Path, manifest: dict[str, Any]) -> pathlib.Path:
    manifest_path = archive_dir / "manifest.json"
    data_root = archive_dir / "artifacts"
    if (archive_dir.is_symlink() or manifest_path.is_symlink() or data_root.is_symlink()
            or not archive_dir.is_dir() or not data_root.is_dir()):
        raise ValueError("helper archive root is missing or uses symlink substitution")
    rows = manifest.get("files")
    if not isinstance(rows, list) or not rows:
        raise ValueError("helper archive file manifest is empty")
    expected = set()
    for row in rows:
        if not isinstance(row, dict) or set(row) != {"path", "sha256", "size"}:
            raise ValueError("helper archive file entry is malformed")
        relative = pathlib.PurePosixPath(str(row["path"]))
        if (not str(row["path"]).isascii() or relative.is_absolute()
                or any(part in {"", ".", ".."} for part in relative.parts)
                or type(row["size"]) is not int or row["size"] < 0
                or not isinstance(row["sha256"], str)
                or re.fullmatch(r"[0-9a-f]{64}", row["sha256"]) is None):
            raise ValueError("helper archive path escapes its fixed root")
        if relative.as_posix() in expected:
            raise ValueError("helper archive manifest repeats a path")
        path = data_root.joinpath(*relative.parts)
        if path.is_symlink() or not path.is_file() or not path.resolve().is_relative_to(data_root.resolve()):
            raise ValueError("helper archive member is missing or escapes its root")
        raw = path.read_bytes()
        if len(raw) != row["size"] or hashlib.sha256(raw).hexdigest() != row["sha256"]:
            raise ValueError("helper archive member digest mismatch")
        expected.add(relative.as_posix())
    actual = {str(path.relative_to(data_root)) for path in data_root.rglob("*") if path.is_file()}
    if any(path.is_symlink() for path in data_root.rglob("*")) or actual != expected:
        raise ValueError("helper archive has missing or unmanifested members")
    return data_root


def _require_finalized_issue_phase_projection(issue_phase: object, mapping_phase: object) -> None:
    """Allow the finalizer's documented Issue-body/mapping phase pair after receipt validation."""
    if issue_phase == mapping_phase or (issue_phase == "task_done" and mapping_phase == "post_merge_done"):
        return
    raise ValueError("fresh live helper terminal phase lacks the receipt-backed mapping projection")


def _normalize_archived_helper_return_path(artifact: object, original_worktree: object,
                                          expected_return_path: object) -> str:
    """Map one old absolute ledger path through authenticated origin to its archive member."""
    if (not isinstance(artifact, str) or not artifact or artifact != artifact.strip()
            or not isinstance(original_worktree, str) or not original_worktree
            or not isinstance(expected_return_path, str) or not expected_return_path):
        raise ValueError("archived role return path identity is malformed")
    origin = pathlib.Path(original_worktree)
    expected = pathlib.PurePosixPath(expected_return_path)
    if (not origin.is_absolute() or ".." in origin.parts or str(origin) != original_worktree
            or expected.is_absolute() or ".." in expected.parts or expected.as_posix() != expected_return_path):
        raise ValueError("archived role return origin or member path is not canonical")
    relative = expected.as_posix()
    absolute = str(origin / pathlib.Path(*expected.parts))
    if artifact not in {relative, absolute}:
        raise ValueError("role-return ledger artifact differs from its authenticated archive member")
    return relative


def _safe_archived_helper_member(data_root: pathlib.Path, member: str) -> pathlib.Path:
    relative = pathlib.PurePosixPath(member)
    if (not member or relative.is_absolute() or ".." in relative.parts
            or relative.as_posix() != member or not relative.parts):
        raise ValueError("archived helper member path is not canonical")
    root = data_root.resolve(strict=True)
    candidate = root
    for part in relative.parts:
        candidate = candidate / part
        if candidate.is_symlink():
            raise ValueError("archived helper member path contains a symlink")
    resolved = candidate.resolve(strict=True)
    if not resolved.is_relative_to(root) or not resolved.is_file():
        raise ValueError("archived helper member escapes its data root or is not a file")
    return resolved


def validate_helper_review_archive(authority: SinglePublicationRecoveryAuthority,
                                   review: dict[str, Any]) -> None:
    root, current_head, current_manifest, current_digest = effective_helper_root()
    if (review.get("helper_closure_sha256") != current_digest
            or review.get("closure_manifest") != current_manifest):
        raise ValueError("recovery is not executing from the independently reviewed effective helper closure")
    helper_uid = str(review.get("helper_task_uid") or "")
    if not re.fullmatch(r"task_[0-9a-f]{32}", helper_uid):
        raise ValueError("helper Task locator is malformed")
    selected_uid, selected_dir, selected_manifest, selected_raw = _unique_helper_archive(
        root, current_digest, current_manifest,
    )
    if selected_uid != helper_uid:
        raise ValueError("v2 helper Task locator differs from the independently resolved archive identity")
    archive_dir = selected_dir
    manifest, raw = strict_json(archive_dir / "manifest.json")
    if raw != selected_raw or manifest != selected_manifest or hashlib.sha256(raw).hexdigest() != review.get("archive_manifest_sha256"):
        raise ValueError("helper archive manifest digest mismatch")
    manifest_keys = {
        "schema", "task_uid", "repository", "issue_number", "issue_url", "project_item_id",
        "pr_number", "pr_url", "source_head_oid", "merge_receipt_sha256",
        "main_sync_receipt_sha256", "main_commit", "patch_equivalence_receipt_sha256",
        "helper_closure_sha256", "closure_manifest", "plan_path", "plan_sha256",
        "ledger_path", "ledger_sha256", "roles", "role_returns", "files",
    }
    if set(manifest) != manifest_keys:
        raise ValueError("helper archive manifest contains unsupported fields")
    if (manifest.get("schema") != HELPER_REVIEW_ARCHIVE_SCHEMA or manifest.get("task_uid") != helper_uid
            or manifest.get("repository") != authority.args.repo
            or manifest.get("helper_closure_sha256") != current_digest
            or manifest.get("closure_manifest") != current_manifest):
        raise ValueError("helper archive source/closure identity mismatch")
    helper_record = _task_record(root, helper_uid, authority.args.repo)
    live_helper = github_issue_record(authority.args.repo, helper_uid)
    if not isinstance(live_helper, dict):
        raise ValueError("fresh live helper Task Issue is unavailable")
    live_helper_pairs = {
        "task_uid": helper_uid, "issue_number": helper_record.get("issue_number"),
        "issue_url": helper_record.get("issue_url"), "owner_role": helper_record.get("owner_role"),
        "worktree_hint": helper_record.get("canonical_worktree"),
        "status": helper_record.get("status"),
        "pr_number": helper_record.get("pr_number"), "pr_url": helper_record.get("pr_url"),
    }
    if any(live_helper.get(key) != expected for key, expected in live_helper_pairs.items()):
        raise ValueError("fresh live helper Task fields differ from canonical task mapping")
    if str(live_helper.get("issue_state") or "").upper() != "CLOSED":
        raise ValueError("terminal helper Task Issue is not closed")
    if helper_record.get("status") != "done" or helper_record.get("workflow_phase") != "post_merge_done":
        raise ValueError("helper Task has not completed terminal cleanup/finalization")
    receipt_root, _merge, _sync, source_oid = _verify_merged_task(root, helper_uid, authority.args.repo, helper_record)
    terminal_path = receipt_root / "terminal-cleanup-receipt.json"
    terminal, terminal_raw = strict_json(terminal_path)
    terminal_digest = hashlib.sha256(terminal_raw).hexdigest()
    merge_path = receipt_root / "merge-receipt.json"
    main_sync_path = receipt_root / "main-sync-receipt.json"
    merge_digest = hashlib.sha256(merge_path.read_bytes()).hexdigest()
    main_sync_digest = hashlib.sha256(main_sync_path.read_bytes()).hexdigest()
    if (terminal.get("receipt_type") != "oasis7_terminal_cleanup"
            or terminal.get("issuer") != "post-merge-cleanup"
            or terminal.get("task_uid") != helper_uid
            or terminal.get("repository") != authority.args.repo
            or terminal.get("issue_number") != helper_record.get("issue_number")
            or terminal.get("pr_number") != helper_record.get("pr_number")
            or terminal.get("worktree") != helper_record.get("canonical_worktree")
            or terminal.get("branch") != helper_record.get("task_branch")
            or terminal.get("merge_receipt_sha256") != merge_digest
            or terminal.get("main_sync_receipt_sha256") != main_sync_digest
            or (helper_record.get("phase_receipts") or {}).get("post_merge_done") != terminal
            or (helper_record.get("phase_receipt_sha256") or {}).get("post_merge_done") != terminal_digest):
        raise ValueError("helper terminal cleanup receipt does not match the live Task and receipt chain")
    finalizer_path = root / "scripts/pm/post-merge-finalize.py"
    finalizer_spec = importlib.util.spec_from_file_location("post_merge_finalize_readonly", finalizer_path)
    if finalizer_spec is None or finalizer_spec.loader is None:
        raise ValueError("cannot load the effective terminal cleanup validator")
    finalizer = importlib.util.module_from_spec(finalizer_spec)
    finalizer_spec.loader.exec_module(finalizer)
    try:
        finalizer._validate_cleanup_intent(terminal_path, helper_uid, helper_record, terminal, True)
    except SystemExit as exc:
        raise ValueError(f"helper terminal cleanup intent validation failed: {exc}") from exc
    _require_finalized_issue_phase_projection(
        live_helper.get("workflow_phase"), helper_record.get("workflow_phase"),
    )
    if (source_oid != manifest.get("source_head_oid") or source_oid != review.get("helper_source_oid")
            or manifest.get("main_commit") != _sync.get("main_commit")
            or manifest.get("issue_number") != helper_record.get("issue_number")
            or manifest.get("issue_url") != helper_record.get("issue_url")
            or manifest.get("project_item_id") != helper_record.get("project_item_id")
            or manifest.get("pr_number") != helper_record.get("pr_number")
            or manifest.get("pr_url") != helper_record.get("pr_url")
            or manifest.get("merge_receipt_sha256") != hashlib.sha256((receipt_root / "merge-receipt.json").read_bytes()).hexdigest()
            or manifest.get("main_sync_receipt_sha256") != hashlib.sha256((receipt_root / "main-sync-receipt.json").read_bytes()).hexdigest()):
        raise ValueError("helper archive Task/PR/receipt identity mismatch")
    data_root = _validate_archive_files(archive_dir, manifest)
    plan_path = (data_root / str(manifest.get("plan_path") or "")).resolve(strict=True)
    ledger_path = (data_root / str(manifest.get("ledger_path") or "")).resolve(strict=True)
    if not plan_path.is_relative_to(data_root.resolve()) or not ledger_path.is_relative_to(data_root.resolve()):
        raise ValueError("helper archive plan or ledger escapes its data root")
    plan, plan_raw = strict_json(plan_path)
    if (hashlib.sha256(plan_raw).hexdigest() != manifest.get("plan_sha256")
            or plan.get("task_uid") != helper_uid or plan.get("frozen_head") != source_oid):
        raise ValueError("archived frozen review plan identity/digest mismatch")
    if hashlib.sha256(ledger_path.read_bytes()).hexdigest() != manifest.get("ledger_sha256"):
        raise ValueError("archived review ledger digest mismatch")
    roles = manifest.get("roles")
    role_returns = manifest.get("role_returns")
    if (not isinstance(roles, list) or sorted(roles) != sorted(SinglePublicationRecoveryAuthority.roles)
            or len(roles) != len(set(roles)) or plan.get("roles") != roles
            or not isinstance(role_returns, list) or not all(isinstance(row, dict) for row in role_returns)
            or len(role_returns) != len(roles)
            or sorted(row.get("role", "") for row in role_returns) != sorted(roles)):
        raise ValueError("archived independent review role set is incomplete")
    files_by_path = {str(row["path"]): row for row in manifest["files"]}
    origin_relative = f".pm/scratch/{helper_uid}/publication-helper-review-origin.json"
    if origin_relative not in files_by_path:
        raise ValueError("archive omits its manifest-bound packet origin context")
    origin_context, _origin_raw = strict_json(data_root / origin_relative)
    dispatch_origin = origin_context.get("dispatch_readback") if isinstance(origin_context, dict) else None
    dispatch_payload = dispatch_origin.get("payload") if isinstance(dispatch_origin, dict) else None
    dispatch_comment_id = dispatch_origin.get("comment_id") if isinstance(dispatch_origin, dict) else None
    handoff_helper_path = root / "scripts/pm/review_preflight_handoff.py"
    handoff_helper_spec = importlib.util.spec_from_file_location(
        "review_preflight_handoff_archived_recovery", handoff_helper_path,
    )
    if handoff_helper_spec is None or handoff_helper_spec.loader is None:
        raise ValueError("cannot load the effective live dispatch reader")
    handoff_helper = importlib.util.module_from_spec(handoff_helper_spec)
    handoff_helper_spec.loader.exec_module(handoff_helper)
    if not isinstance(dispatch_payload, dict):
        raise ValueError("archived packet origin omits its authenticated dispatch payload")
    fresh_dispatch = handoff_helper.live_dispatch_readback(root, dispatch_payload, dispatch_comment_id)
    for field in ("issue_number", "issue_url", "comment_id", "author", "body_digest", "payload"):
        if not isinstance(dispatch_origin, dict) or dispatch_origin.get(field) != fresh_dispatch.get(field):
            raise ValueError("fresh authenticated dispatch readback differs from archived packet origin")
    packet_helper_path = root / "scripts/pm/subagent-task-packet.py"
    packet_helper_spec = importlib.util.spec_from_file_location(
        "subagent_task_packet_archived_recovery", packet_helper_path,
    )
    if packet_helper_spec is None or packet_helper_spec.loader is None:
        raise ValueError("cannot load the effective shared packet validator")
    packet_helper = importlib.util.module_from_spec(packet_helper_spec)
    packet_helper_spec.loader.exec_module(packet_helper)
    archived_packet_validator = getattr(packet_helper, "validate_archived_packet", None)
    if not callable(archived_packet_validator):
        raise ValueError("effective packet validator lacks archived-origin validation")
    live_task_identity = {
        "task_uid": helper_uid, "issue_number": helper_record.get("issue_number"),
        "issue_url": helper_record.get("issue_url"),
        "project_item_id": helper_record.get("project_item_id"),
        "repository": helper_record.get("repository"),
        "canonical_worktree": helper_record.get("canonical_worktree"),
        "task_branch": helper_record.get("task_branch"),
        "owner_role": helper_record.get("owner_role"),
        "pr_number": helper_record.get("pr_number"), "pr_url": helper_record.get("pr_url"),
    }
    for optional in ("primary_package", "loop_binding"):
        if optional in helper_record:
            live_task_identity[optional] = helper_record[optional]
    expected_slices = plan.get("expected_slices")
    if not isinstance(expected_slices, list):
        raise ValueError("archived review plan slice set is malformed")
    slices_by_role = {str(row.get("role") or ""): row for row in expected_slices if isinstance(row, dict)}
    if len(slices_by_role) != len(expected_slices) or set(slices_by_role) != set(roles):
        raise ValueError("archived review plan does not cover its exact role set")
    ledger_rows = [json.loads(line, object_pairs_hook=unique_json_object)
                   for line in ledger_path.read_text(encoding="utf-8").splitlines() if line.strip()]
    ledger_by_role = {str(row.get("role") or ""): row for row in ledger_rows if isinstance(row, dict)}
    if len(ledger_by_role) != len(ledger_rows) or set(ledger_by_role) != set(roles):
        raise ValueError("archived provenance ledger does not cover its exact role set")
    seen_return_paths: set[str] = set()
    ledger_artifact_members: dict[str, str] = {}
    resolved_ledger_members: set[str] = set()
    for returned in role_returns:
        PublicationRecoveryAuthority._closed(returned, {
            "role", "slice_id", "packet_sha256", "source_head_oid", "packet_path",
            "return_path", "return_sha256", "verdict",
        })
        role = returned["role"]
        slice_id = str(slices_by_role[role].get("slice_id") or "")
        expected_packet_path = f".pm/scratch/{helper_uid}/slice-packets/{slice_id}.json"
        packet_path = str(returned["packet_path"])
        return_path = str(returned["return_path"])
        if (returned["slice_id"] != slice_id or packet_path != expected_packet_path
                or not return_path.startswith(f".pm/scratch/{helper_uid}/")
                or returned["source_head_oid"] != source_oid or returned["verdict"] != "approved"):
            raise ValueError("archived role return does not bind its exact packet/head")
        packet_row = files_by_path.get(packet_path)
        return_row = files_by_path.get(return_path)
        if (not packet_row or not return_row
                or packet_row["sha256"] != returned["packet_sha256"]
                or return_row["sha256"] != returned["return_sha256"]):
            raise ValueError("archived packet/return digest is not bound to the file manifest")
        packet = json.loads((data_root / packet_path).read_bytes(), object_pairs_hook=unique_json_object)
        if not isinstance(packet, dict):
            raise ValueError("archived review packet is not an object")
        try:
            archived_packet_validator(
                packet, origin_context, archive_root=data_root,
                live_task_identity=live_task_identity, reviewed_source_oid=source_oid,
            )
        except Exception as exc:
            raise ValueError(f"archived packet contract validation failed: {exc}") from exc
        snapshot_wrapper = origin_context.get("bootstrap_snapshot")
        snapshot = snapshot_wrapper.get("value") if isinstance(snapshot_wrapper, dict) else None
        snapshot_git = snapshot.get("git") if isinstance(snapshot, dict) else None
        original_worktree = snapshot_git.get("worktree") if isinstance(snapshot_git, dict) else None
        if original_worktree != helper_record.get("canonical_worktree"):
            raise ValueError("authenticated bootstrap worktree differs from the live helper Task")
        packet_identity = packet.get("identity") or {}
        packet_slice = packet.get("slice") or {}
        if not isinstance(packet_identity, dict) or not isinstance(packet_slice, dict):
            raise ValueError("archived review packet identity/slice is malformed")
        packet_unsigned = {key: value for key, value in packet.items() if key != "packet_digest"}
        packet_preimage = json.dumps(packet_unsigned, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        packet_digest = hashlib.sha256(packet_preimage.encode("utf-8")).hexdigest()
        if (packet.get("schema") != "oasis7-subagent-task-packet/v1"
                or packet.get("packet_digest") != packet_digest
                or packet_identity.get("task_uid") != helper_uid
                or packet_identity.get("head") != source_oid
                or packet_identity.get("project_item_id") != helper_record.get("project_item_id")
                or packet_identity.get("repository") != authority.args.repo
                or packet_identity.get("issue_url") != helper_record.get("issue_url")
                or packet_slice.get("owner_role") != helper_record.get("owner_role")
                or packet_slice.get("formal_sink") != helper_record.get("issue_url")
                or packet_slice.get("role") != role or packet_slice.get("slice_id") != slice_id):
            raise ValueError("archived review packet does not bind its canonical task/role/head")
        archive_return_path = _safe_archived_helper_member(data_root, return_path)
        returned_value = json.loads(archive_return_path.read_bytes(), object_pairs_hook=unique_json_object)
        if not isinstance(returned_value, dict):
            raise ValueError("archived independent return is not an object")
        try:
            handoff_helper.validate_return(
                returned_value, role=role, slice_id=slice_id, task_uid=helper_uid,
                head=source_oid, epoch=str(plan.get("epoch") or ""),
            )
        except Exception as exc:
            raise ValueError(f"archived independent return contract is invalid: {exc}") from exc
        if any(returned_value.get(key) != expected for key, expected in {
                "task_uid": helper_uid, "role": role, "slice_id": slice_id,
                "head": source_oid, "status": "completed", "scope_verdict": "approved",
                "risk_verdict": "approved", "disposition": "no_findings", "findings": [],
                "helper_source_oid": source_oid,
                "helper_closure_sha256": manifest["helper_closure_sha256"],
        }.items()):
            raise ValueError("archived independent return does not approve the exact helper closure")
        ledger_row = ledger_by_role[role]
        ledger_artifacts = ledger_row.get("artifacts")
        if (ledger_row.get("task_uid") != helper_uid or ledger_row.get("slice_id") != slice_id
                or ledger_row.get("status") != "completed"
                or ledger_row.get("artifact_digest") != returned["return_sha256"]
                or not isinstance(ledger_artifacts, list) or len(ledger_artifacts) != 1):
            raise ValueError("archived provenance ledger does not bind the exact role return")
        ledger_return_path = _normalize_archived_helper_return_path(
            ledger_artifacts[0], original_worktree, return_path,
        )
        if ledger_return_path in seen_return_paths:
            raise ValueError("archived provenance ledger duplicates a role return path")
        seen_return_paths.add(ledger_return_path)
        archived_return = _safe_archived_helper_member(data_root, ledger_return_path)
        if (ledger_return_path != return_path
                or hashlib.sha256(archived_return.read_bytes()).hexdigest() != returned["return_sha256"]):
            raise ValueError("archived provenance ledger path or bytes differ from its manifest-bound return")
        absolute_ledger_path = str(pathlib.Path(original_worktree) / pathlib.Path(*pathlib.PurePosixPath(return_path).parts))
        for ledger_path_key in (ledger_return_path, absolute_ledger_path):
            previous = ledger_artifact_members.get(ledger_path_key)
            if previous is not None and previous != ledger_return_path:
                raise ValueError("archived provenance ledger path maps to multiple archive members")
            ledger_artifact_members[ledger_path_key] = ledger_return_path

    provenance_script = root / "scripts/pm/validate-review-provenance.py"
    provenance_spec = importlib.util.spec_from_file_location(
        "validate_review_provenance_archived_recovery", provenance_script,
    )
    if provenance_spec is None or provenance_spec.loader is None:
        raise ValueError("cannot load the effective shared provenance validator")
    provenance_validator = importlib.util.module_from_spec(provenance_spec)
    provenance_spec.loader.exec_module(provenance_validator)

    def archived_artifact_resolver(raw: str, archive_root: pathlib.Path) -> pathlib.Path:
        if archive_root.resolve(strict=True) != data_root.resolve(strict=True):
            raise ValueError("provenance validator archive root differs from the authenticated archive")
        member = ledger_artifact_members.get(raw)
        if member is None:
            raise ValueError("provenance ledger artifact has no exact authenticated archive mapping")
        if member in resolved_ledger_members:
            raise ValueError("provenance ledger repeats an authenticated archive member")
        file_entry = files_by_path.get(member)
        if not isinstance(file_entry, dict):
            raise ValueError("provenance ledger archive member is not listed in the immutable manifest")
        archived_file = _safe_archived_helper_member(data_root, member)
        archived_bytes = archived_file.read_bytes()
        if (len(archived_bytes) != file_entry.get("size")
                or hashlib.sha256(archived_bytes).hexdigest() != file_entry.get("sha256")):
            raise ValueError("provenance ledger archive member differs from its immutable manifest")
        resolved_ledger_members.add(member)
        return archived_file

    provenance_args = argparse.Namespace(
        root=data_root, task_uid=helper_uid, ledger=str(ledger_path),
        roles=",".join(roles), source_head=source_oid, mode="human-operated",
    )
    validator_stdout = io.StringIO()
    validator_stderr = io.StringIO()
    try:
        with contextlib.redirect_stdout(validator_stdout), contextlib.redirect_stderr(validator_stderr):
            validation_result = provenance_validator.validate_ledger(
                provenance_args, argparse.ArgumentParser(prog="validate-review-provenance-archive"),
                archived_artifact_resolver=archived_artifact_resolver,
            )
    except SystemExit as exc:
        detail = validator_stderr.getvalue().strip()
        raise ValueError(f"archived provenance ledger validation failed: {detail or exc}") from exc
    except (OSError, RuntimeError, TypeError, ValueError) as exc:
        raise ValueError(f"archived provenance ledger validation failed: {exc}") from exc
    expected_validation = {
        "status": "passed", "mode": "human-operated", "roles": sorted(roles),
        "source_head": source_oid,
    }
    try:
        actual_validation = json.loads(validator_stdout.getvalue())
    except json.JSONDecodeError as exc:
        raise ValueError("shared provenance validator returned malformed validation evidence") from exc
    if validation_result != 0 or actual_validation != expected_validation or resolved_ledger_members != seen_return_paths:
        raise ValueError("shared provenance validator did not verify every exact archived role return")
    archive_projection = {
        "helper_task_uid": manifest.get("task_uid"),
        "helper_source_oid": manifest.get("source_head_oid"),
        "helper_closure_sha256": manifest.get("helper_closure_sha256"),
        "closure_manifest": manifest.get("closure_manifest"),
        "plan_path": manifest.get("plan_path"),
        "plan_sha256": manifest.get("plan_sha256"),
        "ledger_path": manifest.get("ledger_path"),
        "ledger_sha256": manifest.get("ledger_sha256"),
        "role_returns": manifest.get("role_returns"),
    }
    for key, expected in archive_projection.items():
        if review.get(key) != expected:
            raise ValueError("v2 admission review binding differs from the canonical archive")


def command_archive_publication_helper_review(args: argparse.Namespace) -> int:
    if not re.fullmatch(r"task_[0-9a-f]{32}", args.task_uid):
        raise ValueError("invalid canonical helper Task UID")
    root, _effective_head, effective_manifest, effective_digest = effective_helper_root()
    record = _task_record(root, args.task_uid, DEFAULT_REPO)
    task_root = pathlib.Path(str(record.get("canonical_worktree") or "")).expanduser().resolve(strict=True)
    identity = authoritative_repository_identity(root, DEFAULT_REPO, str(task_root))
    if (identity["canonical_worktree"] != str(task_root) or identity["task_branch"] != record.get("task_branch")
            or identity["default_branch"] != record.get("default_branch")):
        raise ValueError("helper Task canonical worktree identity is invalid")
    receipt_root, _merge, main_sync, source_oid = _verify_merged_task(root, args.task_uid, DEFAULT_REPO, record)
    if run_text(["git", "-C", str(task_root), "rev-parse", "HEAD"]) != source_oid:
        raise ValueError("helper Task worktree is not at the merged source head")
    source_manifest = pm_closure_manifest(root, source_oid, verify_worktree=False)
    main_commit = str(main_sync["main_commit"])
    main_manifest = pm_closure_manifest(root, main_commit, verify_worktree=False)
    if source_manifest != effective_manifest or main_manifest != source_manifest:
        raise ValueError("reviewed source, integrated main and effective helper closures differ")
    plan, returns, files, plan_path, ledger_path = _review_inputs(
        task_root, args.task_uid, source_oid, root, record,
    )
    closure_digest = hashlib.sha256(PublicationRecoveryAuthority.encoded(source_manifest)).hexdigest()
    manifest = {
        "schema": HELPER_REVIEW_ARCHIVE_SCHEMA, "task_uid": args.task_uid, "repository": DEFAULT_REPO,
        "issue_number": record["issue_number"], "issue_url": record["issue_url"],
        "project_item_id": record["project_item_id"], "pr_number": record["pr_number"], "pr_url": record["pr_url"],
        "source_head_oid": source_oid,
        "merge_receipt_sha256": hashlib.sha256((receipt_root / "merge-receipt.json").read_bytes()).hexdigest(),
        "main_sync_receipt_sha256": hashlib.sha256((receipt_root / "main-sync-receipt.json").read_bytes()).hexdigest(),
        "main_commit": main_commit,
        "patch_equivalence_receipt_sha256": hashlib.sha256((receipt_root / "patch-equivalence-receipt.json").read_bytes()).hexdigest()
            if (receipt_root / "patch-equivalence-receipt.json").is_file() else None,
        "helper_closure_sha256": closure_digest, "closure_manifest": source_manifest,
        "plan_path": str(plan_path.relative_to(task_root)), "plan_sha256": hashlib.sha256(plan_path.read_bytes()).hexdigest(),
        "ledger_path": str(ledger_path.relative_to(task_root)), "ledger_sha256": hashlib.sha256(ledger_path.read_bytes()).hexdigest(),
        "roles": plan["roles"], "role_returns": returns, "files": files,
    }
    archive_dir = _archive_dir(root, args.task_uid)
    archive_root = archive_dir.parent
    archive_root.mkdir(parents=True, exist_ok=True)
    manifest_bytes = (json.dumps(manifest, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode("utf-8")
    if archive_dir.exists():
        old, old_raw = strict_json(archive_dir / "manifest.json")
        _validate_archive_files(archive_dir, old)
        if old_raw != manifest_bytes:
            raise ValueError("helper review archive is create-once and contains different evidence")
    else:
        stage = pathlib.Path(tempfile.mkdtemp(prefix=".publication-helper-review.", dir=archive_root))
        try:
            data_root = stage / "artifacts"
            for entry in files:
                relative = pathlib.PurePosixPath(entry["path"])
                source = task_root.joinpath(*relative.parts)
                destination = data_root.joinpath(*relative.parts)
                destination.parent.mkdir(parents=True, exist_ok=True)
                raw = source.read_bytes()
                if hashlib.sha256(raw).hexdigest() != entry["sha256"]:
                    raise ValueError("review artifact changed while creating helper archive")
                with destination.open("xb") as handle:
                    handle.write(raw)
                    handle.flush()
                    os.fsync(handle.fileno())
            with (stage / "manifest.json").open("xb") as handle:
                handle.write(manifest_bytes)
                handle.flush()
                os.fsync(handle.fileno())
            _validate_archive_files(stage, manifest)
            os.rename(stage, archive_dir)
        finally:
            if stage.exists():
                shutil.rmtree(stage)
    readback, readback_raw = strict_json(archive_dir / "manifest.json")
    _validate_archive_files(archive_dir, readback)
    if readback_raw != manifest_bytes:
        raise ValueError("helper archive manifest readback mismatch")
    print(json.dumps({
        "status": "archived", "task_uid": args.task_uid, "source_head_oid": source_oid,
        "archive_path": str(archive_dir), "manifest_sha256": hashlib.sha256(readback_raw).hexdigest(),
        "helper_closure_sha256": effective_digest,
    }, sort_keys=True))
    return 0


def _comment_ref(comment: dict[str, Any]) -> dict[str, Any]:
    return {
        "comment_id": comment["id"], "comment_url": comment["html_url"],
        "body_sha256": hashlib.sha256(comment["body"].encode("utf-8")).hexdigest(),
        "author_login": (comment.get("user") or {}).get("login"),
    }


def _parse_recovery_request(comment: dict[str, Any]) -> dict[str, Any]:
    marker = "<!-- oasis7-publication-recovery-request/v1 -->"
    fence = chr(96) * 3
    match = re.fullmatch(re.escape(marker) + r"\s*" + re.escape(fence) + r"json\s*\n(.*?)\n"
                         + re.escape(fence) + r"\s*", str(comment.get("body") or ""), re.S)
    if not match:
        raise ValueError("malformed user recovery request")
    value = json.loads(match[1], object_pairs_hook=unique_json_object)
    keys = {"schema", "operation", "repository", "task_uid", "issue_number", "issue_url",
            "pr_number", "pr_url", "publication_id", "action_id"}
    if not isinstance(value, dict) or set(value) != keys:
        raise ValueError("user recovery request has unsupported fields")
    canonical = PublicationRecoveryAuthority.encoded(value).decode("utf-8")
    expected_body = marker + "\n" + fence + "json\n" + canonical + "\n" + fence
    if (comment.get("body") != expected_body
            or value.get("schema") != "oasis7-publication-recovery-request/v1"
            or value.get("operation") != "resume_uncertain_record_pr"):
        raise ValueError("user recovery request is not canonical")
    return value


def _parse_record_pr_recovery_action_id(value: object) -> str:
    match = re.fullmatch(r"record-pr:(sha256:[0-9a-f]{64})", value) if isinstance(value, str) else None
    if match is None:
        raise ValueError("recovery requires one exact persisted record-pr action selector")
    return match.group(1)


def command_admit_record_pr_recovery(args: argparse.Namespace) -> int:
    root, effective_head, effective_manifest, effective_digest = effective_helper_root()
    task_uid = args.task_uid
    if not re.fullmatch(r"task_[0-9a-f]{32}", task_uid):
        raise ValueError("invalid canonical Task UID")
    publication_id = _parse_record_pr_recovery_action_id(args.action_id)
    mapping = load_mapping(root / ".pm/github-project-sync/tasks.json")
    record = _task_record(root, task_uid, DEFAULT_REPO)
    target_root = pathlib.Path(str(record.get("canonical_worktree") or "")).expanduser().resolve(strict=True)
    identity = authoritative_repository_identity(root, DEFAULT_REPO, str(target_root))
    if any(record.get(key) != identity.get(key) for key in
           ("repository", "canonical_worktree", "task_branch", "default_branch")):
        raise ValueError("canonical target worktree/branch identity differs from Task mapping")
    if record.get("worktree_hint") != str(target_root):
        raise ValueError("target Task worktree hint differs from its canonical mapping")

    comments = github_issue_comments(DEFAULT_REPO, int(record["issue_number"]))
    admissions = [c for c in comments if any(marker in c["body"] for marker in (
        PublicationRecoveryAuthority.marker, SinglePublicationRecoveryAuthority.marker,
    ))]
    if admissions:
        raise ValueError("an existing recovery admission must be consumed through the public publisher")
    request_marker = "<!-- oasis7-publication-recovery-request/v1 -->"
    requests = [c for c in comments if request_marker in c["body"]]
    if len(requests) != 1:
        raise ValueError("exactly one user-authorized recovery request is required")
    request_comment = requests[0]
    request = _parse_recovery_request(request_comment)

    publication_module = load_pr_projection_publication_module()
    publication_comments = []
    binding_comments = []
    for comment in comments:
        body = str(comment.get("body") or "")
        if "<!-- oasis7-ci-publication/v1 -->" in body:
            publication_comments.append((comment, publication_module.parse_publication_comment(body)))
        if "<!-- oasis7-ci-publication-binding/v1 -->" in body:
            binding_comments.append((comment, publication_module.parse_publication_binding_comment(body)))
    current = [(c, p) for c, p in publication_comments if p.get("publication_id") == publication_id]
    if len(current) != 1:
        raise ValueError("exact unique current publication intent is missing or ambiguous")
    intent_comment, intent = current[0]
    conflicts = [
        publication for _comment, publication in publication_comments
        if publication.get("task_uid") == task_uid
        and publication.get("source_head_oid") == intent.get("source_head_oid")
        and publication.get("source_scope_oid") == intent.get("source_scope_oid")
        and publication.get("publication_id") != publication_id
    ]
    if conflicts:
        raise ValueError("competing same-task/head/scope publication blocks recovery")
    common = pathlib.Path(run_text([
        "git", "-C", str(target_root), "rev-parse", "--path-format=absolute", "--git-common-dir",
    ])).resolve()
    from pr_projection_journal import publication_paths
    journal_path, _lock_path = publication_paths(common, DEFAULT_REPO, identity["task_branch"], publication_id)
    raw = journal_path.read_bytes()
    journal = json.loads(raw, object_pairs_hook=unique_json_object)
    if raw != PublicationRecoveryAuthority.encoded(journal) + b"\n":
        raise ValueError("current publication journal is not canonical")
    action_rows = [a for a in journal.get("actions", []) if a.get("action_id") == args.action_id]
    all_record_actions = [a for a in journal.get("actions", []) if a.get("kind") == "record_pr"]
    if len(action_rows) != 1 or len(all_record_actions) != 1:
        raise ValueError("one unique current record-pr action is required")
    action = action_rows[0]
    expected_action = action.get("expected") or {}
    if (action.get("kind") != "record_pr" or action.get("state") != "uncertain"
            or expected_action.get("publication_id") != publication_id
            or expected_action.get("task_uid") != task_uid
            or type(expected_action.get("pr_number")) is not int):
        raise ValueError("current record-pr action is not a persisted uncertain action")
    pr_number = expected_action["pr_number"]
    pr_url = f"https://github.com/{DEFAULT_REPO}/pull/{pr_number}"
    binding = publication_module.build_publication_binding(intent, pr_number, pr_url)
    request_identity = {
        "repository": DEFAULT_REPO, "task_uid": task_uid,
        "issue_number": record["issue_number"], "issue_url": record["issue_url"],
        "pr_number": pr_number, "pr_url": pr_url, "publication_id": publication_id,
        "action_id": args.action_id,
    }
    if request != {"schema": "oasis7-publication-recovery-request/v1",
                   "operation": "resume_uncertain_record_pr", **request_identity}:
        raise ValueError("user recovery request does not bind the exact persisted action")
    matching_bindings = [(c, b) for c, b in binding_comments if b.get("publication_id") == publication_id]
    if len(matching_bindings) > 1 or (matching_bindings and matching_bindings[0][1] != binding):
        raise ValueError("reciprocal publication binding is duplicate or conflicting")

    project_info = mapping.get("project") or {}
    project_owner = str(project_info.get("owner") or DEFAULT_PROJECT_OWNER)
    project_number = int(project_info.get("number") or DEFAULT_PROJECT_NUMBER)
    project_id, _fields = load_sync_module().project_context(project_owner, project_number)
    plan_comments = [c for c in comments if "Plan-Gap Evidence:" in c["body"]]
    if len(plan_comments) != 1:
        raise ValueError("unique live Plan-Gap scope comment is required")
    plan_comment = plan_comments[0]
    current_actor = json.loads(run_text(["gh", "api", "user"])).get("login")
    if not isinstance(current_actor, str) or not current_actor:
        raise ValueError("authenticated current GitHub actor is unavailable")

    archive_uid, _archive_dir, archive_manifest, archive_raw = _unique_helper_archive(
        root, effective_digest, effective_manifest,
    )
    helper_review = {
        "helper_task_uid": archive_uid,
        "archive_manifest_sha256": hashlib.sha256(archive_raw).hexdigest(),
        "helper_source_oid": archive_manifest.get("source_head_oid"),
        "helper_closure_sha256": archive_manifest.get("helper_closure_sha256"),
        "closure_manifest": archive_manifest.get("closure_manifest"),
        "plan_path": archive_manifest.get("plan_path"),
        "plan_sha256": archive_manifest.get("plan_sha256"),
        "ledger_path": archive_manifest.get("ledger_path"),
        "ledger_sha256": archive_manifest.get("ledger_sha256"),
        "role_returns": archive_manifest.get("role_returns"),
    }
    current_action = {
        "publication_id": publication_id, "action_id": args.action_id,
        "journal_sha256": hashlib.sha256(raw).hexdigest(),
        "H": intent["source_head_oid"], "B": intent["planner_authority_oid"],
        "S": intent["source_scope_oid"], "D": intent["projection_digest"],
        "intent_comment_id": intent_comment["id"],
        "intent_body_sha256": hashlib.sha256(intent_comment["body"].encode("utf-8")).hexdigest(),
        "intent_author_login": (intent_comment.get("user") or {}).get("login"),
    }
    admission_envelope = {
        "schema": "oasis7-publication-recovery-admission/v2",
        "identity": {
            "repository": DEFAULT_REPO, "task_uid": task_uid, "issue_number": record["issue_number"],
            "issue_url": record["issue_url"], "pr_number": pr_number, "pr_url": pr_url,
            "project_id": project_id, "project_item_id": record["project_item_id"],
            "canonical_worktree": str(target_root), "source_ref": record["task_branch"],
            "target_ref": record["default_branch"],
        },
        "operation": "record_pr_publication_recovery",
        "recovery_request": _comment_ref(request_comment),
        "plan_scope": _comment_ref(plan_comment),
        "current_action": current_action,
        "helper_review": helper_review,
        "unrelated_snapshot": None,
        "admission_author_login": current_actor,
    }
    args_obj = argparse.Namespace(
        root=target_root, task_uid=task_uid, repo=DEFAULT_REPO,
        project_owner=project_owner, project_number=project_number,
        mapping=".pm/github-project-sync/tasks.json", pr_url=pr_url, draft_candidate=True,
        existing_ready_update=False, role="tpm", publication_binding_json=None,
        recovery_required=True,
    )
    authority = object.__new__(SinglePublicationRecoveryAuthority)
    authority.args, authority.record, authority.binding = args_obj, dict(record), binding
    authority.record["project_id"] = project_id
    authority.intent, authority.module = intent, publication_module
    authority.root = target_root
    authority.envelope = admission_envelope
    authority.comment = intent_comment
    authority.envelope_body = ""
    authority.issue_baseline = None
    authority.project_baseline = None
    authority._scope(comments)
    authority._helpers()
    authority._lineage(comments)
    authority.check(pre_admission=True)
    fence = chr(96) * 3
    body = (SinglePublicationRecoveryAuthority.marker + "\n" + fence + "json\n"
            + authority.encoded(authority.envelope).decode("utf-8") + "\n" + fence)
    if len(body.encode("utf-8")) > 60 * 1024:
        raise ValueError("recovery admission exceeds the supported Task Issue comment size")

    post_url = None
    post_error = None
    try:
        post_url = issue_comment(DEFAULT_REPO, int(record["issue_number"]), body)
    except (OSError, subprocess.SubprocessError, RuntimeError) as exc:
        post_error = exc
    readback_comments = github_issue_comments(DEFAULT_REPO, int(record["issue_number"]))
    matches = [c for c in readback_comments if c.get("body") == body]
    if len(matches) != 1:
        raise ValueError("recovery admission POST is uncertain; exact complete readback did not find one comment") from post_error
    posted = matches[0]
    authority._comment_identity(posted, current_actor)
    if post_url is not None and post_url != posted.get("html_url"):
        raise ValueError("recovery admission POST URL differs from authoritative comment readback")
    admitted = SinglePublicationRecoveryAuthority(
        args_obj, record, binding, intent, publication_module, readback_comments,
    )
    admitted.check()
    print(json.dumps({
        "status": "admitted", "task_uid": task_uid, "action_id": args.action_id,
        "comment_id": posted["id"], "comment_url": posted["html_url"],
        "body_sha256": hashlib.sha256(body.encode("utf-8")).hexdigest(),
        "helper_task_uid": archive_uid, "helper_source_oid": helper_review["helper_source_oid"],
        "effective_helper_head": effective_head,
    }, sort_keys=True))
    return 0


def command_record_pr(args: argparse.Namespace) -> int:
    mapping_path, mapping, record = require_record(args)
    previous = str(record.get("status") or "")
    previous_phase = str(record.get("workflow_phase") or "")
    is_draft_candidate = bool(getattr(args, "draft_candidate", False))
    is_ready_update = bool(getattr(args, "existing_ready_update", False))
    if is_ready_update and is_draft_candidate:
        die("record-pr: existing ready update and draft candidate are mutually exclusive")
    if not re.fullmatch(
        rf"https://github\.com/{re.escape(args.repo)}/pull/[1-9][0-9]*(?:[?#].*)?",
        args.pr_url,
        re.IGNORECASE,
    ):
        die("record-pr: PR URL repository mismatch or malformed PR URL")
    existing_pr_urls = [str(record[key]) for key in ("pr_url", "pull_request_url") if record.get(key)]
    existing_pr_number = record.get("pr_number")
    requested_pr_number = pr_number_from_url(args.pr_url)
    if any(url != args.pr_url for url in existing_pr_urls):
        die("record-pr: a different PR is already bound to this task; same-UID multi-PR lifecycle is not active")
    if existing_pr_number and requested_pr_number != int(existing_pr_number):
        die("record-pr: a different PR number is already bound to this task; same-UID multi-PR lifecycle is not active")
    if previous == "done" or previous_phase in TERMINAL_WORKFLOW_PHASES:
        die(
            "record-pr: terminal task cannot be reclassified; use its canonical finalizer or terminal runbook"
        )
    if is_ready_update:
        if ((previous, previous_phase) not in {("pr_watch", "pr_watch"), ("ready", "pre_pr_ready")}
                or not existing_pr_number or not existing_pr_urls):
            die("record-pr: existing ready update requires the exact already-bound ready/pr_watch PR")
    elif not is_draft_candidate and (previous, previous_phase) != ("ready", "pre_pr_ready"):
        die(
            "record-pr: non-draft pr_watch transition requires task truth at ready/pre_pr_ready; "
            "use prepare-task-pr.sh --promote-draft with canonical CI/review evidence"
        )
    if requested_pr_number is None:
        die("record-pr: PR number is missing or malformed")
    publication_binding_path = getattr(args, "publication_binding_json", None)
    recovery_required = bool(getattr(args, "recovery_required", False))
    publication_binding = None
    publication_intent = None
    publication_module = None
    binding_comment_exists = False
    comments: list[dict[str, Any]] = []
    if publication_binding_path:
        publication_module = load_pr_projection_publication_module()
        try:
            publication_binding = json.loads(
                pathlib.Path(publication_binding_path).read_text(encoding="utf-8")
            )
            publication_binding = publication_module.validate_publication_binding(publication_binding)
        except (OSError, json.JSONDecodeError, ValueError) as exc:
            die(f"record-pr: CI publication binding is invalid: {exc}")
        if (publication_binding["repository"], publication_binding["task_uid"],
                publication_binding["pr_number"], publication_binding["pr_url"]) != (
                args.repo, args.task_uid, requested_pr_number, args.pr_url):
            die("record-pr: CI publication binding differs from canonical Task/PR identity")
        comments = github_issue_comments(args.repo, int(record["issue_number"]))
        publication_records = []
        binding_records = []
        for comment in comments:
            body = str(comment.get("body") or "")
            if "<!-- oasis7-ci-publication/v1 -->" in body:
                try:
                    publication_records.append(publication_module.parse_publication_comment(body))
                except ValueError as exc:
                    die(f"record-pr: malformed CI publication intent: {exc}")
            if "<!-- oasis7-ci-publication-binding/v1 -->" in body:
                try:
                    binding_records.append(publication_module.parse_publication_binding_comment(body))
                except ValueError as exc:
                    die(f"record-pr: malformed CI publication binding: {exc}")
        matching_publications = [
            item for item in publication_records
            if item.get("publication_id") == publication_binding["publication_id"]
        ]
        if len(matching_publications) != 1:
            die("record-pr: exact unique CI publication intent is not present on Task Issue")
        intent = matching_publications[0]
        publication_intent = intent
        conflicting_same_head = [
            item for item in publication_records
            if (item.get("task_uid") == intent["task_uid"]
                    and item.get("source_head_oid") == intent["source_head_oid"]
                    and item.get("source_scope_oid") == intent["source_scope_oid"]
                    and item.get("publication_id") != intent["publication_id"])
        ]
        if conflicting_same_head:
            die("record-pr: same-H Task publication conflict requires explicit invalidation")
        try:
            publication_module.validate_publication_binding(
                publication_binding, intent,
            )
        except ValueError as exc:
            die(f"record-pr: CI publication binding does not match its intent: {exc}")
        matching_bindings = [
            item for item in binding_records
            if item.get("publication_id") == publication_binding["publication_id"]
        ]
        if matching_bindings:
            if len(matching_bindings) != 1 or matching_bindings[0] != publication_binding:
                die("record-pr: conflicting reciprocal CI publication binding already exists")
            binding_comment_exists = True
    if recovery_required and (publication_binding is None or not is_draft_candidate):
        die("record-pr: required recovery needs an exact draft publication binding")
    recovery = None
    recovery_markers = {
        PublicationRecoveryAuthority.marker,
        SinglePublicationRecoveryAuthority.marker,
    }
    recovery_admissions = [
        c for c in comments
        if any(marker in c["body"] for marker in recovery_markers)
    ]
    if recovery_required and len(recovery_admissions) != 1:
        die("record-pr: required publication recovery admission is missing or ambiguous")
    if (publication_binding is not None and is_draft_candidate
            and (recovery_required or recovery_admissions)):
        try:
            if len(recovery_admissions) != 1:
                raise ValueError("one unique record-pr recovery admission is required")
            authority_type = (SinglePublicationRecoveryAuthority
                              if SinglePublicationRecoveryAuthority.marker in recovery_admissions[0]["body"]
                              else PublicationRecoveryAuthority)
            recovery = authority_type(args, record, publication_binding,
                                      publication_intent, publication_module, comments)
            live_issue, _ = recovery.check()
        except (ValueError, OSError, RuntimeError, subprocess.SubprocessError, KeyError, TypeError) as exc:
            die(f"record-pr: recovery admission rejected: {exc}")
    else:
        live_issue = validate_record_pr_live_identity(
        args,
        record,
        requested_pr_number,
        allow_exact_publication_poststate=(publication_binding is not None and is_draft_candidate),
        publication_intent=publication_intent if is_ready_update else None,
        publication_module=publication_module if is_ready_update else None,
    )
    record["pr_url"] = args.pr_url
    number = pr_number_from_url(args.pr_url)
    if number is not None:
        record["pr_number"] = number
    target_status = previous if is_ready_update else ("committed" if is_draft_candidate else "pr_watch")
    target_phase = previous_phase if is_ready_update else ("verification" if is_draft_candidate else "pr_watch")
    record["status"] = target_status
    record["workflow_phase"] = target_phase
    record.setdefault("merge_hold", {
        "kind": "normal_pr_ci_watch",
        "active": False,
        "requester": args.role,
        "reason": "default PR purpose decision recorded by record-pr",
        "resume_authority": args.role,
        "recorded_at": now(),
    })
    record["updated_at"] = now()
    cleared_traceability = set() if recovery else synchronize_live_issue_traceability(
        args.repo, args.task_uid, record, live=live_issue,
    )
    task = task_from_record(args.task_uid, record)
    updated_fields = 0
    pending_issue_body = None
    if recovery:
        sync = load_sync_module()
        project_id, fields = sync.project_context(args.project_owner, args.project_number)
        if project_id != recovery.record["project_id"]:
            die("record-pr: recovery Project context drift")
        desired = sync.project_field_values(task)
        for name in sorted(recovery.project_owned):
            live_issue, values = recovery.check()
            if str(values.get(name) or "") == str(desired.get(name) or ""):
                continue
            try:
                updated, skipped = sync.update_fields(project_id, record["project_item_id"], task, fields,
                                                      only_fields={name}, current_values=values)
            except subprocess.TimeoutExpired:
                _, observed = recovery.check()
                if str(observed.get(name) or "") != str(desired.get(name) or ""):
                    raise
                updated, skipped = 1, []
            if updated != 1:
                die("record-pr: recovery Project effect not confirmed: " + str(skipped))
            updated_fields += updated
    elif record.get("project_item_id"):
        if is_draft_candidate:
            # A draft candidate owns the explicit committed/verification
            # projection. Updating only the PR field left Project Workflow
            # Phase at execution and forced a manual audit/repair retry.
            updated_fields = update_project_fields(
                args,
                task,
                str(record["project_item_id"]),
                require_lifecycle_projection=True,
            )
        else:
            updated_fields = update_project_fields(args, task, str(record["project_item_id"]))
    if recovery:
        live_issue, _ = recovery.check()
        if any(not same_pr_number(live_issue.get(k), v) if k == "pr_number" else live_issue.get(k) != v
               for k, v in {"status": target_status, "workflow_phase": target_phase,
                            "pr_number": number, "pr_url": args.pr_url}.items()):
            body = recovery.live_body
            for key, value in {"status": target_status, "workflow_phase": target_phase,
                               "pr_number": number, "pr_url": args.pr_url}.items():
                line = f"- {key}: `{value}`"
                if re.search(rf"^- {key}:.*$", body, re.M):
                    body = re.sub(rf"^- {key}:.*$", lambda _: line, body, flags=re.M)
                else:
                    body = body.replace("Task metadata:\n", "Task metadata:\n" + line + "\n", 1)
            pending_issue_body = body
            with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as handle:
                handle.write(pending_issue_body)
                path = pathlib.Path(handle.name)
            try:
                try:
                    run_text(["gh", "issue", "edit", str(record["issue_number"]), "-R", args.repo, "--body-file", str(path)])
                except subprocess.TimeoutExpired:
                    observed, _ = recovery.check()
                    if not all(same_pr_number(observed.get(k), v) if k == "pr_number" else observed.get(k) == v
                               for k, v in {"status": target_status, "workflow_phase": target_phase,
                                            "pr_number": number, "pr_url": args.pr_url}.items()):
                        raise
            finally:
                path.unlink(missing_ok=True)
    else:
        update_issue_body(args.repo, int(record["issue_number"]), task)
    evidence_comment = evidence_body(
        args.task_uid,
        args.role,
        target_phase,
        {
            "Completed": "Existing ready PR source binding updated without changing lifecycle state." if is_ready_update else ("Draft Candidate Action recorded without advancing PR watch." if is_draft_candidate else "PR created and task moved to PR watch."),
            "Pending": "Wait for same-head CI receipt." if is_draft_candidate else "Watch required checks, mergeability, comments, and review threads.",
            "Action": "record-pr",
            "Validation Command": args.validation_command,
            "Expected Result": f"Task phase is {target_phase} and PR URL is mapped.",
            "Actual Result": args.pr_url,
            "Blocker / Next Action": "Obtain new-head required CI and all required-role review, applicable current-target strict evidence and fresh merge gate; old-head approvals are historical." if is_ready_update else ("Obtain the same-head CI receipt, complete role review and ready closeout, then promote the draft." if is_draft_candidate else "Continue normal PR watch/fix/merge unless manual packaging hold is explicitly recorded."),
        },
    )
    comment_url = None
    if publication_binding is not None:
        def normalized_evidence(body: str) -> str:
            return re.sub(r"^Recorded At: [^\n]*\n", "", body.replace("\r\n", "\n"), flags=re.MULTILINE)

        matching_evidence = [
            item for item in comments
            if normalized_evidence(str(item.get("body") or "")) == normalized_evidence(evidence_comment)
        ]
        if len(matching_evidence) > 1:
            die("record-pr: duplicate lifecycle evidence comments make reconciliation ambiguous")
        if matching_evidence:
            comment_url = str(matching_evidence[0].get("html_url") or matching_evidence[0].get("url") or "")
            if not comment_url:
                die("record-pr: existing lifecycle evidence comment lacks URL identity")
        else:
            if recovery:
                recovery.check()
            comment_url = verified_issue_comment(
                args.repo, int(record["issue_number"]), evidence_comment,
            )
    else:
        comment_url = issue_comment(args.repo, int(record["issue_number"]), evidence_comment)
    if comment_url not in record.setdefault("evidence_comments", []):
        record["evidence_comments"].append(comment_url)
    binding_comment_url = None
    if publication_binding is not None and not binding_comment_exists:
        body = publication_module.publication_binding_comment(publication_binding)
        if recovery:
            recovery.check()
        binding_comment_url = verified_issue_comment(
            args.repo, int(record["issue_number"]), body,
        )
        record.setdefault("evidence_comments", []).append(binding_comment_url)
    elif publication_binding is not None:
        matching_comment_urls = [
            str(comment.get("html_url") or comment.get("url") or "")
            for comment in comments
            if "<!-- oasis7-ci-publication-binding/v1 -->" in str(comment.get("body") or "")
            and publication_module.parse_publication_binding_comment(str(comment.get("body") or "")) == publication_binding
        ]
        if len(matching_comment_urls) != 1 or not matching_comment_urls[0]:
            die("record-pr: existing reciprocal binding comment readback is ambiguous")
        binding_comment_url = matching_comment_urls[0]
        if binding_comment_url not in record.setdefault("evidence_comments", []):
            record["evidence_comments"].append(binding_comment_url)
    if recovery:
        recovery.check(final=True)
    merge_task_mapping(mapping_path, args.task_uid, record, clear_keys=cleared_traceability)
    payload = {
        "task_uid": args.task_uid,
        "previous_status": previous,
        "status": target_status,
        "workflow_phase": target_phase,
        "issue_url": record.get("issue_url"),
        "pr_url": args.pr_url,
        "pr_number": record.get("pr_number"),
        "comment_url": comment_url,
        "updated_field_values": updated_fields,
        "publication_binding_comment_url": binding_comment_url,
    }
    print(json.dumps(payload, indent=2, sort_keys=True) if args.json else f"record-pr: recorded {args.pr_url} for {args.task_uid}")
    return 0


def load_pr_projection_publication_module() -> Any:
    path = pathlib.Path(__file__).with_name("pr_projection_publication.py")
    spec = importlib.util.spec_from_file_location("pr_projection_publication_impl", path)
    if spec is None or spec.loader is None:
        die(f"record-pr: cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def github_issue_comments(repository: str, issue_number: int) -> list[dict[str, Any]]:
    try:
        pages = json.loads(run_text([
            "gh", "api", f"repos/{repository}/issues/{issue_number}/comments",
            "--paginate", "--slurp",
        ]))
    except (subprocess.SubprocessError, json.JSONDecodeError) as exc:
        die(f"record-pr: Task publication comment readback failed: {exc}")
    if not isinstance(pages, list) or any(not isinstance(page, list) for page in pages):
        die("record-pr: Task publication comment response is malformed")
    comments = [item for page in pages for item in (page if isinstance(page, list) else [page])]
    if any(not isinstance(item, dict) or not isinstance(item.get("body"), str) for item in comments):
        die("record-pr: Task publication comment entry is malformed")
    return comments


def command_set_merge_hold(args: argparse.Namespace) -> int:
    mapping_path, _mapping, record = require_record(args)
    previous = record.get("merge_hold") or {}
    if args.kind == "normal_pr_ci_watch":
        if not previous.get("active"):
            die("set-merge-hold: no active hold to clear")
        if args.resume_authority != previous.get("resume_authority"):
            die("set-merge-hold: resume authority does not match persisted task truth")
        hold = {**previous, "active": False, "cleared_at": now(), "cleared_by": args.requester, "kind": "normal_pr_ci_watch"}
    else:
        if not all((args.requester, args.reason, args.resume_authority)):
            die("set-merge-hold: active hold requires requester, reason, and resume authority")
        hold = {"kind": args.kind, "active": True, "requester": args.requester, "reason": args.reason, "resume_authority": args.resume_authority, "recorded_at": now()}
    issue_number=int(record["issue_number"]); pr_number=int(record.get("pr_number") or 0)
    if pr_number <= 0:
        die("set-merge-hold: task truth has no recorded PR")
    try:
        live_pr=json.loads(run_text(["gh","pr","view",str(pr_number),"--repo",args.repo,"--json","number,headRefOid,url"]))
    except (subprocess.SubprocessError, json.JSONDecodeError) as exc:
        die(f"set-merge-hold: live PR head readback failed: {exc}")
    head_oid=str(live_pr.get("headRefOid") or "")
    if str(live_pr.get("number") or "") != str(pr_number) or not re.fullmatch(r"[0-9a-f]{40}",head_oid,re.I):
        die("set-merge-hold: live PR identity/headRefOid readback is invalid")
    canonical = "\n".join(["<!-- oasis7-merge-hold -->",f"- task_uid: `{args.task_uid}`",f"- repository: `{args.repo}`",f"- issue_number: `{issue_number}`",f"- pr_number: `{pr_number}`",f"- head_oid: `{head_oid}`","- node_id: `merge_hold`","- kind: `merge_hold`",f"- disposition: `{'active' if hold.get('active') else 'cleared'}`",f"- hold_kind: `{hold['kind']}`",f"- active: `{str(hold.get('active',False)).lower()}`",f"- requester: `{hold.get('requester') or ''}`",f"- reason: `{hold.get('reason') or ''}`",f"- resume_authority: `{hold.get('resume_authority') or ''}`",""])
    comment_url = issue_comment(args.repo, issue_number, canonical)
    comment_id=comment_url.rsplit("issuecomment-",1)[-1]
    readback=json.loads(run_text(["gh","api",f"repos/{args.repo}/issues/comments/{comment_id}"]))
    read_body=str(readback.get("body") or "")
    if read_body != canonical: die("set-merge-hold: GitHub comment readback mismatch")
    evidence_receipt={"source":"github_task_issue_comment","runtime_verified":True,"task_uid":args.task_uid,"repository":args.repo,"issue_number":issue_number,"pr_number":pr_number,"head_oid":head_oid,"node_id":"merge_hold","kind":"merge_hold","disposition":"active" if hold.get("active") else "cleared","github_node_id":str(readback.get("id")),"url":readback.get("html_url"),"author":(readback.get("user") or {}).get("login"),"observed_at":readback.get("created_at"),"digest":hashlib.sha256(read_body.encode()).hexdigest()}
    hold["evidence_receipt"]=evidence_receipt
    record["merge_hold"] = hold
    record.setdefault("evidence_comments", []).append(comment_url)
    record["updated_at"] = now()
    cleared_traceability = synchronize_live_issue_traceability(args.repo, args.task_uid, record)
    update_issue_body(args.repo, int(record["issue_number"]), task_from_record(args.task_uid, record))
    merge_task_mapping(mapping_path, args.task_uid, record, clear_keys=cleared_traceability)
    print(json.dumps({"task_uid": args.task_uid, "merge_hold": hold, "comment_url": comment_url}, indent=2, sort_keys=True) if args.json else f"set-merge-hold: {hold['kind']}")
    return 0


def add_common(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("root", type=pathlib.Path)
    parser.add_argument("--repo", default=DEFAULT_REPO)
    parser.add_argument("--project-owner", default=DEFAULT_PROJECT_OWNER)
    parser.add_argument("--project-number", type=int, default=DEFAULT_PROJECT_NUMBER)
    parser.add_argument("--mapping", default=".pm/github-project-sync/tasks.json")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="GitHub Project-backed active PM task lifecycle.")
    subparsers = parser.add_subparsers(dest="command", required=True)

    new_task = subparsers.add_parser("new-task")
    add_common(new_task)
    new_task.add_argument("--owner-role", required=True)
    new_task.add_argument("--title", required=True)
    new_task.add_argument("--module")
    new_task.add_argument("--priority", choices=("P0", "P1", "P2", "P3"), default="P2")
    new_task.add_argument("--source-signal")
    new_task.add_argument("--source-type")
    new_task.add_argument("--severity", choices=("low", "medium", "high", "critical"))
    new_task.add_argument("--source-ref", action="append", default=[], required=True)
    new_task.add_argument("--doc-ref", action="append", default=[])
    new_task.add_argument("--related-prd", action="append", default=[])
    new_task.add_argument("--acceptance", action="append", default=[])
    new_task.add_argument("--handoff-to", action="append", default=[])
    new_task.add_argument("--worktree-hint")
    new_task.add_argument("--primary-package", type=validate_primary_package)
    new_task.add_argument("--loop-binding", help="Full frozen oasis7.loop-task/v1 JSON file")
    new_task.add_argument("--request-key", help="Persisted logical manual request identity")
    new_task.add_argument("--bootstrap-base-oid", help="Fetched immutable bootstrap base for manual tasks")
    new_task.add_argument("--json", action="store_true")
    new_task.set_defaults(func=command_new_task)

    bind = subparsers.add_parser("bind-loop")
    bind.add_argument("--loop-action-json", help=argparse.SUPPRESS)
    add_common(bind)
    bind.add_argument("--task-uid", required=True)
    bind.add_argument("--loop-binding", required=True)
    bind.add_argument("--manual-request-ref", required=True)
    bind.add_argument("--migrate-epoch", type=int)
    bind.add_argument("--json", action="store_true")
    bind.set_defaults(func=command_bind_loop)

    append = subparsers.add_parser("append-execution-log")
    add_common(append)
    append.add_argument("--task-uid", required=True)
    append.add_argument("--role", required=True)
    append.add_argument("--completed", required=True)
    append.add_argument("--pending", required=True)
    append.add_argument("--action", required=True)
    append.add_argument("--validation-command", required=True)
    append.add_argument("--expected-result", required=True)
    append.add_argument("--actual-result", required=True)
    append.add_argument("--blocker-next-action", required=True)
    append.add_argument("--json", action="store_true")
    append.set_defaults(func=command_append_evidence)

    classify = subparsers.add_parser("classify-non-pr-task")
    add_common(classify)
    classify.add_argument("--task-uid", required=True)
    classify.add_argument("--role", default="repository_health_engineer")
    classify.add_argument("--evidence", required=True)
    classify.add_argument("--validation-command", default="classify-non-pr-task")
    classify.add_argument("--json", action="store_true")
    classify.set_defaults(func=command_classify_non_pr_task)

    report = subparsers.add_parser("workflow-report")
    add_common(report)
    report.add_argument("--role", required=True)
    report.add_argument("--phase", choices=("start", "close", "review"), default="start")
    report.add_argument("--task-uid")
    report.add_argument("--stale-after-days", type=int, default=7)
    report.add_argument("--json", action="store_true")
    report.set_defaults(func=command_workflow_report)

    move = subparsers.add_parser("move-task")
    add_common(move)
    move.add_argument("--task-uid", required=True)
    move.add_argument("--to-status", required=True, choices=ALL_STATUSES)
    move.add_argument("--json", action="store_true")
    move.set_defaults(func=command_move_task)

    closeout = subparsers.add_parser("closeout-task")
    add_common(closeout)
    closeout.add_argument("--task-uid", required=True)
    closeout.add_argument("--role", required=True)
    closeout.add_argument("--to-status", required=True, choices=("ready", "done", "deferred"))
    closeout.add_argument("--claim-json", required=True)
    closeout.add_argument("--pr-receipt")
    closeout.add_argument("--aggregate-plan")
    closeout.add_argument("--aggregate-candidate")
    closeout.add_argument("--aggregate-evidence")
    closeout.add_argument("--aggregate-receipt")
    closeout.add_argument("--json", action="store_true")
    closeout.set_defaults(func=command_closeout_task)

    bind_aggregate = subparsers.add_parser("bind-aggregate-plan")
    add_common(bind_aggregate)
    bind_aggregate.add_argument("--task-uid", required=True)
    bind_aggregate.add_argument("--plan", required=True)
    bind_aggregate.add_argument("--comment-id", type=int, required=True)
    bind_aggregate.add_argument("--json", action="store_true")
    bind_aggregate.set_defaults(func=command_bind_aggregate_plan)

    phase = subparsers.add_parser("set-phase")
    add_common(phase)
    phase.add_argument("--task-uid", required=True)
    phase.add_argument("--role", default="tpm")
    phase.add_argument("--phase", required=True, choices=("main_sync", "post_merge_done"))
    phase.add_argument("--receipt-json", required=True)
    phase.add_argument("--aggregate-plan")
    phase.add_argument("--aggregate-candidate")
    phase.add_argument("--aggregate-evidence")
    phase.add_argument("--aggregate-receipt")
    phase.add_argument("--json", action="store_true")
    phase.set_defaults(func=command_set_phase)

    refresh = subparsers.add_parser("refresh-task")
    add_common(refresh)
    refresh.add_argument("--task-uid", required=True)
    refresh.add_argument("--json", action="store_true")
    refresh.set_defaults(func=command_refresh_task)

    record_pr = subparsers.add_parser("record-pr")
    add_common(record_pr)
    record_pr.add_argument("--task-uid", required=True)
    record_pr.add_argument("--pr-url", required=True)
    record_pr.add_argument("--role", default="tpm")
    record_pr.add_argument("--validation-command", default="./scripts/prepare-task-pr.sh --create")
    record_pr.add_argument("--draft-candidate", action="store_true")
    record_pr.add_argument("--existing-ready-update", action="store_true")
    record_pr.add_argument("--publication-binding-json")
    record_pr.add_argument("--recovery-required", action="store_true")
    record_pr.add_argument("--json", action="store_true")
    record_pr.set_defaults(func=command_record_pr)

    archive_review = subparsers.add_parser("archive-publication-helper-review")
    archive_review.add_argument("--task-uid", required=True)
    archive_review.set_defaults(func=command_archive_publication_helper_review)

    admit_recovery = subparsers.add_parser("admit-record-pr-recovery")
    admit_recovery.add_argument("--task-uid", required=True)
    admit_recovery.add_argument("--action-id", required=True)
    admit_recovery.set_defaults(func=command_admit_record_pr_recovery)

    hold = subparsers.add_parser("set-merge-hold")
    add_common(hold)
    hold.add_argument("--task-uid", required=True)
    hold.add_argument("--kind", required=True, choices=("normal_pr_ci_watch", "manual_packaging_ci_hold", "user_requested_merge_hold"))
    hold.add_argument("--requester", required=True)
    hold.add_argument("--reason", default="")
    hold.add_argument("--resume-authority", required=True)
    hold.add_argument("--role", default="tpm")
    hold.add_argument("--json", action="store_true")
    hold.set_defaults(func=command_set_merge_hold)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    try:
        code = main()
    except Exception as exc:
        api_code = load_sync_module().github_api_error_exit(exc, "github-project-task")
        if api_code is None:
            raise
        raise SystemExit(api_code)
    raise SystemExit(code)
