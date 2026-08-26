#!/usr/bin/env python3
import importlib.util, io, json, sys, tempfile, unittest, zipfile
from contextlib import redirect_stdout
from pathlib import Path
from unittest.mock import patch

P=Path(__file__).with_name("ci-ready-receipt.py")
S=importlib.util.spec_from_file_location("ci_ready_receipt",P); M=importlib.util.module_from_spec(S); S.loader.exec_module(M)
UID="task_12345678901234567890123456789012"

def pr(): return {"draft":True,"state":"open","merged":False,"body":f"Task: {UID}\n\nRefs #1","head":{"sha":"a"*40},"base":{"sha":"b"*40}}
def plan():
  p={"scope":"targeted","selected_capabilities":"pixel_world_bridge;viewer_js_required","reason_summary":"fixture","changed_path_count":"1","planner_config_sha256":"sha256:" + "c"*64}; p.update({k:"false" for k in M.RUN_FIELDS}); p["run_rust_baseline"]="true"; p["run_pixel_world_bridge_lib_tests"]="true"; p["run_pixel_world_bridge_wasm_check"]="true"; return p
def run(conclusion="success",app=42): return {"id":9,"name":"required-gate","status":"completed","conclusion":conclusion,"completed_at":"2026-07-14T00:00:00Z","head_sha":"a"*40,"pull_requests":[{"number":7,"base":{"sha":"b"*40},"head":{"sha":"a"*40}}],"app":{"id":app},"output":{"summary":f"<!-- {M.PLAN_MARKER} -->\n```json\n{json.dumps(plan())}\n```"}}
def null_summary_run(run_id=12345):
  r=run(); r["output"]={"summary":None,"text":None}; r["details_url"]=f"https://github.com/eng-cc/oasis7/actions/runs/{run_id}/job/9"; return r
def artifact(run_id=12345,expired=False):
  return {"id":77,"name":"oasis7-required-plan-v1","expired":expired,"workflow_run":{"id":run_id}}
def envelope(run_id=12345,repository="eng-cc/oasis7",head_oid="a"*40,base_oid="b"*40,check_name="required-gate",planner=None):
  return {"schema":"oasis7-required-plan-v1","repository":repository,"workflow_run_id":run_id,
    "head_oid":head_oid,"base_oid":base_oid,"check_name":check_name,"planner":planner if planner is not None else plan()}
def artifact_zip(payload=None,filename="oasis7-required-plan-v1.json"):
  out=io.BytesIO()
  with zipfile.ZipFile(out,"w") as z: z.writestr(filename,json.dumps(payload if payload is not None else envelope()))
  return out.getvalue()

