#!/usr/bin/env python3
"""Fail-closed config-driven required-gate planner."""
import argparse, fnmatch, hashlib, importlib.util, json, re, subprocess, sys
from pathlib import Path

EXECUTION_CONTRACT="required-domain-split/v1"
LEGACY_CAPABILITIES=("oasis7_required","consensus","distfs","node","net","viewer_js_required","viewer_performance_report","pixel_world_bridge","launcher_web","workspace_support","scenario_regression","operational_contracts","packaging_contracts","workflow_governance","codex_agent_config_validation","compile_metrics","required_gate_baseline","site_quality")
CAPABILITIES=LEGACY_CAPABILITIES+ ("doc_checker_contracts","cargo_tooling_contracts")
LEGACY_FIELDS={"oasis7_required":"run_oasis7_required_tests","consensus":"run_consensus_tests","distfs":"run_distfs_tests","node":"run_oasis7_node_tests","net":"run_oasis7_net_tests","viewer_js_required":"run_viewer_contract_tests","viewer_performance_report":"run_viewer_perf_smoke","pixel_world_bridge":"run_pixel_world_bridge_lib_tests","launcher_web":"run_launcher_web_build","workspace_support":"run_oasis7_workspace_support_crate_tests","scenario_regression":"run_scenario_regression","operational_contracts":"run_operational_contracts","packaging_contracts":"run_operational_contracts","workflow_governance":"run_operational_contracts","codex_agent_config_validation":"run_codex_agent_config_validation","compile_metrics":"run_compile_metrics_contract_tests","required_gate_baseline":"run_required_gate_baseline","site_quality":"run_site_contract_tests"}
FIELDS={**LEGACY_FIELDS,"packaging_contracts":"run_packaging_contracts","workflow_governance":"run_workflow_governance_contracts","doc_checker_contracts":"run_doc_checker_contracts","cargo_tooling_contracts":"run_cargo_tooling_contracts"}
DERIVED_OUTPUT_FIELDS={"run_oasis7_net_libp2p_tests","run_viewer_wasm_check","run_pixel_world_bridge_wasm_check","run_rust_baseline"}
PLANNER_OUTPUT_FIELDS=set(FIELDS.values())|DERIVED_OUTPUT_FIELDS
LEGACY_PLANNER_OUTPUT_FIELDS=set(LEGACY_FIELDS.values())|DERIVED_OUTPUT_FIELDS
VERSIONED_SELECTOR_FIELDS={
  "run_workflow_governance_contracts",
  "run_packaging_contracts",
  "run_doc_checker_contracts",
  "run_cargo_tooling_contracts",
}
VERSIONED_SELECTOR_NAMES={
  "OASIS7_CI_RUN_WORKFLOW_GOVERNANCE_CONTRACTS",
  "OASIS7_CI_RUN_PACKAGING_CONTRACTS",
  "OASIS7_CI_RUN_DOC_CHECKER_CONTRACTS",
  "OASIS7_CI_RUN_CARGO_TOOLING_CONTRACTS",
}
RESOURCE_NAMES={"python","markdown","rust_toolchain","node","system_deps","trunk","wasm_target"}
def die(m): raise SystemExit("plan-rust-required-scope: "+m)

def load_impact_projection(path, expected):
  helper_path=Path(__file__).parent / "pm" / "workflow-impact-projection.py"
  spec=importlib.util.spec_from_file_location("oasis7_workflow_impact_projection", helper_path)
  if spec is None or spec.loader is None: die("impact projection adapter is unavailable")
  helper=importlib.util.module_from_spec(spec); spec.loader.exec_module(helper)
  try:
    value=helper.load_verified_projection(path, expected=expected, repo_root=Path.cwd())
  except Exception as exc:
    die(f"impact projection is invalid: {exc}")
  return value

