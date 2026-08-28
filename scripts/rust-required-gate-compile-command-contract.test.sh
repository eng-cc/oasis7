#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
workflow="$repo_root/.github/workflows/rust.yml"
ci_tests="$repo_root/scripts/ci-tests.sh"

python3 - "$workflow" "$ci_tests" <<'PY'
import pathlib
import re
import sys

workflow = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
ci_tests = pathlib.Path(sys.argv[2]).read_text(encoding="utf-8")

toolchain_match = re.search(
    r"- name: Install pinned Rust toolchains(?P<body>.*?)\n\s*- name:",
    workflow,
    re.S,
)
if toolchain_match is None:
    raise SystemExit("required-gate Rust toolchain install step is missing")
toolchain_body = toolchain_match.group("body")
if "--profile minimal" not in toolchain_body:
    raise SystemExit("required-gate Rust toolchain install must use the minimal profile")
if "--profile default" in toolchain_body:
    raise SystemExit("required-gate Rust toolchain install must not use the default profile")
for component in ("rustfmt", "clippy"):
    if f"--component {component}" not in toolchain_body:
        raise SystemExit(
            "required-gate minimal Rust toolchain install must explicitly include "
            f"{component}"
        )

required_match = re.search(
    r"run_oasis7_required_tier_tests\(\) \{(?P<body>.*?)\n\}",
    ci_tests,
    re.S,
)
if required_match is None:
    raise SystemExit("required-tier Cargo test helper is missing")
required_command = required_match.group("body")
required_feature = re.search(r"--features\s+(\S+)", required_command)
if required_feature is None:
    raise SystemExit("required-tier Cargo test helper must declare its feature set")

bridge_match = re.search(
    r"- name: Run execution bridge binary unit suite(?P<body>.*?)\n\s*- name:",
    workflow,
    re.S,
)
if bridge_match is None:
    raise SystemExit("execution bridge binary test step is missing")
bridge_body = bridge_match.group("body")
if "cargo test -p oasis7 --bin oasis7_chain_runtime" not in bridge_body:
    raise SystemExit("execution bridge step must target oasis7_chain_runtime")
bridge_feature = re.search(r"--features\s+(\S+)", bridge_body)
if bridge_feature is None:
    raise SystemExit("execution bridge Cargo test must declare the required-tier feature set")
if bridge_feature.group(1) != required_feature.group(1):
    raise SystemExit(
        "execution bridge feature set differs from required-tier test set: "
        f"{bridge_feature.group(1)!r} != {required_feature.group(1)!r}"
    )

print(
    "rust-required-gate-compile-command-contract: feature parity "
    f"{required_feature.group(1)}"
)
PY
