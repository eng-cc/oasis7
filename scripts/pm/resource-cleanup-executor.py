#!/usr/bin/env python3
"""Safely release one task's exact worktree and branch resources after delivery."""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys
import tempfile
import uuid
from typing import Any

from portable_file_lock import ensure_lock_byte, fcntl
from worktree_registration import RegistrationError, validate_worktree_registration

SCRIPT_DIR = pathlib.Path(__file__).resolve().parent
UID_RE = re.compile(r"^task_[0-9a-f]{32}$")
OID_RE = re.compile(r"^(?:[0-9a-f]{40}|[0-9a-f]{64})$")
SNAPSHOT_SCHEMA = "oasis7_resource_cleanup_v1"
JOURNAL_SCHEMA = "oasis7_resource_cleanup_journal_v1"
RESOURCE_KINDS = ("worktree", "local_branch", "remote_branch")


class CleanupError(RuntimeError):
    pass


def _now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat()


def _canonical(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def _run(args: list[str], *, cwd: pathlib.Path | None = None, check: bool = True) -> subprocess.CompletedProcess[str]:
    try:
        result = subprocess.run(args, cwd=cwd, text=True, capture_output=True)
    except OSError as exc:
        raise CleanupError(f"cannot execute {args[0]}: {exc}") from exc
    if check and result.returncode:
        detail = (result.stderr or result.stdout).strip()
        raise CleanupError(detail or f"command failed: {' '.join(args)}")
    return result


def _git(repo: pathlib.Path, *args: str, check: bool = True) -> str:
    return _run(["git", "-C", str(repo), *args], check=check).stdout.strip()


def _common_dir(repo: pathlib.Path) -> pathlib.Path:
    raw = pathlib.Path(_git(repo, "rev-parse", "--git-common-dir"))
    return (repo / raw if not raw.is_absolute() else raw).resolve()


def _receipt_root(repo: pathlib.Path, task_uid: str, *, create: bool) -> pathlib.Path:
    helper = SCRIPT_DIR / "canonical-receipt-root.py"
    command = [sys.executable, str(helper), "--default-worktree", str(repo), "--task-uid", task_uid, "--json"]
    if create:
        command.append("--create")
    try:
        result = _run(command)
        return pathlib.Path(json.loads(result.stdout)["receipt_root"]).resolve()
    except (CleanupError, KeyError, TypeError, json.JSONDecodeError) as exc:
        raise CleanupError(f"canonical receipt root is unavailable: {exc}") from exc


def _load_mapping(repo: pathlib.Path, task_uid: str) -> tuple[dict[str, Any], pathlib.Path]:
    path = repo / ".pm/github-project-sync/tasks.json"
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise CleanupError("canonical task mapping is unavailable or malformed") from exc
    records = data.get("tasks") if isinstance(data, dict) else None
    record = records.get(task_uid) if isinstance(records, dict) else None
    if not isinstance(record, dict) or record.get("task_uid") != task_uid:
        raise CleanupError("task UID is not uniquely bound in the canonical mapping")
    if record.get("status") != "done":
        raise CleanupError("task must be done before resource cleanup")
    return record, path


def _origin_repository(repo: pathlib.Path) -> str:
    origin = _git(repo, "remote", "get-url", "origin", check=False)
    match = re.fullmatch(
        r"(?:https?://|ssh://git@|git@)github\.com[:/]([^/\s?#]+/[^/\s?#]+?)(?:\.git)?/?",
        origin.strip(), re.IGNORECASE,
    )
    if not match:
        raise CleanupError("GitHub origin identity is unavailable or unsupported")
    return match.group(1)


def _run_delivery_preflight(repo: pathlib.Path, task_uid: str) -> dict[str, Any]:
    command = [sys.executable, str(SCRIPT_DIR / "post-merge-finalize.py"),
               "--repo-root", str(repo), "--task-uid", task_uid, "--delivery", "--preflight", "--json"]
    result = _run(command, check=False)
    try:
        payload = json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        detail = (result.stderr or result.stdout).strip()
        raise CleanupError(f"delivery readback did not return JSON: {detail or exc}") from exc
    if result.returncode or not isinstance(payload, dict):
        blockers = payload.get("blockers") if isinstance(payload, dict) else None
        raise CleanupError("delivery proof is not complete: " + "; ".join(map(str, blockers or ["producer preflight failed"])))
    delivery = payload.get("delivery")
    if not isinstance(delivery, dict) or delivery.get("state") != "complete" or delivery.get("protocol_version") != 2:
        raise CleanupError("delivery proof is not complete v2 evidence")
    return payload


def _read_delivery_receipt(root: pathlib.Path, task_uid: str, repository: str) -> dict[str, Any]:
    path = root / "terminal-delivery-receipt.json"
    try:
        receipt = json.loads(path.read_bytes())
    except (OSError, json.JSONDecodeError) as exc:
        raise CleanupError("canonical v2 delivery receipt is unavailable or malformed") from exc
    if not isinstance(receipt, dict):
        raise CleanupError("canonical v2 delivery receipt is not an object")
    if (receipt.get("receipt_type") != "oasis7_terminal_delivery" or receipt.get("schema_version") != 2
            or receipt.get("task_uid") != task_uid or receipt.get("repository") != repository
            or receipt.get("completion_semantics") != "delivery_only"):
        raise CleanupError("canonical v2 delivery receipt identity mismatch")
    if not isinstance(receipt.get("head_oid"), str) or not OID_RE.fullmatch(receipt["head_oid"]):
        raise CleanupError("canonical v2 delivery receipt has no valid delivered head")
    return receipt


def _worktree_identity(repo: pathlib.Path, record: dict[str, Any], receipt: dict[str, Any]) -> dict[str, Any]:
    path_raw = record.get("canonical_worktree")
    registration = record.get("worktree_registration")
    if not isinstance(path_raw, str) or not pathlib.Path(path_raw).is_absolute():
        raise CleanupError("task mapping has no canonical absolute worktree path")
    if not isinstance(registration, dict) or set(registration) != {"common_dir", "admin_dir", "instance_id"}:
        raise CleanupError("task worktree registration identity is missing; legacy resource is retained")
    path = pathlib.Path(path_raw).resolve()
    common = pathlib.Path(registration["common_dir"]).resolve()
    admin = pathlib.Path(registration["admin_dir"]).resolve()
    repo_common = _common_dir(repo)
    if path == repo.resolve():
        raise CleanupError("refusing to remove the default worktree")
    if common != repo_common:
        raise CleanupError("task worktree common directory disagrees with the canonical repository")
    if admin.parent != common / "worktrees":
        raise CleanupError("registered admin directory is outside this repository's exact worktree metadata slot")
    instance_id = registration.get("instance_id")
    try:
        parsed_instance = uuid.UUID(str(instance_id))
    except (ValueError, TypeError, AttributeError) as exc:
        raise CleanupError("task worktree instance identity is not a UUID") from exc
    if parsed_instance.version != 4 or str(parsed_instance) != instance_id:
        raise CleanupError("task worktree instance identity is not a canonical UUID")
    if str(path) != path_raw:
        raise CleanupError("mapped task worktree path is not canonical")
    if (str(common) != registration.get("common_dir") or str(admin) != registration.get("admin_dir")):
        raise CleanupError("mapped worktree registration paths are not canonical")
    if receipt.get("worktree") != str(path):
        raise CleanupError("v2 delivery receipt worktree disagrees with trusted task mapping")
    if receipt.get("branch") != record.get("task_branch"):
        raise CleanupError("v2 delivery receipt branch disagrees with trusted task mapping")
    return {
        "path": str(path), "common_dir": str(common), "admin_dir": str(admin),
        "instance_id": instance_id,
        "expected_head_oid": receipt["head_oid"],
    }


def _branch_identity(record: dict[str, Any], receipt: dict[str, Any], repository: str) -> tuple[dict[str, Any], dict[str, Any]]:
    branch = record.get("task_branch")
    if not isinstance(branch, str) or not branch or branch.startswith("-") or "/" not in branch:
        raise CleanupError("task branch identity is malformed")
    if _run(["git", "check-ref-format", f"refs/heads/{branch}"], check=False).returncode:
        raise CleanupError("task branch identity is not a valid Git ref")
    expected = receipt["head_oid"]
    if not OID_RE.fullmatch(expected):
        raise CleanupError("task branch expected OID is malformed")
    full_ref = f"refs/heads/{branch}"
    return ({"full_ref": full_ref, "expected_oid": expected},
            {"remote_repository": repository, "full_ref": full_ref, "expected_oid": expected})


def _worktree_rows(repo: pathlib.Path) -> list[dict[str, Any]]:
    raw = _git(repo, "worktree", "list", "--porcelain")
    rows: list[dict[str, Any]] = []
    current: dict[str, Any] | None = None
    for line in raw.splitlines() + [""]:
        if line.startswith("worktree "):
            if current:
                rows.append(current)
            current = {"path": str(pathlib.Path(line[len("worktree "):]).resolve())}
        elif line == "" and current:
            rows.append(current)
            current = None
        elif current and line.startswith("branch "):
            current["branch"] = line[len("branch "):]
        elif current and line == "locked":
            current["locked"] = True
        elif current and line.startswith("locked "):
            current["locked"] = True
            current["lock_reason"] = line[len("locked "):]
        elif current and line == "bare":
            current["bare"] = True
    return rows


def _mapping_references(repo: pathlib.Path, mapping: pathlib.Path, task_uid: str, target: pathlib.Path) -> bool:
    try:
        records = json.loads(mapping.read_text(encoding="utf-8")).get("tasks") or {}
    except (OSError, json.JSONDecodeError):
        raise CleanupError("cannot inspect task mappings for shared worktree use")
    for other_uid, other in records.items():
        if other_uid == task_uid or not isinstance(other, dict):
            continue
        value = other.get("canonical_worktree")
        if isinstance(value, str):
            try:
                if pathlib.Path(value).resolve() == target:
                    return True
            except OSError:
                continue
    return False


def _process_mentions_path(path: pathlib.Path) -> bool:
    # This is a positive active-use signal only. An empty scan is never treated
    # as proof that every external App or Agent session has stopped.
    result = _run(["ps", "-axo", "pid=,ppid=,command="], check=False)
    if result.returncode:
        raise CleanupError("process-use readback is unavailable")
    rows: list[tuple[int, int, str]] = []
    for line in result.stdout.splitlines():
        fields = line.strip().split(None, 2)
        if len(fields) != 3:
            continue
        try:
            rows.append((int(fields[0]), int(fields[1]), fields[2]))
        except ValueError:
            continue
    ancestors = {os.getpid()}
    parent = os.getppid()
    while parent > 1 and parent not in ancestors:
        ancestors.add(parent)
        next_parent = next((ppid for pid, ppid, _ in rows if pid == parent), 1)
        parent = next_parent
    for pid, _ppid, command in rows:
        if pid in ancestors:
            continue
        if str(path) in command:
            return True
    return False


def _inspect_ignored(worktree: pathlib.Path) -> list[str]:
    result = _run(["git", "-C", str(worktree), "ls-files", "--others", "--ignored", "--exclude-standard", "-z"])
    unknown: list[str] = []
    for raw in result.stdout.split("\0"):
        if not raw:
            continue
        entry = pathlib.PurePosixPath(raw.replace("\\", "/"))
        # `target` is the repository's explicit Cargo build cache. The worktree
        # removal unlinks a target symlink and never follows it.
        if entry.parts and entry.parts[0] == "target":
            target_root = worktree / "target"
            if target_root.is_symlink() or target_root.is_dir():
                continue
        unknown.append(raw)
    return unknown


def _worktree_state(repo: pathlib.Path, mapping: pathlib.Path, task_uid: str,
                    resource_id: dict[str, Any], branch: str, *, mutate: bool,
                    history: dict[str, Any] | None = None) -> dict[str, Any]:
    path = pathlib.Path(resource_id["path"])
    common = pathlib.Path(resource_id["common_dir"])
    admin = pathlib.Path(resource_id["admin_dir"])
    instance = resource_id["instance_id"]
    expected = resource_id["expected_head_oid"]
    marker = admin / "oasis7-instance-id"
    rows = _worktree_rows(repo)
    registered = next((item for item in rows if item["path"] == str(path)), None)
    if not path.exists() and registered is None and not admin.exists():
        return {"state": "already_absent", "operation": "none", "reason": "absent_before_operation",
                "readback": {"path_exists": False, "registered": False, "admin_dir_exists": False,
                             "instance_id": instance, "expected_head_oid": expected}}
    history = history or {"pending": set(), "states": {}}
    prior_state = history.get("states", {}).get("worktree")
    if "worktree" in history.get("pending", set()):
        return {"state": "retained", "operation": "none", "reason": "cleanup_operation_uncertain",
                "readback": {"path_exists": path.exists(), "registered": registered is not None,
                             "admin_dir_exists": admin.exists(), "expected_head_oid": expected}}
    if path.exists() and prior_state in {"removed", "already_absent", "failed"}:
        return {"state": "retained", "operation": "none", "reason": "path_reused_after_release_or_uncertain_cleanup",
                "readback": {"path_exists": True, "registered": registered is not None,
                             "admin_dir_exists": admin.exists(), "prior_state": prior_state,
                             "expected_head_oid": expected}}
    if registered is None or not path.exists() or not admin.is_dir():
        return {"state": "retained", "operation": "none", "reason": "registration_or_path_incomplete",
                "readback": {"path_exists": path.exists(), "registered": registered is not None,
                             "admin_dir_exists": admin.exists(), "expected_head_oid": expected}}
    if registered.get("locked"):
        return {"state": "retained", "operation": "none", "reason": "worktree_locked",
                "readback": {"path_exists": True, "registered": True, "locked": True,
                             "lock_reason": registered.get("lock_reason", ""), "expected_head_oid": expected}}
    try:
        admin_real = pathlib.Path(_git(path, "rev-parse", "--absolute-git-dir")).resolve()
        actual_common = _common_dir(path)
        token = marker.read_text(encoding="utf-8").strip()
    except (CleanupError, OSError, UnicodeDecodeError) as exc:
        return {"state": "retained", "operation": "none", "reason": "instance_identity_unreadable",
                "readback": {"path_exists": True, "registered": True, "detail": str(exc)}}
    if admin_real != admin or actual_common != common or token != instance:
        return {"state": "retained", "operation": "none", "reason": "path_reused_or_recreated",
                "readback": {"path_exists": True, "registered": True, "common_dir": str(actual_common),
                             "admin_dir": str(admin_real), "instance_id": token,
                             "expected_instance_id": instance, "expected_head_oid": expected}}
    try:
        validated_registration = validate_worktree_registration(path, {
            "canonical_worktree": str(path),
            "worktree_registration": {"common_dir": str(common), "admin_dir": str(admin),
                                       "instance_id": instance},
        })
    except RegistrationError as exc:
        return {"state": "retained", "operation": "none", "reason": "path_reused_or_recreated",
                "readback": {"path_exists": True, "registered": True, "detail": str(exc),
                             "expected_head_oid": expected}}
    if validated_registration != {"common_dir": str(common), "admin_dir": str(admin), "instance_id": instance}:
        return {"state": "retained", "operation": "none", "reason": "path_reused_or_recreated",
                "readback": {"path_exists": True, "registered": True, "expected_head_oid": expected}}
    if _mapping_references(repo, mapping, task_uid, path):
        return {"state": "retained", "operation": "none", "reason": "shared_task_worktree_reference",
                "readback": {"path_exists": True, "registered": True, "expected_head_oid": expected}}
    try:
        active_process = _process_mentions_path(path)
    except CleanupError as exc:
        return {"state": "retained", "operation": "none", "reason": "process_use_readback_unavailable",
                "readback": {"path_exists": True, "registered": True, "detail": str(exc)}}
    if active_process:
        return {"state": "retained", "operation": "none", "reason": "active_process",
                "readback": {"path_exists": True, "registered": True, "expected_head_oid": expected}}
    try:
        current_head = _git(path, "rev-parse", "HEAD")
        current_branch = _git(path, "symbolic-ref", "--quiet", "--short", "HEAD")
    except CleanupError as exc:
        return {"state": "retained", "operation": "none", "reason": "worktree_identity_unreadable",
                "readback": {"path_exists": True, "registered": True, "detail": str(exc)}}
    if current_head != expected or current_branch != branch:
        return {"state": "retained", "operation": "none", "reason": "worktree_head_or_branch_changed",
                "readback": {"path_exists": True, "registered": True, "head_oid": current_head,
                             "branch": current_branch, "expected_head_oid": expected}}
    if _git(path, "status", "--porcelain=v1", "--untracked-files=all"):
        return {"state": "retained", "operation": "none", "reason": "tracked_or_untracked_content",
                "readback": {"path_exists": True, "registered": True, "head_oid": current_head}}
    ignored = _inspect_ignored(path)
    if ignored:
        return {"state": "retained", "operation": "none", "reason": "protected_or_unknown_ignored_content",
                "readback": {"path_exists": True, "registered": True, "ignored_paths": ignored[:100],
                             "ignored_path_count": len(ignored), "head_oid": current_head}}
    if mutate:
        try:
            _run(["git", "-C", str(repo), "worktree", "remove", str(path)])
        except CleanupError as exc:
            return {"state": "failed", "operation": "git_worktree_remove", "reason": "worktree_remove_failed",
                    "readback": {"path_exists": path.exists(), "registered": True,
                                 "admin_dir_exists": admin.exists(), "detail": str(exc)}}
        rows_after = _worktree_rows(repo)
        registered_after = any(item["path"] == str(path) for item in rows_after)
        if path.exists() or registered_after or admin.exists():
            return {"state": "failed", "operation": "git_worktree_remove", "reason": "removal_readback_incomplete",
                    "readback": {"path_exists": path.exists(), "registered": registered_after,
                                 "admin_dir_exists": admin.exists(), "expected_head_oid": expected}}
        return {"state": "removed", "operation": "git_worktree_remove", "reason": "removed_and_verified_absent",
                "readback": {"path_exists": False, "registered": False, "admin_dir_exists": False,
                             "instance_id": instance, "expected_head_oid": expected}}
    return {"state": "ready", "operation": "git_worktree_remove", "reason": "identity_clean_and_unused",
            "readback": {"path_exists": True, "registered": True, "head_oid": current_head,
                         "admin_dir": str(admin), "instance_id": instance, "expected_head_oid": expected}}


def _local_branch_state(repo: pathlib.Path, resource_id: dict[str, Any], rows: list[dict[str, Any]], *, mutate: bool,
                        history: dict[str, Any] | None = None,
                        releasable_checkouts: set[str] | None = None) -> dict[str, Any]:
    full_ref, expected = resource_id["full_ref"], resource_id["expected_oid"]
    branch = full_ref.removeprefix("refs/heads/")
    current = _git(repo, "rev-parse", "--verify", f"{full_ref}^{{commit}}", check=False)
    if not current:
        return {"state": "already_absent", "operation": "none", "reason": "absent_before_operation",
                "readback": {"full_ref": full_ref, "exists": False, "expected_oid": expected}}
    history = history or {"pending": set(), "states": {}}
    prior_state = history.get("states", {}).get("local_branch")
    if "local_branch" in history.get("pending", set()) or prior_state in {"removed", "already_absent", "failed"}:
        return {"state": "retained", "operation": "none", "reason": "branch_ref_reused_after_release_or_uncertain_cleanup",
                "readback": {"full_ref": full_ref, "exists": True, "observed_oid": current,
                             "prior_state": prior_state, "expected_oid": expected}}
    if current != expected:
        return {"state": "retained", "operation": "none", "reason": "branch_tip_changed",
                "readback": {"full_ref": full_ref, "exists": True, "observed_oid": current, "expected_oid": expected}}
    users = [row["path"] for row in rows if row.get("branch") == full_ref]
    if users:
        releasable_checkouts = releasable_checkouts or set()
        if not mutate and len(users) == 1 and users[0] in releasable_checkouts:
            return {"state": "ready", "operation": "git_update_ref_cas",
                    "reason": "exact_tip_after_worktree_release", "depends_on": ["worktree"],
                    "readback": {"full_ref": full_ref, "exists": True,
                                 "observed_oid": current, "expected_oid": expected,
                                 "checked_out_in": users,
                                 "worktree_release_required": True}}
        return {"state": "retained", "operation": "none", "reason": "branch_checked_out",
                "readback": {"full_ref": full_ref, "exists": True, "checked_out_in": users, "expected_oid": expected}}
    if mutate:
        # Refresh checkout ownership immediately before the OID compare-and-
        # delete. The ref update itself remains an exact old-OID CAS.
        refreshed_users = [row["path"] for row in _worktree_rows(repo) if row.get("branch") == full_ref]
        if refreshed_users:
            return {"state": "retained", "operation": "none", "reason": "branch_checked_out",
                    "readback": {"full_ref": full_ref, "exists": True,
                                 "checked_out_in": refreshed_users, "expected_oid": expected}}
        result = _run(["git", "-C", str(repo), "update-ref", "-d", full_ref, expected], check=False)
        current_after = _git(repo, "rev-parse", "--verify", f"{full_ref}^{{commit}}", check=False)
        if current_after:
            if current_after != expected or result.returncode == 0:
                return {"state": "retained", "operation": "git_update_ref_cas",
                        "reason": "branch_tip_changed_during_delete" if current_after != expected else "branch_ref_recreated_after_delete",
                        "readback": {"full_ref": full_ref, "exists": True,
                                     "observed_oid": current_after, "expected_oid": expected,
                                     "delete_returncode": result.returncode,
                                     "stderr": result.stderr.strip()}}
            return {"state": "failed", "operation": "git_update_ref_cas", "reason": "branch_delete_failed_ref_unchanged",
                    "readback": {"full_ref": full_ref, "exists": True, "observed_oid": current_after,
                                 "expected_oid": expected, "stderr": result.stderr.strip()}}
        if result.returncode:
            return {"state": "failed", "operation": "git_update_ref_cas", "reason": "branch_disappeared_during_delete",
                    "readback": {"full_ref": full_ref, "exists": False, "expected_oid": expected,
                                 "stderr": result.stderr.strip()}}
        return {"state": "removed", "operation": "git_update_ref_cas", "reason": "removed_and_verified_absent",
                "readback": {"full_ref": full_ref, "exists": False, "expected_oid": expected}}
    return {"state": "ready", "operation": "git_update_ref_cas", "reason": "exact_tip_and_unused",
            "readback": {"full_ref": full_ref, "exists": True, "observed_oid": current, "expected_oid": expected}}


def _remote_branch_state(repo: pathlib.Path, resource_id: dict[str, Any], *, mutate: bool,
                         history: dict[str, Any] | None = None) -> dict[str, Any]:
    full_ref, expected = resource_id["full_ref"], resource_id["expected_oid"]
    line = _run(["git", "-C", str(repo), "ls-remote", "--heads", "origin", full_ref], check=False)
    if line.returncode:
        return {"state": "failed", "operation": "git_ls_remote", "reason": "remote_readback_failed",
                "readback": {"full_ref": full_ref, "readable": False, "stderr": line.stderr.strip(), "expected_oid": expected}}
    observed = line.stdout.split()[0] if line.stdout.split() else ""
    if not observed:
        return {"state": "already_absent", "operation": "none", "reason": "absent_before_operation",
                "readback": {"full_ref": full_ref, "exists": False, "expected_oid": expected}}
    history = history or {"pending": set(), "states": {}}
    prior_state = history.get("states", {}).get("remote_branch")
    if "remote_branch" in history.get("pending", set()) or prior_state in {"removed", "already_absent", "failed"}:
        return {"state": "retained", "operation": "none", "reason": "remote_ref_reused_after_release_or_uncertain_cleanup",
                "readback": {"full_ref": full_ref, "exists": True, "observed_oid": observed,
                             "prior_state": prior_state, "expected_oid": expected}}
    if observed != expected:
        return {"state": "retained", "operation": "none", "reason": "branch_tip_changed",
                "readback": {"full_ref": full_ref, "exists": True, "observed_oid": observed, "expected_oid": expected}}
    if mutate:
        result = _run(["git", "-C", str(repo), "push", f"--force-with-lease={full_ref}:{expected}",
                       "origin", f":{full_ref}"], check=False)
        readback = _run(["git", "-C", str(repo), "ls-remote", "--heads", "origin", full_ref], check=False)
        if readback.returncode:
            return {"state": "failed", "operation": "git_push_delete_with_lease", "reason": "remote_readback_failed_after_delete",
                    "readback": {"full_ref": full_ref, "readable": False, "expected_oid": expected}}
        after = readback.stdout.split()[0] if readback.stdout.split() else ""
        if after:
            state = "retained" if after != expected else "failed"
            reason = "branch_tip_changed_during_delete" if after != expected else "remote_delete_failed"
            return {"state": state, "operation": "git_push_delete_with_lease", "reason": reason,
                    "readback": {"full_ref": full_ref, "exists": True, "observed_oid": after,
                                 "expected_oid": expected, "stderr": result.stderr.strip()}}
        return {"state": "removed", "operation": "git_push_delete_with_lease", "reason": "removed_and_verified_absent",
                "readback": {"full_ref": full_ref, "exists": False, "expected_oid": expected,
                             "push_returncode": result.returncode}}
    return {"state": "ready", "operation": "git_push_delete_with_lease", "reason": "exact_tip_readback",
            "readback": {"full_ref": full_ref, "exists": True, "observed_oid": observed, "expected_oid": expected}}


def _append_journal(path: pathlib.Path, event: dict[str, Any]) -> None:
    payload = _canonical(event) + b"\n"
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
    try:
        with os.fdopen(fd, "ab", closefd=False) as stream:
            stream.write(payload)
            stream.flush()
            os.fsync(stream.fileno())
    finally:
        os.close(fd)


def _atomic_write(path: pathlib.Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(tmp, path)
        if os.name != "nt":
            directory_fd = os.open(path.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
            try:
                os.fsync(directory_fd)
            finally:
                os.close(directory_fd)
    finally:
        pathlib.Path(tmp).unlink(missing_ok=True)


def _journal_revision(path: pathlib.Path, task_uid: str, repository: str) -> int:
    if not path.exists():
        return 0
    last = 0
    last_event = ""
    for number, raw in enumerate(path.read_bytes().splitlines(), 1):
        try:
            entry = json.loads(raw, object_pairs_hook=_unique_object)
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            raise CleanupError(f"cleanup journal line {number} is malformed") from exc
        if not isinstance(entry, dict) or entry.get("schema") != JOURNAL_SCHEMA:
            raise CleanupError("cleanup journal has an unknown event schema")
        if entry.get("task_uid") != task_uid or entry.get("repository") != repository:
            raise CleanupError("cleanup journal task or repository identity conflict")
        revision = entry.get("revision")
        if not isinstance(revision, int) or revision < last:
            raise CleanupError("cleanup journal revision is malformed or regressed")
        if revision == last and last_event == "record":
            raise CleanupError("cleanup journal contains an event after a committed snapshot")
        event = entry.get("event")
        expected_keys = ({"schema", "task_uid", "repository", "revision", "event", "kind", "resource_id", "operation", "operation_id", "observed_at"}
                         if event == "intent" else
                         {"schema", "task_uid", "repository", "revision", "event", "record_sha256", "observed_at"}
                         if event == "record" else set())
        if not expected_keys or set(entry) != expected_keys:
            raise CleanupError(f"cleanup journal line {number} has unknown or missing fields")
        if event == "intent":
            if (entry.get("kind") not in RESOURCE_KINDS or not isinstance(entry.get("resource_id"), dict)
                    or not isinstance(entry.get("operation"), str) or not isinstance(entry.get("operation_id"), str)
                    or not re.fullmatch(r"[0-9a-f]{64}", entry["operation_id"])):
                raise CleanupError(f"cleanup journal line {number} has a malformed intent")
        elif not re.fullmatch(r"[0-9a-f]{64}", str(entry.get("record_sha256") or "")):
            raise CleanupError(f"cleanup journal line {number} has a malformed snapshot digest")
        last = revision
        last_event = str(event)
    return last


def _event(task_uid: str, repository: str, revision: int, event: str, **fields: Any) -> dict[str, Any]:
    return {"schema": JOURNAL_SCHEMA, "task_uid": task_uid, "repository": repository,
            "revision": revision, "event": event, **fields, "observed_at": _now()}


def read_cleanup_record(receipt_root: pathlib.Path, task_uid: str, repository: str,
                        kind: str, resource_id: dict[str, Any]) -> dict[str, Any]:
    """Validate the helper-authored latest snapshot and its exact-byte journal link."""
    if kind not in RESOURCE_KINDS:
        raise ValueError("unsupported cleanup resource kind")
    snapshot_path = receipt_root / "resource-cleanup.json"
    journal_path = receipt_root / "resource-cleanup-journal.jsonl"
    raw = snapshot_path.read_bytes()
    snapshot = json.loads(raw, object_pairs_hook=_unique_object)
    if (not isinstance(snapshot, dict) or set(snapshot) != {"schema", "task_uid", "repository", "revision", "observed_at", "resources"}
            or snapshot.get("schema") != SNAPSHOT_SCHEMA
            or snapshot.get("task_uid") != task_uid or snapshot.get("repository") != repository):
        raise ValueError("resource cleanup snapshot identity mismatch")
    revision = snapshot.get("revision")
    if not isinstance(revision, int) or revision < 1:
        raise ValueError("resource cleanup snapshot revision is invalid")
    if not isinstance(snapshot.get("resources"), list) or len(snapshot["resources"]) != len(RESOURCE_KINDS):
        raise ValueError("resource cleanup snapshot resource set is malformed")
    rows = snapshot["resources"]
    if [row.get("kind") for row in rows if isinstance(row, dict)].__len__() != len(rows):
        raise ValueError("resource cleanup snapshot contains a malformed row")
    if {row.get("kind") for row in rows} != set(RESOURCE_KINDS):
        raise ValueError("resource cleanup snapshot resource kinds are incomplete or duplicated")
    for row in rows:
        if set(row) != {"kind", "resource_id", "state", "operation", "reason", "readback", "observed_at"}:
            raise ValueError("resource cleanup snapshot row has unknown or missing fields")
        if row.get("state") not in {"removed", "already_absent", "retained", "failed"}:
            raise ValueError("resource cleanup snapshot row has an invalid state")
        expected_fields = {"path", "common_dir", "admin_dir", "instance_id", "expected_head_oid"} if row["kind"] == "worktree" else (
            {"full_ref", "expected_oid"} if row["kind"] == "local_branch" else
            {"remote_repository", "full_ref", "expected_oid"})
        if not isinstance(row.get("resource_id"), dict) or set(row["resource_id"]) != expected_fields:
            raise ValueError("resource cleanup snapshot resource identity is malformed")
        if not isinstance(row.get("readback"), dict) or not isinstance(row.get("reason"), str) or not isinstance(row.get("operation"), str):
            raise ValueError("resource cleanup snapshot operation/readback is malformed")
    matching = [row for row in rows
                if isinstance(row, dict) and row.get("kind") == kind and row.get("resource_id") == resource_id]
    if len(matching) != 1:
        raise ValueError("resource cleanup snapshot does not uniquely bind the requested resource")
    linked = False
    linked_count = 0
    latest_record_count = 0
    latest_revision = 0
    latest_event = ""
    snapshot_intents: dict[str, dict[str, Any]] = {}
    for line in journal_path.read_bytes().splitlines():
        entry = json.loads(line, object_pairs_hook=_unique_object)
        if not isinstance(entry, dict) or entry.get("schema") != JOURNAL_SCHEMA:
            raise ValueError("resource cleanup journal has an unknown schema")
        if entry.get("task_uid") != task_uid or entry.get("repository") != repository:
            raise ValueError("resource cleanup journal identity mismatch")
        event_revision = entry.get("revision")
        if not isinstance(event_revision, int) or event_revision < latest_revision:
            raise ValueError("resource cleanup journal revision regressed")
        if event_revision > latest_revision:
            latest_revision = event_revision
            latest_event = ""
        if event_revision == latest_revision:
            latest_event = str(entry.get("event") or "")
        expected_keys = ({"schema", "task_uid", "repository", "revision", "event", "kind", "resource_id", "operation", "operation_id", "observed_at"}
                         if entry.get("event") == "intent" else
                         {"schema", "task_uid", "repository", "revision", "event", "record_sha256", "observed_at"}
                         if entry.get("event") == "record" else set())
        if not expected_keys or set(entry) != expected_keys:
            raise ValueError("resource cleanup journal event has unknown or missing fields")
        if not isinstance(event_revision, int) or isinstance(event_revision, bool) or event_revision < 1:
            raise ValueError("resource cleanup journal revision is malformed")
        if entry.get("event") == "intent":
            journal_kind = entry.get("kind")
            journal_resource = entry.get("resource_id")
            operation_id = entry.get("operation_id")
            operation = entry.get("operation")
            if (journal_kind not in RESOURCE_KINDS or not isinstance(journal_resource, dict)
                    or not isinstance(operation, str) or not operation
                    or not isinstance(operation_id, str) or not re.fullmatch(r"[0-9a-f]{64}", operation_id)):
                raise ValueError("resource cleanup journal intent is malformed")
            expected_operation_id = hashlib.sha256(_canonical({
                "task_uid": task_uid, "revision": event_revision, "kind": journal_kind,
                "resource_id": journal_resource, "operation": operation,
            })).hexdigest()
            if operation_id != expected_operation_id:
                raise ValueError("resource cleanup journal intent digest is invalid")
            if event_revision == revision:
                if journal_kind in snapshot_intents:
                    raise ValueError("resource cleanup snapshot has duplicate intent rows")
                snapshot_intents[journal_kind] = journal_resource
        elif not re.fullmatch(r"[0-9a-f]{64}", str(entry.get("record_sha256") or "")):
            raise ValueError("resource cleanup journal snapshot digest is malformed")
        if event_revision == revision and entry.get("event") == "record":
            latest_record_count += 1
        if (isinstance(entry, dict) and entry.get("schema") == JOURNAL_SCHEMA
                and entry.get("event") == "record" and entry.get("task_uid") == task_uid
                and entry.get("repository") == repository and entry.get("revision") == revision
                and entry.get("record_sha256") == hashlib.sha256(raw).hexdigest()):
            linked = True
            linked_count += 1
    if (not linked or linked_count != 1 or latest_record_count != 1
            or revision != latest_revision or latest_event != "record"):
        raise ValueError("resource cleanup snapshot lacks an exact-byte journal link")
    if (set(snapshot_intents) != set(RESOURCE_KINDS)
            or any(snapshot_intents[row["kind"]] != row["resource_id"] for row in rows)):
        raise ValueError("resource cleanup snapshot is not bound to one intent per typed resource")
    return matching[0]


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _cleanup_history(receipt_root: pathlib.Path, task_uid: str, repository: str) -> dict[str, Any]:
    """Return committed statuses and resources involved in an interrupted operation."""
    journal = receipt_root / "resource-cleanup-journal.jsonl"
    snapshot_path = receipt_root / "resource-cleanup.json"
    history: dict[str, Any] = {"pending": set(), "states": {}}
    if not journal.exists():
        if snapshot_path.exists():
            raise CleanupError("prior cleanup snapshot exists without its journal")
        return history
    entries = [json.loads(line, object_pairs_hook=_unique_object) for line in journal.read_bytes().splitlines()]
    if not entries:
        raise CleanupError("cleanup journal is empty")
    latest_revision = 0
    latest_event = ""
    latest_intents: set[str] = set()
    for entry in entries:
        if (not isinstance(entry, dict) or entry.get("schema") != JOURNAL_SCHEMA
                or entry.get("task_uid") != task_uid or entry.get("repository") != repository):
            raise CleanupError("cleanup journal identity or schema is malformed")
        revision = entry.get("revision")
        if (not isinstance(revision, int) or isinstance(revision, bool) or revision < 1
                or revision < latest_revision):
            raise CleanupError("cleanup journal revision regressed")
        if revision > latest_revision:
            if latest_revision and revision != latest_revision + 1:
                raise CleanupError("cleanup journal revision skipped")
            latest_revision, latest_event, latest_intents = revision, "", set()
        elif latest_event == "record":
            raise CleanupError("cleanup journal contains an event after a committed snapshot")
        event = entry.get("event")
        latest_event = str(event or "")
        expected = ({"schema", "task_uid", "repository", "revision", "event", "kind", "resource_id", "operation", "operation_id", "observed_at"}
                    if event == "intent" else
                    {"schema", "task_uid", "repository", "revision", "event", "record_sha256", "observed_at"}
                    if event == "record" else set())
        if not expected or set(entry) != expected:
            raise CleanupError("cleanup journal event has unknown or missing fields")
        if event == "intent":
            if (entry.get("kind") not in RESOURCE_KINDS or not isinstance(entry.get("resource_id"), dict)
                    or not isinstance(entry.get("operation"), str) or not entry.get("operation")
                    or not re.fullmatch(r"[0-9a-f]{64}", str(entry.get("operation_id") or ""))):
                raise CleanupError("cleanup journal intent has an unknown resource kind")
            expected_operation_id = hashlib.sha256(_canonical({
                "task_uid": task_uid, "revision": revision, "kind": entry["kind"],
                "resource_id": entry["resource_id"], "operation": entry["operation"],
            })).hexdigest()
            if entry["operation_id"] != expected_operation_id:
                raise CleanupError("cleanup journal intent digest is invalid")
            if entry["kind"] in latest_intents:
                raise CleanupError("cleanup journal duplicates a resource intent")
            latest_intents.add(str(entry["kind"]))
        else:
            if (entry.get("event") != "record" or not re.fullmatch(r"[0-9a-f]{64}",
                    str(entry.get("record_sha256") or "")) or set(latest_intents) != set(RESOURCE_KINDS)):
                raise CleanupError("cleanup journal record is malformed or has incomplete intents")
    if latest_event != "record":
        history["pending"] = latest_intents
    if snapshot_path.exists():
        raw = snapshot_path.read_bytes()
        snapshot = json.loads(raw, object_pairs_hook=_unique_object)
        revision = snapshot.get("revision") if isinstance(snapshot, dict) else None
        if (not isinstance(snapshot, dict)
                or set(snapshot) != {"schema", "task_uid", "repository", "revision", "observed_at", "resources"}
                or snapshot.get("schema") != SNAPSHOT_SCHEMA
                or snapshot.get("task_uid") != task_uid or snapshot.get("repository") != repository
                or not isinstance(revision, int) or isinstance(revision, bool)):
            raise CleanupError("prior cleanup snapshot is malformed")
        links = [entry for entry in entries if entry.get("event") == "record" and entry.get("revision") == revision
                 and entry.get("record_sha256") == hashlib.sha256(raw).hexdigest()]
        if len(links) != 1:
            raise CleanupError("prior cleanup snapshot is not uniquely journal-linked")
        rows = snapshot.get("resources")
        if not isinstance(rows, list) or len(rows) != len(RESOURCE_KINDS):
            raise CleanupError("prior cleanup snapshot resource set is malformed")
        if any(not isinstance(row, dict) or set(row) != {
                "kind", "resource_id", "state", "operation", "reason", "readback", "observed_at"}
                for row in rows) or {row.get("kind") for row in rows} != set(RESOURCE_KINDS):
            raise CleanupError("prior cleanup snapshot rows are malformed")
        for row in rows:
            if (not isinstance(row, dict) or row.get("kind") not in RESOURCE_KINDS
                    or row.get("state") not in {"removed", "already_absent", "retained", "failed"}
                    or not isinstance(row.get("resource_id"), dict)
                    or not isinstance(row.get("operation"), str) or not isinstance(row.get("reason"), str)
                    or not isinstance(row.get("readback"), dict)):
                raise CleanupError("prior cleanup snapshot row is malformed")
            expected_fields = {"path", "common_dir", "admin_dir", "instance_id", "expected_head_oid"} if row["kind"] == "worktree" else (
                {"full_ref", "expected_oid"} if row["kind"] == "local_branch" else
                {"remote_repository", "full_ref", "expected_oid"})
            if set(row["resource_id"]) != expected_fields:
                raise CleanupError("prior cleanup snapshot resource identity is malformed")
            history["states"][row["kind"]] = row["state"]
    return history


def _perform(repo: pathlib.Path, task_uid: str, *, preflight: bool, output_json: bool) -> tuple[dict[str, Any], int]:
    if not UID_RE.fullmatch(task_uid):
        raise CleanupError("invalid task UID")
    repo = repo.resolve()
    if _git(repo, "rev-parse", "--is-inside-work-tree") != "true":
        raise CleanupError("invalid canonical repository root")
    repo = pathlib.Path(_git(repo, "rev-parse", "--show-toplevel")).resolve()
    record, mapping = _load_mapping(repo, task_uid)
    repository = str(record.get("repository") or "")
    if repository != _origin_repository(repo):
        raise CleanupError("task repository disagrees with canonical origin")
    root = _receipt_root(repo, task_uid, create=False)
    delivery_info = _run_delivery_preflight(repo, task_uid)
    receipt = _read_delivery_receipt(root, task_uid, repository)
    worktree_id = _worktree_identity(repo, record, receipt)
    branch_id, remote_id = _branch_identity(record, receipt, repository)
    resource_ids = {"worktree": worktree_id, "local_branch": branch_id, "remote_branch": remote_id}
    if not (root / "merge-receipt.json").is_file():
        raise CleanupError("canonical live merge receipt is unavailable")
    branch = str(record.get("task_branch"))
    rows = _worktree_rows(repo)
    history = _cleanup_history(root, task_uid, repository)

    if preflight:
        worktree_outcome = _worktree_state(repo, mapping, task_uid, worktree_id, branch,
                                           mutate=False, history=history)
        # Preflight describes the safe execution order rather than treating
        # the task's own worktree checkout as an independent branch holder.
        # The projection is allowed only when that exact worktree has passed
        # every identity, content, lock, mapping-reference, and activity gate.
        releasable_checkouts = ({worktree_id["path"]}
                                if worktree_outcome["state"] == "ready" else set())
        outcomes = {
            "worktree": worktree_outcome,
            "local_branch": _local_branch_state(repo, branch_id, rows, mutate=False,
                                                 history=history,
                                                 releasable_checkouts=releasable_checkouts),
            "remote_branch": _remote_branch_state(repo, remote_id, mutate=False, history=history),
        }
        blockers = [f"{kind}:{row['reason']}" for kind, row in outcomes.items() if row["state"] not in ("ready", "already_absent")]
        payload = {"status": "ready" if not blockers else "blocked", "task_uid": task_uid,
                   "repository": repository, "delivery": delivery_info.get("delivery"),
                   "cleanup_blockers": blockers,
                   "cleanup": {kind: {"state": row["state"], "reason": row["reason"],
                                      **({"depends_on": row["depends_on"]} if row.get("depends_on") else {})}
                               for kind, row in outcomes.items()},
                   "resources": [{"kind": kind, "resource_id": resource_ids[kind], **row} for kind, row in outcomes.items()]}
        return payload, 0 if not blockers else 3

    lock_path = root / "resource-cleanup.lock"
    with lock_path.open("a+b") as lock:
        ensure_lock_byte(lock)
        fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
        # Refresh delivery proof and every cleanup selector after taking the
        # task-scoped operation lock; no stale pre-lock observation authorizes
        # a destructive action.
        delivery_info = _run_delivery_preflight(repo, task_uid)
        record, mapping = _load_mapping(repo, task_uid)
        receipt = _read_delivery_receipt(root, task_uid, repository)
        worktree_id = _worktree_identity(repo, record, receipt)
        branch_id, remote_id = _branch_identity(record, receipt, repository)
        resource_ids = {"worktree": worktree_id, "local_branch": branch_id, "remote_branch": remote_id}
        history = _cleanup_history(root, task_uid, repository)
        revision = _journal_revision(root / "resource-cleanup-journal.jsonl", task_uid, repository) + 1
        journal = root / "resource-cleanup-journal.jsonl"
        outcomes: dict[str, dict[str, Any]] = {}

        def intent(kind: str, operation: str) -> None:
            op_id = hashlib.sha256(_canonical({"task_uid": task_uid, "revision": revision,
                                                "kind": kind, "resource_id": resource_ids[kind],
                                                "operation": operation})).hexdigest()
            _append_journal(journal, _event(task_uid, repository, revision, "intent", kind=kind,
                                            resource_id=resource_ids[kind], operation=operation,
                                            operation_id=op_id))

        intent("worktree", "git_worktree_remove")
        try:
            outcomes["worktree"] = _worktree_state(repo, mapping, task_uid, worktree_id, branch,
                                                    mutate=True, history=history)
        except (CleanupError, OSError) as exc:
            outcomes["worktree"] = {"state": "failed", "operation": "git_worktree_remove",
                                     "reason": "worktree_readback_or_operation_failed",
                                     "readback": {"detail": str(exc)}}
        current_rows = _worktree_rows(repo)
        intent("local_branch", "git_update_ref_cas")
        try:
            outcomes["local_branch"] = _local_branch_state(repo, branch_id, current_rows,
                                                            mutate=True, history=history)
        except (CleanupError, OSError) as exc:
            outcomes["local_branch"] = {"state": "failed", "operation": "git_update_ref_cas",
                                         "reason": "local_branch_readback_or_operation_failed",
                                         "readback": {"detail": str(exc)}}
        intent("remote_branch", "git_push_delete_with_lease")
        try:
            outcomes["remote_branch"] = _remote_branch_state(repo, remote_id, mutate=True,
                                                              history=history)
        except (CleanupError, OSError) as exc:
            outcomes["remote_branch"] = {"state": "failed", "operation": "git_push_delete_with_lease",
                                          "reason": "remote_branch_readback_or_operation_failed",
                                          "readback": {"detail": str(exc)}}

        snapshot = {
            "schema": SNAPSHOT_SCHEMA,
            "task_uid": task_uid,
            "repository": repository,
            "revision": revision,
            "observed_at": _now(),
            "resources": [
                {"kind": kind, "resource_id": resource_ids[kind], **outcomes[kind], "observed_at": _now()}
                for kind in RESOURCE_KINDS
            ],
        }
        raw = json.dumps(snapshot, ensure_ascii=False, sort_keys=True, indent=2).encode("utf-8") + b"\n"
        _atomic_write(root / "resource-cleanup.json", raw)
        _append_journal(journal, _event(task_uid, repository, revision, "record",
                                        record_sha256=hashlib.sha256(raw).hexdigest()))

    states = {kind: outcomes[kind]["state"] for kind in RESOURCE_KINDS}
    complete = all(state in ("removed", "already_absent") for state in states.values())
    payload = {"status": "cleaned" if complete else "cleanup_deferred", "task_uid": task_uid,
               "repository": repository, "delivery": delivery_info.get("delivery"),
               "cleanup_state": "cleanup_complete" if complete else "cleanup_deferred",
               "cleanup": {kind: {"state": outcomes[kind]["state"], "reason": outcomes[kind]["reason"]}
                           for kind in RESOURCE_KINDS},
               "cleanup_blockers": [f"{kind}:{outcomes[kind]['reason']}" for kind in RESOURCE_KINDS
                                    if outcomes[kind]["state"] not in ("removed", "already_absent")],
               "resources": [{"kind": kind, "resource_id": resource_ids[kind], **outcomes[kind]}
                             for kind in RESOURCE_KINDS]}
    return payload, 0 if complete else 3


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--task-uid", required=True)
    parser.add_argument("--delivery", action="store_true", required=True)
    parser.add_argument("--preflight", action="store_true")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    try:
        payload, code = _perform(pathlib.Path(args.repo_root), args.task_uid,
                                 preflight=args.preflight, output_json=args.json)
    except (CleanupError, OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as exc:
        payload, code = {"status": "blocked", "task_uid": args.task_uid,
                          "cleanup_blockers": [str(exc)]}, 1
    if args.json:
        print(json.dumps(payload, ensure_ascii=False, sort_keys=True))
    elif code:
        print("resource-cleanup: " + "; ".join(payload.get("cleanup_blockers", [payload["status"]])), file=sys.stderr)
    else:
        print(f"resource-cleanup: {payload['status']} {args.task_uid}")
    return code


if __name__ == "__main__":
    raise SystemExit(main())
