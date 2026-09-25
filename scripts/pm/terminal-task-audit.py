#!/usr/bin/env python3
"""Audit one task's cross-sink terminal invariant; mutate only on explicit resume."""
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


def run_json(command: list[str]) -> dict:
    result = subprocess.run(command, text=True, capture_output=True)
    if result.returncode:
        return {"query_error": result.stderr.strip() or result.stdout.strip()}
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        return {"query_error": "invalid JSON response"}


def registered_worktrees(root: pathlib.Path) -> dict[str, str]:
    result = subprocess.run(
        ["git", "-C", str(root), "worktree", "list", "--porcelain"],
        text=True, capture_output=True, check=True,
    )
    entries: dict[str, str] = {}
    current = ""
    for line in result.stdout.splitlines():
        if line.startswith("worktree "):
            current = str(pathlib.Path(line.removeprefix("worktree ")).resolve())
            entries[current] = ""
        elif current and line.startswith("branch refs/heads/"):
            entries[current] = line.removeprefix("branch refs/heads/")
    return entries


def load(path: pathlib.Path) -> dict:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
        return value if isinstance(value, dict) else {}
    except (OSError, json.JSONDecodeError):
        return {}


def digest(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.is_file() else ""


def _canonical_digest(value: object, *, prefix: bool = False) -> str:
    result = hashlib.sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    ).hexdigest()
    return f"sha256:{result}" if prefix else result


def _body_field_values(body: str, key: str) -> list[str]:
    return re.findall(rf"(?m)^\s*(?:-\s*)?{re.escape(key)}:\s*([^\n]*)$", body)


def _body_field(body: str, key: str) -> str | None:
    values = _body_field_values(body, key)
    if len(values) > 1:
        return None
    if not values:
        return None
    return values[0].strip().strip("`").strip()


def _body_field_absent(body: str, key: str) -> bool:
    return not _body_field_values(body, key)


def _load_json(path: pathlib.Path) -> dict:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"JSON object required: {path}")
    return value


def _receipt_root(root: pathlib.Path, task_uid: str) -> pathlib.Path:
    result = subprocess.run(
        [sys.executable, str(root / "scripts/pm/canonical-receipt-root.py"),
         "--default-worktree", str(root), "--task-uid", task_uid],
        text=True, capture_output=True, check=True,
    )
    return pathlib.Path(result.stdout.strip())


def _read_project_items(root: pathlib.Path, item_ids: list[str]) -> dict[str, dict]:
    helper = root / "scripts/pm/github-project-workflow.py"
    spec = importlib.util.spec_from_file_location("terminal_audit_project", helper)
    if spec is None or spec.loader is None:
        raise RuntimeError("GitHub Project helper is unavailable")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.fetch_project_items_by_ids(item_ids)


def _resolve_aggregate_inputs(
    aggregate_plan: pathlib.Path | str | None,
    aggregate_candidate: pathlib.Path | str | None,
    aggregate_evidence: pathlib.Path | str | None,
    aggregate_receipt: pathlib.Path | str | None,
) -> tuple[pathlib.Path, pathlib.Path, pathlib.Path, pathlib.Path]:
    supplied = (aggregate_plan, aggregate_candidate, aggregate_evidence, aggregate_receipt)
    if not all(supplied):
        raise SystemExit(
            "terminal-task-audit: ordered aggregate route requires aggregate plan, candidate, evidence and receipt"
        )
    try:
        return tuple(pathlib.Path(str(item)).expanduser().resolve(strict=True) for item in supplied)  # type: ignore[return-value]
    except OSError as exc:
        raise SystemExit(f"terminal-task-audit: aggregate input is unavailable: {exc}") from exc


