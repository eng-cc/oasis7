#!/usr/bin/env python3
"""Issue/verify a live GitHub CI receipt for a frozen draft-candidate head."""
import argparse, base64, datetime as dt, hashlib, io, json, re, subprocess, sys, zipfile
from pathlib import Path
from ci_ready_receipt_identity import review_evidence_digest, review_evidence_identity

FAIL_STATES = ("stale", "wrong_head", "wrong_app", "superseded", "cancelled", "uncertain")
PLAN_MARKER="oasis7-required-plan-v1"
PLAN_ARTIFACT=PLAN_MARKER
PLAN_MEMBER=f"{PLAN_MARKER}.json"
PROFILE_ARTIFACTS={
    "envelope": ("cargo-package-profile-envelope", "cargo-package-profile-envelope.json"),
    "plan": ("cargo-package-profile-plan", "cargo-package-profile-plan.json"),
    "results": ("cargo-package-profile-results", "cargo-package-profile-results.json"),
    "receipt": ("cargo-package-profile-receipt", "cargo-package-profile-receipt.json"),
}
STAGE_ARTIFACT_PREFIX="cargo-checker-stage-admission-receipt-"
STAGE_ARTIFACT_MEMBER="post-run-receipt.json"
# Keep every planner gate selector in the receipt authority digest, including
# non-Rust governance gates that do not appear in the Rust test matrix.
RUN_FIELDS=(
    "run_oasis7_required_tests", "run_consensus_tests", "run_distfs_tests",
    "run_oasis7_node_tests", "run_oasis7_net_tests", "run_oasis7_net_libp2p_tests",
    "run_viewer_contract_tests", "run_viewer_wasm_check", "run_viewer_perf_smoke",
    "run_pixel_world_bridge_lib_tests", "run_pixel_world_bridge_wasm_check",
    "run_launcher_web_build", "run_oasis7_workspace_support_crate_tests",
    "run_scenario_regression", "run_operational_contracts",
    "run_site_contract_tests",
    "run_codex_agent_config_validation", "run_compile_metrics_contract_tests",
    "run_required_gate_baseline", "run_rust_baseline",
)
EXECUTION_CONTRACT="required-domain-split/v1"
VERSIONED_SELECTOR_FIELDS=(
    "run_workflow_governance_contracts", "run_packaging_contracts",
    "run_doc_checker_contracts", "run_cargo_tooling_contracts",
)
VERSIONED_SELECTOR_CAPABILITIES={
    "run_workflow_governance_contracts":"workflow_governance",
    "run_packaging_contracts":"packaging_contracts",
    "run_doc_checker_contracts":"doc_checker_contracts",
    "run_cargo_tooling_contracts":"cargo_tooling_contracts",
}
VERSIONED_RESOURCE_FIELDS=(
    "needs_python", "needs_markdown", "needs_rust_toolchain", "needs_node",
    "needs_system_deps", "needs_trunk", "needs_wasm_target",
)
WINDOWS_ROLLOUT_JOB="windows-package-rollout-behavior"
MACOS_PACKAGE_JOB="testnet-packages-macos-arm64-contract"
FLEET_HEALTH_JOB="public-testnet-fleet-health-contract"
FLEET_HEALTH_RUNNERS=("ubuntu-24.04", "windows-2022", "macos-14")

def canonical_planner(raw):
    if not isinstance(raw,dict): raise SystemExit("ci-ready-receipt: uncertain planner metadata is not an object")
    execution_contract=raw.get("execution_contract")
    if execution_contract is None:
        if any(field in raw for field in VERSIONED_SELECTOR_FIELDS+VERSIONED_RESOURCE_FIELDS[:2]):
            raise SystemExit("ci-ready-receipt: uncertain mixed execution-contract planner metadata")
        run_fields=RUN_FIELDS
    elif execution_contract==EXECUTION_CONTRACT:
        run_fields=RUN_FIELDS+VERSIONED_SELECTOR_FIELDS
    else:
        raise SystemExit("ci-ready-receipt: uncertain unsupported execution_contract")
    required=("scope","selected_capabilities","reason_summary","changed_path_count","planner_config_sha256",*run_fields)
    if execution_contract is not None:
        required=required+VERSIONED_RESOURCE_FIELDS
    if any(k not in raw for k in required): raise SystemExit("ci-ready-receipt: uncertain incomplete planner metadata")
    if execution_contract is not None:
        versioned_boolean_fields=run_fields+VERSIONED_RESOURCE_FIELDS
        if any(type(raw[k]) is not str or raw[k] not in ("true", "false") for k in versioned_boolean_fields):
            raise SystemExit("ci-ready-receipt: uncertain non-boolean planner metadata for versioned execution contract")
        if raw["needs_python"]!="true" or raw["needs_markdown"]!="true":
            raise SystemExit("ci-ready-receipt: required-gate baseline document checks require Python and Markdown")
    elif any(str(raw[k]).lower() not in ("true","false") for k in run_fields):
        raise SystemExit("ci-ready-receipt: uncertain non-boolean planner metadata")
    try: changed=int(raw["changed_path_count"])
    except Exception: raise SystemExit("ci-ready-receipt: uncertain invalid changed_path_count")
    if not re.fullmatch(r"sha256:[0-9a-f]{64}",str(raw["planner_config_sha256"])): raise SystemExit("ci-ready-receipt: uncertain invalid planner config digest")
    selected=str(raw["selected_capabilities"])
    capabilities=selected.split(";") if selected else []
    if capabilities != sorted(set(capabilities)) or any(not re.fullmatch(r"[a-z0-9_]+", item) for item in capabilities):
        raise SystemExit("ci-ready-receipt: uncertain invalid selected_capabilities")
    plan={"schema":PLAN_MARKER,"scope":str(raw["scope"]),"selected_capabilities":capabilities,"reason_summary":str(raw["reason_summary"]),"changed_path_count":changed,"planner_config_sha256":str(raw["planner_config_sha256"])}
    if execution_contract is not None:
        selected=set(capabilities)
        for field, capability in VERSIONED_SELECTOR_CAPABILITIES.items():
            value=str(raw[field]).lower()=="true"
            if value != (capability in selected):
                raise SystemExit(f"ci-ready-receipt: uncertain contradictory planner selector: {field}")
        plan["execution_contract"]=EXECUTION_CONTRACT
    plan.update({k:str(raw[k]).lower()=="true" for k in run_fields})
    if execution_contract is not None:
        plan.update({field:raw[field]=="true" for field in VERSIONED_RESOURCE_FIELDS})
    projection_fields=("impact_projection_schema","impact_projection_digest","impact_projection_status","test_profile","declared_tests","planner_digest")
    present=[field for field in projection_fields if field in raw]
    if present:
        if len(present)!=len(projection_fields) or raw["impact_projection_schema"]!="oasis7-workflow-impact-projection/v2" or raw["impact_projection_status"]!="verified":
            raise SystemExit("ci-ready-receipt: uncertain incomplete impact projection planner metadata")
        if not re.fullmatch(r"sha256:[0-9a-f]{64}",str(raw["impact_projection_digest"])) or not re.fullmatch(r"sha256:[0-9a-f]{64}",str(raw["planner_digest"])):
            raise SystemExit("ci-ready-receipt: uncertain invalid impact projection digest")
        plan.update({"impact_projection_schema":raw["impact_projection_schema"],"impact_projection_digest":raw["impact_projection_digest"],"impact_projection_status":"verified","impact_projection_test_profile":raw["test_profile"],"impact_projection_declared_tests":str(raw["declared_tests"]).split(";") if raw["declared_tests"] else [],"impact_projection_planner_digest":raw["planner_digest"]})
    return plan

