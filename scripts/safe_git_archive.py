"""Bounded, preflighted archive extraction. No partial tree escapes this context."""
from __future__ import annotations
from contextlib import contextmanager
from dataclasses import dataclass
from enum import Enum
from pathlib import Path, PurePosixPath
import os
import gzip
import bz2
import re
import selectors
import shutil
import subprocess
import tarfile
import tempfile
import time

class ArchivePolicy(Enum):
    AUTHORITY = "authority"
    CARGO_SNAPSHOT = "cargo"

@dataclass(frozen=True)
class ArchiveLimits:
    archive_bytes: int = 512 * 1024 * 1024
    members: int = 100000
    member_bytes: int = 128 * 1024 * 1024
    expanded_bytes: int = 1024 * 1024 * 1024
    seconds: float = 120
    metadata_bytes: int = 4 * 1024 * 1024
    symlink_hops: int = 64

@dataclass(frozen=True)
class ExtractedTree:
    root: Path

def _path(name):
    if not name or "\x00" in name or "\\" in name or re.match(r"^[A-Za-z]:", name):
        raise ValueError("invalid archive path")
    p = PurePosixPath(name)
    if p.is_absolute():
        raise ValueError("tar archive contains absolute member")
    if ".." in p.parts:
        raise ValueError("tar archive contains unsafe member path")
    return p

