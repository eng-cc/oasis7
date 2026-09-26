#!/usr/bin/env python3
"""Read-only pre-admission guard for candidate GitHub Issues.

Candidate bootstrap is an admission boundary even when the cache has no
duplicate path/branch alias: the local task mapping is only a cache, so the
candidate's exact Issue must still be live and uniquely bind the same Task UID
before a snapshot can be created or reused. Active non-candidate tasks are
unchanged. All GitHub calls are GET-only ``gh api`` requests; retirement/apply
is never called.
"""
from __future__ import annotations

import json
import pathlib
import re
import shlex
import subprocess
from typing import Any


GITHUB_REPO_RE = re.compile(r"^[^/\s]+/[^/\s]+$")
TASK_UID_RE = re.compile(r"^task_[0-9a-f]{32}$")
CANONICAL_UID_LINE_RE = re.compile(r"(?m)^task_uid:\s*(.*?)\s*$")


class CandidateAdmissionError(ValueError):
    """Candidate truth could not safely pass read-only bootstrap admission."""

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
