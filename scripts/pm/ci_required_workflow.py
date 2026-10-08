#!/usr/bin/env python3
"""Attempt-local transport and execution for the required workflow.

The scheduling artifact is never v2 evidence. Authority is reconstructed from
the frozen Git object, and workers execute on the separately frozen tested M.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tarfile
import tempfile

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ci_required_execution as execution
import integration_executor_contract as contracts

SCHEMA = "oasis7-required-transport/v1"


def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def write(path, value):
    Path(path).write_text(json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n", encoding="utf-8")


def digest(path):
    return "sha256:" + hashlib.sha256(Path(path).read_bytes()).hexdigest()


def command(argv, root=None, env=None):
    return subprocess.check_output(argv, cwd=root, env=env, text=True).strip()


def git(root, *argv):
    return command(["git", *argv], root)


def export(values):
    with Path(os.environ["GITHUB_OUTPUT"]).open("a", encoding="utf-8") as stream:
        for key, value in values.items():
            if "\n" in str(value):
                raise ValueError("workflow output contains newline")
            stream.write(f"{key}={value}\n")


def archive(root, oid, target):
    target.mkdir(parents=True, exist_ok=True)
    # Git owns paths in this archive; never extract paths from artifact input.
    with tempfile.TemporaryFile() as stream:
        subprocess.run(["git", "archive", oid], cwd=root, stdout=stream, check=True)
        stream.seek(0)
        with tarfile.open(fileobj=stream) as data:
            data.extractall(target)


def runtime_identity(identity):
    expected = {"repository": os.environ["GITHUB_REPOSITORY"],
                "run_id": int(os.environ["GITHUB_RUN_ID"]),
                "run_attempt": int(os.environ["GITHUB_RUN_ATTEMPT"]),
                "event_name": os.environ["GITHUB_EVENT_NAME"],
                "workflow_sha": os.environ["GITHUB_WORKFLOW_SHA"],
                "run_head_sha": os.environ["OASIS7_RUN_HEAD_SHA"]}
    for key, value in expected.items():
        if identity.get(key) != value:
            raise ValueError(f"transport {key} differs from this exact attempt; Re-run all jobs")


def artifact_identity(artifact_id, expected_name):
    artifact = json.loads(command(["gh", "api", f"repos/{os.environ['GITHUB_REPOSITORY']}/actions/artifacts/{artifact_id}"]))
    if (artifact.get("id") != int(artifact_id) or artifact.get("name") != expected_name
            or artifact.get("expired") is not False
            or artifact.get("workflow_run", {}).get("id") != int(os.environ["GITHUB_RUN_ID"])):
        raise ValueError("artifact is not the exact named artifact from this run/attempt")
    return artifact


def authority_context(root, policy_path=None):
    approved = None
    if policy_path and Path(policy_path).is_file():
        approved = read(policy_path)["effective_policy"]["approved_executor_contract_digests"]
    return contracts.resolve_execution_layout(root, approved_digests=approved)


def candidate_environment():
    env = dict(os.environ)
    for key in ("GH_TOKEN", "GITHUB_TOKEN"):
        env.pop(key, None)
    return env


def selectors(selection):
    special = {"run_required_gate_baseline": "OASIS7_CI_RUN_REQUIRED_GATE_BASELINE",
               "run_oasis7_workspace_support_crate_tests": "OASIS7_CI_RUN_WORKSPACE_SUPPORT_CRATE_TESTS"}
    env = candidate_environment()
    for key, value in selection["planner_output"].items():
        if key.startswith(("run_", "needs_")):
            env[special.get(key, "OASIS7_CI_" + key.upper())] = str(value)
    output = selection["planner_output"]
    env.update(OASIS7_CI_EXECUTION_CONTRACT=output.get("execution_contract", ""),
               OASIS7_PRODUCT_DOC_BASE=output["source_scope_base"],
               OASIS7_PRODUCT_DOC_HEAD=output["head_oid"],
               OASIS7_CARGO_SCOPE_BASE=output["source_scope_base"],
               OASIS7_CARGO_SCOPE_HEAD=output["head_oid"],
               OASIS7_CARGO_SCOPE_INTEGRATION_BASE=output["integration_base_oid"],
               OASIS7_CARGO_SCOPE_TRUSTED_FULL_PLAN=str(output.get("scope") == "full").lower(),
               OASIS7_CI_RUN_HOSTED_ACCOUNT_SMOKE="false", OASIS7_CI_RUN_PROVIDER_LIVE_GATE="false")
    return env


def driver(root, authority, selection, tier, transport, extra=()):
    env = selectors(selection)
    env["OASIS7_CARGO_SCOPE_FULL_PLAN"] = str(transport / "scope.json")
    argv = ["bash", str(authority / "scripts/ci-tests.sh"), tier, *extra, "--repo-root", str(root)]
    if (transport / "impact-projection.json").is_file():
        argv.extend(("--impact-projection", str(transport / "impact-projection.json")))
    subprocess.run(argv, cwd=root, env=env, check=True)


def prepare(args):
    root, target = Path(args.root).resolve(), Path(args.target).resolve()
    transport = Path(args.transport).resolve()
    transport.mkdir(parents=True, exist_ok=True)
    scope = read(args.scope)
    authority_oid = args.authority
    authority = transport / "authority"
    archive(root, authority_oid, authority)
    layout = authority_context(authority, args.policy)
    tested = git(target, "rev-parse", "HEAD")
    identity = {"repository": os.environ["GITHUB_REPOSITORY"], "run_id": int(os.environ["GITHUB_RUN_ID"]),
                "run_attempt": int(os.environ["GITHUB_RUN_ATTEMPT"]), "event_name": os.environ["GITHUB_EVENT_NAME"],
                "run_mode": args.run_mode or "ci", "base_sha": scope["base_oid"], "source_head_sha": scope["head_oid"],
                "tested_sha": tested, "tested_tree": git(target, "rev-parse", "HEAD^{tree}"),
                "workflow_sha": os.environ["GITHUB_WORKFLOW_SHA"], "run_head_sha": os.environ["OASIS7_RUN_HEAD_SHA"], "task_uid": scope.get("task_uid", ""),
                "pr_number": args.pr_number, "request_key": args.request_key,
                "planner_digest": scope.get("planner_digest", "").removeprefix("sha256:") or hashlib.sha256(subprocess.check_output(["git", "show", (args.planner_authority or authority_oid) + ":scripts/plan-rust-required-scope.py"], cwd=root)).hexdigest(), "config_digest": scope["planner_config_sha256"].removeprefix("sha256:"),
                "executor_digest": layout["executor_contract_digest"].removeprefix("sha256:"), "source_scope": scope["source_scope_base"],
                "impact_projection_digest": scope.get("impact_projection_digest", "").removeprefix("sha256:") or hashlib.sha256(b"").hexdigest()}
    runtime_identity(identity)
    selection = execution.selection_from_planner(scope, identity["event_name"], identity["run_mode"])
    write(transport / "scope.json", scope)
    write(transport / "identity.json", identity)
    write(transport / "selection.json", selection)
    if args.projection and Path(args.projection).is_file():
        (transport / "impact-projection.json").write_bytes(Path(args.projection).read_bytes())
    if args.policy and Path(args.policy).is_file():
        (transport / "trusted-ci-policy.json").write_bytes(Path(args.policy).read_bytes())
    if args.integration and Path(args.integration).is_file():
        write(transport / "integration.json", read(args.integration))
    if layout["execution_layout"] == "required-parallel/v1":
        subprocess.run([sys.executable, "-I", str(authority / "scripts/pm/ci_required_execution.py"), "plan",
                        "--selection", str(transport / "selection.json"), "--identity", str(transport / "identity.json"),
                        "--output", str(transport / "schedule.json"), "--layout", layout["execution_layout"]], check=True)
        driver(target, authority, selection, "required-plan-baseline", transport)
        schedule = read(transport / "schedule.json")
        workers = schedule["workers"]
    else:
        workers = []
        write(transport / "schedule.json", {"execution_layout": layout["execution_layout"], "workers": []})
    refs = []
    prerequisites = sorted(set((authority_oid, identity["base_sha"], identity["source_head_sha"], identity["source_scope"])))
    try:
        for index, oid in enumerate([tested, *prerequisites]):
            ref = f"refs/oasis7-required-transport/{identity['run_id']}/{identity['run_attempt']}/{index}"
            git(root, "update-ref", ref, oid)
            refs.append(ref)
        if tested not in prerequisites:
            git(root, "bundle", "create", str(transport / "source.bundle"), refs[0], *("^" + ref for ref in refs[1:]))
    finally:
        for ref in refs:
            git(root, "update-ref", "-d", ref)
    # Only exact object bytes and JSON inputs travel; authority is reconstructed.
    import shutil
    shutil.rmtree(authority)
    members = sorted(path.name for path in transport.iterdir() if path.is_file())
    write(transport / "transport.json", {"schema": SCHEMA, "identity": identity, "authority_oid": authority_oid,
          "planner_authority_oid": args.planner_authority or authority_oid,
          "prerequisite_oids": prerequisites,
          "execution_layout": layout["execution_layout"], "members": {name: digest(transport / name) for name in members}})
    matrix = [{"worker": item} for item in workers]
    export({"worker_matrix": json.dumps(matrix, separators=(",", ":")), "has_workers": str(bool(matrix)).lower(),
            "execution_layout": layout["execution_layout"], "tested_sha": tested, "tested_tree": identity["tested_tree"],
            "authority_oid": authority_oid, "transport_digest": digest(transport / "transport.json"),
            "transport_name": f"required-transport-{identity['run_id']}-{identity['run_attempt']}"})


def materialize(args):
    transport, root, authority = Path(args.transport).resolve(), Path(args.root).resolve(), Path(args.authority_root).resolve()
    if digest(transport / "transport.json") != args.transport_digest:
        raise ValueError("transport manifest differs from independently frozen plan digest")
    value = read(transport / "transport.json")
    if value.get("schema") != SCHEMA:
        raise ValueError("unknown required transport")
    runtime_identity(value["identity"])
    artifact_identity(args.artifact_id, f"required-transport-{os.environ['GITHUB_RUN_ID']}-{os.environ['GITHUB_RUN_ATTEMPT']}")
    actual = {path.name for path in transport.iterdir() if path.is_file()} - {"transport.json"}
    if actual != set(value["members"]) or any(digest(transport / name) != checksum for name, checksum in value["members"].items()):
        raise ValueError("required transport content digest differs")
    for key, expected in (("tested_sha", args.tested_sha), ("tested_tree", args.tested_tree)):
        if value["identity"][key] != expected:
            raise ValueError(f"frozen {key} transport identity differs")
    if value["authority_oid"] != args.authority_oid:
        raise ValueError("frozen authority differs")
    expected_prerequisites = sorted(set((args.authority_oid, value["identity"]["base_sha"], value["identity"]["source_head_sha"], value["identity"]["source_scope"])))
    if value.get("prerequisite_oids") != expected_prerequisites:
        raise ValueError("transport prerequisite identity differs")
    for oid in expected_prerequisites:
        if not re.fullmatch(r"[0-9a-f]{40,64}", oid):
            raise ValueError("invalid prerequisite Git object")
        known = subprocess.run(["git", "cat-file", "-e", oid + "^{commit}"], cwd=root, capture_output=True).returncode == 0
        if not known:
            git(root, "fetch", "--no-tags", "origin", oid)
        if git(root, "rev-parse", oid + "^{commit}") != oid:
            raise ValueError("remote prerequisite differs from exact frozen Git object")
    if (transport / "source.bundle").is_file():
        git(root, "bundle", "verify", str(transport / "source.bundle"))
        git(root, "fetch", str(transport / "source.bundle"), "+refs/oasis7-required-transport/*:refs/oasis7-required-transport/*")
    git(root, "checkout", "--detach", args.tested_sha)
    if git(root, "rev-parse", "HEAD^{tree}") != args.tested_tree:
        raise ValueError("checkout is not exact frozen M/tree")
    git(root, "worktree", "add", "--detach", str(authority), args.authority_oid)
    if value["planner_authority_oid"] != args.authority_oid:
        planner_authority = authority.with_name("planner-authority")
        git(root, "worktree", "add", "--detach", str(planner_authority), value["planner_authority_oid"])
    layout = authority_context(authority, transport / "trusted-ci-policy.json")
    if (layout["execution_layout"] != value["execution_layout"]
            or layout["executor_contract_digest"].removeprefix("sha256:") != value["identity"]["executor_digest"]):
        raise ValueError("trusted executor differs from frozen dispatch")
    if read(transport / "identity.json") != value["identity"]:
        raise ValueError("identity member differs from transport identity")
    # The artifact's producer must be the completed plan check in this attempt.
    # Legacy W lacks the extended event API, so the new orchestration reader is
    # tested here while execution remains exclusively on the old trusted driver.
    reader_path = None if layout["execution_layout"] == "required-parallel/v1" else Path(__file__).resolve().parent / "integration_ci.py"
    attempt_jobs(value["identity"], ["required-plan"], authority, validate_workers=False, reader_path=reader_path)
    export({"execution_layout": layout["execution_layout"]})


def profile_plan(root, authority, transport, output):
    identity = read(transport / "identity.json")
    output.mkdir(parents=True, exist_ok=True)
    planner = authority / "scripts/pm/cargo_package_profile_planner.py"
    subprocess.run([sys.executable, str(planner), "--repo-root", str(root),
                    "--integration-base", identity["base_sha"], "--source-head", identity["source_head_sha"],
                    "--policy", ".pm/cargo-package-scope-policy.json", "--checker", "scripts/pm/check-cargo-package-scope",
                    "--profile", "native", "--output", str(output / "cargo-package-profile-plan.json")], cwd=root, env=candidate_environment(), check=True)
    return read(output / "cargo-package-profile-plan.json")


def execute_profile(root, authority, transport, output):
    plan = profile_plan(root, authority, transport, output)
    results = []
    for item in plan["items"]:
        argv = item.get("command")
        if not isinstance(argv, list) or not argv or argv[0] != "cargo" or any(not isinstance(x, str) for x in argv):
            raise ValueError("trusted Cargo profile emitted invalid argv")
        env = candidate_environment()
        env.pop("RUSTC_WRAPPER", None)
        if item["target"] == "native":
            env["OASIS7_WASM_BUILD_STD"] = "0"
        result = subprocess.run(argv, cwd=root, env=env, check=False)
        results.append({"item_id": item["id"], "status": "passed" if result.returncode == 0 else "failed",
                        "exit_code": result.returncode, "plan_id": plan["plan_id"], "command_digest": item["command_digest"],
                        "toolchain": plan["trusted_authority"]["toolchain"],
                        "profile": {key: item[key] for key in ("package", "profile", "target", "features")},
                        **{key: plan[key] for key in ("integration_base", "source_head", "tested_tree")}})
        if result.returncode:
            write(output / "cargo-package-profile-results.json", results)
            raise ValueError("native Cargo profile item failed")
    write(output / "cargo-package-profile-results.json", results)


def worker(args):
    transport, root, authority = Path(args.transport).resolve(), Path(args.root).resolve(), Path(args.authority_root).resolve()
    selection, identity = read(transport / "selection.json"), read(transport / "identity.json")
    runtime_identity(identity)
    if read(transport / "transport.json")["execution_layout"] != "required-parallel/v1":
        raise ValueError("serial layout cannot dispatch workers")
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    if args.worker == "native" and identity["run_mode"] == "integration_revalidation":
        execute_profile(root, authority, transport, output / "profile")
    env = selectors(selection)
    env["OASIS7_CARGO_SCOPE_FULL_PLAN"] = str(transport / "scope.json")
    env["OASIS7_PRODUCT_DOC_BASE"] = identity["source_scope"]
    env["OASIS7_PRODUCT_DOC_HEAD"] = identity["source_head_sha"]
    subprocess.run([sys.executable, "-I", str(authority / "scripts/pm/ci_required_execution.py"), "run-worker",
                    "--plan", str(transport / "schedule.json"), "--worker", args.worker,
                    "--root", str(root), "--result", str(output / "worker.json")], cwd=root, env=env, check=True)


def resources(args):
    selection = read(Path(args.transport) / "selection.json")
    resources = execution.worker_resources(selection, args.worker)
    export(resources)


def attempt_jobs(identity, names, authority, *, validate_workers=True, reader_path=None):
    # This function validates each job AND check-run via GitHub's attempt endpoint.
    reader_path = reader_path or authority / "scripts/pm/integration_ci.py"
    sys.path.insert(0, str(reader_path.parent))
    import importlib.util
    spec = importlib.util.spec_from_file_location("required_attempt_integration", reader_path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    app_id = 15368  # Existing canonical GitHub Actions app, never artifact input.
    pages = json.loads(command(["gh", "api", f"repos/{identity['repository']}/actions/runs/{identity['run_id']}/attempts/{identity['run_attempt']}/jobs?per_page=100", "--paginate", "--slurp"]))
    raw_jobs = [job for page in pages for job in page["jobs"]]
    worker_names = [job["name"] for job in raw_jobs if str(job.get("name", "")).startswith("required-work")]
    if validate_workers and set(names) == {"required-plan"}:
        aggregate = [job for job in raw_jobs if job.get("name") == "required-work"]
        if aggregate:
            if len(aggregate) != 1 or aggregate[0]["status"] != "completed" or aggregate[0]["conclusion"] != "skipped":
                raise ValueError("empty selected matrix may only skip its explicit aggregate")
            worker_names.remove("required-work")
    expected_workers = set(names) - {"required-plan"}
    if validate_workers and (len(worker_names) != len(expected_workers) or set(worker_names) != expected_workers):
        raise ValueError("actual worker job set differs from trusted selected matrix")
    # This API validates ALL raw job/run/head identities and unique names, then
    # obtains exact check runs for every selected obligation. A legitimately
    # skipped empty matrix has no execution obligation or necessarily a check.
    selected = module.attempt_execution_jobs(identity["repository"], identity["run_id"], identity["run_attempt"],
                                         identity["run_head_sha"], app_id, job_names=set(names), require_completed=True,
                                         expected_event=identity["event_name"])
    if ({job["job_name"] for job in selected} != set(names)
            or any(job["status"] != "completed" or job["conclusion"] != "success" for job in selected)):
        raise ValueError("required plan/worker job is not exact completed/success; Re-run all jobs")
    return selected


def replay_selection(authority, transport, selection, identity):
    frozen = selection["planner_output"]
    planner_authority = authority
    if read(transport / "transport.json")["planner_authority_oid"] != read(transport / "transport.json")["authority_oid"]:
        planner_authority = authority.with_name("planner-authority")
    replay = [sys.executable, "-I", str(planner_authority / "scripts/plan-rust-required-scope.py"),
              "--event-name", identity["event_name"], "--base-ref", identity["base_sha"],
              "--head-ref", identity["source_head_sha"], "--config", str(authority / "scripts/ci-required-scope.v2.json")]
    if identity["run_mode"] != "ci":
        replay.extend(("--run-mode", identity["run_mode"]))
    if (transport / "impact-projection.json").is_file():
        replay.extend(("--impact-projection", str(transport / "impact-projection.json"),
                       "--task-uid", identity["task_uid"], "--scope-base-oid", identity["source_scope"]))
    actual = {}
    for line in command(replay, planner_authority).splitlines():
        key, separator, value = line.partition("=")
        if not separator or key in actual:
            raise ValueError("trusted gate planner emitted malformed selection")
        actual[key] = value
    expected = dict(actual)
    expected.update(base_oid=identity["base_sha"], integration_base_oid=identity["base_sha"], head_oid=identity["source_head_sha"],
                    source_scope_base=identity["source_scope"], integration_base=identity["base_sha"],
                    source_head=identity["source_head_sha"], task_uid=identity["task_uid"],
                    planner_authority_oid=read(transport / "transport.json")["planner_authority_oid"])
    if "maintenance_authority_comment_id" in frozen:
        if not re.fullmatch(r"[1-9][0-9]*", frozen["maintenance_authority_comment_id"]):
            raise ValueError("frozen maintenance authority locator is malformed")
        expected["maintenance_authority_comment_id"] = frozen["maintenance_authority_comment_id"]
    if frozen != expected:
        raise ValueError("gate trusted planner recomputation differs from frozen selection")
    expected_digests = {
        "planner_digest": actual.get("planner_digest", "").removeprefix("sha256:") or hashlib.sha256((planner_authority / "scripts/plan-rust-required-scope.py").read_bytes()).hexdigest(),
        "config_digest": hashlib.sha256((authority / "scripts/ci-required-scope.v2.json").read_bytes()).hexdigest(),
        "impact_projection_digest": actual.get("impact_projection_digest", "").removeprefix("sha256:") or hashlib.sha256(b"").hexdigest(),
    }
    if any(identity.get(key) != value for key, value in expected_digests.items()):
        raise ValueError("frozen identity planner/config/projection digest differs from trusted scope")
    recomputed = execution.selection_from_planner(expected, identity["event_name"], identity["run_mode"])
    if recomputed != selection:
        raise ValueError("gate unit/selector/resource selection differs from trusted recomputation")
    return recomputed


def complete_profile(root, authority, transport, results, schedule, identity, jobs):
    profile_output = root / "output/cargo-package-profile"
    import shutil
    native_profile = results / "native/profile"
    if "native" in schedule["workers"]:
        if not native_profile.is_dir():
            raise ValueError("native worker Cargo profile evidence is missing")
        shutil.copytree(native_profile, profile_output, dirs_exist_ok=True)
    else:
        plan = profile_plan(root, authority, transport, profile_output)
        if plan["items"]:
            raise ValueError("nonempty Cargo profile requires selected native worker")
        write(profile_output / "cargo-package-profile-results.json", [])
    plan_path = profile_output / "cargo-package-profile-plan.json"
    plan = read(plan_path)
    # Rebuild the exact profile in M; worker cannot invent a smaller plan.
    with tempfile.TemporaryDirectory() as scratch:
        rebuilt = profile_plan(root, authority, transport, Path(scratch))
    if rebuilt != plan:
        raise ValueError("worker Cargo profile differs from trusted gate recomputation")
    receipt = command([sys.executable, str(authority / "scripts/pm/cargo_package_profile_driver.py"),
                       "--plan", str(plan_path), "--results", str(profile_output / "cargo-package-profile-results.json"),
                       "--integration-base", identity["base_sha"], "--source-head", identity["source_head_sha"],
                       "--tested-tree", identity["tested_tree"]], root)
    (profile_output / "cargo-package-profile-receipt.json").write_text(receipt + "\n")
    # Envelope lookup is deliberately separate from completed worker lookup.
    import integration_ci
    gate = integration_ci.attempt_execution_jobs(identity["repository"], identity["run_id"], identity["run_attempt"],
            identity["workflow_sha"], jobs[0]["check_app_id"], job_names={"required-gate"})
    if (len(gate) != 1 or gate[0]["status"] != "in_progress" or gate[0].get("job_name") != "required-gate"
            or gate[0].get("workflow_run_id") != identity["run_id"] or gate[0].get("run_attempt") != identity["run_attempt"]
            or gate[0].get("head_sha") != identity["workflow_sha"] or gate[0].get("check_app_id") != 15368
            or type(gate[0].get("check_run_id")) is not int or gate[0]["check_run_id"] <= 0):
        raise ValueError("profile envelope needs the actual in-progress gate")
    envelope = {"schema": "oasis7-cargo-package-profile-envelope/v1", "repository": identity["repository"],
                "task_uid": identity["task_uid"], "pr_number": identity["pr_number"],
                "workflow_ref": os.environ["GITHUB_WORKFLOW_REF"], "workflow_sha": identity["workflow_sha"],
                "run_id": identity["run_id"], "run_attempt": identity["run_attempt"], "check_name": "required-gate",
                "check_app_id": gate[0]["check_app_id"], "check_run_id": gate[0]["check_run_id"],
                **{key: plan[key] for key in ("integration_base", "source_head", "tested_tree")},
                "plan_digest": digest(plan_path), "results_digest": digest(profile_output / "cargo-package-profile-results.json"),
                "receipt_digest": digest(profile_output / "cargo-package-profile-receipt.json")}
    write(profile_output / "cargo-package-profile-envelope.json", envelope)


def finalize(args):
    transport, root, authority = Path(args.transport).resolve(), Path(args.root).resolve(), Path(args.authority_root).resolve()
    selection, identity = read(transport / "selection.json"), read(transport / "identity.json")
    runtime_identity(identity)
    replay_selection(authority, transport, selection, identity)
    schedule = read(transport / "schedule.json")
    names = ["required-plan", *(f"required-work ({worker})" for worker in schedule["workers"])]
    jobs = attempt_jobs(identity, names, authority)
    # Resolve artifact IDs from this run using unique attempt-qualified names.
    pages = json.loads(command(["gh", "api", f"repos/{identity['repository']}/actions/runs/{identity['run_id']}/artifacts?per_page=100", "--paginate", "--slurp"]))
    artifacts = [item for page in pages for item in page["artifacts"]]
    results = Path(args.results).resolve()
    results.mkdir(parents=True, exist_ok=True)
    for worker in schedule["workers"]:
        name = f"required-worker-{identity['run_id']}-{identity['run_attempt']}-{worker}"
        matches = [item for item in artifacts if item.get("name") == name]
        if len(matches) != 1:
            raise ValueError("missing or duplicate same-attempt worker artifact; Re-run all jobs")
        artifact_identity(matches[0]["id"], name)
        worker_root = results / worker
        subprocess.run(["gh", "run", "download", str(identity["run_id"]), "--repo", identity["repository"],
                        "--name", name, "--dir", str(worker_root)], check=True)
        # Artifact name was read back uniquely before download; re-read its ID.
        artifact_identity(matches[0]["id"], name)
        write(results / (worker + ".json"), read(worker_root / "worker.json"))
    write(results / "jobs.data", [{**job, "name": job["job_name"], "run_id": job["workflow_run_id"]} for job in jobs])
    subprocess.run([sys.executable, "-I", str(authority / "scripts/pm/ci_required_execution.py"), "verify",
                    "--plan", str(transport / "schedule.json"), "--selection", str(transport / "selection.json"),
                    "--identity", str(transport / "identity.json"), "--results", str(results),
                    "--jobs", str(results / "jobs.data"), "--root", str(root)], check=True)
    profile_output = root / "output/cargo-package-profile"
    if identity["run_mode"] == "integration_revalidation":
        complete_profile(root, authority, transport, results, schedule, identity, jobs)
    driver(root, authority, selection, "required-gate-completion", transport)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    prepare_parser = sub.add_parser("prepare")
    for flag in ("root", "target", "transport", "scope", "authority"):
        prepare_parser.add_argument("--" + flag, required=True)
    for flag in ("policy", "projection", "integration", "run-mode", "request-key", "planner-authority"):
        prepare_parser.add_argument("--" + flag, default="")
    prepare_parser.add_argument("--pr-number", type=int, default=0)
    materialize_parser = sub.add_parser("materialize")
    for flag in ("root", "transport", "authority-root", "artifact-id", "tested-sha", "tested-tree", "authority-oid", "transport-digest"):
        materialize_parser.add_argument("--" + flag, required=True)
    for action in ("worker", "finalize"):
        phase = sub.add_parser(action)
        for flag in ("root", "transport", "authority-root"):
            phase.add_argument("--" + flag, required=True)
        if action == "worker":
            phase.add_argument("--worker", required=True)
            phase.add_argument("--output", required=True)
        else:
            phase.add_argument("--results", required=True)
    phase = sub.add_parser("resources")
    phase.add_argument("--transport", required=True)
    phase.add_argument("--worker", required=True)
    args = parser.parse_args()
    globals()[args.command](args)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError) as exc:
        raise SystemExit(str(exc)) from exc