def planner_from_run(run):
    output=run.get("output") or {}; text="\n".join(str(output.get(k) or "") for k in ("summary","text"))
    m=re.search(r"oasis7-required-plan-v1\s*-->\s*```json\s*(\{.*?\})\s*```",text,re.S)
    if not m: raise SystemExit("ci-ready-receipt: uncertain planner metadata marker missing")
    try: raw=json.loads(m.group(1))
    except Exception as exc: raise SystemExit(f"ci-ready-receipt: uncertain malformed planner metadata: {exc}")
    return canonical_planner(raw)

def gh(*args):
    try:
        return json.loads(subprocess.check_output(["gh", *args], text=True, stderr=subprocess.PIPE))
    except Exception as exc:
        raise SystemExit(f"ci-ready-receipt: uncertain GitHub read: {exc}")

def _positive_int(value, field):
    if isinstance(value,bool):
        raise SystemExit(f"ci-ready-receipt: uncertain invalid {field}")
    try: result=int(value)
    except (TypeError,ValueError):
        raise SystemExit(f"ci-ready-receipt: uncertain invalid {field}")
    if result<1 or str(value)!=str(result):
        raise SystemExit(f"ci-ready-receipt: uncertain invalid {field}")
    return result

def _check_run_id_from_url(value, repository, *, field):
    expected=f"https://api.github.com/repos/{repository}/check-runs/"
    match=re.fullmatch(re.escape(expected)+r"([0-9]+)",str(value or ""))
    if not match:
        raise SystemExit(f"ci-ready-receipt: uncertain {field} check-run URL")
    return _positive_int(match.group(1),f"{field} check-run id")

def _workflow_job_details(check_run, repository):
    expected_prefix=f"https://github.com/{repository}/actions/runs/"
    match=re.fullmatch(re.escape(expected_prefix)+r"([0-9]+)/job/([0-9]+)(?:\?.*)?",str(check_run.get("details_url") or ""))
    if not match:
        raise SystemExit("ci-ready-receipt: uncertain required-gate workflow job URL")
    return int(match.group(1)),int(match.group(2))

def _selected_child_groups(planner):
    operational=planner.get("run_operational_contracts") is True
    versioned=planner.get("execution_contract")==EXECUTION_CONTRACT
    packaging=(planner.get("run_packaging_contracts") is True) if versioned else operational
    return {
        WINDOWS_ROLLOUT_JOB: operational,
        MACOS_PACKAGE_JOB: packaging,
        FLEET_HEALTH_JOB: operational,
    }

def _github_time(value, field):
    if not isinstance(value,str) or not value:
        raise SystemExit(f"ci-ready-receipt: uncertain missing {field}")
    try: parsed=dt.datetime.fromisoformat(value.replace("Z","+00:00"))
    except ValueError as exc:
        raise SystemExit(f"ci-ready-receipt: uncertain malformed {field}: {exc}")
    if parsed.tzinfo is None:
        raise SystemExit(f"ci-ready-receipt: uncertain timezone missing from {field}")
    return parsed.astimezone(dt.timezone.utc)

def _job_identity(job, repository, workflow_run_id, run_attempt, *, field):
    if not isinstance(job,dict):
        raise SystemExit(f"ci-ready-receipt: uncertain malformed {field} job")
    job_id=_positive_int(job.get("id"),f"{field} job id")
    if _positive_int(job.get("run_id"),f"{field} workflow run id")!=workflow_run_id:
        raise SystemExit(f"ci-ready-receipt: uncertain {field} belongs to wrong workflow run")
    if _positive_int(job.get("run_attempt"),f"{field} workflow run attempt")!=run_attempt:
        raise SystemExit(f"ci-ready-receipt: uncertain {field} belongs to wrong workflow attempt")
    check_run_id=_check_run_id_from_url(job.get("check_run_url"),repository,field=field)
    head_sha=job.get("head_sha")
    if not isinstance(head_sha,str) or not re.fullmatch(r"[0-9a-f]{40,64}",head_sha):
        raise SystemExit(f"ci-ready-receipt: uncertain {field} head SHA")
    labels=job.get("labels")
    if not isinstance(labels,list) or any(not isinstance(label,str) for label in labels):
        raise SystemExit(f"ci-ready-receipt: uncertain {field} runner labels")
    return {
        "job_id":job_id,"check_run_id":check_run_id,"run_id":workflow_run_id,
        "run_attempt":run_attempt,"head_sha":head_sha,
        "status":job.get("status"),"conclusion":job.get("conclusion"),
        "labels":sorted(set(labels)),"name":job.get("name"),
    }

