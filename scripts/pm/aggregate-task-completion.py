#!/usr/bin/env python3
"""Produce and revalidate linked-delivery aggregate task-complete receipts."""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import importlib.util
import json
import pathlib
import re
import subprocess
import sys
from typing import Any


PLAN_SCHEMA = "oasis7.aggregate-delivery-plan/v1"
RECEIPT_SCHEMA = "oasis7.aggregate-task-completion/v1"
PLAN_MARKER = "<!-- oasis7-aggregate-delivery-plan/v1 -->"
RECEIPT_TYPE = "oasis7_aggregate_task_complete"
REPOSITORY = "eng-cc/oasis7"
TASK_UID_RE = re.compile(r"^task_[0-9a-f]{32}$")
OID_RE = re.compile(r"^(?:[0-9a-f]{40}|[0-9a-f]{64})$")
SHA256_RE = re.compile(r"^(?:sha256:)?[0-9a-f]{64}$")
PLAN_KEYS = {
    "schema", "task_uid", "repository", "issue_number", "change_id",
    "required_deliveries", "candidate_selection",
}
DELIVERY_KEYS = {
    "ordinal", "obligation_id", "task_uid", "issue_number", "pr_number",
    "pr_url", "depends_on",
}
SELECTION_KEYS = {
    "candidate_sha256", "evidence_sha256", "integration_base_oid",
    "tested_tree_oid", "configuration_digest", "entry", "environment",
    "evidence_window",
}
RECEIPT_KEYS = {
    "schema", "receipt_type", "issuer", "evidence_mode", "status", "claim_type",
    "task_uid", "repository", "issue_number", "plan_comment_id", "plan_body_sha256",
    "plan_sha256", "candidate_sha256", "evidence_sha256", "candidate_selection",
    "effective_validator_commit", "observed_at", "deliveries", "receipt_sha256",
}
RECEIPT_DELIVERY_EXTRA = {
    "merge_commit_oid", "head_oid", "base_ref", "merged_at",
    "task_complete_claim_sha256", "merge_receipt_sha256", "main_sync_receipt_sha256",
    "terminal_receipt_sha256", "terminal_tombstone_sha256",
}
TERMINAL_CHECKS = (
    "mapping_post_merge_done", "terminal_receipt_chain_valid", "finalizer_ledger_committed",
    "terminal_tombstone_valid", "issue_closed", "project_item_bound", "project_item_identity",
    "project_field_values_complete", "project_terminal", "pr_merged", "worktree_absent",
    "task_branch_not_registered_elsewhere", "local_branch_absent", "remote_branch_absent",
)


class ReceiptError(ValueError):
    """Raised when a plan or live delivery proof is incomplete or inconsistent."""


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def digest_bytes(value: bytes, *, prefix: bool = False) -> str:
    result = hashlib.sha256(value).hexdigest()
    return f"sha256:{result}" if prefix else result


def canonical_digest(value: Any, *, prefix: bool = False) -> str:
    return digest_bytes(canonical_bytes(value), prefix=prefix)


