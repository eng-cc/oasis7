#!/usr/bin/env python3
"""RED contract for Issue #2260: CI-backed ready_for_pr promotion."""

from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]


class CiBackedReadyForPrContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        claim = (ROOT / "scripts" / "pm" / "claim-ready.sh").read_text()
        closeout = (ROOT / "scripts" / "pm" / "task-closeout.sh").read_text()
        prepare = (ROOT / "scripts" / "prepare-task-pr.sh").read_text()
        cls.claim = claim
        cls.closeout = closeout
        cls.prepare = prepare
        cls.implementation = "\n".join((claim, closeout, prepare))

    def test_draft_candidate_precedes_ready_and_receipt_gates_promotion(self) -> None:
        for token in ("draft_candidate", "ci_ready_receipt", "promote_draft"):
            self.assertIn(token, self.implementation, token)
        self.assertLess(
            self.implementation.index("draft_candidate"),
            self.implementation.index("promote_draft"),
        )

    def test_ci_receipt_binds_every_trusted_identity_field(self) -> None:
        for token in (
            "repository",
            "task_uid",
            "pr_number",
            "base_oid",
            "head_oid",
            "check_name",
            "check_app_id",
            "check_run_id",
            "planner_digest",
            "conclusion",
            "observed_at",
        ):
            self.assertIn(token, self.implementation, token)

    def test_ready_for_pr_never_runs_heavy_required_locally(self) -> None:
        self.assertNotRegex(
            self.claim,
            r"ready_for_pr[\s\S]{0,4000}ci-tests\.sh\s+required",
            "ready_for_pr must consume trusted CI evidence, not run required locally",
        )

    def test_ci_receipt_fails_closed_for_every_untrusted_run_state(self) -> None:
        for token in (
            "stale",
            "wrong_head",
            "wrong_app",
            "superseded",
            "cancelled",
            "uncertain",
        ):
            self.assertIn(token, self.implementation, token)

    def test_review_and_closeout_require_receipt_same_head(self) -> None:
        for token in ("ci_ready_receipt", "same_head", "reviewed_source_head"):
            self.assertIn(token, self.closeout, token)


if __name__ == "__main__":
    unittest.main()