def _load_aggregate_module(root: pathlib.Path):
    helper = root / "scripts/pm/aggregate-task-completion.py"
    spec = importlib.util.spec_from_file_location("terminal_audit_aggregate", helper)
    if spec is None or spec.loader is None:
        raise RuntimeError("aggregate completion helper is unavailable")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _aggregate_completion_proof(
    root: pathlib.Path,
    task_uid: str,
    record: dict,
    issue: dict,
    paths: tuple[pathlib.Path, pathlib.Path, pathlib.Path, pathlib.Path],
) -> tuple[dict[str, bool], dict[str, object], str]:
    plan_path, candidate_path, evidence_path, receipt_path = paths
    details: dict[str, object] = {}
    checks = {
        "aggregate_completion_receipt_valid": False,
        "aggregate_plan_live": False,
        "aggregate_route_identity": False,
    }
    errors: list[str] = []
    completion_file_sha = digest(receipt_path)
    try:
        aggregate = _load_aggregate_module(root)
        plan = _load_json(plan_path)
        candidate = _load_json(candidate_path)
        evidence = json.loads(evidence_path.read_text(encoding="utf-8"))
        receipt = _load_json(receipt_path)
        aggregate.validate_plan(plan, task_uid)
        observed = aggregate._timestamp(receipt.get("observed_at"), "receipt observed_at")
        aggregate.validate_receipt(receipt, now=observed)
        candidate_sha, evidence_sha = aggregate._validate_candidate(plan, candidate, evidence)

        plan_body = aggregate.PLAN_MARKER + "\n" + aggregate.canonical_bytes(plan).decode("utf-8")
        plan_body_sha = aggregate.digest_bytes(plan_body.encode("utf-8"), prefix=True)
        expected_receipt_file_sha = digest(receipt_path)
        route_matches = (
            plan.get("repository") == record.get("repository")
            and plan.get("issue_number") == record.get("issue_number")
            and receipt.get("task_uid") == task_uid
            and receipt.get("repository") == record.get("repository")
            and receipt.get("issue_number") == record.get("issue_number")
            and receipt.get("plan_comment_id") == int(record.get("aggregate_plan_comment_id") or 0)
            and receipt.get("plan_body_sha256") == plan_body_sha
            and receipt.get("plan_sha256") == aggregate.canonical_digest(plan, prefix=True)
            and receipt.get("candidate_sha256") == candidate_sha
            and receipt.get("evidence_sha256") == evidence_sha
            and receipt.get("candidate_selection") == plan.get("candidate_selection")
            and record.get("aggregate_completion_receipt") == receipt
            and record.get("aggregate_completion_receipt_sha256") == expected_receipt_file_sha
        )
        delivery_keys = aggregate.DELIVERY_KEYS | aggregate.RECEIPT_DELIVERY_EXTRA
        deliveries_match = isinstance(receipt.get("deliveries"), list) and len(receipt["deliveries"]) == len(plan["required_deliveries"])
        if deliveries_match:
            for expected, actual in zip(plan["required_deliveries"], receipt["deliveries"]):
                if not isinstance(actual, dict) or set(actual) != delivery_keys:
                    deliveries_match = False
                    break
                if any(actual.get(key) != value for key, value in expected.items()):
                    deliveries_match = False
                    break
        checks["aggregate_completion_receipt_valid"] = bool(route_matches and deliveries_match)

        issue_body = str(issue.get("body") or "")
        issue_uids = re.findall(r"(?m)^task_uid:\s*(task_[0-9a-f]{32})\s*$", issue_body)
        comment_id = str(receipt.get("plan_comment_id") or "")
        comment = run_json(["gh", "api", f"repos/{record['repository']}/issues/comments/{comment_id}"])
        comment_is_exact = (
            not comment.get("query_error")
            and comment.get("id") == receipt.get("plan_comment_id")
            and comment.get("body") == plan_body
            and str(comment.get("issue_url") or "").rstrip("/").split("/")[-1]
                == str(record.get("issue_number"))
        )
        checks["aggregate_plan_live"] = bool(
            comment_is_exact
            and issue_uids == [task_uid]
            and _body_field(issue_body, "completion_mode") == "ordered_delivery_aggregate"
            and _body_field(issue_body, "aggregate_plan_comment_id") == comment_id
            and _body_field(issue_body, "aggregate_plan_sha256") == plan_body_sha
            and _body_field(issue_body, "aggregate_completion_receipt_sha256") == expected_receipt_file_sha
            and _body_field_absent(issue_body, "pr_number")
            and _body_field_absent(issue_body, "pr_url")
            and _body_field(issue_body, "aggregate_plan_comment_id")
                == str(record.get("aggregate_plan_comment_id") or "")
            and _body_field(issue_body, "aggregate_plan_sha256") == record.get("aggregate_plan_sha256")
        )
        checks["aggregate_route_identity"] = bool(
            record.get("completion_mode") == "ordered_delivery_aggregate"
            and not record.get("pr_number") and not record.get("pr_url")
            and plan.get("task_uid") == task_uid
        )
        details = {"plan": plan, "candidate": candidate, "evidence": evidence, "receipt": receipt}
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError, subprocess.SubprocessError, RuntimeError) as exc:
        errors.append(str(exc))
    details["errors"] = errors
    return checks, details, completion_file_sha


