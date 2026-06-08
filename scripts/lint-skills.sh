#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

python3 - <<'PY'
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(".agents/skills")
MAX_ENTRYPOINT_LINES = 300
CORE_SKILLS_REQUIRING_FAILURE_MODES = {
    "default-workflow-bootstrap",
    "executing-project-tasks",
    "finishing-a-development-branch",
    "receiving-code-review",
    "requesting-repo-owned-review",
    "repo-owned-workflow-router",
    "verification-before-completion",
    "writing-repo-owned-skills",
}


def parse_frontmatter(text: str, path: Path) -> tuple[dict[str, str], str]:
    if not text.startswith("---\n"):
        raise ValueError(f"{path}: missing YAML frontmatter")
    try:
        _, raw, body = text.split("---", 2)
    except ValueError as exc:
        raise ValueError(f"{path}: malformed YAML frontmatter") from exc

    fields: dict[str, str] = {}
    current_key: str | None = None
    current_value: list[str] = []
    for line in raw.splitlines():
        if not line.strip():
            continue
        if not line.startswith(" ") and ":" in line:
            if current_key is not None:
                fields[current_key] = "\n".join(current_value).strip()
            key, value = line.split(":", 1)
            current_key = key.strip()
            current_value = [value.strip().strip("\"'")]
            continue
        if current_key is not None:
            current_value.append(line.strip().strip("\"'"))
    if current_key is not None:
        fields[current_key] = "\n".join(current_value).strip()
    return fields, body


def referenced_supporting_paths(body: str) -> list[str]:
    paths: list[str] = []
    in_supporting = False
    for line in body.splitlines():
        if line.startswith("## "):
            in_supporting = line.strip() == "## Supporting Files"
            continue
        if not in_supporting:
            continue
        for match in re.finditer(r"`([^`]+)`", line):
            value = match.group(1)
            if value.startswith(("references/", "scripts/", "assets/", "templates/", "gallery/", "tests/", "tools/")):
                paths.append(value)
    return paths


failures: list[str] = []
skill_files = sorted(ROOT.glob("*/SKILL.md"))
if not skill_files:
    failures.append("no skill entrypoints found under .agents/skills/*/SKILL.md")

for path in skill_files:
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    skill_name = path.parent.name
    try:
        fields, body = parse_frontmatter(text, path)
    except ValueError as exc:
        failures.append(str(exc))
        continue

    declared_name = fields.get("name", "")
    description = " ".join(fields.get("description", "").split())
    if declared_name != skill_name:
        failures.append(f"{path}: frontmatter name {declared_name!r} must match directory {skill_name!r}")
    if not re.fullmatch(r"[a-z0-9-]+", declared_name):
        failures.append(f"{path}: frontmatter name must use lowercase letters, digits, and hyphens")
    if not description.startswith("Use when"):
        failures.append(f"{path}: description must start with 'Use when' and describe trigger conditions")
    if len(lines) > MAX_ENTRYPOINT_LINES:
        failures.append(
            f"{path}: entrypoint is {len(lines)} lines; keep SKILL.md <= {MAX_ENTRYPOINT_LINES} lines "
            "and move heavy guidance to references/"
        )
    if "## When to Use" not in body:
        failures.append(f"{path}: missing '## When to Use'")
    if "## Guardrails" not in body:
        failures.append(f"{path}: missing '## Guardrails'")
    if skill_name in CORE_SKILLS_REQUIRING_FAILURE_MODES and "## Known Failure Modes" not in body:
        failures.append(f"{path}: core workflow skill must include '## Known Failure Modes'")

    for relative in referenced_supporting_paths(body):
        target = path.parent / relative
        if not target.exists():
            failures.append(f"{path}: Supporting Files entry does not exist: {relative}")

if failures:
    print("lint-skills: FAIL", file=sys.stderr)
    for failure in failures:
        print(f"- {failure}", file=sys.stderr)
    sys.exit(1)

print(f"lint-skills: OK ({len(skill_files)} skill entrypoints checked)")
PY
