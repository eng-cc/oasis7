#!/usr/bin/env python3
"""Build the trusted, complete required-gate test-unit inventory for C4.

This module is loaded from the fixed workflow/planner checkout (W). The target
repository and target commit (M/T) are data inputs only. It never discovers
tests from changed candidate paths; those paths may only select capability
names already emitted by the trusted planner. A complete source-tree closure
does not establish a reusable execution environment: every unit remains
explicitly ineligible for reuse until that environment is independently bound.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import importlib.util
import json
import os
import platform
import re
import subprocess
import sys
from importlib.metadata import PackageNotFoundError, version as package_version
from pathlib import Path
from typing import Any


EXECUTION_CONTRACT = "required-domain-split/v1"
INVENTORY_AUTHORITY_SCHEMA = "oasis7-planner-inventory-authority/v1"
INVENTORY_SCHEMA = "oasis7-trusted-planner-inventory/v1"

BASELINE_OBLIGATIONS = (
    "product-doc-changed-range",
    "product-doc-full-corpus",
    "lint-skills",
    "windows-paths",
    "script-executable-bits",
    "workflow-impact-projection-consumer",
    "cargo-package-scope-and-profile-completion",
    "unified-world-terminology",
    "rust-file-size-regression-and-check",
    "required-domain-selector-validation",
)
BASELINE_CHECKER_PATHS = (
    "scripts/ci-tests.sh",
    "scripts/doc-governance-check.sh",
    "scripts/product-doc-governance-check.py",
    "scripts/product-doc-content-check.py",
    "scripts/lint-skills.sh",
    "scripts/check-windows-paths.sh",
    "scripts/check-script-executable-bits.sh",
    "scripts/pm/workflow-impact-projection.py",
    "scripts/pm/check-cargo-package-scope",
    "scripts/pm/cargo_package_profile_driver.py",
    "scripts/unified-world-code-terminology-scan.test.sh",
    "scripts/check-rust-file-size.test.sh",
    "scripts/check-rust-file-size.sh",
)
PLANNER_SOURCE_PATHS = (
    "scripts/plan-rust-required-scope.py",
    "scripts/ci-required-scope.v2.json",
    "scripts/ci-required-capability-test-inventory.tsv",
    ".github/workflows/rust.yml",
    "scripts/product_doc_markdown.py",
    "scripts/doc-governance-requirements.txt",
    "scripts/pm/ci_required_artifact_v2.py",
)
ROOT_CARGO_INPUTS = (
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "rust-toolchain",
    "rustfmt.toml",
    ".cargo/config.toml",
    ".cargo/config",
)
STATIC_INPUT_PATHS: dict[str, tuple[str, ...]] = {
    "site_quality": ("README.md",),
}

CAPABILITY_RUNNERS: dict[str, tuple[str, ...]] = {
    "required_gate_baseline": ("run_required_gate_checks",),
    "oasis7_required": ("run_oasis7_required_tier_tests", "run_oasis7_required_tier_clippy"),
    "consensus": ("run_oasis7_consensus_tests", "run_oasis7_consensus_clippy"),
    "distfs": ("run_oasis7_distfs_tests", "run_oasis7_distfs_clippy"),
    "node": ("run_oasis7_node_tests", "run_oasis7_node_clippy"),
    "net": ("run_oasis7_net_tests", "run_oasis7_net_libp2p_tests",
            "run_oasis7_net_clippy", "run_oasis7_net_libp2p_clippy"),
    "viewer_js_required": ("run_oasis7_viewer_software_safe_feedback_contract_tests",
                           "run_oasis7_viewer_software_safe_build"),
    "viewer_performance_report": ("run_oasis7_viewer_performance_smoke_report_only",),
    "pixel_world_bridge": ("run_pixel_world_bridge_lib_tests", "run_pixel_world_bridge_wasm_check"),
    "launcher_web": ("run_oasis7_client_launcher_web_build",),
    "workspace_support": ("run_oasis7_workspace_support_crate_tests",),
    "scenario_regression": ("run_scenario_regression_tests",),
    "operational_contracts": ("run_operational_contract_tests",),
    "packaging_contracts": ("run_packaging_contract_tests",),
    "workflow_governance": ("run_workflow_governance_contract_tests",),
    "codex_agent_config_validation": ("run_codex_agent_config_validation",),
    "compile_metrics": ("run_compile_metrics_contract_tests",),
    "site_quality": ("run_site_contract_tests",),
    "doc_checker_contracts": ("run_doc_checker_contract_tests",),
    "cargo_tooling_contracts": ("run_cargo_tooling_contract_tests",),
}

SELECTOR_ENV: dict[str, str] = {
    "oasis7_required": "OASIS7_CI_RUN_OASIS7_REQUIRED_TESTS",
    "consensus": "OASIS7_CI_RUN_CONSENSUS_TESTS",
    "distfs": "OASIS7_CI_RUN_DISTFS_TESTS",
    "node": "OASIS7_CI_RUN_OASIS7_NODE_TESTS",
    "net": "OASIS7_CI_RUN_OASIS7_NET_TESTS",
    "viewer_js_required": "OASIS7_CI_RUN_VIEWER_CONTRACT_TESTS",
    "viewer_performance_report": "OASIS7_CI_RUN_VIEWER_PERF_SMOKE",
    "pixel_world_bridge": "OASIS7_CI_RUN_PIXEL_WORLD_BRIDGE_LIB_TESTS",
    "launcher_web": "OASIS7_CI_RUN_LAUNCHER_WEB_BUILD",
    "workspace_support": "OASIS7_CI_RUN_WORKSPACE_SUPPORT_CRATE_TESTS",
    "scenario_regression": "OASIS7_CI_RUN_SCENARIO_REGRESSION",
    "operational_contracts": "OASIS7_CI_RUN_OPERATIONAL_CONTRACTS",
    "packaging_contracts": "OASIS7_CI_RUN_PACKAGING_CONTRACTS",
    "workflow_governance": "OASIS7_CI_RUN_WORKFLOW_GOVERNANCE_CONTRACTS",
    "codex_agent_config_validation": "OASIS7_CI_RUN_CODEX_AGENT_CONFIG_VALIDATION",
    "compile_metrics": "OASIS7_CI_RUN_COMPILE_METRICS_CONTRACT_TESTS",
    "site_quality": "OASIS7_CI_RUN_SITE_CONTRACT_TESTS",
    "doc_checker_contracts": "OASIS7_CI_RUN_DOC_CHECKER_CONTRACTS",
    "cargo_tooling_contracts": "OASIS7_CI_RUN_CARGO_TOOLING_CONTRACTS",
}

RUST_PACKAGES: dict[str, tuple[str, ...]] = {
    "oasis7_required": ("oasis7",),
    "consensus": ("oasis7_consensus",),
    "distfs": ("oasis7_distfs",),
    "node": ("oasis7_node",),
    "net": ("oasis7_net",),
    "pixel_world_bridge": ("pixel_world_bridge",),
    "launcher_web": ("oasis7_client_launcher",),
    "workspace_support": (
        "oasis7_launcher_ui", "oasis7_proto", "oasis7_wasm_abi", "oasis7_wasm_build",
        "oasis7_wasm_router", "oasis7_wasm_sdk", "oasis7_wasm_store",
        "pixel_world_bridge", "oasis7_wasm_executor", "oasis7_client_launcher",
    ),
    "scenario_regression": ("oasis7",),
}

STATIC_MEMBER_ROOTS: dict[str, tuple[str, ...]] = {
    "viewer_js_required": ("scripts", "crates/oasis7_viewer"),
    "viewer_performance_report": ("scripts", "crates/oasis7_viewer"),
    "operational_contracts": ("scripts", ".github", "doc", "crates"),
    "packaging_contracts": ("scripts", ".github", "site", "crates/oasis7_client_launcher"),
    "workflow_governance": ("scripts", ".agents", ".codex", ".github", "doc/engineering"),
    "codex_agent_config_validation": ("scripts", ".agents", ".codex"),
    "compile_metrics": ("scripts", ".github/workflows", ".cargo"),
    "site_quality": ("scripts", "site", "doc"),
    "doc_checker_contracts": ("scripts", ".agents", "doc/engineering", "doc/product"),
    "cargo_tooling_contracts": ("scripts", ".cargo", "crates/oasis7_client_launcher"),
}

RUST_COMMANDS: dict[str, tuple[str, ...]] = {
    "oasis7_required": (
        "cargo test -p oasis7 --tests --features test_tier_required",
        "cargo clippy -p oasis7 --tests --features test_tier_required -- -D warnings -D clippy::correctness -D clippy::suspicious",
        "cargo test -p oasis7 --bin oasis7_chain_runtime --features test_tier_required execution_bridge_real_tests::real_execution_bridge::tests",
    ),
    "consensus": (
        "cargo test -p oasis7_consensus --lib",
        "cargo clippy -p oasis7_consensus --lib -- -D warnings -D clippy::correctness -D clippy::suspicious",
    ),
    "distfs": (
        "cargo test -p oasis7_distfs --lib",
        "cargo clippy -p oasis7_distfs --lib -- -D warnings -D clippy::correctness -D clippy::suspicious",
    ),
    "node": (
        "cargo test -p oasis7_node --lib",
        "cargo clippy -p oasis7_node --lib -- -D warnings -D clippy::correctness -D clippy::suspicious",
    ),
    "net": (
        "cargo test -p oasis7_net --lib",
        "cargo test -p oasis7_net --features libp2p --lib",
        "cargo clippy -p oasis7_net --lib -- -D warnings -D clippy::correctness -D clippy::suspicious",
        "cargo clippy -p oasis7_net --features libp2p --lib -- -D warnings -D clippy::correctness -D clippy::suspicious",
    ),
    "pixel_world_bridge": (
        "cargo test -p pixel_world_bridge --lib",
        "cargo check -p pixel_world_bridge --target wasm32-unknown-unknown",
    ),
    "launcher_web": ("trunk build --release --dist output/release/web-launcher-dist",),
    "workspace_support": (
        "cargo test -p oasis7_launcher_ui -p oasis7_proto -p oasis7_wasm_abi -p oasis7_wasm_build -p oasis7_wasm_router -p oasis7_wasm_sdk -p oasis7_wasm_store -p pixel_world_bridge --lib",
        "cargo test -p oasis7_wasm_executor --features wasmtime --lib",
        "cargo test -p oasis7_client_launcher --bin oasis7_client_launcher",
    ),
    "scenario_regression": (
        "cargo test -p oasis7 --test oasis7_init_demo --features test_tier_full oasis7_init_demo_runs_",
    ),
}

_UNIT_SCHEMA = "oasis7-required-test-unit/v1"
_POLICY_SCHEMA = "oasis7-required-unit-policy/v1"
_ENV_SCHEMA = "oasis7-required-unit-environment/v1"
_TRUSTED_SOURCE_PRODUCT_ENVIRONMENT_SCHEMA = "oasis7-trusted-source-product-environment/v1"
_TRUSTED_SOURCE_ATTEMPT_SCHEMA = "oasis7-ci-trusted-source-attempt/v1"
_PLAN_INVOCATION_SCHEMA = "oasis7-required-scope-invocation/v1"
_TRUSTED_RUNNER_IMAGE = "ubuntu-24.04"
_TRUSTED_PYTHON_IMPLEMENTATION = "CPython"
_TRUSTED_PYTHON_VERSION = "3.12.3"
_TRUSTED_MARKDOWN_PACKAGES = {"markdown-it-py": "3.0.0", "mdurl": "0.1.2"}
_PLANNER_EVENT_NAMES = {"pull_request", "workflow_dispatch", "push", "schedule"}
_PLANNER_RUN_MODES = {"legacy", "integration_revalidation", "full_escalation"}


class InventoryError(ValueError):
    """The trusted planner or required-unit inventory is incomplete or invalid."""


def _load_module(path: Path, module_name: str):
    spec = importlib.util.spec_from_file_location(module_name, path)
    if spec is None or spec.loader is None:
        raise InventoryError(f"trusted helper is unavailable: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _plan_mapping(plan: str | dict[str, Any]) -> dict[str, str]:
    if isinstance(plan, dict):
        if any(not isinstance(key, str) or not isinstance(value, str) for key, value in plan.items()):
            raise InventoryError("planner output must map strings to strings")
        return dict(plan)
    if not isinstance(plan, str):
        raise InventoryError("trusted planner output is required")
    values: dict[str, str] = {}
    for line in plan.splitlines():
        if not line:
            continue
        key, separator, value = line.partition("=")
        if not separator or not key or key in values:
            raise InventoryError("trusted planner output has malformed or duplicate fields")
        values[key] = value
    return values


def selected_test_units(
    plan: str | dict[str, Any], planner_capabilities: tuple[str, ...] | list[str],
) -> list[str]:
    """Validate the W planner's canonical test-unit projection."""
    values = _plan_mapping(plan)
    if values.get("execution_contract") != EXECUTION_CONTRACT:
        raise InventoryError("required-test inventory needs the versioned trusted planner")
    allowed = set(planner_capabilities)
    selected = values.get("selected_capabilities", "").split(";")
    if (not selected or any(not item for item in selected)
            or selected != sorted(set(selected))):
        raise InventoryError("planner selected_capabilities is incomplete or duplicated")
    if not set(selected).issubset(allowed):
        raise InventoryError("planner selected an unknown capability")
    if selected == ["required_gate_baseline"]:
        selected_capabilities: set[str] = set()
    else:
        selected_capabilities = set(selected)
        selected_capabilities.discard("required_gate_baseline")
    expected = sorted({"required_gate_baseline", *selected_capabilities})
    raw = values.get("required_test_units")
    if raw is None:
        raise InventoryError("planner required_test_units field is missing")
    units = raw.split(";")
    if units != sorted(set(units)) or not units or any(not item for item in units):
        raise InventoryError("planner required_test_units is not canonical")
    if "required_gate_baseline" not in units:
        raise InventoryError("required test units must always include the baseline")
    if units != expected:
        unknown = sorted(set(units) - allowed)
        if unknown:
            raise InventoryError("unknown required test unit: " + ", ".join(unknown))
        raise InventoryError("required test units disagree with selected planner capabilities")
    return units