def _audit_aggregate(
    root: pathlib.Path,
    task_uid: str,
    mapping: dict,
    record: dict,
    aggregate_plan: pathlib.Path | str | None,
    aggregate_candidate: pathlib.Path | str | None,
    aggregate_evidence: pathlib.Path | str | None,
    aggregate_receipt: pathlib.Path | str | None,
) -> dict:
    paths = _resolve_aggregate_inputs(
        aggregate_plan, aggregate_candidate, aggregate_evidence, aggregate_receipt,
    )
    receipt_root = _receipt_root(root, task_uid)
    terminal_path = receipt_root / "aggregate-terminal-receipt.json"
    journal_path = receipt_root / "aggregate-terminal-effects.json"
    issue = run_json([
        "gh", "issue", "view", str(record.get("issue_number")), "-R", str(record.get("repository")),
        "--json", "number,url,state,body,projectItems",
    ])
    checks, details, completion_sha = _aggregate_completion_proof(root, task_uid, record, issue, paths)

    terminal = load(terminal_path)
    terminal_sha = digest(terminal_path)
    expected_terminal_keys = {
        "schema", "receipt_type", "issuer", "task_uid", "repository", "issue_number",
        "aggregate_completion_receipt_sha256", "plan_comment_id", "observed_at", "receipt_sha256",
    }
    terminal_payload = {key: value for key, value in terminal.items() if key != "receipt_sha256"}
    try:
        terminal_observed = dt.datetime.fromisoformat(str(terminal.get("observed_at") or "").replace("Z", "+00:00"))
        observed_valid = terminal_observed.tzinfo is not None
    except ValueError:
        observed_valid = False
    completion_receipt = details.get("receipt") if isinstance(details.get("receipt"), dict) else {}
    terminal_valid = (
        set(terminal) == expected_terminal_keys
        and terminal.get("schema") == "oasis7.aggregate-terminal/v1"
        and terminal.get("receipt_type") == "oasis7_aggregate_terminal"
        and terminal.get("issuer") == "aggregate-task-finalizer"
        and terminal.get("task_uid") == task_uid
        and terminal.get("repository") == record.get("repository")
        and terminal.get("issue_number") == record.get("issue_number")
        and terminal.get("aggregate_completion_receipt_sha256") == completion_sha
        and terminal.get("plan_comment_id") == completion_receipt.get("plan_comment_id")
        and observed_valid
        and terminal.get("receipt_sha256") == _canonical_digest(terminal_payload)
        and (record.get("phase_receipts") or {}).get("post_merge_done") == terminal
        and (record.get("phase_receipt_sha256") or {}).get("post_merge_done") == terminal_sha
    )
    checks["aggregate_terminal_receipt_valid"] = bool(terminal_valid)

    journal = load(journal_path)
    journal_keys = {
        "schema", "task_uid", "terminal_receipt_sha256", "phase_readback",
        "project_readback", "issue_closed_readback",
    }
    checks["aggregate_effect_journal_valid"] = bool(
        set(journal) == journal_keys
        and journal.get("schema") == "oasis7.aggregate-terminal-effects/v1"
        and journal.get("task_uid") == task_uid
        and journal.get("terminal_receipt_sha256") == terminal_sha
        and type(journal.get("phase_readback")) is bool and journal.get("phase_readback") is True
        and type(journal.get("project_readback")) is bool and journal.get("project_readback") is True
        and type(journal.get("issue_closed_readback")) is bool and journal.get("issue_closed_readback") is True
    )

    project_item_id = str(record.get("project_item_id") or "")
    project = mapping.get("project") or {}
    project_item: dict = {}
    project_error = False
    if project_item_id:
        try:
            project_item = _read_project_items(root, [project_item_id]).get(project_item_id) or {}
        except Exception:
            project_error = True
    content = project_item.get("content") or {}
    expected_issue_url = f"https://github.com/{record.get('repository')}/issues/{record.get('issue_number')}"
    project_item_bound = bool(project_item_id and str(project_item.get("id") or "") == project_item_id)
    project_identity = bool(
        not project_error and project_item_bound
        and project.get("owner") and project.get("number")
        and str(project.get("repo") or "") == str(record.get("repository") or "")
        and str(project_item.get("_project_number") or "") == str(project.get("number"))
        and str(project_item.get("_project_owner") or "") == str(project.get("owner"))
        and str(content.get("number") or "") == str(record.get("issue_number"))
        and str(content.get("url") or "") == expected_issue_url
        and re.findall(r"(?m)^task_uid:\s*(task_[0-9a-f]{32})\s*$", str(content.get("body") or "")) == [task_uid]
    )
    project_fields_complete = project_item.get("_field_values_has_next_page") is False
    terminal_phase = record.get("status") == "done" and record.get("workflow_phase") == "post_merge_done"
    expected_fields = {
        "Status": "Done" if terminal_phase else "In Progress",
        "PM Status": "done" if record.get("status") == "done" else str(record.get("status") or ""),
        "Workflow Phase": "done" if record.get("status") == "done" else str(record.get("workflow_phase") or ""),
    }
    project_terminal = bool(
        project_identity and project_fields_complete
        and all(project_item.get(key) == value for key, value in expected_fields.items())
    )
    issue_body = str(issue.get("body") or "")
    issue_uid_matches = re.findall(r"(?m)^task_uid:\s*(task_[0-9a-f]{32})\s*$", issue_body) == [task_uid]
    issue_closed = str(issue.get("state") or "").upper() == "CLOSED"
    issue_identity = bool(
        not issue.get("query_error") and issue_uid_matches
        and issue.get("number") == record.get("issue_number")
        and issue.get("url") == f"https://github.com/{record.get('repository')}/issues/{record.get('issue_number')}"
        and _body_field(issue_body, "completion_mode") == "ordered_delivery_aggregate"
        and _body_field_absent(issue_body, "pr_number") and _body_field_absent(issue_body, "pr_url")
    )
    checks.update({
        "mapping_post_merge_done": terminal_phase,
        "issue_closed": issue_closed and issue_identity,
        "project_item_bound": project_item_bound,
        "project_item_identity": project_identity,
        "project_field_values_complete": project_fields_complete,
        "project_terminal": project_terminal,
    })
    if issue.get("state") == "OPEN" and record.get("workflow_phase") == "task_done" and checks.get("aggregate_completion_receipt_valid"):
        validator = subprocess.run(
            [sys.executable, str(root / "scripts/pm/aggregate-task-completion.py"), "validate",
             "--repo-root", str(root), "--task-uid", task_uid,
             "--record", str(paths[0]), "--candidate", str(paths[1]),
             "--evidence", str(paths[2]), "--receipt", str(paths[3]), "--json"],
            text=True, capture_output=True,
        )
        checks["aggregate_live_completion"] = validator.returncode == 0

    drift = [name for name, ok in checks.items() if not ok]
    errors = details.get("errors") if isinstance(details.get("errors"), list) else []
    return {
        "schema": "oasis7_terminal_task_audit_v1",
        "task_uid": task_uid,
        "status": "reconciled" if not drift else "drifted",
        "checks": checks,
        "drift": drift,
        "aggregate_errors": errors,
        "task": {key: record.get(key) for key in (
            "repository", "issue_number", "status", "workflow_phase", "completion_mode", "project_item_id",
        )},
        "live": {"issue": issue, "project_item": project_item},
        "receipt_root": str(receipt_root),
    }


