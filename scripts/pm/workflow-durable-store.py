#!/usr/bin/env python3
"""Single crash-safe persistence boundary for workflow JSON authority."""
from __future__ import annotations

import argparse
import contextlib
import json
import os
import pathlib
import copy
import hashlib
import re
import tempfile
import time
from typing import Any, Callable, Iterator

try:
    from portable_file_lock import ensure_lock_byte, fcntl
except ModuleNotFoundError:  # Support isolated fixture copies of this module.
    try:
        import fcntl

        def ensure_lock_byte(handle) -> None:
            return None
    except ImportError:  # pragma: no cover - exercised on Windows fixtures
        import msvcrt

        class _WindowsFcntl:
            LOCK_EX = 0
            LOCK_UN = 1

            @staticmethod
            def flock(fd: int, operation: int) -> None:
                import os
                import time

                os.lseek(fd, 0, os.SEEK_SET)
                if operation == _WindowsFcntl.LOCK_UN:
                    msvcrt.locking(fd, msvcrt.LK_UNLCK, 1)
                    return
                while True:
                    try:
                        msvcrt.locking(fd, msvcrt.LK_NBLCK, 1)
                        return
                    except OSError as exc:
                        if (getattr(exc, "winerror", None) not in (33, 36, 158)
                                and getattr(exc, "errno", None) not in (13, 36)):
                            raise
                        time.sleep(0.01)

        fcntl = _WindowsFcntl()

        def ensure_lock_byte(handle) -> None:
            if handle.tell() == 0:
                handle.write(b"0")
                handle.flush()

PHASE_ORDER = {
    "": 0, "blocked": 0, "intake": 1, "bootstrap": 2, "route": 3,
    "execution": 4, "pre_pr_review": 5, "pre_pr_ready": 6, "pr_watch": 7,
    "task_done": 8, "main_sync": 9, "closed_without_merge": 10,
    "post_merge_done": 11,
}
IMMUTABLE_TASK_FIELDS = {
    "task_uid", "repository", "issue_number", "canonical_worktree",
    "task_branch", "default_branch", "project_item_id", "pr_number", "pr_url",
    "merge_receipt", "merge_receipt_sha256",
}
STATUS_ORDER = {"": 0, "candidate": 1, "committed": 2, "ready": 3, "pr_watch": 4, "done": 5}
CAS_MAP_FIELDS = {"phase_receipts", "phase_receipt_sha256", "evidence"}
RETIREMENT_SCHEMA = "oasis7.duplicate-candidate-retirement/v1"
TASK_UID_RE = re.compile(r"task_[0-9a-f]{32}\Z")
RETIREMENT_LEDGER = "retired_duplicate_candidates"


def canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def canonical_digest(value: Any) -> str:
    return "sha256:" + hashlib.sha256(canonical_json_bytes(value)).hexdigest()


def validate_retirement_tombstone(value: Any) -> str:
    """Validate the immutable structural/integrity contract for one tombstone."""
    if not isinstance(value, dict) or value.get("schema") != RETIREMENT_SCHEMA:
        raise ValueError("retirement tombstone has an unsupported schema")
    task_uid = value.get("task_uid")
    if not isinstance(task_uid, str) or TASK_UID_RE.fullmatch(task_uid) is None:
        raise ValueError("retirement tombstone has an invalid task UID")
    old_record = value.get("old_record")
    if not isinstance(old_record, dict) or old_record.get("task_uid") != task_uid:
        raise ValueError("retirement tombstone old record does not bind its task UID")
    if value.get("old_record_sha256") != canonical_digest(old_record):
        raise ValueError("retirement tombstone old-record digest is invalid")
    if value.get("reason") != "duplicate":
        raise ValueError("retirement tombstone reason is not duplicate")
    for field in ("issuer", "time", "transaction_id"):
        if not isinstance(value.get(field), str) or not value[field].strip():
            raise ValueError(f"retirement tombstone {field} is missing")
    evidence = value.get("evidence")
    if not isinstance(evidence, dict):
        raise ValueError("retirement tombstone evidence is missing")
    candidate = evidence.get("candidate_issue")
    replacement = evidence.get("replacement")
    permission = evidence.get("permission")
    comment = evidence.get("disposition_comment")
    discovery = evidence.get("artifact_discovery")
    if not isinstance(candidate, dict) or candidate.get("task_uid") != task_uid:
        raise ValueError("retirement tombstone candidate Issue evidence is invalid")
    if (candidate.get("repository") != old_record.get("repository")
            or str(candidate.get("issue_number")) != str(old_record.get("issue_number"))):
        raise ValueError("retirement tombstone candidate Issue differs from its old record")
    if not evidence.get("candidate_project_item"):
        raise ValueError("retirement tombstone candidate Project item evidence is missing")
    if not isinstance(replacement, dict) or not TASK_UID_RE.fullmatch(str(replacement.get("task_uid") or "")):
        raise ValueError("retirement tombstone replacement evidence is invalid")
    if replacement.get("repository") != old_record.get("repository"):
        raise ValueError("retirement tombstone replacement repository differs from candidate")
    if not isinstance(comment, dict) or not comment.get("id") or not str(comment.get("body_sha256") or "").startswith("sha256:"):
        raise ValueError("retirement tombstone disposition comment evidence is invalid")
    if (not isinstance(permission, dict) or permission.get("permission") != "admin"
            or permission.get("login") != value.get("issuer")):
        raise ValueError("retirement tombstone issuer is not bound to admin permission evidence")
    if not isinstance(discovery, dict) or discovery.get("complete") is not True:
        raise ValueError("retirement tombstone artifact discovery is incomplete")
    digest = value.get("digest")
    unsigned = dict(value)
    unsigned.pop("digest", None)
    if digest != canonical_digest(unsigned):
        raise ValueError("retirement tombstone digest is invalid")
    return task_uid


