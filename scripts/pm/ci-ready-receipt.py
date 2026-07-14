#!/usr/bin/env python3
"""Issue/verify a live GitHub CI receipt for a frozen draft-candidate head."""
import argparse, datetime as dt, hashlib, json, re, subprocess, sys
from pathlib import Path

FAIL_STATES = ("stale", "wrong_head", "wrong_app", "superseded", "cancelled", "uncertain")
PLAN_MARKER="oasis7-required-plan-v1"
RUN_FIELDS=("run_oasis7_required_tests","run_consensus_tests","run_distfs_tests","run_oasis7_node_tests","run_oasis7_net_tests","run_oasis7_net_libp2p_tests","run_viewer_contract_tests","run_viewer_wasm_check","run_viewer_perf_smoke","run_launcher_web_build","run_oasis7_workspace_support_crate_tests")

def planner_from_run(run):
    output=run.get("output") or {}; text="\n".join(str(output.get(k) or "") for k in ("summary","text"))
    m=re.search(r"oasis7-required-plan-v1\s*-->\s*```json\s*(\{.*?\})\s*```",text,re.S)
    if not m: raise SystemExit("ci-ready-receipt: uncertain planner metadata marker missing")
    try: raw=json.loads(m.group(1))
    except Exception as exc: raise SystemExit(f"ci-ready-receipt: uncertain malformed planner metadata: {exc}")
    required=("scope","reason_summary","changed_path_count",*RUN_FIELDS)
    if any(k not in raw for k in required): raise SystemExit("ci-ready-receipt: uncertain incomplete planner metadata")
    if any(str(raw[k]).lower() not in ("true","false") for k in RUN_FIELDS): raise SystemExit("ci-ready-receipt: uncertain non-boolean planner metadata")
    try: changed=int(raw["changed_path_count"])
    except Exception: raise SystemExit("ci-ready-receipt: uncertain invalid changed_path_count")
    plan={"schema":PLAN_MARKER,"scope":str(raw["scope"]),"reason_summary":str(raw["reason_summary"]),"changed_path_count":changed}
    plan.update({k:str(raw[k]).lower()=="true" for k in RUN_FIELDS})
    return plan

def gh(*args):
    try:
        return json.loads(subprocess.check_output(["gh", *args], text=True, stderr=subprocess.PIPE))
    except Exception as exc:
        raise SystemExit(f"ci-ready-receipt: uncertain GitHub read: {exc}")

def now(): return dt.datetime.now(dt.timezone.utc).isoformat()

def live(repository, task_uid, task_issue_number, pr_number, check_name, check_app_id):
    pr=gh("api",f"repos/{repository}/pulls/{pr_number}")
    if not pr.get("draft"): raise SystemExit("ci-ready-receipt: superseded: PR is not a draft candidate")
    body=str(pr.get("body") or "")
    if f"Task: {task_uid}" not in body or f"Refs #{task_issue_number}" not in body:
        raise SystemExit("ci-ready-receipt: uncertain task-to-PR linkage missing")
    head_oid=pr["head"]["sha"]; base_oid=pr["base"]["sha"]
    runs=[]
    for page in range(1,101):
        batch=gh("api",f"repos/{repository}/commits/{head_oid}/check-runs?per_page=100&page={page}").get("check_runs",[])
        runs.extend(batch)
        if len(batch)<100: break
    else: raise SystemExit("ci-ready-receipt: uncertain check-run pagination overflow")
    matches=[]
    for run in runs:
        app_id=(run.get("app") or {}).get("id")
        if run.get("name")==check_name and (check_app_id is None or str(app_id)==str(check_app_id)):
            matches.append(run)
    if not matches: raise SystemExit("ci-ready-receipt: wrong_app or uncertain: required check identity missing")
    matches.sort(key=lambda x:(x.get("completed_at") or "",int(x.get("id") or 0)),reverse=True)
    run=matches[0]
    if run.get("status")!="completed": raise SystemExit("ci-ready-receipt: uncertain: check incomplete")
    conclusion=str(run.get("conclusion") or "").lower()
    if conclusion=="cancelled": raise SystemExit("ci-ready-receipt: cancelled")
    if conclusion!="success": raise SystemExit(f"ci-ready-receipt: required check conclusion={conclusion or 'uncertain'}")
    return pr,run,base_oid,head_oid

def main():
    p=argparse.ArgumentParser()
    p.add_argument("--repository",required=True); p.add_argument("--task-uid",required=True)
    p.add_argument("--task-issue-number",required=True,type=int)
    p.add_argument("--pr-number",required=True,type=int); p.add_argument("--check-name",default="required-gate")
    p.add_argument("--check-app-id"); p.add_argument("--planner-digest",required=True)
    p.add_argument("--receipt"); p.add_argument("--json",action="store_true")
    a=p.parse_args(); pr,run,base_oid,head_oid=live(a.repository,a.task_uid,a.task_issue_number,a.pr_number,a.check_name,a.check_app_id)
    planner=planner_from_run(run)
    trusted_planner_digest=hashlib.sha256(json.dumps(planner,sort_keys=True,separators=(",",":")).encode()).hexdigest()
    if a.planner_digest not in ("auto",trusted_planner_digest):
        raise SystemExit("ci-ready-receipt: uncertain planner_digest does not match live check metadata")
    payload={"receipt_type":"oasis7_ci_ready_receipt","issuer":"github_live_query","repository":a.repository,
      "task_uid":a.task_uid,"task_issue_number":a.task_issue_number,"pr_number":a.pr_number,"base_oid":base_oid,"head_oid":head_oid,
      "check_name":a.check_name,"check_app_id":(run.get("app") or {}).get("id"),"check_run_id":run.get("id"),
      "planner_digest":trusted_planner_digest,"planner":planner,"conclusion":"success","observed_at":now()}
    if a.receipt:
        old=json.loads(Path(a.receipt).read_text(encoding="utf-8"))
        for key,val in payload.items():
            if key=="observed_at": continue
            if old.get(key)!=val: raise SystemExit(f"ci-ready-receipt: wrong_head/wrong_app/superseded receipt mismatch: {key}")
        seen=dt.datetime.fromisoformat(str(old["observed_at"]).replace("Z","+00:00"))
        if not 0 <= (dt.datetime.now(dt.timezone.utc)-seen).total_seconds() <= 600:
            raise SystemExit("ci-ready-receipt: stale")
        payload=old
    print(json.dumps(payload,sort_keys=True,indent=2 if a.json else None))
if __name__=="__main__": main()