def aggregate_resume_command(
    root: pathlib.Path,
    task_uid: str,
    aggregate_plan: pathlib.Path,
    aggregate_candidate: pathlib.Path,
    aggregate_evidence: pathlib.Path,
    aggregate_receipt: pathlib.Path,
) -> list[str]:
    return [
        sys.executable, str(root / "scripts/pm/finalize-aggregate-task.py"),
        "--repo-root", str(root), "--task-uid", task_uid,
        "--record", str(aggregate_plan), "--candidate", str(aggregate_candidate),
        "--evidence", str(aggregate_evidence), "--receipt", str(aggregate_receipt), "--json",
    ]


def resume_aggregate_finalizer(
    root: pathlib.Path,
    task_uid: str,
    aggregate_plan: pathlib.Path,
    aggregate_candidate: pathlib.Path,
    aggregate_evidence: pathlib.Path,
    aggregate_receipt: pathlib.Path,
) -> subprocess.CompletedProcess:
    command = aggregate_resume_command(
        root, task_uid, aggregate_plan, aggregate_candidate, aggregate_evidence, aggregate_receipt,
    )
    resumed = subprocess.run(command, text=True, capture_output=True)
    if resumed.returncode:
        raise SystemExit(
            "terminal-task-audit: aggregate finalizer resume failed: "
            + (resumed.stderr.strip() or resumed.stdout.strip())
        )
    return resumed


