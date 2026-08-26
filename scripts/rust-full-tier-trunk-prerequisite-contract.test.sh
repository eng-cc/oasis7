#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKFLOW="$ROOT_DIR/.github/workflows/rust.yml"

python3 - "$WORKFLOW" <<'PY'
from pathlib import Path
import re
import sys

workflow_path = Path(sys.argv[1])
lines = workflow_path.read_text(encoding="utf-8").splitlines()
job_header = re.compile(r"^  (?P<name>[A-Za-z0-9_-]+):\s*$")
run_key = re.compile(r"^(?P<indent>\s*)run:\s*(?P<body>.*)$")
target = "CI_VERBOSE=1 ./scripts/ci-tests.sh full"
trunk_install = "cargo install trunk --locked"

try:
    jobs_index = next(index for index, line in enumerate(lines) if line == "jobs:")
except StopIteration:
    raise SystemExit("rust.yml contract: missing jobs: mapping")

jobs = {}
current_name = None
for index in range(jobs_index + 1, len(lines)):
    match = job_header.match(lines[index])
    if match:
        current_name = match.group("name")
        jobs[current_name] = []
        continue
    if current_name is not None:
        jobs[current_name].append((index, lines[index]))

def run_commands(job_lines):
    """Return (line number, shell text) for each YAML run key and body."""
    commands = []
    position = 0
    while position < len(job_lines):
        line_number, line = job_lines[position]
        match = run_key.match(line)
        if not match:
            position += 1
            continue

        body = match.group("body").strip()
        commands.append((line_number, body))
        if body in {"|", ">", "|-", "|+", ">-", ">+"}:
            run_indent = len(match.group("indent"))
            position += 1
            while position < len(job_lines):
                body_line_number, body_line = job_lines[position]
                if body_line.strip() and len(body_line) - len(body_line.lstrip()) <= run_indent:
                    break
                commands.append((body_line_number, body_line.strip()))
                position += 1
            continue
        position += 1
    return commands

full_jobs = []
failures = []
for job_name, job_lines in jobs.items():
    commands = run_commands(job_lines)
    invocation_lines = [
        (line_number, command)
        for line_number, command in commands
        if target in command and "--command" not in command
    ]
    if not invocation_lines:
        continue

    full_jobs.append(job_name)
    invocation_line = invocation_lines[0][0]
    install_lines = [
        line_number
        for line_number, command in commands
        if trunk_install in command and line_number < invocation_line
    ]
    if not install_lines:
        failures.append(
            f"{job_name}: full-tier invocation at rust.yml:{invocation_line + 1} "
            f"lacks preceding `{trunk_install}`"
        )

expected_jobs = {"full-regression", "full-escalation"}
missing_expected_jobs = sorted(expected_jobs - set(full_jobs))
if missing_expected_jobs:
    failures.extend(
        f"{job}: expected full-tier job invocation was not found"
        for job in missing_expected_jobs
    )

if failures:
    print("rust-full-tier-trunk-prerequisite contract: FAIL", file=sys.stderr)
    for failure in failures:
        print(f"- {failure}", file=sys.stderr)
    raise SystemExit(1)

print(
    "rust-full-tier-trunk-prerequisite contract: passed "
    f"({len(full_jobs)} full-tier jobs install trunk before invocation)"
)
PY