def _selected_child_job_outcomes(repository, check_run, workflow_run_id, planner, artifact):
    selected=_selected_child_groups(planner)
    if not any(selected.values()):
        return planner

    details_run_id,gate_job_id=_workflow_job_details(check_run,repository)
    if details_run_id!=workflow_run_id:
        raise SystemExit("ci-ready-receipt: uncertain required-gate job belongs to wrong workflow run")
    gate=gh("api",f"repos/{repository}/actions/jobs/{gate_job_id}")
    gate_identity=_job_identity(gate,repository,workflow_run_id,
      _positive_int(gate.get("run_attempt"),"required-gate workflow run attempt"),field="required-gate")
    if gate_identity["job_id"]!=gate_job_id or gate.get("name")!=check_run.get("name"):
        raise SystemExit("ci-ready-receipt: uncertain required-gate job/check identity mismatch")
    expected_check_run_id=_positive_int(check_run.get("id"),"required-gate check-run id")
    if gate_identity["check_run_id"]!=expected_check_run_id:
        raise SystemExit("ci-ready-receipt: uncertain required-gate job/check identity mismatch")
    if (gate.get("status")!="completed" or str(gate.get("conclusion") or "").lower()!="success"
            or gate.get("status")!=check_run.get("status")
            or str(gate.get("conclusion") or "").lower()!=str(check_run.get("conclusion") or "").lower()):
        raise SystemExit("ci-ready-receipt: required-gate workflow job is not completed successfully")
    run_attempt=gate_identity["run_attempt"]
    integration=check_run.get("_integration") or {}
    if integration:
        if (_positive_int(integration.get("workflow_run_id"),"integration workflow run id")!=workflow_run_id
                or _positive_int(integration.get("run_attempt"),"integration workflow run attempt")!=run_attempt):
            raise SystemExit("ci-ready-receipt: required-gate job differs from trusted integration run/attempt")

    gate_started=_github_time(gate.get("started_at"),"required-gate job start")
    gate_completed=_github_time(gate.get("completed_at"),"required-gate job completion")
    artifact_created=_github_time(artifact.get("created_at"),"planner artifact creation time")
    if gate_completed<gate_started or not gate_started<=artifact_created<=gate_completed:
        raise SystemExit("ci-ready-receipt: planner artifact is not bound to required-gate workflow attempt")

    jobs=[]
    for page in range(1,101):
        response=gh("api",f"repos/{repository}/actions/runs/{workflow_run_id}/attempts/{run_attempt}/jobs?per_page=100&page={page}")
        batch=response.get("jobs") if isinstance(response,dict) else None
        if not isinstance(batch,list):
            raise SystemExit("ci-ready-receipt: uncertain malformed workflow attempt jobs response")
        jobs.extend(batch)
        if len(batch)<100: break
    else:
        raise SystemExit("ci-ready-receipt: uncertain workflow attempt job pagination overflow")

    identities=[]; seen_job_ids=set()
    for raw in jobs:
        identity=_job_identity(raw,repository,workflow_run_id,run_attempt,field="workflow attempt")
        if identity["job_id"] in seen_job_ids:
            raise SystemExit("ci-ready-receipt: uncertain duplicate workflow attempt job id")
        seen_job_ids.add(identity["job_id"])
        if identity["head_sha"]!=gate_identity["head_sha"]:
            raise SystemExit("ci-ready-receipt: uncertain child job belongs to a different workflow head")
        identities.append(identity)
    gate_matches=[item for item in identities if item["job_id"]==gate_job_id]
    if len(gate_matches)!=1 or gate_matches[0]!=gate_identity:
        raise SystemExit("ci-ready-receipt: required-gate job is absent from its exact workflow attempt")

    selected_outcomes=[]
    expected_names=(WINDOWS_ROLLOUT_JOB,MACOS_PACKAGE_JOB)
    for name in expected_names:
        if not selected[name]: continue
        matches=[item for item in identities if item["name"]==name]
        if len(matches)!=1:
            raise SystemExit(f"ci-ready-receipt: selected child job missing or ambiguous: {name}")
        selected_outcomes.append(_require_successful_child(matches[0],name,
          "windows-2022" if name==WINDOWS_ROLLOUT_JOB else "ubuntu-24.04"))
    if selected[FLEET_HEALTH_JOB]:
        fleet_jobs=[item for item in identities if item["name"]==FLEET_HEALTH_JOB or str(item["name"] or "").startswith(FLEET_HEALTH_JOB+" (")]
        by_runner={}
        for item in fleet_jobs:
            matched=[runner for runner in FLEET_HEALTH_RUNNERS if item["name"]==f"{FLEET_HEALTH_JOB} ({runner})"]
            if len(matched)!=1:
                raise SystemExit("ci-ready-receipt: selected fleet-health child has unexpected name")
            runner=matched[0]
            if runner in by_runner:
                raise SystemExit(f"ci-ready-receipt: selected fleet-health child is ambiguous: {runner}")
            by_runner[runner]=item
        if set(by_runner)!=set(FLEET_HEALTH_RUNNERS):
            missing=sorted(set(FLEET_HEALTH_RUNNERS)-set(by_runner))
            raise SystemExit("ci-ready-receipt: selected fleet-health child job missing: "+",".join(missing))
        selected_outcomes.extend(_require_successful_child(by_runner[runner],
          f"{FLEET_HEALTH_JOB} ({runner})",runner) for runner in FLEET_HEALTH_RUNNERS)
    check_run_ids=[item["check_run_id"] for item in selected_outcomes]
    if len(check_run_ids)!=len(set(check_run_ids)):
        raise SystemExit("ci-ready-receipt: uncertain selected child jobs share a check-run identity")

    source_digest=hashlib.sha256(json.dumps(planner,sort_keys=True,separators=(",",":")).encode()).hexdigest()
    result=dict(planner)
    result["selected_child_job_outcomes"]={
        "schema":"oasis7-selected-child-job-outcomes/v1",
        "workflow_run_id":workflow_run_id,"run_attempt":run_attempt,
        "required_gate_job_id":gate_job_id,"required_gate_check_run_id":expected_check_run_id,
        "plan_artifact_id":_positive_int(artifact.get("id"),"planner artifact id"),
        "plan_artifact_created_at":artifact_created.isoformat().replace("+00:00","Z"),
        "source_planner_digest":source_digest,
        "jobs":selected_outcomes,
    }
    return result

def _require_successful_child(identity, expected_name, expected_runner):
    if identity["name"]!=expected_name:
        raise SystemExit(f"ci-ready-receipt: selected child job name mismatch: {expected_name}")
    if expected_runner not in identity["labels"]:
        raise SystemExit(f"ci-ready-receipt: selected child job runner mismatch: {expected_name}")
    if identity["status"]!="completed" or str(identity["conclusion"] or "").lower()!="success":
        raise SystemExit(f"ci-ready-receipt: selected child job is not completed successfully: {expected_name}")
    return {key:identity[key] for key in ("name","job_id","check_run_id","run_id","run_attempt","status","conclusion","labels")}

def artifact_bytes(repository, artifact_id):
    try:
        return subprocess.check_output(
            ["gh","api",f"repos/{repository}/actions/artifacts/{artifact_id}/zip"],
            stderr=subprocess.PIPE,
        )
    except Exception as exc:
        raise SystemExit(f"ci-ready-receipt: uncertain artifact download: {exc}")

def planner_for_run(repository, check_run, *, base_oid, head_oid):
    details=str(check_run.get("details_url") or "")
    expected_prefix=f"https://github.com/{repository}/actions/runs/"
    if not details.startswith(expected_prefix):
        raise SystemExit("ci-ready-receipt: uncertain workflow run details_url missing or mismatched")
    match=re.match(re.escape(expected_prefix)+r"(\d+)(?:/|$)",details)
    if not match: raise SystemExit("ci-ready-receipt: uncertain workflow run id missing")
    workflow_run_id=int(match.group(1))
    matches=[]
    for page in range(1,101):
        response=gh("api",f"repos/{repository}/actions/runs/{workflow_run_id}/artifacts?per_page=100&page={page}")
        batch=response.get("artifacts",[])
        matches.extend(a for a in batch if a.get("name")==PLAN_ARTIFACT)
        if len(batch)<100: break
    else: raise SystemExit("ci-ready-receipt: uncertain artifact pagination overflow")
    if len(matches)!=1: raise SystemExit("ci-ready-receipt: uncertain planner artifact missing or ambiguous")
    artifact=matches[0]
    if artifact.get("expired"): raise SystemExit("ci-ready-receipt: uncertain planner artifact expired")
    if int((artifact.get("workflow_run") or {}).get("id") or 0)!=workflow_run_id:
        raise SystemExit("ci-ready-receipt: uncertain planner artifact belongs to wrong run")
    try:
        with zipfile.ZipFile(io.BytesIO(artifact_bytes(repository,artifact["id"]))) as archive:
            members=archive.namelist()
            if members != [PLAN_MEMBER]: raise ValueError(f"expected only {PLAN_MEMBER}")
            envelope=json.loads(archive.read(PLAN_MEMBER))
    except Exception as exc:
        raise SystemExit(f"ci-ready-receipt: uncertain malformed planner artifact: {exc}")
    expected={"schema":PLAN_MARKER,"repository":repository,"workflow_run_id":workflow_run_id,
      "head_oid":head_oid,"base_oid":base_oid,"check_name":check_run.get("name")}
    if not isinstance(envelope,dict): raise SystemExit("ci-ready-receipt: uncertain malformed planner artifact envelope")
    for key,val in expected.items():
        if envelope.get(key)!=val:
            raise SystemExit(f"ci-ready-receipt: uncertain planner artifact identity mismatch: {key}")
    if not isinstance(envelope.get("planner"),dict):
        raise SystemExit("ci-ready-receipt: uncertain incomplete planner artifact envelope")
    planner=canonical_planner(envelope["planner"])
    return _selected_child_job_outcomes(repository,check_run,workflow_run_id,planner,artifact)

