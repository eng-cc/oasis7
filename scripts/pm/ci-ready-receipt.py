#!/usr/bin/env python3
"""Issue/verify a live GitHub CI receipt for a frozen draft-candidate head."""
import argparse, base64, datetime as dt, hashlib, io, json, re, subprocess, sys, tempfile, zipfile
from pathlib import Path
from ci_ready_receipt_identity import review_evidence_digest, review_evidence_identity, current_target_proof_identity

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
SUPPORTED_EXECUTION_CONTRACTS={EXECUTION_CONTRACT,"required-domain-split/v2"}
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
    elif execution_contract in SUPPORTED_EXECUTION_CONTRACTS:
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
        if execution_contract=="required-domain-split/v2" and raw["needs_rust_toolchain"]!="true":
            raise SystemExit("ci-ready-receipt: v2 baseline metadata checks require Rust toolchain")
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
        plan["execution_contract"]=execution_contract
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
    versioned=planner.get("execution_contract") in SUPPORTED_EXECUTION_CONTRACTS
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
    result = _selected_child_job_outcomes(repository,check_run,workflow_run_id,planner,artifact)
    if "current_target_proof" in envelope:
        try:
            result["current_target_proof"] = current_target_proof_identity(envelope["current_target_proof"])
        except ValueError as exc:
            raise SystemExit("ci-ready-receipt: invalid current-target artifact: " + str(exc)) from exc
    return result


def _current_target_tree(repository, q, h, e):
    """Recompute the merge in an isolated object store, never the caller index."""
    with tempfile.TemporaryDirectory(prefix="oasis7-current-target-") as directory:
        def git(*args):
            return subprocess.check_output(["git", "-C", directory, *args], text=True, stderr=subprocess.PIPE).strip()
        git("init", "--bare", "--quiet")
        # Reuse immutable local objects when available. New merge objects go
        # only to this disposable store, leaving the canonical checkout alone.
        try:
            objects = subprocess.check_output(["git", "rev-parse", "--path-format=absolute", "--git-path", "objects"], text=True).strip()
            Path(directory, "objects/info/alternates").write_text(objects + "\n")
            for oid in (q, h, e):
                git("cat-file", "-e", oid + "^{commit}")
        except subprocess.CalledProcessError:
            git("fetch", "--quiet", "--no-tags", "https://github.com/" + repository + ".git", q, h, e)
        parents = git("rev-list", "--parents", "-n", "1", e).split()
        if parents != [e, q, h]:
            raise ValueError("current-target immutable checkout parents differ")
        ancestor = subprocess.run(["git", "-C", directory, "merge-base", "--is-ancestor", q, h], capture_output=True)
        if ancestor.returncode not in (0, 1):
            raise ValueError("current-target ancestry is unreadable")
        return git("rev-parse", h + "^{tree}") if ancestor.returncode == 0 else git("merge-tree", "--write-tree", q, h).splitlines()[0]


def _protected_file(repository, revision, path):
    response = gh("api", f"repos/{repository}/contents/{path}?ref={revision}")
    if response.get("path") != path or response.get("encoding") != "base64":
        raise ValueError("current-target immutable file readback differs: " + path)
    return base64.b64decode("".join(str(response.get("content", "")).split()), validate=True)


