#!/usr/bin/env python3
"""Read-only delivery readiness projection for explicitly declared task artifacts."""
from __future__ import annotations

import hashlib
import importlib.util
import json
import pathlib
import re
import subprocess
import sys
from typing import Any


REPOSITORY = "eng-cc/oasis7"
REQUIRED_GATE_APP_ID = 15368
ARTIFACT_DEPENDENCY_MARKER = "<!-- oasis7-artifact-dependency/v1 -->"
ARTIFACT_DEPENDENCY_SCHEMA = "oasis7-artifact-dependency/v1"
ARTIFACT_PATH_RE = re.compile(r"^[A-Za-z0-9._/-]+$")
OID_RE = re.compile(r"^[0-9a-f]{40,64}$")
TASK_UID_RE = re.compile(r"^task_[0-9a-f]{32}$")
_EXCEPTION_PATH = pathlib.Path(__file__).with_name("workflow-process-exception.py")
_EXCEPTION_SPEC = importlib.util.spec_from_file_location("workflow_process_exception", _EXCEPTION_PATH)
if _EXCEPTION_SPEC is None or _EXCEPTION_SPEC.loader is None:
    raise RuntimeError(f"cannot load process-exception validator at {_EXCEPTION_PATH}")
PROCESS_EXCEPTIONS = importlib.util.module_from_spec(_EXCEPTION_SPEC)
_EXCEPTION_SPEC.loader.exec_module(PROCESS_EXCEPTIONS)


def _action_blocker(
    code: str,
    reason: str,
    *,
    blocks_actions: list[str],
    allowed_actions: list[str],
    next_action_kind: str,
) -> dict[str, Any]:
    return {
        "code": code,
        "blocks_actions": blocks_actions,
        "allowed_actions": allowed_actions,
        "next_action_kind": next_action_kind,
        "next_command": None,
        "reason": reason,
    }


BLOCKER_POLICY: dict[str, tuple[list[str], list[str], str]] = {
    "TASK_BINDING_CONFLICT": (
        ["freeze", "publish", "merge", "state_write", "consume_artifact"],
        ["inspect", "repair_within_authorized_scope"],
        "trusted_mapping_reconciliation",
    ),
    "TEST_HARNESS_FAILURE": (
        ["pass", "merge", "activate", "release", "complete"],
        ["inspect", "diagnose", "repair_within_authorized_scope"],
        "authorized_test_repair",
    ),
    "REVIEW_FINDING_BLOCKING": (
        ["merge", "complete"],
        ["inspect", "repair_within_authorized_scope", "resolve_with_evidence"],
        "resolve_current_review_finding",
    ),
    "CURRENT_CHECK_FAILED": (
        ["merge", "complete"],
        ["inspect", "diagnose", "repair_within_authorized_scope", "rerun_applicable_check"],
        "rerun_current_required_check",
    ),
    "SOURCE_NOT_PUBLISHED": (
        ["validate", "merge"],
        ["inspect", "publish_after_preflight"],
        "publish_verified_source",
    ),
    "CAPABILITY_MISSING_ARCHIVE_READBACK": (
        ["archive", "cleanup", "terminal"],
        ["inspect", "consume_verified_artifact"],
        "obtain_archive_readback",
    ),
    "CLEANUP_DEFERRED": (
        ["archive", "cleanup", "terminal"],
        ["inspect", "consume_verified_artifact"],
        "resume_cleanup_when_readback_exists",
    ),
    "UNCLASSIFIED_BLOCKER": (
        ["freeze", "publish", "merge", "consume_artifact", "cleanup", "terminal", "state_write"],
        ["inspect"],
        "inspect_unclassified_blocker",
    ),
}

DELIVERY_BLOCKS_ACTIONS: dict[str, list[str]] = {
    "REVIEW_FINDING_BLOCKING": ["merge", "complete", "consume_artifact"],
    "CURRENT_CHECK_FAILED": ["merge", "complete", "consume_artifact"],
    "SOURCE_NOT_PUBLISHED": ["validate", "merge", "consume_artifact"],
}


def project_action_blockers(blockers: list[str]) -> list[dict[str, Any]]:
    """Project legacy blocker strings into action-specific, non-authorizing advice."""
    projected: list[dict[str, Any]] = []
    for message in blockers:
        text = str(message)
        lowered = text.lower()
        if "stale identity" in lowered or "ambiguous state" in lowered or "identity drift" in lowered:
            code = "TASK_BINDING_CONFLICT"
        elif "test harness" in lowered or "harness failure" in lowered:
            code = "TEST_HARNESS_FAILURE"
        elif "review finding" in lowered or "review blocks" in lowered:
            code = "REVIEW_FINDING_BLOCKING"
        elif "check" in lowered or "ci " in lowered or lowered.startswith("ci:"):
            code = "CURRENT_CHECK_FAILED"
        elif "source" in lowered and any(term in lowered for term in ("missing", "unpublished", "stale")):
            code = "SOURCE_NOT_PUBLISHED"
        elif "archive" in lowered or "cleanup" in lowered:
            code = "CAPABILITY_MISSING_ARCHIVE_READBACK"
        else:
            code = "UNCLASSIFIED_BLOCKER"
        blocked, allowed, next_kind = BLOCKER_POLICY[code]
        projected.append(_action_blocker(
            code,
            text,
            blocks_actions=list(blocked),
            allowed_actions=list(allowed),
            next_action_kind=next_kind,
        ))
    return projected


def _has_identity(value: Any, expected: str) -> bool:
    return isinstance(value, str) and value == expected


def _ci_ok(value: Any, *, head_oid: str | None = None) -> bool:
    if not isinstance(value, dict):
        return False
    return (
        value.get("status") == "completed"
        and str(value.get("conclusion") or "").lower() == "success"
        and (head_oid is None or value.get("head_oid") == head_oid)
        and type(value.get("run_id")) is int and value["run_id"] > 0
        and type(value.get("run_attempt")) is int and value["run_attempt"] > 0
    )