def _trusted_workflow_source(repository, workflow_sha):
    response=gh("api",f"repos/{repository}/contents/.github/workflows/rust.yml?ref={workflow_sha}")
    if response.get("encoding")!="base64" or not isinstance(response.get("content"),str):
        raise SystemExit("ci-ready-receipt: trusted workflow source is unavailable")
    # GitHub's Contents API line-wraps base64.  Remove only JSON-decoded ASCII
    # whitespace, then retain strict alphabet/padding validation.
    normalized="".join(response["content"].split())
    try: return base64.b64decode(normalized,validate=True)
    except Exception as exc: raise SystemExit(f"ci-ready-receipt: trusted workflow source is malformed: {exc}")

def _verify_live_checker_stage_scope(repository, task_uid, task_issue_number, pr_number, proof):
    try:
        from cargo_checker_stage_admission import CHECKER_ISSUE, CHECKER_SCOPE, CHECKER_TASK_UID
    except Exception as exc:
        raise SystemExit(f"ci-ready-receipt: checker-stage authority module is unavailable: {exc}")
    if repository != "eng-cc/oasis7" or task_issue_number != CHECKER_ISSUE or task_uid != CHECKER_TASK_UID:
        raise SystemExit("ci-ready-receipt: checker-stage task identity is not the trusted Issue binding")
    issue=gh("api",f"repos/{repository}/issues/{CHECKER_ISSUE}")
    if not isinstance(issue,dict) or issue.get("number")!=CHECKER_ISSUE or issue.get("state")!="open" or issue.get("repository_url")!=f"https://api.github.com/repos/{repository}":
        raise SystemExit("ci-ready-receipt: checker-stage task Issue readback is unavailable")
    body=str(issue.get("body") or "").replace("\r\n","\n")
    uids=re.findall(r"(?m)^task_uid:\s*(task_[0-9a-f]{32})\s*$",body)
    if uids != [CHECKER_TASK_UID]:
        raise SystemExit("ci-ready-receipt: checker-stage task Issue UID binding is ambiguous")
    references=set(re.findall(rf"https://github\.com/{re.escape(repository)}/pull/(\d+)\b",body))
    references.update(re.findall(r"(?m)^-?\s*pr_number:\s*`?(\d+)`?\s*$",body))
    if references != {str(pr_number)}:
        raise SystemExit("ci-ready-receipt: checker-stage reciprocal PR binding is missing or mismatched")
    pull=gh("api",f"repos/{repository}/pulls/{pr_number}")
    if not isinstance(pull,dict) or pull.get("number")!=pr_number or pull.get("state")!="open" or pull.get("merged"):
        raise SystemExit("ci-ready-receipt: checker-stage PR is unavailable or not open")
    base=pull.get("base") or {}
    head=pull.get("head") or {}
    if base.get("ref")!="main":
        raise SystemExit("ci-ready-receipt: checker-stage PR base branch is not canonical main")
    if ((base.get("repo") or {}).get("full_name")!=repository or
        (head.get("repo") or {}).get("full_name")!=repository):
        raise SystemExit("ci-ready-receipt: checker-stage PR base/head repository is not canonical")
    if str(head.get("sha") or "") != str(proof.get("head_oid") or ""):
        raise SystemExit("ci-ready-receipt: checker-stage PR head does not match integration source head")
    if str(base.get("sha") or "") != str(proof.get("base_oid") or ""):
        raise SystemExit("ci-ready-receipt: checker-stage PR base does not match integration base")
    files=[]
    for page in range(1,101):
        batch=gh("api",f"repos/{repository}/pulls/{pr_number}/files?per_page=100&page={page}")
        if not isinstance(batch,list):
            raise SystemExit("ci-ready-receipt: checker-stage changed-path readback is unavailable")
        files.extend(batch)
        if len(batch)<100: break
    else: raise SystemExit("ci-ready-receipt: checker-stage changed-path pagination overflow")
    if len(files)!=len(CHECKER_SCOPE):
        raise SystemExit("ci-ready-receipt: checker-stage PR scope is not exactly the trusted two-file set")
    if sorted(item.get("filename") for item in files if isinstance(item,dict)) != sorted(CHECKER_SCOPE):
        raise SystemExit("ci-ready-receipt: checker-stage PR changed paths are outside the trusted two-file set")
    if any(not isinstance(item,dict) or item.get("status")!="modified" or item.get("previous_filename") for item in files):
        raise SystemExit("ci-ready-receipt: checker-stage PR contains rename/copy or non-modification scope")

