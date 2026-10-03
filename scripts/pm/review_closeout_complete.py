#!/usr/bin/env python3
"""Idempotent semantic publication for the v2 review packet."""

from __future__ import annotations

import importlib.util
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


REPOSITORY = "eng-cc/oasis7"


class CloseoutError(ValueError):
    pass


def load_module(root: Path, name: str, path: str) -> Any:
    spec = importlib.util.spec_from_file_location(name, root / "scripts" / "pm" / path)
    if spec is None or spec.loader is None:
        raise CloseoutError(f"cannot load repository helper: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def gh_json(command: list[str], label: str) -> Any:
    try:
        result = subprocess.run(["gh", *command], text=True, capture_output=True, check=False)
    except OSError as exc:
        raise CloseoutError(f"cannot perform live GitHub {label} lookup: {exc}") from exc
    if result.returncode:
        detail = result.stderr.strip() or result.stdout.strip() or "command failed"
        raise CloseoutError(f"live GitHub {label} lookup failed: {detail}")
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise CloseoutError(f"live GitHub {label} lookup returned invalid JSON") from exc


def issue_number_for(root: Path, task_uid: str) -> int:
    mapping_path = root / ".pm" / "github-project-sync" / "tasks.json"
    try:
        mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise CloseoutError(f"cannot read canonical task mapping: {exc}") from exc
    tasks = mapping.get("tasks") if isinstance(mapping, dict) else None
    record = tasks.get(task_uid) if isinstance(tasks, dict) else None
    issue_number = record.get("issue_number") if isinstance(record, dict) else None
    if not isinstance(issue_number, int) or isinstance(issue_number, bool) or issue_number <= 0:
        raise CloseoutError("canonical task Issue mapping is missing or invalid")
    return issue_number


def field(body: str, name: str) -> str | None:
    prefix = f"- {name}: "
    values = [line[len(prefix):] for line in body.splitlines() if line.startswith(prefix)]
    if len(values) > 1:
        raise CloseoutError(f"review packet has duplicate {name} fields")
    return values[0] if values else None


def packet_identity_matches(body: str, task_uid: str, head: str, plan_path: str) -> bool:
    if f"- Review Plan: {plan_path}" not in body.splitlines():
        return False
    return (field(body, "Task UID") == task_uid and field(body, "Source Head") == head
            and field(body, "Review Plan") == plan_path)


def semantic_body(body: str) -> str:
    lines = body.splitlines()
    if lines and lines[0].startswith("## "):
        lines = lines[1:]
    return "\n".join(lines).rstrip("\n")


def verify_comment(comment: dict[str, Any], issue_number: int, expected_body: str,
                   expected_fields: dict[str, str]) -> dict[str, Any]:
    comment_id = comment.get("id")
    if not isinstance(comment_id, int) or isinstance(comment_id, bool) or comment_id <= 0:
        raise CloseoutError("review packet comment ID is invalid")
    direct = gh_json(["api", f"repos/{REPOSITORY}/issues/comments/{comment_id}"], "review packet direct readback")
    if not isinstance(direct, dict) or direct.get("id") != comment_id:
        raise CloseoutError("review packet direct readback identity mismatch")
    if direct.get("issue_url") != f"https://api.github.com/repos/{REPOSITORY}/issues/{issue_number}":
        raise CloseoutError("review packet is not attached to the canonical task Issue")
    body = direct.get("body")
    if not isinstance(body, str) or semantic_body(body) != semantic_body(expected_body):
        raise CloseoutError("live review packet differs from the exact unchanged v2 plan")
    for name, value in expected_fields.items():
        if field(body, name) != value:
            raise CloseoutError(f"review packet {name} does not match the exact plan")
    if field(body, "Pre-PR Local Role Review") != "passed":
        raise CloseoutError("matching review packet is not passed")
    user = direct.get("user")
    author = user.get("login") if isinstance(user, dict) else None
    if not isinstance(author, str) or not author:
        raise CloseoutError("review packet author is missing")
    permission = gh_json(["api", f"repos/{REPOSITORY}/collaborators/{author}/permission"],
                        "review packet author permission")
    if not isinstance(permission, dict) or permission.get("permission") != "admin":
        raise CloseoutError("review packet author is not a current repository admin")
    url = direct.get("html_url")
    if not isinstance(url, str) or not url:
        url = f"https://github.com/{REPOSITORY}/issues/{issue_number}#issuecomment-{comment_id}"
    return {"comment_id": comment_id, "comment_url": url, "author": author,
            "body_digest": hashlib.sha256(body.encode("utf-8")).hexdigest()}


def publish(root: Path, task_uid: str, plan_path: Path, head: str, packet: str) -> dict[str, Any]:
    root = root.resolve(strict=True)
    plan_path = plan_path.resolve(strict=True)
    try:
        plan_path.relative_to(root)
        plan = json.loads(plan_path.read_text(encoding="utf-8"))
    except (ValueError, OSError, json.JSONDecodeError) as exc:
        raise CloseoutError(f"review plan is unavailable inside the repository: {exc}") from exc
    if (not isinstance(plan, dict) or plan.get("schema") != "oasis7-review-plan/v2"
            or plan.get("task_uid") != task_uid or plan.get("frozen_head") != head):
        raise CloseoutError("idempotent packet publication requires the exact current v2 plan")
    roles = plan.get("roles")
    if not isinstance(roles, list) or not roles or not all(isinstance(role, str) and role for role in roles):
        raise CloseoutError("review plan role set is invalid")
    relative_plan = plan_path.relative_to(root).as_posix()
    expected_fields = {
        "Task UID": task_uid, "Source Head": head,
        "Comparison Ref": str(plan.get("comparison_ref", "")),
        "Comparison OID": str(plan.get("comparison_oid", "")),
        "Review Plan": relative_plan,
        "Review Plan Schema": "oasis7-review-plan/v2",
        "Review Evidence Digest": str(plan.get("source_review_digest", "")),
        "Review Roles": ",".join(roles),
    }
    if any(not value for value in expected_fields.values()):
        raise CloseoutError("review plan lacks fields required for idempotent packet identity")
    issue_number = issue_number_for(root, task_uid)
    if packet_identity_matches(packet, task_uid, head, relative_plan) is False:
        raise CloseoutError("generated packet identity does not match its immutable plan")
    for name, value in expected_fields.items():
        if field(packet, name) != value:
            raise CloseoutError(f"generated packet {name} does not match the immutable plan")
    if field(packet, "Pre-PR Local Role Review") != "passed":
        raise CloseoutError("generated packet is not passed")

    closeout = load_module(root, "review_closeout_publication_adapter", "review_closeout_publication.py")
    context = closeout.resolve_context(root, task_uid, plan)
    if int(context["issue_number"]) != issue_number:
        raise CloseoutError("resolved C1 Task Issue differs from canonical task mapping")

    def find_packet(comments: list[dict[str, Any]]) -> list[dict[str, Any]]:
        candidates = []
        for comment in comments:
            body = comment.get("body")
            if isinstance(body, str) and packet_identity_matches(body, task_uid, head, relative_plan):
                candidates.append(comment)
        return candidates

    semantic_digest = hashlib.sha256(semantic_body(packet).encode("utf-8")).hexdigest()
    expected_action = {
        "task_uid": task_uid, "head": head, "epoch": str(plan["epoch"]),
        "review_plan": relative_plan, "semantic_body_digest": semantic_digest,
    }
    result = closeout.publish_comment(
        context, action_id=f"review-packet:{plan['epoch']}", kind="publish_review_packet",
        expected=expected_action, body=packet, find_matches=find_packet,
        verify=lambda comment: verify_comment(comment, issue_number, packet, expected_fields),
    )
    return {"status": "already_complete" if result["status"] == "already_published" else "passed",
            "task_uid": task_uid, "head": head,
            **{key: value for key, value in result.items() if key != "status"}}


def main() -> int:
    try:
        import argparse
        parser = argparse.ArgumentParser(description=__doc__)
        parser.add_argument("--root", required=True)
        parser.add_argument("--task-uid", required=True)
        parser.add_argument("--review-plan", required=True)
        parser.add_argument("--head", required=True)
        args = parser.parse_args()
        result = publish(Path(args.root), args.task_uid, Path(args.review_plan), args.head, sys.stdin.read())
    except (CloseoutError, OSError, ValueError) as exc:
        print(f"review-closeout-complete: {exc}", file=sys.stderr)
        return 2
    print(json.dumps(result, ensure_ascii=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
