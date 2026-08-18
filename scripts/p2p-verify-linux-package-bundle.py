#!/usr/bin/env python3
"""Verify the embedded Linux package provenance and confined checksums."""

from __future__ import annotations

import hashlib
import re
import stat
import sys
from pathlib import Path, PurePosixPath
from typing import NoReturn


def die(message: str) -> NoReturn:
    print(f"error: Linux package bundle verification: {message}", file=sys.stderr)
    raise SystemExit(1)


def regular_file(path: Path, label: str) -> None:
    if path.is_symlink() or not path.is_file() or not stat.S_ISREG(path.lstat().st_mode):
        die(f"{label} must be a regular non-symlink file: {path}")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def parse_buildinfo(path: Path) -> dict[str, str]:
    info: dict[str, str] = {}
    for raw in path.read_text(encoding="utf-8").splitlines():
        if not raw or raw.startswith("#"):
            continue
        key, separator, value = raw.partition("=")
        if not separator or not key.strip():
            continue
        key = key.strip()
        if key in info:
            die(f"BUILDINFO contains duplicate key: {key}")
        info[key] = value.strip()
    for key in ("commit", "package_version", "run_id", "platform"):
        if not info.get(key):
            die(f"BUILDINFO missing {key}")
    if info["platform"] != "linux-x64":
        die(f"BUILDINFO platform mismatch: {info['platform']!r}")
    if not re.fullmatch(r"[0-9a-fA-F]{40}", info["commit"]):
        die("BUILDINFO commit is not a 40-character hexadecimal SHA")
    return info


def verify_sums(root: Path, sums_path: Path) -> set[str]:
    verified: set[str] = set()
    for raw in sums_path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line:
            continue
        expected, separator, name = line.partition("  ")
        if not separator:
            parts = line.split(maxsplit=1)
            if len(parts) != 2:
                die(f"SHA256SUMS contains malformed entry: {raw}")
            expected, name = parts
        expected = expected.strip()
        name = name.lstrip("*").strip()
        relative = PurePosixPath(name.replace("\\", "/"))
        if (
            not re.fullmatch(r"[0-9a-fA-F]{64}", expected)
            or relative.is_absolute()
            or not name
            or any(part in ("", ".", "..") for part in relative.parts)
        ):
            die(f"SHA256SUMS contains unsafe entry: {raw}")
        target = root.joinpath(*relative.parts)
        try:
            target.resolve().relative_to(root.resolve())
        except ValueError:
            die(f"SHA256SUMS entry escapes bundle root: {name}")
        regular_file(target, "SHA256SUMS target")
        actual = sha256(target)
        if actual.lower() != expected.lower():
            die(f"SHA256SUMS mismatch for {name}: expected {expected}, got {actual}")
        if name in verified:
            die(f"SHA256SUMS contains duplicate entry: {name}")
        verified.add(name)
    if not verified:
        die("SHA256SUMS is empty")
    return verified


def verify_bundle(root: Path, expected_version: str, expected_commit: str, expected_run_id: str) -> dict[str, str]:
    if root.is_symlink() or not root.is_dir():
        die(f"package bundle must be a regular directory: {root}")
    root = root.resolve()
    buildinfo = root / "BUILDINFO"
    sums = root / "SHA256SUMS"
    runtime = root / "bin/oasis7_chain_runtime"
    regular_file(buildinfo, "BUILDINFO")
    regular_file(sums, "SHA256SUMS")
    regular_file(runtime, "runtime executable")
    info = parse_buildinfo(buildinfo)
    if info["package_version"] != expected_version:
        die(f"BUILDINFO package_version {info['package_version']!r} does not match CLI {expected_version!r}")
    if info["commit"].lower() != expected_commit.lower():
        die(f"BUILDINFO commit {info['commit']!r} does not match CLI {expected_commit!r}")
    if info["run_id"] != expected_run_id:
        die(f"BUILDINFO run_id {info['run_id']!r} does not match CLI {expected_run_id!r}")
    verified = verify_sums(root, sums)
    required = {"BUILDINFO", "bin/oasis7_chain_runtime"}
    missing = sorted(required - verified)
    if missing:
        die("SHA256SUMS does not cover required files: " + ", ".join(missing))
    if not (runtime.lstat().st_mode & 0o111):
        die(f"runtime executable is not executable: {runtime}")
    return info


if __name__ == "__main__":
    if len(sys.argv) != 5:
        die("usage: p2p-verify-linux-package-bundle.py <bundle-root> <package-version> <commit> <run-id>")
    verify_bundle(Path(sys.argv[1]), sys.argv[2], sys.argv[3], sys.argv[4])
