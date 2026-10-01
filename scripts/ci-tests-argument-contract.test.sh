#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPT="$ROOT_DIR/scripts/ci-tests.sh"
header="$(sed -n '1,45p' "$SCRIPT")"

if ! grep -Fq 'if [[ $# -eq 0 ]]; then' <<<"$header"; then
  echo "ci-tests must reject an omitted tier before any test command can run" >&2
  exit 1
fi

if ! grep -Fq 'Default: none (explicit tier required)' <<<"$header"; then
  echo "ci-tests usage must state that an explicit tier is required" >&2
  exit 1
fi

if ! grep -Fq 'commit|required|full|full-core|full-support) ;;' <<<"$header"; then
  echo "ci-tests must retain every explicit tier" >&2
  exit 1
fi

validator_source="$(sed -n '/^validate_required_gate_execution_contract() {/,/^}/p' "$SCRIPT")"
for required_contract in \
  'required-domain-split/v1' \
  'OASIS7_CI_RUN_WORKFLOW_GOVERNANCE_CONTRACTS' \
  'OASIS7_CI_RUN_PACKAGING_CONTRACTS' \
  'OASIS7_CI_RUN_DOC_CHECKER_CONTRACTS' \
  'OASIS7_CI_RUN_CARGO_TOOLING_CONTRACTS' \
  'OASIS7_CI_NEEDS_PYTHON' \
  'OASIS7_CI_NEEDS_MARKDOWN' \
  'Cargo tooling contracts require the planned Rust toolchain resource' \
  'net libp2p selector must match its planner-derived net selector' \
  'viewer WASM selector must match its planner-derived viewer selector' \
  'pixel-world WASM selector must match its planner-derived library selector' \
  'Rust baseline selector must match the planned Rust toolchain resource' \
  'must be explicitly true or false' \
  'unsupported required-gate execution contract'; do
  if ! grep -Fq "$required_contract" <<<"$validator_source"; then
    echo "ci-tests must fail closed for incomplete or unknown required-gate contract fields: $required_contract" >&2
    exit 1
  fi
done

validation_line="$(grep -n '^validate_required_gate_execution_contract || exit 1$' "$SCRIPT" | cut -d: -f1)"
tool_load_line="$(grep -n '^source "\$driver_dir/viewer-dependency-preflight.sh"$' "$SCRIPT" | cut -d: -f1)"
if [[ -z "$validation_line" || -z "$tool_load_line" ]] || (( validation_line >= tool_load_line )); then
  echo "required-gate contract fields must validate before the test runner loads helpers or dispatches commands" >&2
  exit 1
fi

workflow="$ROOT_DIR/.github/workflows/rust.yml"
artifact_validation_line="$(grep -n 'name: Write required planner artifact' "$workflow" | cut -d: -f1)"
rust_install_line="$(grep -n 'name: Install pinned Rust toolchains' "$workflow" | head -n1 | cut -d: -f1)"
node_setup_line="$(grep -n 'uses: actions/setup-node@' "$workflow" | head -n1 | cut -d: -f1)"
test_dispatch_line="$(grep -n 'name: Run required test tier' "$workflow" | cut -d: -f1)"
if [[ -z "$artifact_validation_line" || -z "$rust_install_line" || -z "$node_setup_line" || -z "$test_dispatch_line" ]] || \
  (( artifact_validation_line >= rust_install_line || artifact_validation_line >= node_setup_line || artifact_validation_line >= test_dispatch_line )); then
  echo "versioned planner fields must validate before tool installation and test dispatch" >&2
  exit 1
fi

