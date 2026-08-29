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
TERMINAL_CAS_FIELDS = (
    "task_uid", "repository", "issue_number", "issue_url", "project_item_id",
    "canonical_worktree", "task_branch", "default_branch", "status",
    "workflow_phase", "pr_number", "pr_url", "completion_mode",
    "non_pr_completion_evidence", *MERGE_AUTHORITY_FIELDS,
)
PROJECT_IDENTITY_FIELDS = ("owner", "number", "id")
RECEIPT_NAME = "closed-without-merge-receipt.json"
LEDGER_NAME = "non-merge-finalizer-ledger.json"


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


def _receipt_path(root: pathlib.Path, task_uid: str) -> pathlib.Path:
    result = subprocess.run(
        [sys.executable, str(CANONICAL_ROOT_HELPER), "--default-worktree", str(root),
         "--task-uid", task_uid, "--create", "--name", RECEIPT_NAME],
        text=True, capture_output=True,
    )
    if result.returncode:
        fail(result.stderr.strip() or "noncanonical receipt root")
    return pathlib.Path(result.stdout.strip()).resolve()


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
        parsed = {"text": raw.decode("utf-8", errors="replace")}
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
                      "--json", "state,body,number,url"])
    body = str(issue.get("body") or "")
    expected_url = f"https://github.com/{repository}/issues/{issue_number}"
    if (str(issue.get("number") or "") != issue_number
            or str(issue.get("url") or "") != str(record.get("issue_url") or expected_url)
            or not re.search(rf"^task_uid:\s*{re.escape(task_uid)}$", body, re.MULTILINE)):
        fail("Issue identity mismatch")
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
                   "--json", "number,url,state,mergedAt,headRefOid,headRefName"])
    merged_at = pr.get("mergedAt")
    if merged_at not in (None, ""):
        fail("recorded PR is MERGED; use post-merge-finalize with its merge receipt")
    if str(pr.get("state") or "").upper() != "CLOSED":
        fail("recorded PR must be CLOSED and unmerged before non-merge finalization")
    if (str(pr.get("number") or "") != pr_number
            or str(pr.get("url") or "") != pr_url):
        fail("recorded PR identity mismatch")
    return pr


def _project_identity(project: dict) -> dict[str, str]:
    """Return the canonical mapping-level Project identity for CAS/receipts."""
    return {key: str(project.get(key) or "") for key in PROJECT_IDENTITY_FIELDS}


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
                       state: str, result: object = None) -> None:
    operation_id = hashlib.sha256(
        f"{task_uid}:closed_without_merge:{effect}".encode()
    ).hexdigest()

    def update(ledger: dict) -> None:
        if ledger and ledger.get("task_uid") not in (None, task_uid):
            fail("non-merge ledger task identity conflict")
        entry = (ledger.setdefault("operations", {})
                 .setdefault(effect, {"operation_id": operation_id, "effect": effect}))
        if entry.get("operation_id") != operation_id:
            fail("non-merge ledger operation identity conflict")
        entry[state] = True
        if result is not None:
            entry["result"] = result
        ledger.update(schema="oasis7_non_merge_finalizer_ledger_v1", task_uid=task_uid)

    durable_store.transact_json(path, update, {})


def _ledger_entry(path: pathlib.Path, effect: str) -> dict:
    return ((durable_store.recover_atomic_journal(path).get("operations") or {}).get(effect) or {})


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


def _reconcile_comment(record: dict, operation_id: str, reason: str,
                       evidence_digest: str, evidence_file: str,
                       public_evidence: str) -> str:
    raw = subprocess.check_output(
        ["gh", "api", f"repos/{record['repository']}/issues/{record['issue_number']}/comments",
         "--paginate", "--slurp"], text=True,
    )
    payload = json.loads(raw or "[]")
    comments = []
    for page in payload if isinstance(payload, list) else [payload]:
        comments.extend(page if isinstance(page, list) else [page])
    marker = f"Operation-ID: {operation_id}"
    issue_marker = f"https://github.com/{record['repository']}/issues/{record['issue_number']}#issuecomment-"
    identity_matches = [
        comment for comment in comments
        if marker in str(comment.get("body") or "")
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
        if str(comment.get("body") or "") == expected_body
    ]
    if len(exact_matches) != 1:
        return ""
    comment = exact_matches[0]
    return str(comment.get("html_url") or comment.get("url") or "")


