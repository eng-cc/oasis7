import sys
from pathlib import Path
from copy import deepcopy
from contextlib import nullcontext
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import installer as api

class Host:
    def __init__(self, plan, release):
        self.facts = deepcopy(plan["observations"][0]["facts"])
        self.config = dict(release_id="old", worker_executable="/usr/local/libexec/oasis7-local-signer/old/oasis7_local_signer_worker", worker_sha256="a"*64, store_inode=11, store_device_id=22, callers=[dict(uid=504, work_inode=33, work_device_id=22)], signer_uid=401, signer_gid=401)
        self.record = dict(stage="complete", completed_actions=list(api.ACTIONS), plan_sha256=api.digest(api.canonical_bytes(plan)), manifest_sha256=release["manifest_sha256"], installation_id=plan["installation_id"], deployment_id=plan["deployment_id"], installation_config=deepcopy(self.config))
        self.events = []; self.receipts = []; self.safe = True; self.existing = False; self.fail = None
    def observe(self, *_): return self.facts
    def lock(self): return nullcontext()
    def recovery_snapshot(self): return deepcopy(self.record), api.digest(api.canonical_bytes(self.record))
    def validate_installed(self, *_): return self.safe
    def installed_config(self): return deepcopy(self.config)
    def repair_preflight(self, *_):
        return dict(safe=self.safe, new_release_absent=True, repair_absent=not self.existing, config_sha256=api.digest(api.canonical_bytes(self.config)), sudo_sha256="b"*64, sudo_policy_sha256="c"*64, runtime_identity=self.facts["runtime_identity"])
    def event(self, name):
        self.events.append(name)
        if self.fail == name: raise OSError("injected failure")
    def repair_receipt(self, record):
        self.event("receipt:"+record["stage"]+":"+str(len(record["completed_actions"]))); self.receipts.append(deepcopy(record)); self.existing = True
    def repair_snapshot(self):
        record=deepcopy(self.receipts[-1]); return record, api.digest(api.canonical_bytes(record))
    def repair_environment(self): return self.facts
    def quarantine_preflight(self, *_): return self.safe
    def disable_sudo(self, *_): self.event("disable_sudo")
    def publish_release(self, *_): self.event("release")
    def repair_binding(self, plan, release, config): self.event("binding"); self.config = deepcopy(config)
    def validate_repaired(self, *_, check_sudo): self.event("validate:"+str(check_sudo)); return self.safe
    def repair_publish_sudo(self, *_): self.event("sudo")
    def verify_caller_doctor(self, *_): self.event("doctor"); return {"exit_code":3,"stdout_sha256":"d"*64,"stderr_sha256":"e"*64}
    def journal(self, *_): raise AssertionError("original journal must never be written")