def validate_current_target_pr_proof(repository, pr, check, planner, *, current_target_oid, scope_only=False,
                                     ready_continuation=None):
    """Authenticate the existing PR run's exact current-target execution proof."""
    from workflow_maintenance import maintenance_comment_id, read_maintenance_authority
    if scope_only:
        from workflow_maintenance import read_maintenance_scope
        scope_reader = read_maintenance_scope
        scope_options = {}
    else:
        scope_reader = read_maintenance_authority
        scope_options = {'ready_continuation': ready_continuation}
    proof = current_target_proof_identity(planner.get("current_target_proof"))
    h, q, e = proof["source_head_oid"], proof["current_target_oid"], proof["checkout_oid"]
    if (proof["repository"] != repository or proof["pr_number"] != pr.get("number")
            or h != pr.get("head", {}).get("sha") or q != current_target_oid
            or proof["maintenance_authority_comment_id"] != maintenance_comment_id(pr.get("body", ""))):
        raise ValueError("current-target proof differs from live PR/target/authority")
    run_id, _ = _workflow_job_details(check, repository)
    if run_id != proof["workflow_run_id"]:
        raise ValueError("current-target check belongs to another run")
    run = gh("api", f"repos/{repository}/actions/runs/{run_id}")
    if (run.get("id") != run_id or run.get("event") != "pull_request"
            or run.get("status") != "completed" or run.get("conclusion") != "success"
            or run.get("run_attempt") != proof["workflow_run_attempt"]
            or run.get("head_sha") not in (h, e)
            or str(run.get("path", "")).split("@", 1)[0] != ".github/workflows/rust.yml"
            or not any(item.get("number") == pr["number"] and item.get("head", {}).get("sha") == h
                       for item in run.get("pull_requests", []) if isinstance(item, dict))):
        raise ValueError("current-target Actions run provenance differs")
    if run.get("workflow_sha") is not None and run["workflow_sha"] != proof["workflow_revision"]:
        raise ValueError("current-target actual workflow revision differs")
    outcomes = planner.get("selected_child_job_outcomes") or {}
    if (outcomes.get("workflow_run_id") != run_id or outcomes.get("run_attempt") != proof["workflow_run_attempt"]
            or planner.get("scope") != "full" or planner.get("impact_projection_test_profile") != "full"
            or planner.get("planner_config_sha256") != "sha256:" + proof["planner_config_sha256"]):
        raise ValueError("current-target run attempt/full required coverage is incomplete")
    authority = scope_reader(repository, proof["maintenance_authority_comment_id"], proof["task_uid"],
        pr["number"], h, (".github/workflows/rust.yml", "scripts/pm/ci-ready-receipt.py", "scripts/pm/ci_ready_receipt_identity.py"),
        ("scripts/pm/ci-ready-receipt.py", "scripts/pm/ci_ready_receipt_identity.py", "scripts/pm/workflow_maintenance.py",
         "scripts/pm/task_primary_package.py"), **scope_options)
    if authority["issue_number"] != proof["task_issue_number"]:
        raise ValueError("current-target Task Issue differs from authenticated authority")
    commits = {}
    for oid in (q, h, e):
        value = gh("api", f"repos/{repository}/git/commits/{oid}")
        if value.get("sha") != oid:
            raise ValueError("current-target Git commit identity differs")
        commits[oid] = value
    if ([item.get("sha") for item in commits[e].get("parents", [])] != [q, h]
            or commits[e].get("tree", {}).get("sha") != proof["tested_tree_oid"]
            or _current_target_tree(repository, q, h, e) != proof["tested_tree_oid"]):
        raise ValueError("current-target deterministic tested tree differs")
    approved_workflow = _protected_file(repository, h, ".github/workflows/rust.yml")
    event_workflow = _protected_file(repository, e, ".github/workflows/rust.yml")
    if event_workflow != approved_workflow:
        raise ValueError("actual event workflow bytes differ from approved source head")
    workflow = _protected_file(repository, proof["workflow_revision"], ".github/workflows/rust.yml")
    if workflow != event_workflow:
        raise ValueError("actual workflow bytes differ from approved source head")
    for path, field in (("scripts/ci-required-scope.v2.json", "planner_config_sha256"), ("scripts/ci-tests.sh", "test_driver_sha256")):
        contents = _protected_file(repository, q, path)
        if hashlib.sha256(contents).hexdigest() != proof[field]:
            raise ValueError("current-target protected authority digest differs: " + field)
        if field == "planner_config_sha256":
            config = json.loads(contents)
            if sorted(config["capabilities"]) != planner.get("selected_capabilities"):
                raise ValueError("current-target full protected capability coverage differs")
    import integration_ci
    if integration_ci.default_branch_head(repository, pr["base"]["ref"]) != q:
        raise ValueError("current-target protected ref moved during verification")
    fresh = gh("api", f"repos/{repository}/pulls/{pr['number']}")
    if any(fresh.get(key) != pr.get(key) for key in ("state", "merged", "draft", "body", "head", "base")):
        raise ValueError("current-target PR identity changed during verification")
    return proof


def read_current_target_proof_without_selection(repository, uid, issue, number,
                                               check_name, app, *, current_target_oid, base_ref):
    """Read authenticated execution facts; this never admits candidate tools.

    The maintenance continuation factory invokes this internal reader from its
    independently materialized immutable H closure. No receipt CLI exposes it.
    """
    pr, check, base, head = live(repository, uid, issue, number, check_name, app,
                               True, base_ref, ordinary_pr=True)
    planner = planner_for_run(repository, check, base_oid=base, head_oid=head)
    proof = validate_current_target_pr_proof(repository, pr, check, planner,
                                            current_target_oid=current_target_oid, scope_only=True)
    if proof["task_uid"] != uid or proof["task_issue_number"] != issue:
        raise ValueError("current-target raw proof differs from canonical Task identity")
    return proof


