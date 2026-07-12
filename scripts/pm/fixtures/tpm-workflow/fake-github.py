#!/usr/bin/env python3
"""Deterministic file-backed GitHub double for TPM lifecycle contract tests."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def load(path: Path) -> dict:
    return json.loads(path.read_text()) if path.exists() else {
        "clock": 0, "calls": [], "tasks": [], "prs": [], "comments": [],
        "merge_receipts": [], "failures": [],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--state", type=Path, required=True)
    parser.add_argument("--operation", required=True)
    parser.add_argument("--payload", default="{}")
    args = parser.parse_args()
    state = load(args.state)
    payload = json.loads(args.payload)
    state["clock"] += 1
    state["calls"].append({"at": state["clock"], "operation": args.operation, "payload": payload})

    failure = next((x for x in state["failures"] if x["operation"] == args.operation and x["remaining"] > 0), None)
    if failure:
        failure["remaining"] -= 1
        args.state.write_text(json.dumps(state, sort_keys=True))
        print(json.dumps({"ok": False, "status": failure["status"], "retry_after": failure.get("retry_after")}))
        return 75

    if args.operation == "update_comment":
        existing = next((x for x in state["comments"] if x["idempotency_key"] == payload["idempotency_key"]), None)
        if existing is None:
            result = payload
        else:
            existing.update(payload)
            result = existing
        args.state.write_text(json.dumps(state, sort_keys=True))
        print(json.dumps({"ok": True, "result": result}, sort_keys=True))
        return 0
    if args.operation == "read_comment":
        existing = next((x for x in state["comments"] if x.get("node_id") == payload.get("node_id")), None)
        args.state.write_text(json.dumps(state, sort_keys=True))
        print(json.dumps({"ok": existing is not None, "result": existing,
                          "status": 200 if existing is not None else 404}, sort_keys=True))
        return 0 if existing is not None else 75
    if args.operation == "read_task_authority":
        authority = state.get("task_authority")
        args.state.write_text(json.dumps(state, sort_keys=True))
        print(json.dumps({"ok": authority is not None, "result": authority,
                          "status": 200 if authority is not None else 404}, sort_keys=True))
        return 0 if authority is not None else 75

    collection = {"create_task": "tasks", "create_pr": "prs", "comment": "comments", "merge": "merge_receipts"}.get(args.operation)
    if collection:
        key = payload["idempotency_key"]
        existing = next((x for x in state[collection] if x["idempotency_key"] == key), None)
        if existing is None:
            existing = dict(payload, node_id=f"{collection}-{len(state[collection]) + 1}")
            state[collection].append(existing)
        result = existing
    else:
        result = payload
    args.state.write_text(json.dumps(state, sort_keys=True))
    print(json.dumps({"ok": True, "result": result}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
