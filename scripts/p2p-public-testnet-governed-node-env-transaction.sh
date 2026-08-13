#!/usr/bin/env bash
set -euo pipefail

# Keep this entrypoint shell-compatible for the repository's script checks while
# putting byte/stat/fsync handling in Python, where it can be made explicit.
exec python3 - "$@" <<'PY'
from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import re
import stat
import sys
import tempfile
from pathlib import Path
from typing import Any

SCHEMA = "oasis7.node_env_transaction.v2"
SERVICE_NAME_RE = re.compile(r"^[A-Za-z0-9_.@:-]+$")


def fail(message: str) -> "NoReturn":
    raise SystemExit(f"error: governed node.env transaction: {message}")


def regular_path(path: Path, label: str) -> Path:
    if path.is_symlink():
        fail(f"{label} must not be a symlink: {path}")
    if not path.is_file():
        fail(f"{label} must be a regular file: {path}")
    return path


def directory_path(path: Path, label: str) -> Path:
    if path.is_symlink() or not path.is_dir():
        fail(f"{label} must be a real directory: {path}")
    return path


def file_stat(path: Path) -> dict[str, int]:
    metadata = os.stat(path, follow_symlinks=False)
    if not stat.S_ISREG(metadata.st_mode):
        fail(f"path is not a regular file: {path}")
    return {
        "uid": int(metadata.st_uid),
        "gid": int(metadata.st_gid),
        "mode": int(stat.S_IMODE(metadata.st_mode)),
    }