def validate_retirement_ledger(mapping: Any) -> dict[str, dict[str, Any]]:
    """Return tombstones keyed by UID, rejecting corruption and active reuse."""
    if not isinstance(mapping, dict):
        raise ValueError("tasks mapping is not an object")
    tasks = mapping.get("tasks", {})
    if not isinstance(tasks, dict):
        raise ValueError("tasks mapping has no tasks object")
    ledger = mapping.get(RETIREMENT_LEDGER, [])
    if not isinstance(ledger, list):
        raise ValueError("retirement tombstone ledger is not a list")
    retired: dict[str, dict[str, Any]] = {}
    transactions: set[str] = set()
    for tombstone in ledger:
        task_uid = validate_retirement_tombstone(tombstone)
        if task_uid in retired:
            raise ValueError(f"duplicate retirement tombstone for {task_uid}")
        transaction_id = str(tombstone["transaction_id"])
        if transaction_id in transactions:
            raise ValueError("duplicate retirement transaction ID")
        transactions.add(transaction_id)
        retired[task_uid] = tombstone
    overlap = sorted(set(retired).intersection(str(uid) for uid in tasks))
    if overlap:
        raise ValueError(f"retired task UID is present in active tasks: {overlap[0]}")
    return retired


def read_mapping(path: pathlib.Path, default: Any | None = None) -> dict[str, Any]:
    """Load and validate a GitHub Project task mapping before any consumer acts."""
    path = pathlib.Path(path)
    if path.exists():
        value = json.loads(path.read_text(encoding="utf-8"))
    else:
        value = copy.deepcopy(default if default is not None else {"version": 1, "tasks": {}})
    validate_retirement_ledger(value)
    return value


def retired_task(mapping: Any, task_uid: str) -> dict[str, Any] | None:
    """Find a structurally valid tombstone for a UID, if one exists."""
    return validate_retirement_ledger(mapping).get(task_uid)


def _preserve_retirement_state(path: pathlib.Path, value: Any, *, allow_append: bool = False) -> None:
    """Protect task maps at the atomic write boundary, including legacy writers."""
    if not isinstance(value, dict) or "tasks" not in value:
        return
    try:
        existing_bytes = path.read_bytes()
    except FileNotFoundError:
        validate_retirement_ledger(value)
        return
    except OSError as exc:
        raise ValueError(f"cannot read existing task mapping before atomic replacement: {exc}") from exc
    try:
        current = json.loads(existing_bytes)
    except (json.JSONDecodeError, UnicodeDecodeError) as exc:
        raise ValueError(f"existing task mapping is unreadable; refusing to replace authority: {exc}") from exc
    if not isinstance(current, dict) or not isinstance(current.get("tasks"), dict):
        raise ValueError("existing task mapping is malformed; refusing to replace authority")
    old_retired = validate_retirement_ledger(current)
    new_retired = validate_retirement_ledger(value)
    overlap = sorted(set(old_retired).intersection(str(uid) for uid in value.get("tasks", {})))
    if overlap:
        raise ValueError(f"cannot reintroduce retired task UID {overlap[0]}")
    old_ledger = list(current.get(RETIREMENT_LEDGER, []))
    new_ledger = list(value.get(RETIREMENT_LEDGER, []))
    if allow_append:
        if new_ledger[:len(old_ledger)] != old_ledger:
            raise ValueError("retirement tombstone ledger is not append-only")
    elif new_ledger != old_ledger:
        # Legacy/full-snapshot writers may omit or carry an older prefix, but
        # cannot append, reorder, replace, or truncate trusted retirement state.
        if new_ledger != old_ledger[:len(new_ledger)]:
            raise ValueError("generic mapping writer cannot modify retirement tombstones")
        if old_ledger:
            value[RETIREMENT_LEDGER] = old_ledger
        elif new_ledger:
            raise ValueError("generic mapping writer cannot append retirement tombstones")


