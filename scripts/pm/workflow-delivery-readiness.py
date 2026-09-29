#!/usr/bin/env python3
"""Read-only readiness projection for the explicitly declared fast-recovery edge.

This helper recognizes only the A (#4139) artifact consumed by B (#4095).
It deliberately does not infer artifact edges for legacy or unrelated tasks.
"""
from __future__ import annotations

import hashlib
import json
import pathlib
import re
import subprocess
from typing import Any


REPOSITORY = "eng-cc/oasis7"
DOWNSTREAM_UID = "task_7d9db27bcc2b4359b02fdf3bff5a609e"
UPSTREAM_UID = "task_5fe52e7477774b10af6655ad298eedd5"
COORDINATOR_ISSUE = 4082
EDGE_COMMENT_ID = 5885353387
DOWNSTREAM_ISSUE = 4095
UPSTREAM_ISSUE = 4137
UPSTREAM_PR = 4139
EDGE_MERGE_OID = "52d86940cad7d6ad0b67dc1e931e350882ef5eb5"
REVIEW_COMMENT_ID = 5884285458
REVIEW_DISPATCH_COMMENT_ID = 5883923527
STRICT_RUN_ID = 36523710522
SOURCE_HEAD_OID = "f8b65ffa9b9a61262936ce37e52b7f7e6ea978ab"
STRICT_BASE_OID = "917f7172e856fc6a3a2523bccd5a5b9b13dcfa29"
REQUIRED_GATE_APP_ID = 15368
HANDOFF_CLOSURE_PATHS = {
    "scripts/pm/record-pre-pr-review.sh",
    "scripts/pm/review-batch-epoch.py",
    "scripts/pm/review-closeout.sh",
    "scripts/pm/review-findings-resolution.py",
    "scripts/pm/review_preflight_handoff.py",
}
SOURCE_PATH = "doc/engineering/workflow/source-of-truth.md"
STRICT_RUN_TITLE = (
    "oasis7-ci|workflow_dispatch|integration_revalidation|"
    f"{UPSTREAM_UID}|{UPSTREAM_PR}|{STRICT_BASE_OID}|{SOURCE_HEAD_OID}"
)
EDGE_COMMENT_BODY = (
    "Explicit artifact dependency declaration for fast-recovery delivery consumption: "
    f"downstream B task #{DOWNSTREAM_ISSUE} UID {DOWNSTREAM_UID} consumes upstream A "
    f"#{UPSTREAM_ISSUE} / merged PR #{UPSTREAM_PR} commit {EDGE_MERGE_OID} as the exact "
    "named source input doc/engineering/workflow/source-of-truth.md plus compatible "
    "review-handoff tool closure from that commit. This edge is artifact-only: it requires "
    "independent live merged identity, applicable review/CI and hold/acceptance verification "
    "under the current source contract; A worktree removal is not a prerequisite for B source "
    "consumption. It grants no generic artifact dependency for unrelated UIDs and does not "
    "relax any resource/environment edge. The subsequent edge from B to original #3971/#3972 "
    "may consume only B merged trusted retirement tools after B review/CI/readback; actual "
    "mapping apply and #3972 candidate validation remain separate actions. Existing "
    "legacy/unclassified dependencies keep terminal semantics. This records the edge already "
    "specified by the user design and previous ordering, not a second state ledger or new authority."
)


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
    """Evaluate the one exact A-to-B edge from independently loaded live facts.

    ``proof`` is an internal, normalized result of ``read_explicit_edge``. The CLI
    never accepts caller-authored proof JSON.
    """
    if task_uid != DOWNSTREAM_UID:
        return {
            "delivery_ready": None,
            "cleanup_state": "not_applicable",
            "action_blockers": [],
            "dependency": "legacy_or_unclassified_terminal_semantics",
        }

    reasons: list[tuple[str, str]] = []
    edge = proof.get("edge") if isinstance(proof, dict) else None
    edge_authority = proof.get("edge_authority") if isinstance(proof, dict) else None
    downstream = proof.get("downstream") if isinstance(proof, dict) else None
    upstream = proof.get("upstream_issue") if isinstance(proof, dict) else None
    pr = proof.get("pr") if isinstance(proof, dict) else None
    review = proof.get("review") if isinstance(proof, dict) else None
    source_ci = proof.get("source_ci") if isinstance(proof, dict) else None
    strict_ci = proof.get("strict_ci") if isinstance(proof, dict) else None
    local_input = proof.get("local_input") if isinstance(proof, dict) else None

    if not isinstance(edge, dict) or not (
        edge.get("coordinator_issue") == COORDINATOR_ISSUE
        and edge.get("comment_id") == EDGE_COMMENT_ID
        and edge.get("downstream_issue") == DOWNSTREAM_ISSUE
        and edge.get("downstream_task_uid") == DOWNSTREAM_UID
        and edge.get("upstream_issue") == UPSTREAM_ISSUE
        and edge.get("upstream_task_uid") == UPSTREAM_UID
        and edge.get("upstream_pr") == UPSTREAM_PR
        and edge.get("merge_commit_oid") == EDGE_MERGE_OID
        and edge.get("required_source_path") == SOURCE_PATH
    ):
        reasons.append(("TASK_BINDING_CONFLICT", "the live coordinator edge does not match the exact approved A-to-B artifact binding"))
    if not isinstance(edge_authority, dict) or not (
        edge_authority.get("comment_id") == EDGE_COMMENT_ID
        and edge_authority.get("author") == "eng-cc"
        and edge_authority.get("permission") == "admin"
        and edge_authority.get("body_valid") is True
    ):
        reasons.append(("TASK_BINDING_CONFLICT", "the exact artifact-edge declaration lacks current live admin-author authority"))
    if not isinstance(downstream, dict) or not (
        downstream.get("issue_number") == DOWNSTREAM_ISSUE
        and downstream.get("task_uid") == DOWNSTREAM_UID
        and downstream.get("input_commit_oid") == EDGE_MERGE_OID
    ):
        reasons.append(("TASK_BINDING_CONFLICT", "the downstream task does not bind the exact named source commit"))
    if not isinstance(downstream, dict) or downstream.get("input_files_match") is not True:
        reasons.append(("SOURCE_NOT_PUBLISHED", "the downstream worktree does not contain the exact named source and review-handoff tool closure"))
    if not isinstance(upstream, dict) or not (
        upstream.get("issue_number") == UPSTREAM_ISSUE
        and upstream.get("task_uid") == UPSTREAM_UID
        and upstream.get("status") == "done"
        and upstream.get("workflow_phase") in {"task_done", "main_sync", "post_merge_done"}
        and str(upstream.get("pr_number") or "") == str(UPSTREAM_PR)
    ):
        reasons.append(("TASK_BINDING_CONFLICT", "the live upstream task identity or delivered phase is incomplete"))
    if not isinstance(upstream, dict) or upstream.get("merge_hold_active") is not False:
        reasons.append(("TASK_BINDING_CONFLICT", "the upstream task has an active or uncertain merge/acceptance hold"))
    if not isinstance(pr, dict) or not (
        pr.get("repository") == REPOSITORY
        and pr.get("number") == UPSTREAM_PR
        and pr.get("issue_number") == UPSTREAM_ISSUE
        and pr.get("task_uid") == UPSTREAM_UID
        and str(pr.get("state") or "").lower() == "closed"
        and pr.get("merged") is True
        and pr.get("merge_commit_oid") == EDGE_MERGE_OID
        and pr.get("base_ref") == "main"
        and isinstance(pr.get("head_oid"), str)
        and re.fullmatch(r"[0-9a-f]{40,64}", pr["head_oid"])
    ):
        reasons.append(("TASK_BINDING_CONFLICT", "live merged PR identity or merge commit differs from the declared artifact"))
    if not isinstance(review, dict) or not (
        review.get("comment_id") == REVIEW_COMMENT_ID
        and review.get("task_uid") == UPSTREAM_UID
        and review.get("source_head_oid") == SOURCE_HEAD_OID
        and review.get("author") == "eng-cc"
        and review.get("passed") is True
        and review.get("findings_disposition") == "addressed"
        and set(review.get("roles") or []) >= {
            "producer_system_designer", "repository_health_engineer", "qa_engineer",
        }
    ):
        reasons.append(("REVIEW_FINDING_BLOCKING", "current source review is missing, stale, incomplete or unresolved"))
    if not isinstance(source_ci, dict) or not (
        source_ci.get("check_name") == "required-gate"
        and source_ci.get("app_id") == REQUIRED_GATE_APP_ID
        and _ci_ok(source_ci, head_oid=SOURCE_HEAD_OID)
    ):
        reasons.append(("CURRENT_CHECK_FAILED", "the current required-gate check for the merged source head is missing or unsuccessful"))
    if not isinstance(strict_ci, dict) or not (
        strict_ci.get("run_id") == STRICT_RUN_ID
        and strict_ci.get("event") == "workflow_dispatch"
        and strict_ci.get("workflow_path") == ".github/workflows/rust.yml"
        and strict_ci.get("head_branch") == "main"
        and strict_ci.get("head_oid") == STRICT_BASE_OID
        and strict_ci.get("status") == "completed"
        and str(strict_ci.get("conclusion") or "").lower() == "success"
        and strict_ci.get("display_title") == STRICT_RUN_TITLE
        and isinstance(strict_ci.get("run_attempt"), int)
        and strict_ci.get("run_attempt", 0) > 0
        and isinstance(strict_ci.get("required_gate_job"), dict)
        and strict_ci["required_gate_job"].get("name") == "required-gate"
        and strict_ci["required_gate_job"].get("status") == "completed"
        and str(strict_ci["required_gate_job"].get("conclusion") or "").lower() == "success"
        and strict_ci["required_gate_job"].get("app_id") == REQUIRED_GATE_APP_ID
        and strict_ci["required_gate_job"].get("run_attempt") == strict_ci.get("run_attempt")
    ):
        reasons.append(("CURRENT_CHECK_FAILED", "the exact trusted current-target integration run or required-gate job is missing or unsuccessful"))
    if not isinstance(local_input, dict) or not (
        local_input.get("repository") == REPOSITORY
        and local_input.get("head_contains_merge_commit") is True
        and local_input.get("source_path_present") is True
        and local_input.get("source_path_matches_merge_commit") is True
        and local_input.get("review_handoff_closure_matches_merge_commit") is True
    ):
        reasons.append(("SOURCE_NOT_PUBLISHED", "the downstream repository does not consume the exact merged source and helper closure"))

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
        "dependency": "4082:A4139-to-B4095:artifact",
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


