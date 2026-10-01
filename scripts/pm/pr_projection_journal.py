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


class PublicationJournal:
    """One publication journal guarded by a permanent branch-scoped lock."""

    def __init__(self, path: Path, lock_path: Path, identity: dict[str, Any]):
        self.path = path
        self.lock_path = lock_path
        self.identity = dict(identity)
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
        }
        _atomic_json(self.path, value)
        return value

    def read(self) -> dict[str, Any]:
        self._assert_locked()
        return self._read_or_create()

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
                 projection_digest: str) -> PublicationJournal:
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
    return PublicationJournal(path, lock_path, identity)