def _update_issue_body(record: dict, task_uid: str, issue: dict,
                       ledger_path: pathlib.Path) -> dict:
    body = str(issue.get("body") or "")
    statuses = STATUS_LINE_RE.findall(body)
    if len(statuses) != 1:
        fail("Issue body is missing exactly one canonical status field")
    updated_body, count = re.subn(
        STATUS_LINE_RE, "- status: `done`", body, count=1,
    )
    if count != 1:
        fail("Issue body is missing exactly one canonical status field")
    _ledger_transition(ledger_path, task_uid, "issue_body_update", "intent")
    if updated_body != body:
        _ledger_transition(ledger_path, task_uid, "issue_body_update", "action")
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
    _ledger_transition(ledger_path, task_uid, "issue_body_update", "readback", readback)
    _ledger_transition(ledger_path, task_uid, "issue_body_update", "committed")
    return readback


def _project_update(root: pathlib.Path, record: dict, task_uid: str,
                    ledger_path: pathlib.Path) -> None:
    item_id, project_meta, live = _project_readback(record, task_uid)
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
    _ledger_transition(ledger_path, task_uid, "project_update", "intent")
    missing = {name for name, value in terminal_fields.items() if str(live.get(name) or "") != value}
    if missing:
        _ledger_transition(ledger_path, task_uid, "project_update", "action", {"fields": sorted(missing)})
        updated, skipped = project_sync.update_fields(fields_project_id, item_id, task, fields, only_fields=missing)
        if skipped or int(updated) != len(missing):
            fail("Project terminal fields were not fully persisted")
        live = project_workflow.fetch_project_items_by_ids([item_id]).get(item_id) or {}
        if live.get("_field_values_has_next_page") is not False:
            fail("Project terminal readback pagination is incomplete")
    if any(str(live.get(name) or "") != value for name, value in terminal_fields.items()):
        fail("Project terminal fields readback mismatch")
    _ledger_transition(ledger_path, task_uid, "project_update", "readback", live)
    _ledger_transition(ledger_path, task_uid, "project_update", "committed")