def _identity_comment(comment_id: int) -> dict[str, Any]:
    value = _gh_json(f"repos/{REPOSITORY}/issues/comments/{comment_id}")
    if not isinstance(value, dict) or type(value.get("id")) is not int:
        raise ValueError(f"GitHub comment readback is malformed: {comment_id}")
    return value


def _workflow_run(run_id: int, *, expected_head: str, expected_event: str,
                  expected_attempt: int | None = None) -> tuple[dict[str, Any], dict[str, Any]]:
    run = _gh_json(f"repos/{REPOSITORY}/actions/runs/{run_id}")
    if not isinstance(run, dict) or run.get("id") != run_id:
        raise ValueError(f"workflow run identity is malformed: {run_id}")
    if (run.get("path") != ".github/workflows/rust.yml"
            or run.get("head_sha") != expected_head
            or run.get("event") != expected_event
            or run.get("status") != "completed"
            or str(run.get("conclusion") or "").lower() != "success"):
        raise ValueError(f"workflow run identity or conclusion is not successful: {run_id}")
    attempt = run.get("run_attempt")
    if type(attempt) is not int or attempt < 1 or (expected_attempt is not None and attempt != expected_attempt):
        raise ValueError(f"workflow run attempt is invalid: {run_id}")
    jobs = _pages(f"repos/{REPOSITORY}/actions/runs/{run_id}/jobs", "jobs")
    return run, {"jobs": jobs, "run_attempt": attempt}


