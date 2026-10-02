#!/usr/bin/env python3
"""Journaled live Issue/Project vector recovery for the C1 record-pr action."""
from __future__ import annotations

import hashlib

from typing import Any, Callable

import pr_projection_transition as transition
from pr_projection_journal import JournalError, PublicationJournal


class RecordPRPending(RuntimeError):
    """A remote write/readback may have happened; reconcile from live truth."""


class RecordPRConflict(RuntimeError):
    """Live authority or a transition vector conflicts with this action."""


def _parent_record_pr_state(journal: PublicationJournal, publication: dict[str, Any],
                            pr_number: int) -> str | None:
    actions = journal.read().get("actions")
    if not isinstance(actions, list):
        return None
    action_id = "record-pr:" + str(publication.get("publication_id") or "")
    matches = [item for item in actions
               if isinstance(item, dict) and item.get("action_id") == action_id]
    if len(matches) != 1:
        return None
    action = matches[0]
    expected = action.get("expected")
    if not (
        action.get("kind") == "record_pr"
        and action.get("state") in {"intent", "uncertain", "observed"}
        and isinstance(expected, dict)
        and expected == {
            "publication_id": publication.get("publication_id"),
            "task_uid": publication.get("task_uid"),
            "pr_number": pr_number,
        }
    ):
        return None
    return str(action["state"])


def _legacy_intent_proven(journal: PublicationJournal, publication: dict[str, Any],
                          pr_number: int) -> bool:
    """Only an unresolved parent action may explain a legacy Project-first prefix."""
    return _parent_record_pr_state(journal, publication, pr_number) in {"intent", "uncertain"}


def _action(journal: PublicationJournal, action_id: str) -> dict[str, Any] | None:
    actions = journal.read().get("actions")
    if not isinstance(actions, list):
        raise RecordPRConflict("publication journal actions are malformed")
    matches = [item for item in actions
               if isinstance(item, dict) and item.get("action_id") == action_id]
    if len(matches) > 1:
        raise RecordPRConflict("publication journal action is duplicated")
    return matches[0] if matches else None


def _vectors(state: dict[str, Any]) -> tuple[dict[str, Any], dict[str, Any]]:
    if not isinstance(state, dict) or set(state) != {"task", "project"}:
        raise RecordPRConflict("live Issue/Project transition read is not closed")
    return state["task"], state["project"]


def _next_expected(sequence: str, step: str, task_before: dict[str, Any],
                   task_target: dict[str, Any], project_before: dict[str, Any],
                   project_target: dict[str, Any],
                   issue_body_proof: dict[str, Any] | None = None) -> dict[str, Any]:
    if step == "issue":
        expected = {"task_before": task_before, "task_target": task_target}
        if issue_body_proof is not None:
            expected["issue_body_proof"] = issue_body_proof
        return expected
    field_name = step.removeprefix("project:")
    if field_name not in {"Workflow Phase", "PR"}:
        raise RecordPRConflict("journal requested an unsupported Project transition field")
    vector_key = "workflow_phase" if field_name == "Workflow Phase" else "pr"
    return {
        "project_field": field_name,
        "before": project_before[vector_key],
        "target": project_target[vector_key],
    }


