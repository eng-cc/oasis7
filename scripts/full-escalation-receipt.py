#!/usr/bin/env python3
"""Validate human-authorized full escalation inputs and emit a bound receipt."""
import argparse
import datetime as dt
import json
import re
from pathlib import Path

REASONS = {"release", "high_risk", "history_defect", "signal"}
CONCLUSIONS = {"success", "failure", "cancelled", "skipped"}
FULL_COMMAND = "CI_VERBOSE=1 ./scripts/ci-tests.sh full"


def die(message):
    raise SystemExit(f"full-escalation-receipt: {message}")


def valid_timestamp(value, name):
    try:
        parsed = dt.datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        die(f"{name} must be an ISO-8601 timestamp")
    if parsed.tzinfo is None:
        die(f"{name} must include a timezone")
    return parsed


def validate(args):
    if args.trigger != "workflow_dispatch":
        die("trigger must be workflow_dispatch")
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", args.repository):
        die("repository must be owner/name")
    if args.run_id < 1 or args.run_attempt < 1:
        die("run_id and run_attempt must be positive")
    if not args.actor:
        die("actor is required")
    if not re.fullmatch(r"task_[a-z0-9]{32}", args.task_uid):
        die("task_uid format is invalid")
    if args.pr_number < 1:
        die("pr_number must be positive")
    if args.escalation_reason not in REASONS:
        die("escalation_reason is invalid")
    evidence = rf"https://github\.com/{re.escape(args.repository)}/issues/[1-9][0-9]*#issuecomment-[1-9][0-9]*"
    if not re.fullmatch(evidence, args.evidence_url):
        die("evidence_url must be a same-repository issue-comment URL")
    if not args.ref:
        die("ref is required")
    for name in ("expected_head", "actual_head", "workflow_commit"):
        if not re.fullmatch(r"[0-9a-f]{40,64}", getattr(args, name)):
            die(f"{name} must be a lowercase git object id")
    if args.expected_head != args.actual_head:
        die("expected_head does not match actual_head")
    if args.command != FULL_COMMAND:
        die("command must bind the canonical full test command")
    started = valid_timestamp(args.started_at, "started_at")
    finished = valid_timestamp(args.finished_at, "finished_at")
    if finished < started:
        die("finished_at precedes started_at")
    if args.conclusion not in CONCLUSIONS:
        die("conclusion is invalid")


def receipt(args):
    return {
        "schema": "oasis7-full-escalation-receipt-v1",
        "trigger": args.trigger,
        "repository": args.repository,
        "run_id": args.run_id,
        "run_attempt": args.run_attempt,
        "actor": args.actor,
        "task_uid": args.task_uid,
        "pr_number": args.pr_number,
        "escalation_reason": args.escalation_reason,
        "evidence_url": args.evidence_url,
        "ref": args.ref,
        "expected_head": args.expected_head,
        "actual_head": args.actual_head,
        "workflow_commit": args.workflow_commit,
        "command": args.command,
        "started_at": args.started_at,
        "finished_at": args.finished_at,
        "conclusion": args.conclusion,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=("validate", "receipt"))
    parser.add_argument("--trigger", required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--run-id", required=True, type=int)
    parser.add_argument("--run-attempt", required=True, type=int)
    parser.add_argument("--actor", required=True)
    parser.add_argument("--task-uid", required=True)
    parser.add_argument("--pr-number", required=True, type=int)
    parser.add_argument("--escalation-reason", required=True)
    parser.add_argument("--evidence-url", required=True)
    parser.add_argument("--ref", required=True)
    parser.add_argument("--expected-head", required=True)
    parser.add_argument("--actual-head", required=True)
    parser.add_argument("--workflow-commit", required=True)
    parser.add_argument("--command", required=True)
    parser.add_argument("--started-at", required=True)
    parser.add_argument("--finished-at", required=True)
    parser.add_argument("--conclusion", required=True)
    parser.add_argument("--output")
    args = parser.parse_args()
    validate(args)
    if args.mode == "receipt":
        payload = json.dumps(receipt(args), sort_keys=True, separators=(",", ":")) + "\n"
        if args.output:
            Path(args.output).write_text(payload, encoding="utf-8")
        else:
            print(payload, end="")


if __name__ == "__main__":
    main()
