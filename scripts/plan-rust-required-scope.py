#!/usr/bin/env python3
"""Fail-closed config-driven required-gate planner."""
import argparse, fnmatch, hashlib, json, subprocess, sys
from pathlib import Path

GATES=("oasis7_required","consensus","distfs","node","net","viewer","launcher_web","workspace_support")
FIELDS={"oasis7_required":"run_oasis7_required_tests","consensus":"run_consensus_tests","distfs":"run_distfs_tests","node":"run_oasis7_node_tests","net":"run_oasis7_net_tests","viewer":"run_viewer_contract_tests","launcher_web":"run_launcher_web_build","workspace_support":"run_oasis7_workspace_support_crate_tests"}
def die(m): raise SystemExit("plan-rust-required-scope: "+m)
def config(path):
  try: raw=Path(path).read_bytes(); c=json.loads(raw)
  except Exception as e: die(f"invalid config: {e}")
  if c.get("schema")!="oasis7-ci-required-scope/v1" or c.get("gates")!=list(GATES) or c.get("unmatched")!="full" or not isinstance(c.get("rules"),list): die("invalid config schema")
  reasons=set()
  for r in c["rules"]:
    if not isinstance(r,dict) or not isinstance(r.get("match"),list) or not r["match"] or any(not isinstance(x,str) or not x for x in r["match"]): die("invalid config rule patterns")
    if not isinstance(r.get("reason"),str) or not r["reason"] or r["reason"] in reasons: die("invalid config rule reason")
    reasons.add(r["reason"])
    if not isinstance(r.get("gates",[]),list) or any(not isinstance(x,str) for x in r.get("gates",[])) or (not r.get("full") and not r.get("minimal") and not r.get("gates")) or not set(r.get("gates",[])).issubset(GATES): die("invalid config rule gates")
  return c,"sha256:"+hashlib.sha256(raw).hexdigest()
def git_paths(a):
  if not a.base_ref: return None
  try:
    head=a.head_ref or "HEAD"; base=subprocess.check_output(["git","merge-base",a.base_ref,head],text=True).strip() if a.event_name=="pull_request" else a.base_ref
    out=subprocess.check_output(["git","diff","--name-status","--find-renames",base,head],text=True)
  except Exception: return None
  paths=[]
  for line in out.splitlines():
    p=line.split("\t")[1:]
    paths.extend(p if len(p)>1 else p[:1])
  return paths
def main():
 p=argparse.ArgumentParser(); p.add_argument("--event-name",required=True);p.add_argument("--base-ref");p.add_argument("--head-ref");p.add_argument("--changed-path",action="append",default=[]);p.add_argument("--github-output");p.add_argument("--config",default=str(Path(__file__).with_name("ci-required-scope.v1.json")));a=p.parse_args()
 c,digest=config(a.config); paths=a.changed_path or git_paths(a); full=a.event_name=="workflow_dispatch" or paths is None; gates=set(); reasons=[]
 if paths is None: paths=[]; reasons.append("unresolvable_changed_paths")
 for path in paths:
  hits=[r for r in c["rules"] if any(fnmatch.fnmatchcase(path,x) for x in r["match"])]
  if not hits: full=True; reasons.append("unclassified_or_unresolvable:"+path)
  for r in hits:
   gates.update(r.get("gates",[])); full|=bool(r.get("full")); reasons.append(r["reason"]+":"+path)
 if full: gates=set(GATES)
 vals={f:"false" for f in FIELDS.values()}; vals.update({FIELDS[x]:"true" for x in gates})
 vals.update({"run_oasis7_net_libp2p_tests":vals["run_oasis7_net_tests"],"run_viewer_wasm_check":vals["run_viewer_contract_tests"],"run_viewer_perf_smoke":vals["run_viewer_contract_tests"],"run_rust_baseline":"true" if full or gates else "false","needs_rust_toolchain":"true" if full or gates else "false","needs_node":"true" if gates & {"viewer","launcher_web"} else "false","needs_system_deps":"true" if gates & {"oasis7_required","viewer","launcher_web"} else "false","needs_wasm_target":"true" if "launcher_web" in gates else "false","needs_trunk":"true" if "launcher_web" in gates else "false","planner_config_sha256":digest,"scope":"full" if full else ("targeted" if gates else "minimal"),"reason_summary":";".join(dict.fromkeys(reasons or ["no_required_gate_inputs_changed"])),"changed_path_count":str(len(paths)),"changed_paths":";".join(paths)})
 text="\n".join(f"{k}={v}" for k,v in vals.items())+"\n"
 if a.github_output: Path(a.github_output).open("a").write(text)
 else: print(text,end="")
if __name__=="__main__": main()
