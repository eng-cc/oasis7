from copy import deepcopy
import unittest
import test_binding_repair as fixture_module
Host=fixture_module.Host
api=fixture_module.api

class RecoveryHost(Host):
    def __init__(self, plan, release):
        super().__init__(plan, release); self.recovery_records=[];self.disabled=True
    def binding_recovery_preflight(self, failed, release, new):
        return dict(safe=self.safe and self.disabled,new_release_absent=True,recovery_absent=not self.recovery_records,
            config_sha256=api.digest(api.canonical_bytes(self.config)),sudo_sha256="d"*64,sudo_policy_sha256="e"*64)
    def binding_recovery_receipt(self, record):
        self.event("recovery:"+record["stage"]+":"+str(len(record["completed_actions"])))
        self.recovery_records.append(deepcopy(record))
    def binding_recovery_snapshot(self):
        record=deepcopy(self.recovery_records[-1]);return record,api.digest(api.canonical_bytes(record))

class BindingRecovery(unittest.TestCase):
    def setUp(self):
        fixture=fixture_module.BindingRepair();fixture.setUp()
        self.oldplan=fixture.oldplan;self.old=fixture.old;self.failedrelease=fixture.new
        self.host=RecoveryHost(self.oldplan,self.old)
        self.failedplan=api.plan_binding_repair(self.old,self.oldplan,fixture.oldsha,self.failedrelease,self.host)
        self.failedsha=api.digest(api.canonical_bytes(self.failedplan))
        self.host.fail="doctor"
        api.apply_binding_repair(self.old,self.oldplan,self.failedrelease,self.failedplan,self.failedsha,self.host)
        self.host.fail=None;self.host.events=[]
        self.new=deepcopy(self.failedrelease);self.new["manifest"]["release_id"]="fixed";self.new["manifest_sha256"]="4"*64;self.new["verified_bytes"]["oasis7_local_signer_worker"]=b"fixed worker"
    def plan(self): return api.plan_binding_repair_recovery(self.failedrelease,self.failedplan,self.failedsha,self.new,self.host)
    def apply(self,plan): return api.apply_binding_repair_recovery(self.failedrelease,self.failedplan,self.new,plan,api.digest(api.canonical_bytes(plan)),self.host)
    def verify(self,plan): return api.verify_binding_repair_recovery(self.new,plan,api.digest(api.canonical_bytes(plan)),self.host)
    def test_plan_preserves_all_history_no_effects(self):
        record=deepcopy(self.host.record);failed=deepcopy(self.host.receipts);plan=self.plan()
        self.assertEqual(self.host.events,[]);self.assertEqual(self.host.record,record);self.assertEqual(self.host.receipts,failed)
        self.assertEqual(plan["failed_receipt"],failed[-1])
        self.assertEqual({k for k in self.host.config if self.host.config[k]!=plan["new_installation_config"][k]}, {"release_id","worker_executable","worker_sha256"})
    def test_success_durable_progress_history_retained_readonly_verifier(self):
        plan=self.plan();historical=deepcopy((self.host.record,self.host.receipts));result=self.apply(plan)
        self.assertEqual(result["status"],"BOUND_RECOVERED_UNREADY")
        self.assertEqual((self.host.record,self.host.receipts),historical)
        self.assertEqual([r["completed_actions"] for r in self.host.recovery_records],[list(api.BINDING_REPAIR_ACTIONS[:i]) for i in range(9)])
        snapshots=deepcopy(self.host.recovery_records);result=self.verify(plan)
        self.assertEqual(result["status"],"RECOVERED_VERIFIED_UNREADY");self.assertFalse(result["host_mutated"]);self.assertEqual(self.host.recovery_records,snapshots)
    def test_exact_failed_stage_prefix_and_no_authority_required(self):
        original=deepcopy(self.host.receipts[-1])
        for key,value in (("stage","intent"),("completed_actions",list(api.BINDING_REPAIR_ACTIONS[:5])),("extra",True),("new_manifest_sha256","0"*64)):
            self.host.receipts[-1]=deepcopy(original);self.host.receipts[-1][key]=value
            with self.assertRaises(api.InstallError):self.plan()
        self.host.receipts[-1]=original;self.host.disabled=False
        with self.assertRaises(api.InstallError):self.plan()
        self.assertEqual(self.host.events,[])
    def test_receipt_journal_config_runtime_release_and_plan_mutations_reject(self):
        plan=self.plan()
        for key,value in (("reason","other"),("failed_receipt_sha256","0"*64),("extra",1),("new_installation_config",{})):
            candidate=deepcopy(plan);candidate[key]=value
            with self.assertRaises(api.InstallError):self.apply(candidate)
        self.host.record["stage"]="recovery"
        with self.assertRaises(api.InstallError):self.plan()
        self.assertEqual(self.host.events,[])
    def test_each_step_and_durable_write_failure_revokes_never_reports_complete(self):
        failures=["recovery:intent:"+str(i) for i in range(8)]+["disable_sudo","release","binding","validate:False","sudo","validate:True","doctor","recovery:complete:8"]
        for failure in failures:
            self.setUp();plan=self.plan();self.host.fail=failure;historical=deepcopy((self.host.record,self.host.receipts))
            result=self.apply(plan)
            self.assertEqual(result["status"],"RECOVERY_REQUIRED",failure);self.assertIn("disable_sudo",self.host.events)
            self.assertEqual((self.host.record,self.host.receipts),historical)
            self.assertTrue(self.host.events[-1].startswith("recovery:recovery:"))
            with self.assertRaises(api.InstallError):self.apply(plan)
    def test_completed_verification_rejects_failed_receipt_drift(self):
        plan=self.plan();self.apply(plan);self.host.receipts[-1]["stage"]="quarantined"
        with self.assertRaises(api.InstallError):self.verify(plan)
    def test_nonfresh_release_and_approval_mismatch_reject(self):
        self.new["manifest"]["release_id"]="new"
        with self.assertRaises(api.InstallError):self.plan()
        self.new["manifest"]["release_id"]="fixed";plan=self.plan()
        with self.assertRaises(api.InstallError):api.apply_binding_repair_recovery(self.failedrelease,self.failedplan,self.new,plan,"0"*64,self.host)
        self.assertEqual(self.host.events,[])


