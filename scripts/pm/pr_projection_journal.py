#!/usr/bin/env python3
"""Durable, single-host journal for ordered PR projection publication.

The journal records local action intent and remote observations.  It never
contains credentials and is not a source of remote truth on recovery.
"""
from __future__ import annotations

from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import re
import tempfile
from typing import Any, Iterator

from portable_file_lock import ensure_lock_byte, fcntl

SCHEMA = "oasis7-pr-publication-journal/v1"
TASK_POST_EVENT_SCHEMA = "oasis7-pr-task-post-event/v1"
TASK_POST_EVENTS_FILE = "task-publication-events.jsonl"
TASK_POST_TAIL_SCHEMA = "oasis7-pr-task-post-tail/v1"
TASK_POST_EVENT_TYPES = frozenset({
    "READ_FAILED", "READ_EMPTY", "READ_MATCH", "READ_CONFLICT",
    "WRITE_INTENT", "POST_ATTEMPTED", "POST_RESPONSE_UNCERTAIN", "RESOLVED",
})
_PUB_ID = re.compile(r"sha256:[0-9a-f]{64}\Z")


class JournalError(RuntimeError):
    pass


def publication_paths(common_dir: str | os.PathLike[str], repository: str,
                      branch: str, publication_id: str) -> tuple[Path, Path]:
    if not isinstance(repository, str) or not re.fullmatch(r"[^/\s]+/[^/\s]+", repository):
        raise JournalError("repository identity is invalid")
    if (not isinstance(branch, str) or not branch.strip()
            or branch.startswith("-") or ".." in branch.split("/")
            or any(ord(ch) < 32 for ch in branch)):
        raise JournalError("source branch identity is invalid")
    if not isinstance(publication_id, str) or not _PUB_ID.fullmatch(publication_id):
        raise JournalError("publication identity is invalid")
    branch_key = hashlib.sha256(f"{repository}\0{branch}".encode()).hexdigest()
    root = Path(common_dir).resolve() / "oasis7" / "pr-publication" / branch_key
    return root / publication_id.removeprefix("sha256:") / "journal.json", root / "publisher.lock"