def _merge_value(current: Any, incoming: Any, *, key: str = "") -> Any:
    """Monotonic deep merge used for stale same-task snapshots."""
    if key == "workflow_phase":
        old, new = str(current or ""), str(incoming or "")
        if old not in PHASE_ORDER or new not in PHASE_ORDER:
            if old and new and old != new:
                raise ValueError(f"unknown workflow phase conflict: {old!r} vs {new!r}")
        return old if PHASE_ORDER.get(old, 0) >= PHASE_ORDER.get(new, 0) else new
    if key == "status":
        old, new = str(current or ""), str(incoming or "")
        if old in STATUS_ORDER and new in STATUS_ORDER:
            return old if STATUS_ORDER[old] >= STATUS_ORDER[new] else new
        if old and new and old != new:
            raise ValueError(f"status CAS conflict: {old!r} vs {new!r}")
        return new or old
    if key == "updated_at":
        return max(str(current or ""), str(incoming or ""))
    if key in CAS_MAP_FIELDS and isinstance(current, dict) and isinstance(incoming, dict):
        merged=dict(current)
        for authority_key, authority_value in incoming.items():
            if authority_key in merged and merged[authority_key] != authority_value:
                raise ValueError(f"{key} CAS conflict: {authority_key}")
            merged.setdefault(authority_key,authority_value)
        return merged
    if isinstance(current, dict) and isinstance(incoming, dict):
        merged = dict(current)
        for child_key, child in incoming.items():
            if child_key in merged:
                merged[child_key] = _merge_value(merged[child_key], child, key=child_key)
            else:
                merged[child_key] = child
        return merged
    if isinstance(current, list) and isinstance(incoming, list):
        merged = list(current)
        for item in incoming:
            if item not in merged:
                merged.append(item)
        return merged
    # Empty stale values never erase durable authority.
    if incoming in (None, "", [], {}):
        return current
    return incoming


def _merge_task(current: dict[str, Any], patch: dict[str, Any]) -> dict[str, Any]:
    patch=copy.deepcopy(patch)
    stale=bool(current.get("updated_at") and patch.get("updated_at") and
               str(patch["updated_at"]) < str(current["updated_at"]))
    if stale:
        # A provably older snapshot is not a CAS attempt. Drop its conflicting
        # authority keys and retain the newer committed values.
        for field in CAS_MAP_FIELDS:
            old_map=current.get(field) or {}; new_map=patch.get(field) or {}
            if isinstance(old_map,dict) and isinstance(new_map,dict):
                patch[field]={key:value for key,value in new_map.items()
                              if key not in old_map or old_map[key]==value}
    for key in IMMUTABLE_TASK_FIELDS:
        old, new = current.get(key), patch.get(key)
        if old not in (None, "") and new not in (None, "") and old != new:
            raise ValueError(f"immutable task identity conflict: {key}")
    return _merge_value(current, patch)


def mapping_lock_path(path: pathlib.Path) -> pathlib.Path:
    path = pathlib.Path(path).resolve()
    return path.with_name(path.name + ".lock")