def derive_delivery_readiness(task_uid: str, proof: dict[str, Any]) -> dict[str, Any]:
    """Evaluate one typed artifact edge from independently loaded live facts.

    No declaration means legacy terminal-delivery semantics. ``proof`` is
    produced internally by ``read_explicit_edge`` and cannot be supplied on CLI.
    """
    declaration = proof.get("declaration") if isinstance(proof, dict) else None
    if not isinstance(declaration, dict):
        return {
            "delivery_ready": None,
            "cleanup_state": "not_applicable",
            "action_blockers": [],
            "dependency": "legacy_or_unclassified_terminal_semantics",
        }

    reasons: list[tuple[str, str]] = []
    authority = proof.get("declaration_authority")
    downstream = proof.get("downstream")
    upstream = proof.get("upstream_issue")
    pr = proof.get("pr")
    review = proof.get("review")
    source_ci = proof.get("source_ci")
    local_input = proof.get("local_input")
    pr_number = pr.get("number") if isinstance(pr, dict) else None
    pr_head_oid = pr.get("head_oid") if isinstance(pr, dict) else None
    upstream_uid = declaration.get("upstream_task_uid")
    artifacts = declaration.get("artifacts")
    locator = declaration.get("source_ci")

    if (declaration.get("schema") != ARTIFACT_DEPENDENCY_SCHEMA
            or declaration.get("task_uid") != task_uid
            or not isinstance(upstream_uid, str) or not TASK_UID_RE.fullmatch(upstream_uid)):
        reasons.append(("TASK_BINDING_CONFLICT", "the typed artifact dependency is malformed or bound to a different task"))
    if not isinstance(authority, dict) or not (
        authority.get("permission") == "admin" and authority.get("body_valid") is True
    ):
        reasons.append(("TASK_BINDING_CONFLICT", "the typed dependency lacks current authenticated Task-Issue authority"))
    if not isinstance(downstream, dict) or not (
        downstream.get("task_uid") == task_uid
        and downstream.get("issue_number") == declaration.get("task_issue_number")
    ):
        reasons.append(("TASK_BINDING_CONFLICT", "the bound Task Issue does not match the dependency declaration"))
    if not isinstance(upstream, dict) or not (
        upstream.get("task_uid") == upstream_uid
        and upstream.get("status") == "done"
        and upstream.get("workflow_phase") in {"task_done", "main_sync", "post_merge_done"}
        and str(upstream.get("pr_number") or "") == str(pr_number or "")
    ):
        reasons.append(("TASK_BINDING_CONFLICT", "the live upstream Task Issue or delivered phase is incomplete"))
    if not isinstance(upstream, dict) or upstream.get("merge_hold_active") is not False:
        reasons.append(("TASK_BINDING_CONFLICT", "the upstream task has an active or uncertain merge/acceptance hold"))
    if not isinstance(pr, dict) or not (
        pr.get("repository") == declaration.get("repository")
        and pr.get("number") == (locator.get("pr_number") if isinstance(locator, dict) else None)
        and pr.get("issue_number") == proof.get("upstream_issue_number")
        and pr.get("task_uid") == upstream_uid
        and str(pr.get("state") or "").lower() == "closed"
        and pr.get("merged") is True
        and isinstance(pr.get("merge_commit_oid"), str)
        and OID_RE.fullmatch(pr["merge_commit_oid"])
        and pr.get("base_ref") == pr.get("default_branch")
        and isinstance(pr.get("head_oid"), str)
        and OID_RE.fullmatch(pr["head_oid"])
    ):
        reasons.append(("TASK_BINDING_CONFLICT", "the uniquely reciprocal live merged PR does not match the declared upstream task"))
    if not isinstance(review, dict) or not (
        review.get("task_uid") == upstream_uid
        and pr_head_oid is not None and review.get("source_head_oid") == pr_head_oid
        and review.get("passed") is True
        and review.get("admin_author") is True
        and review.get("findings_disposition") == "addressed"
        and isinstance(review.get("roles"), list) and bool(review["roles"])
    ):
        reasons.append(("REVIEW_FINDING_BLOCKING", "current source-head review is missing, stale, incomplete or unresolved"))
    if not isinstance(source_ci, dict) or not (
        isinstance(locator, dict)
        and source_ci.get("check_name") == "required-gate"
        and source_ci.get("app_id") == REQUIRED_GATE_APP_ID
        and source_ci.get("check_run_id") == locator.get("check_run_id")
        and source_ci.get("run_id") == locator.get("run_id")
        and source_ci.get("run_attempt") == locator.get("run_attempt")
        and source_ci.get("pr_number") == pr_number
        and source_ci.get("head_oid") == pr_head_oid
        and source_ci.get("pr_association_verified") is True
        and _ci_ok(source_ci, head_oid=pr_head_oid)
    ):
        reasons.append(("CURRENT_CHECK_FAILED", "the exact declared source-PR required-gate check/run/attempt is missing, unassociated or unsuccessful"))
    if not isinstance(artifacts, list) or not artifacts or not isinstance(local_input, dict) or not (
        local_input.get("repository") == declaration.get("repository")
        and local_input.get("head_contains_merge_commit") is True
        and local_input.get("artifacts_match") is True
    ):
        reasons.append(("SOURCE_NOT_PUBLISHED", "the downstream worktree does not contain every exact declared artifact from the merged source"))

    action_blockers = []
    seen: set[tuple[str, str]] = set()
    for code, reason in reasons:
        if (code, reason) in seen:
            continue
        seen.add((code, reason))
        blocked, allowed, next_kind = BLOCKER_POLICY[code]
        action_blockers.append(_action_blocker(
            code, reason,
            blocks_actions=list(DELIVERY_BLOCKS_ACTIONS.get(code, blocked)),
            allowed_actions=list(allowed),
            next_action_kind=next_kind,
        ))
    ready = not reasons
    cleanup_state = "not_applicable"
    if ready and isinstance(upstream, dict):
        phase = str(upstream.get("workflow_phase") or "")
        cleanup_state = "cleanup_deferred" if phase != "post_merge_done" else "cleanup_readback_unverified"
        if cleanup_state == "cleanup_deferred":
            blocked, allowed, next_kind = BLOCKER_POLICY["CLEANUP_DEFERRED"]
            action_blockers.append(_action_blocker(
                "CLEANUP_DEFERRED",
                "the artifact is verified for this explicit consumer, while upstream terminal cleanup/readback is still pending",
                blocks_actions=list(blocked),
                allowed_actions=list(allowed),
                next_action_kind=next_kind,
            ))
    return {
        "delivery_ready": ready,
        "cleanup_state": cleanup_state,
        "action_blockers": action_blockers,
        "dependency": f"{upstream_uid}:artifact",
    }