def _check_for_pr_head(pr_number: int, head_oid: str) -> dict[str, Any]:
    checks = _pages(f"repos/{REPOSITORY}/commits/{head_oid}/check-runs", "check_runs")
    matches = [item for item in checks if isinstance(item, dict)
               and item.get("name") == "required-gate"
               and (item.get("app") or {}).get("id") == REQUIRED_GATE_APP_ID]
    if not matches:
        raise ValueError("exact source-head required-gate check is missing")
    check = max(matches, key=lambda row: row.get("id") if type(row.get("id")) is int else 0)
    if (check.get("head_sha") != head_oid or check.get("status") != "completed"
            or str(check.get("conclusion") or "").lower() != "success"):
        raise ValueError("exact source-head required-gate check is not successful")
    details = str(check.get("details_url") or "")
    match = re.fullmatch(rf"https://github\.com/{re.escape(REPOSITORY)}/actions/runs/([1-9][0-9]*)/job/[1-9][0-9]*", details)
    if not match:
        raise ValueError("required-gate check does not identify a trusted workflow job")
    run_id = int(match.group(1))
    run, attempt_payload = _workflow_run(run_id, expected_head=head_oid, expected_event="pull_request")
    jobs = [item for item in attempt_payload["jobs"] if isinstance(item, dict)
            and item.get("name") == "required-gate"
            and item.get("run_attempt") == attempt_payload["run_attempt"]
            and str(item.get("check_run_url") or "").endswith(f"/check-runs/{check['id']}")]
    if len(jobs) != 1 or jobs[0].get("status") != "completed" or jobs[0].get("conclusion") != "success":
        raise ValueError("required-gate workflow job is missing, ambiguous, or unsuccessful")
    return {
        "check_name": "required-gate",
        "app_id": REQUIRED_GATE_APP_ID,
        "head_oid": head_oid,
        "status": check.get("status"),
        "conclusion": check.get("conclusion"),
        "check_run_id": check.get("id"),
        "run_id": run_id,
        "run_attempt": attempt_payload["run_attempt"],
    }


