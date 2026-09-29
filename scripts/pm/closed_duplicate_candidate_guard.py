#!/usr/bin/env python3
"""Read-only pre-admission guard for candidate GitHub task identities.

Candidate bootstrap is an admission boundary even when the cache has no
duplicate path/branch alias: the local task mapping is only a cache, so the
candidate's exact Issue must still be live and uniquely bind the same Task UID
before a snapshot can be created or reused. Global sync also checks complete
live Issue/Project authority before acting on a candidate whose mapping row is
absent or incomplete. Active non-candidate tasks are unchanged. All GitHub
calls are read-only; retirement/apply is never called.
"""
from __future__ import annotations

import importlib.util
import json
import pathlib
import re
import shlex
import subprocess
from typing import Any


GITHUB_REPO_RE = re.compile(r"^[^/\s]+/[^/\s]+$")
TASK_UID_RE = re.compile(r"^task_[0-9a-f]{32}$")
CANONICAL_UID_LINE_RE = re.compile(r"(?m)^task_uid:\s*(.*?)\s*$")
CANONICAL_UID_FIELD_RE = re.compile(r"(?m)^task_uid:[ \t]*(.*?)[ \t]*\r?$")
_RETIREMENT_COLLECTOR: Any | None = None


class CandidateAdmissionError(ValueError):
    """Candidate truth could not safely pass read-only admission."""

    def __init__(self, message: str, reconcile_command: list[str] | None = None):
        super().__init__(message)
        self.reconcile_command = reconcile_command


def _candidate(task: dict[str, Any]) -> bool:
    return (
        str(task.get("status") or "") == "candidate"
        and str(task.get("workflow_phase") or "bootstrap") in {"", "bootstrap"}
    )


def _issue_number(value: Any) -> int:
    try:
        number = int(value)
    except (TypeError, ValueError) as exc:
        raise CandidateAdmissionError("candidate live Issue identity is missing or malformed") from exc
    if number <= 0:
        raise CandidateAdmissionError("candidate live Issue identity is missing or malformed")
    return number


def _retirement_collector() -> Any:
    """Load the repository-owned complete read collectors used by retirement."""
    global _RETIREMENT_COLLECTOR
    if _RETIREMENT_COLLECTOR is not None:
        return _RETIREMENT_COLLECTOR
    helper_path = pathlib.Path(__file__).with_name("retire-closed-duplicate-candidate.py").resolve()
    spec = importlib.util.spec_from_file_location("closed_duplicate_retirement_live_reader", helper_path)
    if spec is None or spec.loader is None:
        raise CandidateAdmissionError(f"complete live Issue/Project reader is unavailable at {helper_path}")
    module = importlib.util.module_from_spec(spec)
    try:
        spec.loader.exec_module(module)
    except Exception as exc:
        raise CandidateAdmissionError(f"complete live Issue/Project reader could not be loaded: {exc}") from exc
    _RETIREMENT_COLLECTOR = module
    return module


def _complete_issue_inventory(helper: Any, repository: str) -> list[dict[str, Any]]:
    owner, name = repository.split("/", 1)
    try:
        return helper._collect_repository_issues(owner, name, repository)
    except ValueError as exc:
        if str(exc) == "complete GitHub Issue pagination returned no Issues":
            # The collector raises only after the full query and pagination
            # have succeeded; an empty complete repository inventory proves
            # the selected UID is absent from Issues.
            return []
        raise CandidateAdmissionError(f"complete live Issue inventory failed: {exc}") from exc