def _gh_json(path: str) -> Any:
    try:
        result = subprocess.run(["gh", "api", path], text=True, capture_output=True, check=False)
    except OSError as exc:
        raise ValueError(f"GitHub read unavailable: {exc}") from exc
    if result.returncode:
        raise ValueError((result.stderr or result.stdout).strip() or f"GitHub read failed: {path}")
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise ValueError(f"GitHub response is not JSON: {path}") from exc


def _pages(path: str, key: str | None = None) -> list[Any]:
    rows: list[Any] = []
    for page in range(1, 101):
        separator = "&" if "?" in path else "?"
        value = _gh_json(f"{path}{separator}per_page=100&page={page}")
        batch = value if key is None else value.get(key) if isinstance(value, dict) else None
        if not isinstance(batch, list):
            raise ValueError(f"GitHub pagination response is malformed: {path}")
        rows.extend(batch)
        if len(batch) < 100:
            return rows
    raise ValueError(f"GitHub pagination limit exceeded: {path}")


def _body_field(body: str, field: str) -> str | None:
    matches = re.findall(
        rf"(?m)^-[ \t]+{re.escape(field)}:[ \t]*(?:`([^`\r\n]+)`|([^\r\n]+))[ \t]*$",
        body,
    )
    if len(matches) != 1:
        return None
    return (matches[0][0] or matches[0][1]).strip()


def _canonical_uid(body: str) -> str | None:
    matches = re.findall(r"(?m)^task_uid: (task_[0-9a-f]{32})$", body)
    return matches[0] if len(matches) == 1 else None


def _positive_int(value: Any) -> bool:
    return type(value) is int and value > 0


def _is_admin(repository: str, login: str) -> bool:
    from urllib.parse import quote
    endpoint = f"repos/{repository}/collaborators/{quote(login, safe='')}/permission"
    try:
        result = subprocess.run(
            ["gh", "api", "--include", endpoint], text=True, capture_output=True, check=False,
        )
    except OSError as exc:
        raise ValueError(f"GitHub permission read unavailable: {exc}") from exc
    response = (result.stdout or "").replace("\r\n", "\n")
    status_line = response.split("\n", 1)[0]
    match = re.fullmatch(r"HTTP/\S+ ([0-9]{3})(?: .*)?", status_line)
    if not match:
        raise ValueError("GitHub permission response status is unavailable")
    status = int(match.group(1))
    if status == 404:
        return False
    if result.returncode or not 200 <= status < 300:
        raise ValueError(f"GitHub permission read failed with HTTP {status}")
    separator = response.find("\n\n")
    if separator < 0:
        raise ValueError("GitHub permission response body is unavailable")
    try:
        value = json.loads(response[separator + 2:])
    except json.JSONDecodeError as exc:
        raise ValueError("GitHub permission response body is malformed") from exc
    user = value.get("user") if isinstance(value, dict) else None
    if not isinstance(user, dict) or user.get("login") != login:
        raise ValueError("GitHub permission response user does not match the comment author")
    return value.get("permission") == "admin"


def _typed_record(comments: list[Any], marker: str, schema: str, issue_url: str) -> tuple[dict[str, Any], dict[str, Any]] | None:
    candidates = [item for item in comments if isinstance(item, dict)
                  and isinstance(item.get("body"), str) and marker in item["body"]]
    if not candidates:
        return None
    if len(candidates) != 1:
        raise ValueError("Task Issue contains duplicate typed evidence records")
    comment = candidates[0]
    body = comment.get("body")
    if comment.get("issue_url") != issue_url or not body.startswith(marker + "\n") or body.count(marker) != 1:
        raise ValueError("typed evidence marker framing or Task Issue identity is invalid")
    raw = body[len(marker) + 1:]
    try:
        record = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise ValueError("typed evidence is malformed JSON") from exc
    if (not isinstance(record, dict)
            or json.dumps(record, ensure_ascii=True, sort_keys=True, separators=(",", ":")) != raw
            or record.get("schema") != schema
            or type(comment.get("id")) is not int):
        raise ValueError("typed evidence is noncanonical or has an unsupported schema")
    return comment, record


def _associated_pr_numbers(value: Any, repository: str) -> list[int]:
    if not isinstance(value, list) or not value:
        raise ValueError("check/run has no live PR association")
    numbers: list[int] = []
    for item in value:
        number = item.get("number") if isinstance(item, dict) else None
        if not _positive_int(number):
            url = item.get("url") or item.get("html_url") if isinstance(item, dict) else None
            match = re.fullmatch(rf"https://(?:api\.)?github\.com/{re.escape(repository)}/(?:pulls|pull)/([1-9][0-9]*)(?:/.*)?", str(url or ""))
            if not match:
                raise ValueError("check/run PR association is malformed")
            number = int(match.group(1))
        numbers.append(number)
    return numbers


