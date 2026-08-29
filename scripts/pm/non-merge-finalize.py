#!/usr/bin/env python3
"""Finalize a receipt-bound task without a merge receipt.

This is deliberately a separate terminal writer from post-merge-finalize.py.
The two paths share only the Project/Issue identity checks; a non-merge
decision can never mint or satisfy merge/main-sync authority.
"""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import importlib.util
import json
import pathlib
import re
import subprocess
import sys
import tempfile
from collections import OrderedDict

from portable_file_lock import ensure_lock_byte, fcntl


SCRIPT_DIR = pathlib.Path(__file__).resolve().parent
CANONICAL_ROOT_HELPER = SCRIPT_DIR / "canonical-receipt-root.py"
REASONS = ("superseded", "duplicate", "not_planned", "non_pr_completed")
# Non-PR closure is intentionally a negative authority: it must never create
# merge_receipt, merge_receipt_sha256, pr_number, or pr_url authority.
NON_PR_FORBIDDEN_AUTHORITY = ("merge_receipt", "merge_receipt_sha256", "pr_number", "pr_url")
NON_PR_AUTHORITY_NOTE = "non_pr_completed rejects merge_receipt and pr_number authority"
MERGE_AUTHORITY_FIELDS = (
    "merge_receipt", "merge_receipt_sha256", "main_sync_receipt",
    "main_sync_receipt_sha256", "main_sync_authority", "cleanup_receipt",
    "cleanup_authority",
)
STATUS_LINE_RE = re.compile(r"(?m)^- status: `([^`]+)`$")
PR_HEAD_REQUIRED_FIELDS = ("headRefOid", "headRefName")
PR_HEAD_OPTIONAL_FIELDS = ("headRepositoryOwner", "headRepositoryName")
PR_HEAD_FIELDS = PR_HEAD_REQUIRED_FIELDS + PR_HEAD_OPTIONAL_FIELDS
TERMINAL_CAS_FIELDS = (
    "task_uid", "repository", "issue_number", "issue_url", "project_item_id",
    "canonical_worktree", "task_branch", "default_branch", "status",
    "workflow_phase", "pr_number", "pr_url", "completion_mode",
    "non_pr_completion_evidence", "closed_without_merge_reason",
    "closed_without_merge_evidence_sha256", *MERGE_AUTHORITY_FIELDS,
    *PR_HEAD_FIELDS,
)
PROJECT_IDENTITY_FIELDS = ("owner", "number", "id")
NON_MERGE_INTENT_FIELD = "closed_without_merge_intent"
RECEIPT_NAME = "closed-without-merge-receipt.json"
MIGRATED_RECEIPT_NAME = "closed-without-merge-receipt-migrated.json"
LEDGER_NAME = "non-merge-finalizer-ledger.json"
MIGRATED_LEDGER_NAME = "non-merge-finalizer-ledger-migrated.json"
NON_MERGE_RECEIPT_SCHEMA_VERSION = 1
ELIGIBLE_NON_MERGE_PHASES = {
    "", "bootstrap", "planning", "execution", "verification",
    "pre_pr_review", "pre_pr_ready", "pr_watch", "blocked",
    "task_done", "closed_without_merge",
}


def _intent_matches(stored: object, expected: dict) -> bool:
    if not isinstance(stored, dict):
        return False
    normalized = dict(stored)
    if "migrated_receipt_sha256" not in expected:
        normalized.pop("migrated_receipt_sha256", None)
    if "canonical_receipt_sha256" not in expected:
        normalized.pop("canonical_receipt_sha256", None)
    return normalized == expected