def audit(
    root: pathlib.Path,
    task_uid: str,
    *,
    aggregate_plan: pathlib.Path | str | None = None,
    aggregate_candidate: pathlib.Path | str | None = None,
    aggregate_evidence: pathlib.Path | str | None = None,
    aggregate_receipt: pathlib.Path | str | None = None,
) -> dict:
    mapping_path = root / ".pm/github-project-sync/tasks.json"
    mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
    record = (mapping.get("tasks") or {}).get(task_uid)
    if not record:
        raise SystemExit(f"terminal-task-audit: unknown task UID: {task_uid}")
    aggregate_inputs = (aggregate_plan, aggregate_candidate, aggregate_evidence, aggregate_receipt)
    completion_mode = str(record.get("completion_mode") or "")
    if completion_mode == "ordered_delivery_aggregate":
        return _audit_aggregate(
            root, task_uid, mapping, record,
            aggregate_plan, aggregate_candidate, aggregate_evidence, aggregate_receipt,
        )
    if any(aggregate_inputs) or any(record.get(key) for key in (
        "aggregate_plan_comment_id", "aggregate_plan_sha256", "aggregate_completion_receipt",
        "aggregate_completion_receipt_sha256",
    )):
        raise SystemExit("terminal-task-audit: aggregate proof is mixed with non-aggregate task truth")
    if completion_mode not in {"", "pr_task"}:
        raise SystemExit(f"terminal-task-audit: unsupported terminal completion mode: {completion_mode}")
    if not record.get("pr_number"):
        raise SystemExit("terminal-task-audit: post_merge_done task lacks a recognized PR completion route")
    receipt_root_result = subprocess.run(
        [sys.executable, str(root / "scripts/pm/canonical-receipt-root.py"),
         "--default-worktree", str(root), "--task-uid", task_uid],
        text=True, capture_output=True, check=True,
    )
    receipt_root = pathlib.Path(receipt_root_result.stdout.strip())
    terminal = receipt_root / "terminal-cleanup-receipt.json"
    ledger = receipt_root / "finalizer-ledger.json"
    tombstone = receipt_root / "terminal-tombstone.json"
    worktree = str(pathlib.Path(str(record.get("canonical_worktree") or "")).resolve())
    branch = str(record.get("task_branch") or "")
    registrations = registered_worktrees(root)
    worktree_present = worktree in registrations
    branch_registered_elsewhere = any(
        checked_branch == branch and checked_path != worktree
        for checked_path, checked_branch in registrations.items()
    )
    local_branch_present = subprocess.run(
        ["git", "-C", str(root), "show-ref", "--verify", "--quiet", f"refs/heads/{branch}"]
    ).returncode == 0
    remote_branch = subprocess.run(
        ["git", "-C", str(root), "ls-remote", "--heads", "origin", f"refs/heads/{branch}"],
        text=True, capture_output=True,
    )
    issue = run_json(["gh", "issue", "view", str(record.get("issue_number")),
                      "-R", str(record.get("repository")), "--json", "state,projectItems"])
    pr = run_json(["gh", "pr", "view", str(record.get("pr_number")),
                   "-R", str(record.get("repository")), "--json", "state,mergedAt,headRefName"])
    terminal_data, ledger_data, tombstone_data = load(terminal), load(ledger), load(tombstone)
    expected_identity = {
        "task_uid": task_uid, "repository": record.get("repository"),
        "issue_number": record.get("issue_number"), "pr_number": record.get("pr_number"),
    }
    terminal_identity = (
        terminal_data.get("receipt_type") == "oasis7_terminal_cleanup"
        and terminal_data.get("issuer") == "post-merge-cleanup"
        and all(str(terminal_data.get(key)) == str(value) for key, value in expected_identity.items())
        and str(pathlib.Path(str(terminal_data.get("worktree") or "")).resolve()) == worktree
        and terminal_data.get("branch") == branch
        and terminal_data.get("merge_receipt_sha256") == record.get("merge_receipt_sha256")
        and terminal_data.get("main_sync_receipt_sha256") == (record.get("phase_receipt_sha256") or {}).get("main_sync")
        and terminal_data == (record.get("phase_receipts") or {}).get("post_merge_done")
        and digest(terminal) == (record.get("phase_receipt_sha256") or {}).get("post_merge_done")
    )
    operations = ledger_data.get("operations") or {}
    project_bound = bool(record.get("project_item_id"))
    ledger_effects = ["issue_close"] + (["project_update", "evidence_comment"] if project_bound else [])
    ledger_valid = (
        ledger_data.get("schema") == "oasis7_finalizer_ledger_v1"
        and ledger_data.get("task_uid") == task_uid
        and all(
            (operations.get(effect) or {}).get("operation_id")
                == hashlib.sha256(f"{task_uid}:post_merge_done:{effect}".encode()).hexdigest()
            and (operations.get(effect) or {}).get("effect") == effect
            and (operations.get(effect) or {}).get("intent") is True
            and (operations.get(effect) or {}).get("readback") is True
            and (operations.get(effect) or {}).get("committed") is True
            for effect in ledger_effects
        )
    )
    project_item_identity = not project_bound
    project_fields_complete = not project_bound
    project_item_bound = not project_bound
    project_done = not project_bound
    project_live: dict = {}
    if project_bound:
        helper = root / "scripts/pm/github-project-workflow.py"
        spec = importlib.util.spec_from_file_location("terminal_audit_project", helper)
        if spec and spec.loader:
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            project_live = module.fetch_project_items_by_ids([str(record["project_item_id"])]).get(
                str(record["project_item_id"])
            ) or {}
            project = mapping.get("project") or {}
            content = project_live.get("content") or {}
            expected_url = (
                f"https://github.com/{record.get('repository')}/issues/{record.get('issue_number')}"
            )
            project_item_bound = str(project_live.get("id") or "") == str(record.get("project_item_id"))
            project_item_identity = (
                project_item_bound
                and bool(project.get("number"))
                and bool(project.get("owner"))
                and str(project_live.get("_project_number") or "") == str(project.get("number"))
                and str(project_live.get("_project_owner") or "") == str(project.get("owner") or "")
                and str(project.get("repo") or "") == str(record.get("repository") or "")
                and str(content.get("number") or "") == str(record.get("issue_number"))
                and str(content.get("url") or "") == expected_url
                and bool(re.search(
                    rf"^task_uid:\s*{re.escape(task_uid)}$",
                    str(content.get("body") or ""),
                    re.MULTILINE,
                ))
            )
            # The Project helper returns this marker from the GraphQL pageInfo;
            # a truncated fieldValues page is not a terminal readback.
            project_fields_complete = project_live.get("_field_values_has_next_page") is False
            project_done = (
                project_item_identity
                and project_fields_complete
                and all(project_live.get(name) == value for name, value in {
                    "Status": "Done", "PM Status": "done", "Workflow Phase": "done",
                }.items())
            )
    checks = {
        "mapping_post_merge_done": record.get("workflow_phase") == "post_merge_done",
        "terminal_receipt_chain_valid": terminal_identity,
        "finalizer_ledger_committed": ledger_valid,
        "terminal_tombstone_valid": (
            tombstone_data.get("schema") == "oasis7_terminal_tombstone_v1"
            and tombstone_data.get("task_uid") == task_uid
            and all(str(tombstone_data.get(key)) == str(value) for key, value in expected_identity.items())
            and tombstone_data.get("canonical_worktree") == record.get("canonical_worktree")
            and tombstone_data.get("task_branch") == branch
            and tombstone_data.get("workflow_phase") == "post_merge_done"
            and tombstone_data.get("terminal_receipt_sha256") == digest(terminal)
            and tombstone_data.get("checkout_recreation_forbidden") is True
        ),
        "issue_closed": str(issue.get("state") or "").upper() == "CLOSED",
        "project_item_bound": project_item_bound,
        "project_item_identity": project_item_identity,
        "project_field_values_complete": project_fields_complete,
        "project_terminal": project_done,
        "pr_merged": str(pr.get("state") or "").upper() == "MERGED" and bool(pr.get("mergedAt"))
                     and pr.get("headRefName") == branch,
        "worktree_absent": not worktree_present and not pathlib.Path(worktree).exists(),
        "task_branch_not_registered_elsewhere": not branch_registered_elsewhere,
        "local_branch_absent": not local_branch_present,
        "remote_branch_absent": remote_branch.returncode == 0 and not remote_branch.stdout.strip(),
    }
    drift = [name for name, ok in checks.items() if not ok]
    return {
        "schema": "oasis7_terminal_task_audit_v1",
        "task_uid": task_uid,
        "status": "reconciled" if not drift else "drifted",
        "checks": checks,
        "drift": drift,
        "task": {key: record.get(key) for key in
                 ("repository", "issue_number", "pr_number", "status", "workflow_phase",
                  "canonical_worktree", "task_branch")},
        "live": {"issue": issue, "pr": pr, "project_item": project_live},
        "receipt_root": str(receipt_root),
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--task-uid", required=True)
    parser.add_argument("--repo-root", default=".")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--resume-finalizer", action="store_true")
    parser.add_argument("--aggregate-plan")
    parser.add_argument("--aggregate-candidate")
    parser.add_argument("--aggregate-evidence")
    parser.add_argument("--aggregate-receipt")
    args = parser.parse_args(argv)
    root = pathlib.Path(subprocess.check_output(
        ["git", "-C", args.repo_root, "rev-parse", "--show-toplevel"], text=True
    ).strip()).resolve()
    aggregate_inputs = (args.aggregate_plan, args.aggregate_candidate, args.aggregate_evidence, args.aggregate_receipt)
    result = audit(
        root, args.task_uid,
        aggregate_plan=args.aggregate_plan,
        aggregate_candidate=args.aggregate_candidate,
        aggregate_evidence=args.aggregate_evidence,
        aggregate_receipt=args.aggregate_receipt,
    )
    if args.resume_finalizer and result["status"] != "reconciled":
        if result["task"].get("completion_mode") == "ordered_delivery_aggregate":
            paths = _resolve_aggregate_inputs(*aggregate_inputs)
            resume_aggregate_finalizer(root, args.task_uid, *paths)
            result = audit(root, args.task_uid, aggregate_plan=paths[0], aggregate_candidate=paths[1],
                           aggregate_evidence=paths[2], aggregate_receipt=paths[3])
        else:
            pr_number = result["task"].get("pr_number")
            if not pr_number or not result["checks"]["pr_merged"]:
                raise SystemExit("terminal-task-audit: refusing resume without bound merged PR")
            subprocess.run(
                [str(root / "scripts/pm/finalize-task.sh"), "--repo-root", str(root),
                 "--task-uid", args.task_uid, "--pr", str(pr_number), "--resume", "--json"],
                check=True,
            )
            result = audit(root, args.task_uid)
    print(json.dumps(result, indent=2, sort_keys=True) if args.json else
          f"{result['task_uid']}: {result['status']} ({', '.join(result['drift']) or 'no drift'})")
    return 0 if result["status"] == "reconciled" else 1


if __name__ == "__main__":
    raise SystemExit(main())