def _verify_declared_ci(repository: str, locator: dict[str, Any], pr_number: int,
                        head_oid: str, head_branch: str, *,
                        allow_empty_pr_association: bool = False) -> dict[str, Any]:
    required = {"pr_number", "head_oid", "check_run_id", "run_id", "run_attempt"}
    if set(locator) != required or not all(_positive_int(locator.get(key)) for key in
                                           ("pr_number", "check_run_id", "run_id", "run_attempt")):
        raise ValueError("typed source CI locator has missing or unknown fields")
    if locator["pr_number"] != pr_number or locator["head_oid"] != head_oid:
        raise ValueError("typed source CI locator does not bind the derived reciprocal PR head")

    check = _gh_json(f"repos/{repository}/check-runs/{locator['check_run_id']}")
    if not isinstance(check, dict) or check.get("id") != locator["check_run_id"]:
        raise ValueError("declared check-run identity is unreadable or mismatched")
    check_associations = check.get("pull_requests")
    if not isinstance(check_associations, list):
        raise ValueError("check-run PR association is malformed")
    associations = _associated_pr_numbers(check_associations, repository) if check_associations else []
    if (associations and any(number != pr_number for number in associations)
            or not associations and not allow_empty_pr_association):
        raise ValueError("check-run PR association is empty or conflicts with the declared PR")
    if (check.get("name") != "required-gate"
            or (check.get("app") or {}).get("id") != REQUIRED_GATE_APP_ID
            or check.get("head_sha") != head_oid):
        raise ValueError("declared check-run name, app, or head is not the required source check")
    details = str(check.get("details_url") or "")
    match = re.fullmatch(rf"https://github\.com/{re.escape(repository)}/actions/runs/([1-9][0-9]*)/job/([1-9][0-9]*)", details)
    if not match or int(match.group(1)) != locator["run_id"]:
        raise ValueError("declared check-run details do not identify the bound workflow run")

    run_id = locator["run_id"]
    run = _gh_json(f"repos/{repository}/actions/runs/{run_id}")
    if (not isinstance(run, dict) or run.get("id") != run_id
            or run.get("run_attempt") != locator["run_attempt"]
            or run.get("head_sha") != head_oid or run.get("head_branch") != head_branch
            or run.get("path") != ".github/workflows/rust.yml"
            or run.get("event") != "pull_request"):
        raise ValueError("declared workflow run repository/ref/head/event/attempt identity is mismatched")
    run_association_rows = run.get("pull_requests")
    if not isinstance(run_association_rows, list):
        raise ValueError("workflow run PR association is malformed")
    run_associations = _associated_pr_numbers(run_association_rows, repository) if run_association_rows else []
    if (run_associations and any(number != pr_number for number in run_associations)
            or not run_associations and not allow_empty_pr_association):
        raise ValueError("workflow run PR association is empty or conflicts with the declared PR")
    jobs = _pages(f"repos/{repository}/actions/runs/{run_id}/jobs", "jobs")
    job_id = int(match.group(2))
    job_url = f"https://api.github.com/repos/{repository}/check-runs/{locator['check_run_id']}"
    matches = [job for job in jobs if isinstance(job, dict)
               and job.get("id") == job_id and job.get("name") == "required-gate"
               and job.get("run_attempt") == locator["run_attempt"]
               and job.get("check_run_url") == job_url]
    if len(matches) != 1:
        raise ValueError("declared required-gate job is missing or ambiguous for this attempt")
    job = matches[0]
    if (check.get("status") != "completed" or str(check.get("conclusion") or "").lower() != "success"
            or run.get("status") != "completed" or str(run.get("conclusion") or "").lower() != "success"
            or job.get("status") != "completed" or str(job.get("conclusion") or "").lower() != "success"):
        raise ValueError("declared source check/run/job attempt is not successful")
    return {
        "check_name": check.get("name"), "app_id": (check.get("app") or {}).get("id"),
        "head_oid": head_oid, "status": check.get("status"), "conclusion": check.get("conclusion"),
        "check_run_id": check.get("id"), "run_id": run_id, "run_attempt": locator["run_attempt"],
        "pr_number": pr_number, "pr_association_verified": True,
        "pr_association_basis": "live_github" if check_associations or run_association_rows else "authenticated_task_evidence_and_exact_run_identity",
        "workflow_path": run.get("path"), "head_branch": run.get("head_branch"),
        "job_name": job.get("name"), "job_status": job.get("status"),
        "job_conclusion": job.get("conclusion"),
    }


def _safe_artifact_path(name: Any) -> bool:
    if (not isinstance(name, str) or not ARTIFACT_PATH_RE.fullmatch(name)
            or name.startswith("/") or "//" in name):
        return False
    path = pathlib.PurePosixPath(name)
    return (not path.is_absolute() and all(part not in {"", ".", ".."} for part in path.parts)
            and "\\" not in name)


def _artifact_matches(root: pathlib.Path, merge_oid: str, declaration_artifacts: list[Any],
                      changed_files: list[Any]) -> tuple[bool, list[dict[str, str]]]:
    changed: set[str] = set()
    for row in changed_files:
        if not isinstance(row, dict) or not isinstance(row.get("filename"), str):
            raise ValueError("merged PR file list contains malformed entry")
        changed.add(row["filename"])
    digests: list[dict[str, str]] = []
    seen_paths: set[str] = set()
    for item in declaration_artifacts:
        if (not isinstance(item, dict) or set(item) != {"path", "sha256"}
                or not _safe_artifact_path(item.get("path"))
                or not isinstance(item.get("sha256"), str)
                or not re.fullmatch(r"[0-9a-f]{64}", item["sha256"])):
            raise ValueError("typed artifact entry has unsafe path or invalid digest")
        name = item["path"]
        if name in seen_paths:
            raise ValueError("typed artifact paths must be unique")
        seen_paths.add(name)
        if name not in changed:
            return False, digests
        path = root.joinpath(*pathlib.PurePosixPath(name).parts)
        try:
            expected = subprocess.check_output(
                ["git", "-C", str(root), "show", f"{merge_oid}:{name}"], stderr=subprocess.DEVNULL,
            )
            actual = path.read_bytes()
        except (OSError, subprocess.CalledProcessError):
            return False, digests
        digest = hashlib.sha256(expected).hexdigest()
        if digest != item["sha256"] or actual != expected:
            return False, digests
        digests.append({"path": name, "sha256": digest})
    return bool(declaration_artifacts), digests


