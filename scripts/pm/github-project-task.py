#!/usr/bin/env python3
from __future__ import annotations

import argparse
import base64
import hashlib
import importlib.util
import json
import os
import pathlib
import re
import subprocess
import sys
import tempfile
import uuid
from collections import OrderedDict
from datetime import datetime
from typing import Any


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
    if not path.exists():
        return {"version": 1, "tasks": {}}
    return json.loads(path.read_text(encoding="utf-8"))


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
                                         "--json", "body,number,title,url,state,stateReason"]))
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


def validate_record_pr_live_identity(
    args: argparse.Namespace,
    record: dict[str, Any],
    pr_number: int,
    *,
    allow_exact_publication_poststate: bool = False,
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
    commit = binding.get("policy_commit", "")
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        die("manual admission requires immutable policy commit")
    if run_text(["git", "-C", str(tool_root), "rev-parse", "HEAD"]) != commit:
        die("manual admission helper is not running from pinned effective policy")
    run_text(["git", "-C", str(tool_root), "diff", "--no-ext-diff", "--no-textconv", "--exit-code", commit, "--", "scripts/pm"])
    shadows = run_text(["git", "-C", str(tool_root), "ls-files", "--others", "--", "scripts/pm"])
    if any(path.endswith((".py", ".sh", ".json")) for path in shadows.splitlines()):
        die("untracked executable authority in manual helper root")
    sys.path.insert(0, str(pathlib.Path(__file__).parent))
    try:
        import loop_policy
        import loop_contracts
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
            bindings[uid] = validate_loop_binding(dependency)
            pending.extend(dependency["dependencies"])
        result = loop_policy.validate_dependencies(binding, bindings)
        if result.get("status") != "passed":
            die("manual dependency admission blocked: " + str(result.get("blockers")))
    finally:
        sys.path.pop(0)


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
    if args.to_status == "done":
        require_live_issue_route_matches_cache(getattr(args, "repo", DEFAULT_REPO), args.task_uid, original)
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


def project_refresh_graphql(query: str, variables: list[str]) -> dict[str, Any]:
    return json.loads(run_text(["gh", "api", "graphql", "-f", f"query={query}", *variables]))


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
    existing = dict((latest.get("tasks") or {}).get(args.task_uid) or {})
    root = args.root.resolve()
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
        payload = project_refresh_graphql(query, ["-F", f"ids[]={item_id}"])
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
        payload = project_refresh_graphql(query, ["-f", f"q=repo:{args.repo} {args.task_uid} in:body"])
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


def command_record_pr(args: argparse.Namespace) -> int:
    mapping_path, mapping, record = require_record(args)
    previous = str(record.get("status") or "")
    previous_phase = str(record.get("workflow_phase") or "")
    is_draft_candidate = bool(getattr(args, "draft_candidate", False))
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
    if not is_draft_candidate and (previous, previous_phase) != ("ready", "pre_pr_ready"):
        die(
            "record-pr: non-draft pr_watch transition requires task truth at ready/pre_pr_ready; "
            "use prepare-task-pr.sh --promote-draft with canonical CI/review evidence"
        )
    if requested_pr_number is None:
        die("record-pr: PR number is missing or malformed")
    publication_binding = None
    publication_module = None
    binding_comment_exists = False
    comments: list[dict[str, Any]] = []
    if getattr(args, "publication_binding_json", None):
        publication_module = load_pr_projection_publication_module()
        try:
            publication_binding = json.loads(
                pathlib.Path(args.publication_binding_json).read_text(encoding="utf-8")
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
    live_issue = validate_record_pr_live_identity(
        args,
        record,
        requested_pr_number,
        allow_exact_publication_poststate=(publication_binding is not None and is_draft_candidate),
    )
    record["pr_url"] = args.pr_url
    number = pr_number_from_url(args.pr_url)
    if number is not None:
        record["pr_number"] = number
    target_status = "committed" if is_draft_candidate else "pr_watch"
    target_phase = "verification" if is_draft_candidate else "pr_watch"
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
    cleared_traceability = synchronize_live_issue_traceability(
        args.repo, args.task_uid, record, live=live_issue,
    )
    task = task_from_record(args.task_uid, record)
    updated_fields = 0
    if record.get("project_item_id"):
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
    update_issue_body(args.repo, int(record["issue_number"]), task)
    evidence_comment = evidence_body(
        args.task_uid,
        args.role,
        target_phase,
        {
            "Completed": "Draft Candidate Action recorded without advancing PR watch." if is_draft_candidate else "PR created and task moved to PR watch.",
            "Pending": "Wait for same-head CI receipt." if is_draft_candidate else "Watch required checks, mergeability, comments, and review threads.",
            "Action": "record-pr",
            "Validation Command": args.validation_command,
            "Expected Result": f"Task phase is {target_phase} and PR URL is mapped.",
            "Actual Result": args.pr_url,
            "Blocker / Next Action": "Obtain the same-head CI receipt, complete role review and ready closeout, then promote the draft." if is_draft_candidate else "Continue normal PR watch/fix/merge unless manual packaging hold is explicitly recorded.",
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
    if not isinstance(pages, list):
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
    record_pr.add_argument("--publication-binding-json")
    record_pr.add_argument("--json", action="store_true")
    record_pr.set_defaults(func=command_record_pr)

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
    raise SystemExit(main())
