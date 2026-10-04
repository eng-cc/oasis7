#!/usr/bin/env python3
# Cross-platform maintenance: keep this helper compatible with POSIX and native Windows Python.
"""Recover one terminal task record from its registered canonical worktree."""
from __future__ import annotations

import argparse
import base64
import copy
import hashlib
import importlib.util
import json
import os
import pathlib
import re
import subprocess
import sys
from typing import Any


REQUIRED_TASK_FIELDS = (
    "task_uid", "status", "repository", "default_branch",
    "canonical_worktree", "task_branch", "pr_number", "pr_url",
)
TASK_IDENTITY_FIELDS = (
    "task_uid", "repository", "default_branch", "canonical_worktree",
    "task_branch", "pr_number", "pr_url",
)
REQUIRED_RECEIPT_FIELDS = (
    "receipt_type", "issuer", "evidence_mode", "repository",
    "default_branch", "pr_number", "pr_url", "state", "merged_at",
    "head_oid", "base_ref", "observed_at",
)


def fail(message: str) -> None:
    raise SystemExit(f"recover-terminal-task-mapping: {message}")


def run_git(root: pathlib.Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(root), *args], text=True, encoding="utf-8",
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )
    if result.returncode:
        fail(result.stderr.strip() or f"git {' '.join(args)} failed")
    return result.stdout.strip()


def normalized(path: str | pathlib.Path) -> str:
    return os.path.normcase(str(pathlib.Path(path).resolve(strict=False)))


def registered_worktrees(root: pathlib.Path) -> dict[str, dict[str, str]]:
    entries: dict[str, dict[str, str]] = {}
    current: dict[str, str] = {}
    for line in run_git(root, "worktree", "list", "--porcelain").splitlines() + [""]:
        if line.startswith("worktree "):
            if current:
                entries[normalized(current["path"])] = current
            current = {"path": line.removeprefix("worktree "), "branch": ""}
        elif line.startswith("branch refs/heads/") and current:
            current["branch"] = line.removeprefix("branch refs/heads/")
        elif not line and current:
            entries[normalized(current["path"])] = current
            current = {}
    if normalized(root) not in entries:
        fail("default worktree is not registered under its repository")
    return entries