def digest(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def descriptor_integrity(content: bytes, metadata: dict[str, int]) -> str:
    binding = json.dumps(
        {"sha256": digest(content), "stat": metadata},
        ensure_ascii=True,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("ascii")
    return digest(binding)


def descriptor(content: bytes, metadata: dict[str, int]) -> dict[str, Any]:
    return {
        "sha256": digest(content),
        "stat": metadata,
        "integrity_sha256": descriptor_integrity(content, metadata),
        "content_base64": base64.b64encode(content).decode("ascii"),
    }


def descriptor_content(raw: Any, label: str) -> tuple[bytes, dict[str, int], str]:
    if not isinstance(raw, dict):
        fail(f"journal {label} descriptor must be an object")
    try:
        encoded = raw["content_base64"]
        expected_sha = raw["sha256"]
        expected_integrity = raw["integrity_sha256"]
        raw_stat = raw["stat"]
        if (
            not isinstance(encoded, str)
            or not isinstance(expected_sha, str)
            or not isinstance(expected_integrity, str)
            or not isinstance(raw_stat, dict)
        ):
            raise ValueError("invalid descriptor field types")
        if not re.fullmatch(r"[0-9a-f]{64}", expected_sha):
            raise ValueError("invalid sha256")
        if not re.fullmatch(r"[0-9a-f]{64}", expected_integrity):
            raise ValueError("invalid integrity_sha256")
        metadata = {
            key: int(raw_stat[key]) for key in ("uid", "gid", "mode")
        }
        if metadata["uid"] < 0 or metadata["gid"] < 0 or not 0 <= metadata["mode"] <= 0o7777:
            raise ValueError("invalid stat values")
        content = base64.b64decode(encoded, validate=True)
    except (KeyError, TypeError, ValueError, base64.binascii.Error) as error:
        fail(f"journal {label} descriptor is invalid: {error}")
    actual_sha = digest(content)
    if actual_sha != expected_sha:
        fail(f"journal {label} content hash mismatch")
    actual_integrity = descriptor_integrity(content, metadata)
    if actual_integrity != expected_integrity:
        fail(f"journal {label} integrity binding mismatch")
    return content, metadata, actual_sha


def fsync_directory(directory: Path) -> None:
    flags = os.O_RDONLY
    if hasattr(os, "O_DIRECTORY"):
        flags |= os.O_DIRECTORY
    fd = os.open(directory, flags)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def atomic_replace(path: Path, content: bytes, metadata: dict[str, int]) -> None:
    temporary: Path | None = None
    try:
        fd, raw_temporary = tempfile.mkstemp(
            prefix=f".{path.name}.", suffix=".tmp", dir=str(path.parent)
        )
        temporary = Path(raw_temporary)
        try:
            os.fchown(fd, metadata["uid"], metadata["gid"])
            os.fchmod(fd, metadata["mode"])
            written = 0
            while written < len(content):
                written += os.write(fd, content[written:])
            os.fsync(fd)
        finally:
            os.close(fd)
        os.replace(temporary, path)
        temporary = None
        fsync_directory(path.parent)
    finally:
        if temporary is not None:
            try:
                temporary.unlink()
            except FileNotFoundError:
                pass


def write_json(path: Path, payload: dict[str, Any]) -> None:
    if path.is_symlink():
        fail(f"journal must not be a symlink: {path}")
    directory_path(path.parent, "journal parent")
    encoded = (json.dumps(payload, ensure_ascii=True, indent=2, sort_keys=True) + "\n").encode()
    temporary: Path | None = None
    try:
        fd, raw_temporary = tempfile.mkstemp(
            prefix=f".{path.name}.", suffix=".tmp", dir=str(path.parent)
        )
        temporary = Path(raw_temporary)
        try:
            written = 0
            while written < len(encoded):
                written += os.write(fd, encoded[written:])
            os.fsync(fd)
        finally:
            os.close(fd)
        os.replace(temporary, path)
        temporary = None
        fsync_directory(path.parent)
    finally:
        if temporary is not None:
            try:
                temporary.unlink()
            except FileNotFoundError:
                pass


def read_env(path: Path) -> tuple[bytes, str, dict[str, int]]:
    regular_path(path, "env file")
    content = path.read_bytes()
    try:
        text = content.decode("utf-8")
    except UnicodeDecodeError as error:
        fail(f"env file is not valid UTF-8: {error}")
    metadata = file_stat(path)
    return content, text, metadata


def assignment_line(text: str) -> tuple[int, str]:
    lines = text.splitlines(keepends=True)
    matches = [
        (index, line)
        for index, line in enumerate(lines)
        if line.startswith("SERVICE_NAME=")
    ]
    if len(matches) != 1:
        fail(f"env file must contain exactly one SERVICE_NAME assignment (found {len(matches)})")
    return matches[0]


def journal_payload(
    env_path: Path,
    service_name: str,
    before: dict[str, Any],
    after: dict[str, Any],
    phase: str,
    phases: list[str],
    rollback: dict[str, Any] | None = None,
) -> dict[str, Any]:
    payload: dict[str, Any] = {
        "schema_version": SCHEMA,
        "phase": phase,
        "phases": phases,
        "env_path": str(env_path),
        "service_name": service_name,
        "before": before,
        "after": after,
    }
    if rollback is not None:
        payload["rollback"] = rollback
    return payload


def validate_service_name(service_name: str) -> None:
    if not service_name or not SERVICE_NAME_RE.fullmatch(service_name):
        fail("service-name must be a non-empty systemd unit name without separators or control characters")


def replace(args: argparse.Namespace) -> int:
    env_path = Path(args.env_file).expanduser()
    journal_path = Path(args.journal).expanduser().resolve(strict=False)
    if env_path.resolve(strict=False) == journal_path.resolve(strict=False):
        fail("env-file and journal must be different paths")
    if journal_path.exists() or journal_path.is_symlink():
        fail(f"journal already exists: {journal_path}")
    directory_path(journal_path.parent, "journal parent")
    validate_service_name(args.service_name)
    before_content, text, before_stat = read_env(env_path)
    env_path = env_path.resolve(strict=True)
    index, line = assignment_line(text)
    if line.endswith("\r\n"):
        ending = "\r\n"
    elif line.endswith("\n"):
        ending = "\n"
    elif line.endswith("\r"):
        ending = "\r"
    else:
        ending = ""
    lines = text.splitlines(keepends=True)
    lines[index] = f"SERVICE_NAME={args.service_name}{ending}"
    after_content = "".join(lines).encode("utf-8")
    after_stat = dict(before_stat)
    before = descriptor(before_content, before_stat)
    after = descriptor(after_content, after_stat)
    prepared = journal_payload(
        env_path, args.service_name, before, after, "prepared", ["prepared"]
    )
    write_json(journal_path, prepared)
    try:
        atomic_replace(env_path, after_content, after_stat)
        actual_content = env_path.read_bytes()
        actual_stat = file_stat(env_path)
        if actual_content != after_content or actual_stat != after_stat:
            raise RuntimeError("post-replace bytes or metadata mismatch")
        committed = journal_payload(
            env_path,
            args.service_name,
            before,
            after,
            "committed",
            ["prepared", "committed"],
        )
        write_json(journal_path, committed)
    except Exception as error:
        try:
            atomic_replace(env_path, before_content, before_stat)
            restored_content = env_path.read_bytes()
            restored_stat = file_stat(env_path)
            rollback = descriptor(restored_content, restored_stat)
            write_json(
                journal_path,
                journal_payload(
                    env_path,
                    args.service_name,
                    before,
                    after,
                    "rolled_back",
                    ["prepared", "rolled_back"],
                    rollback,
                ),
            )
        except Exception as rollback_error:
            fail(f"transaction failed ({error}); rollback failed ({rollback_error})")
        fail(f"transaction failed and was rolled back: {error}")
    print(f"committed env_path={env_path} journal={journal_path}")
    return 0


def rollback(args: argparse.Namespace) -> int:
    journal_path = Path(args.journal).expanduser().resolve(strict=False)
    regular_path(journal_path, "journal")
    try:
        payload = json.loads(journal_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"invalid journal: {error}")
    if not isinstance(payload, dict) or payload.get("schema_version") != SCHEMA:
        fail(
            "unsupported schema_version; expected "
            f"{SCHEMA} with cryptographically bound content/stat descriptors"
        )
    env_path = Path(str(payload.get("env_path") or ""))
    if not env_path.is_absolute():
        fail("journal env_path must be an absolute canonical path")
    regular_path(env_path, "env file")
    before = payload.get("before")
    after = payload.get("after")
    before_content, before_stat, _ = descriptor_content(before, "before")
    after_content, after_stat, after_sha = descriptor_content(after, "after")
    current_content = env_path.read_bytes()
    current_stat = file_stat(env_path)
    phase = payload.get("phase")
    if phase == "rolled_back":
        if current_content != before_content or current_stat != before_stat:
            fail("rolled-back journal does not match current env file")
        print(f"already_rolled_back env_path={env_path} journal={journal_path}")
        return 0
    if phase not in ("prepared", "committed"):
        fail(f"journal phase is not rollback-eligible: {phase!r}")
    if digest(current_content) == digest(before_content) and current_stat == before_stat:
        print(f"already_rolled_back env_path={env_path} journal={journal_path}")
        return 0
    if digest(current_content) != after_sha or current_stat != after_stat:
        fail("env file changed after transaction; refusing rollback")
    atomic_replace(env_path, before_content, before_stat)
    restored_content = env_path.read_bytes()
    restored_stat = file_stat(env_path)
    if restored_content != before_content or restored_stat != before_stat:
        fail("rollback verification failed")
    phases = list(payload.get("phases") or ["prepared", "committed"])
    if "rolled_back" not in phases:
        phases.append("rolled_back")
    payload["phase"] = "rolled_back"
    payload["phases"] = phases
    payload["rollback"] = descriptor(restored_content, restored_stat)
    write_json(journal_path, payload)
    print(f"rolled_back env_path={env_path} journal={journal_path}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Atomically transact a governed SERVICE_NAME in node.env")
    parser.add_argument("--rollback", action="store_true")
    parser.add_argument("--env-file")
    parser.add_argument("--service-name")
    parser.add_argument("--journal", required=True)
    args = parser.parse_args()
    if args.rollback:
        if args.env_file is not None or args.service_name is not None:
            parser.error("--rollback accepts only --journal")
        return rollback(args)
    if args.env_file is None or args.service_name is None:
        parser.error("replacement requires --env-file, --service-name, and --journal")
    return replace(args)


if __name__ == "__main__":
    raise SystemExit(main())
PY