def _load_module(name: str, path: pathlib.Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot import {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


durable_store = _load_module("workflow_durable_store", SCRIPT_DIR / "workflow-durable-store.py")
project_workflow = _load_module("github_project_workflow", SCRIPT_DIR / "github-project-workflow.py")
project_sync = _load_module("github_project_sync", SCRIPT_DIR / "github-project-sync.py")


def fail(message: str) -> None:
    raise SystemExit(f"non-merge-finalize: {message}")


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat()


def _has_authority(record: dict, fields: tuple[str, ...]) -> bool:
    return any(
        field in record and record.get(field) not in (None, "", {}, [], False)
        for field in fields
    )


def _has_non_pr_task_classification(record: dict) -> bool:
    return (
        str(record.get("completion_mode") or "") == "non_pr_task"
        and record.get("non_pr_completion_evidence") not in (None, "", {}, [], False)
    )


def run_json(command: list[str]) -> dict:
    result = subprocess.run(command, text=True, capture_output=True, check=True)
    try:
        value = json.loads(result.stdout or "{}")
    except json.JSONDecodeError as exc:
        fail(f"invalid JSON from {' '.join(command[:3])}: {exc}")
    return value if isinstance(value, dict) else {}


def _mapping(root: pathlib.Path, task_uid: str) -> tuple[pathlib.Path, dict, dict]:
    path = root / ".pm/github-project-sync/tasks.json"
    if not path.is_file():
        fail("task mapping is unavailable")
    try:
        mapping = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        fail(f"task mapping cannot be read: {exc}")
    tasks = mapping.get("tasks") or {}
    if task_uid not in tasks or not isinstance(tasks[task_uid], dict):
        fail(f"unknown task UID: {task_uid}")
    record = tasks[task_uid]
    return path, mapping, record


def _validate_default_worktree(root: pathlib.Path) -> None:
    result = subprocess.run(
        ["git", "-C", str(root), "worktree", "list", "--porcelain"],
        text=True, capture_output=True,
    )
    if result.returncode:
        fail("repo root is not a registered git worktree")
    first = next(
        (line.removeprefix("worktree ") for line in result.stdout.splitlines()
         if line.startswith("worktree ")),
        "",
    )
    if not first or pathlib.Path(first).resolve() != root.resolve():
        fail("repo root must be the registered default worktree")


def _receipt_path(root: pathlib.Path, task_uid: str) -> pathlib.Path:
    result = subprocess.run(
        [sys.executable, str(CANONICAL_ROOT_HELPER), "--default-worktree", str(root),
         "--task-uid", task_uid, "--create", "--name", RECEIPT_NAME],
        text=True, capture_output=True,
    )
    if result.returncode:
        fail(result.stderr.strip() or "noncanonical receipt root")
    return pathlib.Path(result.stdout.strip()).resolve()


def _migrated_receipt_path(path: pathlib.Path) -> pathlib.Path:
    return path.with_name(MIGRATED_RECEIPT_NAME)


def _migrated_ledger_path(path: pathlib.Path) -> pathlib.Path:
    return path.with_name(MIGRATED_LEDGER_NAME)


def _read_json_object(path: pathlib.Path, label: str) -> dict:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        fail(f"{label} is unreadable: {exc}")
    if not isinstance(value, dict):
        fail(f"{label} is not an object")
    return value


def _write_immutable_json(path: pathlib.Path, value: dict, label: str) -> None:
    encoded = (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n").encode()
    if path.exists():
        try:
            existing = path.read_bytes()
        except OSError as exc:
            fail(f"{label} is unreadable: {exc}")
        if existing != encoded:
            fail(f"{label} immutable content conflict")
        return
    durable_store.atomic_replace_json(path, value)


def resolve_non_merge_receipt(
    root: pathlib.Path, task_uid: str,
) -> tuple[pathlib.Path, pathlib.Path, dict | None, bytes]:
    """Resolve the immutable canonical receipt or its source-bound sidecar."""
    canonical_path = _receipt_path(root, task_uid)
    try:
        canonical_bytes = canonical_path.read_bytes() if canonical_path.exists() else b""
    except OSError as exc:
        raise ValueError(f"existing non-merge receipt is unreadable: {exc}") from exc
    canonical = (_read_json_object(canonical_path, "existing non-merge receipt")
                 if canonical_path.exists() else None)
    migrated_path = _migrated_receipt_path(canonical_path)
    if not migrated_path.exists():
        return canonical_path, canonical_path, canonical, canonical_bytes
    if not canonical_bytes:
        raise ValueError("migrated non-merge receipt is missing its immutable source")
    migrated = _read_json_object(migrated_path, "migrated non-merge receipt")
    source_digest = hashlib.sha256(canonical_bytes).hexdigest()
    if (migrated.get("migrated_from") != RECEIPT_NAME
            or migrated.get("migrated_from_sha256") != source_digest):
        raise ValueError("migrated non-merge receipt source drifted")
    return canonical_path, migrated_path, migrated, canonical_bytes


def _read_evidence(path: pathlib.Path) -> tuple[str, dict, str]:
    try:
        raw = path.read_bytes()
    except OSError as exc:
        fail(f"evidence file cannot be read: {exc}")
    if not raw.strip():
        fail("evidence file must not be empty")
    if len(raw) > 8192:
        fail("evidence file exceeds the 8192-byte Issue evidence limit")
    digest = hashlib.sha256(raw).hexdigest()
    try:
        parsed = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError):
        parsed = {"text": raw.decode("utf-8", errors="replace").strip()}
    if not isinstance(parsed, dict):
        parsed = {"value": parsed}
    public_evidence = raw.decode("utf-8", errors="replace").strip()
    return digest, parsed, public_evidence


def _has_verified_task_complete(record: dict) -> bool:
    for claim in record.get("claim_verifications") or []:
        if not isinstance(claim, dict):
            continue
        if (str(claim.get("claim_type") or "") == "task_complete"
                and str(claim.get("status") or "") == "verified"
                and claim.get("allowed_to_claim") is True
                and str(claim.get("verification_exit_code") or "") in {"", "0"}):
            return True
    return False


def _identity_issue(record: dict, task_uid: str, *, allow_closed: bool = False) -> dict:
    repository = str(record.get("repository") or "")
    issue_number = str(record.get("issue_number") or "")
    if not repository or not issue_number:
        fail("task truth is missing repository or issue_number identity")
    issue = run_json(["gh", "issue", "view", issue_number, "-R", repository,
                      "--json", "state,stateReason,body,number,url"])
    body = str(issue.get("body") or "")
    expected_url = f"https://github.com/{repository}/issues/{issue_number}"
    if (str(issue.get("number") or "") != issue_number
            or str(issue.get("url") or "") != str(record.get("issue_url") or expected_url)
            or not re.search(rf"^task_uid:\s*{re.escape(task_uid)}$", body, re.MULTILINE)):
        fail("Issue identity mismatch")
    body_pr_urls = re.findall(r"(?m)^- pr_url: `([^`]*)`$", body)
    body_pr_numbers = re.findall(r"(?m)^- pr_number: `([^`]*)`$", body)
    mapped_pr_url = str(record.get("pr_url") or "")
    mapped_pr_number = str(record.get("pr_number") or "")
    if body_pr_urls or body_pr_numbers:
        if (len(body_pr_urls) != 1 or len(body_pr_numbers) != 1
                or (body_pr_urls[0], body_pr_numbers[0])
                != (mapped_pr_url, mapped_pr_number)):
            fail("Issue PR identity disagrees with task mapping")
    if str(issue.get("state") or "").upper() == "CLOSED" and not allow_closed:
        fail("Issue is already CLOSED without a matching non-merge receipt")
    return issue


def _identity_pr(record: dict) -> dict | None:
    pr_number = str(record.get("pr_number") or "")
    pr_url = str(record.get("pr_url") or "")
    if not pr_number and not pr_url:
        return None
    if not pr_number or not pr_url:
        fail("recorded PR identity is incomplete")
    pr = run_json(["gh", "pr", "view", pr_number, "-R", str(record["repository"]),
                   "--json", "number,url,state,mergedAt,headRefOid,headRefName,"
                   "headRepositoryOwner,headRepository"])
    merged_at = pr.get("mergedAt")
    if merged_at not in (None, ""):
        fail("recorded PR is MERGED; use post-merge-finalize with its merge receipt")
    if str(pr.get("state") or "").upper() != "CLOSED":
        fail("recorded PR must be CLOSED and unmerged before non-merge finalization")
    if (str(pr.get("number") or "") != pr_number
            or str(pr.get("url") or "") != pr_url):
        fail("recorded PR identity mismatch")
    return pr


def _pr_head_authority(pr: dict) -> dict[str, str]:
    """Extract the immutable PR head identity required by non-merge closeout."""
    authority: dict[str, str] = {}
    for key in PR_HEAD_REQUIRED_FIELDS:
        value = str(pr.get(key) or "")
        if not value:
            fail(f"recorded PR is missing required {key} head authority")
        authority[key] = value
    raw_owner = pr.get("headRepositoryOwner")
    raw_owner = raw_owner if isinstance(raw_owner, dict) else {}
    raw_repository = pr.get("headRepository")
    raw_repository = raw_repository if isinstance(raw_repository, dict) else {}
    owner = str(raw_owner.get("login") or "")
    repository = str(raw_repository.get("name") or "")
    if owner and repository:
        authority.update({
            "headRepositoryOwner": owner,
            "headRepositoryName": repository,
        })
    return authority


def _bind_pr_head(record: dict, pr: dict | None,
                  existing_receipt: dict | None = None) -> dict[str, str]:
    """Bind the live PR head and fail closed on any stored-head mismatch."""
    if pr is None:
        return {}
    authority = _pr_head_authority(pr)
    for key in PR_HEAD_FIELDS:
        if key in record and str(record.get(key) or "") != authority.get(key, ""):
            fail(f"recorded PR {key} drifted from live head authority")
        if existing_receipt is not None and key in existing_receipt:
            if str(existing_receipt.get(key) or "") != authority.get(key, ""):
                fail(f"existing non-merge receipt {key} drifted from live PR head")
    record.update(authority)
    return authority


def _project_identity(project: dict) -> dict[str, str]:
    """Return the canonical mapping-level Project identity for CAS/receipts."""
    return {key: str(project.get(key) or "") for key in PROJECT_IDENTITY_FIELDS}


def _expected_issue_state_reason(reason: str) -> str:
    return "COMPLETED" if reason == "non_pr_completed" else "NOT_PLANNED"


def _verify_receipt_bytes(path: pathlib.Path, expected_digest: str) -> None:
    if hashlib.sha256(path.read_bytes()).hexdigest() != expected_digest:
        fail("non-merge receipt bytes drifted during terminal effects")


def _closeout_intent(task_uid: str, record: dict, reason: str,
                     evidence_digest: str,
                     project_identity: dict[str, str]) -> dict:
    intent = {
        "schema": "oasis7_non_merge_closeout_intent_v1",
        "task_uid": task_uid,
        "repository": record.get("repository"),
        "issue_number": record.get("issue_number"),
        "project_item_id": record.get("project_item_id"),
        "project_identity": project_identity,
        "reason": reason,
        "evidence_sha256": evidence_digest,
        "previous_status": record.get("status"),
        "previous_workflow_phase": record.get("workflow_phase") or "",
        "pr_number": record.get("pr_number"),
        "pr_url": record.get("pr_url"),
    }
    for key in PR_HEAD_FIELDS:
        if key in record:
            intent[key] = record[key]
    return intent


def _evidence_payload_matches(stored: object, current: dict) -> bool:
    """Accept legacy text receipts without erasing their newline spelling."""
    if stored == current:
        return True
    if (not isinstance(stored, dict) or set(stored) != {"text"}
            or set(current) != {"text"}
            or not isinstance(stored.get("text"), str)
            or not isinstance(current.get("text"), str)):
        return False
    stored_text = stored["text"]
    current_text = current["text"]
    # Older readers sometimes retained leading/trailing whitespace around a
    # plain-text payload.  The receipt digest still binds the exact raw bytes,
    # so this normalization cannot authorize a different evidence file.
    return stored_text.strip() == current_text.strip()


def _receipt_recovery_candidate(record: dict, task_uid: str, reason: str,
                                evidence_digest: str,
                                existing_receipt: dict | None,
                                *, evidence_data: dict | None = None,
                                pr: dict | None = None,
                                project_identity: dict[str, str] | None = None,
                                pr_head: dict[str, str] | None = None) -> bool:
    if str(record.get("workflow_phase") or "") != "closed_without_merge":
        return False
    if str(record.get("status") or "") != "done":
        return False
    if not isinstance(existing_receipt, dict):
        return False
    expected = {
        "receipt_type": "oasis7_closed_without_merge",
        "schema_version": NON_MERGE_RECEIPT_SCHEMA_VERSION,
        "issuer": "non-merge-finalize",
        "task_uid": task_uid,
        "repository": record.get("repository"),
        "issue_number": record.get("issue_number"),
        "project_item_id": record.get("project_item_id"),
        "reason": reason,
        "evidence_sha256": evidence_digest,
        "pr_number": record.get("pr_number"),
        "pr_url": record.get("pr_url"),
    }
    if pr is not None:
        expected.update({"pr_state": pr.get("state"), "mergedAt": pr.get("mergedAt")})
    required = tuple(expected)
    if any(key not in existing_receipt or existing_receipt.get(key) != value
           for key, value in expected.items() if key in required):
        return False
    # The terminal mapping may retain a durable reason/evidence snapshot or
    # intent from the older writer.  If present, it is immutable authority;
    # a missing field is legacy-compatible, but a disagreement is not.
    for key, value in {
        "closed_without_merge_reason": reason,
        "closed_without_merge_evidence_sha256": evidence_digest,
    }.items():
        if key in record and record.get(key) != value:
            return False
    intent = record.get(NON_MERGE_INTENT_FIELD)
    if intent is not None:
        if not isinstance(intent, dict):
            return False
        for key, value in expected.items():
            if key in intent and intent.get(key) != value:
                return False
        if pr_head:
            for key, value in pr_head.items():
                if key in intent and intent.get(key) != value:
                    return False
    embedded = record.get("closed_without_merge_receipt")
    if embedded is not None:
        if not isinstance(embedded, dict):
            return False
        for key, value in existing_receipt.items():
            if key in embedded and embedded.get(key) != value:
                return False
    if evidence_data is not None and "evidence" in existing_receipt:
        if not _evidence_payload_matches(existing_receipt.get("evidence"), evidence_data):
            return False
    if project_identity is not None and "project_identity" in existing_receipt:
        if existing_receipt.get("project_identity") != project_identity:
            return False
    if pr_head:
        for key, value in pr_head.items():
            if key in existing_receipt and existing_receipt.get(key) != value:
                return False
    return True


def _validate_existing_receipt(existing_receipt: dict, receipt: dict,
                               evidence_data: dict,
                               project_identity: dict[str, str],
                               pr_head: dict[str, str],
                               *, require_modern_authority: bool = False,
                               legacy_source: dict | None = None) -> None:
    expected = {
        "receipt_type": receipt["receipt_type"],
        "schema_version": receipt["schema_version"],
        "issuer": receipt["issuer"],
        "task_uid": receipt["task_uid"],
        "repository": receipt["repository"],
        "issue_number": receipt["issue_number"],
        "project_item_id": receipt["project_item_id"],
        "reason": receipt["reason"],
        "evidence_sha256": receipt["evidence_sha256"],
        "pr_number": receipt.get("pr_number"),
        "pr_url": receipt.get("pr_url"),
        "pr_state": receipt.get("pr_state"),
        "mergedAt": receipt.get("mergedAt"),
        "previous_status": receipt.get("previous_status"),
        "previous_workflow_phase": receipt.get("previous_workflow_phase"),
    }
    expected.update({key: receipt[key] for key in pr_head if key in existing_receipt})
    if require_modern_authority:
        required_modern = {
            "evidence", "project_identity", "previous_status",
            "previous_workflow_phase",
        }
        if pr_head:
            required_modern.update(PR_HEAD_REQUIRED_FIELDS)
        if any(key not in existing_receipt for key in required_modern):
            fail("migrated non-merge receipt is missing modern authority")
        allowed_modern = {
            *expected, "evidence", "project_identity", *PR_HEAD_FIELDS,
            "migrated_from", "migrated_from_sha256", "observed_at",
        }
        unknown = set(existing_receipt) - allowed_modern
        source_bound = {
            key for key in unknown
            if isinstance(legacy_source, dict)
            and key in legacy_source
            and existing_receipt.get(key) == legacy_source.get(key)
        }
        unknown -= source_bound
        if unknown:
            fail("migrated non-merge receipt has unrecognized immutable fields")
        # Repository identity is optional in the PR head authority.  The
        # required ref OID/name are enforced above only for PR-bound tasks.
        legacy_optional = set(PR_HEAD_OPTIONAL_FIELDS)
    else:
        # A canonical receipt may be a pre-migration legacy record.  Keep its
        # historical omissions compatible while validating every value that
        # is present; migration writes a new immutable sidecar.
        legacy_optional = {
            "evidence", "project_identity", "previous_status",
            "previous_workflow_phase", *PR_HEAD_FIELDS,
        }
    for key, value in expected.items():
        if key not in existing_receipt:
            if key not in legacy_optional:
                fail("existing non-merge receipt is missing immutable identity authority")
            continue
        if existing_receipt.get(key) != value:
            fail("existing non-merge receipt disagrees with current task/reason/evidence authority")
    if "evidence" in existing_receipt and not _evidence_payload_matches(
            existing_receipt.get("evidence"), evidence_data):
        fail("existing non-merge receipt disagrees with current embedded evidence")
    if ("project_identity" in existing_receipt
            and existing_receipt.get("project_identity") != project_identity):
        fail("existing non-merge receipt disagrees with current Project identity authority")
    for key, value in pr_head.items():
        if key in existing_receipt and existing_receipt.get(key) != value:
            fail(f"existing non-merge receipt {key} drifted from live PR head authority")


def _receipt_needs_migration(existing_receipt: dict, pr_head: dict[str, str]) -> bool:
    required = {
        "evidence", "project_identity", "previous_status",
        "previous_workflow_phase", *PR_HEAD_REQUIRED_FIELDS,
    }
    if not pr_head:
        required.difference_update(PR_HEAD_REQUIRED_FIELDS)
    return any(key not in existing_receipt for key in required)


def _bind_migrated_receipt_digest(path: pathlib.Path, task_uid: str,
                                  digest: str) -> None:
    """Bind a migration sidecar before any remote effect can consume it."""
    def update(mapping: dict) -> None:
        current = (mapping.get("tasks") or {}).get(task_uid) or {}
        intent = current.get(NON_MERGE_INTENT_FIELD)
        if not isinstance(intent, dict):
            fail("migrated non-merge receipt has no pending intent authority")
        bound = intent.get("migrated_receipt_sha256")
        if bound not in (None, digest):
            fail("migrated non-merge receipt digest authority drifted")
        intent["migrated_receipt_sha256"] = digest
        current[NON_MERGE_INTENT_FIELD] = intent
        mapping.setdefault("tasks", {})[task_uid] = current

    durable_store.transact_json(path, update)


def _bind_canonical_receipt_digest(path: pathlib.Path, task_uid: str,
                                   digest: str) -> None:
    def update(mapping: dict) -> None:
        current = (mapping.get("tasks") or {}).get(task_uid) or {}
        intent = current.get(NON_MERGE_INTENT_FIELD)
        if not isinstance(intent, dict):
            fail("canonical non-merge receipt has no pending intent authority")
        bound = intent.get("canonical_receipt_sha256")
        if bound not in (None, digest):
            fail("canonical non-merge receipt digest authority drifted")
        intent["canonical_receipt_sha256"] = digest
        current[NON_MERGE_INTENT_FIELD] = intent
        mapping.setdefault("tasks", {})[task_uid] = current
    durable_store.transact_json(path, update)


def _bind_terminal_migrated_receipt_digest(path: pathlib.Path, task_uid: str,
                                           digest: str) -> None:
    """Bind an adopted sidecar on an already-terminal legacy mapping."""
    def update(mapping: dict) -> None:
        current = (mapping.get("tasks") or {}).get(task_uid) or {}
        if (current.get("status"), current.get("workflow_phase")) != (
                "done", "closed_without_merge"):
            fail("terminal migrated receipt digest lacks terminal mapping authority")
        digests = current.setdefault("phase_receipt_sha256", {})
        bound = digests.get("closed_without_merge")
        if bound not in (None, digest):
            fail("terminal migrated receipt digest authority drifted")
        digests["closed_without_merge"] = digest
        mapping.setdefault("tasks", {})[task_uid] = current

    durable_store.transact_json(path, update)


def _project_readback(record: dict, task_uid: str) -> tuple[str, dict, dict]:
    item_id = str(record.get("project_item_id") or "")
    if not item_id:
        fail("task truth is missing project_item_id")
    project_meta = record.get("_project") or record.get("project") or {}
    owner = str(project_meta.get("owner") or "")
    number = int(project_meta.get("number") or 0)
    expected_project_id = str(project_meta.get("id") or "")
    if not owner or not number:
        fail("task truth is missing canonical Project owner/number")
    item = project_workflow.fetch_project_items_by_ids([item_id]).get(item_id) or {}
    if item.get("_field_values_has_next_page") is not False:
        fail("bound Project item fieldValues pagination is incomplete")
    content = item.get("content") or {}
    issue_number = str(record.get("issue_number") or "")
    expected_url = f"https://github.com/{record.get('repository')}/issues/{issue_number}"
    if (str(item.get("id") or "") != item_id
            or str(item.get("_project_owner") or "") != owner
            or str(item.get("_project_number") or "") != str(number)
            or (expected_project_id and str(item.get("_project_id") or "") != expected_project_id)
            or str(content.get("number") or "") != issue_number
            or str(content.get("url") or "") != expected_url
            or not re.search(rf"^task_uid:\s*{re.escape(task_uid)}$", str(content.get("body") or ""), re.MULTILINE)):
        fail("bound Project item identity mismatch")
    return item_id, project_meta, item


def _ledger_transition(path: pathlib.Path, task_uid: str, effect: str,
                       state: str, result: object = None,
                       pr_head: dict[str, str] | None = None) -> None:
    operation_id = _operation_id(task_uid, effect, pr_head)

    def update(ledger: dict) -> None:
        if ledger and ledger.get("task_uid") not in (None, task_uid):
            fail("non-merge ledger task identity conflict")
        entry = (ledger.setdefault("operations", {})
                 .setdefault(effect, {"operation_id": operation_id, "effect": effect}))
        if entry.get("operation_id") != operation_id:
            legacy_operation_id = hashlib.sha256(
                f"{task_uid}:closed_without_merge:{effect}".encode()
            ).hexdigest()
            if (pr_head and entry.get("operation_id") == legacy_operation_id
                    and not entry.get("committed")):
                entry["operation_id"] = operation_id
            else:
                fail("non-merge ledger operation identity conflict")
        entry[state] = True
        if result is not None:
            entry["result"] = result
        ledger.update(schema="oasis7_non_merge_finalizer_ledger_v1", task_uid=task_uid)

    durable_store.transact_json(path, update, {})


def _operation_id(task_uid: str, effect: str,
                  pr_head: dict[str, str] | None = None) -> str:
    bound_head = {
        key: pr_head[key] for key in PR_HEAD_FIELDS
        if pr_head is not None and key in pr_head
    }
    return hashlib.sha256(
        json.dumps({
            "task_uid": task_uid,
            "phase": "closed_without_merge",
            "effect": effect,
            "pr_head": bound_head,
        }, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()

def _ledger_entry(path: pathlib.Path, effect: str) -> dict:
    return ((durable_store.recover_atomic_journal(path).get("operations") or {}).get(effect) or {})


def _validate_migrated_ledger(source: pathlib.Path, destination: pathlib.Path,
                              task_uid: str, pr_head: dict[str, str],
                              required_committed_effects: tuple[str, ...] = ()) -> None:
    source_bytes = source.read_bytes() if source.exists() else b""
    source_digest = hashlib.sha256(source_bytes).hexdigest()
    migrated = _read_json_object(destination, "migrated non-merge ledger")
    if migrated.get("schema") != "oasis7_non_merge_finalizer_ledger_v1":
        fail("migrated non-merge ledger schema is invalid")
    if migrated.get("task_uid") != task_uid:
        fail("migrated non-merge ledger task identity conflict")
    if migrated.get("migrated_from") != source.name:
        fail("migrated non-merge ledger source identity is invalid")
    if migrated.get("migrated_from_sha256") != source_digest:
        fail("migrated non-merge ledger source drifted")
    operations = migrated.get("operations")
    if not isinstance(operations, dict):
        fail("migrated non-merge ledger operations are missing or malformed")

    legacy_operations: dict = {}
    if source.exists():
        legacy = _read_json_object(source, "legacy non-merge ledger")
        if legacy and legacy.get("task_uid") not in (None, task_uid):
            fail("legacy non-merge ledger task identity conflict")
        legacy_operations = legacy.get("operations")
        if not isinstance(legacy_operations, dict):
            fail("legacy non-merge ledger operations are missing or malformed")
        if not set(legacy_operations).issubset(operations):
            fail("migrated non-merge ledger dropped legacy operations")

    for effect, entry in operations.items():
        if not isinstance(entry, dict):
            fail("migrated non-merge ledger operation is malformed")
        if entry.get("effect") not in (None, effect):
            fail("migrated non-merge ledger operation effect is invalid")
        if entry.get("operation_id") != _operation_id(task_uid, effect, pr_head):
            fail("migrated non-merge ledger operation identity conflict")
        for state in ("intent", "action", "committed"):
            if state in entry and entry[state] is not True:
                fail("migrated non-merge ledger state bit is invalid")
        aliases = entry.get("operation_id_aliases")
        if aliases is not None and (
                not isinstance(aliases, list)
                or any(not isinstance(alias, str) or not alias for alias in aliases)):
            fail("migrated non-merge ledger operation aliases are malformed")
        legacy_entry = legacy_operations.get(effect)
        old_operation_id = (legacy_entry.get("operation_id")
                             if isinstance(legacy_entry, dict) else None)
        migrated_alias = entry.get("migrated_from_operation_id")
        if isinstance(legacy_entry, dict):
            for state in ("intent", "action", "readback", "committed"):
                if state in legacy_entry and (
                        state not in entry or entry[state] != legacy_entry[state]):
                    fail("migrated non-merge ledger state drifted from source")
        if legacy_entry is not None and (
                not isinstance(legacy_entry, dict) or not isinstance(old_operation_id, str)
                or migrated_alias != old_operation_id):
            fail("migrated non-merge ledger operation alias is not source-bound")
        if legacy_entry is None and migrated_alias is not None:
            fail("migrated non-merge ledger operation alias has no source")
        if effect in required_committed_effects and entry.get("committed") is not True:
            fail("migrated non-merge ledger committed state is incomplete")


def _migrate_legacy_ledger(source: pathlib.Path, destination: pathlib.Path,
                            task_uid: str, pr_head: dict[str, str],
                            required_committed_effects: tuple[str, ...] = ()) -> None:
    source_bytes = source.read_bytes() if source.exists() else b""
    source_digest = hashlib.sha256(source_bytes).hexdigest()
    if destination.exists():
        _validate_migrated_ledger(
            source, destination, task_uid, pr_head, required_committed_effects,
        )
        return
    legacy = _read_json_object(source, "legacy non-merge ledger") if source.exists() else {}
    if legacy and legacy.get("task_uid") not in (None, task_uid):
        fail("legacy non-merge ledger task identity conflict")
    migrated = json.loads(json.dumps(legacy))
    operations = migrated.get("operations") or {}
    if not isinstance(operations, dict):
        fail("legacy non-merge ledger operations are malformed")
    for effect, entry in operations.items():
        if not isinstance(entry, dict):
            fail("legacy non-merge ledger operation is malformed")
        old_operation_id = entry.get("operation_id")
        if old_operation_id:
            entry["migrated_from_operation_id"] = old_operation_id
        entry["operation_id"] = _operation_id(task_uid, effect, pr_head)
    migrated.update({
        "schema": "oasis7_non_merge_finalizer_ledger_v1",
        "task_uid": task_uid,
        "operations": operations,
        "migrated_from": source.name,
        "migrated_from_sha256": source_digest,
    })
    _write_immutable_json(destination, migrated, "migrated non-merge ledger")
    _validate_migrated_ledger(source, destination, task_uid, pr_head)


def _reserve_closeout(path: pathlib.Path, task_uid: str, record: dict,
                      reason: str, evidence_digest: str,
                      project_identity: dict[str, str]) -> dict:
    intent = _closeout_intent(task_uid, record, reason, evidence_digest,
                              project_identity)

    def update(mapping: dict) -> dict:
        if _project_identity(mapping.get("project") or {}) != project_identity:
            fail("Project identity drifted before terminal effects")
        current = (mapping.get("tasks") or {}).get(task_uid) or {}
        if not current:
            fail(f"unknown task UID: {task_uid}")
        if str(current.get("workflow_phase") or "") == "closed_without_merge":
            return dict(current)
        for key in TERMINAL_CAS_FIELDS:
            # PR head fields are first learned from the live PR read and are
            # therefore absent from a pre-head mapping snapshot.  Reservation
            # binds those fields atomically; any already-recorded value must
            # still match exactly.
            if key in PR_HEAD_FIELDS and key not in current and key in record:
                continue
            if (key in current, current.get(key)) != (key in record, record.get(key)):
                fail(f"task authority drifted before terminal effects: {key}")
        existing = current.get(NON_MERGE_INTENT_FIELD)
        bound_receipt_digests: dict[str, str] = {}
        if existing is not None and existing != intent:
            if not isinstance(existing, dict):
                fail("non-merge closeout intent authority drifted")
            # A pre-head writer may have persisted a strict subset of the
            # current intent.  Validate every legacy field before upgrading;
            # unknown or disagreeing fields remain fail-closed.
            for key, value in existing.items():
                if key == "schema":
                    continue
                if (key in {"migrated_receipt_sha256", "canonical_receipt_sha256"}
                        and isinstance(value, str)
                        and re.fullmatch(r"[0-9a-f]{64}", value)
                        and value == ((record.get(NON_MERGE_INTENT_FIELD) or {})
                                      .get(key))):
                    bound_receipt_digests[key] = value
                    continue
                if key not in intent or intent[key] != value:
                    fail("non-merge closeout intent authority drifted")
        for key in PR_HEAD_FIELDS:
            if key in record:
                current[key] = record[key]
        reserved_intent = dict(intent)
        reserved_intent.update(bound_receipt_digests)
        current[NON_MERGE_INTENT_FIELD] = reserved_intent
        mapping.setdefault("tasks", {})[task_uid] = current
        return dict(current)

    return durable_store.transact_json(path, update)


def _verify_closeout_authority(path: pathlib.Path, task_uid: str, record: dict,
                               reason: str, evidence_digest: str,
                               project_identity: dict[str, str]) -> None:
    intent = _closeout_intent(task_uid, record, reason, evidence_digest,
                              project_identity)
    bound_digest = ((record.get(NON_MERGE_INTENT_FIELD) or {})
                    .get("migrated_receipt_sha256"))
    if bound_digest:
        intent["migrated_receipt_sha256"] = bound_digest
    canonical_digest = ((record.get(NON_MERGE_INTENT_FIELD) or {})
                        .get("canonical_receipt_sha256"))
    if canonical_digest:
        intent["canonical_receipt_sha256"] = canonical_digest

    def check(mapping: dict) -> None:
        if _project_identity(mapping.get("project") or {}) != project_identity:
            fail("Project identity drifted before terminal effects")
        current = (mapping.get("tasks") or {}).get(task_uid) or {}
        if not current:
            fail(f"unknown task UID: {task_uid}")
        if str(current.get("workflow_phase") or "") == "closed_without_merge":
            for key in TERMINAL_CAS_FIELDS:
                if (key in current, current.get(key)) != (key in record, record.get(key)):
                    fail(f"task authority drifted before terminal effects: {key}")
            if current.get("closed_without_merge_reason") != reason:
                fail("task terminal reason authority drifted")
            if current.get("closed_without_merge_evidence_sha256") != evidence_digest:
                fail("task terminal evidence authority drifted")
            expected_digest = ((record.get("phase_receipt_sha256") or {})
                               .get("closed_without_merge"))
            current_digest = ((current.get("phase_receipt_sha256") or {})
                              .get("closed_without_merge"))
            if expected_digest and current_digest != expected_digest:
                fail("task terminal migrated receipt digest authority drifted")
            return
        for key in TERMINAL_CAS_FIELDS:
            if (key in current, current.get(key)) != (key in record, record.get(key)):
                fail(f"task authority drifted before terminal effects: {key}")
        if not _intent_matches(current.get(NON_MERGE_INTENT_FIELD), intent):
            fail("non-merge closeout intent authority drifted")

    durable_store.transact_json(path, check)


def _bind_terminal_pr_head(path: pathlib.Path, task_uid: str,
                           project_identity: dict[str, str],
                           pr_head: dict[str, str], reason: str,
                           evidence_digest: str) -> None:
    """Bind a live PR head into an already-terminal legacy mapping.

    Legacy terminal mappings predate head authority.  This is the only
    additive mutation permitted for a migrated receipt; an existing value
    must match the live snapshot exactly.
    """
    if not pr_head:
        return

    def update(mapping: dict) -> None:
        if _project_identity(mapping.get("project") or {}) != project_identity:
            fail("Project identity drifted before terminal effects")
        current = (mapping.get("tasks") or {}).get(task_uid) or {}
        if not current:
            fail(f"unknown task UID: {task_uid}")
        if str(current.get("workflow_phase") or "") != "closed_without_merge":
            fail("legacy terminal mapping is no longer closed_without_merge")
        bindings = {
            "closed_without_merge_reason": reason,
            "closed_without_merge_evidence_sha256": evidence_digest,
            **pr_head,
        }
        for key, value in bindings.items():
            if key in current and str(current.get(key) or "") != value:
                fail(f"task PR {key} drifted from live head authority")
            current[key] = value
        mapping.setdefault("tasks", {})[task_uid] = current

    durable_store.transact_json(path, update)


def _verify_project_identity(path: pathlib.Path,
                             project_identity: dict[str, str]) -> None:
    def check(mapping: dict) -> None:
        if _project_identity(mapping.get("project") or {}) != project_identity:
            fail("Project identity drifted before terminal effects")

    durable_store.transact_json(path, check)


def _evidence_comment_body(record: dict, task_uid: str, operation_id: str,
                           reason: str, evidence_digest: str,
                           evidence_file: str, public_evidence: str) -> str:
    return (
        "<!-- oasis7-pm-evidence -->\n"
        f"Operation-ID: {operation_id}\nTask UID: {task_uid}\n"
        "Evidence Phase: closed_without_merge\nRole: tpm\n"
        f"Reason: {reason}\nEvidence SHA256: {evidence_digest}\n"
        f"Evidence File: {evidence_file}\n"
        f"Action: non-merge-finalize --task-uid {task_uid} --reason {reason}\n"
        f"Validation Command: gh issue view {record['issue_number']} -R {record['repository']} --json state,body,number,url\n"
        "Expected Result: receipt, Project terminal fields, Issue body, evidence comment, and Issue close read back.\n"
        "Actual Result: bound identity, evidence digest, and terminal Project fields read back before Issue close.\n"
        "Evidence Payload:\n```text\n"
        f"{public_evidence}\n```\n"
    )


def _comment_identity_body(body: str) -> str:
    """Ignore only filename and operation-id spelling; payload remains identity."""
    normalized = re.sub(r"(?m)^Evidence File:[^\n]*(?:\n)(?=Action:)", "", body, count=1)
    return re.sub(r"(?m)^Operation-ID:[^\n]*$", "Operation-ID: <bound>", normalized, count=1)


def _reconcile_comment(record: dict, operation_id: str, reason: str,
                       evidence_digest: str, evidence_file: str,
                       public_evidence: str,
                       operation_id_aliases: tuple[str, ...] = ()) -> str:
    raw = subprocess.check_output(
        ["gh", "api", f"repos/{record['repository']}/issues/{record['issue_number']}/comments",
         "--paginate", "--slurp"], text=True,
    )
    payload = json.loads(raw or "[]")
    comments = []
    for page in payload if isinstance(payload, list) else [payload]:
        comments.extend(page if isinstance(page, list) else [page])
    operation_ids = (operation_id, *operation_id_aliases)
    issue_marker = f"https://github.com/{record['repository']}/issues/{record['issue_number']}#issuecomment-"
    identity_matches = [
        comment for comment in comments
        if any(f"Operation-ID: {candidate}" in str(comment.get("body") or "")
               for candidate in operation_ids)
        and f"Task UID: {record['task_uid']}" in str(comment.get("body") or "")
    ]
    if not identity_matches:
        return ""
    canonical_matches = [
        comment for comment in identity_matches
        if "<!-- oasis7-pm-evidence -->" in str(comment.get("body") or "")
        and "Evidence Phase: closed_without_merge" in str(comment.get("body") or "")
        and issue_marker in str(comment.get("html_url") or comment.get("url") or "")
    ]
    expected_body = _evidence_comment_body(
        record, record["task_uid"], operation_id, reason, evidence_digest,
        evidence_file, public_evidence,
    )
    exact_matches = [
        comment for comment in canonical_matches
        if _comment_identity_body(str(comment.get("body") or ""))
        == _comment_identity_body(expected_body)
    ]
    if len(exact_matches) != 1:
        return ""
    comment = exact_matches[0]
    return str(comment.get("html_url") or comment.get("url") or "")


def _update_issue_body(record: dict, task_uid: str, issue: dict,
                       ledger_path: pathlib.Path,
                       pr_head: dict[str, str] | None = None) -> dict:
    body = str(issue.get("body") or "")
    statuses = STATUS_LINE_RE.findall(body)
    if len(statuses) != 1:
        fail("Issue body is missing exactly one canonical status field")
    updated_body, count = re.subn(
        STATUS_LINE_RE, "- status: `done`", body, count=1,
    )
    if count != 1:
        fail("Issue body is missing exactly one canonical status field")
    _ledger_transition(ledger_path, task_uid, "issue_body_update", "intent", pr_head=pr_head)
    if updated_body != body:
        _ledger_transition(ledger_path, task_uid, "issue_body_update", "action", pr_head=pr_head)
        with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as handle:
            handle.write(updated_body)
            body_path = pathlib.Path(handle.name)
        try:
            subprocess.run(
                ["gh", "issue", "edit", str(record["issue_number"]), "-R",
                 str(record["repository"]), "--body-file", str(body_path)],
                check=True, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            )
        finally:
            body_path.unlink(missing_ok=True)
    readback = _identity_issue(record, task_uid, allow_closed=True)
    readback_statuses = STATUS_LINE_RE.findall(str(readback.get("body") or ""))
    if readback_statuses != ["done"]:
        fail("Issue body status readback mismatch")
    _ledger_transition(ledger_path, task_uid, "issue_body_update", "readback", readback, pr_head)
    _ledger_transition(ledger_path, task_uid, "issue_body_update", "committed", pr_head=pr_head)
    return readback


def _project_update(root: pathlib.Path, record: dict, task_uid: str,
                    ledger_path: pathlib.Path, mapping_path: pathlib.Path,
                    reason: str, evidence_digest: str,
                    project_identity: dict[str, str],
                    pr_head: dict[str, str] | None = None) -> None:
    item_id, project_meta, live = _project_readback(record, task_uid)
    _verify_closeout_authority(mapping_path, task_uid, record, reason,
                               evidence_digest, project_identity)
    task = OrderedDict(record)
    task.update({"task_uid": task_uid, "status": "done", "workflow_phase": "closed_without_merge"})
    expected = project_sync.project_field_values(task)
    terminal_fields = {key: expected[key] for key in ("Status", "PM Status", "Workflow Phase")}
    fields_project_id, fields = project_sync.project_context(str(project_meta["owner"]), int(project_meta["number"]))
    if project_meta.get("id") and str(fields_project_id) != str(project_meta["id"]):
        fail("canonical Project id disagrees with live Project")
    for name, value in terminal_fields.items():
        field = fields.get(name)
        if not field or (name in project_sync.SINGLE_SELECT_FIELDS and value not in (field.get("options_by_name") or {})):
            fail(f"Project terminal field unavailable: {name}={value}")
    _verify_closeout_authority(mapping_path, task_uid, record, reason,
                               evidence_digest, project_identity)
    _ledger_transition(ledger_path, task_uid, "project_update", "intent", pr_head=pr_head)
    missing = {name for name, value in terminal_fields.items() if str(live.get(name) or "") != value}
    if missing:
        _verify_closeout_authority(mapping_path, task_uid, record, reason,
                                   evidence_digest, project_identity)
        _ledger_transition(ledger_path, task_uid, "project_update", "action", {"fields": sorted(missing)}, pr_head)
        updated, skipped = project_sync.update_fields(fields_project_id, item_id, task, fields, only_fields=missing)
        if skipped or int(updated) != len(missing):
            fail("Project terminal fields were not fully persisted")
        live = project_workflow.fetch_project_items_by_ids([item_id]).get(item_id) or {}
        if live.get("_field_values_has_next_page") is not False:
            fail("Project terminal readback pagination is incomplete")
    if any(str(live.get(name) or "") != value for name, value in terminal_fields.items()):
        fail("Project terminal fields readback mismatch")
    _ledger_transition(ledger_path, task_uid, "project_update", "readback", live, pr_head)
    _ledger_transition(ledger_path, task_uid, "project_update", "committed", pr_head=pr_head)


def _commit_mapping(path: pathlib.Path, task_uid: str, record: dict,
                    receipt: dict, receipt_digest: str, comment: str,
                    reason: str, evidence_digest: str,
                    project_identity: dict[str, str]) -> None:
    intent = _closeout_intent(task_uid, record, reason, evidence_digest,
                              project_identity)
    bound_digest = ((record.get(NON_MERGE_INTENT_FIELD) or {})
                    .get("migrated_receipt_sha256"))
    if bound_digest:
        intent["migrated_receipt_sha256"] = bound_digest
    canonical_digest = ((record.get(NON_MERGE_INTENT_FIELD) or {})
                        .get("canonical_receipt_sha256"))
    if canonical_digest:
        intent["canonical_receipt_sha256"] = canonical_digest

    def update(mapping: dict) -> None:
        if _project_identity(mapping.get("project") or {}) != project_identity:
            fail("Project identity drifted during terminal effects")
        current = (mapping.get("tasks") or {}).get(task_uid) or {}
        for key in TERMINAL_CAS_FIELDS:
            if (key in current, current.get(key)) != (key in record, record.get(key)):
                fail(f"task authority drifted during terminal effects: {key}")
        current_phase = str(current.get("workflow_phase") or "")
        if current_phase not in ELIGIBLE_NON_MERGE_PHASES:
            fail(f"workflow phase is not eligible for non-merge closeout: {current_phase}")
        if current_phase == "closed_without_merge":
            expected_digest = ((record.get("phase_receipt_sha256") or {})
                               .get("closed_without_merge"))
            current_digest = ((current.get("phase_receipt_sha256") or {})
                              .get("closed_without_merge"))
            if expected_digest and current_digest != expected_digest:
                fail("task terminal migrated receipt digest authority drifted")
        if (current_phase != "closed_without_merge"
                and not _intent_matches(current.get(NON_MERGE_INTENT_FIELD), intent)):
            fail("non-merge closeout intent authority drifted")
        current["status"] = "done"
        current["workflow_phase"] = "closed_without_merge"
        current.pop(NON_MERGE_INTENT_FIELD, None)
        current["closed_without_merge_reason"] = reason
        current["closed_without_merge_evidence_sha256"] = evidence_digest
        current["closed_without_merge_receipt"] = receipt
        current.setdefault("phase_receipts", {})["closed_without_merge"] = receipt
        current.setdefault("phase_receipt_sha256", {})["closed_without_merge"] = receipt_digest
        current.setdefault("evidence_comments", [])
        if comment and comment not in current["evidence_comments"]:
            current["evidence_comments"].append(comment)
        if not current.get("last_closed_at"):
            current["last_closed_at"] = now()
        if not current.get("non_merge_finalized_at"):
            current["non_merge_finalized_at"] = now()
        current["last_evidence_at"] = now()
        mapping.setdefault("tasks", {})[task_uid] = current

    durable_store.transact_json(path, update)


def _write_tombstone(receipt_path: pathlib.Path, record: dict, digest: str) -> None:
    tombstone = {
        "schema": "oasis7_terminal_tombstone_v1",
        "task_uid": record.get("task_uid"),
        "repository": record.get("repository"),
        "issue_number": record.get("issue_number"),
        "pr_number": record.get("pr_number"),
        "canonical_worktree": record.get("canonical_worktree"),
        "task_branch": record.get("task_branch"),
        "workflow_phase": "closed_without_merge",
        "terminal_receipt_sha256": digest,
        "checkout_recreation_forbidden": True,
    }
    durable_store.replace_json(receipt_path.with_name("terminal-tombstone.json"), tombstone)


def _finalize(root: pathlib.Path, task_uid: str, reason: str,
              evidence_path: pathlib.Path) -> dict:
    _validate_default_worktree(root)
    mapping_path, mapping, record = _mapping(root, task_uid)
    # Project metadata is canonical mapping-level truth; keep it out of the
    # persisted task record while making it available to the bound readback.
    record = dict(record)
    record["_project"] = mapping.get("project") or {}
    was_terminal = str(record.get("workflow_phase") or "") == "closed_without_merge"
    if str(record.get("workflow_phase") or "") not in ELIGIBLE_NON_MERGE_PHASES:
        fail(f"workflow phase is not eligible for non-merge closeout: {record.get('workflow_phase')}")
    project_identity = _project_identity(record["_project"])
    if not all(project_identity.values()):
        fail("task truth is missing complete canonical Project identity")
    if str(record.get("task_uid") or task_uid) != task_uid:
        fail("task UID mapping identity mismatch")
    if str(record.get("workflow_phase") or "") == "post_merge_done":
        fail("post_merge_done requires merged-PR finalization")
    if _has_authority(record, MERGE_AUTHORITY_FIELDS):
        fail("non-merge closeout cannot carry merge receipt authority")
    if reason == "non_pr_completed" and _has_authority(record, NON_PR_FORBIDDEN_AUTHORITY):
        fail("non_pr_completed cannot carry PR or merge receipt authority")
    evidence_digest, evidence_data, public_evidence = _read_evidence(evidence_path)
    try:
        (canonical_receipt_path, receipt_path, existing_receipt,
         canonical_receipt_bytes) = resolve_non_merge_receipt(root, task_uid)
    except ValueError as exc:
        fail(str(exc))
    canonical_ledger_path = canonical_receipt_path.with_name(LEDGER_NAME)
    ledger_path = canonical_ledger_path
    receipt_needs_write = False
    migrated_receipt_path = _migrated_receipt_path(canonical_receipt_path)
    if receipt_path == migrated_receipt_path:
        ledger_path = _migrated_ledger_path(canonical_ledger_path)
    historical_pr_head = {
        key: str(record.get(key) or "") for key in PR_HEAD_REQUIRED_FIELDS
        if record.get(key)
    }
    pr = _identity_pr(record)
    pr_head = _bind_pr_head(record, pr, existing_receipt)
    recovery_candidate = _receipt_recovery_candidate(
        record, task_uid, reason, evidence_digest, existing_receipt,
        evidence_data=evidence_data, pr=pr, project_identity=project_identity,
        pr_head=pr_head,
    )
    if (str(record.get("workflow_phase") or "") == "closed_without_merge"
            and not recovery_candidate):
        fail("closed_without_merge requires a matching non-merge receipt")
    if reason == "non_pr_completed" and not recovery_candidate and not (
            record.get("status") == "done"
            and record.get("workflow_phase") == "task_done"
            and record.get("last_closed_at")
            and _has_verified_task_complete(record)
            and _has_non_pr_task_classification(record)):
        fail("non_pr_completed requires task_done, verified task_complete closeout evidence, and non_pr_task classification/evidence")
    if reason in {"superseded", "duplicate"} and pr is None:
        fail(f"{reason} requires an exact bound CLOSED unmerged PR")
    # Any legacy receipt missing a modern authority field is upgraded into an
    # immutable sidecar, regardless of whether the mapping has reached its
    # terminal phase yet.  The validator below still checks every immutable
    # identity field before this path is allowed to proceed.
    legacy_receipt_migration = (
        existing_receipt is not None
        and receipt_path == canonical_receipt_path
        and _receipt_needs_migration(existing_receipt, pr_head)
    )
    if legacy_receipt_migration:
        receipt_has_head = all(
            existing_receipt.get(key) == pr_head.get(key)
            for key in PR_HEAD_REQUIRED_FIELDS
        )
        mapping_has_head = all(
            historical_pr_head.get(key) == pr_head.get(key)
            for key in PR_HEAD_REQUIRED_FIELDS
        )
        if pr_head and not (receipt_has_head or mapping_has_head):
            fail("PR-bound legacy non-merge receipt lacks historical PR head authority")
        receipt_path = migrated_receipt_path
        ledger_path = _migrated_ledger_path(canonical_ledger_path)
    issue = _identity_issue(record, task_uid, allow_closed=existing_receipt is not None)
    if (str(issue.get("state") or "").upper() == "CLOSED"
            and str(issue.get("stateReason") or "").upper()
            != _expected_issue_state_reason(reason)):
        fail("Issue close stateReason disagrees before terminal effects")
    receipt = {
        "receipt_type": "oasis7_closed_without_merge",
        "schema_version": NON_MERGE_RECEIPT_SCHEMA_VERSION,
        "issuer": "non-merge-finalize",
        "task_uid": task_uid,
        "repository": record.get("repository"),
        "issue_number": record.get("issue_number"),
        "project_item_id": record.get("project_item_id"),
        "project_identity": project_identity,
        "reason": reason,
        "evidence_sha256": evidence_digest,
        "evidence": evidence_data,
        "previous_status": (
            existing_receipt.get("previous_status")
            if was_terminal and existing_receipt is not None
            else record.get("status")
        ),
        "previous_workflow_phase": (
            existing_receipt.get("previous_workflow_phase")
            if was_terminal and existing_receipt is not None
            else record.get("workflow_phase") or ""
        ),
        "pr_number": record.get("pr_number"),
        "pr_url": record.get("pr_url"),
        "pr_state": (pr or {}).get("state") if pr else None,
        "mergedAt": (pr or {}).get("mergedAt") if pr else None,
    }
    receipt.update(pr_head)
    if existing_receipt is not None:
        legacy_source = (
            _read_json_object(canonical_receipt_path, "legacy non-merge receipt")
            if receipt_path == migrated_receipt_path else None
        )
        _validate_existing_receipt(
            existing_receipt, receipt, evidence_data, project_identity, pr_head,
            require_modern_authority=(
                not legacy_receipt_migration
                and (receipt_path == migrated_receipt_path
                     or not _receipt_needs_migration(existing_receipt, pr_head))
            ),
            legacy_source=legacy_source,
        )
        if not legacy_receipt_migration:
            receipt = existing_receipt
    if legacy_receipt_migration:
        # Preserve every legacy lifecycle/history field and only fill fields
        # absent from the legacy payload with current authority.  In
        # particular, retain the original text evidence representation.
        migrated_receipt = dict(receipt)
        migrated_receipt.update(existing_receipt)
        receipt = migrated_receipt
        receipt.update({
            "migrated_from": RECEIPT_NAME,
            "migrated_from_sha256": hashlib.sha256(canonical_receipt_bytes).hexdigest(),
        })
        if "observed_at" not in existing_receipt:
            receipt.pop("observed_at", None)
    # Complete the live Project binding readback before persisting either
    # terminal intent or receipt authority.  This keeps a missing item or
    # malformed binding from leaving a resumable-looking local closeout.
    _project_readback(record, task_uid)
    if recovery_candidate:
        _bind_terminal_pr_head(
            mapping_path, task_uid, project_identity, pr_head, reason, evidence_digest,
        )
        record.update({
            "closed_without_merge_reason": reason,
            "closed_without_merge_evidence_sha256": evidence_digest,
        })
        _verify_project_identity(mapping_path, project_identity)
    else:
        _reserve_closeout(mapping_path, task_uid, record, reason,
                          evidence_digest, project_identity)
    if legacy_receipt_migration:
        _write_immutable_json(receipt_path, receipt, "migrated non-merge receipt")
        receipt_digest = hashlib.sha256(receipt_path.read_bytes()).hexdigest()
        if was_terminal:
            _bind_terminal_migrated_receipt_digest(
                mapping_path, task_uid, receipt_digest,
            )
            record.setdefault("phase_receipt_sha256", {})[
                "closed_without_merge"
            ] = receipt_digest
        else:
            _bind_migrated_receipt_digest(mapping_path, task_uid, receipt_digest)
            record.setdefault(NON_MERGE_INTENT_FIELD, {})[
                "migrated_receipt_sha256"
            ] = receipt_digest
    elif receipt_path == migrated_receipt_path:
        receipt_digest = hashlib.sha256(receipt_path.read_bytes()).hexdigest()
        if was_terminal:
            bound = ((record.get("phase_receipt_sha256") or {})
                     .get("closed_without_merge"))
        else:
            intent = record.get(NON_MERGE_INTENT_FIELD) or {}
            bound = intent.get("migrated_receipt_sha256")
        if bound is None and was_terminal:
            _bind_terminal_migrated_receipt_digest(
                mapping_path, task_uid, receipt_digest,
            )
            record.setdefault("phase_receipt_sha256", {})[
                "closed_without_merge"
            ] = receipt_digest
        elif bound is None:
            _bind_migrated_receipt_digest(mapping_path, task_uid, receipt_digest)
            record.setdefault(NON_MERGE_INTENT_FIELD, {})[
                "migrated_receipt_sha256"
            ] = receipt_digest
        elif bound != receipt_digest:
            fail("migrated non-merge receipt lacks matching immutable digest authority")
    if receipt_path == migrated_receipt_path:
        required = (
            ("issue_body_update", "project_update", "evidence_comment")
            if record.get("closed_without_merge_receipt") == receipt else ()
        )
        _migrate_legacy_ledger(
            canonical_ledger_path, ledger_path, task_uid, pr_head,
            required_committed_effects=required,
        )
    elif existing_receipt is None or receipt_needs_write:
        durable_store.replace_json(receipt_path, receipt)
    if receipt_path == canonical_receipt_path:
        receipt_digest = hashlib.sha256(receipt_path.read_bytes()).hexdigest()
        bound = (((record.get("phase_receipt_sha256") or {})
                  .get("closed_without_merge")) if was_terminal else
                 ((record.get(NON_MERGE_INTENT_FIELD) or {})
                  .get("canonical_receipt_sha256")))
        if bound is None and was_terminal:
            _bind_terminal_migrated_receipt_digest(
                mapping_path, task_uid, receipt_digest,
            )
            record.setdefault("phase_receipt_sha256", {})[
                "closed_without_merge"
            ] = receipt_digest
        elif bound is None:
            _bind_canonical_receipt_digest(mapping_path, task_uid, receipt_digest)
            record.setdefault(NON_MERGE_INTENT_FIELD, {})[
                "canonical_receipt_sha256"
            ] = receipt_digest
        elif bound != receipt_digest:
            fail("canonical non-merge receipt lacks matching immutable digest authority")
    receipt_digest = hashlib.sha256(receipt_path.read_bytes()).hexdigest()
    # Read the bound comment stream while the task is still pre-terminal.  A
    # deterministic mapping drift exposed by this readback is rejected before
    # publishing Issue, Project, or comment terminal effects.
    comment_operation_id = _operation_id(task_uid, "evidence_comment", pr_head)
    _reconcile_comment(
        record, comment_operation_id, reason, evidence_digest,
        evidence_path.name, public_evidence,
    )
    # The reservation (or already-terminal receipt) is checked again before
    # the first remote terminal mutation.
    _verify_closeout_authority(mapping_path, task_uid, record, reason,
                               evidence_digest, project_identity)
    issue = _update_issue_body(record, task_uid, issue, ledger_path, pr_head)
    _verify_closeout_authority(mapping_path, task_uid, record, reason,
                               evidence_digest, project_identity)
    _project_update(root, record, task_uid, ledger_path, mapping_path,
                    reason, evidence_digest, project_identity, pr_head)
    _verify_closeout_authority(mapping_path, task_uid, record, reason,
                               evidence_digest, project_identity)
    comment_entry = _ledger_entry(ledger_path, "evidence_comment")
    comment_operation_aliases = tuple(
        alias for alias in (
            comment_entry.get("migrated_from_operation_id"),
            *(comment_entry.get("operation_id_aliases") or []
              if isinstance(comment_entry.get("operation_id_aliases"), list) else ()),
        )
        if isinstance(alias, str) and alias and alias != comment_operation_id
    )
    comment = ""
    if comment_entry.get("committed"):
        comment = _reconcile_comment(
            record, comment_operation_id, reason, evidence_digest,
            evidence_path.name, public_evidence, comment_operation_aliases,
        )
        if not comment:
            fail("committed evidence comment live readback disagrees with current reason/evidence")
    elif comment_entry.get("action"):
        comment = _reconcile_comment(
            record, comment_operation_id, reason, evidence_digest,
            evidence_path.name, public_evidence, comment_operation_aliases,
        )
    if not comment:
        _ledger_transition(ledger_path, task_uid, "evidence_comment", "intent", pr_head=pr_head)
        _ledger_transition(ledger_path, task_uid, "evidence_comment", "action", pr_head=pr_head)
        body = _evidence_comment_body(
            record, task_uid, comment_operation_id, reason, evidence_digest,
            evidence_path.name, public_evidence,
        )
        with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as handle:
            handle.write(body)
            body_path = pathlib.Path(handle.name)
        try:
            _verify_closeout_authority(mapping_path, task_uid, record, reason,
                                       evidence_digest, project_identity)
            subprocess.run(["gh", "issue", "comment", str(record["issue_number"]),
                            "-R", str(record["repository"]), "--body-file", str(body_path)],
                           check=True, text=True, stdout=subprocess.PIPE,
                           stderr=subprocess.PIPE)
        finally:
            body_path.unlink(missing_ok=True)
        comment = _reconcile_comment(
            record, comment_operation_id, reason, evidence_digest,
            evidence_path.name, public_evidence, comment_operation_aliases,
        )
        if not comment:
            fail("evidence comment live readback has no unique matching operation")
    _ledger_transition(ledger_path, task_uid, "evidence_comment", "readback", comment, pr_head)
    _ledger_transition(ledger_path, task_uid, "evidence_comment", "committed", pr_head=pr_head)
    _verify_receipt_bytes(receipt_path, receipt_digest)
    if pr is not None:
        _bind_pr_head(record, _identity_pr(record), receipt)
    _commit_mapping(mapping_path, task_uid, record, receipt, receipt_digest, comment,
                    reason, evidence_digest, project_identity)
    # The mapping CAS has now published the terminal record; subsequent
    # pre-close checks must compare against that committed snapshot.
    record.update({
        "status": "done",
        "workflow_phase": "closed_without_merge",
        "closed_without_merge_reason": reason,
        "closed_without_merge_evidence_sha256": evidence_digest,
    })
    record.setdefault("phase_receipt_sha256", {})[
        "closed_without_merge"
    ] = receipt_digest
    _ledger_transition(ledger_path, task_uid, "issue_close", "intent", pr_head=pr_head)
    _verify_receipt_bytes(receipt_path, receipt_digest)
    issue = run_json(["gh", "issue", "view", str(record["issue_number"]), "-R",
                      str(record["repository"]), "--json", "state,stateReason,body,number,url"])
    if str(issue.get("state") or "").upper() != "CLOSED":
        _verify_closeout_authority(mapping_path, task_uid, record, reason,
                                   evidence_digest, project_identity)
        _ledger_transition(ledger_path, task_uid, "issue_close", "action", pr_head=pr_head)
        close_reason = "completed" if reason == "non_pr_completed" else "not planned"
        subprocess.run(["gh", "issue", "close", str(record["issue_number"]),
                        "-R", str(record["repository"]), "--reason", close_reason],
                       check=True, text=True, stdout=subprocess.PIPE,
                       stderr=subprocess.PIPE)
        issue = run_json(["gh", "issue", "view", str(record["issue_number"]), "-R",
                          str(record["repository"]), "--json", "state,stateReason,body,number,url"])
    _ledger_transition(ledger_path, task_uid, "issue_close", "readback", issue, pr_head)
    if str(issue.get("state") or "").upper() != "CLOSED":
        fail("Issue close live readback mismatch")
    expected_state_reason = _expected_issue_state_reason(reason)
    if str(issue.get("stateReason") or "").upper() != expected_state_reason:
        fail("Issue close stateReason readback mismatch")
    _ledger_transition(ledger_path, task_uid, "issue_close", "committed", pr_head=pr_head)
    _verify_receipt_bytes(receipt_path, receipt_digest)
    _write_tombstone(receipt_path, record, receipt_digest)
    return {"status": "already_finalized" if was_terminal else "finalized",
            "task_uid": task_uid, "reason": reason, "receipt": str(receipt_path),
            "ledger": str(ledger_path)}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Receipt-bound non-merge task terminal finalizer")
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--task-uid", required=True)
    parser.add_argument("--reason", required=True, choices=REASONS)
    parser.add_argument("--evidence-file", required=True)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    root = pathlib.Path(args.repo_root).resolve()
    mapping_path = root / ".pm/github-project-sync/tasks.json"
    lock_path = mapping_path.with_name(f"{mapping_path.name}.{args.task_uid}.finalizer-lock")
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    with lock_path.open("a+b") as lock:
        ensure_lock_byte(lock)
        fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
        result = _finalize(root, args.task_uid, args.reason, pathlib.Path(args.evidence_file).resolve())
    print(json.dumps(result, indent=2, sort_keys=True) if args.json else
          f"non-merge-finalize: {result['status']} {args.task_uid} ({args.reason})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