def _source_files_match(root: pathlib.Path, merge_oid: str, files: list[dict[str, Any]]) -> tuple[bool, list[dict[str, str]]]:
    names: list[str] = []
    seen: set[str] = set()
    for row in files:
        if not isinstance(row, dict) or not isinstance(row.get("filename"), str):
            raise ValueError("merged PR file list contains malformed entry")
        name = row["filename"]
        path = pathlib.PurePosixPath(name)
        if path.is_absolute() or not name or any(part in {"", ".", ".."} for part in path.parts):
            raise ValueError("merged PR file list contains unsafe repository path")
        if name in seen:
            raise ValueError("merged PR file list contains duplicate path")
        seen.add(name)
        names.append(name)
    if SOURCE_PATH not in seen or not HANDOFF_CLOSURE_PATHS <= seen:
        return False, []
    exact: list[dict[str, str]] = []
    for name in sorted(names):
        path = root.joinpath(*pathlib.PurePosixPath(name).parts)
        if not path.is_file():
            return False, exact
        try:
            expected = subprocess.check_output(
                ["git", "-C", str(root), "show", f"{merge_oid}:{name}"],
                stderr=subprocess.DEVNULL,
            )
            actual = path.read_bytes()
        except (OSError, subprocess.CalledProcessError):
            return False, exact
        if actual != expected:
            return False, exact
        exact.append({"path": name, "sha256": hashlib.sha256(actual).hexdigest()})
    return True, exact


