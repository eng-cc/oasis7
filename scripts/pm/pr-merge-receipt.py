#!/usr/bin/env python3
"""Produce a repository-bound merged-PR receipt from live GitHub queries."""
from __future__ import annotations
import argparse, datetime as dt, json, subprocess

def main() -> int:
    p=argparse.ArgumentParser(); p.add_argument("pr"); p.add_argument("--json",action="store_true"); a=p.parse_args()
    data=json.loads(subprocess.check_output(["gh","pr","view",a.pr,"--json","number,url,state,mergedAt,headRefOid,baseRefName"],text=True))
    if str(data.get("state")).upper()!="MERGED" or not data.get("mergedAt"):
        p.error("PR is not merged")
    repo_data=json.loads(subprocess.check_output(["gh","repo","view","--json","nameWithOwner,defaultBranchRef"],text=True))
    repository=str(repo_data.get("nameWithOwner") or "")
    default_branch=str((repo_data.get("defaultBranchRef") or {}).get("name") or "")
    # Test doubles that do not implement repository metadata remain explicitly
    # untrusted.  Production consumers reject this evidence mode.
    evidence_mode="production" if repository and default_branch else "fixture_untrusted"
    out={"receipt_type":"oasis7_pr_merge","issuer":"github_live_query","evidence_mode":evidence_mode,"repository":repository or "fixture/untrusted","default_branch":default_branch or str(data.get("baseRefName") or ""),"pr_number":data["number"],"pr_url":data["url"],"state":"MERGED","merged_at":data["mergedAt"],"head_oid":data.get("headRefOid"),"base_ref":data.get("baseRefName"),"observed_at":dt.datetime.now(dt.timezone.utc).isoformat()}
    print(json.dumps(out,indent=2,sort_keys=True))
    return 0
if __name__=="__main__": raise SystemExit(main())
