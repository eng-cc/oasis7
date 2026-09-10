#!/usr/bin/env python3
"""Prove v2 task closeout live-validates every receipt before reuse."""

import importlib.util
import datetime as dt
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


PM = Path(__file__).parent
ROOT = PM.parent.parent
UID = "task_12345678901234567890123456789012"


def load_identity_module(path: Path):
    spec = importlib.util.spec_from_file_location("ci_ready_receipt_identity_fixture", path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class TaskCloseoutV2LiveValidationTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name) / "fixture"
        (self.root / "scripts/pm").mkdir(parents=True)
        (self.root / ".pm/github-project-sync").mkdir(parents=True)
        shutil.copy2(PM / "task-closeout.sh", self.root / "scripts/pm/task-closeout.sh")
        shutil.copy2(PM / "ci_ready_receipt_identity.py", self.root / "scripts/pm/ci_ready_receipt_identity.py")
        subprocess.run(["git", "-C", str(self.root), "init", "-q"], check=True)
        subprocess.run(["git", "-C", str(self.root), "config", "user.email", "fixture@example.invalid"], check=True)
        subprocess.run(["git", "-C", str(self.root), "config", "user.name", "Fixture"], check=True)
        (self.root / "tracked").write_text("fixture\n", encoding="utf-8")
        subprocess.run(["git", "-C", str(self.root), "add", "tracked"], check=True)
        subprocess.run(["git", "-C", str(self.root), "commit", "-qm", "fixture"], check=True)
        self.head = subprocess.check_output(["git", "-C", str(self.root), "rev-parse", "HEAD"], text=True).strip()
        (self.root / ".pm/github-project-sync/tasks.json").write_text("{}\n", encoding="utf-8")
        self.events = self.root / "events.log"
        self._write_helpers()
        self.plan, self.receipt = self._write_v2_inputs()
        self.packet = self.root / "review.md"
        self.packet.write_text(
            "\n".join(
                [
                    "Pre-PR Local Role Review: passed",
                    f"- Source Head: {self.head}",
                    "- Review Roles: qa_engineer",
                    "- Slice Ledger: ledger.jsonl",
                    "- Review Plan: plan.json",
                    "- Review Plan Schema: oasis7-review-plan/v2",
                    f"- Review Evidence Digest: {'a' * 64}",
                    "",
                ]
            ),
            encoding="utf-8",
        )
        (self.root / "ledger.jsonl").write_text("{}\n", encoding="utf-8")

    def tearDown(self):
        self.temp.cleanup()

    def _write_helpers(self):
        (self.root / "scripts/pm/validate-review-provenance.py").write_text(
            "#!/usr/bin/env python3\n", encoding="utf-8"
        )
        workflow = self.root / "scripts/pm/github-project-workflow.sh"
        workflow.write_text(
            f"#!/usr/bin/env bash\nprintf '%s\\n' '{{\"status\":\"ok\",\"selected_task\":{{\"task_uid\":\"{UID}\",\"target\":\"ready\",\"workflow_phase\":\"pre_pr_ready\"}}}}'\n",
            encoding="utf-8",
        )
        claim = self.root / "scripts/pm/claim-ready.sh"
        claim.write_text(
            "#!/usr/bin/env bash\necho claim >>\"${EVENTS}\"\nprintf '%s\\n' '{\"claim_type\":\"ready_for_pr\",\"status\":\"verified\",\"allowed_to_claim\":true,\"verification_exit_code\":0}'\n",
            encoding="utf-8",
        )
        transition = self.root / "scripts/pm/github-project-task.py"
        transition.write_text(
            f"#!/usr/bin/env python3\nfrom pathlib import Path\nimport os\nPath(os.environ['EVENTS']).write_text(Path(os.environ['EVENTS']).read_text() + 'transition\\n')\nprint('{{\"task_uid\":\"{UID}\",\"status\":\"ready\",\"issue_url\":\"https://example.invalid/1\"}}')\n",
            encoding="utf-8",
        )
        ci = self.root / "scripts/pm/ci-ready-receipt.py"
        ci.write_text(
            """#!/usr/bin/env python3
import json
import os
import sys
from pathlib import Path

args = sys.argv[1:]
Path(os.environ['CI_CALL_LOG']).write_text(Path(os.environ['CI_CALL_LOG']).read_text() + 'called\\n')
if '--integration-run-id' not in args:
    raise SystemExit('v2 closeout did not bind the current integration request/run')
receipt = json.loads(Path(args[args.index('--receipt') + 1]).read_text())
if os.environ['CI_MODE'] == 'reject':
    raise SystemExit('forged receipt rejected by live validator')
if os.environ['CI_MODE'] == 'changed-tree':
    receipt['tested_tree_oid'] = 'e' * 40
print(json.dumps(receipt))
""",
            encoding="utf-8",
        )
        for path in (workflow, claim, transition, ci):
            path.chmod(0o755)

    def _write_v2_inputs(self):
        identity = load_identity_module(self.root / "scripts/pm/ci_ready_receipt_identity.py")
        source = identity.source_review_identity(
            task_uid=UID,
            bootstrap_epoch="1" * 64,
            repository="eng-cc/oasis7",
            pr_number=7,
            source_head_oid=self.head,
            source_scope_oid="b" * 40,
            changed_paths_digest="2" * 64,
            ordered_role_ids=["qa_engineer"],
            role_contract_digest="3" * 64,
            review_policy_digest="4" * 64,
            input_contract_digest="5" * 64,
        )
        receipt = {
            "receipt_type": "oasis7_ci_ready_receipt",
            "issuer": "github_live_query",
            "live_validation": "ci-ready-receipt-live",
            "trusted_integration_artifact": True,
            "repository": "eng-cc/oasis7",
            "task_uid": UID,
            "task_issue_number": 1,
            "pr_number": 7,
            "base_oid": "c" * 40,
            "head_oid": self.head,
            "check_name": "required-gate",
            "check_app_id": 42,
            "check_run_id": 13,
            "planner_digest": "6" * 64,
            "conclusion": "success",
            "observed_at": dt.datetime.now(dt.timezone.utc).isoformat(),
            "integration_run_id": 12,
            "integration_base_oid": "c" * 40,
            "source_head_oid": self.head,
            "workflow_ref": "eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main",
            "workflow_sha": "7" * 40,
            "request_id": 11,
            "request_created_at": "2026-09-11T00:00:00Z",
            "run_id": 12,
            "run_attempt": 1,
            "tested_tree_oid": "d" * 40,
        }
        integration = identity.integration_ci_identity(receipt)
        plan = {
            "schema": "oasis7-review-plan/v2",
            "task_uid": UID,
            "frozen_head": self.head,
            "roles": ["qa_engineer"],
            "source_review_identity": source,
            "source_review_digest": identity.source_review_digest(source),
            "integration_ci_identity": integration,
            "integration_ci_digest": identity.integration_ci_digest(integration),
            "integration_ci_provenance": {
                "live_validation": "ci-ready-receipt-live",
                "trusted_integration_artifact": True,
            },
            "preflight": {"ledger_path": str(self.root / "ledger.jsonl")},
        }
        plan_path = self.root / "plan.json"
        plan_path.write_text(json.dumps(plan), encoding="utf-8")
        receipt_path = self.root / "receipt.json"
        receipt_path.write_text(json.dumps(receipt), encoding="utf-8")
        return plan_path, receipt_path

    def _run_closeout(self, mode):
        self.events.write_text("", encoding="utf-8")
        ci_log = self.root / "ci-calls.log"
        ci_log.write_text("", encoding="utf-8")
        env = {
            **os.environ,
            "PM_ROOT_DIR": str(self.root),
            "EVENTS": str(self.events),
            "CI_CALL_LOG": str(ci_log),
            "CI_MODE": mode,
        }
        return subprocess.run(
            [
                str(self.root / "scripts/pm/task-closeout.sh"),
                "--role", "tpm",
                "--task-uid", UID,
                "--to-status", "ready",
                "--claim-type", "ready_for_pr",
                "--verification-profile", "production",
                "--review-packet-file", str(self.packet),
                "--ci-ready-receipt", str(self.receipt),
                "--json",
            ],
            cwd=self.root,
            env=env,
            text=True,
            capture_output=True,
        )

    def test_fresh_forged_marker_is_live_validated_before_transition(self):
        result = self._run_closeout("reject")
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual((self.root / "ci-calls.log").read_text(), "called\n")
        self.assertEqual((self.root / "events.log").read_text(), "")

    def test_newer_dispatch_tree_cannot_reuse_source_review(self):
        receipt = json.loads(self.receipt.read_text(encoding="utf-8"))
        receipt.update(
            {
                "integration_run_id": 100,
                "request_id": 99,
                "request_created_at": "2026-09-11T01:00:00Z",
                "run_id": 100,
                "check_run_id": 101,
            }
        )
        self.receipt.write_text(json.dumps(receipt), encoding="utf-8")
        result = self._run_closeout("changed-tree")
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual((self.root / "ci-calls.log").read_text(), "called\n")
        self.assertEqual((self.root / "events.log").read_text(), "")


if __name__ == "__main__":
    unittest.main()