def required_test_unit_registry(
    planner_capabilities: tuple[str, ...] | list[str],
) -> dict[str, dict[str, Any]]:
    """Return every unit contract the versioned planner can select."""
    if set(planner_capabilities) != set(CAPABILITY_RUNNERS):
        raise InventoryError("required-test inventory registry does not cover planner capabilities")
    registry: dict[str, dict[str, Any]] = {}
    for capability, runners in CAPABILITY_RUNNERS.items():
        commands = list(RUST_COMMANDS.get(capability, ()))
        if not commands:
            commands = [f"required runner function: {runner}" for runner in runners]
        if capability == "required_gate_baseline":
            commands = list(BASELINE_OBLIGATIONS)
        registry[capability] = {
            "runner_functions": list(runners),
            "commands": commands,
            "obligations": list(commands),
            "package_names": list(RUST_PACKAGES.get(capability, ())),
            "input_paths": list(STATIC_INPUT_PATHS.get(capability, ())),
            "member_roots": list(STATIC_MEMBER_ROOTS.get(capability, ())),
            "selector_env": SELECTOR_ENV.get(capability),
        }
    return registry


def package_dependency_roots(
    metadata: dict[str, Any], repo_root: str | Path, package_names: list[str] | tuple[str, ...],
) -> list[str]:
    """Resolve transitive workspace package source roots from Cargo metadata."""
    root = Path(repo_root).resolve()
    packages = metadata.get("packages")
    resolved = metadata.get("resolve")
    if not isinstance(packages, list) or not isinstance(resolved, dict):
        raise InventoryError("Cargo metadata lacks a resolved package graph")
    by_id = {item.get("id"): item for item in packages if isinstance(item, dict)}
    nodes = resolved.get("nodes")
    if not isinstance(nodes, list):
        raise InventoryError("Cargo metadata package graph is incomplete")
    node_by_id = {item.get("id"): item for item in nodes if isinstance(item, dict)}
    roots = [item.get("id") for item in packages
             if isinstance(item, dict) and item.get("name") in package_names
             and isinstance(item.get("manifest_path"), str)]
    if not roots or any(root_id not in node_by_id for root_id in roots):
        raise InventoryError("selected Cargo package is absent from the resolved graph")
    seen: set[str] = set()
    pending = list(roots)
    selected_paths: set[str] = set()
    while pending:
        package_id = pending.pop()
        if package_id in seen:
            continue
        seen.add(package_id)
        package = by_id.get(package_id)
        if not isinstance(package, dict) or not isinstance(package.get("manifest_path"), str):
            raise InventoryError("Cargo metadata dependency has no package manifest")
        manifest = Path(package["manifest_path"]).resolve()
        try:
            relative_manifest = manifest.relative_to(root).as_posix()
        except ValueError:
            # Registry and toolchain dependencies are version-pinned by Cargo.lock;
            # they are not workspace source roots.
            relative_manifest = None
        if relative_manifest is not None:
            if not relative_manifest.endswith("/Cargo.toml"):
                raise InventoryError("Cargo workspace package manifest path is malformed")
            selected_paths.add(relative_manifest.rsplit("/", 1)[0])
        node = node_by_id[package_id]
        dependencies = node.get("deps")
        if not isinstance(dependencies, list):
            raise InventoryError("Cargo metadata dependency edges are malformed")
        for dependency in dependencies:
            if not isinstance(dependency, dict) or not isinstance(dependency.get("pkg"), str):
                raise InventoryError("Cargo metadata dependency edge is malformed")
            dependency_id = dependency["pkg"]
            if dependency_id not in by_id:
                raise InventoryError("Cargo metadata dependency target is missing")
            pending.append(dependency_id)
    return sorted(selected_paths)