def read_explicit_edge(root: pathlib.Path, task: dict[str, Any]) -> dict[str, Any]:
    """Resolve a typed artifact dependency from the bound Task Issue's live evidence."""
    task_uid = str(task.get("task_uid") or "")
    repository = str(task.get("repository") or "")
    issue_number_raw = task.get("issue_number")
    if isinstance(issue_number_raw, str) and re.fullmatch(r"[1-9][0-9]*", issue_number_raw):
        issue_number_raw = int(issue_number_raw)
    if repository != REPOSITORY:
        return {}
    if not TASK_UID_RE.fullmatch(task_uid) or not _positive_int(issue_number_raw):
        raise ValueError("bound task identity is malformed for dependency discovery")
    issue_number = int(issue_number_raw)
    issue_url = f"https://api.github.com/repos/{repository}/issues/{issue_number}"
    issue = _gh_json(f"repos/{repository}/issues/{issue_number}")
    body = issue.get("body") if isinstance(issue, dict) else None
    if (not isinstance(issue, dict) or issue.get("number") != issue_number
            or "pull_request" in issue or not isinstance(body, str)
            or _canonical_uid(body) != task_uid):
        raise ValueError("bound Task Issue live UID/number identity is invalid")
    comments = _pages(f"repos/{repository}/issues/{issue_number}/comments", None)
    typed = _typed_record(comments, ARTIFACT_DEPENDENCY_MARKER, ARTIFACT_DEPENDENCY_SCHEMA, issue_url)
    if typed is None:
        return {}
    issue = _resolve_project_task_issue(task_uid, issue_number)
    body = issue["body"]
    declaration_comment, declaration = typed
    author = (declaration_comment.get("user") or {}).get("login")
    if (not isinstance(author, str) or not _is_admin(repository, author)
            or declaration.get("repository") != repository
            or declaration.get("task_uid") != task_uid
            or declaration.get("task_issue_number") != issue_number):
        raise ValueError("typed artifact dependency lacks current Task-Issue admin authority or exact binding")
    expected_fields = {"schema", "repository", "task_uid", "task_issue_number", "upstream_task_uid", "artifacts", "source_ci"}
    if (set(declaration) != expected_fields
            or not isinstance(declaration.get("upstream_task_uid"), str)
            or not TASK_UID_RE.fullmatch(declaration["upstream_task_uid"])
            or not isinstance(declaration.get("artifacts"), list) or not declaration["artifacts"]
            or not isinstance(declaration.get("source_ci"), dict)):
        raise ValueError("typed artifact dependency fields are incomplete or unknown")

    upstream_issue = _resolve_project_task_issue(declaration["upstream_task_uid"])
    upstream_number = upstream_issue.get("number")
    upstream_body = upstream_issue["body"]
    if not _positive_int(upstream_number):
        raise ValueError("upstream Task Issue number is malformed")
    project_task_pr_number, project_task_pr_url = _project_readback()._live_task_pr(
        upstream_issue, declaration["upstream_task_uid"],
    )
    pr_number = project_task_pr_number
    if project_task_pr_url != f"https://github.com/{repository}/pull/{pr_number}":
        raise ValueError("Project Task Issue PR URL does not match its canonical PR number")
    pr = _gh_json(f"repos/{repository}/pulls/{pr_number}")
    repo = _gh_json(f"repos/{repository}")
    default_branch = repo.get("default_branch") if isinstance(repo, dict) else None
    pr_body = str(pr.get("body") or "") if isinstance(pr, dict) else ""
    pr_head = pr.get("head") if isinstance(pr, dict) else None
    pr_base = pr.get("base") if isinstance(pr, dict) else None
    task_refs = re.findall(rf"(?m)^Task: {re.escape(declaration['upstream_task_uid'])}$", pr_body)
    issue_refs = re.findall(rf"(?m)^Refs #{upstream_number}$", pr_body)
    head_oid = pr_head.get("sha") if isinstance(pr_head, dict) else None
    head_branch = pr_head.get("ref") if isinstance(pr_head, dict) else None
    merge_oid = pr.get("merge_commit_sha") if isinstance(pr, dict) else None
    if (not isinstance(pr, dict) or pr.get("number") != pr_number
            or len(task_refs) != 1 or len(issue_refs) != 1
            or pr.get("state") != "closed" or pr.get("merged") is not True
            or not isinstance(pr_base, dict) or pr_base.get("ref") != default_branch
            or not isinstance(head_oid, str) or not OID_RE.fullmatch(head_oid)
            or not isinstance(head_branch, str) or not isinstance(merge_oid, str) or not OID_RE.fullmatch(merge_oid)):
        raise ValueError("reciprocal source PR is not a verified merged delivery to the default branch")

    locator = declaration["source_ci"]
    source_ci = _verify_declared_ci(
        repository, locator, pr_number, head_oid, head_branch,
        allow_empty_pr_association=True,
    )
    source_comments = _pages(f"repos/{repository}/issues/{upstream_number}/comments", None)
    reviews = []
    for comment in source_comments:
        text = comment.get("body") if isinstance(comment, dict) else None
        if not isinstance(text, str):
            continue
        if (_body_field(text, "Task UID") == declaration["upstream_task_uid"]
                and _body_field(text, "Source Head") == head_oid
                and _body_field(text, "Review Roles") is not None
                and any(line.startswith("- Pre-PR Local Role Review:") for line in text.splitlines())):
            reviews.append(comment)
    if not reviews:
        raise ValueError("source Task Issue lacks an exact source-head review packet")
    reviews.sort(key=lambda item: (str(item.get("updated_at") or item.get("created_at") or ""),
                                   int(item.get("id") or 0)))
    review_comment = reviews[-1]
    review_author = (review_comment.get("user") or {}).get("login")
    review_text = str(review_comment.get("body") or "")
    role_line = next((line for line in review_text.splitlines() if line.startswith("- Review Roles: ")), "")
    roles = [role.strip() for role in role_line.removeprefix("- Review Roles: ").split(",") if role.strip()]
    review = {
        "comment_id": review_comment.get("id"), "task_uid": _body_field(review_text, "Task UID"),
        "source_head_oid": _body_field(review_text, "Source Head"),
        "passed": "- Pre-PR Local Role Review: passed" in review_text,
        "roles": roles, "findings_disposition": _body_field(review_text, "Review Findings Disposition"),
        "admin_author": isinstance(review_author, str) and _is_admin(repository, review_author),
    }
    if review_comment.get("issue_url") != f"https://api.github.com/repos/{repository}/issues/{upstream_number}":
        raise ValueError("source review packet is published on the wrong Task Issue")

    changed_files = _pages(f"repos/{repository}/pulls/{pr_number}/files", None)
    artifact_ok, file_digests = _artifact_matches(root, merge_oid, declaration["artifacts"], changed_files)
    local_head = _git_value(root, "rev-parse", "HEAD")
    ancestry = subprocess.run(
        ["git", "-C", str(root), "merge-base", "--is-ancestor", merge_oid, "HEAD"],
        capture_output=True, check=False,
    ).returncode == 0
    local_input = {
        "repository": repository if _repository_identity(root) == repository else None,
        "head_oid": local_head, "head_contains_merge_commit": ancestry,
        "artifacts_match": artifact_ok and ancestry and _repository_identity(root) == repository,
        "file_digests": file_digests,
    }
    hold_value = _body_field(upstream_body, "merge_hold_active")
    merge_hold_active = True if hold_value == "true" else False if hold_value == "false" else None
    proof = {
        "declaration": declaration,
        "declaration_authority": {
            "comment_id": declaration_comment.get("id"), "author": author,
            "permission": "admin", "body_valid": True,
        },
        "downstream": {"issue_number": issue_number, "task_uid": task_uid},
        "upstream_issue_number": upstream_number,
        "upstream_issue": {
            "issue_number": upstream_number,
            "task_uid": _canonical_uid(upstream_body),
            "status": _body_field(upstream_body, "status"),
            "workflow_phase": _body_field(upstream_body, "workflow_phase"),
            "pr_number": pr_number,
            "merge_hold_active": merge_hold_active,
        },
        "pr": {
            "repository": repository, "number": pr_number, "issue_number": upstream_number,
            "task_uid": declaration["upstream_task_uid"], "state": pr.get("state"),
            "merged": pr.get("merged"), "merge_commit_oid": merge_oid,
            "head_oid": head_oid, "base_ref": pr_base.get("ref"),
            "default_branch": default_branch,
        },
        "review": review,
        "source_ci": source_ci,
        "local_input": local_input,
    }
    return proof