def _commit_mapping(path: pathlib.Path, task_uid: str, record: dict,
                    receipt: dict, receipt_digest: str, comment: str,
                    reason: str, evidence_digest: str,
                    project_identity: dict[str, str]) -> None:
    def update(mapping: dict) -> None:
        if _project_identity(mapping.get("project") or {}) != project_identity:
            fail("Project identity drifted during terminal effects")
        current = (mapping.get("tasks") or {}).get(task_uid) or {}
        for key in TERMINAL_CAS_FIELDS:
            if (key in current, current.get(key)) != (key in record, record.get(key)):
                fail(f"task authority drifted during terminal effects: {key}")
        current_phase = str(current.get("workflow_phase") or "")
        eligible_phases = {
            "", "bootstrap", "planning", "execution", "verification",
            "pre_pr_review", "pre_pr_ready", "pr_watch", "blocked",
            "task_done", "closed_without_merge",
        }
        if current_phase not in eligible_phases:
            fail(f"workflow phase is not eligible for non-merge closeout: {current_phase}")
        current["status"] = "done"
        current["workflow_phase"] = "closed_without_merge"
        current["closed_without_merge_reason"] = reason
        current["closed_without_merge_evidence_sha256"] = evidence_digest
        current["closed_without_merge_receipt"] = receipt
        current.setdefault("phase_receipts", {})["closed_without_merge"] = receipt
        current.setdefault("phase_receipt_sha256", {})["closed_without_merge"] = receipt_digest
        current.setdefault("evidence_comments", [])
        if comment and comment not in current["evidence_comments"]:
            current["evidence_comments"].append(comment)
        current["last_closed_at"] = now()
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
    mapping_path, mapping, record = _mapping(root, task_uid)
    # Project metadata is canonical mapping-level truth; keep it out of the
    # persisted task record while making it available to the bound readback.
    record = dict(record)
    record["_project"] = mapping.get("project") or {}
    project_identity = _project_identity(record["_project"])
    if str(record.get("task_uid") or task_uid) != task_uid:
        fail("task UID mapping identity mismatch")
    if str(record.get("workflow_phase") or "") == "post_merge_done":
        fail("post_merge_done requires merged-PR finalization")
    if _has_authority(record, MERGE_AUTHORITY_FIELDS):
        fail("non-merge closeout cannot carry merge receipt authority")
    if reason == "non_pr_completed" and _has_authority(record, NON_PR_FORBIDDEN_AUTHORITY):
        fail("non_pr_completed cannot carry PR or merge receipt authority")
    if reason == "non_pr_completed" and not (
            record.get("status") == "done"
            and record.get("workflow_phase") == "task_done"
            and record.get("last_closed_at")
            and _has_verified_task_complete(record)
            and _has_non_pr_task_classification(record)):
        fail("non_pr_completed requires task_done, verified task_complete closeout evidence, and non_pr_task classification/evidence")
    evidence_digest, evidence_data, public_evidence = _read_evidence(evidence_path)
    pr = _identity_pr(record)
    if reason in {"superseded", "duplicate"} and pr is None:
        fail(f"{reason} requires an exact bound CLOSED unmerged PR")
    receipt_path = _receipt_path(root, task_uid)
    ledger_path = receipt_path.with_name(LEDGER_NAME)
    existing_receipt = None
    if receipt_path.exists():
        try:
            existing_receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            fail(f"existing non-merge receipt is unreadable: {exc}")
        if not isinstance(existing_receipt, dict):
            fail("existing non-merge receipt is not an object")
    issue = _identity_issue(record, task_uid, allow_closed=existing_receipt is not None)
    receipt = {
        "receipt_type": "oasis7_closed_without_merge",
        "schema_version": 1,
        "issuer": "non-merge-finalize",
        "task_uid": task_uid,
        "repository": record.get("repository"),
        "issue_number": record.get("issue_number"),
        "project_item_id": record.get("project_item_id"),
        "project_identity": project_identity,
        "reason": reason,
        "evidence_sha256": evidence_digest,
        "evidence": evidence_data,
        "previous_status": record.get("status"),
        "previous_workflow_phase": record.get("workflow_phase") or "",
        "pr_number": record.get("pr_number"),
        "pr_url": record.get("pr_url"),
        "pr_state": (pr or {}).get("state") if pr else None,
        "mergedAt": (pr or {}).get("mergedAt") if pr else None,
        "observed_at": now(),
    }
    if existing_receipt is not None:
        expected_existing = {
            "receipt_type": receipt["receipt_type"], "schema_version": receipt["schema_version"],
            "issuer": receipt["issuer"], "task_uid": task_uid,
            "repository": record.get("repository"), "issue_number": record.get("issue_number"),
            "project_item_id": record.get("project_item_id"), "reason": reason,
            "evidence_sha256": evidence_digest,
            "pr_number": receipt.get("pr_number"), "pr_url": receipt.get("pr_url"),
            "pr_state": receipt.get("pr_state"), "mergedAt": receipt.get("mergedAt"),
        }
        if any(existing_receipt.get(key) != value for key, value in expected_existing.items()):
            fail("existing non-merge receipt disagrees with current task/reason/evidence authority")
        stored_project_identity = existing_receipt.get("project_identity")
        if stored_project_identity is not None and stored_project_identity != project_identity:
            fail("existing non-merge receipt disagrees with current Project identity authority")
        receipt = existing_receipt
        if stored_project_identity is None:
            # Receipts written before Project identity was bound remain
            # retryable, but are upgraded before their digest is used as
            # terminal authority.
            receipt["project_identity"] = project_identity
            durable_store.replace_json(receipt_path, receipt)
    else:
        durable_store.replace_json(receipt_path, receipt)
    receipt_digest = hashlib.sha256(receipt_path.read_bytes()).hexdigest()
    issue = _update_issue_body(record, task_uid, issue, ledger_path)
    _project_update(root, record, task_uid, ledger_path)
    comment_operation_id = hashlib.sha256(
        f"{task_uid}:closed_without_merge:evidence_comment".encode()
    ).hexdigest()
    comment_entry = _ledger_entry(ledger_path, "evidence_comment")
    comment = ""
    if comment_entry.get("committed"):
        comment = _reconcile_comment(
            record, comment_operation_id, reason, evidence_digest,
            evidence_path.name, public_evidence,
        )
        if not comment:
            fail("committed evidence comment live readback disagrees with current reason/evidence")
    elif comment_entry.get("action"):
        comment = _reconcile_comment(
            record, comment_operation_id, reason, evidence_digest,
            evidence_path.name, public_evidence,
        )
    if not comment:
        _ledger_transition(ledger_path, task_uid, "evidence_comment", "intent")
        _ledger_transition(ledger_path, task_uid, "evidence_comment", "action")
        body = _evidence_comment_body(
            record, task_uid, comment_operation_id, reason, evidence_digest,
            evidence_path.name, public_evidence,
        )
        with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as handle:
            handle.write(body)
            body_path = pathlib.Path(handle.name)
        try:
            subprocess.run(["gh", "issue", "comment", str(record["issue_number"]),
                            "-R", str(record["repository"]), "--body-file", str(body_path)],
                           check=True, text=True, stdout=subprocess.PIPE,
                           stderr=subprocess.PIPE)
        finally:
            body_path.unlink(missing_ok=True)
        comment = _reconcile_comment(
            record, comment_operation_id, reason, evidence_digest,
            evidence_path.name, public_evidence,
        )
        if not comment:
            fail("evidence comment live readback has no unique matching operation")
    _ledger_transition(ledger_path, task_uid, "evidence_comment", "readback", comment)
    _ledger_transition(ledger_path, task_uid, "evidence_comment", "committed")
    _commit_mapping(mapping_path, task_uid, record, receipt, receipt_digest, comment,
                    reason, evidence_digest, project_identity)
    _ledger_transition(ledger_path, task_uid, "issue_close", "intent")
    issue = run_json(["gh", "issue", "view", str(record["issue_number"]), "-R",
                      str(record["repository"]), "--json", "state,body,number,url"])
    if str(issue.get("state") or "").upper() != "CLOSED":
        _ledger_transition(ledger_path, task_uid, "issue_close", "action")
        close_reason = "completed" if reason == "non_pr_completed" else "not planned"
        subprocess.run(["gh", "issue", "close", str(record["issue_number"]),
                        "-R", str(record["repository"]), "--reason", close_reason],
                       check=True, text=True, stdout=subprocess.PIPE,
                       stderr=subprocess.PIPE)
        issue = run_json(["gh", "issue", "view", str(record["issue_number"]), "-R",
                          str(record["repository"]), "--json", "state,body,number,url"])
    _ledger_transition(ledger_path, task_uid, "issue_close", "readback", issue)
    if str(issue.get("state") or "").upper() != "CLOSED":
        fail("Issue close live readback mismatch")
    _ledger_transition(ledger_path, task_uid, "issue_close", "committed")
    _write_tombstone(receipt_path, record, receipt_digest)
    return {"status": "already_finalized" if str(record.get("workflow_phase") or "") == "closed_without_merge" else "finalized",
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
