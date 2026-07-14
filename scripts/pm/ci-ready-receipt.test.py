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
  p={"scope":"targeted","reason_summary":"fixture","changed_path_count":"1"}; p.update({k:"false" for k in M.RUN_FIELDS}); return p
def run(conclusion="success",app=42): return {"id":9,"name":"required-gate","status":"completed","conclusion":conclusion,"completed_at":"2026-07-14T00:00:00Z","app":{"id":app},"output":{"summary":f"<!-- {M.PLAN_MARKER} -->\n```json\n{json.dumps(plan())}\n```"}}
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
  def invoke_verify(self, mutate=None):
    planner=M.planner_from_run(run()); digest=M.hashlib.sha256(json.dumps(planner,sort_keys=True,separators=(",",":")).encode()).hexdigest()
    receipt={"receipt_type":"oasis7_ci_ready_receipt","issuer":"github_live_query","repository":"eng-cc/oasis7","task_uid":UID,"task_issue_number":1,"pr_number":7,"base_oid":"b"*40,"head_oid":"a"*40,"check_name":"required-gate","check_app_id":42,"check_run_id":9,"planner_digest":digest,"planner":planner,"conclusion":"success","observed_at":M.now()}
    if mutate: mutate(receipt)
    with tempfile.NamedTemporaryFile("w",delete=False) as f: json.dump(receipt,f); name=f.name
    argv=[str(P),"--repository","eng-cc/oasis7","--task-uid",UID,"--task-issue-number","1","--pr-number","7","--check-name","required-gate","--check-app-id","42","--planner-digest",digest,"--receipt",name]
    with self.api(),patch.object(sys,"argv",argv),redirect_stdout(io.StringIO()): M.main()
  def test_wrong_head(self):
    with self.assertRaisesRegex(SystemExit,"wrong_head"): self.invoke_verify(lambda r:r.update(head_oid="c"*40))
  def test_stale(self):
    with self.assertRaisesRegex(SystemExit,"stale"): self.invoke_verify(lambda r:r.update(observed_at="2000-01-01T00:00:00+00:00"))
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
