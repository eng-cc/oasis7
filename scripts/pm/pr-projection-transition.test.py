#!/usr/bin/env python3
import importlib.util
from pathlib import Path
import unittest


PATH = Path(__file__).with_name("pr_projection_transition.py")
SPEC = importlib.util.spec_from_file_location("pr_projection_transition", PATH)
API = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(API)


UID = "task_" + "a" * 32
PR = 991
URL = f"https://github.com/eng-cc/oasis7/pull/{PR}"


def states(*, issue_done=False, phase_done=False, pr_done=False):
    task = {
        "status": "committed",
        "workflow_phase": "verification" if issue_done else "execution",
        "pr_number": PR if issue_done else None,
        "pr_url": URL if issue_done else None,
    }
    project = {
        "task_uid": UID, "status": "In Progress", "pm_status": "committed",
        "workflow_phase": "verification" if phase_done else "execution",
        "pr": URL if pr_done else "",
    }
    return task, project


class TransitionTests(unittest.TestCase):
    def classify(self, task, project, sequence=API.ISSUE_FIRST, *, legacy=False):
        return API.classify_record_pr_state(
            task, project, task_uid=UID, pr_number=PR, pr_url=URL,
            sequence=sequence, legacy_journal_proven=legacy,
        )

    def test_issue_first_reachable_prefixes_include_t21(self):
        for task, project in (
            states(),
            states(issue_done=True),
            states(issue_done=True, phase_done=True),
            states(issue_done=True, phase_done=True, pr_done=True),
        ):
            self.assertIn(self.classify(task, project)["status"], {"partial", "complete"})
        issue_done, project_old = states(issue_done=True)
        self.assertEqual(["project:Workflow Phase", "project:PR"],
                         self.classify(issue_done, project_old)["next_steps"])

    def test_new_writer_rejects_project_ahead_of_issue(self):
        task_before, _ = states()
        _, project_phase = states(phase_done=True)
        self.assertEqual("conflict", self.classify(task_before, project_phase)["status"])

    def test_legacy_project_first_requires_exact_journal_proof(self):
        task, project = states(phase_done=True)
        sequence = API.LEGACY_PROJECT_FIRST
        self.assertEqual("conflict", self.classify(task, project, sequence)["status"])
        self.assertEqual("partial", self.classify(task, project, sequence, legacy=True)["status"])

    def test_legacy_project_first_rejects_issue_before_project_splice(self):
        task_done, project_old = states(issue_done=True)
        self.assertEqual("conflict", self.classify(
            task_done, project_old, API.LEGACY_PROJECT_FIRST, legacy=True,
        )["status"])

    def test_field_membership_does_not_admit_crossed_vector(self):
        task_done, project_crossed = states(issue_done=True, pr_done=True)
        project_crossed["workflow_phase"] = "execution"
        self.assertEqual("conflict", self.classify(task_done, project_crossed)["status"])

    def test_task_partial_pair_and_third_values_are_rejected(self):
        task, project = states()
        task["pr_number"] = PR
        self.assertEqual("conflict", self.classify(task, project)["status"])
        task, project = states()
        project["pr"] = "https://github.com/eng-cc/oasis7/pull/1"
        self.assertEqual("conflict", self.classify(task, project)["status"])


if __name__ == "__main__":
    unittest.main()