class ReceiptTest(unittest.TestCase):
  def api(self, r=None, runs=None):
    return patch.object(M,"gh",side_effect=[r or pr(),{"check_runs":runs if runs is not None else [run()]}])
  def test_success(self):
    with self.api(): self.assertEqual("a"*40,M.live("eng-cc/oasis7",UID,1,7,"required-gate","42")[3])
  def test_live_receipt_uses_check_run_base_after_pr_base_moves(self):
    moved=pr(); moved["base"]["sha"]="c"*40
    with self.api(r=moved):
      self.assertEqual("b"*40,M.live("eng-cc/oasis7",UID,1,7,"required-gate","42")[2])
  def test_planner_config_digest_is_bound_into_the_issued_receipt(self):
    receipt=self.invoke_verify()
    self.assertIn("planner_config_sha256",receipt["planner"],
                  "canonical receipt planner omits the planner configuration digest")
    self.assertIn("planner_config_sha256",receipt,
                  "issued receipt omits the planner configuration digest")
    self.assertEqual(plan()["planner_config_sha256"],receipt["planner"]["planner_config_sha256"])
    self.assertEqual(plan()["planner_config_sha256"],receipt["planner_config_sha256"])
    self.assertIs(receipt["planner"]["run_rust_baseline"],True)
    self.assertIn("run_rust_baseline",receipt,
                  "issued receipt omits the run_rust_baseline boolean")
    self.assertIs(receipt["run_rust_baseline"],True)
    self.assertEqual(["pixel_world_bridge","viewer_js_required"],receipt["planner"]["selected_capabilities"])
    self.assertIs(receipt["planner"]["run_pixel_world_bridge_lib_tests"],True)
    self.assertIs(receipt["planner"]["run_pixel_world_bridge_wasm_check"],True)

  def test_all_planner_gate_selectors_are_preserved_in_receipt_authority(self):
    raw=plan()
    for field in ("run_scenario_regression", "run_operational_contracts",
                  "run_codex_agent_config_validation", "run_required_gate_baseline"):
      raw[field]="true"
    planner=M.canonical_planner(raw)
    for field in ("run_scenario_regression", "run_operational_contracts",
                  "run_codex_agent_config_validation", "run_required_gate_baseline"):
      self.assertIs(planner[field], True,
                    f"canonical planner dropped {field} from CI receipt authority")

    changed=dict(raw)
    changed["run_operational_contracts"]="false"
    changed_planner=M.canonical_planner(changed)
    digest=lambda value: M.hashlib.sha256(
      json.dumps(value,sort_keys=True,separators=(",", ":")).encode()
    ).hexdigest()
    self.assertNotEqual(digest(planner), digest(changed_planner),
                        "non-Rust gate selector changes must alter planner authority")
  def test_invalid_or_missing_capability_selection_fails_closed(self):
    for selected in (None,"viewer_js_required;pixel_world_bridge","viewer-js"):
      raw=plan()
      if selected is None: raw.pop("selected_capabilities")
      else: raw["selected_capabilities"]=selected
      with self.subTest(selected=selected):
        with self.assertRaisesRegex(SystemExit,"selected_capabilities|incomplete"):
          M.canonical_planner(raw)
  def test_ready_pr_requires_explicit_recovery_mode(self):
    ready=pr(); ready["draft"]=False
    with self.api(r=ready):
      with self.assertRaisesRegex(SystemExit,"superseded"): M.live("eng-cc/oasis7",UID,1,7,"required-gate","42")
    with self.api(r=ready):
      self.assertEqual("a"*40,M.live("eng-cc/oasis7",UID,1,7,"required-gate","42",allow_ready_pr=True)[3])
    for state,merged in (("closed",False),("open",True)):
      bad=pr(); bad.update(draft=False,state=state,merged=merged)
      with self.api(r=bad):
        with self.assertRaisesRegex(SystemExit,"superseded"): M.live("eng-cc/oasis7",UID,1,7,"required-gate","42",allow_ready_pr=True)
  def test_wrong_app(self):
    with self.api():
      with self.assertRaisesRegex(SystemExit,"wrong_app|uncertain"): M.live("eng-cc/oasis7",UID,1,7,"required-gate","77")
  def test_cancelled(self):
    with self.api(runs=[run("cancelled")]):
      with self.assertRaisesRegex(SystemExit,"cancelled"): M.live("eng-cc/oasis7",UID,1,7,"required-gate","42")
  def test_uncertain_linkage(self):
    bad=pr(); bad["body"]="Refs #1"
    with self.api(r=bad):
      with self.assertRaisesRegex(SystemExit,"uncertain"): M.live("eng-cc/oasis7",UID,1,7,"required-gate","42")
  def invoke_verify(self, mutate=None, refresh=False, live_run=None):
    planner=M.planner_from_run(run()); digest=M.hashlib.sha256(json.dumps(planner,sort_keys=True,separators=(",",":")).encode()).hexdigest()
    receipt={"receipt_type":"oasis7_ci_ready_receipt","issuer":"github_live_query","repository":"eng-cc/oasis7","task_uid":UID,"task_issue_number":1,"pr_number":7,"base_oid":"b"*40,"head_oid":"a"*40,"check_name":"required-gate","check_app_id":42,"check_run_id":9,"planner_digest":digest,"planner":planner,"planner_config_sha256":planner["planner_config_sha256"],"run_rust_baseline":planner["run_rust_baseline"],"conclusion":"success","observed_at":M.now()}
    if mutate: mutate(receipt)
    with tempfile.NamedTemporaryFile("w",delete=False) as f: json.dump(receipt,f); name=f.name
    argv=[str(P),"--repository","eng-cc/oasis7","--task-uid",UID,"--task-issue-number","1","--pr-number","7","--check-name","required-gate","--check-app-id","42","--planner-digest",digest,"--receipt",name]
    if refresh: argv.append("--refresh-same-identity")
    output=io.StringIO()
    with self.api(runs=[live_run or run()]),patch.object(sys,"argv",argv),redirect_stdout(output): M.main()
    return json.loads(output.getvalue())
  def test_wrong_head(self):
    with self.assertRaisesRegex(SystemExit,"wrong_head"): self.invoke_verify(lambda r:r.update(head_oid="c"*40))
  def test_legacy_receipt_without_planner_config_digest_is_rejected(self):
    with self.assertRaisesRegex(SystemExit,"planner_config_sha256|mismatch"):
      self.invoke_verify(lambda r:r.pop("planner_config_sha256"))
  def test_legacy_receipt_without_run_rust_baseline_is_rejected(self):
    with self.assertRaisesRegex(SystemExit,"run_rust_baseline|mismatch"):
      self.invoke_verify(lambda r:r.pop("run_rust_baseline"))
  def test_stale(self):
    with self.assertRaisesRegex(SystemExit,"stale"): self.invoke_verify(lambda r:r.update(observed_at="2000-01-01T00:00:00+00:00"))
  def test_explicit_same_identity_refresh_changes_only_observed_at(self):
    before={}
    def stale(r):
      r.update(observed_at="2000-01-01T00:00:00+00:00"); before.update(r)
    refreshed=self.invoke_verify(stale,refresh=True)
    self.assertNotEqual(before["observed_at"],refreshed["observed_at"])
    self.assertEqual({k:v for k,v in before.items() if k!="observed_at"},
                     {k:v for k,v in refreshed.items() if k!="observed_at"})
  def test_review_evidence_identity_ignores_refresh_time_but_binds_ci_authority(self):
    receipt=self.invoke_verify()
    refreshed={**receipt,"observed_at":"2099-01-01T00:00:00+00:00"}
    self.assertEqual(M.review_evidence_identity(receipt), M.review_evidence_identity(refreshed))
    for field,value in (("check_run_id",999),("head_oid","d"*40),("planner_digest","e"*64)):
      changed={**receipt,field:value}
      self.assertNotEqual(M.review_evidence_identity(receipt), M.review_evidence_identity(changed))
  def test_refresh_fails_on_check_identity_or_conclusion_drift(self):
    cases=(run(app=77), {**run(),"id":10}, run("failure"))
    for changed in cases:
      with self.subTest(changed=changed):
        with self.assertRaisesRegex(SystemExit,"wrong_app|mismatch|conclusion"):
          self.invoke_verify(lambda r:r.update(observed_at="2000-01-01T00:00:00+00:00"),refresh=True,live_run=changed)
  def test_uncertain_missing_planner(self):
    bad=run(); bad["output"]={"summary":"no marker"}
    with self.assertRaisesRegex(SystemExit,"uncertain"): M.planner_from_run(bad)
  def planner_from_artifact(self,meta=None,payload=None,data=None,run_id=12345):
    check=null_summary_run(run_id)
    artifacts={"artifacts":[meta if meta is not None else artifact(run_id)]}
    blob=artifact_zip(payload) if data is None else data
    with patch.object(M,"gh",return_value=artifacts),patch.object(M,"artifact_bytes",return_value=blob,create=True):
      return M.planner_for_run("eng-cc/oasis7",check,base_oid="b"*40,head_oid="a"*40)
  def test_null_summary_uses_same_workflow_run_planner_artifact(self):
    self.assertEqual("targeted",self.planner_from_artifact()["scope"])
  def test_missing_artifact_fails_closed(self):
    with patch.object(M,"gh",return_value={"artifacts":[]}):
      with self.assertRaisesRegex(SystemExit,"uncertain.*artifact|artifact.*missing"): M.planner_for_run("eng-cc/oasis7",null_summary_run(),base_oid="b"*40,head_oid="a"*40)
  def test_expired_artifact_fails_closed(self):
    with self.assertRaisesRegex(SystemExit,"expired|uncertain"): self.planner_from_artifact(meta=artifact(expired=True))
  def test_wrong_run_artifact_fails_closed(self):
    with self.assertRaisesRegex(SystemExit,"wrong.run|uncertain|artifact"): self.planner_from_artifact(meta=artifact(run_id=999),run_id=12345)
  def test_malformed_artifact_fails_closed(self):
    for bad in (b"not a zip",artifact_zip(filename="wrong.json"),artifact_zip(payload={"schema":"oasis7-required-plan-v1"})):
      with self.subTest(data=bad[:16]):
        with self.assertRaisesRegex(SystemExit,"malformed|incomplete|uncertain"): self.planner_from_artifact(data=bad)
  def test_artifact_identity_mismatch_fails_closed(self):
    cases=(
      {"repository":"wrong/repo"},
      {"run_id":999},
      {"head_oid":"c"*40},
      {"base_oid":"d"*40},
      {"check_name":"wrong-gate"},
    )
    for changed in cases:
      with self.subTest(changed=changed):
        payload=envelope(**changed)
        with self.assertRaisesRegex(SystemExit,"mismatch|wrong|uncertain"):
          self.planner_from_artifact(payload=payload)
if __name__=="__main__": unittest.main()
