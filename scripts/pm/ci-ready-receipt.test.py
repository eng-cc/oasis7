#!/usr/bin/env python3
import base64, importlib.util, io, json, sys, tempfile, unittest, zipfile
from contextlib import redirect_stdout, ExitStack
from pathlib import Path
from unittest.mock import patch
import integration_ci
import integration_executor_contract as request_contract

P=Path(__file__).with_name("ci-ready-receipt.py")
S=importlib.util.spec_from_file_location("ci_ready_receipt",P); M=importlib.util.module_from_spec(S); S.loader.exec_module(M)
AP=Path(__file__).with_name("cargo_checker_stage_admission.py")
AS=importlib.util.spec_from_file_location("cargo_checker_stage_admission",AP); A=importlib.util.module_from_spec(AS); AS.loader.exec_module(A)
UID="task_12345678901234567890123456789012"

KEYED_POLICY={"enabled_capabilities":["input-scope-reuse/v1"],
  "approved_executor_contract_digests":["sha256:"+"9"*64],"check_app_id":42}

def keyed_request_identity(repository="eng-cc/oasis7",task_uid=UID,pr_number=7,source_head_oid="a"*40):
  return {"repository":repository,"task_uid":task_uid,"pr_number":pr_number,
    "bootstrap_epoch":1,"source_head_oid":source_head_oid,"publication_id":"publication-1",
    "source_projection_digest":"sha256:"+"1"*64,"unit_ids":["required-gate"],
    "input_fingerprints":{"required-gate":"sha256:"+"2"*64},
    "executor_contract_digest":"sha256:"+"8"*64,
    "effective_policy_digest":request_contract.effective_policy_digest(KEYED_POLICY),
    "purpose":"integration_revalidation","applicability_mode":"input_scoped",
    "snapshot_target_oid":None}

def write_request_journal(directory,*,identity=None,run_id=12345,run_attempt=1,status="observed"):
  value=identity or keyed_request_identity()
  key=request_contract.validation_request_key(value)
  request_contract.reserve_validation_request(directory,key,value,"b"*40)
  if status in ("dispatch_uncertain","observed"):
    request_contract.mark_validation_dispatch_started(directory,key)
  if status=="observed":
    request_contract.mark_validation_request_observed(directory,key,run_id,run_attempt)
  return key,value

def trusted_policy_context(policy=KEYED_POLICY,workflow_sha="f"*40):
  identity={"schema":request_contract.EFFECTIVE_POLICY_IDENTITY_SCHEMA,
    "digest":request_contract.effective_policy_digest(policy)}
  workflow_ref="eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main"
  return {"schema":"oasis7-trusted-ci-reuse-policy-context/v1",
    "repository":"eng-cc/oasis7","workflow_ref":workflow_ref,
    "workflow_sha":workflow_sha,"policy_source_sha256":"sha256:"+"3"*64,
    "effective_policy":policy,"effective_policy_identity":identity,
    "planner_inventory_authority":{"schema":"oasis7-planner-inventory-authority/v1",
      "repository":"eng-cc/oasis7","workflow_ref":workflow_ref,
      "planner_authority_oid":workflow_sha,"planner_config_sha256":"sha256:"+"4"*64}}

def keyed_request_entry(run_id=12345,attempt=2,workflow_sha="f"*40):
  return {"id":run_id,"run_attempt":attempt,"requested_at":1780000001.0,
    "workflow_run_head_sha":workflow_sha}

def pr(): return {"draft":True,"state":"open","merged":False,"body":f"Task: {UID}\n\nRefs #1","head":{"sha":"a"*40},"base":{"sha":"b"*40,"ref":"main"}}
def plan():
  p={"scope":"targeted","selected_capabilities":"pixel_world_bridge;viewer_js_required","reason_summary":"fixture","changed_path_count":"1","planner_config_sha256":"sha256:" + "c"*64}; p.update({k:"false" for k in M.RUN_FIELDS}); p["run_rust_baseline"]="true"; p["run_pixel_world_bridge_lib_tests"]="true"; p["run_pixel_world_bridge_wasm_check"]="true"; return p
def versioned_plan():
  p=plan(); p["selected_capabilities"]="packaging_contracts;pixel_world_bridge;viewer_js_required"
  p["execution_contract"]=M.EXECUTION_CONTRACT
  p.update({field:"false" for field in M.VERSIONED_SELECTOR_FIELDS})
  p["run_packaging_contracts"]="true"
  p.update({field:"false" for field in M.VERSIONED_RESOURCE_FIELDS})
  p["needs_python"]="true"; p["needs_markdown"]="true"
  return p
def run(conclusion="success",app=42,planner=None): return {"id":9,"name":"required-gate","status":"completed","conclusion":conclusion,"completed_at":"2026-07-14T00:00:00Z","head_sha":"a"*40,"pull_requests":[{"number":7,"base":{"sha":"b"*40},"head":{"sha":"a"*40}}],"app":{"id":app},"output":{"summary":f"<!-- {M.PLAN_MARKER} -->\n```json\n{json.dumps(planner if planner is not None else plan())}\n```"}}
def null_summary_run(run_id=12345):
  r=run(); r["output"]={"summary":None,"text":None}; r["details_url"]=f"https://github.com/eng-cc/oasis7/actions/runs/{run_id}/job/9"; return r
def artifact(run_id=12345,expired=False):
  return {"id":77,"name":"oasis7-required-plan-v1","expired":expired,"created_at":"2026-09-25T10:10:00Z","workflow_run":{"id":run_id}}
def envelope(run_id=12345,repository="eng-cc/oasis7",head_oid="a"*40,base_oid="b"*40,check_name="required-gate",planner=None):
  return {"schema":"oasis7-required-plan-v1","repository":repository,"workflow_run_id":run_id,
    "head_oid":head_oid,"base_oid":base_oid,"check_name":check_name,"planner":planner if planner is not None else plan()}
def artifact_zip(payload=None,filename="oasis7-required-plan-v1.json"):
  out=io.BytesIO()
  with zipfile.ZipFile(out,"w") as z: z.writestr(filename,json.dumps(payload if payload is not None else envelope()))
  return out.getvalue()