def _verified_v2_required_evidence(repository, check_run, proof, *, request_key,
                                   request_identity, task_uid, pr_number,
                                   integration_base_oid, head_oid):
    """Validate exact-attempt v2 plan and every required result artifact.

    Runtime proof supplies live artifact/check locators and the independently
    trusted planner binding. Artifact payloads remain claims until their full
    identity, input closure, policy and result obligations match that proof.
    """
    try:
        import ci_input_scope as input_scope
        import ci_required_artifact_v2 as required_v2
        import integration_executor_contract as request_contract

        def require(condition, message):
            if not condition:
                raise ValueError(message)

        def positive_int(value, field):
            if type(value) is not int or value <= 0:
                raise ValueError(f"{field} must be a positive integer")
            return value

        plan_artifact_id = positive_int(
            proof.get("required_plan_v2_artifact_id"), "v2 plan artifact ID",
        )
        run_id = positive_int(proof.get("workflow_run_id"), "workflow run ID")
        run_attempt = positive_int(proof.get("run_attempt"), "workflow run attempt")
        check_app_id = positive_int(proof.get("check_app_id"), "check app ID")
        check_run_id = positive_int(proof.get("check_run_id"), "check-run ID")
        if (run_id != proof.get("request_id") or run_id != proof.get("run_id")
                or run_attempt != proof.get("run_attempt")
                or check_run_id != check_run.get("id")
                or check_app_id != (check_run.get("app") or {}).get("id")
                or check_run.get("name") != "required-gate"
                or check_run.get("status") != "completed"
                or str(check_run.get("conclusion") or "").lower() != "success"):
            raise ValueError("v2 plan check or exact attempt identity mismatch")

        plan_name = required_v2.plan_artifact_name(run_id, run_attempt)
        if proof.get("required_plan_v2_artifact_name") != plan_name:
            raise ValueError("v2 plan artifact name is missing or belongs to another attempt")
        plan = required_v2.validate_plan_payload(
            proof.get("required_plan_v2_payload"), require_complete=True,
        )
        if not isinstance(plan, dict):
            raise ValueError("v2 plan payload is malformed")

        policy_context = proof.get("trusted_policy_context")
        trusted_policy = policy_context.get("effective_policy") if isinstance(policy_context, dict) else None
        effective_policy_identity = proof.get("effective_policy_identity")
        if (not isinstance(trusted_policy, dict)
                or not isinstance(effective_policy_identity, dict)
                or policy_context.get("effective_policy_identity") != effective_policy_identity
                or request_contract.effective_policy_digest(trusted_policy)
                   != effective_policy_identity.get("digest")
                or effective_policy_identity.get("digest")
                   != request_identity.get("effective_policy_digest")
                or effective_policy_identity.get("schema")
                   != request_contract.EFFECTIVE_POLICY_IDENTITY_SCHEMA
                or "input-scope-reuse/v1" not in trusted_policy.get("enabled_capabilities", [])):
            raise ValueError("v2 reuse policy is missing, disabled, or not authenticated")
        planner_authority = proof.get("planner_inventory_authority")
        require(isinstance(planner_authority, dict), "trusted planner authority is missing")
        if (policy_context.get("planner_inventory_authority") != planner_authority
                or str(trusted_policy.get("check_app_id")) != str(check_app_id)):
            raise ValueError("v2 planner authority or required-check app differs from trusted policy")

        workflow_ref = proof.get("workflow_ref")
        workflow_sha = proof.get("workflow_sha")
        tested_commit_oid = proof.get("tested_commit_oid")
        tested_tree_oid = proof.get("tested_tree_oid")
        source_scope_oid = proof.get("source_scope_oid")
        expected = {
            "request_key": request_key,
            "request_identity": request_identity,
            "repository": repository,
            "task_uid": task_uid,
            "pr_number": pr_number,
            "bootstrap_epoch": request_identity.get("bootstrap_epoch"),
            "source_head_oid": head_oid,
            "source_scope_oid": source_scope_oid,
            "source_projection_digest": request_identity.get("source_projection_digest"),
            "integration_base_oid": integration_base_oid,
            "tested_commit_oid": tested_commit_oid,
            "tested_tree_oid": tested_tree_oid,
            "workflow_ref": workflow_ref,
            "workflow_sha": workflow_sha,
            "workflow_run_id": run_id,
            "run_attempt": run_attempt,
            "check_name": "required-gate",
            "check_app_id": check_app_id,
            "check_run_id": check_run_id,
            "job_id": proof.get("job_id"),
            "job_name": proof.get("job_name"),
            "executor_contract_digest": request_identity.get("executor_contract_digest"),
            "effective_policy_identity": effective_policy_identity,
            "planner_inventory_authority": planner_authority,
        }
        for field, value in expected.items():
            if plan.get(field) != value:
                raise ValueError("v2 plan identity mismatch: " + field)
        projection = plan.get("planner_output")
        if (not isinstance(projection, dict)
                or projection.get("source_scope_base") != source_scope_oid
                or projection.get("impact_projection_digest")
                   != request_identity.get("source_projection_digest")):
            raise ValueError("v2 plan source scope or projection does not match trusted W output")

        issuer = plan.get("planner_inventory_issuer")
        if not isinstance(issuer, dict):
            raise ValueError("v2 plan planner inventory issuer is missing")
        trusted_inventory = proof.get("trusted_planner_inventory")
        expected_inventory = {
            "schema": input_scope.TRUSTED_PLANNER_INVENTORY_SCHEMA,
            "authority": planner_authority,
            "producer": {
                "run_id": run_id,
                "run_attempt": run_attempt,
                "check_app_id": check_app_id,
                "check_run_id": check_run_id,
                "artifact_id": plan_artifact_id,
            },
            "target_oid": tested_commit_oid,
            "target_tree_oid": tested_tree_oid,
            "unit_ids": plan.get("unit_ids"),
            "inventory_digest": plan.get("planner_inventory_digest"),
        }
        if trusted_inventory != expected_inventory:
            raise ValueError("v2 planner inventory binding is not tied to live W/R/A/check/artifact")
        input_scope_snapshot = input_scope.validate_input_scope_snapshot(
            plan.get("input_scope"), trusted_planner_inventory=trusted_inventory,
        )
        if input_scope_snapshot["closure_status"]["status"] != "complete":
            raise ValueError("v2 planner inventory closure is incomplete")
        if (input_scope_snapshot["target_oid"] != tested_commit_oid
                or input_scope_snapshot["target_tree_oid"] != tested_tree_oid
                or input_scope_snapshot["required_test_units"] != plan.get("unit_ids")
                or plan.get("required_test_units") != plan.get("unit_ids")
                or plan.get("unit_ids") != sorted(set(plan.get("unit_ids", [])))):
            raise ValueError("v2 input scope does not cover the complete exact unit inventory")
        recomputed_inventory_digest = input_scope.planner_inventory_digest(
            plan.get("unit_specs"), plan.get("product_corpus"),
            tested_commit_oid, tested_tree_oid,
        )
        if (recomputed_inventory_digest != trusted_inventory["inventory_digest"]
                or plan.get("planner_inventory_digest") != recomputed_inventory_digest
                or plan.get("planner_inventory_issuer") != {
                    key: value for key, value in trusted_inventory.items()
                    if key != "producer"
                } | {"producer": {
                    key: trusted_inventory["producer"][key]
                    for key in ("run_id", "run_attempt", "check_app_id", "check_run_id")
                }}):
            raise ValueError("v2 unit contracts or embedded issuer do not match trusted inventory")
        specs = plan.get("unit_specs")
        if (not isinstance(specs, list)
                or sorted(spec.get("unit_id") for spec in specs if isinstance(spec, dict))
                   != plan.get("unit_ids")):
            raise ValueError("v2 complete unit contract list is malformed")
        specs_by_id = {spec["unit_id"]: spec for spec in specs}
        fingerprints = input_scope_snapshot["input_fingerprints"]
        if plan.get("input_fingerprints") != fingerprints:
            raise ValueError("v2 plan fingerprints do not match the complete input scope")
        required_child_jobs = {
            unit_id: sorted(
                [MACOS_PACKAGE_JOB] if unit_id == "packaging_contracts" else
                [WINDOWS_ROLLOUT_JOB,
                 *(f"{FLEET_HEALTH_JOB} ({runner})" for runner in FLEET_HEALTH_RUNNERS)]
                if unit_id == "operational_contracts" else []
            )
            for unit_id in plan["unit_ids"]
        }
        if plan.get("execution_job_requirements") != required_child_jobs:
            raise ValueError("v2 execution job requirements differ from the trusted unit mapping")

        raw_results = proof.get("required_result_v2_artifacts")
        if not isinstance(raw_results, list) or len(raw_results) != len(plan["unit_ids"]):
            raise ValueError("v2 result artifact set is missing required units")
        seen_artifact_ids = set()
        seen_names = set()
        results_by_unit = {}
        normalized_results = []
        for raw in raw_results:
            if not isinstance(raw, dict) or set(raw) != {"artifact_id", "name", "payload"}:
                raise ValueError("v2 result artifact locator is malformed")
            result_artifact_id = positive_int(raw["artifact_id"], "v2 result artifact ID")
            result_name = required_v2.result_artifact_name(run_id, run_attempt, raw["payload"].get("unit_id") if isinstance(raw["payload"], dict) else "")
            if (raw["name"] != result_name or result_artifact_id == plan_artifact_id
                    or result_artifact_id in seen_artifact_ids or result_name in seen_names):
                raise ValueError("v2 result artifact is duplicate or belongs to another attempt")
            if not isinstance(raw["payload"], dict):
                raise ValueError("v2 result payload is malformed")
            result = required_v2.validate_result_payload(
                raw["payload"], plan=plan, plan_artifact_id=plan_artifact_id,
                expected_unit_id=raw["payload"].get("unit_id"),
            )
            unit_id = result.get("unit_id")
            if unit_id not in specs_by_id or unit_id in results_by_unit:
                raise ValueError("v2 result unit is missing, duplicated, or outside the inventory")
            for field, value in expected.items():
                if field in result and result.get(field) != value:
                    raise ValueError("v2 result identity mismatch: " + field)
            if (result.get("plan_artifact_id") != plan_artifact_id
                    or result.get("planner_inventory_digest") != trusted_inventory["inventory_digest"]
                    or result.get("input_digest") != fingerprints.get(unit_id)
                    or result.get("obligation_ids") != specs_by_id[unit_id].get("obligation_set")
                    or result.get("status") != "passed"
                    or result.get("disposition") != "executed"
                    or result.get("exit_code") != 0):
                raise ValueError("v2 result does not discharge its exact input-bound obligation")
            if (result.get("job_id") != proof.get("job_id")
                    or result.get("job_name") != proof.get("job_name")
                    or result.get("check_app_id") != check_app_id
                    or result.get("check_run_id") != check_run_id):
                raise ValueError("v2 result direct executor differs from the verified required-gate check")
            seen_artifact_ids.add(result_artifact_id)
            seen_names.add(result_name)
            results_by_unit[unit_id] = result
            normalized_results.append({
                "artifact_id": result_artifact_id,
                "name": result_name,
                "payload": result,
            })
        if set(results_by_unit) != set(plan["unit_ids"]):
            raise ValueError("v2 result artifacts do not cover the complete required inventory")

        execution_jobs = proof.get("execution_jobs")
        if not isinstance(execution_jobs, list) or not execution_jobs:
            raise ValueError("v2 exact-attempt execution job proof is missing")
        job_keys = set()
        verified_jobs = []
        expected_job_fields = {
            "workflow_run_id", "run_attempt", "job_id", "job_name", "check_name",
            "check_app_id", "check_run_id", "head_sha", "status", "conclusion", "labels",
        }
        for job in execution_jobs:
            if not isinstance(job, dict) or set(job) != expected_job_fields:
                raise ValueError("v2 exact-attempt execution job record is malformed")
            if (job["workflow_run_id"] != run_id or job["run_attempt"] != run_attempt
                    or job["check_app_id"] != check_app_id or job["head_sha"] != workflow_sha):
                raise ValueError("v2 execution job belongs to another run, attempt, app, or workflow SHA")
            if (not isinstance(job["labels"], list)
                    or any(not isinstance(label, str) for label in job["labels"])
                    or job["labels"] != sorted(set(job["labels"]))):
                raise ValueError("v2 exact-attempt job runner labels are malformed")
            if (job["status"] not in ("queued", "in_progress", "completed")
                    or (job["status"] == "completed" and job["conclusion"] not in (
                        "success", "failure", "cancelled", "skipped", "timed_out",
                        "action_required", "neutral", "stale",
                    ))
                    or (job["status"] != "completed" and job["conclusion"] is not None)):
                raise ValueError("v2 exact-attempt workflow job state is malformed")
            job_id = positive_int(job["job_id"], "workflow job ID")
            child_check_id = positive_int(job["check_run_id"], "workflow job check-run ID")
            if (job_id, child_check_id) in job_keys:
                raise ValueError("v2 exact-attempt execution job is duplicated")
            job_keys.add((job_id, child_check_id))
            verified_jobs.append(job)
        gate_jobs = [job for job in verified_jobs if job["job_name"] == "required-gate"]
        if (len(gate_jobs) != 1 or gate_jobs[0]["job_id"] != proof.get("job_id")
                or gate_jobs[0]["check_run_id"] != check_run_id
                or gate_jobs[0]["check_name"] != "required-gate"):
            raise ValueError("v2 exact-attempt required-gate job/check locator mismatch")
        source_attempt = proof.get("trusted_source_attempt")
        expected_source_attempt = {
            "schema": "oasis7-ci-trusted-source-attempt/v1",
            "request_key": request_key,
            "workflow_run_id": run_id,
            "run_attempt": run_attempt,
            "check_app_id": check_app_id,
            "check_run_id": check_run_id,
            "job_id": gate_jobs[0]["job_id"],
            "job_name": "required-gate",
            "plan_artifact_id": plan_artifact_id,
            "plan_artifact_name": plan_name,
            "result_artifacts": [
                {"unit_id": item["payload"]["unit_id"],
                 "artifact_id": item["artifact_id"], "name": item["name"]}
                for item in sorted(normalized_results, key=lambda item: item["payload"]["unit_id"])
            ],
        }
        source_attempt_fields = set(expected_source_attempt)
        if (not isinstance(source_attempt, dict) or set(source_attempt) != source_attempt_fields
                or any(type(source_attempt.get(field)) is not int or source_attempt[field] <= 0
                       for field in (
                           "workflow_run_id", "run_attempt", "check_app_id", "check_run_id",
                           "job_id", "plan_artifact_id",
                       ))
                or not isinstance(source_attempt.get("result_artifacts"), list)
                or any(not isinstance(item, dict) or set(item) != {"unit_id", "artifact_id", "name"}
                       or not isinstance(item.get("unit_id"), str) or not item["unit_id"]
                       or type(item.get("artifact_id")) is not int or item["artifact_id"] <= 0
                       or not isinstance(item.get("name"), str)
                       for item in source_attempt["result_artifacts"])
                or source_attempt != expected_source_attempt):
            raise ValueError("v2 trusted source attempt differs from exact live readback")
        jobs_by_name = {}
        for job in verified_jobs:
            jobs_by_name.setdefault(job["job_name"], []).append(job)
        all_required_job_names = {"required-gate"}
        for children in required_child_jobs.values():
            all_required_job_names.update(children)
        for name in all_required_job_names:
            if len(jobs_by_name.get(name, [])) != 1:
                raise ValueError("v2 required exact-attempt job is missing or ambiguous: " + name)
            selected_job = jobs_by_name[name][0]
            if (selected_job["status"] != "completed"
                    or selected_job["conclusion"] != "success"):
                raise ValueError("v2 required exact-attempt job is pending or failed: " + name)
        verified_job_pairs = {(job["job_id"], job["check_run_id"]): job for job in verified_jobs}
        for unit_id, result in results_by_unit.items():
            result_jobs = result.get("execution_jobs")
            expected_names = sorted(["required-gate", *required_child_jobs[unit_id]])
            if (not isinstance(result_jobs, list)
                    or sorted(job.get("job_name") for job in result_jobs if isinstance(job, dict))
                       != expected_names):
                raise ValueError("v2 result omits or duplicates a required execution job")
            result_pairs = set()
            for job in result_jobs:
                if not isinstance(job, dict):
                    raise ValueError("v2 result execution job record is malformed")
                pair = (job.get("job_id"), job.get("check_run_id"))
                if (pair not in verified_job_pairs or pair in result_pairs
                        or job != verified_job_pairs[pair]):
                    raise ValueError("v2 result execution jobs differ from live attempt proof")
                result_pairs.add(pair)
            expected_pairs = {
                (jobs_by_name[name][0]["job_id"], jobs_by_name[name][0]["check_run_id"])
                for name in expected_names
            }
            if result_pairs != expected_pairs:
                raise ValueError("v2 result exact job set differs from trusted per-unit requirements")

        planner = canonical_planner(plan.get("planner_output"))
        if planner["planner_config_sha256"] != plan.get("planner_config_sha256"):
            raise ValueError("v2 planner config digest does not match the W-replayed planner")
        normalized_results.sort(key=lambda item: item["payload"]["unit_id"])
        verified_jobs.sort(key=lambda item: (item["job_name"], item["job_id"]))
        return {
            "planner": planner,
            "request_key": request_key,
            "request_identity": request_identity,
            "source_scope_oid": source_scope_oid,
            "trusted_policy_context": policy_context,
            "effective_policy_identity": effective_policy_identity,
            "required_plan_v2_artifact_id": plan_artifact_id,
            "required_plan_v2_artifact_name": plan_name,
            "required_plan_v2_payload": plan,
            "required_result_v2_artifacts": normalized_results,
            "trusted_planner_inventory": trusted_inventory,
            "execution_jobs": verified_jobs,
            "trusted_source_attempt": expected_source_attempt,
        }
    except (KeyError, TypeError, ValueError, ImportError, AttributeError) as exc:
        raise SystemExit("ci-ready-receipt: trusted v2 required evidence blocked: " + str(exc)) from exc