for workflow_field in \
  'execution_contract=source.get("execution_contract", "")' \
  'required-domain-split/v1' \
  'unsupported required-gate execution contract' \
  'run_workflow_governance_contracts' \
  'run_packaging_contracts' \
  'run_doc_checker_contracts' \
  'run_cargo_tooling_contracts' \
  'needs_python' \
  'needs_markdown' \
  'needs_rust_toolchain' \
  'needs_node' \
  'needs_system_deps' \
  'needs_trunk' \
  'needs_wasm_target' \
  'Cargo tooling contracts require the planned Rust toolchain resource' \
  'OASIS7_CI_EXECUTION_CONTRACT:' \
  'OASIS7_CI_RUN_WORKFLOW_GOVERNANCE_CONTRACTS:' \
  'OASIS7_CI_RUN_PACKAGING_CONTRACTS:' \
  'OASIS7_CI_RUN_DOC_CHECKER_CONTRACTS:' \
  'OASIS7_CI_RUN_CARGO_TOOLING_CONTRACTS:' \
  'OASIS7_CI_NEEDS_PYTHON:' \
  'OASIS7_CI_NEEDS_MARKDOWN:' \
  'OASIS7_CI_NEEDS_RUST_TOOLCHAIN:' \
  'OASIS7_CI_NEEDS_NODE:' \
  'OASIS7_CI_NEEDS_SYSTEM_DEPS:' \
  'OASIS7_CI_NEEDS_TRUNK:' \
  'OASIS7_CI_NEEDS_WASM_TARGET:'; do
  if ! grep -Fq "$workflow_field" "$workflow"; then
    echo "workflow must validate, artifact, and forward the versioned selector/resource field: $workflow_field" >&2
    exit 1
  fi
done

for strict_mapping in \
  'OASIS7_CI_RUN_OASIS7_REQUIRED_TESTS OASIS7_CI_NEEDS_RUST_TOOLCHAIN OASIS7_CI_NEEDS_SYSTEM_DEPS' \
  'OASIS7_CI_RUN_CONSENSUS_TESTS OASIS7_CI_NEEDS_RUST_TOOLCHAIN' \
  'OASIS7_CI_RUN_DISTFS_TESTS OASIS7_CI_NEEDS_RUST_TOOLCHAIN' \
  'OASIS7_CI_RUN_OASIS7_NODE_TESTS OASIS7_CI_NEEDS_RUST_TOOLCHAIN' \
  'OASIS7_CI_RUN_OASIS7_NET_TESTS OASIS7_CI_NEEDS_RUST_TOOLCHAIN' \
  'OASIS7_CI_RUN_OASIS7_NET_LIBP2P_TESTS OASIS7_CI_NEEDS_RUST_TOOLCHAIN' \
  'OASIS7_CI_RUN_VIEWER_CONTRACT_TESTS OASIS7_CI_NEEDS_RUST_TOOLCHAIN OASIS7_CI_NEEDS_NODE OASIS7_CI_NEEDS_SYSTEM_DEPS OASIS7_CI_NEEDS_WASM_TARGET' \
  'OASIS7_CI_RUN_VIEWER_WASM_CHECK OASIS7_CI_NEEDS_RUST_TOOLCHAIN OASIS7_CI_NEEDS_NODE OASIS7_CI_NEEDS_SYSTEM_DEPS OASIS7_CI_NEEDS_WASM_TARGET' \
  'OASIS7_CI_RUN_VIEWER_PERF_SMOKE OASIS7_CI_NEEDS_NODE OASIS7_CI_NEEDS_SYSTEM_DEPS' \
  'OASIS7_CI_RUN_PIXEL_WORLD_BRIDGE_LIB_TESTS OASIS7_CI_NEEDS_RUST_TOOLCHAIN OASIS7_CI_NEEDS_SYSTEM_DEPS OASIS7_CI_NEEDS_WASM_TARGET' \
  'OASIS7_CI_RUN_PIXEL_WORLD_BRIDGE_WASM_CHECK OASIS7_CI_NEEDS_RUST_TOOLCHAIN OASIS7_CI_NEEDS_SYSTEM_DEPS OASIS7_CI_NEEDS_WASM_TARGET' \
  'OASIS7_CI_RUN_LAUNCHER_WEB_BUILD OASIS7_CI_NEEDS_RUST_TOOLCHAIN OASIS7_CI_NEEDS_NODE OASIS7_CI_NEEDS_SYSTEM_DEPS OASIS7_CI_NEEDS_WASM_TARGET OASIS7_CI_NEEDS_TRUNK' \
  'OASIS7_CI_RUN_WORKSPACE_SUPPORT_CRATE_TESTS OASIS7_CI_NEEDS_RUST_TOOLCHAIN OASIS7_CI_NEEDS_SYSTEM_DEPS' \
  'OASIS7_CI_RUN_SCENARIO_REGRESSION OASIS7_CI_NEEDS_RUST_TOOLCHAIN' \
  'OASIS7_CI_RUN_DOC_CHECKER_CONTRACTS OASIS7_CI_NEEDS_PYTHON OASIS7_CI_NEEDS_MARKDOWN' \
  'OASIS7_CI_RUN_RUST_BASELINE OASIS7_CI_NEEDS_RUST_TOOLCHAIN' \
  'OASIS7_CI_RUN_CARGO_TOOLING_CONTRACTS OASIS7_CI_NEEDS_RUST_TOOLCHAIN'; do
  if ! grep -Fq "require_planned_resources $strict_mapping" <<<"$validator_source"; then
    echo "runner must reject selector/resource contradictions: $strict_mapping" >&2
    exit 1
  fi