def read_explicit_edge(root: pathlib.Path, task_uid: str) -> dict[str, Any]:
    """Read and validate current GitHub/working-tree proof for the one A-to-B edge."""
    if task_uid != DOWNSTREAM_UID:
        return {}
    edge_comment = _identity_comment(EDGE_COMMENT_ID)
    permission = _gh_json(f"repos/{REPOSITORY}/collaborators/{edge_comment.get('user', {}).get('login')}/permission")
    edge_text = edge_comment.get("body")
    if (edge_comment.get("issue_url") != f"https://api.github.com/repos/{REPOSITORY}/issues/{COORDINATOR_ISSUE}"
            or edge_text != EDGE_COMMENT_BODY
            or (edge_comment.get("user") or {}).get("login") != "eng-cc"
            or not isinstance(permission, dict) or permission.get("permission") != "admin"):
        raise ValueError("explicit artifact edge comment or current admin authority is invalid")

    downstream_issue = _gh_json(f"repos/{REPOSITORY}/issues/{DOWNSTREAM_ISSUE}")
    downstream_body = downstream_issue.get("body") if isinstance(downstream_issue, dict) else None
    if (not isinstance(downstream_issue, dict) or downstream_issue.get("number") != DOWNSTREAM_ISSUE
            or not isinstance(downstream_body, str) or _canonical_uid(downstream_body) != DOWNSTREAM_UID):
        raise ValueError("downstream task Issue does not bind the authorized UID")

    upstream_issue = _gh_json(f"repos/{REPOSITORY}/issues/{UPSTREAM_ISSUE}")
    upstream_body = upstream_issue.get("body") if isinstance(upstream_issue, dict) else None
    if (not isinstance(upstream_issue, dict) or upstream_issue.get("number") != UPSTREAM_ISSUE
            or not isinstance(upstream_body, str) or _canonical_uid(upstream_body) != UPSTREAM_UID):
        raise ValueError("upstream task Issue does not bind the authorized UID")

    pr = _gh_json(f"repos/{REPOSITORY}/pulls/{UPSTREAM_PR}")
    if not isinstance(pr, dict):
        raise ValueError("upstream PR read is malformed")
    pr_body = str(pr.get("body") or "")
    pr_identity = {
        "repository": REPOSITORY,
        "number": pr.get("number"),
        "issue_number": UPSTREAM_ISSUE,
        "task_uid": UPSTREAM_UID,
        "state": pr.get("state"),
        "merged": pr.get("merged"),
        "merge_commit_oid": pr.get("merge_commit_sha"),
        "head_oid": ((pr.get("head") or {}).get("sha")),
        "base_ref": ((pr.get("base") or {}).get("ref")),
    }
    if f"Task: {UPSTREAM_UID}" not in pr_body or f"Refs #{UPSTREAM_ISSUE}" not in pr_body:
        raise ValueError("upstream PR does not bind the exact task Issue")

    review_comment = _identity_comment(REVIEW_COMMENT_ID)
    review_text = str(review_comment.get("body") or "")
    role_line = next((line for line in review_text.splitlines() if line.startswith("- Review Roles: ")), "")
    roles = [role.strip() for role in role_line.removeprefix("- Review Roles: ").split(",")] if role_line else []
    review = {
        "comment_id": review_comment.get("id"),
        "task_uid": _body_field(review_text, "Task UID"),
        "source_head_oid": _body_field(review_text, "Source Head"),
        "author": (review_comment.get("user") or {}).get("login"),
        "passed": "- Pre-PR Local Role Review: passed" in review_text,
        "roles": roles,
        "findings_disposition": _body_field(review_text, "Review Findings Disposition"),
    }
    if review_comment.get("issue_url") != f"https://api.github.com/repos/{REPOSITORY}/issues/{UPSTREAM_ISSUE}":
        raise ValueError("source review packet is published on the wrong Issue")

    dispatch_comment = _identity_comment(REVIEW_DISPATCH_COMMENT_ID)
    dispatch_text = str(dispatch_comment.get("body") or "")
    if (dispatch_comment.get("issue_url") != f"https://api.github.com/repos/{REPOSITORY}/issues/{UPSTREAM_ISSUE}"
            or UPSTREAM_UID not in dispatch_text
            or f"PR # {UPSTREAM_PR}" not in dispatch_text and f"PR #{UPSTREAM_PR}" not in dispatch_text
            or f"main@{STRICT_BASE_OID}..{SOURCE_HEAD_OID}" not in dispatch_text
            or f"strict run {STRICT_RUN_ID} pending" not in dispatch_text):
        raise ValueError("source review/strict-CI dispatch identity is not exact")

    source_ci = _check_for_pr_head(UPSTREAM_PR, SOURCE_HEAD_OID)
    strict_run = _gh_json(f"repos/{REPOSITORY}/actions/runs/{STRICT_RUN_ID}")
    strict_jobs_payload = _pages(f"repos/{REPOSITORY}/actions/runs/{STRICT_RUN_ID}/jobs", "jobs")
    strict_matches = [job for job in strict_jobs_payload if isinstance(job, dict) and job.get("name") == "required-gate"]
    strict_job: dict[str, Any] = {}
    if len(strict_matches) == 1:
        raw_job = strict_matches[0]
        check_run_url = str(raw_job.get("check_run_url") or "")
        match = re.fullmatch(rf"https://api\.github\.com/repos/{re.escape(REPOSITORY)}/check-runs/([1-9][0-9]*)", check_run_url)
        if match:
            check = _gh_json(f"repos/{REPOSITORY}/check-runs/{match.group(1)}")
            strict_job = {
                "name": raw_job.get("name"),
                "status": raw_job.get("status"),
                "conclusion": raw_job.get("conclusion"),
                "app_id": ((check.get("app") or {}).get("id")) if isinstance(check, dict) else None,
                "run_attempt": raw_job.get("run_attempt"),
            }
    strict_ci = {
        key: strict_run.get(key) for key in (
            "id", "run_attempt", "event", "path", "head_branch", "head_sha", "status", "conclusion", "display_title",
        )
    } if isinstance(strict_run, dict) else {}
    strict_ci["run_id"] = strict_ci.pop("id", None)
    strict_ci["workflow_path"] = strict_ci.pop("path", None)
    strict_ci["head_oid"] = strict_ci.pop("head_sha", None)
    strict_ci["required_gate_job"] = strict_job

    files = _pages(f"repos/{REPOSITORY}/pulls/{UPSTREAM_PR}/files", None)
    files_match, file_digests = _source_files_match(root, EDGE_MERGE_OID, files)
    local_head = _git_value(root, "rev-parse", "HEAD")
    ancestry = subprocess.run(
        ["git", "-C", str(root), "merge-base", "--is-ancestor", EDGE_MERGE_OID, "HEAD"],
        capture_output=True,
        check=False,
    ).returncode == 0
    local_input = {
        "repository": REPOSITORY if _repository_identity(root) == REPOSITORY else None,
        "head_oid": local_head,
        "head_contains_merge_commit": ancestry,
        "source_path_present": any(item.get("path") == SOURCE_PATH for item in file_digests),
        "source_path_matches_merge_commit": any(item.get("path") == SOURCE_PATH for item in file_digests),
        "review_handoff_closure_matches_merge_commit": HANDOFF_CLOSURE_PATHS <= {
            item.get("path") for item in file_digests
        },
        "file_digests": file_digests,
    }
    hold_value = _body_field(upstream_body, "merge_hold_active")
    merge_hold_active = True if hold_value == "true" else False if hold_value == "false" else None
    upstream_identity = {
        "issue_number": upstream_issue.get("number"),
        "task_uid": _canonical_uid(upstream_body),
        "status": _body_field(upstream_body, "status"),
        "workflow_phase": _body_field(upstream_body, "workflow_phase"),
        "pr_number": _body_field(upstream_body, "pr_number"),
        "merge_hold_active": merge_hold_active,
    }
    proof = {
        "edge": {
            "coordinator_issue": COORDINATOR_ISSUE,
            "comment_id": edge_comment.get("id"),
            "downstream_issue": downstream_issue.get("number"),
            "downstream_task_uid": _canonical_uid(downstream_body),
            "upstream_issue": UPSTREAM_ISSUE,
            "upstream_task_uid": _canonical_uid(upstream_body),
            "upstream_pr": pr_identity["number"],
            "merge_commit_oid": pr_identity["merge_commit_oid"],
            "required_source_path": SOURCE_PATH,
        },
        "edge_authority": {
            "comment_id": edge_comment.get("id"),
            "author": (edge_comment.get("user") or {}).get("login"),
            "permission": permission.get("permission") if isinstance(permission, dict) else None,
            "body_valid": edge_text == EDGE_COMMENT_BODY,
        },
        "downstream": {
            "issue_number": downstream_issue.get("number"),
            "task_uid": _canonical_uid(downstream_body),
            "input_commit_oid": EDGE_MERGE_OID if ancestry else None,
            "input_files_match": files_match and ancestry and local_input["repository"] == REPOSITORY,
        },
        "upstream_issue": upstream_identity,
        "pr": pr_identity,
        "review": review,
        "source_ci": source_ci,
        "strict_ci": strict_ci,
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
        if (not isinstance(pr, dict) or pr.get("number") != pr_number
                or f"Task: {task.get('task_uid')}" not in body
                or f"Refs #{issue_number}" not in body
                or not isinstance(head_oid, str) or not re.fullmatch(r"[0-9a-f]{40,64}", head_oid)):
            raise ValueError("live PR identity/head does not match task mapping")
        projection["remote_pr_head_oid"] = head_oid
        checks = _pages(f"repos/{repository}/commits/{head_oid}/check-runs", "check_runs")
        matching = [row for row in checks if isinstance(row, dict)
                    and row.get("name") == "required-gate"
                    and (row.get("app") or {}).get("id") == REQUIRED_GATE_APP_ID
                    and any(isinstance(link, dict) and link.get("number") == pr_number
                            for link in (row.get("pull_requests") or []))]
        if not matching:
            raise ValueError("current remote head has no required-gate check")
        check = max(matching, key=lambda row: row.get("id") if type(row.get("id")) is int else 0)
        details = str(check.get("details_url") or "")
        match = re.fullmatch(rf"https://github\.com/{re.escape(repository)}/actions/runs/([1-9][0-9]*)/job/[1-9][0-9]*", details)
        if not match:
            raise ValueError("required-gate check run locator is malformed")
        run_id = int(match.group(1))
        run = _gh_json(f"repos/{repository}/actions/runs/{run_id}")
        attempt = run.get("run_attempt") if isinstance(run, dict) else None
        if (check.get("head_sha") != head_oid or not isinstance(run, dict)
                or run.get("id") != run_id or run.get("head_sha") != head_oid
                or run.get("path") != ".github/workflows/rust.yml"
                or run.get("event") != "pull_request"
                or type(attempt) is not int or attempt < 1):
            raise ValueError("required-gate workflow run identity is uncertain")
        jobs = _pages(f"repos/{repository}/actions/runs/{run_id}/jobs", "jobs")
        required_check_url = f"https://api.github.com/repos/{repository}/check-runs/{check.get('id')}"
        matching_jobs = [job for job in jobs if isinstance(job, dict)
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
        if (local_head and head_oid != local_head):
            blocked, allowed, next_kind = BLOCKER_POLICY["SOURCE_NOT_PUBLISHED"]
            blockers.append(_action_blocker(
                "SOURCE_NOT_PUBLISHED",
                "the local candidate head differs from the live PR head; current validation applies only to the remote head",
                blocks_actions=list(blocked), allowed_actions=list(allowed), next_action_kind=next_kind,
            ))
        if (check.get("status") != "completed"
                or str(check.get("conclusion") or "").lower() != "success"
                or job.get("status") != "completed"
                or str(job.get("conclusion") or "").lower() != "success"):
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
    """Build the additive B projection without changing legacy lifecycle gates."""
    uid = str(task.get("task_uid") or "")
    delivery = derive_delivery_readiness(uid, {})
    if uid == DOWNSTREAM_UID:
        try:
            delivery = derive_delivery_readiness(uid, read_explicit_edge(root, uid))
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
        "failure_phase": pull_request.get("failure_phase"),
    }
