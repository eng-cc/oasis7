#!/usr/bin/env python3
"""Validate and render versioned required-gate planner environment inputs."""

from __future__ import annotations

import sys


EXECUTION_CONTRACT = "required-domain-split/v1"
PLANNER_SELECTOR_FIELDS = (
    "run_oasis7_required_tests",
    "run_consensus_tests",
    "run_distfs_tests",
    "run_oasis7_node_tests",
    "run_oasis7_net_tests",
    "run_oasis7_net_libp2p_tests",
    "run_viewer_contract_tests",
    "run_viewer_wasm_check",
    "run_viewer_perf_smoke",
    "run_pixel_world_bridge_lib_tests",
    "run_pixel_world_bridge_wasm_check",
    "run_launcher_web_build",
    "run_oasis7_workspace_support_crate_tests",
    "run_scenario_regression",
    "run_operational_contracts",
    "run_workflow_governance_contracts",
    "run_packaging_contracts",
    "run_doc_checker_contracts",
    "run_cargo_tooling_contracts",
    "run_site_contract_tests",
    "run_codex_agent_config_validation",
    "run_compile_metrics_contract_tests",
    "run_rust_baseline",
)
VERSIONED_PLANNER_FIELDS = (
    "run_workflow_governance_contracts",
    "run_packaging_contracts",
    "run_doc_checker_contracts",
    "run_cargo_tooling_contracts",
    "needs_python",
    "needs_markdown",
)
RESOURCE_FIELDS = (
    "needs_python",
    "needs_markdown",
    "needs_rust_toolchain",
    "needs_node",
    "needs_system_deps",
    "needs_trunk",
    "needs_wasm_target",
)
VERSIONED_ENV_FIELDS = (
    ("run_workflow_governance_contracts", "OASIS7_CI_RUN_WORKFLOW_GOVERNANCE_CONTRACTS"),
    ("run_packaging_contracts", "OASIS7_CI_RUN_PACKAGING_CONTRACTS"),
    ("run_doc_checker_contracts", "OASIS7_CI_RUN_DOC_CHECKER_CONTRACTS"),
    ("run_cargo_tooling_contracts", "OASIS7_CI_RUN_CARGO_TOOLING_CONTRACTS"),
    ("needs_python", "OASIS7_CI_NEEDS_PYTHON"),
    ("needs_markdown", "OASIS7_CI_NEEDS_MARKDOWN"),
    ("needs_rust_toolchain", "OASIS7_CI_NEEDS_RUST_TOOLCHAIN"),
    ("needs_node", "OASIS7_CI_NEEDS_NODE"),
    ("needs_system_deps", "OASIS7_CI_NEEDS_SYSTEM_DEPS"),
    ("needs_trunk", "OASIS7_CI_NEEDS_TRUNK"),
    ("needs_wasm_target", "OASIS7_CI_NEEDS_WASM_TARGET"),
)


def planner_values(text: str) -> dict[str, str]:
    values: dict[str, str] = {}
    for line_number, line in enumerate(text.splitlines(), start=1):
        if not line or "=" not in line:
            continue
        key, value = line.split("=", 1)
        if not key or key in values:
            raise ValueError(f"planner output has an empty or duplicate key on line {line_number}")
        values[key] = value
    return values


def render_versioned_environment(text: str) -> str:
    values = planner_values(text)
    contract = values.get("execution_contract", "")
    if not contract:
        mixed = sorted(set(VERSIONED_PLANNER_FIELDS).intersection(values))
        if mixed:
            raise ValueError("unversioned planner output contains versioned fields: " + ", ".join(mixed))
        return ""
    if contract != EXECUTION_CONTRACT:
        raise ValueError("unsupported planner execution contract: " + contract)

    required_fields = set(PLANNER_SELECTOR_FIELDS) | set(RESOURCE_FIELDS) | {"execution_contract"}
    missing = sorted(required_fields.difference(values))
    if missing:
        raise ValueError("versioned planner output is missing fields: " + ", ".join(missing))
    malformed = sorted(field for field in required_fields if field != "execution_contract" and values[field] not in {"true", "false"})
    if malformed:
        raise ValueError("versioned planner fields must be exact true/false: " + ", ".join(malformed))
    if values["needs_python"] != "true" or values["needs_markdown"] != "true":
        raise ValueError("required-gate baseline document checks require Python and Markdown")

    assignments = [f"OASIS7_CI_EXECUTION_CONTRACT={EXECUTION_CONTRACT}"]
    assignments.extend(f"{environment}={values[field]}" for field, environment in VERSIONED_ENV_FIELDS)
    return " ".join(assignments)


def main() -> int:
    try:
        print(render_versioned_environment(sys.stdin.read()))
    except ValueError as exc:
        print(f"required-gate-local-env: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