def _complete_project_inventory(
    helper: Any, project_owner: str, project_number: int, repository: str,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    try:
        return helper._collect_project_items(project_owner, project_number, repository)
    except ValueError as exc:
        if str(exc) == "complete GitHub Project read returned no Issue items":
            # The canonical collector reached a complete, well-formed Project
            # page chain but found no Issue items. This is a valid empty
            # Project for a genuinely new task, not incomplete authority.
            return {}, []
        raise CandidateAdmissionError(f"complete live Project inventory failed: {exc}") from exc


def guard_missing_mapping_candidates(
    task_uids: list[str],
    repository: str,
    project_owner: str,
    project_number: int,
    *,
    skip_recover: bool,
) -> None:
    """Block sync when missing cache rows hide existing live candidate UIDs.

    A fully complete Issue and canonical Project read proves the ordinary new
    task case only when neither surface already binds a selected UID. The
    inventory is shared across selected candidates to keep broad sync bounded.
    The guard is read-only and never synthesizes a replacement mapping row.
    """
    ordered_uids = list(dict.fromkeys(task_uids))
    selected_uids = set(ordered_uids)
    if not selected_uids:
        return
    if any(TASK_UID_RE.fullmatch(task_uid) is None for task_uid in selected_uids):
        raise CandidateAdmissionError("selected candidate Task UID is malformed")
    if not GITHUB_REPO_RE.fullmatch(repository) or project_number <= 0 or not project_owner:
        raise CandidateAdmissionError("selected candidate repository or Project identity is malformed")

    helper = _retirement_collector()
    try:
        issues = _complete_issue_inventory(helper, repository)
        project, project_items = _complete_project_inventory(helper, project_owner, project_number, repository)
    except CandidateAdmissionError:
        raise
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as exc:
        raise CandidateAdmissionError(
            f"complete live Issue/Project discovery failed for selected candidate UIDs: {exc}"
        ) from exc

    if project and (
        project.get("owner") != project_owner
        or int(project.get("number") or 0) != project_number
        or not str(project.get("id") or "")
    ):
        raise CandidateAdmissionError("complete live Project identity conflicts with the selected Project")

    issues_by_uid: dict[str, list[dict[str, Any]]] = {task_uid: [] for task_uid in ordered_uids}
    for issue in issues:
        raw_uid_fields = [value.strip(" \t\r") for value in CANONICAL_UID_FIELD_RE.findall(str(issue.get("body") or ""))]
        matching_fields = [value for value in raw_uid_fields if value in selected_uids]
        if not matching_fields:
            continue
        if len(raw_uid_fields) != 1 or len(set(matching_fields)) != 1:
            for task_uid in set(matching_fields):
                raise CandidateAdmissionError(
                    f"live Issue #{issue.get('number')} has ambiguous canonical task_uid fields for {task_uid}"
                )
        task_uid = matching_fields[0]
        if raw_uid_fields != [task_uid]:
            raise CandidateAdmissionError(
                f"live Issue #{issue.get('number')} has ambiguous canonical task_uid fields for {task_uid}"
            )
        issues_by_uid[task_uid].append(issue)

    items_by_uid: dict[str, list[dict[str, Any]]] = {task_uid: [] for task_uid in ordered_uids}
    for item in project_items:
        fields = item.get("fields") if isinstance(item.get("fields"), dict) else {}
        item_uids = {
            str(item.get("task_uid") or ""),
            str(fields.get("Task UID") or ""),
        } & selected_uids
        if len(item_uids) > 1:
            raise CandidateAdmissionError(
                f"live Project item {item.get('id')} has conflicting Task UID fields"
            )
        if item_uids:
            items_by_uid[next(iter(item_uids))].append(item)

    for task_uid in ordered_uids:
        matching_issues = issues_by_uid[task_uid]
        matching_items = items_by_uid[task_uid]
        if len(matching_issues) > 1:
            raise CandidateAdmissionError(f"complete live Issue discovery found multiple Issues for {task_uid}")
        if len(matching_items) > 1:
            raise CandidateAdmissionError(f"complete live Project discovery found multiple items for {task_uid}")
        if not matching_issues:
            if matching_items:
                raise CandidateAdmissionError(
                    f"live Project item names {task_uid} without one exact matching live Issue; sync was refused"
                )
            continue

        if not project:
            raise CandidateAdmissionError(
                f"live Issue for {task_uid} exists but the complete canonical Project has no matching Issue item"
            )

        issue = matching_issues[0]
        issue_number = int(issue.get("number") or 0)
        issue_items = [item for item in project_items if int(item.get("issue_number") or 0) == issue_number]
        if (
            len(issue_items) != 1
            or len(matching_items) != 1
            or issue_items[0].get("id") != matching_items[0].get("id")
        ):
            duplicate = (
                str(issue.get("state") or "").upper() == "CLOSED"
                and str(issue.get("state_reason") or "").upper() == "DUPLICATE"
            )
            status = "closed as DUPLICATE" if duplicate else "already exists"
            raise CandidateAdmissionError(
                f"live Task UID {task_uid} {status}, but its canonical Project identity is missing or conflicting; "
                "reconcile the mapping before global sync"
            )

        item = matching_items[0]
        fields = item.get("fields") if isinstance(item.get("fields"), dict) else {}
        item_task_uid = str(item.get("task_uid") or "")
        field_task_uid = str(fields.get("Task UID") or "")
        state = str(issue.get("state") or "").upper()
        state_reason = str(issue.get("state_reason") or "").upper()
        duplicate = state == "CLOSED" and state_reason == "DUPLICATE"
        item_identity_is_valid = (
            item_task_uid == task_uid
            and (not field_task_uid or field_task_uid == task_uid)
            and item.get("repository") == repository
            and int(item.get("issue_number") or 0) == issue_number
            and str(item.get("issue_url") or "") == str(issue.get("url") or "")
            and str(item.get("project_id") or "") == str(project.get("id") or "")
            and str(item.get("project_owner") or "") == project_owner
            and int(item.get("project_number") or 0) == project_number
            and item.get("archived") is False
        )
        if not item_identity_is_valid:
            raise CandidateAdmissionError(
                f"live Task UID {task_uid} has a stale or conflicting canonical Project item; sync was refused"
            )

        if duplicate:
            expected_fields = {
                "Status": "Done",
                "PM Status": "candidate",
                "Workflow Phase": "bootstrap",
                "Canonical Worktree": "",
            }
            if not all(fields.get(name) == value for name, value in expected_fields.items()):
                raise CandidateAdmissionError(
                    f"live closed-DUPLICATE candidate {task_uid} has conflicting Project fields; sync was refused"
                )
            raise CandidateAdmissionError(
                f"live candidate Task UID {task_uid} is closed as DUPLICATE but absent from the active mapping; "
                "reconcile the stale mapping before global sync"
            )
        if state == "OPEN":
            if skip_recover:
                raise CandidateAdmissionError(
                    f"live candidate Task UID {task_uid} already exists without an active mapping; "
                    "--skip-recover would create a duplicate Issue, so rerun global sync with recovery enabled"
                )
            # The normal recovery path may restore this uniquely proven active
            # identity through its established Project mapping recovery.
            continue
        raise CandidateAdmissionError(
            f"live candidate Task UID {task_uid} is not open but absent from the active mapping "
            f"(state={state or 'unknown'}, stateReason={state_reason or 'unknown'}); sync was refused"
        )


def _live_issue(repository: str, issue_number: int) -> dict[str, Any]:
    # `gh api` without -X/--method uses GET. Do not accept caller-provided
    # query, method, endpoint, or proof input: identity comes from the mapping.
    try:
        result = subprocess.run(
            ["gh", "api", f"repos/{repository}/issues/{issue_number}"],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=30,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise CandidateAdmissionError(f"live Issue read is unavailable: {exc}") from exc
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or f"exit {result.returncode}"
        raise CandidateAdmissionError(f"live Issue read is unavailable: {detail}")
    try:
        issue = json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise CandidateAdmissionError("live Issue read returned malformed JSON") from exc
    if not isinstance(issue, dict):
        raise CandidateAdmissionError("live Issue read returned a malformed object")
    if str(issue.get("number") or "") != str(issue_number):
        raise CandidateAdmissionError("live Issue readback number differs from the candidate mapping")
    body = issue.get("body")
    if not isinstance(body, str):
        raise CandidateAdmissionError("live Issue body is unavailable")
    uid_lines = CANONICAL_UID_LINE_RE.findall(body)
    if len(uid_lines) != 1:
        raise CandidateAdmissionError("live Issue canonical task_uid field is missing or ambiguous")
    return issue


def _live_disposition_comment_id(repository: str, issue_number: int) -> int:
    endpoint = f"repos/{repository}/issues/{issue_number}/comments?per_page=100"
    try:
        result = subprocess.run(
            ["gh", "api", endpoint, "--paginate", "--slurp"],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=30,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise CandidateAdmissionError(f"live disposition comment pagination is unavailable: {exc}") from exc
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or f"exit {result.returncode}"
        raise CandidateAdmissionError(f"live disposition comment pagination is unavailable: {detail}")
    try:
        pages = json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise CandidateAdmissionError("live disposition comment pagination returned malformed JSON") from exc
    if not isinstance(pages, list) or not pages:
        raise CandidateAdmissionError("live disposition comment pagination is incomplete")
    comments: list[dict[str, Any]] = []
    for page in pages:
        values = page if isinstance(page, list) else [page]
        if not isinstance(values, list) or any(not isinstance(item, dict) for item in values):
            raise CandidateAdmissionError("live disposition comment pagination returned a malformed page")
        comments.extend(values)
    marked = [
        comment for comment in comments
        if str(comment.get("body") or "").startswith("<!-- oasis7.duplicate-candidate-disposition/v1 -->\n")
    ]
    if len(marked) != 1:
        raise CandidateAdmissionError("live duplicate disposition marker is missing or ambiguous")
    try:
        comment_id = int(marked[0].get("id") or 0)
    except (TypeError, ValueError) as exc:
        raise CandidateAdmissionError("live duplicate disposition marker has no server ID") from exc
    if comment_id <= 0:
        raise CandidateAdmissionError("live duplicate disposition marker has no server ID")
    return comment_id


def _reconcile_command(mapping_path: pathlib.Path, task_uid: str, comment_id: int) -> list[str]:
    # This is a locator for the established read-only preflight only. It does
    # not confer trust on this checkout or authorize the explicit apply mode.
    mapping_root = mapping_path.resolve().parents[2]
    helper = pathlib.Path(__file__).with_name("retire-closed-duplicate-candidate.py").resolve()
    return [
        "python3", str(helper),
        "--mapping-root", str(mapping_root),
        "--task-uid", task_uid,
        "--disposition-comment-id", str(comment_id),
        "--preflight",
    ]


def guard_candidate_issue(
    mapping: dict[str, Any],
    mapping_path: pathlib.Path,
    task_uid: str,
    task: dict[str, Any],
) -> None:
    """Verify candidate admission against one fresh live Issue read.

    Non-candidate and already-started task behavior is unchanged. Every
    bootstrap candidate fails closed on unavailable/ambiguous live identity
    and directs a closed duplicate to retirement preflight.
    """
    if not _candidate(task):
        return
    if TASK_UID_RE.fullmatch(task_uid) is None or task.get("task_uid") != task_uid:
        raise CandidateAdmissionError("candidate mapping Task UID is malformed or differs from its key")
    project = mapping.get("project")
    repository = str((project or {}).get("repo") or task.get("repository") or "")
    if not GITHUB_REPO_RE.fullmatch(repository) or task.get("repository") != repository:
        raise CandidateAdmissionError("candidate mapping repository identity is missing or conflicting")
    issue_number = _issue_number(task.get("issue_number"))
    issue = _live_issue(repository, issue_number)
    uid = CANONICAL_UID_LINE_RE.findall(str(issue.get("body") or ""))[0]
    if uid != task_uid:
        raise CandidateAdmissionError("live Issue canonical task_uid differs from the candidate mapping")
    state = str(issue.get("state") or "").upper()
    state_reason = str(issue.get("state_reason") or "").upper()
    if state == "CLOSED" and state_reason == "DUPLICATE":
        comment_id = _live_disposition_comment_id(repository, issue_number)
        command = _reconcile_command(mapping_path, task_uid, comment_id)
        raise CandidateAdmissionError(
            "live candidate Issue is closed as DUPLICATE; reconcile its stale mapping before bootstrap; "
            f"supported read-only preflight: {shlex.join(command)}",
            command,
        )
    if state != "OPEN":
        raise CandidateAdmissionError("live candidate Issue is not open; refresh canonical task truth before bootstrap")