def _checker_stage_disposition(repository, check_run, proof, artifacts, workflow_source, *, task_uid, task_issue_number, pr_number):
    try:
        from cargo_checker_stage_admission import verify_durable_postrun_receipt, AdmissionError
    except Exception as exc:
        raise SystemExit(f"ci-ready-receipt: checker-stage receipt verifier is unavailable: {exc}")
    required_markers=(b"id: checker-stage",b"git diff --name-status --find-renames",b"cargo-checker-stage-admission-receipt-")
    if any(marker not in workflow_source for marker in required_markers):
        raise SystemExit("ci-ready-receipt: trusted workflow lacks the checker-stage integration route")
    workflow_run_id=int(proof.get("workflow_run_id") or 0)
    run_attempt=int(proof.get("run_attempt") or 0)
    if workflow_run_id<1 or run_attempt<1:
        raise SystemExit("ci-ready-receipt: checker-stage workflow run identity is incomplete")
    expected_name=f"{STAGE_ARTIFACT_PREFIX}{workflow_run_id}-{run_attempt}"
    prefixed=[item for item in artifacts if isinstance(item,dict) and str(item.get("name") or "").startswith(STAGE_ARTIFACT_PREFIX)]
    if len(prefixed)!=1 or prefixed[0].get("name")!=expected_name:
        raise SystemExit("ci-ready-receipt: checker-stage receipt artifacts are missing, duplicate, or unexpected")
    matches=[item for item in artifacts if isinstance(item,dict) and item.get("name")==expected_name]
    if len(matches)!=1 or matches[0].get("expired") is not False:
        raise SystemExit("ci-ready-receipt: checker-stage receipt artifact is missing, ambiguous, or expired")
    artifact=matches[0]
    if int((artifact.get("workflow_run") or {}).get("id") or 0)!=workflow_run_id:
        raise SystemExit("ci-ready-receipt: checker-stage receipt artifact belongs to the wrong workflow run")
    try:
        with zipfile.ZipFile(io.BytesIO(artifact_bytes(repository,artifact["id"]))) as archive:
            if archive.namelist()!=[STAGE_ARTIFACT_MEMBER]: raise ValueError("unexpected archive members")
            receipt_bytes=archive.read(STAGE_ARTIFACT_MEMBER)
            receipt=json.loads(receipt_bytes)
    except Exception as exc:
        raise SystemExit(f"ci-ready-receipt: malformed checker-stage receipt artifact: {exc}")
    try:
        verify_durable_postrun_receipt(receipt)
    except AdmissionError as exc:
        raise SystemExit(f"ci-ready-receipt: invalid checker-stage durable receipt: {exc}")
    if not isinstance(receipt,dict) or receipt.get("repository")!=repository or receipt.get("task_uid")!=task_uid or receipt.get("pr_number")!=pr_number:
        raise SystemExit("ci-ready-receipt: checker-stage receipt task/PR identity mismatch")
    runner=receipt.get("runner") or {}
    expected_runner={"run_id":str(workflow_run_id),"run_attempt":str(run_attempt),"workflow_ref":proof.get("workflow_ref"),"workflow_sha":proof.get("workflow_sha")}
    if any(runner.get(field)!=value for field,value in expected_runner.items()):
        raise SystemExit("ci-ready-receipt: checker-stage receipt workflow identity mismatch")
    if check_run.get("status")!="completed" or str(check_run.get("conclusion") or "").lower()!="success":
        raise SystemExit("ci-ready-receipt: checker-stage required-gate check is not completed successfully")
    live_check=receipt.get("check") or {}
    # The required-gate check in integration_revalidation executes on the
    # trusted target checkout B; the receipt still binds the source PR head H
    # independently below.  Do not confuse the check execution identity with
    # the source head identity.
    expected_check={"check_name":check_run.get("name"),"check_app_id":(check_run.get("app") or {}).get("id"),"check_run_id":check_run.get("id"),"check_head":proof.get("base_oid"),"workflow_run_id":str(workflow_run_id)}
    if any(live_check.get(field)!=value for field,value in expected_check.items()):
        raise SystemExit("ci-ready-receipt: checker-stage receipt check identity mismatch")
    expected_scope_base=scope_base_for_run(repository,proof.get("base_oid"),proof.get("head_oid"))
    expected_tree=proof.get("tested_tree_oid")
    if any(receipt.get(field)!=value for field,value in (("base_oid",proof.get("base_oid")),("head_oid",proof.get("head_oid")),("scope_base_oid",expected_scope_base),("tested_tree",expected_tree))):
        raise SystemExit("ci-ready-receipt: checker-stage receipt B/H/T or scope-base mismatch")
    _verify_live_checker_stage_scope(repository,task_uid,task_issue_number,pr_number,proof)
    return {"schema":"oasis7-cargo-checker-stage-coverage/v1","execution_disposition":"trusted_checker_stage_receipt","disposition_validated":True,
      "repository":repository,"task_uid":task_uid,"task_issue_number":task_issue_number,"pr_number":pr_number,
      "workflow_ref":proof.get("workflow_ref"),"workflow_sha":proof.get("workflow_sha"),"run_id":workflow_run_id,"run_attempt":run_attempt,
      "check_name":check_run.get("name"),"check_app_id":(check_run.get("app") or {}).get("id"),"check_run_id":check_run.get("id"),
      "integration_base":proof.get("base_oid"),"source_head":proof.get("head_oid"),"tested_tree":proof.get("tested_tree_oid"),
      "scope_base":expected_scope_base,"stage_receipt_artifact":expected_name,
      "stage_receipt_digest":"sha256:"+hashlib.sha256(receipt_bytes).hexdigest()}

def cargo_package_profile_for_run(repository, check_run, proof, planner, *, task_uid, task_issue_number, pr_number):
    workflow_run_id=int(proof.get("workflow_run_id") or 0)
    if workflow_run_id < 1:
        raise SystemExit("ci-ready-receipt: package profile workflow run identity missing")
    artifacts=[]
    for page in range(1,101):
        response=gh("api",f"repos/{repository}/actions/runs/{workflow_run_id}/artifacts?per_page=100&page={page}")
        batch=response.get("artifacts",[]); artifacts.extend(batch)
        if len(batch)<100: break
    else: raise SystemExit("ci-ready-receipt: package profile artifact pagination overflow")
    profile_names={value[0] for value in PROFILE_ARTIFACTS.values()}
    present={item.get("name") for item in artifacts} & profile_names
    stage_artifacts=[item for item in artifacts if isinstance(item,dict) and str(item.get("name") or "").startswith(STAGE_ARTIFACT_PREFIX)]
    if stage_artifacts and present:
        raise SystemExit("ci-ready-receipt: checker-stage and package-profile artifacts cannot be mixed")
    if not present:
        workflow_source=_trusted_workflow_source(repository,proof.get("workflow_sha"))
        if stage_artifacts:
            return _checker_stage_disposition(repository,check_run,proof,artifacts,workflow_source,task_uid=task_uid,task_issue_number=task_issue_number,pr_number=pr_number)
        if b"cargo-package-profile-envelope" in workflow_source:
            raise SystemExit("ci-ready-receipt: package profile artifact missing from envelope-capable trusted workflow")
        planner_run_fields=RUN_FIELDS+VERSIONED_SELECTOR_FIELDS if planner.get("execution_contract")==EXECUTION_CONTRACT else RUN_FIELDS
        if planner.get("scope")!="full" or not all(planner.get(field) is True for field in planner_run_fields):
            raise SystemExit("ci-ready-receipt: pre-envelope trusted workflow requires complete conservative full coverage")
        return {
          "schema":"oasis7-cargo-package-profile-bootstrap-compatibility/v1",
          "execution_disposition":"legacy_required_coverage","disposition_validated":True,
          "repository":repository,"task_uid":task_uid,"task_issue_number":task_issue_number,
          "pr_number":pr_number,"workflow_ref":proof.get("workflow_ref"),
          "workflow_sha":proof.get("workflow_sha"),"run_id":workflow_run_id,
          "run_attempt":proof.get("run_attempt"),"check_name":check_run.get("name"),
          "check_app_id":(check_run.get("app") or {}).get("id"),"check_run_id":check_run.get("id"),
          "integration_base":proof.get("base_oid"),"source_head":proof.get("head_oid"),
          "tested_tree":proof.get("tested_tree_oid"),
          "trusted_workflow_sha256":"sha256:"+hashlib.sha256(workflow_source).hexdigest(),
        }
    if present != profile_names:
        raise SystemExit("ci-ready-receipt: package profile artifact set is partial")
    payloads={}
    for key,(artifact_name,member_name) in PROFILE_ARTIFACTS.items():
        matches=[item for item in artifacts if item.get("name")==artifact_name]
        if len(matches)!=1 or matches[0].get("expired"):
            raise SystemExit(f"ci-ready-receipt: package profile artifact missing, ambiguous, or expired: {artifact_name}")
        artifact=matches[0]
        if int((artifact.get("workflow_run") or {}).get("id") or 0)!=workflow_run_id:
            raise SystemExit("ci-ready-receipt: package profile artifact belongs to wrong run")
        try:
            with zipfile.ZipFile(io.BytesIO(artifact_bytes(repository,artifact["id"]))) as archive:
                if archive.namelist()!=[member_name]: raise ValueError("unexpected archive members")
                payloads[key]=archive.read(member_name)
        except Exception as exc:
            raise SystemExit(f"ci-ready-receipt: malformed package profile artifact: {exc}")
    try: envelope=json.loads(payloads["envelope"])
    except Exception as exc: raise SystemExit(f"ci-ready-receipt: malformed package profile envelope: {exc}")
    expected={
      "schema":"oasis7-cargo-package-profile-envelope/v1","repository":repository,
      "task_uid":task_uid,"pr_number":pr_number,"workflow_ref":proof.get("workflow_ref"),
      "workflow_sha":proof.get("workflow_sha"),"run_id":workflow_run_id,
      "run_attempt":proof.get("run_attempt"),"check_name":check_run.get("name"),
      "check_app_id":(check_run.get("app") or {}).get("id"),"check_run_id":check_run.get("id"),
      "integration_base":proof.get("base_oid"),"source_head":proof.get("head_oid"),
      "tested_tree":proof.get("tested_tree_oid"),
    }
    if "task_issue_number" in envelope: expected["task_issue_number"]=task_issue_number
    for key,value in expected.items():
        if envelope.get(key)!=value:
            raise SystemExit(f"ci-ready-receipt: package profile identity mismatch: {key}")
    digest_fields={"plan":"plan_digest","results":"results_digest","receipt":"receipt_digest"}
    for key,digest_field in digest_fields.items():
        expected_digest="sha256:"+hashlib.sha256(payloads[key]).hexdigest()
        if envelope.get(digest_field)!=expected_digest:
            raise SystemExit(f"ci-ready-receipt: package profile digest mismatch: {key}")
    return envelope