def reconcile_record_pr_vector(
    journal: PublicationJournal,
    publication: dict[str, Any],
    *,
    task_uid: str,
    pr_number: int,
    pr_url: str,
    read_live: Callable[[], dict[str, Any]],
    write_issue: Callable[[dict[str, Any]], None],
    write_project_field: Callable[[str, str], None],
    issue_body_before: str | None = None,
    issue_body_target: str | None = None,
    issue_body_default_merge_hold: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Reach only one exact ordered prefix, journaling every live step.

    New actions are Issue first, then the two Project fields in deterministic
    order.  A pre-existing parent record-pr intent can prove the old
    Project-first prefix for recovery; field membership alone never can.
    Caller must hold the C1 branch journal lock for the whole function.
    """
    if not journal._lock or publication.get("task_uid") != task_uid:
        raise RecordPRConflict("C1 record-pr journal is not held or its Task identity differs")
    publication_id = publication.get("publication_id")
    vector_action = "record-pr-vector:" + str(publication_id)
    action = _action(journal, vector_action)
    parent_state = _parent_record_pr_state(journal, publication, pr_number)
    if parent_state is None:
        raise RecordPRConflict("exact parent C1 record-pr intent is missing or unresolved")

    live = read_live()
    task_state, project_state = _vectors(live)
    if action:
        prior_expected = action.get("expected")
        sequence = prior_expected.get("sequence") if isinstance(prior_expected, dict) else None
        if (action.get("kind") != "record_pr_vector"
                or sequence not in {transition.ISSUE_FIRST, transition.LEGACY_PROJECT_FIRST}
                or (sequence == transition.LEGACY_PROJECT_FIRST
                    and parent_state not in {"intent", "uncertain", "observed"})):
            raise RecordPRConflict("C1 record-pr vector journal sequence is invalid")
        # Once the new writer has journaled its closed transition sequence,
        # never reinterpret a crossed/new-order vector through legacy rules.
        classified = transition.classify_record_pr_state(
            task_state, project_state, task_uid=task_uid,
            pr_number=pr_number, pr_url=pr_url, sequence=sequence,
            legacy_journal_proven=(sequence == transition.LEGACY_PROJECT_FIRST),
        )
    else:
        classified = transition.classify_record_pr_state(
            task_state, project_state, task_uid=task_uid,
            pr_number=pr_number, pr_url=pr_url, sequence=transition.ISSUE_FIRST,
        )
        if classified["status"] == "conflict" and _legacy_intent_proven(journal, publication, pr_number):
            classified = transition.classify_record_pr_state(
                task_state, project_state, task_uid=task_uid,
                pr_number=pr_number, pr_url=pr_url,
                sequence=transition.LEGACY_PROJECT_FIRST,
                legacy_journal_proven=True,
            )
    if (not action and parent_state == "observed"
            and classified["status"] != "complete"):
        # Without a vector journal, an already-observed parent action can
        # confirm a fully-complete legacy state but cannot authorize writes
        # from a newly observed partial vector.
        raise RecordPRConflict("observed parent action lacks a journaled partial-vector recovery")
    if classified["status"] == "conflict":
        raise RecordPRConflict(classified["reason"])

    expected_action = {
        "publication_id": publication_id,
        "task_uid": task_uid,
        "pr_number": pr_number,
        "pr_url": pr_url,
        "source_head_oid": publication.get("source_head_oid"),
        "source_scope_oid": publication.get("source_scope_oid"),
        "projection_digest": publication.get("projection_digest"),
        "sequence": classified["sequence"],
    }
    if action and action.get("expected") != expected_action:
        raise RecordPRConflict("C1 record-pr vector journal identity conflicts")

    if not action:
        journal.intent(vector_action, "record_pr_vector", expected_action)

    issue_before = {"status": "committed", "workflow_phase": "execution",
                    "pr_number": None, "pr_url": None}
    issue_target = {"status": "committed", "workflow_phase": "verification",
                    "pr_number": pr_number, "pr_url": pr_url}
    project_before = {"task_uid": task_uid, "status": "In Progress",
                      "pm_status": "committed", "workflow_phase": "execution", "pr": ""}
    project_target = {**project_before, "workflow_phase": "verification", "pr": pr_url}
    issue_body_proof = None
    if issue_body_before is not None or issue_body_target is not None:
        if not isinstance(issue_body_before, str) or not isinstance(issue_body_target, str):
            raise RecordPRConflict("record-pr Issue body proof is incomplete")
        issue_body_proof = {
            "schema": "oasis7-record-pr-issue-body-proof/v1",
            "before_body": issue_body_before,
            "before_body_sha256": hashlib.sha256(issue_body_before.encode("utf-8")).hexdigest(),
            "target_body": issue_body_target,
            "target_body_sha256": hashlib.sha256(issue_body_target.encode("utf-8")).hexdigest(),
        }
        if isinstance(issue_body_default_merge_hold, dict):
            issue_body_proof["writer_default_merge_hold"] = issue_body_default_merge_hold

    for step in classified["next_steps"]:
        before, after = _vectors(live)
        current = transition.classify_record_pr_state(
            before, after, task_uid=task_uid, pr_number=pr_number, pr_url=pr_url,
            sequence=classified["sequence"],
            legacy_journal_proven=(classified["sequence"] == transition.LEGACY_PROJECT_FIRST),
        )
        if current["status"] == "conflict" or current["next_steps"][0] != step:
            journal.disposition("CONFLICT")
            raise RecordPRConflict("live vector moved outside the next journaled record-pr step")
        task_expected = _next_expected(classified["sequence"], step, issue_before,
                                       issue_target, project_before, project_target,
                                       issue_body_proof=issue_body_proof)
        step_id = "record-pr-step:" + str(publication_id) + ":" + step
        step_expected = {
            "publication_id": publication_id, "task_uid": task_uid,
            "pr_number": pr_number, "pr_url": pr_url,
            "source_head_oid": publication.get("source_head_oid"),
            "source_scope_oid": publication.get("source_scope_oid"),
            "projection_digest": publication.get("projection_digest"),
            "sequence": classified["sequence"], **task_expected,
        }
        old_step = _action(journal, step_id)
        if old_step and (old_step.get("kind") != "record_pr_transition_step"
                         or old_step.get("expected") != step_expected):
            raise RecordPRConflict("record-pr step journal identity conflicts")
        journal.intent(step_id, "record_pr_transition_step", step_expected)
        if old_step and old_step.get("state") == "observed":
            raise RecordPRConflict("journal-observed record-pr step is no longer live")
        try:
            if step == "issue":
                if before == issue_before:
                    write_issue(issue_target)
                elif before != issue_target:
                    raise RecordPRConflict("Task Issue is outside exact before/target state")
            else:
                name = step.removeprefix("project:")
                vector_key = "workflow_phase" if name == "Workflow Phase" else "pr"
                if after.get(vector_key) == project_before[vector_key]:
                    value = project_target[vector_key]
                    write_project_field(name, value)
                elif after.get(vector_key) != project_target[vector_key]:
                    raise RecordPRConflict("Project field is outside exact before/target state")
            observed_live = read_live()
        except RecordPRConflict:
            journal.disposition("CONFLICT")
            raise
        except Exception as exc:
            try:
                observed_live = read_live()
            except Exception:
                journal.uncertain(step_id, "READBACK_UNAVAILABLE")
                raise RecordPRPending(f"record-pr step {step} outcome is unconfirmed: {exc}") from exc
            observed_task, observed_project = _vectors(observed_live)
            if step == "issue":
                observed_target = observed_task == issue_target
                observed_before = observed_task == issue_before
            else:
                key = "workflow_phase" if step == "project:Workflow Phase" else "pr"
                expected_value = project_target[key]
                before_value = project_before[key]
                observed_target = observed_project.get(key) == expected_value
                observed_before = observed_project.get(key) == before_value
            if observed_target:
                if step == "issue":
                    observed_value = observed_task
                else:
                    key = "workflow_phase" if step == "project:Workflow Phase" else "pr"
                    observed_value = {"task_uid": observed_project.get("task_uid"),
                                     "field": step.removeprefix("project:"),
                                     "value": observed_project.get(key)}
                journal.observe(step_id, {"step": step, "readback": observed_value})
                live = observed_live
                continue
            if observed_before:
                journal.uncertain(step_id, "WRITE_NOT_OBSERVED")
                raise RecordPRPending(f"record-pr step {step} was not observed; retry after reconciliation") from exc
            journal.disposition("CONFLICT")
            raise RecordPRConflict(f"record-pr step {step} readback is a third value") from exc

        observed_task, observed_project = _vectors(observed_live)
        if step == "issue":
            if observed_task != issue_target:
                if observed_task == issue_before:
                    journal.uncertain(step_id, "WRITE_NOT_OBSERVED")
                    raise RecordPRPending("Task Issue write did not reach its exact target")
                journal.disposition("CONFLICT")
                raise RecordPRConflict("Task Issue readback is a third value")
        else:
            key = "workflow_phase" if step == "project:Workflow Phase" else "pr"
            if observed_project.get(key) != project_target[key]:
                if observed_project.get(key) == project_before[key]:
                    journal.uncertain(step_id, "WRITE_NOT_OBSERVED")
                    raise RecordPRPending(f"Project {step} write did not reach its exact target")
                journal.disposition("CONFLICT")
                raise RecordPRConflict(f"Project {step} readback is a third value")
        if step == "issue":
            observed_value = observed_task
        else:
            key = "workflow_phase" if step == "project:Workflow Phase" else "pr"
            observed_value = {"task_uid": observed_project.get("task_uid"),
                             "field": step.removeprefix("project:"),
                             "value": observed_project.get(key)}
        journal.observe(step_id, {"step": step, "readback": observed_value})
        live = observed_live

    task_state, project_state = _vectors(live)
    final = transition.classify_record_pr_state(
        task_state, project_state, task_uid=task_uid,
        pr_number=pr_number, pr_url=pr_url,
        sequence=classified["sequence"],
        legacy_journal_proven=(classified["sequence"] == transition.LEGACY_PROJECT_FIRST),
    )
    if final["status"] != "complete":
        journal.uncertain(vector_action, "VECTOR_READBACK_INCOMPLETE")
        raise RecordPRPending("complete Issue/Project record-pr vector was not read back")
    journal.observe(vector_action, {
        "sequence": final["sequence"], "task": issue_target,
        "project": project_target,
    }, phase="METADATA_CONFIRMED")
    return {"status": "complete", "sequence": final["sequence"],
            "task": issue_target, "project": project_target}