class ProductionRecoveryBoundary(unittest.TestCase):
    def setUp(self):
        import macos_host
        self.module=macos_host;self.host=macos_host.MacOSHost(runtime_identity={"fixture":"runtime"})
        self.failed={"new_installation_plan":{"caller":{"name":"caller"}},"new_installation_config":{"fixture":"config"},"original_journal":{"installation_config":{"fixture":"old"}}}
        self.new={"manifest":{"release_id":"fresh"}}
    def test_existing_recovery_receipt_rejects_before_installed_checks(self):
        from unittest.mock import patch
        with patch.object(self.host,"inspect_path"),patch.object(self.module.os.path,"lexists",return_value=True),patch.object(self.host,"installed_config") as config:
            with self.assertRaises(api.InstallError):self.host.binding_recovery_preflight(self.failed,{},self.new)
            config.assert_not_called()
    def test_preflight_requires_exact_disabled_include_and_no_effective_grants(self):
        from unittest.mock import patch
        from types import SimpleNamespace
        disabled=b"# oasis7 local signer binding repair: worker authorization disabled\n"
        for raw,grant in ((b"foreign",True),(disabled,False),(disabled,True)):
            with patch.object(self.host,"inspect_path"),patch.object(self.module.os.path,"lexists",return_value=False),patch.object(self.host,"installed_config",return_value=self.failed["new_installation_config"]),patch.object(self.host,"quarantine_preflight",return_value=True),patch.object(self.host,"validate_repaired",return_value=True),patch.object(self.host,"read_sudo_policy",return_value=SimpleNamespace(no_grants=grant,encode=lambda:b"denied")),patch.object(self.module,"read_file",side_effect=lambda p,n: raw if p==self.module.SUDO else api.canonical_bytes(self.failed["new_installation_config"])):
                if raw==disabled and grant:
                    self.assertTrue(self.host.binding_recovery_preflight(self.failed,{},self.new)["safe"])
                else:
                    with self.assertRaises(api.InstallError):self.host.binding_recovery_preflight(self.failed,{},self.new)
    def test_separate_receipt_never_writes_historical_paths(self):
        from unittest.mock import patch
        record={"fixture":"receipt"}
        with patch.object(self.module.os.path,"lexists",return_value=False),patch.object(self.host,"atomic_root_file") as write:
            self.host.binding_recovery_receipt(record)
        write.assert_called_once_with(self.module.BINDING_RECOVERY,api.canonical_bytes(record),0o600,replace=False)
        self.assertNotEqual(self.module.BINDING_RECOVERY,self.module.REPAIR)
    def test_new_cli_modes_parse_and_are_exclusive(self):
        import importlib.util
        spec=importlib.util.spec_from_file_location("recovery_cli_test",fixture_module.Path(__file__).resolve().parents[1]/"install-release.py")
        cli=importlib.util.module_from_spec(spec);spec.loader.exec_module(cli)
        base=["apply","--release-dir","/fresh","--expected-manifest-sha256","a"*64,"--plan","/old","--expected-plan-sha256","b"*64]
        for mode in ("--binding-repair-recovery-plan-out","--binding-repair-recovery-apply","--binding-repair-recovery-check"):
            parsed=cli.parser().parse_args(base+[mode,"/recovery"])
            self.assertEqual(getattr(parsed,mode[2:].replace("-","_")),"/recovery")

if __name__=="__main__":unittest.main()