class BindingRepair(unittest.TestCase):
    def setUp(self):
        self.old = dict(manifest=dict(release_id="old",target="aarch64-apple-darwin"), manifest_sha256="a"*64)
        self.new = dict(manifest=dict(release_id="new",target="aarch64-apple-darwin"), manifest_sha256="f"*64,verified_bytes={"oasis7_local_signer_worker":b"new worker"})
        self.oldplan = dict(schema_version=api.PLAN_SCHEMA,release_id="old",manifest_sha256="a"*64,installation_id="installation",deployment_id="deployment",store_dir="/store",caller=dict(name="caller",work_dir="/work"),signer=dict(name="signer"),signing_enabled=False,observations=[dict(facts=dict(platform="darwin",root=True,runtime_identity={"runtime":"pinned"}))])
        self.host = Host(self.oldplan,self.old);self.oldsha=api.digest(api.canonical_bytes(self.oldplan))
    def plan(self): return api.plan_binding_repair(self.old,self.oldplan,self.oldsha,self.new,self.host)
    def apply(self, plan): return api.apply_binding_repair(self.old,self.oldplan,self.new,plan,api.digest(api.canonical_bytes(plan)),self.host)
    def test_plan_readonly_and_only_three_config_fields_change(self):
        plan=self.plan();self.assertEqual(self.host.events,[])
        old=self.host.config;new=plan["new_installation_config"]
        self.assertEqual({k for k in old if old[k]!=new[k]}, {"release_id","worker_executable","worker_sha256"})
        self.assertEqual(plan["original_journal"],self.host.record)
    def test_success_revokes_before_rebinding_and_preserves_original(self):
        plan=self.plan();before=deepcopy(self.host.record);result=self.apply(plan)
        self.assertEqual(result["status"],"BOUND_REPAIRED_UNREADY");self.assertFalse(result["signing_enabled"])
        self.assertEqual(self.host.record,before)
        self.assertEqual(self.host.events,["receipt:intent:0","receipt:intent:1","disable_sudo","receipt:intent:2","release","receipt:intent:3","binding","receipt:intent:4","validate:False","receipt:intent:5","sudo","receipt:intent:6","validate:True","doctor","receipt:intent:7","receipt:complete:8"])
        self.assertEqual(result["remaining_actions"],[])
    def test_closed_plan_mutations_reject_before_effects(self):
        original=self.plan()
        for key,value in (("reason","other"),("extra",True),("new_installation_config",{}),("original_sudo_sha256","1"*64),("original_journal_sha256","2"*64),("actions",[]),("signing_enabled",True)):
            plan=deepcopy(original);plan[key]=value
            with self.assertRaises(api.InstallError):self.apply(plan)
            self.assertEqual(self.host.events,[])
    def test_original_journal_and_runtime_drift_reject(self):
        original=deepcopy(self.host.record)
        for key,value in (("stage","recovery"),("completed_actions",list(api.ACTIONS[:-1])),("installation_config",{}),("extra",1)):
            self.host.record=deepcopy(original);self.host.record[key]=value
            with self.assertRaises(api.InstallError):self.plan()
        self.host.record=original;self.host.facts["runtime_identity"]={"runtime":"drift"}
        with self.assertRaises(api.InstallError):self.plan()
        self.assertEqual(self.host.events,[])
    def test_reused_release_or_receipt_reject(self):
        self.new["manifest"]["release_id"]="old"
        with self.assertRaises(api.InstallError):self.plan()
        self.new["manifest"]["release_id"]="new";self.host.existing=True
        with self.assertRaises(api.InstallError):self.plan()
    def test_each_ambiguous_failure_never_claims_completion_and_revokes(self):
        for failure in ("receipt:intent:0","receipt:intent:1","disable_sudo","receipt:intent:2","release","receipt:intent:3","binding","receipt:intent:4","validate:False","receipt:intent:5","sudo","receipt:intent:6","validate:True","doctor","receipt:intent:7","receipt:complete:8"):
            self.setUp();plan=self.plan();before=deepcopy(self.host.record);self.host.fail=failure
            result=self.apply(plan)
            self.assertEqual(result["status"],"RECOVERY_REQUIRED",failure)
            self.assertEqual(self.host.record,before)
            self.assertIn("disable_sudo",self.host.events)
            self.assertTrue(self.host.events[-1].startswith("receipt:recovery:"))
            with self.assertRaises(api.InstallError):self.apply(plan)
    def test_approval_mismatch_zero_effects(self):
        plan=self.plan()
        with self.assertRaises(api.InstallError):api.apply_binding_repair(self.old,self.oldplan,self.new,plan,"0"*64,self.host)
        self.assertEqual(self.host.events,[])

    def quarantine(self, plan, sha=None):
        return api.quarantine_binding_repair(self.oldplan,plan,api.digest(api.canonical_bytes(plan)),sha or self.host.repair_snapshot()[1],self.host)
    def interrupted(self, failure="receipt:intent:6"):
        plan=self.plan(); self.host.fail=failure; self.apply(plan); self.host.fail=None; self.host.events=[]; return plan
    def test_each_progress_step_is_durable_and_doctor_evidence_retained(self):
        plan=self.plan(); self.apply(plan)
        self.assertEqual([r["completed_actions"] for r in self.host.receipts], [list(api.BINDING_REPAIR_ACTIONS[:i]) for i in range(9)])
        self.assertEqual(self.host.receipts[-1]["doctor_result"]["exit_code"],3)
    def test_quarantine_only_revokes_and_keeps_original_progress(self):
        plan=self.interrupted(); original=deepcopy(self.host.record);progress=deepcopy(self.host.receipts[-1]["completed_actions"])
        result=self.quarantine(plan)
        self.assertEqual(result["status"],"QUARANTINED_UNREADY")
        self.assertEqual(self.host.events,["disable_sudo","receipt:quarantined:"+str(len(progress))])
        self.assertEqual(self.host.record,original);self.assertEqual(self.host.receipts[-1]["completed_actions"],progress)
        with self.assertRaises(api.InstallError):self.quarantine(plan)
    def test_quarantine_wrong_receipt_or_host_drift_zero_effect(self):
        plan=self.interrupted()
        with self.assertRaises(api.InstallError):self.quarantine(plan,"0"*64)
        self.host.safe=False
        with self.assertRaises(api.InstallError):self.quarantine(plan)
        self.assertEqual(self.host.events,[])
    def test_quarantine_complete_and_nonprefix_receipts_reject(self):
        plan=self.plan();self.apply(plan);self.host.events=[]
        with self.assertRaises(api.InstallError):self.quarantine(plan)
        self.host.receipts[-1]["stage"]="intent";self.host.receipts[-1]["completed_actions"]=["sudo"]
        with self.assertRaises(api.InstallError):self.quarantine(plan)
        self.assertEqual(self.host.events,[])
    def test_quarantine_failed_write_reports_recovery(self):
        plan=self.interrupted();self.host.fail="receipt:quarantined:6"
        result=self.quarantine(plan)
        self.assertEqual(result["status"],"RECOVERY_REQUIRED");self.assertTrue(result["host_mutated"])
    def test_readonly_current_verification_no_mutations(self):
        plan=self.plan();self.apply(plan);before=deepcopy(self.host.receipts);self.host.events=[]
        result=api.verify_binding_repair(self.new,plan,api.digest(api.canonical_bytes(plan)),self.host)
        self.assertEqual(result["status"],"REPAIRED_VERIFIED_UNREADY");self.assertFalse(result["host_mutated"])
        self.assertEqual(self.host.receipts,before);self.assertEqual(self.host.events,["validate:True","doctor"])
    def test_current_verification_rejects_history_or_runtime_drift(self):
        plan=self.plan();self.apply(plan);self.host.events=[]
        self.host.facts["runtime_identity"]={"runtime":"wrong"}
        with self.assertRaises(api.InstallError):api.verify_binding_repair(self.new,plan,api.digest(api.canonical_bytes(plan)),self.host)
        self.assertEqual(self.host.events,[])
        self.host.facts=deepcopy(self.oldplan["observations"][0]["facts"]);self.host.record["stage"]="recovery"
        with self.assertRaises(api.InstallError):api.verify_binding_repair(self.new,plan,api.digest(api.canonical_bytes(plan)),self.host)

if __name__ == "__main__": unittest.main()