def _fsync_dir(path: pathlib.Path) -> None:
    if os.name == "nt":
        return
    fd = os.open(path, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def _atomic_replace(source: str | os.PathLike[str], destination: pathlib.Path) -> None:
    """Retry transient Windows sharing violations around atomic replacement."""
    deadline = time.monotonic() + 5.0
    while True:
        try:
            os.replace(source, destination)
            return
        except PermissionError as exc:
            if (os.name != "nt" or time.monotonic() >= deadline
                    or getattr(exc, "winerror", None) not in (5, 32)):
                raise
            time.sleep(0.01)


def atomic_replace_json(path: pathlib.Path, value: Any, *, allow_retirement_append: bool = False) -> None:
    path = pathlib.Path(path).resolve()
    _preserve_retirement_state(path, value, allow_append=allow_retirement_append)
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix=path.name + ".tmp.", dir=path.parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as output:
            json.dump(value, output, indent=2, sort_keys=True, ensure_ascii=False)
            output.write("\n")
            output.flush()
            os.fsync(output.fileno())
        _atomic_replace(temporary, path)
        _fsync_dir(path.parent)
    finally:
        pathlib.Path(temporary).unlink(missing_ok=True)


@contextlib.contextmanager
def locked_json(
    path: pathlib.Path,
    default: Any | None = None,
    *,
    write_back: bool = True,
    allow_retirement_append: bool = False,
) -> Iterator[Any]:
    path = pathlib.Path(path).resolve()
    path.parent.mkdir(parents=True, exist_ok=True)
    lock = mapping_lock_path(path)
    with lock.open("a+b") as handle:
        ensure_lock_byte(handle)
        fcntl.flock(handle.fileno(), fcntl.LOCK_EX)
        value = json.loads(path.read_text(encoding="utf-8")) if path.exists() else (default if default is not None else {})
        yield value
        if write_back:
            atomic_replace_json(path, value, allow_retirement_append=allow_retirement_append)


def transact_json(path: pathlib.Path, update: Callable[[Any], Any], default: Any | None = None) -> Any:
    with locked_json(path, default) as value:
        result = update(value)
    return result


def replace_json(path: pathlib.Path, value: Any) -> None:
    """Replace a document while participating in the same path-scoped lock."""
    def update(current: Any) -> None:
        if isinstance(current, dict) and isinstance(value, dict):
            current.clear(); current.update(value)
        else:
            raise TypeError("replace_json currently requires JSON objects")
    transact_json(path, update, {})


def merge_mapping_document(path: pathlib.Path, snapshot: dict[str, Any]) -> None:
    """Merge a possibly stale mapping snapshot without dropping other writers."""
    def update(latest: dict[str, Any]) -> None:
        current_retired = validate_retirement_ledger(latest)
        incoming_ledger = snapshot.get(RETIREMENT_LEDGER)
        if incoming_ledger is not None:
            if not isinstance(incoming_ledger, list):
                raise ValueError("retirement tombstone ledger is not a list")
            validate_retirement_ledger({"tasks": snapshot.get("tasks", {}), RETIREMENT_LEDGER: incoming_ledger})
            if incoming_ledger != list(current_retired.values()):
                if incoming_ledger != list(current_retired.values())[:len(incoming_ledger)]:
                    raise ValueError("generic mapping merge cannot modify retirement tombstones")
        for key, value in snapshot.items():
            if key not in ("tasks", RETIREMENT_LEDGER): latest[key] = value
        tasks=latest.setdefault("tasks",{})
        for uid, record in (snapshot.get("tasks") or {}).items():
            if uid in current_retired:
                raise ValueError(f"cannot reintroduce retired task UID {uid}")
            tasks[uid] = _merge_task(tasks.get(uid, {}), record)
        validate_retirement_ledger(latest)
    transact_json(path, update, {"version":1,"tasks":{}})


def merge_task_record(path: pathlib.Path, task_uid: str, patch: dict[str, Any]) -> dict[str, Any]:
    def update(mapping: dict[str, Any]) -> dict[str, Any]:
        retired = validate_retirement_ledger(mapping)
        if task_uid in retired:
            raise ValueError(f"cannot recreate retired task UID {task_uid}")
        mapping.setdefault("version", 1)
        tasks = mapping.setdefault("tasks", {})
        record = _merge_task(tasks.get(task_uid, {}), patch)
        tasks[task_uid] = record
        validate_retirement_ledger(mapping)
        return dict(record)
    return transact_json(path, update, {"version": 1, "tasks": {}})


def atomic_journal_transition(path: pathlib.Path, value: dict[str, Any]) -> None:
    path = pathlib.Path(path)
    with locked_json(path, {}) as journal:
        old_revision = int(journal.get("revision", 0))
        new_revision = int(value.get("revision", old_revision + 1))
        if journal and new_revision <= old_revision:
            raise ValueError("journal revision must advance")
        journal.clear(); journal.update(value); journal["revision"] = new_revision


def recover_atomic_journal(path: pathlib.Path) -> dict[str, Any]:
    path = pathlib.Path(path)
    return json.loads(path.read_text(encoding="utf-8")) if path.exists() else {}


def main() -> int:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    write = sub.add_parser("write-journal")
    write.add_argument("--path", required=True); write.add_argument("--json", required=True)
    replace = sub.add_parser("replace-json-file")
    replace.add_argument("--path", required=True); replace.add_argument("--json-file", required=True)
    args = parser.parse_args()
    if args.command == "write-journal":
        atomic_journal_transition(pathlib.Path(args.path), json.loads(args.json))
    elif args.command == "replace-json-file":
        atomic_replace_json(pathlib.Path(args.path), json.loads(pathlib.Path(args.json_file).read_text(encoding="utf-8")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