def now(): return dt.datetime.now(dt.timezone.utc).isoformat()

def scope_base_for_run(repository, integration_base, head):
    comparison = gh("api", f"repos/{repository}/compare/{integration_base}...{head}")
    scope_base = (comparison.get('merge_base_commit') or {}).get('sha')
    if not isinstance(scope_base, str) or not re.fullmatch(r'[0-9a-f]{40,64}', scope_base):
        raise SystemExit('ci-ready-receipt: exact run comparison merge-base unavailable')
    return scope_base

def check_run_pull_request_identity(check_run, pr_number, expected_head, expected_base_ref=None):
    matches=[item for item in (check_run.get("pull_requests") or [])
             if int(item.get("number") or 0)==pr_number]
    if len(matches)!=1:
        raise SystemExit("ci-ready-receipt: uncertain check run PR identity missing or ambiguous")
    item=matches[0]
    base_ref=str((item.get("base") or {}).get("ref") or "")
    base_oid=str((item.get("base") or {}).get("sha") or "")
    head_oid=str((item.get("head") or {}).get("sha") or "")
    run_head=str(check_run.get("head_sha") or head_oid)
    if not re.fullmatch(r"[0-9a-f]{40,64}",base_oid):
        raise SystemExit("ci-ready-receipt: uncertain check run base OID missing or invalid")
    if expected_base_ref is not None and base_ref != str(expected_base_ref):
        raise SystemExit("ci-ready-receipt: wrong_base_ref check run PR identity mismatch")
    if not re.fullmatch(r"[0-9a-f]{40,64}",head_oid) or head_oid!=expected_head or run_head!=expected_head:
        raise SystemExit("ci-ready-receipt: wrong_head check run PR identity mismatch")
    return base_oid,head_oid

def live(repository, task_uid, task_issue_number, pr_number, check_name, check_app_id,
         allow_ready_pr=False, expected_base_ref=None, ordinary_pr=False):
    if check_app_id is None or not re.fullmatch(r"[0-9]+",str(check_app_id)):
        raise SystemExit("ci-ready-receipt: check app id is required")
    pr=gh("api",f"repos/{repository}/pulls/{pr_number}")
    if not pr.get("draft") and not allow_ready_pr: raise SystemExit("ci-ready-receipt: superseded: PR is not a draft candidate")
    if not pr.get("draft") and (str(pr.get("state") or "").lower()!="open" or bool(pr.get("merged"))):
        raise SystemExit("ci-ready-receipt: superseded: recovery PR is not open and unmerged")
    body=str(pr.get("body") or "")
    if f"Task: {task_uid}" not in body or f"Refs #{task_issue_number}" not in body:
        raise SystemExit("ci-ready-receipt: uncertain task-to-PR linkage missing")
    if expected_base_ref is not None and str((pr.get("base") or {}).get("ref") or "") != str(expected_base_ref):
        raise SystemExit("ci-ready-receipt: wrong_base_ref PR base identity mismatch")
    head_oid=pr["head"]["sha"]
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
    # Check-run completion time is absent while a run is pending.  Sorting by
    # it first lets an older green run outrank a newer pending run, which would
    # turn an in-flight check into ordinary readiness evidence.  GitHub check
    # run IDs are immutable creation-order locators, so use that ordering
    # before validating status and conclusion.
    matches.sort(key=lambda x: int(x.get("id") or 0), reverse=True)
    run=matches[0]
    base_oid,head_oid=check_run_pull_request_identity(run,pr_number,head_oid,expected_base_ref)
    if not ordinary_pr and str((pr.get("base") or {}).get("sha") or "") != base_oid:
        raise SystemExit("ci-ready-receipt: stale integration base; rerun required CI against current target without rebasing source")
    if run.get("status")!="completed": raise SystemExit("ci-ready-receipt: uncertain: check incomplete")
    conclusion=str(run.get("conclusion") or "").lower()
    if conclusion=="cancelled": raise SystemExit("ci-ready-receipt: cancelled")
    if conclusion!="success": raise SystemExit(f"ci-ready-receipt: required check conclusion={conclusion or 'uncertain'}")
    if ordinary_pr:
        fresh=gh("api",f"repos/{repository}/pulls/{pr_number}")
        if (fresh.get("state")!="open" or fresh.get("merged")
                or (not allow_ready_pr and not fresh.get("draft"))
                or f"Refs #{task_issue_number}" not in (fresh.get("body") or "")
                or f"Task: {task_uid}" not in (fresh.get("body") or "")
                or fresh.get("base",{}).get("ref")!=pr.get("base",{}).get("ref")
                or fresh.get("head",{}).get("sha")!=head_oid):
            raise SystemExit("ci-ready-receipt: PR source/ref identity changed during ordinary CI verification")
        pr=fresh
    return pr,run,base_oid,head_oid

