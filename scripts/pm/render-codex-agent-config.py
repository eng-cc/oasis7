#!/usr/bin/env python3
"""Render Codex registry/adapters from structured role-card projections."""

from __future__ import annotations

import argparse
import re
from pathlib import Path
from typing import Any

try:
    import tomllib
except ModuleNotFoundError as error:  # pragma: no cover
    raise SystemExit("render-codex-agent-config: Python 3.11+ tomllib is required") from error


FIELDS = {
    "schema",
    "registry_description",
    "context_contract",
    "domain_contract",
    "operational_constraints",
    "return_contract",
}


def fail(message: str) -> None:
    raise SystemExit(f"render-codex-agent-config: {message}")


def projection(role_card: Path, role: str) -> dict[str, Any]:
    text = role_card.read_text(encoding="utf-8")
    match = re.search(
        r"^## Codex Adapter Projection\n```toml\n(.*?)^```$",
        text,
        re.MULTILINE | re.DOTALL,
    )
    if not match:
        fail(f"{role_card} missing structured Codex Adapter Projection")
    try:
        value = tomllib.loads(match.group(1))
    except tomllib.TOMLDecodeError as error:
        fail(f"invalid projection TOML in {role_card}: {error}")
    if set(value) != FIELDS or value.get("schema") != 1:
        fail(f"{role_card} projection must use schema=1 and exact fields {sorted(FIELDS)}")
    for key in FIELDS - {"schema"}:
        if not isinstance(value[key], str) or not value[key].strip():
            fail(f"{role_card} projection field {key} must be a non-blank string")
        value[key] = value[key].strip()
    return value


def instructions(role: str, value: dict[str, Any]) -> str:
    return "\n\n".join(
        [
            f"You are the oasis7 {role} bounded specialist subagent. {value['context_contract']}",
            value["domain_contract"],
            value["operational_constraints"],
            value["return_contract"],
        ]
    )


def adapter_toml(role: str, value: dict[str, Any]) -> str:
    rendered = instructions(role, value)
    if '"""' in rendered:
        fail(f"{role} rendered instructions contain unsupported triple quotes")
    return f'developer_instructions = """\n{rendered}\n"""\n'


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--role", required=True)
    parser.add_argument("--field", choices=("description", "instructions", "adapter-toml"), required=True)
    args = parser.parse_args()
    root = args.root.resolve()
    value = projection(root / f".agents/roles/{args.role}.md", args.role)
    if args.field == "description":
        print(value["registry_description"])
    elif args.field == "instructions":
        print(instructions(args.role, value))
    else:
        print(adapter_toml(args.role, value), end="")


if __name__ == "__main__":
    main()