def stage_receipt(task_uid=UID,pr_number=7,base_oid="b"*40,head_oid="a"*40,scope_base_oid="b"*40,tested_tree="c"*40,run_id="12345",run_attempt="1",status="passed",check_head=None):
  command=["python3","scripts/pm/check-cargo-package-scope","--base",scope_base_oid,"--head",head_oid,"--json"]
  receipt={"schema":A.SCHEMA,"phase":"post_run","activation":"provisional","repository":"eng-cc/oasis7","default_branch":"main","task_uid":task_uid,"pr_number":pr_number,
    "base_oid":base_oid,"head_oid":head_oid,"scope_base_oid":scope_base_oid,"tested_tree":tested_tree,
    "runner":{"run_id":run_id,"run_attempt":run_attempt,"workflow_ref":"eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main","workflow_sha":"w"*40},
    "normative_authority":{"merged_commit":"d"*40},"planner_authority":{"merged_commit":"e"*40},
    "executing_planner":{"merged_commit":"e"*40,"source_head":"f"*40,"bytes_sha256":"sha256:"+"d"*64},
    "check":{"check_name":"required-gate","check_app_id":42,"check_run_id":9,"check_head":head_oid if check_head is None else check_head,"workflow_run_id":run_id},
    "checker_command_digest":A.command_digest(command),
    "result":{"status":status,"exit_code":0 if status=="passed" else 1,"command":command,"command_digest":A.command_digest(command),"base_oid":base_oid,"head_oid":head_oid,"scope_base_oid":scope_base_oid,"tested_tree":tested_tree}}
  receipt["receipt_digest"]=A.durable_receipt_digest(receipt)
  return receipt
def stage_artifact(run_id=12345,run_attempt=1,expired=False):
  return {"id":88,"name":f"cargo-checker-stage-admission-receipt-{run_id}-{run_attempt}","expired":expired,"workflow_run":{"id":run_id}}
def stage_artifact_zip(receipt=None):
  out=io.BytesIO()
  with zipfile.ZipFile(out,"w") as z: z.writestr("post-run-receipt.json",json.dumps(receipt if receipt is not None else stage_receipt()))
  return out.getvalue()
def action_job(job_id,name,runner,*,run_id=12345,attempt=2,check_run_id=None,status="completed",conclusion="success",head_sha="f"*40):
  return {"id":job_id,"run_id":run_id,"run_attempt":attempt,"name":name,
    "status":status,"conclusion":conclusion,"head_sha":head_sha,
    "labels":[runner] if runner else [],
    "check_run_url":f"https://api.github.com/repos/eng-cc/oasis7/check-runs/{check_run_id if check_run_id is not None else job_id+100000}"}
def selected_action_context(children,*,run_id=12345,attempt=2,gate_job_id=9009,gate_check_id=9,artifact_created="2026-09-25T10:10:00Z"):
  gate=action_job(gate_job_id,"required-gate","ubuntu-24.04",run_id=run_id,attempt=attempt,check_run_id=gate_check_id)
  gate.update(started_at="2026-09-25T10:00:00Z",completed_at="2026-09-25T10:20:00Z")
  plan_meta=artifact(run_id); plan_meta["created_at"]=artifact_created
  def read(*args):
    path=args[-1]
    if path==f"repos/eng-cc/oasis7/actions/runs/{run_id}/artifacts?per_page=100&page=1":return {"artifacts":[plan_meta]}
    if path==f"repos/eng-cc/oasis7/actions/jobs/{gate_job_id}":return gate
    if path==f"repos/eng-cc/oasis7/actions/runs/{run_id}/attempts/{attempt}/jobs?per_page=100&page=1":return {"jobs":[gate,*children]}
    raise AssertionError(path)
  return read