def _trusted_workflow_source(repository, workflow_sha):
    response=gh("api",f"repos/{repository}/contents/.github/workflows/rust.yml?ref={workflow_sha}")
    if response.get("encoding")!="base64" or not isinstance(response.get("content"),str):
        raise SystemExit("ci-ready-receipt: trusted workflow source is unavailable")
    # GitHub's Contents API line-wraps base64.  Remove only JSON-decoded ASCII
    # whitespace, then retain strict alphabet/padding validation.
    normalized="".join(response["content"].split())
    try: return base64.b64decode(normalized,validate=True)
    except Exception as exc: raise SystemExit(f"ci-ready-receipt: trusted workflow source is malformed: {exc}")

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
    if not present:
        workflow_source=_trusted_workflow_source(repository,proof.get("workflow_sha"))
        if b"cargo-package-profile-envelope" in workflow_source:
            raise SystemExit("ci-ready-receipt: package profile artifact missing from envelope-capable trusted workflow")
        planner_run_fields=RUN_FIELDS+VERSIONED_SELECTOR_FIELDS if planner.get("execution_contract") in SUPPORTED_EXECUTION_CONTRACTS else RUN_FIELDS
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

def _standalone_ready_continuation(repository, uid, issue, number, check_name, app,
                                   root, review_plan_path):
    """A plan is a locator; the live factory establishes ready authority itself."""
    pull = gh('api', f'repos/{repository}/pulls/{number}')
    from workflow_maintenance import maintenance_comment_id, read_maintenance_scope, read_ready_maintenance_continuation
    locator = maintenance_comment_id(pull.get('body') or '')
    if locator is None or pull.get('draft') is not False:
        return None
    if not review_plan_path:
        raise ValueError('ready maintenance receipt requires the existing review-plan locator')
    root = Path(root).resolve(strict=True)
    tool_directory = Path(__file__).absolute().parent
    if tool_directory.is_symlink() or tool_directory.parent.is_symlink():
        raise ValueError('standalone receipt tool directory is symlinked')
    head = (pull.get('head') or {}).get('sha')
    if subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'HEAD'], text=True).strip() != head:
        raise ValueError('standalone receipt root is not exact live H')
    closure = tuple('scripts/pm/' + name for name in (
        'ci-ready-receipt.py', 'ci_ready_receipt_identity.py', 'workflow_maintenance.py', 'task_primary_package.py'))
    scope = read_maintenance_scope(repository, locator, uid, number, head, required_tool_paths=closure)
    if scope['issue_number'] != issue:
        raise ValueError('standalone receipt Task Issue differs')
    for relative in closure:
        entry = subprocess.check_output(['git', '-C', str(root), 'ls-tree', head, '--', relative], text=True).split()
        path = tool_directory / Path(relative).name
        expected = subprocess.check_output(['git', '-C', str(root), 'show', head + ':' + relative])
        if (not entry or entry[0] not in ('100644', '100755') or path.is_symlink()
                or not path.is_file() or path.read_bytes() != expected):
            raise ValueError('standalone receipt approved closure differs: ' + relative)
    repository_info = gh('api', f'repos/{repository}')
    branch = repository_info.get('default_branch')
    q = gh('api', f'repos/{repository}/commits/{branch}').get('sha')
    if not re.fullmatch(r'[0-9a-f]{40,64}', str(q)):
        raise ValueError('standalone receipt protected Q is unresolved')
    # Direct entry must not execute a changed H carrier as release authority.
    for name in ('integration_ci.py', 'integration_executor_contract.py', 'ci_input_scope.py',
                 'ci_required_artifact_v2.py', 'ci_reuse_policy.py'):
        path = tool_directory / name
        expected = subprocess.check_output(['git', '-C', str(root), 'show', q + ':scripts/pm/' + name])
        if path.is_symlink() or not path.is_file() or path.read_bytes() != expected:
            raise ValueError('standalone receipt carrier differs from protected Q: ' + name)
    return read_ready_maintenance_continuation(root, repository, locator, uid, number, head,
        check_name=check_name, app=int(app), review_plan_path=review_plan_path)


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
    p.add_argument('--root', default=str(Path(__file__).resolve().parents[2]), help='canonical Task worktree')
    p.add_argument('--review-plan', help='existing exact-head review-plan locator for independently authenticated ready continuation')
    a=p.parse_args()
    existing=json.loads(Path(a.receipt).read_text()) if a.receipt else {}
    if existing.get('request_key') and a.request_key!=existing['request_key']:
        raise SystemExit('ci-ready-receipt: explicit --request-key is required and must match the existing keyed receipt')
    bound_base_ref = a.base_ref or existing.get("base_ref")
    continuation = None
    if a.allow_ready_pr and a.request_key is None and not (a.integration_run_id or existing.get('integration_run_id')):
        continuation = _standalone_ready_continuation(a.repository,a.task_uid,a.task_issue_number,
            a.pr_number,a.check_name,a.check_app_id,a.root,a.review_plan)
    plan_locator = None
    if a.review_plan:
        plan_locator = json.loads(Path(a.review_plan).read_text()).get('strict_exception_comment_id')
    pr,run,base_oid,head_oid=selected_live(a.repository,a.task_uid,a.task_issue_number,a.pr_number,a.check_name,a.check_app_id,a.allow_ready_pr,bound_base_ref,a.integration_run_id or existing.get('integration_run_id'),request_key=a.request_key,ready_continuation=continuation,strict_exception_plan_locator=plan_locator,canonical_root=a.root)
    keyed_v2_evidence = None
    if a.request_key is not None:
        proof=run.get('_integration') or {}
        if proof.get('request_key')!=a.request_key:
            raise SystemExit('ci-ready-receipt: keyed integration request identity mismatch')
        keyed_v2_evidence = _verified_v2_required_evidence(
            a.repository, run, proof, request_key=a.request_key,
            request_identity=proof.get("request_identity"),
            task_uid=a.task_uid, pr_number=a.pr_number,
            integration_base_oid=base_oid, head_oid=head_oid,
        )
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
    if keyed_v2_evidence is not None:
        planner=keyed_v2_evidence["planner"]
    else:
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
    if planner.get("execution_contract") in SUPPORTED_EXECUTION_CONTRACTS:
        payload["execution_contract"]=planner["execution_contract"]
    if old is None or "base_ref" in old:
        payload["base_ref"] = pr.get("base", {}).get("ref")
    if old is None or "ci_validation_mode" in old:
        payload["ci_validation_mode"] = ("trusted_integration" if run.get("_integration")
                                         else "current_target_pr" if run.get("_current_target") else "ordinary_pr")
    if run.get("_current_target"):
        payload["current_target_proof"] = run["_current_target"]
    if old is None or "live_validation" in old:
        payload["live_validation"] = "ci-ready-receipt-live"
    if planner.get("impact_projection_status") == "verified":
        payload.update(impact_projection_schema=planner["impact_projection_schema"],impact_projection_digest=planner["impact_projection_digest"],impact_projection_planner_digest=planner["impact_projection_planner_digest"])
    if old is None or 'scope_base_oid' in old:
        payload['scope_base_oid'] = (
            keyed_v2_evidence["source_scope_oid"] if keyed_v2_evidence is not None
            else scope_base_for_run(a.repository, base_oid, head_oid)
        )
        payload['integration_base_oid'] = base_oid
    if run.get('_integration'):
        proof=run['_integration']
        payload.update(integration_run_id=proof['workflow_run_id'],tested_tree_oid=proof['tested_tree_oid'],tested_commit_oid=proof['tested_commit_oid'],workflow_sha=proof['workflow_sha'])
        if old is None or any(key in old for key in ('request_id', 'workflow_ref', 'run_attempt')):
            payload.update(workflow_ref=proof['workflow_ref'],request_id=proof['request_id'],request_created_at=proof['request_created_at'],run_id=proof['run_id'],run_attempt=proof['run_attempt'],live_validation='ci-ready-receipt-live',trusted_integration_artifact=True)
        if keyed_v2_evidence is not None:
            payload.update({
                key: keyed_v2_evidence[key]
                for key in (
                    "request_key", "request_identity", "source_scope_oid",
                    "trusted_policy_context", "effective_policy_identity",
                    "required_plan_v2_artifact_id", "required_plan_v2_artifact_name",
                    "required_plan_v2_payload", "required_result_v2_artifacts",
                    "trusted_planner_inventory", "execution_jobs", "trusted_source_attempt",
                )
            })
            payload["bootstrap_epoch"] = keyed_v2_evidence["request_identity"]["bootstrap_epoch"]
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
def _trusted_validation_request(request_key,repository,uid,number,head_oid,
                                expected_integration_base_oid=None):
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
    integration_base_oid=record.get("integration_base_oid")
    if not isinstance(integration_base_oid,str) or not re.fullmatch(r"[0-9a-f]{40,64}",integration_base_oid):
        raise ValueError("validation request journal immutable integration base is invalid")
    if (expected_integration_base_oid is not None
            and integration_base_oid!=expected_integration_base_oid):
        raise ValueError("validation request journal immutable integration base mismatch")
    return record,identity