def main():
    p=argparse.ArgumentParser()
    p.add_argument("--repository",required=True); p.add_argument("--task-uid",required=True)
    p.add_argument("--task-issue-number",required=True,type=int)
    p.add_argument("--pr-number",required=True,type=int); p.add_argument("--check-name",default="required-gate")
    p.add_argument("--check-app-id",required=True); p.add_argument("--planner-digest",required=True)
    p.add_argument("--receipt"); p.add_argument("--allow-ready-pr",action="store_true"); p.add_argument("--json",action="store_true")
    p.add_argument("--base-ref", help="require the live PR and check-run base ref to match this branch")
    p.add_argument("--refresh-same-identity",action="store_true",
                   help="refresh only observed_at after complete live identity/planner validation")
    p.add_argument('--integration-run-id',type=int,help='new trusted manual integration workflow run')
    p.add_argument('--request-key',help='explicit authorized keyed integration request from the canonical local journal')
    a=p.parse_args()
    existing=json.loads(Path(a.receipt).read_text()) if a.receipt else {}
    if existing.get('request_key') and a.request_key!=existing['request_key']:
        raise SystemExit('ci-ready-receipt: explicit --request-key is required and must match the existing keyed receipt')
    bound_base_ref = a.base_ref or existing.get("base_ref")
    pr,run,base_oid,head_oid=selected_live(a.repository,a.task_uid,a.task_issue_number,a.pr_number,a.check_name,a.check_app_id,a.allow_ready_pr,bound_base_ref,a.integration_run_id or existing.get('integration_run_id'),request_key=a.request_key)
    if a.request_key is not None:
        proof=run.get('_integration') or {}
        if proof.get('request_key')!=a.request_key:
            raise SystemExit('ci-ready-receipt: keyed integration request identity mismatch')
        raise SystemExit('ci-ready-receipt: full trusted v2 planner inventory/results are unavailable; refusing v1 or older evidence fallback')
    old=None
    if a.receipt:
        old=json.loads(Path(a.receipt).read_text(encoding="utf-8"))
        live_identity={"repository":a.repository,"task_uid":a.task_uid,"task_issue_number":a.task_issue_number,
          "pr_number":a.pr_number,"base_oid":base_oid,"head_oid":head_oid,"check_name":a.check_name,
          "check_app_id":(run.get("app") or {}).get("id"),"check_run_id":run.get("id"),"conclusion":"success"}
        if "base_ref" in old:
            live_identity["base_ref"] = pr.get("base", {}).get("ref")
        for key,val in live_identity.items():
            if old.get(key)!=val: raise SystemExit(f"ci-ready-receipt: wrong_head/wrong_app/superseded receipt mismatch: {key}")
        seen=dt.datetime.fromisoformat(str(old["observed_at"]).replace("Z","+00:00"))
        if not a.refresh_same_identity and not 0 <= (dt.datetime.now(dt.timezone.utc)-seen).total_seconds() <= 600:
            raise SystemExit("ci-ready-receipt: stale")
    # Current production runs bind planner metadata through the same-run
    # artifact. Legacy/mock check runs without a workflow URL retain the
    # repository's signed check-summary contract and still fail closed when
    # the marker or canonical planner fields are absent.
    planner=(planner_for_run(a.repository,run,base_oid=base_oid,head_oid=head_oid)
             if run.get("details_url") else planner_from_run(run))
    if not run.get("details_url") and any(_selected_child_groups(planner).values()):
        raise SystemExit("ci-ready-receipt: selected child jobs require same-run workflow attempt evidence")
    trusted_planner_digest=hashlib.sha256(json.dumps(planner,sort_keys=True,separators=(",",":")).encode()).hexdigest()
    if a.planner_digest not in ("auto",trusted_planner_digest):
        raise SystemExit("ci-ready-receipt: uncertain planner_digest does not match live check metadata")
    payload={"receipt_type":"oasis7_ci_ready_receipt","issuer":"github_live_query","repository":a.repository,
      "task_uid":a.task_uid,"task_issue_number":a.task_issue_number,"pr_number":a.pr_number,"base_oid":base_oid,"head_oid":head_oid,
      "check_name":a.check_name,"check_app_id":(run.get("app") or {}).get("id"),"check_run_id":run.get("id"),
      "planner_digest":trusted_planner_digest,"planner":planner,"planner_config_sha256":planner["planner_config_sha256"],"run_rust_baseline":planner["run_rust_baseline"],"conclusion":"success","observed_at":now()}
    if planner.get("execution_contract")==EXECUTION_CONTRACT:
        payload["execution_contract"]=EXECUTION_CONTRACT
    if old is None or "base_ref" in old:
        payload["base_ref"] = pr.get("base", {}).get("ref")
    if old is None or "ci_validation_mode" in old:
        payload["ci_validation_mode"] = "trusted_integration" if run.get("_integration") else "ordinary_pr"
    if old is None or "live_validation" in old:
        payload["live_validation"] = "ci-ready-receipt-live"
    if planner.get("impact_projection_status") == "verified":
        payload.update(impact_projection_schema=planner["impact_projection_schema"],impact_projection_digest=planner["impact_projection_digest"],impact_projection_planner_digest=planner["impact_projection_planner_digest"])
    if old is None or 'scope_base_oid' in old:
        payload['scope_base_oid'] = scope_base_for_run(a.repository, base_oid, head_oid)
        payload['integration_base_oid'] = base_oid
    if run.get('_integration'):
        proof=run['_integration']
        payload.update(integration_run_id=proof['workflow_run_id'],tested_tree_oid=proof['tested_tree_oid'],tested_commit_oid=proof['tested_commit_oid'],workflow_sha=proof['workflow_sha'])
        if old is None or any(key in old for key in ('request_id', 'workflow_ref', 'run_attempt')):
            payload.update(workflow_ref=proof['workflow_ref'],request_id=proof['request_id'],request_created_at=proof['request_created_at'],run_id=proof['run_id'],run_attempt=proof['run_attempt'],live_validation='ci-ready-receipt-live',trusted_integration_artifact=True)
        payload["cargo_package_profile"]=cargo_package_profile_for_run(
            a.repository,run,proof,planner,task_uid=a.task_uid,
            task_issue_number=a.task_issue_number,pr_number=a.pr_number)
    elif old is not None and "cargo_package_profile" in old:
        raise ValueError("ci-ready-receipt: package profile evidence is detached from a trusted integration run")
    payload["review_evidence_digest"]=review_evidence_digest(payload)
    if old is not None:
        for key,val in payload.items():
            if key in ("observed_at", "review_evidence_digest"): continue
            if old.get(key)!=val: raise SystemExit(f"ci-ready-receipt: wrong_head/wrong_app/superseded receipt mismatch: {key}")
        if old.get("review_evidence_digest", payload["review_evidence_digest"]) != payload["review_evidence_digest"]:
            raise SystemExit("ci-ready-receipt: review evidence authority digest mismatch")
        refreshed = {**old, "observed_at": payload["observed_at"]} if a.refresh_same_identity else old
        if "review_evidence_digest" in old:
            refreshed = {**refreshed, "review_evidence_digest": payload["review_evidence_digest"]}
        payload = refreshed
    print(json.dumps(payload,sort_keys=True,indent=2 if a.json else None))