def _git_value(root: pathlib.Path, *args: str) -> str:
    try:
        return subprocess.check_output(["git", "-C", str(root), *args], text=True, stderr=subprocess.DEVNULL).strip()
    except (OSError, subprocess.CalledProcessError):
        return ""


def _repository_identity(root: pathlib.Path) -> str | None:
    origin = _git_value(root, "config", "--get", "remote.origin.url")
    match = re.fullmatch(r"(?:https://github\.com/|git@github\.com:)([^/]+/[^/]+?)(?:\.git)?", origin)
    return match.group(1) if match else None


_PROJECT_READBACK_MODULE: Any = None


def _project_readback() -> Any:
    global _PROJECT_READBACK_MODULE
    if _PROJECT_READBACK_MODULE is not None:
        return _PROJECT_READBACK_MODULE
    path = pathlib.Path(__file__).with_name("ci_reuse_validation_readback.py")
    if not path.is_file() or path.is_symlink():
        raise ValueError("canonical Project task resolver is unavailable")
    spec = importlib.util.spec_from_file_location("workflow_delivery_project_readback", path)
    if spec is None or spec.loader is None:
        raise ValueError("canonical Project task resolver cannot be loaded")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    sys.path.insert(0, str(path.parent))
    try:
        spec.loader.exec_module(module)
    except (OSError, ImportError, ValueError) as exc:
        raise ValueError("canonical Project task resolver failed to load") from exc
    finally:
        sys.path.pop(0)
    _PROJECT_READBACK_MODULE = module
    return module


def _resolve_project_task_issue(task_uid: str, expected_issue_number: int | None = None) -> dict[str, Any]:
    reader = _project_readback()
    selected = reader.GitHubReadOnly().resolve_project_task_issue(task_uid)
    number = selected.get("number") if isinstance(selected, dict) else None
    if not _positive_int(number) or (expected_issue_number is not None and number != expected_issue_number):
        raise ValueError("Project-backed Task Issue does not match the bound Issue number")
    live = _gh_json(f"repos/{REPOSITORY}/issues/{number}")
    if (not isinstance(live, dict) or live.get("number") != number
            or live.get("url") != selected.get("url") or live.get("body") != selected.get("body")
            or reader._live_task_uid(live, number) != task_uid
            or reader._live_task_pr(live, task_uid) != reader._live_task_pr(selected, task_uid)):
        raise ValueError("live REST Task Issue identity differs from its canonical Project record")
    return live