def _require_historical_base_ancestor_of_target(repository,historical_base_oid,current_target_oid):
    """Prove the journal's immutable B is still an ancestor of live PR target Q."""
    if (not isinstance(historical_base_oid,str)
            or not re.fullmatch(r"[0-9a-f]{40,64}",historical_base_oid)
            or not isinstance(current_target_oid,str)
            or not re.fullmatch(r"[0-9a-f]{40,64}",current_target_oid)):
        raise ValueError("historical integration base or current PR target is invalid")
    if historical_base_oid==current_target_oid:
        return
    comparison=gh(
        "api",
        f"repos/{repository}/compare/{historical_base_oid}...{current_target_oid}",
    )
    base_commit=comparison.get("base_commit") if isinstance(comparison,dict) else None
    head_commit=comparison.get("head_commit") if isinstance(comparison,dict) else None
    merge_base_commit=comparison.get("merge_base_commit") if isinstance(comparison,dict) else None
    if (not isinstance(base_commit,dict) or not isinstance(head_commit,dict)
            or not isinstance(merge_base_commit,dict)
            or base_commit.get("sha")!=historical_base_oid
            or head_commit.get("sha")!=current_target_oid
            or merge_base_commit.get("sha")!=historical_base_oid):
        raise ValueError("historical integration base is not an ancestor of current PR target")