def _trusted_validation_request(request_key,repository,uid,number,head_oid,integration_base_oid):
    """Load an explicitly selected request only from this repo's canonical journal."""
    if not isinstance(request_key,str) or not re.fullmatch(r"sha256:[0-9a-f]{64}",request_key):
        raise ValueError("explicit validation request key is invalid")
    import integration_ci
    import integration_executor_contract as request_contract
    root=Path(__file__).resolve().parents[2]
    journal_dir=integration_ci.git_common_dir(root)
    path=request_contract._request_path(journal_dir,request_key)
    if not path.is_file():
        raise ValueError("validation request journal is missing")
    record=request_contract._read_request_record(path,request_key)
    identity=request_contract.validation_request_identity(record.get("identity"))
    if (identity!=record.get("identity")
            or request_contract.validation_request_key(identity)!=request_key):
        raise ValueError("validation request journal identity does not match explicit key")
    if record.get("status")!="observed":
        raise ValueError("validation request journal has no observed workflow run")
    expected={"repository":repository,"task_uid":uid,"pr_number":number,
      "source_head_oid":head_oid}
    for field,value in expected.items():
        if identity.get(field)!=value:
            raise ValueError(f"validation request journal identity mismatch: {field}")
    if record.get("integration_base_oid")!=integration_base_oid:
        raise ValueError("validation request journal immutable integration base mismatch")
    return record,identity


def selected_live(repository,uid,issue,number,check_name,app,allow_ready_pr=False,base_ref=None,
                  integration_run_id=None, require_integration=False, require_dispatch=False,
                  request_key=None):
    from integration_ci import current_request, verified_run
    import integration_ci
    pr=gh('api',f'repos/{repository}/pulls/{number}')
    if (not allow_ready_pr and not pr.get('draft')) or f'Refs #{issue}' not in (pr.get('body') or '') or f'Task: {uid}' not in (pr.get('body') or ''):
        raise SystemExit('ci-ready-receipt: manual integration task/draft identity mismatch')
    if pr.get('state')!='open' or pr.get('merged'):
        raise SystemExit('ci-ready-receipt: integration PR not open')
    if base_ref and pr['base']['ref']!=base_ref: raise SystemExit('ci-ready-receipt: manual integration base ref mismatch')
    base,head=pr['base']['sha'],pr['head']['sha']
    try:
        request_record=None
        request_identity=None
        effective_policy=None
        policy_context=None
        if request_key is not None:
            request_record,request_identity=_trusted_validation_request(
              request_key,repository,uid,number,head,base)
        selected=current_request(repository,uid,number,base,head,pr['base']['ref'],
          request_key=request_key)
        if request_key is not None and selected is None:
            raise ValueError('explicit keyed current request is absent from complete readback')
        if selected is not None:
            if integration_run_id is not None and int(integration_run_id)!=selected["id"]:
                raise ValueError('explicit integration locator superseded by current request')
            if check_name!='required-gate': raise ValueError('unsupported manual check')
            if request_key is not None:
                if selected["id"]!=request_record["run_id"]:
                    raise ValueError('current workflow run differs from durable validation request journal')
                if selected["run_attempt"]<request_record["run_attempt"]:
                    raise ValueError('current workflow attempt is older than durable validation request journal')
                workflow_sha=selected.get('workflow_run_head_sha')
                if not isinstance(workflow_sha,str) or not re.fullmatch(r'[0-9a-f]{40,64}',workflow_sha):
                    raise ValueError('trusted workflow SHA is missing from current request readback')
                policy_context=integration_ci.trusted_policy_context(
                  repository,pr['base']['ref'],workflow_sha,workflow_sha)
                effective_policy=policy_context.get('effective_policy')
                import integration_executor_contract as request_contract
                if (not isinstance(effective_policy,dict)
                        or request_contract.effective_policy_digest(effective_policy)
                          != request_identity['effective_policy_digest']):
                    raise ValueError('journal request effective policy differs from trusted workflow W')
                if str(effective_policy.get('check_app_id'))!=str(app):
                    raise ValueError('check app differs from trusted workflow W policy')
            if request_key is not None:
                check,proof=verified_run(repository,uid,number,base,head,selected["id"],app,
                  request_key=request_key,expected_attempt=selected["run_attempt"],
                  request_identity=request_identity,effective_policy=effective_policy)
            else:
                check,proof=verified_run(repository,uid,number,base,head,selected["id"],app,
                  expected_attempt=selected["run_attempt"])
            if request_key is not None:
                expected_context=policy_context
                if (proof.get('workflow_run_id')!=selected['id']
                        or proof.get('run_attempt')!=selected['run_attempt']
                        or str(proof.get('check_app_id'))!=str((check.get('app') or {}).get('id'))
                        or proof.get('check_run_id')!=check.get('id')
                        or proof.get('trusted_policy_context')!=expected_context
                        or proof.get('effective_policy_identity')!=expected_context.get('effective_policy_identity')
                        or proof.get('planner_inventory_authority')!=expected_context.get('planner_inventory_authority')):
                    raise ValueError('verified workflow attempt or trusted policy context mismatch')
            proof={**proof,
              "request_id": selected["id"],
              "request_created_at": dt.datetime.fromtimestamp(selected["requested_at"], dt.timezone.utc).isoformat(),
              "run_id": proof.get("workflow_run_id", selected["id"]),
              "run_attempt": selected["run_attempt"],
              "check_app_id": (check.get("app") or {}).get("id"),
              "check_run_id": check.get("id")}
            if request_key is not None:
                proof.update(request_key=request_key,request_identity=request_identity)
            if current_request(repository,uid,number,base,head,pr['base']['ref'],
              request_key=request_key)!=selected:
                raise ValueError('current request changed during integration verification')
            fresh=gh('api',f'repos/{repository}/pulls/{number}')
            if (fresh.get('state')!='open' or fresh.get('merged')
                or (not allow_ready_pr and not fresh.get('draft'))
                or f'Refs #{issue}' not in (fresh.get('body') or '')
                or f'Task: {uid}' not in (fresh.get('body') or '')
                or fresh.get('base',{}).get('sha')!=base
                or fresh.get('base',{}).get('ref')!=pr['base']['ref']
                or fresh.get('head',{}).get('sha')!=head):
                raise ValueError('PR identity or admission changed during integration verification')
            return fresh,{**check,'_integration':proof},base,head
        if integration_run_id is not None:
            raise ValueError('explicit integration locator absent from verified current request range')
    except (ValueError,KeyError,OSError,ImportError,TypeError,subprocess.SubprocessError) as exc:
        raise SystemExit('ci-ready-receipt: current request blocked: '+str(exc)) from exc
    if require_integration:
        if require_dispatch:
            raise SystemExit('ci-ready-receipt: strict integration request is absent')
        # Compatibility callers may request the strict base/check contract
        # before manual dispatch is available.  This path never relaxes to
        # ordinary PR evidence; the normal high-risk path supplies a matching
        # workflow_dispatch request and uses the branch above.
        return live(repository,uid,issue,number,check_name,app,allow_ready_pr,base_ref,ordinary_pr=False)
    # Only proven absence permits ordinary PR evidence. Never consult old green
    # after a matching request has failed, remains pending or is unreadable.
    ordinary=live(repository,uid,issue,number,check_name,app,allow_ready_pr,base_ref,ordinary_pr=True)
    if current_request(repository,uid,number,base,head,pr["base"]["ref"]) is not None:
        raise SystemExit("ci-ready-receipt: current request changed during ordinary CI verification")
    return ordinary

if __name__=="__main__": main()