def _current_pr_process_waiver(repository: str, task: dict[str, Any], pr_number: int,
                               head_oid: str, head_branch: str) -> dict[str, Any]:
    issue_number = task.get("issue_number")
    task_uid = str(task.get("task_uid") or "")
    if type(issue_number) is str and re.fullmatch(r"[1-9][0-9]*", issue_number):
        issue_number = int(issue_number)
    if not _positive_int(issue_number) or not TASK_UID_RE.fullmatch(task_uid):
        raise ValueError("bound Task Issue identity is incomplete for process exception readback")
    issue = _gh_json(f"repos/{repository}/issues/{issue_number}")
    body = issue.get("body") if isinstance(issue, dict) else None
    if (not isinstance(issue, dict) or issue.get("number") != issue_number
            or "pull_request" in issue or not isinstance(body, str)
            or _canonical_uid(body) != task_uid):
        raise ValueError("live Task Issue identity is invalid for process exception readback")
    comments = _pages(f"repos/{repository}/issues/{issue_number}/comments", None)
    verified: list[dict[str, Any]] = []

    def validate_replacement(evidence: dict[str, Any], _record: dict[str, Any]) -> bool:
        try:
            verified.append(_verify_declared_ci(
                repository, evidence, pr_number, head_oid, head_branch,
                allow_empty_pr_association=True,
            ))
            return True
        except (OSError, ValueError):
            return False

    result = PROCESS_EXCEPTIONS.resolve_process_exception(
        comments,
        repository=repository,
        issue_number=issue_number,
        task_uid=task_uid,
        action="current_pr_validation",
        head_oid=head_oid,
        scope="required-gate",
        live_admin_by_login=lambda login: _is_admin(repository, login),
        replacement_validator=validate_replacement,
        process_check_waivable=True,
    )
    if result.get("applicable") is True and verified:
        result["verified_replacement_ci"] = verified[-1]
    return result