def _exact_keys(value: Any, required: set[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ReceiptError(f"{label} must be a JSON object")
    keys = set(value)
    missing, extra = required - keys, keys - required
    if missing or extra:
        details = []
        if missing:
            details.append("missing " + ", ".join(sorted(missing)))
        if extra:
            details.append("unknown " + ", ".join(sorted(extra)))
        raise ReceiptError(f"{label} fields invalid: {'; '.join(details)}")
    if any(item is None for item in value.values()):
        raise ReceiptError(f"{label} contains null fields")
    return value


def _nonempty(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ReceiptError(f"{label} must be a non-empty string")
    return value


def _positive_int(value: Any, label: str) -> int:
    if type(value) is not int or value < 1:
        raise ReceiptError(f"{label} must be a positive integer")
    return value


def validate_plan(plan: Any, task_uid: str) -> dict[str, Any]:
    plan = _exact_keys(plan, PLAN_KEYS, "aggregate delivery plan")
    if plan["schema"] != PLAN_SCHEMA:
        raise ReceiptError("unsupported aggregate delivery plan schema")
    if not TASK_UID_RE.fullmatch(task_uid) or plan["task_uid"] != task_uid:
        raise ReceiptError("plan Task UID does not match requested coordinator")
    if plan["repository"] != REPOSITORY:
        raise ReceiptError("plan repository is not canonical")
    _positive_int(plan["issue_number"], "plan issue_number")
    _nonempty(plan["change_id"], "plan change_id")

    deliveries = plan["required_deliveries"]
    if not isinstance(deliveries, list) or len(deliveries) < 2:
        raise ReceiptError("aggregate plan requires at least two required deliveries")
    seen_ids: set[str] = set()
    seen_uids: set[str] = set()
    seen_issues: set[int] = set()
    seen_prs: set[int] = set()
    by_ordinal: dict[int, dict[str, Any]] = {}
    for index, raw in enumerate(deliveries, start=1):
        delivery = _exact_keys(raw, DELIVERY_KEYS, f"delivery {index}")
        ordinal = _positive_int(delivery["ordinal"], f"delivery {index} ordinal")
        if ordinal != index:
            raise ReceiptError("delivery ordinals must be unique, contiguous, and ordered")
        obligation_id = _nonempty(delivery["obligation_id"], f"delivery {index} obligation_id")
        child_uid = delivery["task_uid"]
        if not isinstance(child_uid, str) or not TASK_UID_RE.fullmatch(child_uid):
            raise ReceiptError(f"delivery {index} has invalid child Task UID")
        issue_number = _positive_int(delivery["issue_number"], f"delivery {index} issue_number")
        pr_number = _positive_int(delivery["pr_number"], f"delivery {index} pr_number")
        pr_url = _nonempty(delivery["pr_url"], f"delivery {index} pr_url")
        if pr_url != f"https://github.com/{REPOSITORY}/pull/{pr_number}":
            raise ReceiptError(f"delivery {index} PR URL is not canonical")
        depends_on = delivery["depends_on"]
        if not isinstance(depends_on, list) or any(not isinstance(item, str) for item in depends_on):
            raise ReceiptError(f"delivery {index} depends_on must be a string array")
        if len(depends_on) != len(set(depends_on)) or obligation_id in depends_on:
            raise ReceiptError(f"delivery {index} has duplicate or self dependency")
        if set(depends_on) - seen_ids:
            raise ReceiptError(f"delivery {index} dependency must name an earlier delivery")
        if obligation_id in seen_ids or child_uid in seen_uids or issue_number in seen_issues or pr_number in seen_prs:
            raise ReceiptError("aggregate delivery plan contains duplicate obligation, task, issue, or PR identity")
        if child_uid == task_uid or issue_number == plan["issue_number"]:
            raise ReceiptError("coordinator cannot also be a delivery task")
        seen_ids.add(obligation_id)
        seen_uids.add(child_uid)
        seen_issues.add(issue_number)
        seen_prs.add(pr_number)
        by_ordinal[ordinal] = delivery

    selection = _exact_keys(plan["candidate_selection"], SELECTION_KEYS, "candidate_selection")
    for key in ("candidate_sha256", "evidence_sha256"):
        if not isinstance(selection[key], str) or not re.fullmatch(r"[0-9a-f]{64}", selection[key]):
            raise ReceiptError(f"candidate_selection {key} must be a lowercase SHA-256 digest")
    for key in ("integration_base_oid", "tested_tree_oid"):
        if not isinstance(selection[key], str) or not OID_RE.fullmatch(selection[key]):
            raise ReceiptError(f"candidate_selection {key} is not a commit OID")
    if not isinstance(selection["configuration_digest"], str) or not SHA256_RE.fullmatch(selection["configuration_digest"]):
        raise ReceiptError("candidate_selection configuration_digest is invalid")
    _nonempty(selection["entry"], "candidate_selection entry")
    _nonempty(selection["environment"], "candidate_selection environment")
    window = _exact_keys(selection["evidence_window"], {"started_at", "ended_at"}, "evidence_window")
    start, end = _timestamp(window["started_at"], "evidence_window.started_at"), _timestamp(window["ended_at"], "evidence_window.ended_at")
    if end < start:
        raise ReceiptError("evidence_window is not ordered")
    return plan


def _timestamp(value: Any, label: str) -> dt.datetime:
    if not isinstance(value, str) or not value:
        raise ReceiptError(f"{label} must be an RFC3339 timestamp")
    try:
        parsed = dt.datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise ReceiptError(f"{label} must be an RFC3339 timestamp") from exc
    if parsed.tzinfo is None:
        raise ReceiptError(f"{label} must include a timezone")
    return parsed.astimezone(dt.timezone.utc)


def _parse_body_field(body: str, key: str) -> str | None:
    values = re.findall(rf"(?m)^\s*(?:-\s*)?{re.escape(key)}:\s*([^\n]*)$", body)
    if len(values) > 1:
        raise ReceiptError(f"coordinator Issue repeats {key}")
    if not values:
        return None
    return values[0].strip().strip("`").strip()


def _validate_coordinator(
    task_uid: str,
    plan: dict[str, Any],
    issue: dict[str, Any],
    comment: dict[str, Any],
) -> tuple[int, str]:
    if issue.get("number") != plan["issue_number"] or issue.get("repository") != plan["repository"]:
        raise ReceiptError("live coordinator Issue identity does not match plan")
    if str(issue.get("state") or "").upper() != "OPEN":
        raise ReceiptError("coordinator Issue must remain open until aggregate terminal finalization")
    body = issue.get("body")
    if not isinstance(body, str):
        raise ReceiptError("live coordinator Issue body is unavailable")
    task_values = re.findall(r"(?m)^task_uid:\s*(task_[0-9a-f]{32})\s*$", body)
    if task_values != [task_uid]:
        raise ReceiptError("coordinator Issue must declare exactly the requested Task UID")
    if _parse_body_field(body, "completion_mode") != "ordered_delivery_aggregate":
        raise ReceiptError("coordinator Issue does not select ordered_delivery_aggregate")
    if _parse_body_field(body, "pr_number") or _parse_body_field(body, "pr_url"):
        raise ReceiptError("aggregate coordinator cannot bind a singular PR")

    comment_id = _positive_int(comment.get("id"), "plan comment ID")
    if comment.get("issue_number") != plan["issue_number"]:
        raise ReceiptError("plan comment belongs to a different Issue")
    expected_body = PLAN_MARKER + "\n" + canonical_bytes(plan).decode("utf-8")
    if comment.get("body") != expected_body:
        raise ReceiptError("live plan comment body does not match canonical plan JSON")
    expected_pointer = str(comment_id)
    if _parse_body_field(body, "aggregate_plan_comment_id") != expected_pointer:
        raise ReceiptError("coordinator Issue plan comment ID does not match live plan comment")
    body_digest = digest_bytes(expected_body.encode("utf-8"), prefix=True)
    if _parse_body_field(body, "aggregate_plan_sha256") != body_digest:
        raise ReceiptError("coordinator Issue plan digest does not match live plan comment")
    if comment.get("permission") != "admin" or not _nonempty(comment.get("author"), "plan comment author"):
        raise ReceiptError("plan comment author lacks current repository admin authority")
    return comment_id, body_digest


def _validate_candidate(plan: dict[str, Any], candidate: Any, evidence: Any) -> tuple[str, str]:
    if not isinstance(candidate, dict):
        raise ReceiptError("aggregate candidate must be a JSON object")
    if not isinstance(evidence, list) or not evidence or any(not isinstance(row, dict) for row in evidence):
        raise ReceiptError("aggregate evidence must be a non-empty array of objects")
    selection = plan["candidate_selection"]
    candidate_sha = canonical_digest(candidate)
    evidence_sha = canonical_digest(evidence)
    if candidate_sha != selection["candidate_sha256"]:
        raise ReceiptError("aggregate candidate digest does not match frozen plan")
    if evidence_sha != selection["evidence_sha256"]:
        raise ReceiptError("aggregate evidence digest does not match frozen plan")
    if candidate.get("change_id") != plan["change_id"]:
        raise ReceiptError("aggregate candidate change_id does not match plan")
    for key in (
        "integration_base_oid", "tested_tree_oid", "configuration_digest",
        "entry", "environment", "evidence_window",
    ):
        if candidate.get(key) != selection[key]:
            raise ReceiptError(f"aggregate candidate {key} does not match frozen plan")
    return candidate_sha, evidence_sha


def _validate_child_report(delivery: dict[str, Any], report: dict[str, Any], default_branch: str) -> dict[str, Any]:
    if report.get("status") != "reconciled":
        raise ReceiptError(f"child {delivery['task_uid']} terminal audit is not reconciled")
    task = report.get("task")
    if not isinstance(task, dict):
        raise ReceiptError("child terminal audit omitted task identity")
    expected = {
        "task_uid": delivery["task_uid"], "repository": REPOSITORY,
        "issue_number": delivery["issue_number"], "pr_number": delivery["pr_number"],
        "pr_url": delivery["pr_url"], "status": "done", "workflow_phase": "post_merge_done",
    }
    if any(str(task.get(key) or "") != str(value) for key, value in expected.items()):
        raise ReceiptError(f"child task/PR identity mismatch for {delivery['task_uid']}")
    checks = report.get("checks")
    if not isinstance(checks, dict) or any(checks.get(key) is not True for key in TERMINAL_CHECKS):
        raise ReceiptError(f"child {delivery['task_uid']} terminal receipt chain is incomplete")
    claims = task.get("claim_verifications")
    if not isinstance(claims, list):
        raise ReceiptError(f"child {delivery['task_uid']} lacks task_complete claim history")
    verified_claims = [
        claim for claim in claims
        if isinstance(claim, dict) and claim.get("claim_type") == "task_complete"
        and claim.get("status") == "verified" and str(claim.get("verification_exit_code", "0")) == "0"
    ]
    if not verified_claims:
        raise ReceiptError(f"child {delivery['task_uid']} lacks verified task_complete evidence")

    live = report.get("live")
    if not isinstance(live, dict):
        raise ReceiptError("child terminal audit omitted live readback")
    issue, pr = live.get("issue"), live.get("pr")
    if not isinstance(issue, dict) or not isinstance(pr, dict):
        raise ReceiptError("child live Issue/PR readback is incomplete")
    if issue.get("number") != delivery["issue_number"] or str(issue.get("state") or "").upper() != "CLOSED":
        raise ReceiptError(f"child Issue {delivery['issue_number']} is not the expected closed Issue")
    if (pr.get("number") != delivery["pr_number"] or pr.get("url") != delivery["pr_url"]
            or str(pr.get("state") or "").upper() != "MERGED"
            or pr.get("base_ref") != default_branch):
        raise ReceiptError(f"child PR identity/base/merge state mismatch for #{delivery['pr_number']}")
    for key in ("merge_commit_oid", "head_oid", "merged_at"):
        if not pr.get(key):
            raise ReceiptError(f"child PR live readback omitted {key}")
    if not OID_RE.fullmatch(str(pr["merge_commit_oid"])) or not OID_RE.fullmatch(str(pr["head_oid"])):
        raise ReceiptError("child PR live readback has an invalid commit identity")

    proof = report.get("proof")
    if not isinstance(proof, dict):
        raise ReceiptError("child terminal audit omitted receipt digests")
    required_proofs = (
        "task_complete_claim_sha256", "merge_receipt_sha256", "main_sync_receipt_sha256",
        "terminal_receipt_sha256", "terminal_tombstone_sha256",
    )
    for key in required_proofs:
        if not isinstance(proof.get(key), str) or not SHA256_RE.fullmatch(proof[key]):
            raise ReceiptError(f"child receipt digest is missing or invalid: {key}")
    if proof["task_complete_claim_sha256"] != canonical_digest(verified_claims[-1], prefix=True):
        raise ReceiptError("child task_complete claim digest does not match task truth")
    return {
        **delivery,
        "merge_commit_oid": pr["merge_commit_oid"],
        "head_oid": pr["head_oid"],
        "base_ref": pr["base_ref"],
        "merged_at": pr["merged_at"],
        **{key: proof[key] for key in required_proofs},
    }


def build_receipt(
    *, task_uid: str, plan: Any, candidate: Any, evidence: Any,
    coordinator_issue: dict[str, Any], plan_comment: dict[str, Any],
    child_reports: dict[str, dict[str, Any]], effective_validator_commit: str,
    observed_at: str,
) -> dict[str, Any]:
    plan = validate_plan(plan, task_uid)
    if not OID_RE.fullmatch(effective_validator_commit):
        raise ReceiptError("effective validator commit is invalid")
    observed = _timestamp(observed_at, "observed_at")
    plan_comment_id, plan_body_sha = _validate_coordinator(task_uid, plan, coordinator_issue, plan_comment)
    candidate_sha, evidence_sha = _validate_candidate(plan, candidate, evidence)
    if set(child_reports) != {row["task_uid"] for row in plan["required_deliveries"]}:
        raise ReceiptError("live child report set does not exactly match required deliveries")
    default_branch = coordinator_issue.get("default_branch")
    if not isinstance(default_branch, str) or not default_branch:
        raise ReceiptError("live repository default branch is unavailable")
    deliveries = [
        _validate_child_report(delivery, child_reports[delivery["task_uid"]], default_branch)
        for delivery in plan["required_deliveries"]
    ]
    receipt = {
        "schema": RECEIPT_SCHEMA,
        "receipt_type": RECEIPT_TYPE,
        "issuer": "github_live_query",
        "evidence_mode": "production",
        "status": "verified",
        "claim_type": "task_complete",
        "task_uid": task_uid,
        "repository": plan["repository"],
        "issue_number": plan["issue_number"],
        "plan_comment_id": plan_comment_id,
        "plan_body_sha256": plan_body_sha,
        "plan_sha256": canonical_digest(plan, prefix=True),
        "candidate_sha256": candidate_sha,
        "evidence_sha256": evidence_sha,
        "candidate_selection": plan["candidate_selection"],
        "effective_validator_commit": effective_validator_commit,
        "observed_at": observed.isoformat().replace("+00:00", "Z"),
        "deliveries": deliveries,
    }
    receipt["receipt_sha256"] = canonical_digest(receipt, prefix=True)
    return receipt


def validate_receipt(receipt: Any, *, now: dt.datetime | None = None) -> dict[str, Any]:
    receipt = _exact_keys(receipt, RECEIPT_KEYS, "aggregate task-complete receipt")
    if receipt["schema"] != RECEIPT_SCHEMA or receipt["receipt_type"] != RECEIPT_TYPE:
        raise ReceiptError("unsupported aggregate task-complete receipt")
    fixed = {
        "issuer": "github_live_query", "evidence_mode": "production",
        "status": "verified", "claim_type": "task_complete",
    }
    if any(receipt.get(key) != value for key, value in fixed.items()):
        raise ReceiptError("aggregate task-complete receipt has an invalid authority/result")
    unsigned = {key: value for key, value in receipt.items() if key != "receipt_sha256"}
    if receipt["receipt_sha256"] != canonical_digest(unsigned, prefix=True):
        raise ReceiptError("aggregate task-complete receipt digest mismatch")
    if not TASK_UID_RE.fullmatch(str(receipt.get("task_uid") or "")):
        raise ReceiptError("aggregate task-complete receipt Task UID is invalid")
    for key in ("plan_body_sha256", "plan_sha256", "receipt_sha256"):
        if not isinstance(receipt.get(key), str) or not SHA256_RE.fullmatch(receipt[key]):
            raise ReceiptError(f"aggregate task-complete receipt digest is invalid: {key}")
    _timestamp(receipt.get("observed_at"), "receipt observed_at")
    deliveries = receipt.get("deliveries")
    if not isinstance(deliveries, list) or len(deliveries) < 2:
        raise ReceiptError("aggregate task-complete receipt requires multiple deliveries")
    expected_delivery_keys = DELIVERY_KEYS | RECEIPT_DELIVERY_EXTRA
    for index, delivery in enumerate(deliveries, start=1):
        _exact_keys(delivery, expected_delivery_keys, f"receipt delivery {index}")
        if delivery.get("ordinal") != index:
            raise ReceiptError("receipt delivery ordinals are not canonical")
    current = now or dt.datetime.now(dt.timezone.utc)
    age = (current - _timestamp(receipt["observed_at"], "receipt observed_at")).total_seconds()
    if age < -30 or age > 600:
        raise ReceiptError("aggregate task-complete receipt is stale")
    return receipt


def _run_json(command: list[str]) -> Any:
    result = subprocess.run(command, text=True, capture_output=True)
    if result.returncode:
        raise ReceiptError(result.stderr.strip() or result.stdout.strip() or "GitHub readback failed")
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise ReceiptError("GitHub readback returned invalid JSON") from exc


def _run_json_lines(command: list[str]) -> list[dict[str, Any]]:
    result = subprocess.run(command, text=True, capture_output=True)
    if result.returncode:
        raise ReceiptError(result.stderr.strip() or result.stdout.strip() or "GitHub pagination failed")
    try:
        values = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
    except json.JSONDecodeError as exc:
        raise ReceiptError("GitHub pagination returned invalid JSON") from exc
    if not all(isinstance(item, dict) for item in values):
        raise ReceiptError("GitHub pagination returned a non-object")
    return values


def _read_plan_pointer(body: str) -> tuple[str, str]:
    comment_id = _parse_body_field(body, "aggregate_plan_comment_id")
    digest = _parse_body_field(body, "aggregate_plan_sha256")
    if not comment_id or not comment_id.isdigit() or not digest:
        raise ReceiptError("coordinator Issue body lacks aggregate plan comment ID/digest")
    return comment_id, digest


def read_coordinator(repo_root: pathlib.Path, plan: dict[str, Any]) -> tuple[dict[str, Any], dict[str, Any]]:
    issue = _run_json(["gh", "api", f"repos/{REPOSITORY}/issues/{plan['issue_number']}"])
    if not isinstance(issue, dict):
        raise ReceiptError("coordinator Issue readback is not an object")
    comment_id, _ = _read_plan_pointer(str(issue.get("body") or ""))
    comments = _run_json_lines([
        "gh", "api", "--paginate", "--jq", ".[]",
        f"repos/{REPOSITORY}/issues/{plan['issue_number']}/comments",
    ])
    marker_comments = [item for item in comments if str(item.get("body") or "").startswith(PLAN_MARKER)]
    if len(marker_comments) != 1:
        raise ReceiptError("coordinator Issue must have exactly one aggregate delivery plan comment")
    comment = marker_comments[0]
    if str(comment.get("id") or "") != comment_id:
        raise ReceiptError("coordinator Issue pointer does not select the unique plan comment")
    author = str((comment.get("user") or {}).get("login") or "")
    if not author:
        raise ReceiptError("plan comment live server author is missing")
    permission = _run_json([
        "gh", "api", f"repos/{REPOSITORY}/collaborators/{author}/permission",
    ])
    comment["author"] = author
    comment["permission"] = str(permission.get("permission") or "")
    comment["issue_number"] = int(plan["issue_number"])
    repo = _run_json(["gh", "repo", "view", REPOSITORY, "--json", "defaultBranchRef"])
    branch = (repo.get("defaultBranchRef") or {}).get("name") if isinstance(repo, dict) else None
    if not branch:
        raise ReceiptError("live repository default branch readback is missing")
    issue["repository"] = REPOSITORY
    issue["default_branch"] = branch
    return issue, comment


def _load_mapping(repo_root: pathlib.Path) -> dict[str, Any]:
    try:
        mapping = json.loads((repo_root / ".pm/github-project-sync/tasks.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ReceiptError(f"task mapping is unavailable: {exc}") from exc
    if not isinstance(mapping, dict) or not isinstance(mapping.get("tasks"), dict):
        raise ReceiptError("task mapping shape is invalid")
    return mapping


def _import_terminal_audit(repo_root: pathlib.Path):
    path = repo_root / "scripts/pm/terminal-task-audit.py"
    spec = importlib.util.spec_from_file_location("aggregate_child_terminal_audit", path)
    if spec is None or spec.loader is None:
        raise ReceiptError("terminal task audit helper is unavailable")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def read_child_report(repo_root: pathlib.Path, delivery: dict[str, Any], default_branch: str) -> dict[str, Any]:
    mapping = _load_mapping(repo_root)
    task = (mapping.get("tasks") or {}).get(delivery["task_uid"])
    if not isinstance(task, dict):
        raise ReceiptError(f"child task {delivery['task_uid']} is absent from the selected task mapping")
    terminal_module = _import_terminal_audit(repo_root)
    report = terminal_module.audit(repo_root, delivery["task_uid"])
    if not isinstance(report, dict) or report.get("status") != "reconciled":
        raise ReceiptError(f"child task {delivery['task_uid']} terminal readback is not reconciled")

    live_issue = _run_json([
        "gh", "issue", "view", str(delivery["issue_number"]), "-R", REPOSITORY,
        "--json", "number,url,state,body",
    ])
    live_pr = _run_json([
        "gh", "pr", "view", str(delivery["pr_number"]), "-R", REPOSITORY,
        "--json", "number,url,state,mergedAt,mergeCommit,headRefOid,baseRefName,body",
    ])
    report_live = report.get("live") if isinstance(report.get("live"), dict) else {}
    audit_pr = report_live.get("pr") if isinstance(report_live.get("pr"), dict) else {}
    if str(live_issue.get("state") or "").upper() != "CLOSED":
        raise ReceiptError(f"child Issue #{delivery['issue_number']} is not closed")
    issue_uids = re.findall(r"(?m)^task_uid:\s*(task_[0-9a-f]{32})\s*$", str(live_issue.get("body") or ""))
    if issue_uids != [delivery["task_uid"]]:
        raise ReceiptError(f"child Issue #{delivery['issue_number']} Task UID mismatch")
    if task.get("task_uid") != delivery["task_uid"] or task.get("repository") != REPOSITORY:
        raise ReceiptError(f"child task mapping identity mismatch: {delivery['task_uid']}")
    for key in ("issue_number", "pr_number", "pr_url"):
        if str(task.get(key) or "") != str(delivery[key]):
            raise ReceiptError(f"child task mapping {key} mismatch: {delivery['task_uid']}")
    if task.get("status") != "done" or task.get("workflow_phase") != "post_merge_done":
        raise ReceiptError(f"child task {delivery['task_uid']} is not terminal")
    if task.get("completion_mode") == "ordered_delivery_aggregate":
        raise ReceiptError("aggregate coordinator cannot be used as a child delivery")
    if live_issue.get("number") != delivery["issue_number"] or live_issue.get("url") != f"https://github.com/{REPOSITORY}/issues/{delivery['issue_number']}":
        raise ReceiptError(f"child Issue URL/number mismatch: {delivery['task_uid']}")
    for key, expected in (("pr_number", delivery["pr_number"]), ("pr_url", delivery["pr_url"])):
        if _parse_body_field(str(live_issue.get("body") or ""), key) != str(expected):
            raise ReceiptError(f"child Issue {key} does not reciprocate the planned PR")
    if live_pr.get("number") != delivery["pr_number"] or live_pr.get("url") != delivery["pr_url"]:
        raise ReceiptError(f"child PR number/URL mismatch: #{delivery['pr_number']}")
    if str(live_pr.get("state") or "").upper() != "MERGED" or not live_pr.get("mergedAt"):
        raise ReceiptError(f"child PR #{delivery['pr_number']} is not merged")
    if live_pr.get("baseRefName") != default_branch:
        raise ReceiptError(f"child PR #{delivery['pr_number']} does not target the repository default branch")
    pr_body = str(live_pr.get("body") or "")
    reference = re.compile(rf"\brefs?\s+#\s*{delivery['issue_number']}\b", re.IGNORECASE)
    if not reference.search(pr_body):
        raise ReceiptError(f"child PR #{delivery['pr_number']} does not reciprocally reference its task Issue")
    if (audit_pr.get("state") != "MERGED" or not audit_pr.get("mergedAt")
            or audit_pr.get("headRefName") != task.get("task_branch")):
        raise ReceiptError(f"child PR #{delivery['pr_number']} terminal audit live readback disagrees")

    receipt_root = pathlib.Path(str(report.get("receipt_root") or ""))
    file_digests = {
        key: digest_bytes((receipt_root / filename).read_bytes(), prefix=True)
        for key, filename in (
            ("merge_receipt_sha256", "merge-receipt.json"),
            ("main_sync_receipt_sha256", "main-sync-receipt.json"),
            ("terminal_receipt_sha256", "terminal-cleanup-receipt.json"),
            ("terminal_tombstone_sha256", "terminal-tombstone.json"),
        )
    }
    claims = task.get("claim_verifications")
    if not isinstance(claims, list):
        raise ReceiptError(f"child {delivery['task_uid']} lacks task_complete claim history")
    verified_claims = [
        claim for claim in claims
        if isinstance(claim, dict) and claim.get("claim_type") == "task_complete"
        and claim.get("status") == "verified" and str(claim.get("verification_exit_code", "0")) == "0"
    ]
    if not verified_claims:
        raise ReceiptError(f"child {delivery['task_uid']} lacks verified task_complete evidence")
    proof = {"task_complete_claim_sha256": canonical_digest(verified_claims[-1], prefix=True), **file_digests}
    merge = json.loads((receipt_root / "merge-receipt.json").read_text(encoding="utf-8"))
    main_sync = json.loads((receipt_root / "main-sync-receipt.json").read_text(encoding="utf-8"))
    terminal = json.loads((receipt_root / "terminal-cleanup-receipt.json").read_text(encoding="utf-8"))
    tombstone = json.loads((receipt_root / "terminal-tombstone.json").read_text(encoding="utf-8"))
    record = dict(task)
    record["claim_verifications"] = claims
    normalized_live = {
        "issue": {
            "number": live_issue["number"], "url": live_issue["url"], "state": live_issue["state"],
            "body": live_issue["body"],
        },
        "pr": {
            "number": live_pr["number"], "url": live_pr["url"], "state": live_pr["state"],
            "base_ref": live_pr["baseRefName"], "head_oid": live_pr["headRefOid"],
            "merge_commit_oid": (live_pr.get("mergeCommit") or {}).get("oid"),
            "merged_at": live_pr["mergedAt"], "body": live_pr.get("body") or "",
        },
    }
    return {
        **report,
        "task": record,
        "live": normalized_live,
        "proof": proof,
    }


def _load_json(path: pathlib.Path, label: str) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ReceiptError(f"cannot read {label}: {exc}") from exc


def _git_head(root: pathlib.Path) -> str:
    result = subprocess.run(["git", "-C", str(root), "rev-parse", "HEAD"], text=True, capture_output=True)
    if result.returncode or not OID_RE.fullmatch(result.stdout.strip()):
        raise ReceiptError("effective validator commit cannot be established")
    return result.stdout.strip()


def collect_inputs(args: argparse.Namespace) -> dict[str, Any]:
    root = pathlib.Path(args.repo_root).resolve()
    plan = _load_json(pathlib.Path(args.record), "aggregate delivery plan")
    validate_plan(plan, args.task_uid)
    candidate = _load_json(pathlib.Path(args.candidate), "aggregate candidate")
    evidence = _load_json(pathlib.Path(args.evidence), "aggregate evidence")
    coordinator, comment = read_coordinator(root, plan)
    repo = _run_json(["gh", "repo", "view", REPOSITORY, "--json", "defaultBranchRef"])
    default_branch = (repo.get("defaultBranchRef") or {}).get("name") if isinstance(repo, dict) else None
    if not default_branch:
        raise ReceiptError("live repository default branch readback is missing")
    coordinator["default_branch"] = default_branch
    child_reports = {
        delivery["task_uid"]: read_child_report(root, delivery, default_branch)
        for delivery in plan["required_deliveries"]
    }
    return {
        "task_uid": args.task_uid, "plan": plan, "candidate": candidate, "evidence": evidence,
        "coordinator_issue": coordinator, "plan_comment": comment,
        "child_reports": child_reports, "effective_validator_commit": _git_head(root),
    }


def _write_receipt(path: pathlib.Path, receipt: dict[str, Any]) -> None:
    path = path.expanduser().resolve()
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    try:
        with path.open("x", encoding="utf-8", newline="\n") as stream:
            stream.write(payload)
            stream.flush()
    except FileExistsError as exc:
        raise ReceiptError("aggregate receipt already exists; validate the existing immutable receipt") from exc


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("create", "validate"):
        command = sub.add_parser(name)
        command.add_argument("--repo-root", required=True)
        command.add_argument("--task-uid", required=True)
        command.add_argument("--record", required=True)
        command.add_argument("--candidate", required=True)
        command.add_argument("--evidence", required=True)
        if name == "create":
            command.add_argument("--output", required=True)
        else:
            command.add_argument("--receipt", required=True)
        command.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    try:
        inputs = collect_inputs(args)
        if args.command == "create":
            receipt = build_receipt(**inputs, observed_at=dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z"))
            _write_receipt(pathlib.Path(args.output), receipt)
        else:
            existing = validate_receipt(_load_json(pathlib.Path(args.receipt), "aggregate receipt"))
            expected = build_receipt(**inputs, observed_at=existing["observed_at"])
            if existing != expected:
                raise ReceiptError("aggregate receipt disagrees with fresh coordinator/candidate/delivery readback")
            receipt = existing
    except (ReceiptError, OSError, KeyError, TypeError, subprocess.SubprocessError, json.JSONDecodeError) as exc:
        parser.exit(1, f"aggregate-task-completion: {exc}\n")
    if args.json:
        print(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True))
    else:
        print(f"aggregate-task-completion: {args.command} verified {args.task_uid}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