def _sha256(path: Path) -> str:
    try:
        return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()
    except OSError as exc:
        raise InventoryError(f"trusted planner input is unreadable: {path}") from exc


def _require_clean_checkout(repo_root: Path, label: str) -> None:
    try:
        status = subprocess.run(
            ["git", "-C", str(repo_root), "status", "--porcelain=v1", "--untracked-files=all"],
            check=True, capture_output=True, text=True,
        ).stdout
    except (OSError, subprocess.CalledProcessError) as exc:
        raise InventoryError(f"{label} checkout status is unavailable") from exc
    if status:
        raise InventoryError(f"{label} checkout contains local changes")


def _trusted_sources(planner_root: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for relative in (
        *PLANNER_SOURCE_PATHS,
        "scripts/ci-tests.sh",
        "scripts/pm/ci_input_scope.py",
        "scripts/pm/ci_required_inventory.py",
    ):
        path = planner_root / relative
        if path.is_symlink() or not path.is_file():
            raise InventoryError(f"trusted W planner source is not a regular file: {relative}")
        values[relative] = _sha256(path)
    return values


def _trusted_source_product_environment(value: Any) -> tuple[dict[str, Any], str]:
    """Validate the source environment only when its attempt and job were authenticated."""
    fields = {"schema", "trusted_source_attempt", "required_gate_job", "environment_contract"}
    if (not isinstance(value, dict) or set(value) != fields
            or value.get("schema") != _TRUSTED_SOURCE_PRODUCT_ENVIRONMENT_SCHEMA):
        raise InventoryError("trusted source product environment fields are incomplete or unsupported")
    attempt = value["trusted_source_attempt"]
    attempt_fields = {
        "schema", "request_key", "workflow_run_id", "run_attempt", "check_app_id",
        "check_run_id", "job_id", "job_name", "plan_artifact_id", "plan_artifact_name",
        "result_artifacts",
    }
    if (not isinstance(attempt, dict) or set(attempt) != attempt_fields
            or attempt.get("schema") != _TRUSTED_SOURCE_ATTEMPT_SCHEMA
            or not isinstance(attempt.get("request_key"), str)
            or not re.fullmatch(r"sha256:[0-9a-f]{64}", attempt["request_key"])):
        raise InventoryError("trusted source product environment attempt binding is malformed")
    integer_fields = (
        "workflow_run_id", "run_attempt", "check_app_id", "check_run_id", "job_id", "plan_artifact_id",
    )
    if any(type(attempt.get(field)) is not int or attempt[field] < 1 for field in integer_fields):
        raise InventoryError("trusted source product environment attempt IDs are invalid")
    run_id, attempt_no = attempt["workflow_run_id"], attempt["run_attempt"]
    if (attempt["job_name"] != "required-gate"
            or attempt["plan_artifact_name"] != f"oasis7-required-plan-v2-{run_id}-a{attempt_no}"):
        raise InventoryError("trusted source product environment attempt locator is invalid")
    result_rows = attempt["result_artifacts"]
    if not isinstance(result_rows, list) or not result_rows:
        raise InventoryError("trusted source product environment has no source result artifacts")
    seen_units: set[str] = set()
    seen_artifact_ids: set[int] = {attempt["plan_artifact_id"]}
    normalized_rows: list[dict[str, Any]] = []
    for row in result_rows:
        if (not isinstance(row, dict) or set(row) != {"unit_id", "artifact_id", "name"}
                or not isinstance(row.get("unit_id"), str) or not row["unit_id"]
                or type(row.get("artifact_id")) is not int or row["artifact_id"] < 1
                or not isinstance(row.get("name"), str)):
            raise InventoryError("trusted source product environment result locator is malformed")
        expected_name = "oasis7-required-result-v2-{}-a{}-{}".format(
            run_id, attempt_no, hashlib.sha256(row["unit_id"].encode("utf-8")).hexdigest(),
        )
        if (row["name"] != expected_name or row["unit_id"] in seen_units
                or row["artifact_id"] in seen_artifact_ids):
            raise InventoryError("trusted source product environment result locator is duplicate or mismatched")
        seen_units.add(row["unit_id"])
        seen_artifact_ids.add(row["artifact_id"])
        normalized_rows.append(dict(row))
    if normalized_rows != sorted(normalized_rows, key=lambda row: row["unit_id"]):
        raise InventoryError("trusted source product environment result locators are not sorted")

    job = value["required_gate_job"]
    job_fields = {
        "workflow_run_id", "run_attempt", "job_id", "job_name", "check_name",
        "check_app_id", "check_run_id", "head_sha", "status", "conclusion", "labels",
    }
    if (not isinstance(job, dict) or set(job) != job_fields
            or any(type(job.get(field)) is not int for field in (
                "workflow_run_id", "run_attempt", "job_id", "check_app_id", "check_run_id",
            ))
            or any(job.get(field) != attempt.get(source) for field, source in (
                ("workflow_run_id", "workflow_run_id"), ("run_attempt", "run_attempt"),
                ("job_id", "job_id"), ("job_name", "job_name"),
                ("check_app_id", "check_app_id"), ("check_run_id", "check_run_id"),
            ))
            or job.get("check_name") != "required-gate"
            or not isinstance(job.get("head_sha"), str)
            or not re.fullmatch(r"[0-9a-f]{40}", job["head_sha"])
            or job.get("status") != "completed" or job.get("conclusion") != "success"
            or job.get("labels") != ["ubuntu-24.04"]):
        raise InventoryError("trusted source product environment required-gate job is not the pinned live job")

    environment = value["environment_contract"]
    env_fields = {
        "schema", "runner_image", "runner_identity", "runner_image_matches_trusted_W",
        "python_implementation", "python_version", "python_full_version", "python_cache_tag",
        "python_runtime_matches_trusted_W", "runtime_packages", "markdown_runtime_matches_trusted_W",
        "markdown_parser_matches_trusted_W", "markdown_requirements_match_trusted_W",
        "external_inputs", "reuse_eligible",
    }
    if not isinstance(environment, dict) or set(environment) != env_fields:
        raise InventoryError("trusted source product environment contract is incomplete or unsupported")
    runner_fields = {
        "runner_image", "runner_os", "image_os", "image_version", "os_id", "os_version_id",
        "matches_trusted_W",
    }
    runner = environment.get("runner_identity")
    packages = environment.get("runtime_packages")
    if (environment.get("schema") != _ENV_SCHEMA
            or not isinstance(environment.get("runner_image"), str) or not environment["runner_image"]
            or not isinstance(runner, dict) or set(runner) != runner_fields
            or runner.get("runner_image") != environment["runner_image"]
            or not isinstance(packages, dict) or set(packages) != set(_TRUSTED_MARKDOWN_PACKAGES)
            or any(not isinstance(packages.get(name), str) or not packages[name]
                   for name in _TRUSTED_MARKDOWN_PACKAGES)
            or any(type(environment.get(field)) is not bool for field in (
                "runner_image_matches_trusted_W", "python_runtime_matches_trusted_W",
                "markdown_runtime_matches_trusted_W", "markdown_parser_matches_trusted_W",
                "markdown_requirements_match_trusted_W", "reuse_eligible",
            ))
            or type(runner.get("matches_trusted_W")) is not bool
            or environment.get("runner_image_matches_trusted_W") != runner["matches_trusted_W"]):
        raise InventoryError("trusted source product environment contract is malformed")
    for field in ("runner_os", "image_os", "image_version", "os_id", "os_version_id"):
        if not isinstance(runner.get(field), str) or not runner[field]:
            raise InventoryError("trusted source product runner identity is incomplete")
    python_fields = ("python_implementation", "python_version", "python_full_version", "python_cache_tag")
    if any(not isinstance(environment.get(field), str) or not environment[field] for field in python_fields):
        raise InventoryError("trusted source Python identity is incomplete")
    python_matches = (
        environment["python_implementation"] == _TRUSTED_PYTHON_IMPLEMENTATION
        and environment["python_version"] == _TRUSTED_PYTHON_VERSION
        and environment["python_full_version"].startswith(_TRUSTED_PYTHON_VERSION + " ")
        and environment["python_cache_tag"] == "cpython-312"
    )
    package_matches = packages == _TRUSTED_MARKDOWN_PACKAGES
    markdown_runtime_matches = environment["markdown_runtime_matches_trusted_W"]
    if markdown_runtime_matches and not package_matches:
        raise InventoryError("trusted source Markdown runtime flag conflicts with observed packages")
    expected_eligible = (
        environment["runner_image_matches_trusted_W"] and python_matches
        and markdown_runtime_matches and environment["markdown_parser_matches_trusted_W"]
        and environment["markdown_requirements_match_trusted_W"]
    )
    if (environment["python_runtime_matches_trusted_W"] != python_matches
            or environment["reuse_eligible"] != expected_eligible
            or not isinstance(environment.get("external_inputs"), str)
            or not environment["external_inputs"]):
        raise InventoryError("trusted source product environment flags disagree with observed identities")
    canonical_attempt = json.dumps(
        attempt, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False,
    ).encode("utf-8")
    return environment, "sha256:" + hashlib.sha256(canonical_attempt).hexdigest()


def trusted_product_environment_from_plan(
    plan: dict[str, Any], execution_jobs: list[dict[str, Any]], trusted_source_attempt: dict[str, Any],
) -> dict[str, Any]:
    """Bind the product checker environment to authenticated source plan/job/artifacts."""
    if not isinstance(plan, dict) or not isinstance(execution_jobs, list):
        raise InventoryError("trusted source product plan or execution jobs are malformed")
    gates = [job for job in execution_jobs if isinstance(job, dict) and job.get("job_name") == "required-gate"]
    if len(gates) != 1:
        raise InventoryError("trusted source product environment requires one exact required-gate job")
    gate = gates[0]
    if (gate.get("head_sha") != plan.get("workflow_sha")
            or gate.get("workflow_run_id") != plan.get("workflow_run_id")
            or gate.get("run_attempt") != plan.get("run_attempt")
            or gate.get("check_app_id") != plan.get("check_app_id")
            or gate.get("check_run_id") != plan.get("check_run_id")
            or gate.get("job_id") != plan.get("job_id")):
        raise InventoryError("trusted source product required-gate job differs from its plan")
    raw_specs = plan.get("unit_specs")
    product_corpus = plan.get("product_corpus")
    if not isinstance(raw_specs, list) or not isinstance(product_corpus, dict):
        raise InventoryError("trusted source product plan inventory is malformed")
    product_specs = [
        spec for spec in raw_specs
        if isinstance(spec, dict) and isinstance(spec.get("unit_id"), str)
        and spec["unit_id"].startswith("product-")
    ]
    if not product_specs or product_corpus.get("status") != "complete":
        raise InventoryError("trusted source product environment is missing complete product units")
    contracts = [spec.get("environment_contract") for spec in product_specs]
    if (any(not isinstance(contract, dict) for contract in contracts)
            or any(contract != contracts[0] for contract in contracts[1:])):
        raise InventoryError("trusted source product units disagree on their environment contract")
    envelope = {
        "schema": _TRUSTED_SOURCE_PRODUCT_ENVIRONMENT_SCHEMA,
        "trusted_source_attempt": trusted_source_attempt,
        "required_gate_job": gate,
        "environment_contract": contracts[0],
    }
    _trusted_source_product_environment(envelope)
    source_result_units = [item["unit_id"] for item in trusted_source_attempt["result_artifacts"]]
    if source_result_units != plan.get("required_test_units"):
        raise InventoryError("trusted source product environment result set differs from its plan")
    if (trusted_source_attempt.get("workflow_run_id") != plan.get("workflow_run_id")
            or trusted_source_attempt.get("run_attempt") != plan.get("run_attempt")
            or trusted_source_attempt.get("check_app_id") != plan.get("check_app_id")
            or trusted_source_attempt.get("check_run_id") != plan.get("check_run_id")
            or trusted_source_attempt.get("job_id") != plan.get("job_id")
            or type(trusted_source_attempt.get("plan_artifact_id")) is not int
            or trusted_source_attempt.get("job_name") != "required-gate"
            or trusted_source_attempt.get("request_key") != plan.get("request_key")):
        raise InventoryError("trusted source product environment differs from its validated plan")
    return envelope


def _product_environment_contract(
    planner_root: Path, target_repo_root: Path, target_oid: str, c2: Any,
    trusted_source_product_environment: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Bind the pure Markdown checker runtime and enable only its exact closure."""
    target_entries = c2.git_tree_entries(str(target_repo_root), target_oid)[2]

    def target_blob(relative: str) -> bytes | None:
        entry = target_entries.get(relative)
        if not entry or entry["type"] != "blob" or entry["mode"] == "120000":
            return None
        try:
            result = subprocess.run(
                ["git", "-C", str(target_repo_root), "cat-file", "blob", entry["oid"]],
                check=True, capture_output=True,
            )
        except (OSError, subprocess.CalledProcessError) as exc:
            raise InventoryError(f"target product checker input is unreadable: {relative}") from exc
        return result.stdout

    parser_relative = "scripts/product_doc_markdown.py"
    requirements_relative = "scripts/doc-governance-requirements.txt"
    try:
        trusted_parser = (planner_root / parser_relative).read_bytes()
        trusted_requirements = (planner_root / requirements_relative).read_bytes()
    except OSError as exc:
        raise InventoryError("trusted product Markdown runtime contract is unavailable") from exc
    target_parser = target_blob(parser_relative)
    target_requirements = target_blob(requirements_relative)
    parser_matches = target_parser is not None and target_parser == trusted_parser
    requirements_match = target_requirements is not None and target_requirements == trusted_requirements
    requirement_lines = [
        line.strip() for line in trusted_requirements.decode("utf-8", errors="replace").splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    ]
    source_environment_eligible = True
    if trusted_source_product_environment is None:
        runtime_packages: dict[str, str] = {}
        for distribution in _TRUSTED_MARKDOWN_PACKAGES:
            try:
                runtime_packages[distribution] = package_version(distribution)
            except PackageNotFoundError:
                runtime_packages[distribution] = "unavailable"
        runner = _observe_runner_identity()
        python = _observe_python_identity()
        python_implementation=python["implementation"]
        python_version=python["version"]
        python_full_version=python["full_version"]
        python_cache_tag=python["cache_tag"]
        python_matches = (
            python_implementation == _TRUSTED_PYTHON_IMPLEMENTATION
            and python_version == _TRUSTED_PYTHON_VERSION
            and python_full_version.startswith(_TRUSTED_PYTHON_VERSION + " ")
            and python_cache_tag == "cpython-312"
        )
        runner_matches=runner["matches_trusted_W"]
    else:
        observed, _ = _trusted_source_product_environment(
            trusted_source_product_environment,
        )
        runner=observed["runner_identity"]
        runtime_packages=observed["runtime_packages"]
        python_implementation=observed["python_implementation"]
        python_version=observed["python_version"]
        python_full_version=observed["python_full_version"]
        python_cache_tag=observed["python_cache_tag"]
        python_matches=observed["python_runtime_matches_trusted_W"]
        runner_matches=observed["runner_image_matches_trusted_W"]
        source_environment_eligible=(
            observed["reuse_eligible"]
            and observed["markdown_parser_matches_trusted_W"]
            and observed["markdown_requirements_match_trusted_W"]
        )
    dependencies_match = (
        requirement_lines == ["markdown-it-py==3.0.0"]
        and runtime_packages == _TRUSTED_MARKDOWN_PACKAGES
    )
    reuse_eligible = (
        parser_matches and requirements_match and dependencies_match
        and runner_matches and python_matches and source_environment_eligible
    )
    result={
        "schema": _ENV_SCHEMA,
        "runner_image": runner["runner_image"],
        "runner_identity": runner,
        "runner_image_matches_trusted_W": runner_matches,
        "python_implementation": python_implementation,
        "python_version": python_version,
        "python_full_version": python_full_version,
        "python_cache_tag": python_cache_tag,
        "python_runtime_matches_trusted_W": python_matches,
        "runtime_packages": runtime_packages,
        "markdown_runtime_matches_trusted_W": dependencies_match,
        "markdown_parser_matches_trusted_W": parser_matches,
        "markdown_requirements_match_trusted_W": requirements_match,
        "external_inputs": "fully-bound-python-and-pinned-markdown-runtime" if reuse_eligible
            else "unverified-runner-python-markdown-runtime-or-target-parser",
        "reuse_eligible": reuse_eligible,
    }
    return result


def _observe_runner_identity() -> dict[str, Any]:
    """Read the actual GitHub runner and OS identity; never infer it from W."""
    release: dict[str, str] = {}
    try:
        for line in Path("/etc/os-release").read_text(encoding="utf-8").splitlines():
            key, separator, value = line.partition("=")
            if separator:
                release[key] = value.strip().strip('"').strip("'")
    except OSError:
        pass
    runner_os = os.environ.get("RUNNER_OS", "unavailable")
    image_os = os.environ.get("ImageOS", "unavailable")
    image_version = os.environ.get("ImageVersion", "unavailable")
    matches = (
        platform.system() == "Linux"
        and runner_os == "Linux"
        and image_os == "ubuntu24"
        and bool(image_version)
        and image_version != "unavailable"
        and release.get("ID") == "ubuntu"
        and release.get("VERSION_ID") == "24.04"
    )
    if matches:
        image = _TRUSTED_RUNNER_IMAGE
    elif image_os != "unavailable":
        image = image_os
    elif platform.system() == "Darwin":
        mac_version = platform.mac_ver()[0]
        image = "macos-" + ".".join(mac_version.split(".")[:2]) if mac_version else "macos-unknown"
    else:
        image = "unknown"
    return {
        "runner_image": image,
        "runner_os": runner_os,
        "image_os": image_os,
        "image_version": image_version,
        "os_id": release.get("ID", "unavailable"),
        "os_version_id": release.get("VERSION_ID", "unavailable"),
        "matches_trusted_W": matches,
    }


def _observe_python_identity() -> dict[str, str]:
    """Record the interpreter that actually ran the product-corpus checker."""
    return {
        "implementation": platform.python_implementation(),
        "version": platform.python_version(),
        "full_version": sys.version,
        "cache_tag": sys.implementation.cache_tag or "unavailable",
    }


def _canonical_digest(value: Any) -> str:
    encoded = json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False,
    ).encode("utf-8")
    return "sha256:" + hashlib.sha256(encoded).hexdigest()


def _validate_changed_paths(changed_paths: list[str] | tuple[str, ...]) -> list[str]:
    if not isinstance(changed_paths, (list, tuple)):
        raise InventoryError("trusted planner changed paths must be a list")
    paths: list[str] = []
    for path in changed_paths:
        if (not isinstance(path, str) or not path or "\x00" in path
                or "\n" in path or "\r" in path or "\\" in path or path.startswith("/")
                or any(part in {"", ".", ".."} for part in path.split("/"))):
            raise InventoryError("trusted planner changed paths are malformed")
        paths.append(path)
    if len(paths) != len(set(paths)):
        raise InventoryError("trusted planner changed paths must be unique")
    return paths


def _validate_ambient_github_identity(
    repository: str, event_name: str, producer: dict[str, int],
) -> None:
    """When running in Actions, bind supplied run identity to ambient context."""
    expected = {
        "GITHUB_REPOSITORY": repository,
        "GITHUB_EVENT_NAME": event_name,
        "GITHUB_RUN_ID": str(producer["run_id"]),
        "GITHUB_RUN_ATTEMPT": str(producer["run_attempt"]),
    }
    if not any(name in os.environ for name in expected):
        return
    mismatched = [name for name, value in expected.items() if os.environ.get(name) != value]
    if mismatched:
        raise InventoryError(
            "producer identity does not match current GitHub run context: "
            + ", ".join(mismatched)
        )


def _replay_trusted_planner(
    planner_root: Path, planner_path: Path, config_path: Path,
    plan: dict[str, str], *, event_name: str, run_mode: str,
    changed_paths: list[str], base_ref: str | None, head_ref: str | None,
    task_uid: str | None, scope_base_oid: str | None,
    impact_projection: str | None,
) -> tuple[dict[str, str], str]:
    """Re-run W with the invocation context and reject supplied plan drift."""
    if event_name not in _PLANNER_EVENT_NAMES:
        raise InventoryError("trusted planner event_name is unsupported")
    if run_mode not in _PLANNER_RUN_MODES:
        raise InventoryError("trusted planner run_mode is unsupported")
    if (base_ref is None) != (head_ref is None):
        raise InventoryError("trusted planner base_ref and head_ref must be supplied together")
    if base_ref is not None and not changed_paths:
        raise InventoryError(
            "trusted planner exact changed paths are required when commit refs are supplied"
        )
    for name, ref in (("base_ref", base_ref), ("head_ref", head_ref),
                      ("scope_base_oid", scope_base_oid)):
        if ref is not None and not re.fullmatch(r"(?:[0-9a-f]{40}|[0-9a-f]{64})", ref):
            raise InventoryError(f"trusted planner {name} must be a full commit OID")
    if impact_projection is not None and not (task_uid and head_ref and scope_base_oid):
        raise InventoryError("trusted planner projection identity is incomplete")
    if task_uid is not None and not re.fullmatch(r"task_[0-9a-f]{32}", task_uid):
        raise InventoryError("trusted planner task UID is malformed")

    command = [
        sys.executable, str(planner_path), "--event-name", event_name,
        "--run-mode", run_mode, "--config", str(config_path),
    ]
    if base_ref is not None:
        command.extend(("--base-ref", base_ref, "--head-ref", head_ref or ""))
    if task_uid is not None:
        command.extend(("--task-uid", task_uid))
    if scope_base_oid is not None:
        command.extend(("--scope-base-oid", scope_base_oid))
    if impact_projection is not None:
        command.extend(("--impact-projection", impact_projection))
    for path in changed_paths:
        command.append("--changed-path=" + path)
    projection_path = Path(impact_projection) if impact_projection is not None else None
    if projection_path is not None and (projection_path.is_symlink() or not projection_path.is_file()):
        raise InventoryError("trusted planner impact projection is not a regular file")
    projection_digest_before = _sha256(projection_path) if projection_path is not None else ""
    try:
        actual_text = subprocess.run(
            command, cwd=planner_root, check=True, capture_output=True, text=True,
        ).stdout
    except (OSError, subprocess.CalledProcessError) as exc:
        detail = exc.stderr.strip() if isinstance(exc, subprocess.CalledProcessError) else str(exc)
        raise InventoryError("trusted W planner invocation failed: " + detail) from exc
    projection_digest_after = _sha256(projection_path) if projection_path is not None else ""
    if projection_digest_after != projection_digest_before:
        raise InventoryError("trusted planner impact projection changed during invocation")
    actual = _plan_mapping(actual_text)
    if plan != actual:
        raise InventoryError("planner output does not match trusted W invocation event and changed paths")
    return actual, projection_digest_after


def _validate_trusted_plan(
    planner_root: Path, plan: dict[str, str], repository: str, workflow_ref: str,
    planner_authority_oid: str, *, event_name: str, run_mode: str,
    changed_paths: list[str], base_ref: str | None, head_ref: str | None,
    task_uid: str | None, scope_base_oid: str | None,
    impact_projection: str | None, producer: dict[str, int],
) -> tuple[Any, Any, dict[str, Any], dict[str, Any]]:
    planner_path = planner_root / "scripts/plan-rust-required-scope.py"
    config_path = planner_root / "scripts/ci-required-scope.v2.json"
    runner_path = planner_root / "scripts/ci-tests.sh"
    inventory_path = planner_root / "scripts/ci-required-capability-test-inventory.tsv"
    c2_path = planner_root / "scripts/pm/ci_input_scope.py"
    if not all(path.is_file() for path in (planner_path, config_path, runner_path, inventory_path, c2_path)):
        raise InventoryError("trusted W planner, runner, C2 helper, or inventory is unavailable")
    if not re.fullmatch(r"(?:[0-9a-f]{40}|[0-9a-f]{64})", planner_authority_oid):
        raise InventoryError("trusted planner authority OID must be a full Git commit OID")
    if repository != "eng-cc/oasis7":
        raise InventoryError("planner inventory repository is not the canonical repository")
    try:
        checkout_oid = subprocess.run(
            ["git", "-C", str(planner_root), "rev-parse", "--verify", "--end-of-options",
             "HEAD^{commit}"],
            check=True, capture_output=True, text=True,
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError) as exc:
        raise InventoryError("trusted W planner checkout identity is unavailable") from exc
    if checkout_oid != planner_authority_oid:
        raise InventoryError("trusted W planner checkout does not match planner authority OID")
    _require_clean_checkout(planner_root, "trusted W planner")
    expected_workflow = repository + "/.github/workflows/rust.yml@refs/heads/"
    if not workflow_ref.startswith(expected_workflow) or not workflow_ref[len(expected_workflow):]:
        raise InventoryError("planner workflow_ref must identify the trusted default-branch Rust workflow")
    try:
        config_raw = config_path.read_bytes()
        config = json.loads(config_raw)
    except (OSError, ValueError) as exc:
        raise InventoryError("trusted W planner config is unreadable") from exc
    config_digest = "sha256:" + hashlib.sha256(config_raw).hexdigest()
    if not isinstance(config, dict) or config.get("execution_contract") != EXECUTION_CONTRACT:
        raise InventoryError("trusted W planner config does not use the versioned execution contract")
    if plan.get("execution_contract") != EXECUTION_CONTRACT:
        raise InventoryError("planner output execution contract is missing or unsupported")
    if plan.get("planner_config_sha256") != config_digest:
        raise InventoryError("planner output is not bound to the trusted W planner config")
    planner = _load_module(planner_path, "trusted_required_scope_planner")
    capabilities = tuple(getattr(planner, "CAPABILITIES", ()))
    if config.get("capabilities") != list(capabilities):
        raise InventoryError("trusted planner source and config capability sets disagree")
    registry = required_test_unit_registry(capabilities)
    runner_text = runner_path.read_text(encoding="utf-8")
    if "run_required_gate_checks" not in runner_text:
        raise InventoryError("trusted required-gate baseline dispatcher is missing")
    for capability, spec in registry.items():
        for function in spec["runner_functions"]:
            if not re.search(r"(?m)^" + re.escape(function) + r"\(\)\s*\{", runner_text):
                raise InventoryError(f"trusted required-gate runner is missing: {function}")
        selector = spec["selector_env"]
        if capability != "required_gate_baseline" and (not selector or selector not in runner_text):
            raise InventoryError(f"trusted required-gate selector is missing: {capability}")
    changed_paths = _validate_changed_paths(changed_paths)
    plan, projection_digest = _replay_trusted_planner(
        planner_root, planner_path, config_path, plan,
        event_name=event_name, run_mode=run_mode, changed_paths=changed_paths,
        base_ref=base_ref, head_ref=head_ref, task_uid=task_uid,
        scope_base_oid=scope_base_oid, impact_projection=impact_projection,
    )
    plan_units = selected_test_units(plan, capabilities)
    selection = {
        "schema": _PLAN_INVOCATION_SCHEMA,
        "planner_authority_oid": planner_authority_oid,
        "planner_config_sha256": config_digest,
        "event_name": event_name,
        "run_mode": run_mode,
        "base_ref": base_ref or "",
        "head_ref": head_ref or "",
        "task_uid": task_uid or "",
        "scope_base_oid": scope_base_oid or "",
        "impact_projection_sha256": projection_digest,
        "changed_paths": changed_paths,
        "planner_output_sha256": _canonical_digest(plan),
    }
    selection_digest = _canonical_digest(selection)
    return planner, _load_module(c2_path, "trusted_ci_input_scope"), registry, {
        "config_digest": config_digest,
        "trusted_sources": _trusted_sources(planner_root),
        "plan_units": plan_units,
        "runner_text": runner_text,
        "planner_selection": selection,
        "planner_selection_digest": selection_digest,
        "planner_invocation": {**selection, "producer": producer, "digest": selection_digest},
    }


def _inventory_test_paths(
    planner_root: Path, capability: str,
) -> list[str]:
    inventory_path = planner_root / "scripts/ci-required-capability-test-inventory.tsv"
    paths: set[str] = set()
    try:
        with inventory_path.open(encoding="utf-8", newline="") as source:
            reader = csv.DictReader(source, delimiter="\t")
            if not {"new_required_selection", "test_paths"}.issubset(reader.fieldnames or []):
                raise InventoryError("trusted capability test inventory header is incomplete")
            for row in reader:
                selections = re.split(r"[;,]", row.get("new_required_selection", ""))
                if capability not in {item.strip() for item in selections}:
                    continue
                for raw in row.get("test_paths", "").split(","):
                    path = raw.strip()
                    if path.startswith("scripts/"):
                        paths.add(path)
    except OSError as exc:
        raise InventoryError("trusted capability test inventory is unreadable") from exc
    return sorted(paths)


def _top_level_roots(c2: Any, repo_root: str | Path, target_oid: str) -> tuple[list[str], list[str]]:
    _, _, entries = c2.git_tree_entries(str(repo_root), target_oid)
    roots = sorted(path for path, entry in entries.items()
                   if "/" not in path and entry["type"] == "tree")
    files = sorted(path for path, entry in entries.items()
                  if "/" not in path and entry["type"] != "tree")
    return roots, files


def _cargo_metadata(repo_root: Path) -> dict[str, Any]:
    command = [
        "cargo", "metadata", "--locked", "--offline", "--all-features",
        "--format-version", "1",
    ]
    try:
        result = subprocess.run(
            command, cwd=repo_root, check=True, capture_output=True, text=True,
        )
        value = json.loads(result.stdout)
    except (OSError, subprocess.CalledProcessError, ValueError) as exc:
        raise InventoryError("trusted cargo metadata cannot prove workspace package closure") from exc
    if not isinstance(value, dict) or Path(value.get("workspace_root", "")).resolve() != repo_root.resolve():
        raise InventoryError("Cargo metadata workspace identity differs from target M")
    return value


def _target_member_roots(
    capability: str, registry: dict[str, dict[str, Any]], metadata: dict[str, Any] | None,
    target_repo_root: Path, c2: Any, target_oid: str,
) -> list[str]:
    spec = registry[capability]
    roots = set(spec["member_roots"])
    package_names = spec["package_names"]
    if package_names:
        if metadata is None:
            raise InventoryError("Cargo package closure is unavailable")
        roots.update(package_dependency_roots(metadata, target_repo_root, package_names))
    if capability == "required_gate_baseline":
        top_dirs, _ = _top_level_roots(c2, target_repo_root, target_oid)
        roots.update(top_dirs)
    return sorted(roots)


def _target_root_inputs(
    capability: str, target_repo_root: Path, c2: Any, target_oid: str,
) -> list[str]:
    _, _, entries = c2.git_tree_entries(str(target_repo_root), target_oid)
    paths = {path for path in ROOT_CARGO_INPUTS if path in entries and entries[path]["type"] != "tree"}
    if capability == "required_gate_baseline":
        _, root_files = _top_level_roots(c2, target_repo_root, target_oid)
        paths.update(root_files)
    return sorted(paths)


def _target_input_paths(
    capability: str, spec: dict[str, Any], target_repo_root: Path, c2: Any, target_oid: str,
) -> list[str]:
    """Combine canonical file inputs with dynamic root-level workspace inputs."""
    return sorted(set(spec["input_paths"]) | set(
        _target_root_inputs(capability, target_repo_root, c2, target_oid)
    ))


def _unit_spec(
    capability: str, spec: dict[str, Any], planner: Any, planner_facts: dict[str, Any],
    planner_root: Path, target_repo_root: Path, target_oid: str, c2: Any,
    metadata: dict[str, Any] | None,
) -> dict[str, Any]:
    commands = list(spec["commands"])
    test_paths = _inventory_test_paths(planner_root, capability)
    for path in test_paths:
        if path not in commands:
            commands.append(path)
    command_paths = {
        "scripts/ci-tests.sh",
        "scripts/plan-rust-required-scope.py",
        "scripts/ci-required-scope.v2.json",
        "scripts/ci-required-capability-test-inventory.tsv",
        *(BASELINE_CHECKER_PATHS if capability == "required_gate_baseline" else ()),
        *test_paths,
    }
    if capability in {"product", "required_gate_baseline", "doc_checker_contracts"}:
        command_paths.update({
            "scripts/doc-governance-check.sh",
            "scripts/product-doc-governance-check.py",
            "scripts/product-doc-content-check.py",
        })
    if spec["package_names"]:
        command_paths.update({
            "Cargo.toml", "Cargo.lock",
        })
    target_entries = c2.git_tree_entries(str(target_repo_root), target_oid)[2]
    if spec["package_names"]:
        command_paths.update(
            path for path in ("rust-toolchain.toml", "rust-toolchain")
            if path in target_entries and target_entries[path]["type"] != "tree"
        )
    missing_commands = sorted(path for path in command_paths if path not in target_entries
                              or target_entries[path]["type"] == "tree")
    if missing_commands:
        raise InventoryError("required checker or test source is missing from target M: "
                             + ", ".join(missing_commands))
    planner_fields = getattr(planner, "FIELDS", {})
    policy = {
        "schema": _POLICY_SCHEMA,
        "required_check": "required-gate",
        "capability": capability,
        "selector_field": planner_fields.get(capability),
        "reuse_state": "disabled-pending-independent-activation",
        "reuse_eligible": False,
    }
    environment = {
        "schema": _ENV_SCHEMA,
        "runner_image": "ubuntu-24.04",
        "rust_metadata_mode": "locked-offline-all-features" if spec["package_names"] else "not-applicable",
        "external_inputs": "not-yet-independently-bounded",
        "reuse_eligible": False,
    }
    contract = {
        "schema": _UNIT_SCHEMA,
        "unit_id": capability,
        "runner_functions": spec["runner_functions"],
        "commands_and_obligations": commands,
        "package_names": spec["package_names"],
        "trusted_W": {
            "authority_oid": planner_facts["planner_authority_oid"],
            "sources": planner_facts["trusted_sources"],
            "planner_invocation_digest": planner_facts["planner_selection_digest"],
        },
    }
    return {
        "unit_id": capability,
        "unit_contract": contract,
        "obligation_set": [f"{capability}:{index:03d}:{item}" for index, item in enumerate(commands)],
        "command_checker_paths": sorted(command_paths),
        "input_paths": _target_input_paths(
            capability, spec, target_repo_root, c2, target_oid,
        ),
        "member_roots": _target_member_roots(
            capability, {capability: spec}, metadata, target_repo_root, c2, target_oid,
        ),
        "dependency_edges": [],
        "applicable_policy": policy,
        "environment_contract": environment,
    }


def build_required_inventory(
    planner_root: str | Path,
    target_repo_root: str | Path,
    target_oid: str,
    planner_output: str | dict[str, Any],
    *,
    repository: str,
    workflow_ref: str,
    planner_authority_oid: str,
    event_name: str,
    run_mode: str,
    changed_paths: list[str],
    base_ref: str | None = None,
    head_ref: str | None = None,
    task_uid: str | None = None,
    scope_base_oid: str | None = None,
    impact_projection: str | None = None,
    run_id: int,
    run_attempt: int,
    check_app_id: int,
    check_run_id: int,
    trusted_source_product_environment: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Build M/T unit specs and C2 snapshot from an authenticated W plan.

    The current required-gate baseline intentionally includes every tracked
    top-level workspace root. Capability scopes use the complete Cargo package
    dependency closure or conservative domain roots. If trusted Cargo metadata
    is unavailable, the only result is an all-unit unknown-closure fallback.
    """
    trusted_root = Path(planner_root).resolve()
    target_root = Path(target_repo_root).resolve()
    plan = _plan_mapping(planner_output)
    producer = {
        "run_id": run_id,
        "run_attempt": run_attempt,
        "check_app_id": check_app_id,
        "check_run_id": check_run_id,
    }
    if any(type(value) is not int or value <= 0 for value in producer.values()):
        raise InventoryError("producer run, attempt, app, and check IDs must be positive integers")
    _validate_ambient_github_identity(repository, event_name, producer)
    planner, c2, registry, planner_facts = _validate_trusted_plan(
        trusted_root, plan, repository, workflow_ref, planner_authority_oid,
        event_name=event_name, run_mode=run_mode, changed_paths=changed_paths,
        base_ref=base_ref, head_ref=head_ref, task_uid=task_uid,
        scope_base_oid=scope_base_oid, impact_projection=impact_projection,
        producer=producer,
    )
    selected_ids = selected_test_units(plan, tuple(planner.CAPABILITIES))
    repo = subprocess.run(
        ["git", "-C", str(target_root), "rev-parse", "--verify", "HEAD^{commit}"],
        check=True, capture_output=True, text=True,
    ).stdout.strip()
    target_commit, target_tree_oid, _ = c2.git_tree_entries(str(target_root), target_oid)
    if repo != target_commit:
        raise InventoryError("target working tree HEAD does not equal the planner target commit")
    _require_clean_checkout(target_root, "target M")
    planner_facts["planner_authority_oid"] = planner_authority_oid
    planner_facts["producer"] = producer
    specs: list[dict[str, Any]] = []
    metadata: dict[str, Any] | None = None
    closure_status = "complete"
    closure_reason: str | None = None
    try:
        if any(registry[unit_id]["package_names"] for unit_id in selected_ids):
            metadata = _cargo_metadata(target_root)
        for unit_id in selected_ids:
            specs.append(_unit_spec(
                unit_id, registry[unit_id], planner, planner_facts, trusted_root,
                target_root, target_commit, c2, metadata,
            ))
    except (InventoryError, c2.InputScopeError) as exc:
        closure_status = "unknown"
        closure_reason = "required_unit_input_closure_unavailable:" + str(exc)
        specs = []
        for unit_id in selected_ids:
            base = registry[unit_id]
            # Keep the full W-derived obligation set even when input closure
            # cannot be proved. Empty input scopes cannot authorize reuse.
            commands = list(base["commands"])
            for path in _inventory_test_paths(trusted_root, unit_id):
                if path not in commands:
                    commands.append(path)
            command_paths = {
                "scripts/ci-tests.sh", "scripts/plan-rust-required-scope.py",
                "scripts/ci-required-scope.v2.json",
                "scripts/ci-required-capability-test-inventory.tsv",
                *(BASELINE_CHECKER_PATHS if unit_id == "required_gate_baseline" else ()),
                *_inventory_test_paths(trusted_root, unit_id),
            }
            contract = {
                "schema": _UNIT_SCHEMA, "unit_id": unit_id,
                "runner_functions": base["runner_functions"],
                "commands_and_obligations": commands,
                "package_names": base["package_names"],
                "trusted_W": {
                    "authority_oid": planner_authority_oid,
                    "sources": planner_facts["trusted_sources"],
                    "planner_invocation_digest": planner_facts["planner_selection_digest"],
                },
            }
            specs.append({
                "unit_id": unit_id, "unit_contract": contract,
                "obligation_set": [f"{unit_id}:{index:03d}:{item}" for index, item in enumerate(commands)],
                "command_checker_paths": sorted(command_paths),
                "input_paths": [], "member_roots": [], "dependency_edges": [],
                "applicable_policy": {
                    "schema": _POLICY_SCHEMA, "required_check": "required-gate",
                    "capability": unit_id, "selector_field": getattr(planner, "FIELDS", {}).get(unit_id),
                    "reuse_state": "disabled-pending-independent-activation",
                    "reuse_eligible": False,
                },
                "environment_contract": {
                    "schema": _ENV_SCHEMA, "runner_image": "ubuntu-24.04",
                    "external_inputs": "unknown", "reuse_eligible": False,
                },
            })

    if not specs or [item["unit_id"] for item in specs] != selected_ids:
        raise InventoryError("required unit inventory is incomplete or noncanonical")
    product_checker_paths = [
        "scripts/ci-tests.sh", "scripts/doc-governance-check.sh",
        "scripts/product-doc-governance-check.py", "scripts/product-doc-content-check.py",
    ]
    product_policy = {
        "schema": _POLICY_SCHEMA, "required_check": "required-gate",
        "capability": "required_gate_baseline",
        "reuse_state": "disabled-pending-independent-activation",
    }
    product_environment = _product_environment_contract(
        trusted_root, target_root, target_commit, c2,
        trusted_source_product_environment=trusted_source_product_environment,
    )
    product_policy["reuse_eligible"] = product_environment["reuse_eligible"]
    parser_path = str(trusted_root / "scripts")
    inserted_parser_path = parser_path not in sys.path
    if inserted_parser_path:
        sys.path.insert(0, parser_path)
    try:
        product_specs, product_corpus = c2.build_product_corpus_unit_specs(
            str(target_root), target_commit,
            command_checker_paths=product_checker_paths,
            applicable_policy=product_policy,
            environment_contract=product_environment,
        )
    finally:
        if inserted_parser_path:
            sys.path.remove(parser_path)
    if product_corpus.get("status") != "complete" or product_corpus.get("errors"):
        raise InventoryError("product-corpus closure is incomplete; required gate must remain blocking")
    specs.extend(product_specs)
    inventory_digest = c2.planner_inventory_digest(specs, product_corpus, target_commit, target_tree_oid)
    issuer = {
        "schema": INVENTORY_SCHEMA,
        "authority": {
            "schema": INVENTORY_AUTHORITY_SCHEMA,
            "repository": repository,
            "workflow_ref": workflow_ref,
            "planner_authority_oid": planner_authority_oid,
            "planner_config_sha256": planner_facts["config_digest"],
        },
        "producer": producer,
        "target_oid": target_commit,
        "target_tree_oid": target_tree_oid,
        "unit_ids": sorted(item["unit_id"] for item in specs),
        "inventory_digest": inventory_digest,
    }
    fallback_ids = sorted(item["unit_id"] for item in specs)
    snapshot = c2.build_input_scope_snapshot(
        str(target_root), target_commit, specs, product_corpus,
        planner_inventory_issuer=issuer,
        closure_status=closure_status,
        closure_reason=closure_reason,
        fallback_unit_ids=fallback_ids if closure_status == "unknown" else None,
        fallback_scope_complete=closure_status == "unknown",
    )
    return {
        "schema": "oasis7-required-test-inventory/v1",
        "planner_authority_oid": planner_authority_oid,
        "planner_config_sha256": planner_facts["config_digest"],
        "planner_invocation": planner_facts["planner_invocation"],
        "planner_output": plan,
        "selected_test_units": selected_ids,
        "unit_specs": specs,
        "product_corpus": product_corpus,
        "planner_inventory_issuer": issuer,
        "input_scope": snapshot,
        "closure_status": closure_status,
        "closure_reason": closure_reason,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--planner-root", required=True)
    parser.add_argument("--target-repo-root", required=True)
    parser.add_argument("--target-oid", required=True)
    parser.add_argument("--planner-output-json", required=True,
                        help="JSON object made from the trusted W planner's step outputs")
    parser.add_argument("--repository", required=True)
    parser.add_argument("--workflow-ref", required=True)
    parser.add_argument("--planner-authority-oid", required=True)
    parser.add_argument("--event-name", choices=sorted(_PLANNER_EVENT_NAMES), required=True)
    parser.add_argument("--run-mode", choices=sorted(_PLANNER_RUN_MODES), required=True)
    parser.add_argument("--changed-path", action="append", default=[])
    parser.add_argument("--base-ref")
    parser.add_argument("--head-ref")
    parser.add_argument("--task-uid")
    parser.add_argument("--scope-base-oid")
    parser.add_argument("--impact-projection")
    parser.add_argument("--run-id", required=True, type=int)
    parser.add_argument("--run-attempt", required=True, type=int)
    parser.add_argument("--check-app-id", required=True, type=int)
    parser.add_argument("--check-run-id", required=True, type=int)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        plan = json.loads(Path(args.planner_output_json).read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise SystemExit(f"ci-required-inventory: trusted planner output is unreadable: {exc}") from exc
    if not isinstance(plan, dict):
        raise SystemExit("ci-required-inventory: planner output JSON must be an object")
    try:
        result = build_required_inventory(
            args.planner_root, args.target_repo_root, args.target_oid, plan,
            repository=args.repository,
            workflow_ref=args.workflow_ref,
            planner_authority_oid=args.planner_authority_oid,
            event_name=args.event_name,
            run_mode=args.run_mode,
            changed_paths=args.changed_path,
            base_ref=args.base_ref,
            head_ref=args.head_ref,
            task_uid=args.task_uid,
            scope_base_oid=args.scope_base_oid,
            impact_projection=args.impact_projection,
            run_id=args.run_id,
            run_attempt=args.run_attempt,
            check_app_id=args.check_app_id,
            check_run_id=args.check_run_id,
        )
    except (InventoryError, subprocess.CalledProcessError, OSError) as exc:
        raise SystemExit(f"ci-required-inventory: {exc}") from exc
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = output.with_name(output.name + ".tmp")
    temporary.write_text(json.dumps(result, sort_keys=True, separators=(",", ":")) + "\n",
                         encoding="utf-8")
    os.replace(temporary, output)


if __name__ == "__main__":
    main()