done

for workflow_mapping in \
  '"run_oasis7_required_tests": ("needs_rust_toolchain", "needs_system_deps")' \
  '"run_consensus_tests": ("needs_rust_toolchain",)' \
  '"run_distfs_tests": ("needs_rust_toolchain",)' \
  '"run_oasis7_node_tests": ("needs_rust_toolchain",)' \
  '"run_oasis7_net_tests": ("needs_rust_toolchain",)' \
  '"run_oasis7_net_libp2p_tests": ("needs_rust_toolchain",)' \
  '"run_viewer_contract_tests": ("needs_rust_toolchain", "needs_node", "needs_system_deps", "needs_wasm_target")' \
  '"run_viewer_wasm_check": ("needs_rust_toolchain", "needs_node", "needs_system_deps", "needs_wasm_target")' \
  '"run_viewer_perf_smoke": ("needs_node", "needs_system_deps")' \
  '"run_pixel_world_bridge_lib_tests": ("needs_rust_toolchain", "needs_system_deps", "needs_wasm_target")' \
  '"run_pixel_world_bridge_wasm_check": ("needs_rust_toolchain", "needs_system_deps", "needs_wasm_target")' \
  '"run_launcher_web_build": ("needs_rust_toolchain", "needs_node", "needs_system_deps", "needs_wasm_target", "needs_trunk")' \
  '"run_oasis7_workspace_support_crate_tests": ("needs_rust_toolchain", "needs_system_deps")' \
  '"run_scenario_regression": ("needs_rust_toolchain",)' \
  '"run_doc_checker_contracts": ("needs_python", "needs_markdown")' \
  '"run_cargo_tooling_contracts": ("needs_rust_toolchain",)' \
  '"run_rust_baseline": ("needs_rust_toolchain",)'; do
  if ! grep -Fq "$workflow_mapping" "$workflow"; then
    echo "workflow artifact preflight must reject selector/resource contradictions: $workflow_mapping" >&2
    exit 1
  fi
done

for selector_field in \
  run_operational_contracts run_workflow_governance_contracts run_packaging_contracts \
  run_doc_checker_contracts run_cargo_tooling_contracts run_site_contract_tests \
  run_codex_agent_config_validation run_compile_metrics_contract_tests run_rust_baseline; do
  if ! grep -Fq "\"$selector_field\":" "$workflow"; then
    echo "workflow fixed selector/resource allowlist is missing $selector_field" >&2
    exit 1
  fi
done

system_deps_step="$(sed -n '/name: Install system deps/,/name: Install product-document Markdown parser/p' "$workflow")"
if ! grep -Fq "outputs.needs_system_deps == 'true'" <<<"$system_deps_step" || \
   ! grep -Fq "outputs.execution_contract != 'required-domain-split/v1'" <<<"$system_deps_step" || \
   ! grep -Fq "outputs.run_oasis7_workspace_support_crate_tests == 'true'" <<<"$system_deps_step"; then
  echo "versioned system-dependency installation must follow planned resources while legacy keeps its selector fallback" >&2
  exit 1
fi

python3 - "$ROOT_DIR" "$workflow" <<'PY'
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import textwrap

root = pathlib.Path(sys.argv[1])
workflow_path = pathlib.Path(sys.argv[2])
workflow_text = workflow_path.read_text(encoding="utf-8")
transport_env = "SCOPE_OUTPUTS_PATH: ${{ runner.temp }}/required-scope-outputs.json"
if workflow_text.count(transport_env) != 3:
    raise SystemExit("scope outputs must use one bounded runner-temp file in the report, keyed-plan and artifact steps")
if "PLAN_JSON: ${{ toJSON(steps.scope.outputs) }}" in workflow_text or 'os.environ["PLAN_JSON"]' in workflow_text:
    raise SystemExit("scope outputs must not be copied into an oversized PLAN_JSON environment variable")
if workflow_text.count('pathlib.Path(os.environ["SCOPE_OUTPUTS_PATH"]).read_text(encoding="utf-8")') != 3:
    raise SystemExit("all required-planner consumers must read the complete scope output file")