def _pr_workflow_changed(repository, number, pr, root):
    try:
        paths = subprocess.check_output(['git', '-C', str(root), 'diff', '--name-only',
            pr['base']['sha'] + '...' + pr['head']['sha']], text=True, stderr=subprocess.DEVNULL).splitlines()
        return '.github/workflows/rust.yml' in paths
    except subprocess.CalledProcessError:
        pages = gh('api', '--paginate', '--slurp', f'repos/{repository}/pulls/{number}/files?per_page=100')
        if not isinstance(pages, list) or any(not isinstance(page, list) for page in pages):
            raise ValueError('strict exception diff inventory unreadable')
        return any(item.get('filename') == '.github/workflows/rust.yml' for page in pages for item in page)


def selected_live(repository,uid,issue,number,check_name,app,allow_ready_pr=False,base_ref=None,
                  integration_run_id=None, require_integration=False, require_dispatch=False,
                  request_key=None, ready_continuation=None, strict_exception_plan_locator=None, canonical_root=None):
    pr=gh('api',f'repos/{repository}/pulls/{number}')
    if (not allow_ready_pr and not pr.get('draft')) or f'Refs #{issue}' not in (pr.get('body') or '') or f'Task: {uid}' not in (pr.get('body') or ''):
        raise SystemExit('ci-ready-receipt: manual integration task/draft identity mismatch')
    if pr.get('state')!='open' or pr.get('merged'):
        raise SystemExit('ci-ready-receipt: integration PR not open')
    if base_ref and pr['base']['ref']!=base_ref: raise SystemExit('ci-ready-receipt: manual integration base ref mismatch')
    maintenance_selected = bool(re.search(
        r"^Workflow Maintenance Authority: [1-9][0-9]*$", pr.get("body", ""), re.M))
    strict_selector = (request_key is not None or integration_run_id is not None or require_integration)
    import ci_ready_receipt_identity as identity
    root = Path(canonical_root) if canonical_root is not None else Path(__file__).resolve().parents[2]
    # Inspect immutable workflow bytes, not a risk label. Ordinary fixtures and
    # helper-only changes retain their lazy path; real orchestration changes
    # independently read current Task evidence even if the PR marker was hidden.
    pointer = bool(re.search(r'(?m)^Strict Integration Exception:', pr.get('body') or ''))
    selected_exception = None
    potential = pointer or strict_exception_plan_locator is not None
    if not potential and not strict_selector:
        potential = _pr_workflow_changed(repository, number, pr, root)
    if potential:
        from integration_ci import default_branch_head
        q = default_branch_head(repository, pr['base']['ref'])
        selected_exception = identity.select_strict_exception_locator(root=root, repository=repository,
            task_uid=uid, issue=issue, pr_number=number, source_head_oid=pr['head']['sha'], target_oid=q,
            pr_body=pr.get('body'), plan_locator=strict_exception_plan_locator, github=lambda *args: gh('api', *args))
    if selected_exception is not None:
        producer = identity._strict_exception_module()
        facts = producer.read_protected(repository, uid, issue, pr, q, root)
        if facts is None:
            raise SystemExit('ci-ready-receipt: selected strict exception facts are unavailable')
        require_integration = True
        require_dispatch = True
        strict_selector = True
    if not strict_selector and not maintenance_selected:
        # Ordinary proof is complete from the live PR check itself. Do not load
        # integration history merely to establish that no historical dispatch
        # ever existed; a currently bound request uses the explicit branch below.
        return live(repository,uid,issue,number,check_name,app,allow_ready_pr,base_ref,ordinary_pr=True)

    from integration_ci import current_request, verified_run
    import integration_ci
    # PR-associated base.sha can remain historical after an additive source
    # sync. Strict legacy requests bind the independently resolved live ref.
    current_target_oid=integration_ci.default_branch_head(repository,pr['base']['ref'])
    head=pr['head']['sha']
    base=current_target_oid
    try:
        request_record=None
        request_identity=None
        effective_policy=None
        policy_context=None
        if request_key is not None:
            request_record,request_identity=_trusted_validation_request(
              request_key,repository,uid,number,head)
            base=request_record["integration_base_oid"]
            _require_historical_base_ancestor_of_target(repository,base,current_target_oid)
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
            if selected_exception is not None:
                required_scope = planner_for_run(repository, check, base_oid=base, head_oid=head)
                if (required_scope.get('scope') != 'full'
                        or required_scope.get('impact_projection_test_profile') != 'full'):
                    raise ValueError('selected strict exception run does not execute required full scope/profile')
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
                or (request_key is None and fresh.get('base',{}).get('sha')!=pr.get('base',{}).get('sha'))
                or fresh.get('base',{}).get('ref')!=pr['base']['ref']
                or fresh.get('head',{}).get('sha')!=head):
                raise ValueError('PR identity or admission changed during integration verification')
            if request_key is None and integration_ci.default_branch_head(repository,fresh['base']['ref'])!=current_target_oid:
                raise ValueError('default-branch target moved during integration verification')
            if request_key is not None:
                current_target_oid=integration_ci.default_branch_head(repository,fresh['base']['ref'])
                if (not isinstance(current_target_oid,str)
                        or not re.fullmatch(r'[0-9a-f]{40,64}',current_target_oid)):
                    raise ValueError('current default-branch target is invalid')
                _require_historical_base_ancestor_of_target(repository,base,current_target_oid)
                proof.update(
                  integration_base_oid=base,
                  current_target_oid=current_target_oid,
                )
            return fresh,{**check,'_integration':proof},base,head
        if integration_run_id is not None:
            raise ValueError('explicit integration locator absent from verified current request range')
    except (ValueError,KeyError,OSError,ImportError,TypeError,subprocess.SubprocessError) as exc:
        raise SystemExit('ci-ready-receipt: current request blocked: '+str(exc)) from exc
    if (request_key is None and integration_run_id is None and maintenance_selected and selected_exception is None):
        ordinary = live(repository,uid,issue,number,check_name,app,allow_ready_pr,base_ref,ordinary_pr=True)
        candidate_pr, candidate_check, candidate_base, candidate_head = ordinary
        planner = planner_for_run(repository,candidate_check,base_oid=candidate_base,head_oid=candidate_head)
        if "current_target_proof" in planner:
            try:
                proof = validate_current_target_pr_proof(repository,candidate_pr,candidate_check,planner,
                                                        current_target_oid=current_target_oid,
                                                        ready_continuation=ready_continuation)
                if proof["task_uid"] != uid or proof["task_issue_number"] != issue:
                    raise ValueError("current-target proof differs from caller Task identity")
                if current_request(repository,uid,number,base,head,pr["base"]["ref"]) is not None:
                    raise ValueError("current integration request changed during PR proof verification")
            except (ValueError,KeyError,TypeError,OSError,subprocess.SubprocessError) as exc:
                raise SystemExit("ci-ready-receipt: current-target PR proof blocked: " + str(exc)) from exc
            return candidate_pr,{**candidate_check,"_current_target":proof},candidate_base,candidate_head
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
