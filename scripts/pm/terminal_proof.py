"""Canonical identity for repository-produced merged-terminal receipts."""
from __future__ import annotations

import hashlib
import json
import pathlib
import subprocess
import sys


def receipt_chain_digest(
    task_uid: str,
    repository: str,
    issue_number: int,
    pr_number: int,
    pr_url: str,
    merge_receipt_sha256: str,
    main_sync_receipt_sha256: str,
    terminal_receipt_sha256: str,
) -> str:
    """Bind task/PR identity and all producer receipt digests to one value."""
    payload = {
        "task_uid": task_uid,
        "repository": repository,
        "issue_number": issue_number,
        "pr_number": pr_number,
        "pr_url": pr_url,
        "merge_receipt_sha256": merge_receipt_sha256,
        "main_sync_receipt_sha256": main_sync_receipt_sha256,
        "terminal_receipt_sha256": terminal_receipt_sha256,
    }
    canonical = json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def _receipt(root: pathlib.Path, name: str) -> dict:
    path = root / name
    if not path.is_file():
        raise ValueError(f"canonical terminal receipt missing: {name}")
    raw = path.read_bytes()
    try:
        record = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ValueError(f"canonical terminal receipt is not valid JSON: {name}") from exc
    if not isinstance(record, dict):
        raise ValueError(f"canonical terminal receipt is not an object: {name}")
    return {"bytes": raw, "digest": hashlib.sha256(raw).hexdigest(), "record": record}


def read_receipt_chain(repo_root: pathlib.Path, task_uid: str) -> dict:
    """Read only the task UID's repository-owned canonical receipt root."""
    helper = pathlib.Path(__file__).with_name("canonical-receipt-root.py")
    try:
        raw = subprocess.check_output(
            [sys.executable, str(helper), "--default-worktree", str(repo_root),
             "--task-uid", task_uid, "--json"],
            text=True,
            stderr=subprocess.PIPE,
        )
        root = pathlib.Path(json.loads(raw)["receipt_root"])
    except (OSError, subprocess.SubprocessError, KeyError, TypeError, json.JSONDecodeError) as exc:
        raise ValueError("canonical terminal receipt root unavailable") from exc
    receipts = {
        "merge": _receipt(root, "merge-receipt.json"),
        "main_sync": _receipt(root, "main-sync-receipt.json"),
        "terminal": _receipt(root, "terminal-cleanup-receipt.json"),
        "ledger": _receipt(root, "finalizer-ledger.json"),
        "tombstone": _receipt(root, "terminal-tombstone.json"),
    }
    if receipts["main_sync"]["record"].get("integration_mode") == "patch_equivalence":
        receipts["patch_equivalence"] = _receipt(root, "patch-equivalence-receipt.json")
    return receipts