report_step = workflow_text.split("      - name: Report planned scope\n", 1)[1].split(
    "\n      - name: Install pinned Markdown runtime", 1
)[0]
if "${{ steps.scope.outputs." in report_step:
    raise SystemExit("planned-scope report must print file-backed values without shell interpolation")
scope_step = workflow_text.split("      - id: scope\n", 1)[1].split(
    "\n      - name: Report planned scope", 1
)[0]
transport_marker = 'python3 - "${GITHUB_OUTPUT}" "${RUNNER_TEMP}/required-scope-outputs.json" <<\'PY\'\n'
if transport_marker not in scope_step or "required-scope output" not in scope_step:
    raise SystemExit("scope step must materialize the original GITHUB_OUTPUT values into a JSON file")
transport_code = textwrap.dedent(scope_step.split(transport_marker, 1)[1].split("\n          PY", 1)[0])
report_marker = "python3 -I - <<'PY'\n"
if report_marker not in report_step:
    raise SystemExit("planned-scope report must parse the file-backed scope outputs")
report_code = textwrap.dedent(report_step.split(report_marker, 1)[1].split("\n          PY", 1)[0])
step = workflow_text.split("      - name: Write required planner artifact\n", 1)[1].split(
    "\n      - name: Upload required planner artifact", 1
)[0]
marker = "python3 -I - <<'PY'\n"
if marker not in step or "\n          PY" not in step:
    raise SystemExit("could not extract required planner artifact preflight")
artifact_code = textwrap.dedent(step.split(marker, 1)[1].split("\n          PY", 1)[0])
planner_result = subprocess.run(
    [
        sys.executable,
        str(root / "scripts/plan-rust-required-scope.py"),
        "--event-name",
        "workflow_dispatch",
        "--config",
        str(root / "scripts/fixtures/ci-required-scope.versioned-test.json"),
    ],
    cwd=root,
    text=True,
    capture_output=True,
    check=True,
)
planner = dict(line.split("=", 1) for line in planner_result.stdout.splitlines() if "=" in line)
if planner.get("execution_contract") != "required-domain-split/v1":
    raise SystemExit("versioned artifact-preflight vector did not use the versioned test fixture")
planner.update(
    head_oid="a" * 40,
    base_oid="b" * 40,
    integration_base_oid="b" * 40,
    source_scope_base="b" * 40,
    integration_base="b" * 40,
    source_head="a" * 40,
    task_uid="task_11111111111111111111111111111111",
)