def read_mapping(path: pathlib.Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        fail(f"cannot read valid task mapping {path}: {exc}")
    if not isinstance(value, dict) or not isinstance(value.get("tasks"), dict):
        fail(f"task mapping has invalid structure: {path}")
    return value


def validate_record(
    record: Any, *, task_uid: str, main_ref: str,
    receipt: dict[str, Any], registry: dict[str, dict[str, str]],
) -> dict[str, Any]:
    if not isinstance(record, dict):
        fail("terminal task record is not an object")
    for field in REQUIRED_TASK_FIELDS:
        if record.get(field) in (None, ""):
            fail(f"terminal task record is missing {field}")
    if record.get("task_uid") != task_uid:
        fail("terminal task record task_uid mismatch")
    if record.get("status") != "done":
        fail("terminal task record status must be done")
    canonical_key = normalized(str(record["canonical_worktree"]))
    registered = registry.get(canonical_key)
    if not registered:
        fail("canonical task worktree is not registered under the default repository")
    if not registered.get("branch") or registered["branch"] != record.get("task_branch"):
        fail("canonical task worktree branch identity mismatch")
    if record.get("default_branch") != main_ref:
        fail("terminal task default branch identity mismatch")
    for field in REQUIRED_RECEIPT_FIELDS:
        if receipt.get(field) in (None, ""):
            fail(f"merge receipt is missing {field}")
    expected_receipt = {
        "receipt_type": "oasis7_pr_merge",
        "issuer": "github_live_query",
        "evidence_mode": "production",
        "state": "MERGED",
        "repository": record["repository"],
        "default_branch": record["default_branch"],
        "pr_number": record["pr_number"],
        "pr_url": record["pr_url"],
        "base_ref": main_ref,
    }
    for field, expected in expected_receipt.items():
        if str(receipt.get(field)) != str(expected):
            fail(f"merge receipt {field} disagrees with terminal task identity")
    return record


def task_identity(record: dict[str, Any]) -> tuple[str, ...]:
    values = []
    for field in TASK_IDENTITY_FIELDS:
        value = normalized(str(record[field])) if field == "canonical_worktree" else str(record[field])
        values.append(value)
    return tuple(values)


def load_store(script_dir: pathlib.Path):
    path = script_dir / "workflow-durable-store.py"
    spec = importlib.util.spec_from_file_location("workflow_durable_store", path)
    if spec is None or spec.loader is None:
        fail("cannot load workflow durable store")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def import_recovered(store: Any, mapping_path: pathlib.Path, task_uid: str,
                     recovered: dict[str, Any]) -> None:
    def import_if_absent(mapping: dict[str, Any]) -> None:
        tasks = mapping.setdefault("tasks", {})
        if not isinstance(tasks, dict):
            fail("default task mapping has invalid tasks structure")
        if task_uid not in tasks:
            tasks[task_uid] = recovered
        elif tasks[task_uid] != recovered:
            fail("default task mapping changed to a conflicting task record during recovery")
    store.transact_json(mapping_path, import_if_absent, {"version": 1, "tasks": {}})

def reconcile_recovered(store: Any, mapping_path: pathlib.Path, task_uid: str,
                        previous: dict[str, Any] | None,
                        recovered: dict[str, Any]) -> None:
    def replace_if_unchanged(mapping: dict[str, Any]) -> None:
        tasks = mapping.setdefault("tasks", {})
        if not isinstance(tasks, dict):
            fail("default task mapping has invalid tasks structure")
        current = tasks.get(task_uid)
        if current != previous:
            fail("default task mapping changed during terminal identity recovery")
        tasks[task_uid] = recovered
    store.transact_json(mapping_path, replace_if_unchanged, {"version": 1, "tasks": {}})


def _unique_json_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            raise ValueError(f"duplicate JSON key: {key}")
        value[key] = item
    return value


def _read_json_object(path: pathlib.Path, label: str) -> tuple[dict[str, Any], bytes]:
    raw = path.read_bytes()
    try:
        value = json.loads(raw.decode("utf-8"), object_pairs_hook=_unique_json_object)
    except (UnicodeDecodeError, json.JSONDecodeError, ValueError) as exc:
        fail(f"{label} is not unique-key UTF-8 JSON: {exc}")
    if not isinstance(value, dict):
        fail(f"{label} is not a JSON object")
    return value, raw


def _issue_field(body: str, key: str, *, required: bool = True) -> str | None:
    matches = re.findall(rf"(?m)^-[ \t]+{re.escape(key)}[ \t]*:[ \t]*`([^`]+)`[ \t]*$", body)
    if len(matches) > 1 or (required and len(matches) != 1):
        fail(f"live Task Issue {key} field is missing or ambiguous")
    return matches[0] if matches else None


def _issue_claim_history(body: str) -> list[dict[str, Any]]:
    fields = re.findall(r"(?m)^-[ \t]+claim_verifications_b64[ \t]*:[ \t]*`([^`]+)`[ \t]*$", body)
    if len(fields) != 1:
        fail("live Task Issue claim_verifications_b64 field is missing or ambiguous")
    try:
        encoded = fields[0]
        decoded = base64.b64decode(encoded + "=" * (-len(encoded) % 4), altchars=b"-_", validate=True)
        value = json.loads(decoded.decode("utf-8"), object_pairs_hook=_unique_json_object)
    except (ValueError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        fail(f"live Task Issue claim history is malformed: {exc}")
    if not isinstance(value, list) or any(not isinstance(row, dict) for row in value):
        fail("live Task Issue claim history must be an array of objects")
    return value


def _v2_selector(record: dict[str, Any], task_uid: str) -> tuple[str, str, int, str] | None:
    type_map = record.get("phase_receipt_type")
    digest_map = record.get("phase_receipt_sha256")
    comment_ids = record.get("phase_receipt_comment_id")
    comment_digests = record.get("phase_receipt_comment_sha256")
    has_v2_marker = any(key in record for key in (
        "phase_receipt_type", "phase_receipt_comment_id", "phase_receipt_comment_sha256",
    ))
    if not has_v2_marker:
        return None
    if not all(isinstance(value, dict) for value in (type_map, digest_map, comment_ids, comment_digests)):
        fail("stored v2 delivery selector is incomplete or malformed")
    if type_map.get("post_merge_done") != "oasis7_terminal_delivery":
        fail("stored terminal version selector is unknown or mixed")
    digest_value = digest_map.get("post_merge_done")
    comment_id = comment_ids.get("post_merge_done")
    comment_digest = comment_digests.get("post_merge_done")
    if (not isinstance(digest_value, str) or not re.fullmatch(r"[0-9a-f]{64}", digest_value)
            or type(comment_id) is not int or comment_id < 1
            or not isinstance(comment_digest, str) or not re.fullmatch(r"[0-9a-f]{64}", comment_digest)):
        fail("stored v2 delivery selector digest/comment binding is malformed")
    if record.get("task_uid") not in (None, task_uid):
        fail("stored v2 delivery selector Task UID mismatch")
    return "oasis7_terminal_delivery", digest_value, comment_id, comment_digest


def recover_v2_from_live(root: pathlib.Path, mapping_path: pathlib.Path, task_uid: str,
                          mapping: dict[str, Any], existing: dict[str, Any] | None,
                          registry: dict[str, dict[str, str]]) -> str | None:
    """Rebuild only a previously selected v2 row from its live Issue and exact proof."""
    task_maps: list[dict[str, Any]] = []
    if isinstance(existing, dict):
        task_maps.append(existing)
    for entry in registry.values():
        source = pathlib.Path(entry["path"]) / ".pm/github-project-sync/tasks.json"
        if source.resolve(strict=False) == mapping_path or not source.is_file():
            continue
        candidate_mapping = read_mapping(source)
        candidate = (candidate_mapping.get("tasks") or {}).get(task_uid)
        if isinstance(candidate, dict):
            task_maps.append(candidate)
    selections = []
    for candidate in task_maps:
        selected = _v2_selector(candidate, task_uid)
        if selected is not None:
            selections.append((selected, candidate))
    if not selections:
        return None
    if len({selected for selected, _ in selections}) != 1:
        fail("conflicting v2 terminal selectors exist across task mappings")
    selector, seed = selections[0]
    if selector[0] != "oasis7_terminal_delivery":
        fail("unsupported terminal selector during v2 recovery")
    repository = str(seed.get("repository") or "")
    issue_number = seed.get("issue_number")
    if not re.fullmatch(r"[^/\s]+/[^/\s]+", repository) or type(issue_number) is not int or issue_number < 1:
        fail("stored v2 recovery seed lacks its repository/Issue identity")
    receipt_root_result = subprocess.run(
        [sys.executable, str(root / "scripts/pm/canonical-receipt-root.py"),
         "--default-worktree", str(root), "--task-uid", task_uid],
        text=True, capture_output=True,
    )
    if receipt_root_result.returncode:
        fail(receipt_root_result.stderr.strip() or "canonical v2 receipt root is unavailable")
    receipt_root = pathlib.Path(receipt_root_result.stdout.strip())
    receipt_path = receipt_root / "terminal-delivery-receipt.json"
    receipt, receipt_bytes = _read_json_object(receipt_path, "terminal delivery receipt")
    receipt_digest = hashlib.sha256(receipt_bytes).hexdigest()
    if receipt_digest != selector[1]:
        fail("stored v2 selector does not bind the exact delivery receipt bytes")
    if receipt.get("task_uid") != task_uid or receipt.get("repository") != repository or receipt.get("issue_number") != issue_number:
        fail("terminal delivery receipt disagrees with the stored recovery identity")
    pr_number = receipt.get("pr_number")
    pr_url = receipt.get("pr_url")
    if type(pr_number) is not int or pr_number < 1 or pr_url != f"https://github.com/{repository}/pull/{pr_number}":
        fail("terminal delivery receipt has an invalid PR identity")
    if seed.get("pr_number") not in (None, pr_number) or seed.get("pr_url") not in (None, pr_url):
        fail("stored v2 recovery seed conflicts with the selected PR identity")
    merge_receipt, merge_bytes = _read_json_object(receipt_root / "merge-receipt.json", "merge receipt")

    try:
        import loop_terminal
        import terminal_proof
        issue = loop_terminal.read_issue(repository, issue_number)
        if (issue.get("number") != issue_number
                or issue.get("html_url", issue.get("url")) != f"https://github.com/{repository}/issues/{issue_number}"):
            fail("live v2 recovery Issue identity mismatch")
        body = issue.get("body")
        if not isinstance(body, str) or re.findall(r"(?m)^task_uid:[ \t]*(task_[0-9a-f]{32})[ \t]*$", body) != [task_uid]:
            fail("live v2 recovery Issue Task UID is missing or ambiguous")
        if _issue_field(body, "status") != "done" or _issue_field(body, "workflow_phase") != "post_merge_done":
            fail("live v2 recovery Issue is not terminal")
        if _issue_field(body, "pr_number") != str(pr_number) or _issue_field(body, "pr_url") != pr_url:
            fail("live v2 recovery Issue PR binding disagrees with the selected receipt")
        project_item = loop_terminal.read_live_project_item(repository, issue_number)
        if seed.get("project_item_id") not in (None, project_item.get("id")):
            fail("stored v2 recovery seed conflicts with the live Project item identity")
        live_pr = loop_terminal.read_pull_request(repository, pr_number)
        comments = loop_terminal.read_comments(repository, issue_number)
        live_repository = terminal_proof.read_live_repository(
            repository, str(receipt.get("merge_commit_oid") or ""),
            str(receipt.get("observed_target_oid") or ""),
        )
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as exc:
        fail(f"v2 recovery live readback failed: {exc}")

    recovered = copy.deepcopy(seed)
    recovered.update({
        "task_uid": task_uid, "repository": repository, "issue_number": issue_number,
        "pr_number": pr_number, "pr_url": pr_url,
        "status": "done", "workflow_phase": "post_merge_done",
        "completion_mode": "pr_task", "project_item_id": project_item.get("id"),
        "default_branch": receipt.get("default_branch"),
        "canonical_worktree": receipt.get("worktree"), "task_branch": receipt.get("branch"),
        "merge_receipt": merge_receipt,
        "merge_receipt_sha256": hashlib.sha256(merge_bytes).hexdigest(),
        "claim_verifications": _issue_claim_history(body),
        "phase_receipts": {**(seed.get("phase_receipts") if isinstance(seed.get("phase_receipts"), dict) else {}),
                            "post_merge_done": receipt},
        "phase_receipt_type": {**seed["phase_receipt_type"], "post_merge_done": selector[0]},
        "phase_receipt_sha256": {**seed["phase_receipt_sha256"], "post_merge_done": selector[1]},
        "phase_receipt_comment_id": {**seed["phase_receipt_comment_id"], "post_merge_done": selector[2]},
        "phase_receipt_comment_sha256": {**seed["phase_receipt_comment_sha256"], "post_merge_done": selector[3]},
    })
    proof = loop_terminal.read_shared_terminal_proof(
        repository, task_uid, repo_root=root, record=recovered,
        live_issue=issue, live_project_item=project_item, live_pr=live_pr,
        live_repository=live_repository, comments=comments,
    )
    if proof.get("status") != "passed" or proof.get("protocol_version") != 2:
        fail("shared terminal proof did not validate selected v2 recovery")

    store = load_store(pathlib.Path(__file__).resolve().parent)
    if existing is not None:
        if existing == recovered:
            return "existing"
        reconcile_recovered(store, mapping_path, task_uid, existing, recovered)
        return "repaired"
    import_recovered(store, mapping_path, task_uid, recovered)
    return "imported"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--mapping", required=True)
    parser.add_argument("--task-uid", required=True)
    parser.add_argument("--main-ref")
    parser.add_argument("--pr-receipt")
    args = parser.parse_args()

    root = pathlib.Path(args.repo_root).resolve(strict=True)
    mapping_path = pathlib.Path(args.mapping).resolve(strict=True)
    registry = registered_worktrees(root)
    default = read_mapping(mapping_path)
    default_tasks = default.get("tasks") or {}
    if args.task_uid in default_tasks and not isinstance(default_tasks[args.task_uid], dict):
        fail("existing terminal task record is incomplete")
    existing = default_tasks.get(args.task_uid)
    recovered_v2 = recover_v2_from_live(
        root, mapping_path, args.task_uid, default, existing, registry,
    )
    if recovered_v2 is not None:
        print(recovered_v2)
        return 0
    if not args.main_ref or not args.pr_receipt:
        fail("no previously selected v2 proof exists; legacy recovery requires --main-ref and --pr-receipt")
    receipt_path = pathlib.Path(args.pr_receipt).resolve(strict=True)
    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    if not isinstance(receipt, dict):
        fail("merge receipt is not an object")
    if existing is not None:
        existing_canonical = normalized(str(existing.get("canonical_worktree") or ""))
        root_canonical = normalized(str(root))
        existing_branch = str(existing.get("task_branch") or "")
        # Preserve the established fast path for a task-bound destination.
        # Recovery is required only for the known poisoned signature where a
        # terminal task was rebound to the default root/default branch.
        if existing_canonical != root_canonical and existing_branch != args.main_ref:
            print("existing")
            return 0

    discovered: list[tuple[pathlib.Path, dict[str, Any]]] = []
    for entry in registry.values():
        candidate_path = pathlib.Path(entry["path"]) / ".pm/github-project-sync/tasks.json"
        if candidate_path.resolve(strict=False) == mapping_path or not candidate_path.is_file():
            continue
        candidate_mapping = read_mapping(candidate_path)
        candidate = (candidate_mapping.get("tasks") or {}).get(args.task_uid)
        if candidate is None:
            continue
        validated = validate_record(
            candidate, task_uid=args.task_uid, main_ref=args.main_ref,
            receipt=receipt, registry=registry,
        )
        discovered.append((candidate_path, validated))
    if not discovered:
        fail("terminal task is absent from all registered worktree mappings")

    identities = {task_identity(record) for _, record in discovered}
    if len(identities) != 1:
        fail("conflicting terminal task identities exist across registered worktrees")
    canonical_key = normalized(str(discovered[0][1]["canonical_worktree"]))
    authoritative = [record for path, record in discovered
                     if normalized(path.parent.parent.parent) == canonical_key]
    if len(authoritative) != 1:
        fail("terminal task record is not retained by exactly one canonical task worktree")
    recovered = copy.deepcopy(authoritative[0])

    store = load_store(pathlib.Path(__file__).resolve().parent)
    if existing is not None:
        try:
            validated_existing = validate_record(
                existing, task_uid=args.task_uid, main_ref=args.main_ref,
                receipt=receipt, registry=registry,
            )
        except SystemExit:
            validated_existing = None
        if validated_existing is not None and task_identity(validated_existing) == task_identity(recovered):
            print("existing")
            return 0
        reconcile_recovered(store, mapping_path, args.task_uid, existing, recovered)
        print("repaired")
    else:
        import_recovered(store, mapping_path, args.task_uid, recovered)
        print("imported")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