class ReceiptTest(unittest.TestCase):
  def api(self, r=None, runs=None, actions=None):
    def read(*args):
      path=args[-1]
      if actions is not None and ("/actions/jobs/" in path or "/attempts/" in path or "/artifacts?" in path):
        return actions(*args)
      if '/pulls/' in path:return r or pr()
      if '/check-runs?' in path:return {"check_runs":runs if runs is not None else [run()]}
      if '/runs?' in path:return {'workflow_runs':[]}
      if '/compare/' in path:return {'merge_base_commit':{'sha':'b'*40}}
      raise AssertionError(path)
    stack=ExitStack();stack.enter_context(patch.object(M,'gh',side_effect=read));stack.enter_context(patch.object(integration_ci,'gh',side_effect=read));return stack
  def test_scope_query_uses_gh_api_argv(self):
    with patch.object(M.subprocess,"check_output",return_value=json.dumps({"merge_base_commit":{"sha":"d"*40}})) as call:
      self.assertEqual(M.scope_base_for_run("eng-cc/oasis7","b"*40,"a"*40),"d"*40)
      self.assertEqual(call.call_args.args[0][:2],["gh","api"])

  def test_scope_base_is_bound_in_review_digest(self):
    receipt=self.invoke_verify()
    receipt.update(scope_base_oid=receipt["base_oid"],integration_base_oid=receipt["base_oid"])
    original=M.review_evidence_digest(receipt)
    receipt["scope_base_oid"]="d"*40
    self.assertNotEqual(original,M.review_evidence_digest(receipt))
    receipt["integration_base_oid"]="e"*40
    with self.assertRaisesRegex(ValueError,"scope/integration"):
      M.review_evidence_digest(receipt)

  def test_new_receipt_records_both_bases(self):
    planner=M.planner_from_run(run())
    digest=M.hashlib.sha256(json.dumps(planner,sort_keys=True,separators=(",",":")).encode()).hexdigest()
    argv=[str(P),"--repository","eng-cc/oasis7","--task-uid",UID,"--task-issue-number","1","--pr-number","7","--check-app-id","42","--planner-digest",digest]
    output=io.StringIO()
    with self.api(),patch.object(sys,"argv",argv),redirect_stdout(output): M.main()
    issued=json.loads(output.getvalue())
    self.assertEqual(issued["scope_base_oid"],"b"*40)
    self.assertEqual(issued["integration_base_oid"],issued["base_oid"])
    self.assertEqual("main", issued["base_ref"])
    self.assertEqual("ordinary_pr", issued["ci_validation_mode"])

  def test_versioned_receipt_records_and_binds_the_execution_contract(self):
    versioned_run=run(planner=versioned_plan())
    versioned_run["output"]={"summary":None,"text":None}
    versioned_run["details_url"]="https://github.com/eng-cc/oasis7/actions/runs/12345/job/9009"
    children=[
      action_job(9101,M.WINDOWS_ROLLOUT_JOB,"windows-2022",status="completed",conclusion="skipped"),
      action_job(9102,M.MACOS_PACKAGE_JOB,"ubuntu-24.04",check_run_id=9202),
      *(action_job(9110+i,f"{M.FLEET_HEALTH_JOB} ({runner})",runner,status="completed",conclusion="skipped")
        for i,runner in enumerate(M.FLEET_HEALTH_RUNNERS)),
    ]
    actions=selected_action_context(children)
    argv=[str(P),"--repository","eng-cc/oasis7","--task-uid",UID,"--task-issue-number","1","--pr-number","7","--check-app-id","42","--planner-digest","auto"]
    output=io.StringIO()
    with self.api(runs=[versioned_run],actions=actions),patch.object(M,"artifact_bytes",return_value=artifact_zip(envelope(planner=versioned_plan()))):
      planner=M.planner_for_run("eng-cc/oasis7",versioned_run,base_oid="b"*40,head_oid="a"*40)
      digest=M.hashlib.sha256(json.dumps(planner,sort_keys=True,separators=(",",":")).encode()).hexdigest()
      argv[-1]=digest
      with patch.object(sys,"argv",argv),redirect_stdout(output): M.main()
    issued=json.loads(output.getvalue())
    self.assertEqual(M.EXECUTION_CONTRACT,issued["execution_contract"])
    self.assertEqual(M.EXECUTION_CONTRACT,issued["planner"]["execution_contract"])
    for field in M.VERSIONED_SELECTOR_FIELDS:
      self.assertIn(field,issued["planner"])
    identity=M.review_evidence_identity(issued)
    self.assertEqual(M.EXECUTION_CONTRACT,identity["execution_contract"])
    for field in M.VERSIONED_SELECTOR_FIELDS:
      self.assertEqual(issued["planner"][field],identity[field])
    child_evidence=issued["planner"]["selected_child_job_outcomes"]
    self.assertEqual(12345,child_evidence["workflow_run_id"])
    self.assertEqual(2,child_evidence["run_attempt"])
    self.assertEqual([M.MACOS_PACKAGE_JOB],[job["name"] for job in child_evidence["jobs"]])
    tampered=json.loads(json.dumps(issued))
    tampered["planner"]["run_packaging_contracts"]=False
    tampered["planner"]["selected_capabilities"].remove("packaging_contracts")
    with self.assertRaisesRegex(ValueError,"planner digest mismatch"):
      M.review_evidence_identity(tampered)
    resource_tampered=json.loads(json.dumps(issued))
    resource_tampered["planner"]["needs_node"]=True
    with self.assertRaisesRegex(ValueError,"planner digest mismatch"):
      M.review_evidence_identity(resource_tampered)
    child_tampered=json.loads(json.dumps(issued))
    child_tampered["planner"]["selected_child_job_outcomes"]["jobs"][0]["check_run_id"]+=1
    with self.assertRaisesRegex(ValueError,"planner digest mismatch"):
      M.review_evidence_identity(child_tampered)
    contradictory=json.loads(json.dumps(issued))
    contradictory["planner"]["run_packaging_contracts"]=False
    contradictory["planner_digest"]=M.hashlib.sha256(
      json.dumps(contradictory["planner"],sort_keys=True,separators=(",",":")).encode()
    ).hexdigest()
    with self.assertRaisesRegex(ValueError,"contradicts selected capabilities"):
      M.review_evidence_identity(contradictory)

  def test_success(self):
    with self.api(): self.assertEqual("a"*40,M.live("eng-cc/oasis7",UID,1,7,"required-gate","42")[3])
  def test_live_receipt_rejects_stale_integration_after_pr_base_moves(self):
    moved=pr(); moved["base"]["sha"]="c"*40
    with self.api(r=moved):
      with self.assertRaisesRegex(SystemExit,"integration.*rerun"):
        M.live("eng-cc/oasis7",UID,1,7,"required-gate","42")
  def test_ordinary_pr_receipt_allows_unrelated_target_advance(self):
    moved=pr(); moved["base"]["sha"]="c"*40
    moved_run=run(); moved_run["pull_requests"][0]["base"]["sha"]="c"*40
    with self.api(r=moved,runs=[moved_run]):
      _, observed, base, head = M.live(
        "eng-cc/oasis7", UID, 1, 7, "required-gate", "42", ordinary_pr=True
      )
    self.assertEqual("c"*40, base)
    self.assertEqual("a"*40, head)
    self.assertEqual("success", observed["conclusion"])
  def test_ordinary_pr_receipt_still_rejects_invalid_check(self):
    for conclusion in ("failure", "cancelled"):
      with self.subTest(conclusion=conclusion), self.api(runs=[run(conclusion)]):
        with self.assertRaisesRegex(SystemExit, conclusion):
          M.live("eng-cc/oasis7", UID, 1, 7, "required-gate", "42", ordinary_pr=True)
    wrong=run(); wrong["head_sha"]="c"*40; wrong["pull_requests"][0]["head"]["sha"]="c"*40
    with self.api(runs=[wrong]):
      with self.assertRaisesRegex(SystemExit, "wrong_head"):
        M.live("eng-cc/oasis7", UID, 1, 7, "required-gate", "42", ordinary_pr=True)
  def test_ordinary_pr_receipt_rejects_newer_pending_run_instead_of_old_green(self):
    older_green=run()
    newer_pending=run()
    newer_pending.update(id=10, status="in_progress", conclusion=None, completed_at=None)
    with self.api(runs=[older_green, newer_pending]):
      with self.assertRaisesRegex(SystemExit, "check incomplete"):
        M.live("eng-cc/oasis7", UID, 1, 7, "required-gate", "42", ordinary_pr=True)
  def test_selected_integration_request_is_verified_at_its_exact_attempt(self):
    request={"id":12345,"run_attempt":1,"requested_at":1780000000.0}
    check={"id":902,"name":"required-gate","app":{"id":42},
      "status":"completed","conclusion":"success"}
    proof={"workflow_run_id":12345,"run_attempt":1,"tested_tree_oid":"c"*40,
      "tested_commit_oid":"d"*40,"workflow_sha":"e"*40,
      "workflow_ref":"eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main"}
    with patch.object(M,"gh",side_effect=[pr(),pr()]), \
         patch.object(integration_ci,"current_request",side_effect=[request,request]), \
         patch.object(integration_ci,"verified_run",return_value=(check,proof)) as verified:
      _, observed, _, _=M.selected_live("eng-cc/oasis7",UID,1,7,"required-gate","42")
    verified.assert_called_once_with(
      "eng-cc/oasis7",UID,7,"b"*40,"a"*40,12345,"42",expected_attempt=1
    )
    self.assertEqual(1,observed["_integration"]["run_attempt"])
  def test_latest_integration_attempt_blocks_older_green_on_pending_failure_or_missing_artifact(self):
    # A1 is known-good, but the selected request now points at A2. The reader
    # must ask for A2 explicitly and fail closed for each incomplete outcome.
    request={"id":12345,"run_attempt":2,"requested_at":1780000001.0}
    for reason in ("check incomplete","check failure","artifact missing"):
      def verify(*args,expected_attempt=None):
        if expected_attempt==1:
          return ({"id":901,"name":"required-gate","app":{"id":42},
            "status":"completed","conclusion":"success"},
            {"workflow_run_id":12345,"run_attempt":1})
        if expected_attempt==2:
          raise ValueError(reason)
        raise ValueError("expected attempt is missing")
      with self.subTest(reason=reason), \
           patch.object(M,"gh",side_effect=[pr(),pr()]), \
           patch.object(integration_ci,"current_request",return_value=request), \
           patch.object(integration_ci,"verified_run",side_effect=verify) as verified, \
           patch.object(M,"live") as ordinary:
        with self.assertRaisesRegex(SystemExit,"current request blocked: "+reason):
          M.selected_live("eng-cc/oasis7",UID,1,7,"required-gate","42")
        verified.assert_called_once_with(
          "eng-cc/oasis7",UID,7,"b"*40,"a"*40,12345,"42",expected_attempt=2
        )
        ordinary.assert_not_called()
  def test_explicit_request_key_requires_matching_observed_common_dir_journal(self):
    with tempfile.TemporaryDirectory() as temp:
      journal=Path(temp)
      key,identity=write_request_journal(journal)
      with patch.object(integration_ci,"git_common_dir",return_value=journal):
        record,observed=M._trusted_validation_request(key,"eng-cc/oasis7",UID,7,"a"*40,"b"*40)
        self.assertEqual(identity,observed)
        self.assertEqual(12345,record["run_id"])
        for expected,pattern in (
          (("owner/repo",UID,7,"a"*40,"b"*40),"repository"),
          (("eng-cc/oasis7","task_ffffffffffffffffffffffffffffffff",7,"a"*40,"b"*40),"task_uid"),
          (("eng-cc/oasis7",UID,8,"a"*40,"b"*40),"pr_number"),
          (("eng-cc/oasis7",UID,7,"c"*40,"b"*40),"source_head_oid"),
          (("eng-cc/oasis7",UID,7,"a"*40,"c"*40),"immutable integration base"),
        ):
          with self.subTest(expected=expected),self.assertRaisesRegex(ValueError,pattern):
            M._trusted_validation_request(key,*expected)
        with self.assertRaisesRegex(ValueError,"journal is missing"):
          M._trusted_validation_request("sha256:"+"0"*64,"eng-cc/oasis7",UID,7,"a"*40,"b"*40)

  def test_unobserved_request_journal_cannot_authorize_keyed_selection(self):
    with tempfile.TemporaryDirectory() as temp:
      journal=Path(temp)
      key,_=write_request_journal(journal,status="dispatch_uncertain")
      with patch.object(integration_ci,"git_common_dir",return_value=journal), \
           patch.object(M,"gh",side_effect=[pr()]), \
           patch.object(integration_ci,"current_request") as current:
        with self.assertRaisesRegex(SystemExit,"journal has no observed workflow run"):
          M.selected_live("eng-cc/oasis7",UID,1,7,"required-gate","42",request_key=key)
        current.assert_not_called()

  def test_keyed_selection_passes_journal_identity_and_trusted_policy_to_verifier(self):
    with tempfile.TemporaryDirectory() as temp:
      journal=Path(temp)
      key,identity=write_request_journal(journal,run_attempt=1)
      request=keyed_request_entry(attempt=2)
      context=trusted_policy_context(workflow_sha="f"*40)
      check={"id":902,"name":"required-gate","app":{"id":42},
        "status":"completed","conclusion":"success"}
      proof={"workflow_run_id":12345,"run_attempt":2,"check_app_id":42,
        "check_run_id":902,"plan_artifact_id":777,
        "trusted_policy_context":context,
        "effective_policy_identity":context["effective_policy_identity"],
        "planner_inventory_authority":context["planner_inventory_authority"]}
      with patch.object(integration_ci,"git_common_dir",return_value=journal), \
           patch.object(M,"gh",side_effect=[pr(),pr()]), \
           patch.object(integration_ci,"current_request",side_effect=[request,request]) as current, \
           patch.object(integration_ci,"trusted_policy_context",return_value=context) as policy, \
           patch.object(integration_ci,"verified_run",return_value=(check,proof)) as verified:
        _,observed,_,_=M.selected_live(
          "eng-cc/oasis7",UID,1,7,"required-gate","42",request_key=key)
      self.assertEqual(2,observed["_integration"]["run_attempt"])
      self.assertEqual(key,observed["_integration"]["request_key"])
      self.assertEqual(identity,observed["_integration"]["request_identity"])
      self.assertEqual(2,current.call_count)
      self.assertEqual(key,current.call_args.kwargs["request_key"])
      policy.assert_called_once_with("eng-cc/oasis7","main","f"*40,"f"*40)
      verified.assert_called_once_with(
        "eng-cc/oasis7",UID,7,"b"*40,"a"*40,12345,"42",
        request_key=key,expected_attempt=2,request_identity=identity,
        effective_policy=KEYED_POLICY)

  def test_keyed_readback_rejects_run_attempt_app_check_and_policy_context_mismatch(self):
    for changed in ("run", "attempt", "app", "check", "policy", "authority"):
      with self.subTest(changed=changed),tempfile.TemporaryDirectory() as temp:
        journal=Path(temp)
        key,_=write_request_journal(journal,run_attempt=1)
        request=keyed_request_entry(attempt=2)
        context=trusted_policy_context(workflow_sha="f"*40)
        proof={"workflow_run_id":12345,"run_attempt":2,"check_app_id":42,
          "check_run_id":902,"plan_artifact_id":777,
          "trusted_policy_context":context,
          "effective_policy_identity":context["effective_policy_identity"],
          "planner_inventory_authority":context["planner_inventory_authority"]}
        if changed=="run": proof["workflow_run_id"]=12346
        elif changed=="attempt": proof["run_attempt"]=1
        elif changed=="app": proof["check_app_id"]=99
        elif changed=="check": proof["check_run_id"]=903
        elif changed=="policy": proof["effective_policy_identity"]={"schema":"oasis7-ci-effective-policy-identity/v1","digest":"sha256:"+"0"*64}
        elif changed=="authority": proof["planner_inventory_authority"]={**context["planner_inventory_authority"],"planner_authority_oid":"e"*40}
        check={"id":902,"name":"required-gate","app":{"id":42},
          "status":"completed","conclusion":"success"}
        with patch.object(integration_ci,"git_common_dir",return_value=journal), \
             patch.object(M,"gh",side_effect=[pr()]), \
             patch.object(integration_ci,"current_request",return_value=request), \
             patch.object(integration_ci,"trusted_policy_context",return_value=context), \
             patch.object(integration_ci,"verified_run",return_value=(check,proof)), \
             patch.object(M,"live") as ordinary:
          with self.assertRaisesRegex(SystemExit,"verified workflow attempt or trusted policy context mismatch"):
            M.selected_live("eng-cc/oasis7",UID,1,7,"required-gate","42",request_key=key)
          ordinary.assert_not_called()

  def test_keyed_request_change_during_verification_blocks_receipt(self):
    with tempfile.TemporaryDirectory() as temp:
      journal=Path(temp)
      key,_=write_request_journal(journal,run_attempt=1)
      first=keyed_request_entry(attempt=2)
      second=keyed_request_entry(attempt=3)
      context=trusted_policy_context(workflow_sha="f"*40)
      check={"id":902,"name":"required-gate","app":{"id":42},
        "status":"completed","conclusion":"success"}
      proof={"workflow_run_id":12345,"run_attempt":2,"check_app_id":42,
        "check_run_id":902,"plan_artifact_id":777,
        "trusted_policy_context":context,
        "effective_policy_identity":context["effective_policy_identity"],
        "planner_inventory_authority":context["planner_inventory_authority"]}
      with patch.object(integration_ci,"git_common_dir",return_value=journal), \
           patch.object(M,"gh",side_effect=[pr()]), \
           patch.object(integration_ci,"current_request",side_effect=[first,second]), \
           patch.object(integration_ci,"trusted_policy_context",return_value=context), \
           patch.object(integration_ci,"verified_run",return_value=(check,proof)), \
           patch.object(M,"live") as ordinary:
        with self.assertRaisesRegex(SystemExit,"current request changed during integration verification"):
          M.selected_live("eng-cc/oasis7",UID,1,7,"required-gate","42",request_key=key)
        ordinary.assert_not_called()

  def test_keyed_latest_attempt_failures_never_fall_back_to_older_or_ordinary_evidence(self):
    for reason in ("check incomplete","check failure","artifact missing"):
      with self.subTest(reason=reason),tempfile.TemporaryDirectory() as temp:
        journal=Path(temp)
        key,_=write_request_journal(journal,run_attempt=1)
        request=keyed_request_entry(attempt=2)
        context=trusted_policy_context(workflow_sha="f"*40)
        def verify(*args,request_key=None,expected_attempt=None,request_identity=None,effective_policy=None):
          if expected_attempt==1:
            return ({"id":901,"name":"required-gate","app":{"id":42},
              "status":"completed","conclusion":"success"},
              {"workflow_run_id":12345,"run_attempt":1})
          raise ValueError(reason)
        with patch.object(integration_ci,"git_common_dir",return_value=journal), \
             patch.object(M,"gh",side_effect=[pr()]), \
             patch.object(integration_ci,"current_request",return_value=request) as current, \
             patch.object(integration_ci,"trusted_policy_context",return_value=context), \
             patch.object(integration_ci,"verified_run",side_effect=verify) as verified, \
             patch.object(M,"live") as ordinary:
          with self.assertRaisesRegex(SystemExit,"current request blocked: "+reason):
            M.selected_live("eng-cc/oasis7",UID,1,7,"required-gate","42",request_key=key)
          current.assert_called_once_with(
            "eng-cc/oasis7",UID,7,"b"*40,"a"*40,"main",request_key=key)
          verified.assert_called_once()
          self.assertEqual(2,verified.call_args.kwargs["expected_attempt"])
          ordinary.assert_not_called()

  def test_keyed_request_absence_and_missing_v2_inventory_do_not_use_v1_fallback(self):
    with tempfile.TemporaryDirectory() as temp:
      journal=Path(temp)
      key,_=write_request_journal(journal)
      with patch.object(integration_ci,"git_common_dir",return_value=journal), \
           patch.object(M,"gh",side_effect=[pr()]), \
           patch.object(integration_ci,"current_request",return_value=None), \
           patch.object(M,"live") as ordinary:
        with self.assertRaisesRegex(SystemExit,"explicit keyed current request is absent"):
          M.selected_live("eng-cc/oasis7",UID,1,7,"required-gate","42",request_key=key)
        ordinary.assert_not_called()

    keyed_run=run();key="sha256:"+"7"*64
    keyed_run["_integration"]={"request_key":key}
    argv=[str(P),"--repository","eng-cc/oasis7","--task-uid",UID,"--task-issue-number","1",
      "--pr-number","7","--check-app-id","42","--planner-digest","auto","--request-key",key]
    with patch.object(M,"selected_live",return_value=(pr(),keyed_run,"b"*40,"a"*40)), \
         patch.object(M,"planner_for_run") as legacy_reader, \
         patch.object(sys,"argv",argv):
      with self.assertRaisesRegex(SystemExit,"full trusted v2 planner inventory/results are unavailable"):
        M.main()
      legacy_reader.assert_not_called()

  def test_refresh_of_keyed_receipt_requires_explicit_request_key(self):
    key="sha256:"+"6"*64
    with tempfile.TemporaryDirectory() as temp:
      receipt=Path(temp)/"receipt.json"
      receipt.write_text(json.dumps({"request_key":key}),encoding="utf-8")
      argv=[str(P),"--repository","eng-cc/oasis7","--task-uid",UID,"--task-issue-number","1",
        "--pr-number","7","--check-app-id","42","--planner-digest","auto","--receipt",str(receipt)]
      with patch.object(M,"selected_live") as selected,patch.object(sys,"argv",argv):
        with self.assertRaisesRegex(SystemExit,"explicit --request-key is required"):
          M.main()
        selected.assert_not_called()
  def test_expected_base_ref_rejects_same_oid_pr_retarget(self):
    moved=pr(); moved["base"]["ref"]="release"
    moved_run=run(); moved_run["pull_requests"][0]["base"]["ref"]="release"
    with self.api(r=moved,runs=[moved_run]):
      with self.assertRaisesRegex(SystemExit,"wrong_base_ref|base identity"):
        M.live("eng-cc/oasis7",UID,1,7,"required-gate","42",expected_base_ref="main")
  def test_receipt_bound_target_ref_is_rechecked_on_refresh(self):
    with self.assertRaisesRegex(SystemExit, "base ref mismatch"):
      self.invoke_verify(lambda receipt: receipt.update(base_ref="release"))
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
                  "run_codex_agent_config_validation", "run_required_gate_baseline",
                  "run_site_contract_tests"):
      raw[field]="true"
    planner=M.canonical_planner(raw)
    for field in ("run_scenario_regression", "run_operational_contracts",
                  "run_codex_agent_config_validation", "run_required_gate_baseline",
                  "run_site_contract_tests"):
      self.assertIn(field, planner,
                    f"canonical planner omitted {field} from CI receipt authority")
      self.assertIs(planner[field], True,
                    f"canonical planner dropped {field} from CI receipt authority")

    changed=dict(raw)
    changed["run_site_contract_tests"]="false"
    changed_planner=M.canonical_planner(changed)
    digest=lambda value: M.hashlib.sha256(
      json.dumps(value,sort_keys=True,separators=(",", ":")).encode()
    ).hexdigest()
    self.assertNotEqual(digest(planner), digest(changed_planner),
                        "non-Rust gate selector changes must alter planner authority")

  def test_versioned_planner_digest_binds_contract_and_each_new_selector(self):
    legacy=M.canonical_planner(plan())
    versioned=M.canonical_planner(versioned_plan())
    digest=lambda value: M.hashlib.sha256(
      json.dumps(value,sort_keys=True,separators=(",",":")).encode()
    ).hexdigest()
    self.assertNotIn("execution_contract",legacy)
    self.assertNotIn("run_packaging_contracts",legacy)
    self.assertEqual(M.EXECUTION_CONTRACT,versioned["execution_contract"])
    self.assertNotEqual(digest(legacy),digest(versioned))
    for capability in M.VERSIONED_SELECTOR_CAPABILITIES.values():
      changed_raw=versioned_plan()
      selected={"packaging_contracts","pixel_world_bridge","viewer_js_required"}
      if capability in selected:
        selected.remove(capability)
      else:
        selected.add(capability)
      changed_raw["selected_capabilities"]=";".join(sorted(selected))
      for selector, selected_capability in M.VERSIONED_SELECTOR_CAPABILITIES.items():
        changed_raw[selector]="true" if selected_capability in selected else "false"
      changed=M.canonical_planner(changed_raw)
      self.assertNotEqual(digest(versioned),digest(changed),capability)
    changed_resource=versioned_plan(); changed_resource["needs_node"]="true"
    self.assertNotEqual(digest(versioned),digest(M.canonical_planner(changed_resource)),"needs_node")

  def test_planner_rejects_unknown_partial_and_mixed_execution_contracts(self):
    unknown=versioned_plan(); unknown["execution_contract"]="required-domain-split/v999"
    partial=versioned_plan(); partial.pop("run_doc_checker_contracts")
    mixed=plan(); mixed["run_packaging_contracts"]="false"
    noncanonical=versioned_plan(); noncanonical["run_packaging_contracts"]="TRUE"
    nonstring=versioned_plan(); nonstring["run_packaging_contracts"]=True
    resource_partial=versioned_plan(); resource_partial.pop("needs_wasm_target")
    resource_nonstring=versioned_plan(); resource_nonstring["needs_node"]=False
    baseline_resource_missing=versioned_plan(); baseline_resource_missing["needs_python"]="false"
    for raw in (unknown,partial,mixed,noncanonical,nonstring,resource_partial,resource_nonstring,baseline_resource_missing):
      with self.subTest(raw=raw):
        with self.assertRaisesRegex(SystemExit,"execution[-_]contract|incomplete|versioned|baseline"):
          M.canonical_planner(raw)
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
  def test_check_app_selector_is_mandatory(self):
    with self.assertRaisesRegex(SystemExit,"check app id is required"):
      M.live("eng-cc/oasis7",UID,1,7,"required-gate",None)
  def test_newer_wrong_app_success_cannot_mask_ruleset_bound_failure(self):
    required=run("failure",app=42)
    wrong=run("success",app=99999)
    wrong.update(id=10,completed_at="2026-07-14T00:01:00Z")
    with self.api(runs=[required,wrong]):
      with self.assertRaisesRegex(SystemExit,"conclusion=failure"):
        M.live("eng-cc/oasis7",UID,1,7,"required-gate","42")
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
  def test_selected_children_require_same_run_job_identity(self):
    selected=run(planner={**plan(),"run_operational_contracts":"true"})
    planner=M.canonical_planner({**plan(),"run_operational_contracts":"true"})
    digest=M.hashlib.sha256(json.dumps(planner,sort_keys=True,separators=(",",":")).encode()).hexdigest()
    argv=[str(P),"--repository","eng-cc/oasis7","--task-uid",UID,"--task-issue-number","1","--pr-number","7","--check-app-id","42","--planner-digest",digest]
    with self.api(runs=[selected]),patch.object(sys,"argv",argv),redirect_stdout(io.StringIO()):
      with self.assertRaisesRegex(SystemExit,"selected child jobs require same-run workflow attempt evidence"):
        M.main()
  def planner_from_artifact(self,meta=None,payload=None,data=None,run_id=12345):
    check=null_summary_run(run_id)
    artifacts={"artifacts":[meta if meta is not None else artifact(run_id)]}
    blob=artifact_zip(payload) if data is None else data
    with patch.object(M,"gh",return_value=artifacts),patch.object(M,"artifact_bytes",return_value=blob,create=True):
      return M.planner_for_run("eng-cc/oasis7",check,base_oid="b"*40,head_oid="a"*40)
  def planner_with_selected_children(self,raw,children,*,run_id=12345,attempt=2,gate_job_id=9009,gate_check_id=9,artifact_created="2026-09-25T10:10:00Z"):
    check=null_summary_run(run_id)
    check["id"]=gate_check_id
    check["details_url"]=f"https://github.com/eng-cc/oasis7/actions/runs/{run_id}/job/{gate_job_id}"
    payload=envelope(run_id=run_id,planner=raw)
    actions=selected_action_context(children,run_id=run_id,attempt=attempt,
      gate_job_id=gate_job_id,gate_check_id=gate_check_id,artifact_created=artifact_created)
    with patch.object(M,"gh",side_effect=actions),patch.object(M,"artifact_bytes",return_value=artifact_zip(payload)):
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
  def test_versioned_packaging_selection_requires_mac_and_permits_unselected_skips(self):
    children=[
      action_job(9101,M.WINDOWS_ROLLOUT_JOB,"windows-2022",status="completed",conclusion="skipped"),
      action_job(9102,M.MACOS_PACKAGE_JOB,"ubuntu-24.04",check_run_id=9202),
      *(action_job(9110+i,f"{M.FLEET_HEALTH_JOB} ({runner})",runner,status="completed",conclusion="skipped")
        for i,runner in enumerate(M.FLEET_HEALTH_RUNNERS)),
    ]
    raw=versioned_plan()
    planner=self.planner_with_selected_children(raw,children)
    outcome=planner["selected_child_job_outcomes"]
    self.assertEqual(12345,outcome["workflow_run_id"])
    self.assertEqual(2,outcome["run_attempt"])
    self.assertEqual(9009,outcome["required_gate_job_id"])
    self.assertEqual(9,outcome["required_gate_check_run_id"])
    self.assertEqual([M.MACOS_PACKAGE_JOB],[job["name"] for job in outcome["jobs"]])
    source_digest=M.hashlib.sha256(json.dumps(M.canonical_planner(raw),sort_keys=True,separators=(",",":")).encode()).hexdigest()
    self.assertEqual(source_digest,outcome["source_planner_digest"])
  def test_legacy_operational_selection_requires_windows_macos_and_all_fleet_children(self):
    raw=plan(); raw["run_operational_contracts"]="true"
    children=[
      action_job(9101,M.WINDOWS_ROLLOUT_JOB,"windows-2022"),
      action_job(9102,M.MACOS_PACKAGE_JOB,"ubuntu-24.04"),
      *(action_job(9110+i,f"{M.FLEET_HEALTH_JOB} ({runner})",runner)
        for i,runner in enumerate(M.FLEET_HEALTH_RUNNERS)),
    ]
    planner=self.planner_with_selected_children(raw,children)
    names={job["name"] for job in planner["selected_child_job_outcomes"]["jobs"]}
    expected={M.WINDOWS_ROLLOUT_JOB,M.MACOS_PACKAGE_JOB}
    expected.update(f"{M.FLEET_HEALTH_JOB} ({runner})" for runner in M.FLEET_HEALTH_RUNNERS)
    self.assertEqual(expected,names)
  def test_selected_child_failure_cancelled_skipped_or_missing_fails_closed(self):
    raw=versioned_plan()
    valid=action_job(9102,M.MACOS_PACKAGE_JOB,"ubuntu-24.04",check_run_id=9202)
    cases=(
      ("failed",[{**valid,"conclusion":"failure"}]),
      ("cancelled",[{**valid,"conclusion":"cancelled"}]),
      ("skipped",[{**valid,"conclusion":"skipped"}]),
      ("missing",[]),
    )
    for label,mac_jobs in cases:
      children=[action_job(9101,M.WINDOWS_ROLLOUT_JOB,"windows-2022",status="completed",conclusion="skipped"),*mac_jobs]
      with self.subTest(label=label):
        with self.assertRaisesRegex(SystemExit,"selected child job"):
          self.planner_with_selected_children(raw,children)
  def test_selected_child_wrong_attempt_and_plan_artifact_outside_attempt_fail_closed(self):
    raw=versioned_plan()
    mac=action_job(9102,M.MACOS_PACKAGE_JOB,"ubuntu-24.04",check_run_id=9202,attempt=1)
    with self.assertRaisesRegex(SystemExit,"wrong workflow attempt"):
      self.planner_with_selected_children(raw,[mac])
    mac=action_job(9102,M.MACOS_PACKAGE_JOB,"ubuntu-24.04",check_run_id=9202)
    with self.assertRaisesRegex(SystemExit,"not bound to required-gate workflow attempt"):
      self.planner_with_selected_children(raw,[mac],artifact_created="2026-09-25T10:30:00Z")
  def test_pre_envelope_trusted_workflow_accepts_only_complete_full_coverage(self):
    proof={"workflow_run_id":12345,"workflow_sha":"b"*40,
      "workflow_ref":"eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main",
      "run_attempt":1,"base_oid":"b"*40,"head_oid":"a"*40,"tested_tree_oid":"t"*40}
    check=run(); check["details_url"]="https://github.com/eng-cc/oasis7/actions/runs/12345/job/9"
    full=M.canonical_planner({**plan(),"scope":"full",**{field:"true" for field in M.RUN_FIELDS}})
    encoded=base64.b64encode(b"name: Rust\n# trusted workflow before package profile envelope producer\n").decode()
    workflow="\n".join(encoded[index:index+12] for index in range(0,len(encoded),12))+"\n"
    def read(*args):
      if "artifacts?" in args[-1]: return {"artifacts":[]}
      if "/contents/.github/workflows/rust.yml?ref=" in args[-1]: return {"encoding":"base64","content":workflow}
      raise AssertionError(args[-1])
    with patch.object(M,"gh",side_effect=read):
      disposition=M.cargo_package_profile_for_run(
        "eng-cc/oasis7",check,proof,full,task_uid=UID,task_issue_number=1,pr_number=7)
    self.assertEqual("legacy_required_coverage",disposition["execution_disposition"])
    self.assertIs(disposition["disposition_validated"],True)
    self.assertEqual("b"*40,disposition["workflow_sha"])
    receipt=self.invoke_verify()
    receipt.update(integration_run_id=12345,tested_tree_oid="t"*40,tested_commit_oid="c"*40,
                   workflow_sha="b"*40,cargo_package_profile=disposition)
    original=M.review_evidence_digest(receipt)
    receipt["cargo_package_profile"]={**disposition,"workflow_sha":"d"*40}
    self.assertNotEqual(original,M.review_evidence_digest(receipt))

  def test_trusted_workflow_source_rejects_non_base64_after_whitespace_normalization(self):
    with patch.object(M,"gh",return_value={"encoding":"base64","content":"bmFtZTogUnVzdAo=\n!"}):
      with self.assertRaisesRegex(SystemExit,"malformed"):
        M._trusted_workflow_source("eng-cc/oasis7","b"*40)

  def test_missing_envelope_fails_after_trusted_workflow_has_producer(self):
    proof={"workflow_run_id":12345,"workflow_sha":"b"*40,
      "workflow_ref":"eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main",
      "run_attempt":1,"base_oid":"b"*40,"head_oid":"a"*40,"tested_tree_oid":"t"*40}
    check=run()
    full=M.canonical_planner({**plan(),"scope":"full",**{field:"true" for field in M.RUN_FIELDS}})
    workflow=base64.b64encode(b"name: cargo-package-profile-envelope\n").decode()
    def read(*args):
      if "artifacts?" in args[-1]: return {"artifacts":[]}
      return {"encoding":"base64","content":workflow}
    with patch.object(M,"gh",side_effect=read):
      with self.assertRaisesRegex(SystemExit,"envelope-capable|artifact missing"):
        M.cargo_package_profile_for_run(
          "eng-cc/oasis7",check,proof,full,task_uid=UID,task_issue_number=1,pr_number=7)

  def test_checker_stage_receipt_disposition_accepts_exact_completed_run(self):
    proof={"workflow_run_id":12345,"workflow_sha":"w"*40,
      "workflow_ref":"eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main",
      "run_attempt":1,"base_oid":"b"*40,"head_oid":"a"*40,"tested_tree_oid":"c"*40}
    check=run()
    full=M.canonical_planner({**plan(),"scope":"full",**{field:"true" for field in M.RUN_FIELDS}})
    workflow=base64.b64encode(b"id: checker-stage\ngit diff --name-status --find-renames\ncargo-package-profile-envelope\ncargo-checker-stage-admission-receipt-\n").decode()
    def read(*args):
      path=args[-1]
      if "artifacts?" in path:return {"artifacts":[stage_artifact()]}
      if "/contents/.github/workflows/rust.yml?ref=" in path:return {"encoding":"base64","content":workflow}
      if "/compare/" in path:return {"merge_base_commit":{"sha":"b"*40}}
      if path.endswith("/issues/3827"):
        return {"number":3827,"state":"open","repository_url":"https://api.github.com/repos/eng-cc/oasis7",
          "body":f"task_uid: {A.CHECKER_TASK_UID}\n- pr_url: https://github.com/eng-cc/oasis7/pull/7\n"}
      if path.endswith("/pulls/7/files?per_page=100&page=1"):
        return [{"filename":value,"status":"modified"} for value in A.CHECKER_SCOPE]
      if "/pulls/7" in path:
        return {"number":7,"state":"open","merged":False,"body":f"Task: {A.CHECKER_TASK_UID}\n\nRefs #3827",
          "head":{"sha":"a"*40,"repo":{"full_name":"eng-cc/oasis7"}},"base":{"sha":"b"*40,"ref":"main","repo":{"full_name":"eng-cc/oasis7"}}}
      raise AssertionError(path)
    with patch.object(M,"gh",side_effect=read),patch.object(M,"artifact_bytes",return_value=stage_artifact_zip(stage_receipt(task_uid=A.CHECKER_TASK_UID,check_head="b"*40))):
      disposition=M.cargo_package_profile_for_run("eng-cc/oasis7",check,proof,full,task_uid=A.CHECKER_TASK_UID,task_issue_number=3827,pr_number=7)
    self.assertEqual("trusted_checker_stage_receipt",disposition["execution_disposition"])
    self.assertTrue(disposition["disposition_validated"])
    self.assertEqual(12345,disposition["run_id"])
    self.assertEqual("a"*40,disposition["source_head"])

  def test_checker_stage_receipt_rejects_source_head_as_check_identity(self):
    proof={"workflow_run_id":12345,"workflow_sha":"w"*40,
      "workflow_ref":"eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main",
      "run_attempt":1,"base_oid":"b"*40,"head_oid":"a"*40,"tested_tree_oid":"c"*40}
    check=run()
    full=M.canonical_planner({**plan(),"scope":"full",**{field:"true" for field in M.RUN_FIELDS}})
    workflow=base64.b64encode(b"id: checker-stage\ngit diff --name-status --find-renames\ncargo-package-profile-envelope\ncargo-checker-stage-admission-receipt-\n").decode()
    def read(*args):
      path=args[-1]
      if "artifacts?" in path:return {"artifacts":[stage_artifact()]}
      if "/contents/.github/workflows/rust.yml?ref=" in path:return {"encoding":"base64","content":workflow}
      if "/compare/" in path:return {"merge_base_commit":{"sha":"b"*40}}
      if path.endswith("/issues/3827"):
        return {"number":3827,"state":"open","repository_url":"https://api.github.com/repos/eng-cc/oasis7",
          "body":f"task_uid: {A.CHECKER_TASK_UID}\n- pr_url: https://github.com/eng-cc/oasis7/pull/7\n"}
      if path.endswith("/pulls/7/files?per_page=100&page=1"):
        return [{"filename":value,"status":"modified"} for value in A.CHECKER_SCOPE]
      if "/pulls/7" in path:
        return {"number":7,"state":"open","merged":False,"body":f"Task: {A.CHECKER_TASK_UID}\n\nRefs #3827",
          "head":{"sha":"a"*40,"repo":{"full_name":"eng-cc/oasis7"}},"base":{"sha":"b"*40,"ref":"main","repo":{"full_name":"eng-cc/oasis7"}}}
      raise AssertionError(path)
    with patch.object(M,"gh",side_effect=read),patch.object(M,"artifact_bytes",return_value=stage_artifact_zip(stage_receipt(task_uid=A.CHECKER_TASK_UID,check_head="a"*40))):
      with self.assertRaisesRegex(SystemExit,"receipt check identity mismatch"):
        M.cargo_package_profile_for_run("eng-cc/oasis7",check,proof,full,task_uid=A.CHECKER_TASK_UID,task_issue_number=3827,pr_number=7)

  def test_checker_stage_receipt_tamper_and_expiry_fail_closed(self):
    proof={"workflow_run_id":12345,"workflow_sha":"w"*40,
      "workflow_ref":"eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main",
      "run_attempt":1,"base_oid":"b"*40,"head_oid":"a"*40,"tested_tree_oid":"c"*40}
    check=run(); full=M.canonical_planner({**plan(),"scope":"full",**{field:"true" for field in M.RUN_FIELDS}})
    workflow=base64.b64encode(b"id: checker-stage\ngit diff --name-status --find-renames\ncargo-package-profile-envelope\ncargo-checker-stage-admission-receipt-\n").decode()
    for changed in ("digest", "head", "status", "scope", "wrong_task", "wrong_check", "wrong_base_ref", "wrong_base_repo", "wrong_head_repo", "expired", "missing", "wrong_run", "duplicate", "extra_stage"):
      receipt=stage_receipt(task_uid=A.CHECKER_TASK_UID)
      artifact_items=[] if changed=="missing" else ([stage_artifact(run_id=999)] if changed=="wrong_run" else ([stage_artifact(),stage_artifact()] if changed=="duplicate" else ([stage_artifact(),{"id":89,"name":"cargo-checker-stage-admission-receipt-extra","expired":False,"workflow_run":{"id":12345}}] if changed=="extra_stage" else [stage_artifact(expired=changed=="expired")])) )
      if changed=="digest": receipt["receipt_digest"]="sha256:"+"e"*64
      elif changed=="head": receipt["head_oid"]="c"*40; receipt["result"]["head_oid"]="c"*40; receipt["receipt_digest"]=A.durable_receipt_digest(receipt)
      elif changed=="status": receipt["result"]["status"]="failed"; receipt["result"]["exit_code"]=1; receipt["receipt_digest"]=A.durable_receipt_digest(receipt)
      elif changed=="scope": receipt["scope_base_oid"]="c"*40; receipt["result"]["scope_base_oid"]="c"*40; receipt["receipt_digest"]=A.durable_receipt_digest(receipt)
      elif changed=="wrong_task": receipt["task_uid"]="task_ffffffffffffffffffffffffffffffff"; receipt["receipt_digest"]=A.durable_receipt_digest(receipt)
      elif changed=="wrong_check": receipt["check"]["check_run_id"]=99; receipt["receipt_digest"]=A.durable_receipt_digest(receipt)
      def read(*args):
        path=args[-1]
        if "artifacts?" in path:return {"artifacts":artifact_items}
        if "/contents/.github/workflows/rust.yml?ref=" in path:return {"encoding":"base64","content":workflow}
        if "/compare/" in path:return {"merge_base_commit":{"sha":"b"*40}}
        if path.endswith("/issues/3827"):
          return {"number":3827,"state":"open","repository_url":"https://api.github.com/repos/eng-cc/oasis7",
            "body":f"task_uid: {A.CHECKER_TASK_UID}\n- pr_url: https://github.com/eng-cc/oasis7/pull/7\n"}
        if path.endswith("/pulls/7/files?per_page=100&page=1"):
          return [{"filename":value,"status":"modified"} for value in A.CHECKER_SCOPE]
        if "/pulls/7" in path:
          base={"sha":"b"*40,"ref":"main","repo":{"full_name":"eng-cc/oasis7"}}
          head={"sha":"a"*40,"repo":{"full_name":"eng-cc/oasis7"}}
          if changed=="wrong_base_ref": base["ref"]="release"
          elif changed=="wrong_base_repo": base["repo"]={"full_name":"other/repo"}
          elif changed=="wrong_head_repo": head["repo"]={"full_name":"fork/oasis7"}
          return {"number":7,"state":"open","merged":False,"body":f"Task: {A.CHECKER_TASK_UID}\n\nRefs #3827","head":head,"base":base}
        raise AssertionError(path)
      with self.subTest(changed=changed),patch.object(M,"gh",side_effect=read),patch.object(M,"artifact_bytes",return_value=stage_artifact_zip(receipt)):
        with self.assertRaisesRegex(SystemExit,"stage|receipt|expired|digest|identity|successful|mismatch|repository|base|canonical"):
          M.cargo_package_profile_for_run("eng-cc/oasis7",check,proof,full,task_uid=A.CHECKER_TASK_UID,task_issue_number=3827,pr_number=7)
if __name__=="__main__": unittest.main()
