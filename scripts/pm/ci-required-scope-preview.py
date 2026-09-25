#!/usr/bin/env python3
"""Trusted, dispatch-only preview of versioned required-gate path selection.

This is diagnostic evidence only. It never writes CI receipts or readiness
artifacts and accepts no candidate-provided planner, policy, or expected plan.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path


REPOSITORY = "eng-cc/oasis7"
TASK_UID_RE = re.compile(r"(?<![A-Za-z0-9_])task_[0-9a-f]{32}(?![A-Za-z0-9_])")
SHA_RE = re.compile(r"[0-9a-f]{40}\Z")
FIXTURE = "scripts/fixtures/ci-required-scope.versioned-test.json"
EFFECTIVE_CONFIG = "scripts/ci-required-scope.v2.json"
EXPECTED_FIXTURE_SHA256 = "2d4228e5d7446c393ea3811084183860b5f8b4bb0e3b673957a3a020ab172dec"
SCENARIOS = {
    "ordinary_document": {
        "path": "doc/product/world-rules-core-gameplay.prd.md",
        "capability": None,
    },
    "doc_checker_contracts": {
        "path": "scripts/product-doc-governance-check.test.py",
        "capability": "doc_checker_contracts",
    },
    "workflow_governance": {
        "path": "scripts/pm/ci-ready-receipt.test.py",
        "capability": "workflow_governance",
    },
    "packaging_contracts": {
        "path": "scripts/testnet-packages-macos-arm64-contract.test.sh",
        "capability": "packaging_contracts",
    },
    "operational_contracts": {
        "path": "scripts/p2p-public-testnet-identity-v2-signing-tool.test.py",
        "capability": "operational_contracts",
    },
}
PLANNER_SELECTORS = (
    "run_oasis7_required_tests", "run_consensus_tests", "run_distfs_tests",
    "run_oasis7_node_tests", "run_oasis7_net_tests", "run_oasis7_net_libp2p_tests",
    "run_viewer_contract_tests", "run_viewer_wasm_check", "run_viewer_perf_smoke",
    "run_pixel_world_bridge_lib_tests", "run_pixel_world_bridge_wasm_check",
    "run_launcher_web_build", "run_oasis7_workspace_support_crate_tests",
    "run_scenario_regression", "run_operational_contracts", "run_packaging_contracts",
    "run_workflow_governance_contracts", "run_doc_checker_contracts",
    "run_cargo_tooling_contracts", "run_site_contract_tests",
    "run_codex_agent_config_validation", "run_compile_metrics_contract_tests",
    "run_rust_baseline",
)
PLANNER_RESOURCES = (
    "needs_python", "needs_markdown", "needs_rust_toolchain", "needs_node",
    "needs_system_deps", "needs_trunk", "needs_wasm_target",
)


def fail(message: str) -> None:
    raise SystemExit(f"required-gate-preview: {message}")


def validate_context(env: dict[str, str]) -> dict[str, str]:
    required = {
        "PREVIEW_REPOSITORY", "PREVIEW_EVENT_NAME", "PREVIEW_REF", "PREVIEW_WORKFLOW_SHA", "PREVIEW_RUN_ATTEMPT",
        "PREVIEW_PR_NUMBER", "PREVIEW_TASK_UID", "PREVIEW_BASE_SHA", "PREVIEW_HEAD_SHA",
        "PREVIEW_TESTED_TREE_SHA", "PREVIEW_SCENARIO",
    }
    missing = sorted(required - env.keys())
    if missing:
        fail("missing dispatch context: " + ", ".join(missing))
    if env["PREVIEW_REPOSITORY"] != REPOSITORY:
        fail("only the canonical repository is accepted")
    if env["PREVIEW_EVENT_NAME"] != "workflow_dispatch" or env["PREVIEW_REF"] != "refs/heads/main":
        fail("preview must run from workflow_dispatch on refs/heads/main")
    if env["PREVIEW_RUN_ATTEMPT"] != "1":
        fail("preview run attempts must be fresh; dispatch a new run instead of rerunning")
    if not re.fullmatch(r"[1-9][0-9]{0,8}", env["PREVIEW_PR_NUMBER"]):
        fail("PR number must be a positive decimal integer")
    if not re.fullmatch(r"task_[0-9a-f]{32}", env["PREVIEW_TASK_UID"]):
        fail("task UID must be a canonical task_<32 lowercase hex> value")
    for name in ("PREVIEW_WORKFLOW_SHA", "PREVIEW_BASE_SHA", "PREVIEW_HEAD_SHA", "PREVIEW_TESTED_TREE_SHA"):
        if not SHA_RE.fullmatch(env[name]):
            fail(f"{name} must be a full lowercase commit/tree SHA")
    if env["PREVIEW_WORKFLOW_SHA"] != env["PREVIEW_BASE_SHA"]:
        fail("trusted workflow/planner SHA must equal the exact PR base SHA")
    if env["PREVIEW_SCENARIO"] not in SCENARIOS:
        fail("unsupported scenario")
    return {key: env[key] for key in sorted(required)}


def git(root: Path, *args: str, capture: bool = True) -> str:
    result = subprocess.run(["git", "-C", str(root), *args], text=True,
                            stdout=subprocess.PIPE if capture else None,
                            stderr=subprocess.PIPE if capture else None)
    if result.returncode:
        detail = (result.stderr or "").strip() if capture else ""
        fail(f"git {' '.join(args)} failed ({result.returncode}) {detail}".strip())
    return (result.stdout or "").strip() if capture else ""


def validate_pr_payload(payload: dict, context: dict[str, str]) -> dict:
    base = payload.get("base") or {}
    head = payload.get("head") or {}
    body = payload.get("body") or ""
    exact_uids = TASK_UID_RE.findall(body)
    if payload.get("state") != "open":
        fail("candidate PR is not open")
    if (base.get("repo") or {}).get("full_name") != REPOSITORY or base.get("ref") != "main":
        fail("candidate PR base must be canonical eng-cc/oasis7 main")
    if (head.get("repo") or {}).get("full_name") != REPOSITORY:
        fail("candidate PR head must be in the canonical repository")
    if base.get("sha") != context["PREVIEW_BASE_SHA"] or head.get("sha") != context["PREVIEW_HEAD_SHA"]:
        fail("live PR base/head do not match the supplied frozen identity")
    if exact_uids != [context["PREVIEW_TASK_UID"]]:
        fail("PR body must contain exactly the supplied Task UID once")
    return payload


def read_public_pr(context: dict[str, str]) -> dict:
    url = f"https://api.github.com/repos/{REPOSITORY}/pulls/{context['PREVIEW_PR_NUMBER']}"
    request = urllib.request.Request(url, headers={"Accept": "application/vnd.github+json", "User-Agent": "oasis7-required-preview"})
    try:
        with urllib.request.urlopen(request, timeout=20) as response:
            payload = json.load(response)
    except Exception as exc:
        fail(f"anonymous public PR readback failed: {exc}")
    return validate_pr_payload(payload, context)


def validate_merge_tree(root: Path, base: str, head: str, merge: str, expected_tree: str) -> list[str]:
    parents = git(root, "rev-list", "--parents", "-n", "1", merge).split()
    if parents != [merge, base, head]:
        fail("PR merge ref must have exactly the supplied base and head parents")
    tree = git(root, "rev-parse", f"{merge}^{{tree}}")
    if tree != expected_tree:
        fail("fetched PR merge tree does not match tested_tree_sha")
    changed = git(root, "diff", "--name-only", base, merge).splitlines()
    if changed != [EFFECTIVE_CONFIG]:
        fail("activation preview accepts only the effective required-scope config change; got: " + repr(changed))
    return changed


def validate_candidate_config(root: Path, merge: str) -> None:
    fixture_bytes = (root / FIXTURE).read_bytes()
    if hashlib.sha256(fixture_bytes).hexdigest() != EXPECTED_FIXTURE_SHA256:
        fail("trusted versioned fixture digest changed; update must be independently reviewed")
    candidate_bytes = subprocess.check_output(["git", "-C", str(root), "show", f"{merge}:{EFFECTIVE_CONFIG}"])
    if candidate_bytes != fixture_bytes:
        fail("candidate effective config is not byte-identical to the trusted versioned fixture")


def fetch_and_verify_tree(root: Path, context: dict[str, str]) -> tuple[str, Path]:
    pr = context["PREVIEW_PR_NUMBER"]
    # Fetch remote refs, never values chosen as arbitrary refspecs by the caller.
    git(root, "fetch", "--no-tags", "origin",
        "+refs/heads/main:refs/remotes/preview/main",
        f"+refs/pull/{pr}/head:refs/remotes/preview/pr-{pr}-head",
        f"+refs/pull/{pr}/merge:refs/remotes/preview/pr-{pr}-merge")
    base = git(root, "rev-parse", "refs/remotes/preview/main^{commit}")
    head = git(root, "rev-parse", f"refs/remotes/preview/pr-{pr}-head^{{commit}}")
    merge = git(root, "rev-parse", f"refs/remotes/preview/pr-{pr}-merge^{{commit}}")
    if base != context["PREVIEW_BASE_SHA"] or head != context["PREVIEW_HEAD_SHA"]:
        fail("fetched public base/head refs do not match supplied frozen identity")
    validate_merge_tree(root, base, head, merge, context["PREVIEW_TESTED_TREE_SHA"])
    validate_candidate_config(root, merge)
    target = Path(tempfile.mkdtemp(prefix="oasis7-required-preview-tree-"))
    target.rmdir()
    git(root, "worktree", "add", "--detach", str(target), merge)
    return merge, target


def parse_plan(output: str) -> dict[str, str]:
    fields: dict[str, str] = {}
    for line in output.splitlines():
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        if key in fields:
            fail(f"planner printed duplicate field {key}")
        fields[key] = value
    return fields


def plan_scenario(root: Path, scenario: str) -> dict[str, str]:
    selected = SCENARIOS[scenario]
    command = [sys.executable, str(root / "scripts/plan-rust-required-scope.py"),
               "--event-name", "pull_request", "--config", str(root / FIXTURE),
               "--changed-path", selected["path"]]
    print("planner_command=" + json.dumps(command), flush=True)
    result = subprocess.run(command, cwd=root, text=True, capture_output=True)
    if result.returncode:
        fail("trusted planner failed: " + (result.stderr.strip() or str(result.returncode)))
    fields = parse_plan(result.stdout)
    expected_capability = selected["capability"]
    expected_selection = expected_capability or "required_gate_baseline"
    expected_scope = "targeted" if expected_capability else "minimal"
    if fields.get("scope") != expected_scope or fields.get("selected_capabilities") != expected_selection:
        fail("trusted planner selection does not match the fixed scenario contract")
    if fields.get("execution_contract") != "required-domain-split/v1":
        fail("trusted planner did not load the versioned execution contract")
    for field in PLANNER_SELECTORS:
        selected_here = bool(expected_capability and field == "run_" + expected_capability)
        selected_here |= expected_capability == "workflow_governance" and field == "run_workflow_governance_contracts"
        expected = "true" if selected_here else "false"
        # The required baseline is unconditional; all other capability selectors remain off.
        if field == "run_required_gate_baseline":
            continue
        if fields.get(field) != expected:
            fail(f"planner selector {field}={fields.get(field)!r}, expected {expected!r}")
    for field in PLANNER_RESOURCES:
        expected = "true" if field in ("needs_python", "needs_markdown") else "false"
        if fields.get(field) != expected:
            fail(f"planner resource {field}={fields.get(field)!r}, expected {expected!r}")
    if fields.get("run_required_gate_baseline") != "true":
        fail("required-gate baseline must remain selected")
    return fields


def dispatcher_environment(fields: dict[str, str], context: dict[str, str], merge: str) -> dict[str, str]:
    env = os.environ.copy()
    for protected_name in (
        "GH_TOKEN", "GITHUB_TOKEN", "ACTIONS_RUNTIME_TOKEN", "ACTIONS_ID_TOKEN_REQUEST_URL",
        "ACTIONS_ID_TOKEN_REQUEST_TOKEN", "GITHUB_OUTPUT", "GITHUB_ENV", "GITHUB_PATH",
        "GITHUB_STATE", "GITHUB_STEP_SUMMARY",
    ):
        env.pop(protected_name, None)
    env.update({
        "CI": "true", "GITHUB_ACTIONS": "true",
        "OASIS7_PRODUCT_DOC_BASE": context["PREVIEW_BASE_SHA"],
        "OASIS7_PRODUCT_DOC_HEAD": merge,
        # Manual-only selectors are never inferred from synthetic paths.
        "OASIS7_CI_RUN_HOSTED_ACCOUNT_SMOKE": "false",
        "OASIS7_CI_RUN_PROVIDER_LIVE_GATE": "false",
    })
    python_bin = env.get("PREVIEW_PYTHON")
    if not python_bin:
        fail("preview virtual-environment Python is required for selected child tests")
    env["PATH"] = str(Path(python_bin).resolve().parent) + os.pathsep + env.get("PATH", "")
    for key, value in fields.items():
        if key.startswith("run_"):
            env["OASIS7_CI_" + key.upper()] = value
        elif key.startswith("needs_"):
            env["OASIS7_CI_" + key.upper()] = value
        elif key == "execution_contract":
            env["OASIS7_CI_EXECUTION_CONTRACT"] = value
    return env


def write_workflow_outputs(path: Path, context: dict[str, str], merge: str, fields: dict[str, str]) -> None:
    values = {
        "scenario": context["PREVIEW_SCENARIO"],
        "base_sha": context["PREVIEW_BASE_SHA"],
        "head_sha": context["PREVIEW_HEAD_SHA"],
        "merge_commit": merge,
        "tested_tree_sha": context["PREVIEW_TESTED_TREE_SHA"],
        "run_packaging_contracts": fields["run_packaging_contracts"],
        "run_operational_contracts": fields["run_operational_contracts"],
    }
    if not re.fullmatch(r"[0-9a-f]{40}", merge):
        fail("validated merge commit is not a full SHA")
    if any(value not in ("true", "false") for key, value in values.items() if key.startswith("run_")):
        fail("selected-child outputs must be explicit booleans")
    with path.open("a", encoding="utf-8") as handle:
        for key, value in values.items():
            if "\n" in value or "\r" in value:
                fail(f"unsafe workflow output value for {key}")
            handle.write(f"{key}={value}\n")


def run_child(root: Path, target: Path, fields: dict[str, str], context: dict[str, str], merge: str) -> int:
    env = dispatcher_environment(fields, context, merge)
    command = [str(root / "scripts/ci-tests.sh"), "required", "--repo-root", str(target)]
    print("dispatcher_command=" + json.dumps(command), flush=True)
    start = time.monotonic()
    process = subprocess.Popen(command, cwd=root, env=env, text=True, stdout=subprocess.PIPE,
                               stderr=subprocess.STDOUT, bufsize=1)
    assert process.stdout is not None
    for line in process.stdout:
        print("child> " + line.rstrip("\n"), flush=True)
    code = process.wait()
    elapsed = time.monotonic() - start
    print(f"selected_child_exit={code}", flush=True)
    print(f"selected_child_elapsed_seconds={elapsed:.3f}", flush=True)
    return code


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--trusted-root", required=True, type=Path)
    args = parser.parse_args()
    root = args.trusted_root.resolve()
    context = validate_context(dict(os.environ))
    actual_workflow_sha = git(root, "rev-parse", "HEAD")
    if actual_workflow_sha != context["PREVIEW_WORKFLOW_SHA"]:
        fail("checked-out trusted workflow commit does not match the dispatch SHA")
    print("preview_mode=non_required_diagnostic_only", flush=True)
    print("workflow_sha=" + actual_workflow_sha, flush=True)
    print("identity=" + json.dumps(context, sort_keys=True), flush=True)
    read_public_pr(context)
    merge = ""
    target: Path | None = None
    try:
        merge, target = fetch_and_verify_tree(root, context)
        print("merge_commit=" + merge, flush=True)
        print("tested_tree_sha=" + context["PREVIEW_TESTED_TREE_SHA"], flush=True)
        print("actual_tree_diff=scripts/ci-required-scope.v2.json (config-only)", flush=True)
        print("trusted_fixture_sha256=" + EXPECTED_FIXTURE_SHA256, flush=True)
        fields = plan_scenario(root, context["PREVIEW_SCENARIO"])
        output_path = os.environ.get("GITHUB_OUTPUT")
        if output_path:
            write_workflow_outputs(Path(output_path), context, merge, fields)
        print("synthetic_changed_path=" + SCENARIOS[context["PREVIEW_SCENARIO"]]["path"], flush=True)
        if context["PREVIEW_SCENARIO"] == "ordinary_document":
            print("changed_range_document_validation=not_exercised_by_config_only_tree; synthetic path selects planner only", flush=True)
        print("planned_selectors=" + json.dumps({key: fields[key] for key in PLANNER_SELECTORS if key in fields}, sort_keys=True), flush=True)
        print("planned_resources=" + json.dumps({key: fields[key] for key in PLANNER_RESOURCES if key in fields}, sort_keys=True), flush=True)
        print("readiness_receipt=not-created", flush=True)
        print("required_gate_authority=unchanged", flush=True)
        return run_child(root, target, fields, context, merge)
    finally:
        if target is not None:
            subprocess.run(["git", "-C", str(root), "worktree", "remove", "--force", str(target)],
                           check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


if __name__ == "__main__":
    raise SystemExit(main())