@contextmanager
def _bounded_tar(archive_path, limits, started):
    # tarfile interprets PAX/GNU headers before yielding members. Inflate to a
    # bounded disk spool and inspect physical headers before giving it any input.
    with open(archive_path, "rb") as source:
        magic = source.read(6)
        source.seek(0)
        if magic.startswith(b"\x1f\x8b"):
            decoded = gzip.GzipFile(fileobj=source)
        elif magic.startswith(b"BZh"):
            decoded = bz2.BZ2File(source)
        elif magic.startswith(b"\xfd7zXZ\x00"):
            raise ValueError("XZ archives are not supported by this local policy")
        else:
            decoded = source
        with tempfile.TemporaryFile() as spool:
            bound = limits.expanded_bytes + limits.members * 1024 + 10240
            size = 0
            while True:
                if time.monotonic() - started > limits.seconds:
                    raise ValueError("archive decompression timed out")
                chunk = decoded.read(min(65536, max(1, bound - size + 1)))
                if not chunk: break
                size += len(chunk)
                if size > bound: raise ValueError("archive decompressed byte limit exceeded")
                spool.write(chunk)
            spool.seek(0)
            physical_members = 0
            metadata_bytes = 0
            terminated = False
            while True:
                if time.monotonic() - started > limits.seconds:
                    raise ValueError("archive header preflight timed out")
                header = spool.read(512)
                if not header: break
                if len(header) != 512: raise ValueError("truncated tar header")
                if header == bytes(512):
                    if spool.read(512) != bytes(512): raise ValueError("truncated tar terminator")
                    terminated = True
                    while True:
                        if time.monotonic() - started > limits.seconds:
                            raise ValueError("archive terminator preflight timed out")
                        padding = spool.read(65536)
                        if not padding: break
                        if any(padding): raise ValueError("nonzero content after tar terminator")
                    break
                member = tarfile.TarInfo.frombuf(header, "utf-8", "surrogateescape")
                physical_members += 1
                if physical_members > limits.members: raise ValueError("archive member limit exceeded")
                if member.size < 0 or member.size > limits.member_bytes:
                    raise ValueError("archive member byte limit exceeded")
                if member.type in (tarfile.XHDTYPE, tarfile.XGLTYPE, tarfile.GNUTYPE_LONGNAME, tarfile.GNUTYPE_LONGLINK):
                    metadata_bytes += member.size
                    if member.size > 65536 or metadata_bytes > limits.metadata_bytes:
                        raise ValueError("archive extended header limit exceeded")
                end = spool.tell() + ((member.size + 511) // 512) * 512
                if end > size: raise ValueError("truncated tar member")
                spool.seek(end)
            if not terminated: raise ValueError("missing tar terminator")
            spool.seek(0)
            with tarfile.open(fileobj=spool, mode="r:") as archive:
                yield archive
        if decoded is not source: decoded.close()

def extract_archive(archive_path, destination, *, policy=ArchivePolicy.AUTHORITY, limits=ArchiveLimits()):
    """Destination must be absent; errors remove the entire newly created tree."""
    destination = Path(destination)
    if destination.exists() or destination.is_symlink():
        raise ValueError("archive destination already exists")
    if Path(archive_path).stat().st_size > limits.archive_bytes:
        raise ValueError("archive byte limit exceeded")
    started = time.monotonic()
    try:
        with _bounded_tar(archive_path, limits, started) as archive:
            entries = {}
            total = 0
            # Iterate rather than getmembers: enforce bounds before retaining descriptors.
            for member in archive:
                if time.monotonic() - started > limits.seconds:
                    raise ValueError("archive validation timed out")
                path = _path(member.name)
                if path in entries or len(entries) >= limits.members:
                    raise ValueError("duplicate member or member limit exceeded")
                if member.size < 0 or member.size > limits.member_bytes:
                    raise ValueError("archive member byte limit exceeded")
                total += member.size
                if total > limits.expanded_bytes:
                    raise ValueError("archive expanded byte limit exceeded")
                if not (member.isdir() or member.isfile() or (member.issym() and policy == ArchivePolicy.CARGO_SNAPSHOT)):
                    raise ValueError("tar archive contains non-regular member")
                entries[path] = member
            if not entries:
                raise ValueError("empty archive")
            # Resolve links recursively before any write; parent links are disallowed.
            def link_target(path):
                seen = set()
                while True:
                    if path in seen or len(seen) >= limits.symlink_hops:
                        raise ValueError("archive symlink cycle or hop limit")
                    member = entries.get(path)
                    if not member or not member.issym(): return path
                    seen.add(path)
                    target = member.linkname
                    if not target or "\\" in target or re.match(r"^[A-Za-z]:", target) or target.startswith("/"):
                        raise ValueError("unsafe symlink target")
                    parts = list(path.parent.parts)
                    for part in PurePosixPath(target).parts:
                        if part == "..":
                            if not parts: raise ValueError("symlink escapes archive")
                            parts.pop()
                        elif part != ".": parts.append(part)
                    path = PurePosixPath(*parts)
                    for parent in path.parents:
                        if parent in entries and not entries[parent].isdir():
                            raise ValueError("symlink target traverses a non-directory")
            for path, member in entries.items():
                for parent in path.parents:
                    if parent in entries and not entries[parent].isdir():
                        raise ValueError("archive path traverses non-directory")
                if member.issym(): link_target(path)
            destination.mkdir(parents=True)
            # Explicit filter is independent of Python's changing default; links
            # are created last after our stricter chain validation.
            for path, member in entries.items():
                if member.issym(): continue
                if time.monotonic() - started > limits.seconds:
                    raise ValueError("archive extraction timed out")
                # Explicit standard-library policy, including on Python before
                # tarfile added filters. Never invoke a version-dependent default.
                target = destination.joinpath(*_path(member.name).parts)
                if not target.resolve().is_relative_to(destination.resolve()):
                    raise ValueError("extraction target escapes root")
                if member.isdir():
                    target.mkdir(parents=True, exist_ok=True)
                else:
                    target.parent.mkdir(parents=True, exist_ok=True)
                    source = archive.extractfile(member)
                    if source is None: raise ValueError("unreadable member")
                    with source, target.open("xb") as output:
                        remaining = member.size
                        while remaining:
                            if time.monotonic() - started > limits.seconds:
                                raise ValueError("archive extraction timed out")
                            chunk = source.read(min(65536, remaining))
                            if not chunk: raise ValueError("truncated archive member")
                            output.write(chunk)
                            remaining -= len(chunk)
                    target.chmod((member.mode & 0o777) or 0o644)
            for path, member in entries.items():
                if member.issym():
                    target = destination.joinpath(*path.parts)
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.symlink_to(member.linkname)
                    if not target.resolve().is_relative_to(destination.resolve()):
                        raise ValueError("symlink extraction escapes root")
    except BaseException:
        if destination.exists(): shutil.rmtree(destination)
        raise

@contextmanager
def extracted_git_tree(repo, commit_oid, *, policy=ArchivePolicy.CARGO_SNAPSHOT, limits=ArchiveLimits()):
    if not re.fullmatch(r"[0-9a-fA-F]{40}", commit_oid):
        raise ValueError("archive requires a fixed commit OID")
    actual = subprocess.check_output(["git", "-C", str(repo), "rev-parse", "--verify", commit_oid + "^{commit}"], text=True, timeout=limits.seconds).strip()
    if actual != commit_oid.lower(): raise ValueError("OID is not a commit")
    with tempfile.TemporaryDirectory(prefix="oasis7-git-tree-") as temporary:
        root = Path(temporary)
        archive_path = root / "source.tar"
        process = subprocess.Popen(["git", "-C", str(repo), "archive", "--format=tar", commit_oid], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        deadline = time.monotonic() + limits.seconds
        size = 0
        errors = bytearray()
        try:
            with selectors.DefaultSelector() as selector, archive_path.open("wb") as output:
                selector.register(process.stdout, selectors.EVENT_READ, "stdout")
                selector.register(process.stderr, selectors.EVENT_READ, "stderr")
                while selector.get_map():
                    remaining = deadline - time.monotonic()
                    if remaining <= 0: raise ValueError("git archive timed out")
                    for key, _ in selector.select(min(remaining, 0.1)):
                        chunk = os.read(key.fileobj.fileno(), 65536)
                        if not chunk:
                            selector.unregister(key.fileobj)
                        elif key.data == "stdout":
                            size += len(chunk)
                            if size > limits.archive_bytes: raise ValueError("git archive byte limit exceeded")
                            output.write(chunk)
                        else:
                            errors.extend(chunk[:max(0, 8192-len(errors))])
            if process.wait(timeout=max(0.01, deadline-time.monotonic())):
                raise ValueError("git archive failed: " + errors.decode(errors="replace"))
            extract_archive(archive_path, root / "tree", policy=policy, limits=limits)
            yield ExtractedTree(root / "tree")
        finally:
            if process.poll() is None: process.kill()
            process.wait()
            process.stdout.close()
            process.stderr.close()
