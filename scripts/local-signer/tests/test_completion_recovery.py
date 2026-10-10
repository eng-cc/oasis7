import sys
from pathlib import Path
from copy import deepcopy
import unittest
from contextlib import nullcontext
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import installer as api

class RecoveryHost:
    def __init__(self, plan, release):
        self.facts = deepcopy(plan["observations"][0]["facts"])
        self.config = {"fixture": "config"}
        self.record = dict(stage="recovery", completed_actions=list(api.ACTIONS[:-1]),
            plan_sha256=api.digest(api.canonical_bytes(plan)), manifest_sha256=release["manifest_sha256"],
            installation_id=plan["installation_id"], deployment_id=plan["deployment_id"], installation_config=self.config)
        self.events = []
        self.safe = True
        self.fail_write = False
    def observe(self, *_): return self.facts
    def lock(self): return nullcontext()
    def recovery_snapshot(self): return deepcopy(self.record), api.digest(api.canonical_bytes(self.record))
    def validate_installed(self, *_): return self.safe
    def installed_config(self): return self.config
    def journal(self, value):
        self.events.append(("journal", deepcopy(value)))
        if self.fail_write: raise OSError("fixture fsync failure")
        self.record = deepcopy(value)

class CompletionRecovery(unittest.TestCase):
    def setUp(self):
        self.release = {"manifest": {"release_id": "original-release"}, "manifest_sha256": "a" * 64}
        self.plan = dict(schema_version=api.PLAN_SCHEMA, release_id="original-release", manifest_sha256="a" * 64,
            installation_id="installation", deployment_id="deployment", signing_enabled=False, store_dir="/fixture/store", caller={"work_dir": "/fixture/work", "name": "caller"}, signer={"name": "_oasis7_signer"},
            observations=[{"facts": {"platform": "darwin", "root": True, "runtime_identity": {"fixture": "runtime"}}}])
        self.host = RecoveryHost(self.plan, self.release)
        self.pdigest = api.digest(api.canonical_bytes(self.plan))
        self.jdigest = self.host.recovery_snapshot()[1]
    def run_recovery(self, check_only=False):
        return api.recover_completion(self.release, self.plan, self.pdigest, self.host,
                                      expected_journal_sha256=self.jdigest, check_only=check_only)
    def test_check_only_has_zero_writes(self):
        result = self.run_recovery(True)
        self.assertEqual(result["status"], "RECOVERY_VERIFIED")
        self.assertEqual(result["journal_sha256"], self.jdigest)
        self.assertEqual(self.host.events, [])
    def test_exact_completion_only_one_write_no_replay(self):
        result = self.run_recovery()
        self.assertEqual(result["status"], "INSTALLED_UNREADY")
        self.assertFalse(result["signing_enabled"])
        self.assertEqual([kind for kind, _ in self.host.events], ["journal"])
        self.assertEqual(self.host.record["stage"], "complete")
        self.assertEqual(self.host.record["completed_actions"], list(api.ACTIONS))
        self.assertEqual(self.host.record["manifest_sha256"], "a" * 64)
    def test_prefix_stage_and_binding_changes_reject_without_write(self):
        original = deepcopy(self.host.record)
        for key, value in (("stage", "intent"), ("completed_actions", ["identity"]),
                           ("manifest_sha256", "b" * 64), ("plan_sha256", "b" * 64),
                           ("installation_config", {"fixture": "changed"}), ("deployment_id", "other")):
            self.host.record = deepcopy(original); self.host.record[key] = value
            self.jdigest = self.host.recovery_snapshot()[1]
            with self.assertRaises(api.InstallError): self.run_recovery()
            self.assertEqual(self.host.events, [])
    def test_runtime_or_installed_state_drift_rejects(self):
        self.host.facts["runtime_identity"] = {"fixture": "other"}
        with self.assertRaises(api.InstallError): self.run_recovery()
        self.host.facts = deepcopy(self.plan["observations"][0]["facts"]);self.host.safe = False
        with self.assertRaises(api.InstallError): self.run_recovery()
        self.assertEqual(self.host.events, [])
    def test_wrong_journal_preimage_rejects(self):
        self.jdigest = "f" * 64
        with self.assertRaises(api.InstallError): self.run_recovery()
        self.assertEqual(self.host.events, [])
    def test_failed_durable_write_never_reports_complete(self):
        self.host.fail_write = True
        result = self.run_recovery()
        self.assertEqual(result["status"], "RECOVERY_REQUIRED")
        self.assertEqual(self.host.record["stage"], "recovery")
