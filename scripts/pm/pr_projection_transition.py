#!/usr/bin/env python3
"""Pure reachability checks for the C1 record-pr Issue/Project transition."""
from __future__ import annotations

from typing import Any


TRANSITION_SCHEMA = "oasis7-pr-record-transition/v1"
ISSUE_FIRST = "issue_then_project"
LEGACY_PROJECT_FIRST = "legacy_project_then_issue"

_TASK_FIELDS = frozenset({"status", "workflow_phase", "pr_number", "pr_url"})
_PROJECT_FIELDS = frozenset({"task_uid", "status", "pm_status", "workflow_phase", "pr"})


def _task_vectors(pr_number: int, pr_url: str) -> tuple[dict[str, Any], dict[str, Any]]:
    return (
        {"status": "committed", "workflow_phase": "execution",
         "pr_number": None, "pr_url": None},
        {"status": "committed", "workflow_phase": "verification",
         "pr_number": pr_number, "pr_url": pr_url},
    )


def _project_vectors(task_uid: str, pr_url: str) -> tuple[dict[str, str], dict[str, str]]:
    return (
        {"task_uid": task_uid, "status": "In Progress", "pm_status": "committed",
         "workflow_phase": "execution", "pr": ""},
        {"task_uid": task_uid, "status": "In Progress", "pm_status": "committed",
         "workflow_phase": "verification", "pr": pr_url},
    )


def _is_project_prefix(actual: dict[str, Any], before: dict[str, Any],
                       target: dict[str, Any]) -> tuple[bool, int]:
    """Project fields have the fixed prior writer order phase, then PR."""
    phase = actual.get("workflow_phase")
    pr = actual.get("pr")
    prefixes = (
        (before["workflow_phase"], before["pr"]),
        (target["workflow_phase"], before["pr"]),
        (target["workflow_phase"], target["pr"]),
    )
    if (actual.get("task_uid") != before["task_uid"]
            or actual.get("status") != before["status"]
            or actual.get("pm_status") != before["pm_status"]):
        return False, -1
    try:
        return True, prefixes.index((phase, pr))
    except ValueError:
        return False, -1


def classify_record_pr_state(task_state: Any, project_state: Any, *,
                             task_uid: str, pr_number: int, pr_url: str,
                             sequence: str,
                             legacy_journal_proven: bool = False) -> dict[str, Any]:
    """Return the one reachable step prefix for the selected publication.

    New writers use `issue_then_project`, so Issue-complete/Project-incomplete
    is a real prefix. Existing journals may prove the former deterministic
    Project-fields-before-Issue sequence. The legacy route is never inferred
    from field membership alone.
    """
    if (not isinstance(task_uid, str) or not task_uid
            or type(pr_number) is not int or pr_number < 1
            or not isinstance(pr_url, str) or not pr_url
            or sequence not in {ISSUE_FIRST, LEGACY_PROJECT_FIRST}):
        return {"status": "conflict", "reason": "record-pr transition identity or sequence is invalid"}
    if sequence == LEGACY_PROJECT_FIRST and legacy_journal_proven is not True:
        return {"status": "conflict", "reason": "legacy Project-first sequence lacks exact journal proof"}
    if not isinstance(task_state, dict) or frozenset(task_state) != _TASK_FIELDS:
        return {"status": "conflict", "reason": "live Task transition vector is not closed"}
    if not isinstance(project_state, dict) or frozenset(project_state) != _PROJECT_FIELDS:
        return {"status": "conflict", "reason": "live Project transition vector is not closed"}

    task_before, task_after = _task_vectors(pr_number, pr_url)
    project_before, project_after = _project_vectors(task_uid, pr_url)
    if task_state not in (task_before, task_after):
        return {"status": "conflict", "reason": "live Task Issue is outside exact before/target vectors"}
    project_ok, project_prefix = _is_project_prefix(project_state, project_before, project_after)
    if not project_ok:
        return {"status": "conflict", "reason": "live Project fields are outside deterministic before/target prefix"}

    if sequence == ISSUE_FIRST:
        reachable = (
            (task_before, project_prefix == 0, 0),
            (task_after, project_prefix == 0, 1),
            (task_after, project_prefix == 1, 2),
            (task_after, project_prefix == 2, 3),
        )
        steps = ["issue", "project:Workflow Phase", "project:PR"]
    else:
        reachable = (
            (task_before, project_prefix == 0, 0),
            (task_before, project_prefix == 1, 1),
            (task_before, project_prefix == 2, 2),
            (task_after, project_prefix == 2, 3),
        )
        steps = ["project:Workflow Phase", "project:PR", "issue"]
    progress = next((count for expected_task, project_matches, count in reachable
                     if task_state == expected_task and project_matches), None)
    if progress is None:
        return {"status": "conflict", "reason": "Issue/Project vector is not reachable in this journaled action sequence"}
    return {
        "status": "complete" if progress == len(steps) else "partial",
        "sequence": sequence,
        "completed_steps": steps[:progress],
        "next_steps": steps[progress:],
        "task_target": task_after,
        "project_target": project_after,
    }
