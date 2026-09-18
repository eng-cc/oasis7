#!/usr/bin/env python3
"""Fail-closed completion validation for Cargo package/profile plans."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
from typing import Any, Iterable


SCHEMA = "oasis7-cargo-package-profile-plan/v1"


class DriverError(RuntimeError):
    pass


def validate_planned_items(
    plan: dict[str, Any],
    results: Iterable[dict[str, Any]],
    *,
    integration_base: str,
    source_head: str,
    tested_tree: str,
) -> dict[str, Any]:
    if plan.get("schema") != SCHEMA:
        raise DriverError("unknown plan schema")
    plan_id = plan.get("plan_id")
    if not isinstance(plan_id, str) or re.fullmatch(r"sha256:[0-9a-f]{64}", plan_id) is None:
        raise DriverError("plan_id must be a sha256 digest bound to plan content")
    unsigned = dict(plan)
    unsigned.pop("plan_id", None)
    expected_plan_id = "sha256:" + hashlib.sha256(
        json.dumps(unsigned, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    if plan_id != expected_plan_id:
        raise DriverError("plan_id digest is forged or does not bind plan content")
    expected_identity = {
        "integration_base": integration_base,
        "source_head": source_head,
        "tested_tree": tested_tree,
    }
    for field, value in expected_identity.items():
        if plan.get(field) != value:
            raise DriverError(f"stale {field.replace('_', ' ')} identity")

    authority = plan.get("trusted_authority")
    if not isinstance(authority, dict) or any(
        not authority.get(field)
        for field in ("policy_sha256", "planner_sha256", "toolchain")
    ):
        raise DriverError("trusted policy/planner/toolchain evidence is incomplete")

    selected = plan.get("selected_items")
    items = plan.get("items")
    if not isinstance(selected, list) or len(selected) != len(set(selected)):
        raise DriverError("duplicate or invalid planned items")
    if not isinstance(items, list) or [item.get("id") for item in items] != selected:
        raise DriverError("planned item inventory mismatch")
    if not selected:
        disposition = plan.get("execution_disposition")
        if disposition not in {"legacy_required_coverage", "full_escalation"}:
            raise DriverError("empty planned items require an explicit legacy/full disposition")
        if plan.get("disposition_validated") is not True:
            raise DriverError("empty planned item disposition is not validated")
        if disposition == "full_escalation":
            raise DriverError("full escalation requires a separate passing exact-identity full-tier receipt")
        result_list = list(results)
        if result_list:
            raise DriverError("unknown results for explicit empty-plan disposition")
        return {
            "status": "passed",
            "plan_id": plan.get("plan_id"),
            "completed_items": [],
            "execution_disposition": disposition,
            **expected_identity,
        }

    planned_by_id: dict[str, dict[str, Any]] = {}
    for item in items:
        command = item.get("command")
        command_digest = item.get("command_digest")
        if not isinstance(command, list) or not command or command[0] != "cargo":
            raise DriverError("planned command evidence is invalid")
        if command_digest != "sha256:" + hashlib.sha256(
            json.dumps(command, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest():
            raise DriverError("planned command digest evidence is invalid")
        planned_by_id[item["id"]] = item
    by_id: dict[str, dict[str, Any]] = {}
    for result in results:
        item_id = result.get("item_id")
        if item_id not in selected:
            raise DriverError(f"unknown planned item result: {item_id}")
        if item_id in by_id:
            raise DriverError(f"duplicate planned item result: {item_id}")
        planned = planned_by_id[item_id]
        status = result.get("status")
        if status == "skipped":
            raise DriverError(f"skipped planned item: {item_id}")
        if result.get("exit_code") != 0:
            raise DriverError(f"nonzero exit for planned item: {item_id}")
        if status != "passed":
            raise DriverError(f"planned item did not pass: {item_id}")
        expected_profile = {
            field: planned.get(field)
            for field in ("package", "profile", "target", "features")
        }
        if result.get("plan_id") != plan_id or result.get("command_digest") != planned.get("command_digest"):
            raise DriverError(f"result plan/command evidence mismatch: {item_id}")
        if result.get("toolchain") != authority.get("toolchain") or result.get("profile") != expected_profile:
            raise DriverError(f"result profile/toolchain evidence mismatch: {item_id}")
        for field, value in expected_identity.items():
            if result.get(field) != value:
                raise DriverError(f"{field.replace('_', ' ')} identity mismatch")
        by_id[item_id] = result
    missing = [item_id for item_id in selected if item_id not in by_id]
    if missing:
        raise DriverError("missing planned item results: " + ", ".join(missing))
    return {
        "status": "passed",
        "plan_id": plan.get("plan_id"),
        "completed_items": list(selected),
        **expected_identity,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plan", required=True)
    parser.add_argument("--results", required=True)
    parser.add_argument("--integration-base", required=True)
    parser.add_argument("--source-head", required=True)
    parser.add_argument("--tested-tree", required=True)
    args = parser.parse_args()
    plan = json.loads(Path(args.plan).read_text(encoding="utf-8"))
    results = json.loads(Path(args.results).read_text(encoding="utf-8"))
    if not isinstance(results, list):
        raise DriverError("results must be a list")
    receipt = validate_planned_items(
        plan,
        results,
        integration_base=args.integration_base,
        source_head=args.source_head,
        tested_tree=args.tested_tree,
    )
    print(json.dumps(receipt, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