def policy_full_projection_for_unverified_closure(projection, capabilities=CAPABILITIES):
  closure=projection.get("closure_status")
  if not isinstance(closure,dict) or closure.get("status")=="complete": return False
  status=closure.get("status")
  reasons=projection.get("ci_reasons")
  identity=projection.get("planner_identity")
  capabilities=sorted(capabilities)
  return (
    isinstance(status,str)
    and isinstance(reasons,list)
    and "dependency_closure_unverified:"+status in reasons
    and projection.get("ci_scope")=="full"
    and projection.get("test_profile")=="full"
    and projection.get("ci_capabilities")==capabilities
    and isinstance(identity,dict)
    and identity.get("scope")=="full"
    and identity.get("selected_capabilities")==capabilities
  )

def config(path):
  try: raw=Path(path).read_bytes(); c=json.loads(raw)
  except Exception as e: die(f"invalid config: {e}")
  if not isinstance(c,dict) or c.get("schema")!="oasis7-ci-required-scope/v2" or c.get("unmatched")!="full" or not isinstance(c.get("rules"),list): die("invalid config schema")
  execution_contract=c.get("execution_contract")
  if execution_contract is None:
    if "resource_requirements" in c or "baseline_resources" in c:
      die("legacy config cannot declare versioned resource fields")
    if c.get("capabilities")!=list(LEGACY_CAPABILITIES): die("invalid legacy config schema")
    legacy=True; allowed_capabilities=set(LEGACY_CAPABILITIES); allowed_outputs=LEGACY_PLANNER_OUTPUT_FIELDS
  else:
    if execution_contract!=EXECUTION_CONTRACT: die("unsupported execution_contract")
    if c.get("capabilities")!=list(CAPABILITIES): die("invalid versioned config capabilities")
    legacy=False; allowed_capabilities=set(CAPABILITIES); allowed_outputs=PLANNER_OUTPUT_FIELDS
    baseline_resources=c.get("baseline_resources")
    if (not isinstance(baseline_resources,list)
        or any(not isinstance(item,str) or item not in RESOURCE_NAMES for item in baseline_resources)
        or len(baseline_resources)!=len(set(baseline_resources))
        or not {"python","markdown"}.issubset(baseline_resources)):
      die("invalid versioned baseline resource requirements")
    resource_requirements=c.get("resource_requirements")
    if not isinstance(resource_requirements,dict) or set(resource_requirements)!=allowed_capabilities:
      die("incomplete versioned resource requirements")
    for capability, resources in resource_requirements.items():
      if (not isinstance(resources,list)
          or any(not isinstance(item,str) or item not in RESOURCE_NAMES for item in resources)
          or len(resources)!=len(set(resources))):
        die("invalid versioned resource requirement for "+capability)
    doc_resources=set(resource_requirements["doc_checker_contracts"])
    cargo_resources=set(resource_requirements["cargo_tooling_contracts"])
    if doc_resources!={"python","markdown"} or "rust_toolchain" in doc_resources:
      die("doc checker resource requirements must be Python and Markdown without Rust")
    if "rust_toolchain" not in cargo_resources:
      die("cargo tooling resource requirements must include Rust toolchain")
  ownership=c.get("selector_ownership")
  if not isinstance(ownership,list) or not ownership: die("invalid selector ownership registry")
  declared={}
  for item in ownership:
    if not isinstance(item,dict): die("invalid selector ownership entry")
    name=item.get("name")
    if not isinstance(name,str) or not re.fullmatch(r"OASIS7_CI_RUN_[A-Z0-9_]+",name) or name in declared: die("invalid selector ownership name")
    mode=item.get("mode")
    if mode=="planner-owned":
      field=item.get("planner_field")
      if field not in allowed_outputs or "owner" in item or "reason" in item: die("invalid planner-owned selector metadata")
    elif mode=="manual-only":
      if not isinstance(item.get("owner"),str) or not item["owner"] or not isinstance(item.get("reason"),str) or not item["reason"] or "planner_field" in item: die("invalid manual-only selector metadata")
    else: die("invalid selector ownership mode")
    declared[name]=item
  selector_source=Path(__file__).with_name("ci-tests.sh")
  if not selector_source.is_file():
    die(f"selector source is missing: {selector_source}")
  try:
    inventory=set(re.findall(r"OASIS7_CI_RUN_[A-Z0-9_]+",selector_source.read_text(encoding="utf-8")))
  except OSError as e:
    die(f"selector source is unreadable: {e}")
  expected_inventory=inventory if not legacy else inventory-VERSIONED_SELECTOR_NAMES
  if expected_inventory != set(declared):
    die("selector ownership registry does not match ci-tests selectors")
  if not legacy:
    selector_fields={item.get("name"):item.get("planner_field") for item in ownership if item.get("mode")=="planner-owned"}
    expected_versioned={
      "OASIS7_CI_RUN_WORKFLOW_GOVERNANCE_CONTRACTS":"run_workflow_governance_contracts",
      "OASIS7_CI_RUN_PACKAGING_CONTRACTS":"run_packaging_contracts",
      "OASIS7_CI_RUN_DOC_CHECKER_CONTRACTS":"run_doc_checker_contracts",
      "OASIS7_CI_RUN_CARGO_TOOLING_CONTRACTS":"run_cargo_tooling_contracts",
    }
    if any(selector_fields.get(name)!=field for name,field in expected_versioned.items()):
      die("versioned selector ownership is incomplete or aliased")
  reasons=set()
  for r in c["rules"]:
    if not isinstance(r,dict) or not isinstance(r.get("match"),list) or not r["match"] or any(not isinstance(x,str) or not x for x in r["match"]): die("invalid config rule patterns")
    if not isinstance(r.get("reason"),str) or not r["reason"] or r["reason"] in reasons: die("invalid config rule reason")
    reasons.add(r["reason"])
    if not isinstance(r.get("full",False),bool) or not isinstance(r.get("minimal",False),bool): die("invalid config rule selectors")
    if not isinstance(r.get("requires_rust",False),bool): die("invalid config rule requires_rust selector")
    if not isinstance(r.get("capabilities",[]),list) or any(not isinstance(x,str) for x in r.get("capabilities",[])) or (not r.get("full") and not r.get("minimal") and not r.get("capabilities")) or not set(r.get("capabilities",[])).issubset(allowed_capabilities): die("invalid config rule capabilities")
  return c,"sha256:"+hashlib.sha256(raw).hexdigest(),legacy
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
 p=argparse.ArgumentParser(); p.add_argument("--event-name",required=True);p.add_argument("--run-mode",choices=("legacy","integration_revalidation","full_escalation"),default="legacy");p.add_argument("--base-ref");p.add_argument("--head-ref");p.add_argument("--task-uid");p.add_argument("--scope-base-oid");p.add_argument("--changed-path",action="append",default=[]);p.add_argument("--github-output");p.add_argument("--config",default=str(Path(__file__).with_name("ci-required-scope.v2.json")));p.add_argument("--impact-projection",help="verified digest-bound workflow impact projection");a=p.parse_args()
 c,digest,legacy=config(a.config); active_capabilities=LEGACY_CAPABILITIES if legacy else CAPABILITIES
 fields=LEGACY_FIELDS if legacy else FIELDS
 paths=a.changed_path or git_paths(a); projection=None
 source_scope_base=""
 if a.base_ref and a.head_ref:
  try: source_scope_base=subprocess.check_output(["git","merge-base",a.base_ref,a.head_ref],text=True).strip()
  except Exception: source_scope_base=""
 if a.impact_projection:
  if paths is None: die("impact projection requires resolvable changed paths")
  if not a.task_uid or not a.head_ref or not a.scope_base_oid: die("impact projection requires --task-uid, --head-ref and --scope-base-oid")
  if not a.changed_path:
   try:
    resolved_head=subprocess.check_output(["git","rev-parse",f"{a.head_ref}^{{commit}}"],text=True).strip()
    resolved_base=subprocess.check_output(["git","rev-parse",f"{a.scope_base_oid}^{{commit}}"],text=True).strip()
    merge_base=subprocess.check_output(["git","merge-base",a.scope_base_oid,a.head_ref],text=True).strip()
   except Exception as exc: die(f"impact projection git identity cannot be verified: {exc}")
   if resolved_head!=a.head_ref or resolved_base!=a.scope_base_oid or merge_base!=a.scope_base_oid: die("impact projection git head/base identity mismatch")
  # A pull-request projection describes the source-review range and therefore
  # must match the paths selected by that event.  Integration revalidation is
  # different: the trusted workflow deliberately plans the complete required
  # gate against the current target plus the unchanged source.  Target-only
  # commits can add paths to that execution range, so requiring the projection's
  # source paths to equal the integration diff would reject a valid full run.
  projection_expected={"task_uid":a.task_uid,"source_head_oid":a.head_ref,"scope_base_oid":a.scope_base_oid}
  if a.event_name != "workflow_dispatch":
   projection_expected["changed_paths"]=paths
  projection=load_impact_projection(a.impact_projection,projection_expected)
 full=a.run_mode=="full_escalation" or (a.run_mode=="legacy" and a.event_name=="workflow_dispatch") or paths is None or (projection is not None and projection["test_profile"]=="full"); capabilities=set(); explicit_rust=False; reasons=["required_gate_baseline:always_on"]
 if a.base_ref and a.head_ref and not source_scope_base:
  full=True; reasons.append("unresolvable_source_scope_base")
 if paths is None: paths=[]; reasons.append("unresolvable_changed_paths")
 for path in paths:
  hits=[r for r in c["rules"] if any(fnmatch.fnmatchcase(path,x) for x in r["match"])]
  if not hits: full=True; reasons.append("unclassified_or_unresolvable:"+path)
  for r in hits:
   capabilities.update(r.get("capabilities",[]))
   full|=bool(r.get("full")); explicit_rust|=bool(r.get("requires_rust",False)); reasons.append(r["reason"]+":"+path)
 if full: capabilities=set(active_capabilities)
 vals={f:"false" for f in fields.values()}
 vals.update({fields[x]:"true" for x in capabilities})
 vals["run_required_gate_baseline"]="true"
 if legacy:
  requires_rust=full or explicit_rust or bool(capabilities-{"workflow_governance","codex_agent_config_validation","compile_metrics","viewer_performance_report","operational_contracts","packaging_contracts","site_quality"})
  resources={
    "rust_toolchain":requires_rust,
    "node":bool(capabilities & {"viewer_js_required","viewer_performance_report","launcher_web"}),
    "system_deps":bool(capabilities & {"oasis7_required","viewer_js_required","viewer_performance_report","pixel_world_bridge","launcher_web"}),
    "wasm_target":bool(capabilities & {"pixel_world_bridge","launcher_web"}),
    "trunk":"launcher_web" in capabilities,
  }
 else:
  resources=set(c["baseline_resources"])
  for capability in capabilities:
   resources.update(c["resource_requirements"][capability])
  if explicit_rust: resources.add("rust_toolchain")
  requires_rust="rust_toolchain" in resources
  resources={name:name in resources for name in RESOURCE_NAMES}
  vals["execution_contract"]=EXECUTION_CONTRACT
  vals["needs_python"]="true" if resources["python"] else "false"
  vals["needs_markdown"]="true" if resources["markdown"] else "false"
 vals.update({"run_oasis7_net_libp2p_tests":vals["run_oasis7_net_tests"],"run_viewer_wasm_check":vals["run_viewer_contract_tests"],"run_pixel_world_bridge_wasm_check":vals["run_pixel_world_bridge_lib_tests"],"run_rust_baseline":"true" if requires_rust else "false","needs_rust_toolchain":"true" if resources["rust_toolchain"] else "false","needs_node":"true" if resources["node"] else "false","needs_system_deps":"true" if resources["system_deps"] else "false","needs_wasm_target":"true" if resources["wasm_target"] else "false","needs_trunk":"true" if resources["trunk"] else "false","planner_config_sha256":digest,"source_scope_base":source_scope_base,"integration_base":a.base_ref or "","source_head":a.head_ref or "HEAD","selected_capabilities":";".join(sorted(capabilities or {"required_gate_baseline"})),"scope":"full" if full else ("targeted" if capabilities else "minimal"),"reason_summary":";".join(dict.fromkeys(reasons)),"changed_path_count":str(len(paths)),"changed_paths":";".join(paths)})
 vals["required_test_units"]=";".join(sorted({"required_gate_baseline",*capabilities}))
 if projection is not None:
  actual_capabilities=sorted(capabilities or {"required_gate_baseline"})
  actual_scope=vals["scope"]
  if projection["planner_config_sha256"] != digest: die("impact projection planner config identity mismatch")
  # Integration execution may include target-only paths, but the projection's
  # source plan must still match the independently resolved source-only diff.
  if a.event_name=="workflow_dispatch" and a.run_mode=="integration_revalidation":
   source_paths=(projection["changed_paths"] if a.changed_path else
     git_paths(argparse.Namespace(base_ref=a.scope_base_oid,head_ref=a.head_ref,event_name="pull_request")))
   if source_paths is None or sorted(source_paths)!=projection["changed_paths"]:
    die("impact projection source changed paths identity mismatch")
   source_cmd=[sys.executable,str(Path(__file__).resolve()),"--event-name","pull_request","--config",a.config]
   for source_path in source_paths: source_cmd.extend(("--changed-path",source_path))
   source_plan=subprocess.run(source_cmd,text=True,capture_output=True)
   if source_plan.returncode: die("impact projection source planner cannot be verified: "+source_plan.stderr.strip())
   source_fields=dict(line.split("=",1) for line in source_plan.stdout.splitlines() if "=" in line)
   source_capabilities=source_fields.get("selected_capabilities","").split(";")
   unverified_closure=projection["closure_status"]["status"]!="complete"
   policy_full=policy_full_projection_for_unverified_closure(projection,active_capabilities)
   if unverified_closure and not policy_full:
    die("impact projection with unverified dependency closure is not full")
   if policy_full:
    if actual_scope!="full" or actual_capabilities!=sorted(active_capabilities):
     die("unverified dependency closure integration execution scope is not full")
   else:
    if projection["ci_scope"]!=source_fields.get("scope"):
     die("impact projection source scope identity mismatch")
    if projection["ci_capabilities"]!=source_capabilities:
     die("impact projection source capabilities identity mismatch")
   if not set(source_capabilities).issubset(set(actual_capabilities)|{"required_gate_baseline"}):
    die("integration execution omits source capabilities")
  # Full-only modes still require the complete gate; other modes require
  # equality unless independently verified target-only paths widen execution.
  full_only_mode = a.run_mode=="full_escalation" or (a.run_mode=="legacy" and a.event_name=="workflow_dispatch")
  integration_widened = a.run_mode=="integration_revalidation" and a.event_name=="workflow_dispatch"
  if not full_only_mode and not integration_widened:
   if projection["ci_scope"] != actual_scope: die("impact projection planner scope identity mismatch")
   if projection["ci_capabilities"] != actual_capabilities: die("impact projection planner capabilities identity mismatch")
  elif full_only_mode and (actual_scope != "full" or actual_capabilities != sorted(active_capabilities)):
   die("full-only impact projection execution scope is not full")
  vals.update({"impact_projection_schema":projection["schema"],"impact_projection_digest":projection["projection_digest"],"impact_projection_status":"verified","test_profile":projection["test_profile"],"declared_tests":";".join(projection["declared_tests"]),"planner_digest":projection["planner_digest"]})
 text="\n".join(f"{k}={v}" for k,v in vals.items())+"\n"
 if a.github_output: Path(a.github_output).open("a").write(text)
 else: print(text,end="")
if __name__=="__main__": main()
