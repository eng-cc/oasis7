#!/usr/bin/env python3
"""Behavior tests for C1 record-pr vector recovery and fault readback."""
from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path

import pr_projection_journal
import pr_projection_record_pr as record_pr
import pr_projection_transition as transition


UID = "task_" + "1" * 32
NUMBER = 42
URL = "https://github.com/eng-cc/oasis7/pull/42"
PUB = "sha256:" + "a" * 64
PUBLICATION = {
    "publication_id": PUB,
    "task_uid": UID,
    "source_head_oid": "1" * 40,
    "source_scope_oid": "2" * 40,
    "projection_digest": "sha256:" + "b" * 64,
}
TASK_BEFORE = {"status": "committed", "workflow_phase": "execution",
               "pr_number": None, "pr_url": None}
TASK_AFTER = {"status": "committed", "workflow_phase": "verification",
              "pr_number": NUMBER, "pr_url": URL}
PROJECT_BEFORE = {"task_uid": UID, "status": "In Progress",
                  "pm_status": "committed", "workflow_phase": "execution", "pr": ""}
PROJECT_AFTER = {**PROJECT_BEFORE, "workflow_phase": "verification", "pr": URL}


class FakeRemote:
    def __init__(self, task=None, project=None, fail_after=None, fail_before=None):
        self.state = {"task": copy.deepcopy(task or TASK_BEFORE),
                      "project": copy.deepcopy(project or PROJECT_BEFORE)}
        self.fail_after = fail_after
        self.fail_before = fail_before
        self.calls = []

    def read(self):
        return copy.deepcopy(self.state)

    def write_issue(self, target):
        self.calls.append("issue")
        if self.fail_before == "issue":
            self.fail_before = None
            raise RuntimeError("transport failed before Issue update")
        self.state["task"] = copy.deepcopy(target)
        if self.fail_after == "issue":
            self.fail_after = None
            raise RuntimeError("transport failed after Issue update")

    def write_project(self, field, value):
        name = "project:" + field
        self.calls.append(name)
        if self.fail_before == name:
            self.fail_before = None
            raise RuntimeError("transport failed before Project update")
        key = "workflow_phase" if field == "Workflow Phase" else "pr"
        self.state["project"][key] = value
        if self.fail_after == name:
            self.fail_after = None
            raise RuntimeError("transport failed after Project update")


class RecordPRTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.journal = pr_projection_journal.open_journal(
            self.temp.name, "eng-cc/oasis7", "task/record", PUB,
            task_uid=UID, source_head_oid=PUBLICATION["source_head_oid"],
            scope_base_oid=PUBLICATION["source_scope_oid"],
            projection_digest=PUBLICATION["projection_digest"],
        )
        self.lock = self.journal.locked()
        self.lock.__enter__()
        self.journal.intent("record-pr:" + PUB, "record_pr", {
            "publication_id": PUB, "task_uid": UID, "pr_number": NUMBER,
        })

    def tearDown(self):
        self.lock.__exit__(None, None, None)
        self.temp.cleanup()

    def reconcile(self, remote):
        return record_pr.reconcile_record_pr_vector(
            self.journal, PUBLICATION, task_uid=UID, pr_number=NUMBER,
            pr_url=URL, read_live=remote.read, write_issue=remote.write_issue,
            write_project_field=remote.write_project,
        )

    def test_issue_first_reaches_each_ordered_step(self):
        remote = FakeRemote()
        result = self.reconcile(remote)
        self.assertEqual(result["status"], "complete")
        self.assertEqual(remote.calls, ["issue", "project:Workflow Phase", "project:PR"])
        self.assertEqual(remote.read(), {"task": TASK_AFTER, "project": PROJECT_AFTER})

    def test_issue_complete_project_old_is_t21_reachable_prefix(self):
        remote = FakeRemote(task=TASK_AFTER)
        result = self.reconcile(remote)
        self.assertEqual(result["status"], "complete")
        self.assertEqual(remote.calls, ["project:Workflow Phase", "project:PR"])

    def test_observed_parent_replays_completed_vector_as_live_noop(self):
        remote = FakeRemote()
        self.reconcile(remote)
        first_calls = list(remote.calls)
        self.journal.observe("record-pr:" + PUB, {"pr_number": NUMBER},
                            phase="METADATA_CONFIRMED")

        result = self.reconcile(remote)

        self.assertEqual(result["status"], "complete")
        self.assertEqual(remote.calls, first_calls)
        self.assertEqual(remote.read(), {"task": TASK_AFTER, "project": PROJECT_AFTER})

    def test_observed_parent_without_vector_journal_cannot_authorize_partial_writes(self):
        self.journal.observe("record-pr:" + PUB, {"pr_number": NUMBER},
                            phase="METADATA_CONFIRMED")
        remote = FakeRemote(task=TASK_AFTER)

        with self.assertRaises(record_pr.RecordPRConflict):
            self.reconcile(remote)

        self.assertEqual(remote.calls, [])

    def test_post_effect_transport_failures_use_readback_without_replay(self):
        for step in ("issue", "project:Workflow Phase", "project:PR"):
            with self.subTest(step=step):
                self.journal.path.unlink()
                self.journal.read()
                self.journal.intent("record-pr:" + PUB, "record_pr", {
                    "publication_id": PUB, "task_uid": UID, "pr_number": NUMBER,
                })
                remote = FakeRemote(fail_after=step)
                result = self.reconcile(remote)
                self.assertEqual(result["status"], "complete")
                self.assertEqual(remote.calls.count(step), 1)
                self.assertEqual(remote.read(), {"task": TASK_AFTER, "project": PROJECT_AFTER})

    def test_pre_effect_transport_failure_stays_pending_then_reconciles(self):
        remote = FakeRemote(fail_before="issue")
        with self.assertRaises(record_pr.RecordPRPending):
            self.reconcile(remote)
        self.assertEqual(remote.read(), {"task": TASK_BEFORE, "project": PROJECT_BEFORE})
        result = self.reconcile(remote)
        self.assertEqual(result["status"], "complete")
        self.assertEqual(remote.calls.count("issue"), 2)

    def test_crossed_issue_project_vector_is_conflict(self):
        crossed = copy.deepcopy(PROJECT_BEFORE)
        crossed["pr"] = URL
        remote = FakeRemote(project=crossed)
        with self.assertRaises(record_pr.RecordPRConflict):
            self.reconcile(remote)
        self.assertEqual(remote.calls, [])

    def test_legacy_project_first_requires_exact_parent_intent_and_prefix(self):
        remote = FakeRemote(project={**PROJECT_BEFORE, "workflow_phase": "verification"})
        result = self.reconcile(remote)
        self.assertEqual(result["status"], "complete")
        self.assertEqual(remote.calls, ["project:PR", "issue"])

    def test_legacy_project_first_without_parent_intent_is_rejected(self):
        self.journal.path.unlink()
        self.journal.intent("different", "record_pr", {
            "publication_id": PUB, "task_uid": UID, "pr_number": NUMBER,
        })
        remote = FakeRemote(project={**PROJECT_BEFORE, "workflow_phase": "verification"})
        with self.assertRaises(record_pr.RecordPRConflict):
            self.reconcile(remote)
        self.assertEqual(remote.calls, [])


if __name__ == "__main__":
    unittest.main(verbosity=2)