def validate_receipt_chain(
    receipts: dict,
    *,
    repository: str,
    task_uid: str,
    issue_number: int,
    pr_number: int,
    pr_url: str,
    pr: dict,
) -> dict[str, str]:
    """Validate producer schemas and return digests of their actual bytes."""
    required = ("merge", "main_sync", "terminal", "ledger", "tombstone")
    if any(not isinstance(receipts.get(name), dict) for name in required):
        raise ValueError("canonical terminal receipt chain is incomplete")
    merge = receipts["merge"]["record"]
    main_sync = receipts["main_sync"]["record"]
    terminal = receipts["terminal"]["record"]
    ledger = receipts["ledger"]["record"]
    tombstone = receipts["tombstone"]["record"]
    merge_digest = receipts["merge"]["digest"]
    main_sync_digest = receipts["main_sync"]["digest"]
    terminal_digest = receipts["terminal"]["digest"]
    for key, expected in {
        "receipt_type": "oasis7_pr_merge",
        "issuer": "github_live_query",
        "evidence_mode": "production",
        "repository": repository,
        "pr_number": pr_number,
        "pr_url": pr_url,
        "state": "MERGED",
    }.items():
        if merge.get(key) != expected:
            raise ValueError("canonical merge receipt provenance mismatch")
    if (not isinstance(merge.get("default_branch"), str) or not merge.get("default_branch")
            or not isinstance(merge.get("base_ref"), str) or not merge.get("base_ref")):
        raise ValueError("canonical merge receipt branch provenance missing")
    if (not merge.get("merged_at") or not merge.get("observed_at")
            or not re_fullmatch_oid(merge.get("head_oid"))):
        raise ValueError("canonical merge receipt lacks merged version")
    if merge.get("base_ref") != merge.get("default_branch"):
        raise ValueError("canonical merge receipt targets a non-default branch")
    live_base = ((pr.get("base") or {}).get("ref"))
    if live_base and live_base != merge.get("base_ref"):
        raise ValueError("canonical merge receipt base branch disagrees with live PR")
    live_head = ((pr.get("head") or {}).get("sha"))
    if live_head and live_head != merge.get("head_oid"):
        raise ValueError("canonical merge receipt head version disagrees with live PR")
    if (main_sync.get("receipt_type"), main_sync.get("issuer"), main_sync.get("task_uid"),
            main_sync.get("repository"), main_sync.get("default_branch"),
            main_sync.get("merge_receipt_sha256")) != (
            "oasis7_main_sync", "post-merge-main-sync", task_uid, repository,
            merge.get("default_branch"), merge_digest):
        raise ValueError("canonical main-sync receipt provenance mismatch")
    integration_mode = main_sync.get("integration_mode")
    if integration_mode not in ("ancestry", "patch_equivalence") or not main_sync.get("observed_at"):
        raise ValueError("canonical main-sync receipt integration mode is invalid")
    if not re_fullmatch_oid(main_sync.get("main_commit")) or not re_fullmatch_oid(main_sync.get("remote_main_commit")):
        raise ValueError("canonical main-sync receipt lacks synchronized producer commits")
    if main_sync.get("main_commit") != main_sync.get("remote_main_commit"):
        raise ValueError("canonical main-sync receipt local/remote commits disagree")
    patch_fields = (
        "patch_equivalence_receipt_sha256", "patch_id", "projected_tree_oid",
        "main_tree_oid", "integration_commit", "integration_parent",
    )
    if integration_mode == "ancestry":
        if any(field in main_sync for field in patch_fields):
            raise ValueError("canonical ancestry main-sync receipt contains patch-equivalence fields")
    else:
        if not re_fullmatch_sha256(main_sync.get("patch_equivalence_receipt_sha256")):
            raise ValueError("canonical patch-equivalence main-sync receipt lacks patch receipt digest")
        if not re_fullmatch_oid(main_sync.get("patch_id")):
            raise ValueError("canonical patch-equivalence main-sync receipt lacks patch identity")
        if not re_fullmatch_oid(main_sync.get("projected_tree_oid")) or not re_fullmatch_oid(main_sync.get("main_tree_oid")):
            raise ValueError("canonical patch-equivalence main-sync receipt lacks tree identity")
        if not re_fullmatch_oid(main_sync.get("integration_commit")) or not re_fullmatch_oid(main_sync.get("integration_parent")):
            raise ValueError("canonical patch-equivalence main-sync receipt lacks integration commit identity")
        if main_sync.get("projected_tree_oid") != main_sync.get("main_tree_oid"):
            raise ValueError("canonical patch-equivalence main-sync trees disagree")
        patch = receipts.get("patch_equivalence")
        if not isinstance(patch, dict) or not isinstance(patch.get("record"), dict):
            raise ValueError("canonical patch-equivalence receipt is unavailable")
        patch_record = patch["record"]
        if patch.get("digest") != main_sync.get("patch_equivalence_receipt_sha256"):
            raise ValueError("canonical patch-equivalence receipt digest disagrees with main-sync")
        expected_patch = {
            "receipt_type": "oasis7_patch_equivalence",
            "schema_version": 2,
            "issuer": "oasis7_patch_equivalence_helper",
            "branch_tip": merge.get("head_oid"),
            "main_commit": main_sync.get("integration_commit"),
            "main_parent": main_sync.get("integration_parent"),
            "patch_id": main_sync.get("patch_id"),
            "projected_tree_oid": main_sync.get("projected_tree_oid"),
            "main_tree_oid": main_sync.get("main_tree_oid"),
        }
        if any(patch_record.get(key) != value for key, value in expected_patch.items()):
            raise ValueError("canonical patch-equivalence receipt provenance mismatch")
    if (terminal.get("receipt_type"), terminal.get("issuer"), terminal.get("task_uid"),
            terminal.get("repository"), terminal.get("issue_number"), terminal.get("pr_number"),
            terminal.get("merge_receipt_sha256"), terminal.get("main_sync_receipt_sha256")) != (
            "oasis7_terminal_cleanup", "post-merge-cleanup", task_uid, repository,
            issue_number, pr_number, merge_digest, main_sync_digest):
        raise ValueError("canonical terminal receipt provenance mismatch")
    if not terminal.get("worktree") or not terminal.get("branch") or not terminal.get("observed_at"):
        raise ValueError("canonical terminal receipt lacks worktree/branch provenance")
    if ledger.get("schema") != "oasis7_finalizer_ledger_v1" or ledger.get("task_uid") != task_uid:
        raise ValueError("canonical finalizer ledger provenance mismatch")
    operations = ledger.get("operations") or {}
    for effect in ("project_update", "evidence_comment", "issue_close"):
        entry = operations.get(effect) or {}
        operation_id = hashlib.sha256(f"{task_uid}:post_merge_done:{effect}".encode()).hexdigest()
        if (entry.get("effect"), entry.get("operation_id"), entry.get("committed")) != (effect, operation_id, True):
            raise ValueError(f"canonical finalizer ledger lacks committed {effect}")
    if (tombstone.get("schema"), tombstone.get("task_uid"), tombstone.get("repository"),
            tombstone.get("issue_number"), tombstone.get("pr_number"),
            tombstone.get("workflow_phase"), tombstone.get("terminal_receipt_sha256"),
            tombstone.get("checkout_recreation_forbidden")) != (
            "oasis7_terminal_tombstone_v1", task_uid, repository, issue_number,
            pr_number, "post_merge_done", terminal_digest, True):
        raise ValueError("canonical terminal tombstone provenance mismatch")
    if (tombstone.get("canonical_worktree"), tombstone.get("task_branch")) != (
            terminal.get("worktree"), terminal.get("branch")):
        raise ValueError("canonical terminal tombstone checkout provenance mismatch")
    return {
        "merge": merge_digest,
        "main_sync": main_sync_digest,
        "terminal": terminal_digest,
    }


def re_fullmatch_oid(value: object) -> bool:
    return isinstance(value, str) and len(value) == 40 and all(c in "0123456789abcdef" for c in value)


def re_fullmatch_sha256(value: object) -> bool:
    return isinstance(value, str) and len(value) == 64 and all(c in "0123456789abcdef" for c in value)
