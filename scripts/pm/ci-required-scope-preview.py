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
import shutil
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
EXPECTED_LEGACY_CONFIG_SHA256 = "d656841b3c9fcf66fcd5ea1c37b43d9628e61d13ea48be8bda54e71511d505b4"
SCENARIOS = {
    "ordinary_document": {
        "path": "doc/product/world-rules-core-gameplay/first-session-and-continuation.prd.md",
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
        "PREVIEW_RUN_ID", "PREVIEW_PR_NUMBER", "PREVIEW_TASK_UID", "PREVIEW_BASE_SHA", "PREVIEW_HEAD_SHA",
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
    if not re.fullmatch(r"[1-9][0-9]{0,19}", env["PREVIEW_RUN_ID"]):
        fail("preview run ID must be a positive decimal integer")
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


def plan_scenario(root: Path, scenario: str, mode: str = "versioned") -> dict[str, str]:
    selected = SCENARIOS[scenario]
    if mode not in ("legacy", "versioned"):
        fail("planner mode must be legacy or versioned")
    if mode == "legacy" and scenario != "ordinary_document":
        fail("legacy comparison is defined only for the ordinary-document scenario")
    config_path = EFFECTIVE_CONFIG if mode == "legacy" else FIXTURE
    command = [sys.executable, str(root / "scripts/plan-rust-required-scope.py"),
               "--event-name", "pull_request", "--config", str(root / config_path),
               "--changed-path", selected["path"]]
    print(f"planner_mode={mode}", flush=True)
    print("planner_command=" + json.dumps(command), flush=True)
    result = subprocess.run(command, cwd=root, text=True, capture_output=True)
    if result.returncode:
        fail("trusted planner failed: " + (result.stderr.strip() or str(result.returncode)))
    fields = parse_plan(result.stdout)
    expected_digest = EXPECTED_LEGACY_CONFIG_SHA256 if mode == "legacy" else EXPECTED_FIXTURE_SHA256
    if fields.get("planner_config_sha256") != "sha256:" + expected_digest:
        fail(f"{mode} planner config digest does not match the trusted pinned config")
    if mode == "legacy":
        if fields.get("execution_contract") is not None:
            fail("legacy planner unexpectedly emitted a versioned execution contract")
        if fields.get("scope") != "minimal" or fields.get("selected_capabilities") != "required_gate_baseline":
            fail("legacy ordinary-document scenario must select only the required baseline")
        for field, expected in {
            "run_required_gate_baseline": "true",
            "run_operational_contracts": "false",
            "run_rust_baseline": "false",
            "needs_rust_toolchain": "false",
            "needs_node": "false",
            "needs_system_deps": "false",
            "needs_wasm_target": "false",
            "needs_trunk": "false",
        }.items():
            if fields.get(field) != expected:
                fail(f"legacy planner selector {field}={fields.get(field)!r}, expected {expected!r}")
        for field, value in fields.items():
            if field.startswith("run_") and field != "run_required_gate_baseline" and value != "false":
                fail(f"legacy ordinary-document planner unexpectedly enabled {field}={value!r}")
        for field in (
            "run_workflow_governance_contracts", "run_packaging_contracts",
            "run_doc_checker_contracts", "run_cargo_tooling_contracts",
            "needs_python", "needs_markdown",
        ):
            if field in fields:
                fail(f"legacy planner unexpectedly emitted versioned field {field}")
        return fields
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


def dispatcher_environment(
    fields: dict[str, str], context: dict[str, str], merge: str, product_head: str | None = None
) -> dict[str, str]:
    env = os.environ.copy()
    for protected_name in (
        "GH_TOKEN", "GITHUB_TOKEN", "ACTIONS_RUNTIME_TOKEN", "ACTIONS_ID_TOKEN_REQUEST_URL",
        "ACTIONS_ID_TOKEN_REQUEST_TOKEN", "GITHUB_OUTPUT", "GITHUB_ENV", "GITHUB_PATH",
        "GITHUB_STATE", "GITHUB_STEP_SUMMARY",
    ):
        env.pop(protected_name, None)
    for name in tuple(env):
        if name.startswith(("OASIS7_CI_RUN_", "OASIS7_CI_NEEDS_")) or name == "OASIS7_CI_EXECUTION_CONTRACT":
            env.pop(name, None)
    env.update({
        "CI": "true", "GITHUB_ACTIONS": "true",
        "OASIS7_PRODUCT_DOC_BASE": context["PREVIEW_BASE_SHA"],
        "OASIS7_PRODUCT_DOC_HEAD": product_head or merge,
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


def write_workflow_outputs(
    path: Path,
    context: dict[str, str],
    merge: str,
    fields: dict[str, str],
    extra: dict[str, str] | None = None,
) -> None:
    values = {
        "scenario": context["PREVIEW_SCENARIO"],
        "preview_run_id": context["PREVIEW_RUN_ID"],
        "base_sha": context["PREVIEW_BASE_SHA"],
        "head_sha": context["PREVIEW_HEAD_SHA"],
        "merge_commit": merge,
        "tested_tree_sha": context["PREVIEW_TESTED_TREE_SHA"],
        "run_packaging_contracts": fields["run_packaging_contracts"],
        "run_operational_contracts": fields["run_operational_contracts"],
    }
    if extra:
        if set(values).intersection(extra):
            fail("preview output extension duplicates a base identity or selector field")
        if any(not re.fullmatch(r"[a-z][a-z0-9_]*", key) for key in extra):
            fail("preview output extension contains an invalid key")
        values.update(extra)
    if not re.fullmatch(r"[0-9a-f]{40}", merge):
        fail("validated merge commit is not a full SHA")
    if any(value not in ("true", "false") for key, value in values.items() if key.startswith("run_")):
        fail("selected-child outputs must be explicit booleans")
    with path.open("a", encoding="utf-8") as handle:
        for key, value in values.items():
            if "\n" in value or "\r" in value:
                fail(f"unsafe workflow output value for {key}")
            handle.write(f"{key}={value}\n")


def run_child(
    root: Path,
    target: Path,
    fields: dict[str, str],
    context: dict[str, str],
    merge: str,
    *,
    product_head: str | None = None,
    pair_leg: str | None = None,
) -> tuple[int, float]:
    env = dispatcher_environment(fields, context, merge, product_head)
    command = [str(root / "scripts/ci-tests.sh"), "required", "--repo-root", str(target)]
    if pair_leg:
        print(f"pair_leg={pair_leg}", flush=True)
        print("pair_execution_contract=" + fields.get("execution_contract", "legacy"), flush=True)
        print("pair_planner_config_sha256=" + fields["planner_config_sha256"], flush=True)
        print("pair_selected_capabilities=" + fields["selected_capabilities"], flush=True)
        print("pair_product_doc_base=" + context["PREVIEW_BASE_SHA"], flush=True)
        print("pair_product_doc_head=" + (product_head or merge), flush=True)
        selected_env = {
            key: value for key, value in env.items()
            if key.startswith(("OASIS7_CI_RUN_", "OASIS7_CI_NEEDS_"))
            or key in ("OASIS7_CI_EXECUTION_CONTRACT", "OASIS7_PRODUCT_DOC_BASE", "OASIS7_PRODUCT_DOC_HEAD")
        }
        print("pair_dispatcher_environment=" + json.dumps(selected_env, sort_keys=True), flush=True)
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
    if pair_leg:
        print(f"pair_result={'success' if code == 0 else 'failure'}", flush=True)
    return code, elapsed


def run_document_timing_pair(
    root: Path,
    overlay: Path,
    fields_by_mode: dict[str, dict[str, str]],
    context: dict[str, str],
    merge: str,
    overlay_commit: str,
    pair_order: tuple[str, str],
) -> dict[str, tuple[int, float]]:
    if set(pair_order) != {"legacy", "versioned"} or len(pair_order) != 2:
        fail("ordinary-document pair must contain each planner mode exactly once")
    results: dict[str, tuple[int, float]] = {}
    for mode in pair_order:
        results[mode] = run_child(
            root,
            overlay,
            fields_by_mode[mode],
            context,
            merge,
            product_head=overlay_commit,
            pair_leg=mode,
        )
    return results


def create_document_overlay(root: Path, merge: str) -> tuple[Path, str, str]:
    """Create a disposable one-document commit whose parent is the exact candidate merge."""
    parent = Path(tempfile.mkdtemp(prefix="oasis7-required-preview-document-overlay-"))
    overlay = parent / "candidate"
    path = SCENARIOS["ordinary_document"]["path"]
    try:
        git(root, "worktree", "add", "--detach", str(overlay), merge)
        if git(overlay, "status", "--porcelain"):
            fail("fresh document overlay worktree is not clean")
        document = overlay / path
        if not document.is_file():
            fail("trusted ordinary-document preview path is missing from candidate tree")
        content = document.read_bytes()
        separator = b"" if not content or content.endswith(b"\n") else b"\n"
        document.write_bytes(
            content + separator + b"Preview-only changed-range probe; this sentence exists in the disposable overlay only.\n"
        )
        git(overlay, "add", "--", path)
        commit_env = os.environ.copy()
        commit_env.update({
            "GIT_AUTHOR_NAME": "Oasis7 Preview Fixture",
            "GIT_AUTHOR_EMAIL": "preview-fixture@example.invalid",
            "GIT_COMMITTER_NAME": "Oasis7 Preview Fixture",
            "GIT_COMMITTER_EMAIL": "preview-fixture@example.invalid",
        })
        committed = subprocess.run(
            ["git", "-C", str(overlay), "commit", "-m", "Disposable product-document range fixture"],
            text=True, capture_output=True, env=commit_env,
        )
        if committed.returncode:
            fail("cannot commit disposable document overlay: " + (committed.stderr or "").strip())
        commit = git(overlay, "rev-parse", "HEAD^{commit}")
        tree = git(overlay, "rev-parse", "HEAD^{tree}")
        if git(overlay, "rev-parse", "HEAD^1") != merge:
            fail("disposable document overlay must be a direct child of the exact candidate merge")
        changed = git(overlay, "diff", "--name-only", merge, commit, "--", "doc/product").splitlines()
        if changed != [path]:
            fail("disposable overlay must change exactly the fixed ordinary product document")
        if git(overlay, "status", "--porcelain"):
            fail("disposable document overlay did not commit its only change")
        return overlay, commit, tree
    except BaseException:
        subprocess.run(
            ["git", "-C", str(root), "worktree", "remove", "--force", str(overlay)],
            check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        shutil.rmtree(parent, ignore_errors=True)
        raise


def remove_document_overlay(root: Path, overlay: Path) -> None:
    subprocess.run(
        ["git", "-C", str(root), "worktree", "remove", "--force", str(overlay)],
        check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    shutil.rmtree(overlay.parent, ignore_errors=True)


def verify_changed_range_document(root: Path, overlay: Path, base: str, head: str) -> tuple[int, float]:
    command = [
        sys.executable, str(root / "scripts/product-doc-content-check.py"),
        "--repo-root", str(overlay), "--base", base, "--head", head,
    ]
    print("changed_range_document_command=" + json.dumps(command), flush=True)
    start = time.monotonic()
    result = subprocess.run(command, cwd=overlay, text=True, capture_output=True)
    elapsed = time.monotonic() - start
    if result.stdout:
        for line in result.stdout.splitlines():
            print("document-check> " + line, flush=True)
    if result.stderr:
        for line in result.stderr.splitlines():
            print("document-check! " + line, flush=True)
    print(f"changed_range_document_check_elapsed_seconds={elapsed:.3f}", flush=True)
    if result.returncode:
        fail("disposable changed-range product-document check failed")
    expected = "product-doc-content: OK (checked 1 changed/new documents)"
    if expected not in result.stdout:
        fail("disposable changed-range product-document check did not select exactly one document")
    return 1, elapsed


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
        print("synthetic_changed_path=" + SCENARIOS[context["PREVIEW_SCENARIO"]]["path"], flush=True)
        print("planned_selectors=" + json.dumps({key: fields[key] for key in PLANNER_SELECTORS if key in fields}, sort_keys=True), flush=True)
        print("planned_resources=" + json.dumps({key: fields[key] for key in PLANNER_RESOURCES if key in fields}, sort_keys=True), flush=True)
        print("readiness_receipt=not-created", flush=True)
        print("required_gate_authority=unchanged", flush=True)
        if context["PREVIEW_SCENARIO"] != "ordinary_document":
            if output_path:
                write_workflow_outputs(Path(output_path), context, merge, fields)
            code, _elapsed = run_child(root, target, fields, context, merge)
            return code

        legacy_fields = plan_scenario(root, "ordinary_document", mode="legacy")
        overlay: Path | None = None
        try:
            overlay, overlay_commit, overlay_tree = create_document_overlay(root, merge)
            print("candidate_merge_commit=" + merge, flush=True)
            print("document_overlay_parent_commit=" + merge, flush=True)
            print("document_overlay_commit=" + overlay_commit, flush=True)
            print("document_overlay_tree_sha=" + overlay_tree, flush=True)
            print("document_overlay_path=" + SCENARIOS["ordinary_document"]["path"], flush=True)
            print("document_overlay_scope=temporary child commit of candidate T; candidate T is unchanged", flush=True)
            _changed_count, check_elapsed = verify_changed_range_document(
                root, overlay, context["PREVIEW_BASE_SHA"], overlay_commit
            )

            pair_order = (
                ("legacy", "versioned")
                if int(context["PREVIEW_RUN_ID"]) % 2
                else ("versioned", "legacy")
            )
            print("pair_order=" + ";".join(pair_order), flush=True)
            pair_fields = {"legacy": legacy_fields, "versioned": fields}
            pair_results = run_document_timing_pair(
                root, overlay, pair_fields, context, merge, overlay_commit, pair_order
            )

            legacy_code, legacy_elapsed = pair_results["legacy"]
            versioned_code, versioned_elapsed = pair_results["versioned"]
            extras = {
                "pair_order": ";".join(pair_order),
                "legacy_execution_contract": "legacy",
                "versioned_execution_contract": fields["execution_contract"],
                "legacy_config_sha256": legacy_fields["planner_config_sha256"],
                "versioned_config_sha256": fields["planner_config_sha256"],
                "legacy_selected_capabilities": legacy_fields["selected_capabilities"],
                "versioned_selected_capabilities": fields["selected_capabilities"],
                "document_overlay_parent_commit": merge,
                "document_overlay_commit": overlay_commit,
                "document_overlay_tree_sha": overlay_tree,
                "document_changed_path": SCENARIOS["ordinary_document"]["path"],
                "document_changed_count": "1",
                "document_range_check_result": "success",
                "document_range_check_elapsed_seconds": f"{check_elapsed:.3f}",
                "legacy_run_result": "success" if legacy_code == 0 else "failure",
                "legacy_elapsed_seconds": f"{legacy_elapsed:.3f}",
                "versioned_run_result": "success" if versioned_code == 0 else "failure",
                "versioned_elapsed_seconds": f"{versioned_elapsed:.3f}",
            }
            if output_path:
                write_workflow_outputs(Path(output_path), context, merge, fields, extras)
            return 0 if legacy_code == 0 and versioned_code == 0 else 1
        finally:
            if overlay is not None:
                remove_document_overlay(root, overlay)
    finally:
        if target is not None:
            subprocess.run(["git", "-C", str(root), "worktree", "remove", "--force", str(target)],
                           check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


if __name__ == "__main__":
    raise SystemExit(main())