def read_current_pr_projection(root: pathlib.Path, task: dict[str, Any]) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    """Expose local/remote source and latest required-gate identity without mutation."""
    local_head = _git_value(root, "rev-parse", "HEAD")
    projection: dict[str, Any] = {
        "local_candidate_head_oid": local_head or None,
        "remote_pr_head_oid": None,
        "required_ci": None,
        "failure_phase": None,
    }
    blockers: list[dict[str, Any]] = []
    repository = task.get("repository")
    issue_number = task.get("issue_number")
    pr_number = task.get("pr_number")
    if type(pr_number) is str and re.fullmatch(r"[1-9][0-9]*", pr_number):
        pr_number = int(pr_number)
    if (repository != REPOSITORY or not isinstance(repository, str)
            or type(pr_number) is not int or pr_number < 1):
        return projection, blockers
    try:
        pr = _gh_json(f"repos/{repository}/pulls/{pr_number}")
        body = str(pr.get("body") or "") if isinstance(pr, dict) else ""
        head_oid = ((pr.get("head") or {}).get("sha")) if isinstance(pr, dict) else None
        task_uid = str(task.get("task_uid") or "")
        exact_task = bool(re.search(rf"(?m)^Task: {re.escape(task_uid)}$", body))
        exact_issue = bool(re.search(rf"(?m)^Refs #{issue_number}$", body))
        if (not isinstance(pr, dict) or pr.get("number") != pr_number
                or not exact_task or not exact_issue
                or not isinstance(head_oid, str) or not re.fullmatch(r"[0-9a-f]{40,64}", head_oid)):
            raise ValueError("live PR identity/head does not match task mapping")
        projection["remote_pr_head_oid"] = head_oid
        if local_head and head_oid != local_head:
            blocked, allowed, next_kind = BLOCKER_POLICY["SOURCE_NOT_PUBLISHED"]
            blockers.append(_action_blocker(
                "SOURCE_NOT_PUBLISHED",
                "the local candidate head differs from the live PR head; current validation applies only to the remote head",
                blocks_actions=list(blocked), allowed_actions=list(allowed), next_action_kind=next_kind,
            ))
        checks = _pages(f"repos/{repository}/commits/{head_oid}/check-runs", "check_runs")
        matching = []
        for row in checks:
            if (not isinstance(row, dict) or row.get("name") != "required-gate"
                    or (row.get("app") or {}).get("id") != REQUIRED_GATE_APP_ID
                    or row.get("head_sha") != head_oid):
                continue
            try:
                associations = _associated_pr_numbers(row.get("pull_requests"), repository)
            except ValueError:
                continue
            if associations and all(number == pr_number for number in associations):
                matching.append(row)
        if not matching:
            exception = _current_pr_process_waiver(repository, task, pr_number, head_oid,
                                                  str(((pr.get("head") or {}).get("ref")) or ""))
            if exception.get("applicable") is not True:
                raise ValueError("current remote head has no unambiguous required-gate check associated with this PR")
            projection["process_exception"] = exception
            projection["replacement_ci_identity"] = exception["verified_replacement_ci"]
            return projection, blockers
        check = max(matching, key=lambda row: row.get("id") if type(row.get("id")) is int else 0)
        details = str(check.get("details_url") or "")
        match = re.fullmatch(rf"https://github\.com/{re.escape(repository)}/actions/runs/([1-9][0-9]*)/job/([1-9][0-9]*)", details)
        if not match:
            raise ValueError("required-gate check run locator is malformed")
        run_id = int(match.group(1))
        run = _gh_json(f"repos/{repository}/actions/runs/{run_id}")
        attempt = run.get("run_attempt") if isinstance(run, dict) else None
        if (check.get("head_sha") != head_oid or not isinstance(run, dict)
                or run.get("id") != run_id or run.get("head_sha") != head_oid
                or run.get("head_branch") != ((pr.get("head") or {}).get("ref"))
                or run.get("path") != ".github/workflows/rust.yml"
                or run.get("event") != "pull_request"
                or type(attempt) is not int or attempt < 1):
            raise ValueError("required-gate workflow run identity is uncertain")
        run_association_rows = run.get("pull_requests")
        if not isinstance(run_association_rows, list):
            raise ValueError("required-gate workflow run PR association is malformed")
        run_associations = _associated_pr_numbers(run_association_rows, repository) if run_association_rows else []
        association_exception = None
        if run_associations and any(number != pr_number for number in run_associations):
            raise ValueError("required-gate workflow run has conflicting PR association")
        if not run_associations:
            association_exception = _current_pr_process_waiver(
                repository, task, pr_number, head_oid,
                str(((pr.get("head") or {}).get("ref")) or ""),
            )
            if association_exception.get("applicable") is not True:
                raise ValueError("required-gate workflow run has empty or conflicting PR association")
            projection["process_exception"] = association_exception
            projection["replacement_ci_identity"] = association_exception["verified_replacement_ci"]
        jobs = _pages(f"repos/{repository}/actions/runs/{run_id}/jobs", "jobs")
        required_check_url = f"https://api.github.com/repos/{repository}/check-runs/{check.get('id')}"
        expected_job_id = int(match.group(2))
        matching_jobs = [job for job in jobs if isinstance(job, dict)
                         and job.get("id") == expected_job_id
                         and job.get("name") == "required-gate"
                         and job.get("run_attempt") == attempt
                         and job.get("check_run_url") == required_check_url]
        if len(matching_jobs) != 1:
            raise ValueError("current required-gate job is missing or ambiguous for this run attempt")
        job = matching_jobs[0]
        projection["required_ci"] = {
            "check_name": "required-gate",
            "check_app_id": REQUIRED_GATE_APP_ID,
            "check_run_id": check.get("id"),
            "run_id": run_id,
            "run_attempt": attempt,
            "event": run.get("event"),
            "workflow_path": run.get("path"),
            "head_oid": check.get("head_sha"),
            "status": check.get("status"),
            "conclusion": check.get("conclusion"),
            "job_name": job.get("name"),
            "job_status": job.get("status"),
            "job_conclusion": job.get("conclusion"),
        }
        if (check.get("status") != "completed"
                or str(check.get("conclusion") or "").lower() != "success"
                or job.get("status") != "completed"
                or str(job.get("conclusion") or "").lower() != "success"):
            exception = _current_pr_process_waiver(repository, task, pr_number, head_oid,
                                                  str(((pr.get("head") or {}).get("ref")) or ""))
            if exception.get("applicable") is True:
                projection["process_exception"] = exception
                projection["replacement_ci_identity"] = exception["verified_replacement_ci"]
            else:
                blocked, allowed, next_kind = BLOCKER_POLICY["CURRENT_CHECK_FAILED"]
                blockers.append(_action_blocker(
                    "CURRENT_CHECK_FAILED",
                    "the latest required-gate check on the live PR head is pending or unsuccessful",
                    blocks_actions=list(blocked), allowed_actions=list(allowed), next_action_kind=next_kind,
                ))
            failures = [item for item in jobs if isinstance(item, dict)
                        and str(item.get("conclusion") or "").lower() not in {"", "success", "skipped"}]
            if failures:
                failed = failures[0]
                steps = failed.get("steps") if isinstance(failed.get("steps"), list) else []
                failed_steps = [step for step in steps if isinstance(step, dict)
                                and str(step.get("conclusion") or "").lower() not in {"", "success", "skipped"}]
                projection["failure_phase"] = {
                    "job": failed.get("name"),
                    "step": failed_steps[0].get("name") if failed_steps else None,
                }
    except (OSError, ValueError) as exc:
        projection["read_status"] = "uncertain"
        projection["read_error"] = str(exc)
        blocked, allowed, next_kind = BLOCKER_POLICY["CURRENT_CHECK_FAILED"]
        blockers.append(_action_blocker(
            "CURRENT_CHECK_FAILED",
            f"live PR/check identity could not be verified: {exc}",
            blocks_actions=list(blocked), allowed_actions=list(allowed), next_action_kind=next_kind,
        ))
    return projection, blockers


def workflow_projection(root: pathlib.Path, task: dict[str, Any], legacy_blockers: list[str]) -> dict[str, Any]:
    """Build the additive delivery projection without changing legacy lifecycle gates."""
    uid = str(task.get("task_uid") or "")
    delivery = derive_delivery_readiness(uid, {})
    try:
        delivery = derive_delivery_readiness(uid, read_explicit_edge(root, task))
    except (OSError, ValueError) as exc:
        delivery = derive_delivery_readiness(uid, {})
        delivery["delivery_ready"] = False
        delivery["cleanup_state"] = "not_applicable"
        delivery["action_blockers"] = project_action_blockers(
            [f"stale identity: delivery edge readback failed ({exc})"]
        )

    pull_request, pr_blockers = read_current_pr_projection(root, task)
    action_blockers = [*project_action_blockers(legacy_blockers),
                       *delivery.get("action_blockers", []), *pr_blockers]
    return {
        "delivery_ready": delivery.get("delivery_ready"),
        "cleanup_state": delivery.get("cleanup_state", "not_applicable"),
        "action_blockers": action_blockers,
        "candidate_head_oid": pull_request.get("local_candidate_head_oid"),
        "remote_pr_head_oid": pull_request.get("remote_pr_head_oid"),
        "ci_identity": pull_request.get("required_ci"),
        "process_exception": pull_request.get("process_exception"),
        "replacement_ci_identity": pull_request.get("replacement_ci_identity"),
        "failure_phase": pull_request.get("failure_phase"),
    }