def _fsync_dir(path: Path) -> None:
    if os.name == "nt":
        return
    fd = os.open(path, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def _atomic_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temp_name = tempfile.mkstemp(prefix=path.name + ".", dir=path.parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            json.dump(value, handle, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temp_name, path)
        _fsync_dir(path.parent)
    finally:
        Path(temp_name).unlink(missing_ok=True)


def _canonical_digest(value: Any) -> str:
    raw = json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(",", ":")).encode("utf-8")
    return "sha256:" + hashlib.sha256(raw).hexdigest()


class PublicationJournal:
    """One publication journal guarded by a permanent branch-scoped lock."""

    def __init__(self, path: Path, lock_path: Path, identity: dict[str, Any], *,
                 common_dir: Path | None = None,
                 canonical_worktree: Path | None = None):
        self.path = path
        self.lock_path = lock_path
        self.identity = dict(identity)
        self.common_dir = (common_dir or path.parent).resolve()
        self.canonical_worktree = (
            canonical_worktree.resolve() if canonical_worktree is not None else None
        )
        self.task_events_path = path.with_name(TASK_POST_EVENTS_FILE)
        self._lock = None

    @contextmanager
    def locked(self) -> Iterator["PublicationJournal"]:
        self.lock_path.parent.mkdir(parents=True, exist_ok=True)
        handle = self.lock_path.open("a+b")
        ensure_lock_byte(handle)
        fcntl.flock(handle.fileno(), fcntl.LOCK_EX)
        self._lock = handle
        try:
            self._read_or_create()
            yield self
        finally:
            self._lock = None
            handle.close()

    def _assert_locked(self) -> None:
        if self._lock is None:
            raise JournalError("journal mutation requires the publication lock")

    def inherited_lock_fd(self) -> int:
        """Return the active reservation descriptor for a child writer lease."""
        self._assert_locked()
        return self._lock.fileno()

    def _read_or_create(self) -> dict[str, Any]:
        self._assert_locked()
        if self.path.exists():
            try:
                value = json.loads(self.path.read_text(encoding="utf-8"))
            except (OSError, json.JSONDecodeError) as exc:
                raise JournalError("publication journal is unreadable") from exc
            if (not isinstance(value, dict) or value.get("schema") != SCHEMA
                    or value.get("identity") != self.identity
                    or not isinstance(value.get("actions"), list)):
                raise JournalError("publication journal identity or schema conflicts")
            return value
        value = {
            "schema": SCHEMA,
            "identity": self.identity,
            "phase": "PREPARED",
            "disposition": None,
            "actions": [],
            "task_post_tail": self._task_post_tail(0, b""),
        }
        _atomic_json(self.path, value)
        return value

    @staticmethod
    def _task_post_tail(sequence: int, raw: bytes) -> dict[str, Any]:
        return {
            "schema": TASK_POST_TAIL_SCHEMA,
            "sequence": sequence,
            "digest": "sha256:" + hashlib.sha256(raw).hexdigest(),
        }

    @staticmethod
    def _validate_task_post_tail(value: Any) -> dict[str, Any]:
        required = {"schema", "sequence", "digest"}
        if (not isinstance(value, dict) or set(value) != required
                or value.get("schema") != TASK_POST_TAIL_SCHEMA
                or type(value.get("sequence")) is not int or value["sequence"] < 0
                or not isinstance(value.get("digest"), str)
                or not _PUB_ID.fullmatch(value["digest"])):
            raise JournalError("Task publication root tail anchor is malformed")
        return value

    def read_action_state(self) -> dict[str, Any]:
        """Read the main journal without interpreting its separate event stream."""
        self._assert_locked()
        return self._read_or_create()

    def has_task_post_tail(self) -> bool:
        self._assert_locked()
        return "task_post_tail" in self._read_or_create()

    def anchor_task_history_after_exact_readback(self) -> None:
        """Anchor legacy history only after the caller verified the exact remote record."""
        self._assert_locked()
        state = self._read_or_create()
        if "task_post_tail" in state:
            self.read_task_events()
            return
        exists = self.task_events_path.exists()
        try:
            raw = self.task_events_path.read_bytes() if exists else b""
        except OSError as exc:
            raise JournalError("Task publication event journal is unreadable") from exc
        events = self._decode_task_events(raw, exists=exists)
        state["task_post_tail"] = self._task_post_tail(len(events), raw)
        try:
            _atomic_json(self.path, state)
        except OSError as exc:
            raise JournalError("Task publication root tail anchor update was not confirmed durable") from exc
        if self.read_task_events() != events:
            raise JournalError("Task publication event history changed while anchoring exact readback")

    def read(self) -> dict[str, Any]:
        self._assert_locked()
        value = self._read_or_create()
        value["task_post_events"] = self.read_task_events()
        return value

    def _decode_task_events(self, raw: bytes, *, exists: bool) -> list[dict[str, Any]]:
        if exists and (not raw or not raw.endswith(b"\n")):
            raise JournalError("Task publication event journal is truncated")
        lines = raw.splitlines() if exists else []
        events: list[dict[str, Any]] = []
        seen_identities: dict[str, dict[str, Any]] = {}
        for expected_sequence, line in enumerate(lines, start=1):
            try:
                value = json.loads(line.decode("utf-8"))
            except (UnicodeDecodeError, json.JSONDecodeError) as exc:
                raise JournalError("Task publication event journal is malformed") from exc
            required = {
                "schema", "sequence", "event", "action_id", "identity",
                "identity_digest", "evidence",
            }
            if (not isinstance(value, dict) or set(value) != required
                    or value.get("schema") != TASK_POST_EVENT_SCHEMA
                    or type(value.get("sequence")) is not int
                    or value.get("sequence") != expected_sequence
                    or not isinstance(value.get("event"), str)
                    or value.get("event") not in TASK_POST_EVENT_TYPES
                    or not isinstance(value.get("action_id"), str)
                    or not value["action_id"]
                    or not isinstance(value.get("identity"), dict)
                    or value.get("identity_digest") != _canonical_digest(value["identity"])
                    or not isinstance(value.get("evidence"), dict)):
                raise JournalError("Task publication event identity or sequence is invalid")
            prior_identity = seen_identities.setdefault(value["action_id"], value["identity"])
            if prior_identity != value["identity"]:
                raise JournalError("Task publication action identity changed in event history")
            events.append(value)
        self._validate_task_event_sequences(events)
        return events

    def read_task_events(self, action_id: str | None = None, *,
                         identity: dict[str, Any] | None = None,
                         allow_unanchored: bool = False) -> list[dict[str, Any]]:
        """Read and validate the append-only Task-comment POST event stream."""
        self._assert_locked()
        state = self._read_or_create()
        anchored = "task_post_tail" in state
        if not anchored and not allow_unanchored:
            raise JournalError("Task publication event history lacks a durable root tail anchor")
        anchor = self._validate_task_post_tail(state["task_post_tail"]) if anchored else None
        try:
            exists = self.task_events_path.exists()
            raw = self.task_events_path.read_bytes() if exists else b""
        except OSError as exc:
            raise JournalError("Task publication event journal is unreadable") from exc
        events = self._decode_task_events(raw, exists=exists)
        if (anchor is not None
                and (anchor["sequence"] != len(events)
                     or anchor["digest"] != "sha256:" + hashlib.sha256(raw).hexdigest())):
            raise JournalError("Task publication event history differs from its durable root tail anchor")
        if action_id is not None:
            events = [event for event in events if event["action_id"] == action_id]
        if identity is not None and any(event["identity"] != identity for event in events):
            raise JournalError("Task publication action does not match its original identity")
        return events

    @staticmethod
    def _validate_task_event_sequences(events: list[dict[str, Any]]) -> None:
        histories: dict[str, list[str]] = {}
        for value in events:
            history = histories.setdefault(value["action_id"], [])
            event = value["event"]
            if history and history[-1] == "RESOLVED":
                raise JournalError("Task publication event follows terminal resolution")
            if event == "WRITE_INTENT" and (not history or history[-1] != "READ_EMPTY"):
                raise JournalError("Task write intent lacks fresh empty-read boundary")
            if event == "POST_ATTEMPTED" and (not history or history[-1] != "WRITE_INTENT"):
                raise JournalError("Task POST attempt lacks adjacent durable write intent")
            if event == "POST_RESPONSE_UNCERTAIN" and "POST_ATTEMPTED" not in history:
                raise JournalError("Task POST uncertainty lacks durable attempt marker")
            if event == "RESOLVED" and (not history or history[-1] != "READ_MATCH"):
                raise JournalError("Task publication resolution lacks exact readback")
            if "POST_ATTEMPTED" in history and event in {"WRITE_INTENT", "POST_ATTEMPTED"}:
                raise JournalError("Task publication history contains a repeated POST attempt")
            history.append(event)

    def append_task_event(self, action_id: str, event: str, identity: dict[str, Any],
                          evidence: dict[str, Any] | None = None) -> dict[str, Any]:
        """Append one immutable, fsynced Task-comment protocol event."""
        self._assert_locked()
        if (not isinstance(action_id, str) or not action_id
                or event not in TASK_POST_EVENT_TYPES
                or not isinstance(identity, dict)
                or (evidence is not None and not isinstance(evidence, dict))):
            raise JournalError("Task publication event is incomplete")
        current = self.read_task_events()
        matching = [item for item in current if item["action_id"] == action_id]
        if any(item["identity"] != identity for item in matching):
            raise JournalError("Task publication action identity conflicts with prior evidence")
        action_events = [item["event"] for item in matching]
        if event == "WRITE_INTENT" and (not action_events or action_events[-1] != "READ_EMPTY"):
            raise JournalError("Task write intent requires fresh complete empty read evidence")
        if event == "POST_ATTEMPTED" and (not action_events or action_events[-1] != "WRITE_INTENT"):
            raise JournalError("Task POST attempt requires a durable write intent")
        if event == "RESOLVED" and not any(
                item in {"POST_ATTEMPTED", "READ_MATCH"} for item in action_events):
            raise JournalError("Task publication resolution lacks exact readback evidence")
        if "POST_ATTEMPTED" in action_events and event in {"WRITE_INTENT", "POST_ATTEMPTED"}:
            raise JournalError("Task POST attempt is already pending or resolved")
        record = {
            "schema": TASK_POST_EVENT_SCHEMA,
            "sequence": len(self._read_all_task_events()) + 1,
            "event": event,
            "action_id": action_id,
            "identity": identity,
            "identity_digest": _canonical_digest(identity),
            "evidence": evidence or {},
        }
        root_state = self._read_or_create()
        old_tail = self._validate_task_post_tail(root_state.get("task_post_tail"))
        self.task_events_path.parent.mkdir(parents=True, exist_ok=True)
        existed = self.task_events_path.exists()
        encoded = json.dumps(record, ensure_ascii=False, sort_keys=True,
                             separators=(",", ":")).encode("utf-8") + b"\n"
        try:
            fd = os.open(self.task_events_path, os.O_WRONLY | os.O_APPEND | os.O_CREAT, 0o600)
            try:
                offset = 0
                while offset < len(encoded):
                    written = os.write(fd, encoded[offset:])
                    if written <= 0:
                        raise OSError("short Task publication event append")
                    offset += written
                os.fsync(fd)
            finally:
                os.close(fd)
            if not existed:
                _fsync_dir(self.task_events_path.parent)
        except OSError as exc:
            raise JournalError("Task publication event append was not confirmed durable") from exc
        try:
            new_raw = self.task_events_path.read_bytes()
        except OSError as exc:
            raise JournalError("Task publication event append readback failed") from exc
        observed = self._decode_task_events(new_raw, exists=True)
        if not observed or observed[-1] != record:
            raise JournalError("Task publication event append failed readback verification")
        current_root = self._read_or_create()
        if self._validate_task_post_tail(current_root.get("task_post_tail")) != old_tail:
            raise JournalError("Task publication root tail anchor changed during event append")
        current_root["task_post_tail"] = self._task_post_tail(len(observed), new_raw)
        try:
            _atomic_json(self.path, current_root)
        except OSError as exc:
            raise JournalError(
                "Task publication root tail anchor update was not confirmed durable",
            ) from exc
        observed = self.read_task_events()
        if not observed or observed[-1] != record:
            raise JournalError("Task publication event/root tail verification failed")
        return record

    def _read_all_task_events(self) -> list[dict[str, Any]]:
        return self.read_task_events()

    def intent(self, action_id: str, kind: str, expected: dict[str, Any]) -> dict[str, Any]:
        self._assert_locked()
        if not action_id or not kind or not isinstance(expected, dict):
            raise JournalError("publication action identity is incomplete")
        value = self._read_or_create()
        matches = [item for item in value["actions"] if item.get("action_id") == action_id]
        if matches:
            old = matches[-1]
            if old.get("kind") != kind or old.get("expected") != expected:
                raise JournalError("publication action identity conflicts with its journal")
            if old.get("state") == "observed":
                return old
            return old
        action = {"action_id": action_id, "kind": kind, "expected": expected,
                  "state": "intent"}
        value["actions"].append(action)
        _atomic_json(self.path, value)
        return action

    def observe(self, action_id: str, observed: dict[str, Any], *, phase: str | None = None) -> None:
        self._assert_locked()
        value = self._read_or_create()
        matches = [item for item in value["actions"] if item.get("action_id") == action_id]
        if len(matches) != 1:
            raise JournalError("publication observation has no unique prior intent")
        action = matches[0]
        if not isinstance(observed, dict):
            raise JournalError("publication observation must be an object")
        if action.get("state") == "observed" and action.get("observed") != observed:
            raise JournalError("publication observation changed after confirmation")
        action["state"] = "observed"
        action["observed"] = observed
        if phase is not None:
            value["phase"] = phase
        value["disposition"] = None
        _atomic_json(self.path, value)

    def uncertain(self, action_id: str, code: str) -> None:
        self._assert_locked()
        value = self._read_or_create()
        matches = [item for item in value["actions"] if item.get("action_id") == action_id]
        if len(matches) != 1:
            raise JournalError("uncertain action has no unique prior intent")
        matches[0]["state"] = "uncertain"
        value["disposition"] = code
        _atomic_json(self.path, value)

    def disposition(self, code: str) -> None:
        self._assert_locked()
        value = self._read_or_create()
        value["disposition"] = code
        if code in {"SUPERSEDED", "CONFLICT", "UNCERTAIN"}:
            value["phase"] = code
        _atomic_json(self.path, value)


def open_journal(common_dir: str | os.PathLike[str], repository: str,
                 branch: str, publication_id: str, *, task_uid: str,
                 source_head_oid: str, scope_base_oid: str,
                 projection_digest: str,
                 canonical_worktree: str | os.PathLike[str] | None = None) -> PublicationJournal:
    path, lock_path = publication_paths(common_dir, repository, branch, publication_id)
    identity = {
        "repository": repository,
        "branch": branch,
        "publication_id": publication_id,
        "task_uid": task_uid,
        "source_head_oid": source_head_oid,
        "scope_base_oid": scope_base_oid,
        "projection_digest": projection_digest,
    }
    return PublicationJournal(
        path, lock_path, identity, common_dir=Path(common_dir),
        canonical_worktree=Path(canonical_worktree) if canonical_worktree is not None else None,
    )