with tempfile.TemporaryDirectory(prefix="oasis7-workflow-preflight-") as temp:
    work = pathlib.Path(temp)
    (work / "output/required-plan").mkdir(parents=True)
    script = work / "artifact-preflight.py"
    script.write_text(artifact_code, encoding="utf-8")
    report_script = work / "scope-report.py"
    report_script.write_text(report_code, encoding="utf-8")

    paths = []
    for index in range(782):
        prefix = f"doc/testing/evidence/large-scope/{index:04d}/"
        target_path_bytes = 110 if index < 736 else 109
        paths.append(prefix + "x" * (target_path_bytes - len(prefix)))
    large = dict(planner)
    large["changed_path_count"] = str(len(paths))
    large["changed_paths"] = ";".join(paths)
    large["reason_summary"] = ";".join(f"evidence:{path}" for path in paths)
    marker = work / "shell-escape-was-executed"
    injection = f"$(touch {marker})"
    large["reason_summary"] += ";" + injection
    changed_paths_bytes = len(large["changed_paths"].encode("utf-8"))
    if changed_paths_bytes != 86755:
        raise SystemExit(f"782-path transport fixture changed its frozen path payload size: {changed_paths_bytes}")
    target_payload_bytes = 188608
    current_payload_bytes = len(json.dumps(large, ensure_ascii=False, separators=(",", ":")).encode("utf-8"))
    padding_bytes = target_payload_bytes - current_payload_bytes
    if padding_bytes < 0:
        raise SystemExit(f"782-path transport fixture exceeds the frozen planner payload size: {current_payload_bytes}")
    large["reason_summary"] += "r" * padding_bytes
    large_payload_bytes = len(json.dumps(large, ensure_ascii=False, separators=(",", ":")).encode("utf-8"))
    if large_payload_bytes != target_payload_bytes or large_payload_bytes <= 131072:
        raise SystemExit(f"scope transport fixture must match the 188608-byte Linux boundary regression: {large_payload_bytes}")

    github_output = work / "GITHUB_OUTPUT"
    transport_path = work / "required-scope-outputs.json"
    output_values = dict(large)
    output_lines = [f"{key}={value}" for key, value in output_values.items()]
    output_lines.extend((
        f"base_oid={output_values['base_oid']}",
        f"integration_base_oid={output_values['integration_base_oid']}",
        f"head_oid={output_values['head_oid']}",
        f"source_scope_base={output_values['source_scope_base']}",
        f"integration_base={output_values['integration_base']}",
        f"source_head={output_values['source_head']}",
        f"task_uid={output_values['task_uid']}",
    ))
    github_output.write_text("\n".join(output_lines) + "\n", encoding="utf-8")
    materialized = subprocess.run(
        [sys.executable, "-I", "-", str(github_output), str(transport_path)],
        input=transport_code,
        text=True,
        capture_output=True,
    )
    if materialized.returncode:
        raise SystemExit(f"large scope output materialization failed: {materialized.stderr}")
    transported = json.loads(transport_path.read_text(encoding="utf-8"))
    if transported != output_values:
        raise SystemExit("file transport changed or dropped required-planner output content")
    report_env = os.environ.copy()
    report_env.pop("PLAN_JSON", None)
    report_env["SCOPE_OUTPUTS_PATH"] = str(transport_path)
    reported = subprocess.run(
        [sys.executable, "-I", str(report_script)], cwd=work, env=report_env, text=True, capture_output=True
    )
    expected_report_summary = f"reason_summary={large['reason_summary']}"
    report_summary = next((line for line in reported.stdout.splitlines() if line.startswith("reason_summary=")), "<missing>")
    expected_text_present = report_summary == expected_report_summary
    if reported.returncode or not expected_text_present or marker.exists():
        raise SystemExit(
            "scope report shell-interpreted or altered planner-provided text: "
            f"status={reported.returncode}, marker={marker.exists()}, stderr={reported.stderr!r}, "
            f"expected_text_present={expected_text_present}, summary_bytes={len(report_summary.encode('utf-8'))}, "
            f"summary_prefix={report_summary[:120]!r}"
        )
    bad_output = work / "conflicting-GITHUB_OUTPUT"
    bad_target = work / "conflicting-plan.json"
    bad_output.write_text("changed_paths=first\nchanged_paths=second\n", encoding="utf-8")
    conflict = subprocess.run(
        [sys.executable, "-I", "-", str(bad_output), str(bad_target)],
        input=transport_code,
        text=True,
        capture_output=True,
    )
    if conflict.returncode == 0 or bad_target.exists():
        raise SystemExit("conflicting duplicate planner outputs must fail before publishing the transport file")

    def invoke(value):
        plan_path = work / "scope-outputs.json"
        plan_path.write_text(json.dumps(value, ensure_ascii=False, separators=(",", ":")) + "\n", encoding="utf-8")
        env = os.environ.copy()
        env.pop("PLAN_JSON", None)
        env.update(SCOPE_OUTPUTS_PATH=str(plan_path), REPOSITORY="eng-cc/oasis7", WORKFLOW_RUN_ID="1")
        if len(env["SCOPE_OUTPUTS_PATH"].encode("utf-8")) >= 131072:
            raise SystemExit("scope file path unexpectedly exceeds the per-environment-string limit")
        return subprocess.run(
            [sys.executable, "-I", str(script)], cwd=work, env=env, text=True, capture_output=True
        )

    versioned = invoke(planner)
    if versioned.returncode:
        raise SystemExit(f"valid versioned planner failed artifact preflight: {versioned.stderr}")

    large_result = invoke(large)
    if large_result.returncode:
        raise SystemExit(f"large file-backed planner failed artifact preflight: {large_result.stderr}")
    large_artifact = json.loads((work / "output/required-plan/oasis7-required-plan-v1.json").read_text(encoding="utf-8"))
    if large_artifact["planner"].get("reason_summary") != large["reason_summary"] or marker.exists():
        raise SystemExit("large artifact changed planner content or evaluated shell-like text")

    legacy = dict(planner)
    for field in (
        "execution_contract", "run_workflow_governance_contracts", "run_packaging_contracts",
        "run_doc_checker_contracts", "run_cargo_tooling_contracts", "needs_python", "needs_markdown",
    ):
        legacy.pop(field, None)
    legacy_result = invoke(legacy)
    if legacy_result.returncode:
        raise SystemExit(f"legacy planner failed artifact preflight: {legacy_result.stderr}")

    mismatched = dict(planner)
    selectors = (
        "run_oasis7_required_tests", "run_consensus_tests", "run_distfs_tests", "run_oasis7_node_tests",
        "run_oasis7_net_tests", "run_oasis7_net_libp2p_tests", "run_viewer_contract_tests",
        "run_viewer_wasm_check", "run_viewer_perf_smoke", "run_pixel_world_bridge_lib_tests",
        "run_pixel_world_bridge_wasm_check", "run_launcher_web_build", "run_oasis7_workspace_support_crate_tests",
        "run_scenario_regression", "run_operational_contracts", "run_site_contract_tests",
        "run_codex_agent_config_validation", "run_compile_metrics_contract_tests", "run_rust_baseline",
        "run_workflow_governance_contracts", "run_packaging_contracts", "run_doc_checker_contracts",
        "run_cargo_tooling_contracts",
    )
    for field in selectors:
        mismatched[field] = "false"
    for field in ("needs_node", "needs_trunk", "needs_wasm_target"):
        mismatched[field] = "false"
    mismatched.update(
        run_pixel_world_bridge_lib_tests="true",
        run_pixel_world_bridge_wasm_check="true",
        run_rust_baseline="true",
        needs_rust_toolchain="true",
        needs_system_deps="true",
        needs_wasm_target="false",
    )
    rejected = invoke(mismatched)
    expected = "run_pixel_world_bridge_lib_tests requires planned resources: needs_wasm_target"
    if rejected.returncode == 0 or expected not in rejected.stderr:
        raise SystemExit("workflow artifact preflight accepted a selected pixel-world WASM check without its WASM target")

    missing_rust = dict(mismatched)
    missing_rust.update(run_rust_baseline="false", needs_rust_toolchain="false")
    rejected_rust = invoke(missing_rust)
    expected_rust = "run_pixel_world_bridge_lib_tests requires planned resources: needs_rust_toolchain, needs_wasm_target"
    if rejected_rust.returncode == 0 or expected_rust not in rejected_rust.stderr:
        raise SystemExit("workflow artifact preflight accepted a selected pixel-world WASM check without Rust/WASM resources")

    mismatched_alias = dict(mismatched)
    mismatched_alias.update(
        run_pixel_world_bridge_lib_tests="false",
        run_pixel_world_bridge_wasm_check="true",
        run_rust_baseline="true",
        needs_rust_toolchain="true",
        needs_system_deps="true",
        needs_wasm_target="true",
    )
    rejected_alias = invoke(mismatched_alias)
    if rejected_alias.returncode == 0 or "run_pixel_world_bridge_wasm_check must match planner-derived run_pixel_world_bridge_lib_tests" not in rejected_alias.stderr:
        raise SystemExit("workflow artifact preflight accepted contradictory planner-derived pixel-world aliases")

    missing_workspace_deps = dict(mismatched)
    missing_workspace_deps.update(
        run_pixel_world_bridge_lib_tests="false",
        run_pixel_world_bridge_wasm_check="false",
        run_oasis7_workspace_support_crate_tests="true",
        run_rust_baseline="true",
        needs_rust_toolchain="true",
        needs_system_deps="false",
    )
    rejected_workspace = invoke(missing_workspace_deps)
    expected_workspace = "run_oasis7_workspace_support_crate_tests requires planned resources: needs_system_deps"
    if rejected_workspace.returncode == 0 or expected_workspace not in rejected_workspace.stderr:
        raise SystemExit("workflow artifact preflight accepted workspace-support tests without native system dependencies")
PY

macos_package_job="$(sed -n '/^  testnet-packages-macos-arm64-contract:/,/^  public-testnet-fleet-health-contract:/p' "$workflow")"
if ! grep -Fq "outputs.run_packaging_contracts == 'true'" <<<"$macos_package_job" || \
   ! grep -Fq "outputs.run_operational_contracts == 'true'" <<<"$macos_package_job" || \
   ! grep -Fq "outputs.execution_contract == 'required-domain-split/v1'" <<<"$macos_package_job"; then
  echo "macOS package child job must use packaging selection in versioned mode and preserve legacy routing" >&2
  exit 1
fi

echo "ci-tests-argument-contract.test: OK"
