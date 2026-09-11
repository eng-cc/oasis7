#!/usr/bin/env python3
"""Code-owned, independently pinned managed-peer snapshot; no caller overrides."""

import hashlib
import json
import os
from pathlib import Path
import re
import stat
from types import MappingProxyType


REGISTRY_PATH = Path("/operator/truth/managed-peer-registry.json")
REGISTRY_SHA256 = None
SCHEMA = "oasis7.managed_peer_registry.v1"
NETWORK_ID = "oasis7-public-testnet-governed-20260606"
NODE_IDS = MappingProxyType({
    "storage-205": "triad-testnet-storage",
    "sequencer-204": "triad-testnet-sequencer",
    "linux-lan-observer": "triad-testnet-local",
    "windows-observer": "triad-testnet-windows-observer",
    "macos-observer": "triad-testnet-fourth-local",
})


def fail(message):
    raise SystemExit(f"error: managed peer registry: {message}")


def protected_paths():
    return (Path(__file__), Path(REGISTRY_PATH))


def _object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            fail("duplicate JSON field")
        result[key] = value
    return result


def _identity(value):
    return (value.st_dev, value.st_ino, value.st_mode, value.st_uid,
            value.st_size, value.st_mtime_ns, value.st_ctime_ns)


def load_snapshot():
    """Read fresh exact pinned bytes and return immutable digest/epoch/peer maps."""
    if not isinstance(REGISTRY_SHA256, str) or not re.fullmatch(r"[0-9a-fA-F]{64}", REGISTRY_SHA256):
        fail("digest pin is not provisioned")
    path = Path(REGISTRY_PATH)
    if not path.is_absolute():
        fail("code-owned path is not absolute")
    try:
        for ancestor in (path, *path.parents):
            if ancestor.is_symlink() and ancestor not in (Path("/var"), Path("/tmp")):
                fail("authority path must not contain symlinks")
            if ancestor != path:
                try:
                    directory = ancestor.stat()
                except OSError as error:
                    fail(f"cannot stat authority ancestor: {error.__class__.__name__}")
                if not stat.S_ISDIR(directory.st_mode):
                    fail("authority has a non-directory ancestor")
                if directory.st_uid not in {0, os.getuid()}:
                    fail("authority has a foreign-owned replaceable ancestor")
                mode = stat.S_IMODE(directory.st_mode)
                sticky_safe = bool(mode & stat.S_ISVTX) and directory.st_uid == 0
                if mode & 0o022 and not sticky_safe:
                    fail("authority has an unauthorized-writable ancestor")
        descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
        try:
            before = os.fstat(descriptor)
            if not stat.S_ISREG(before.st_mode):
                fail("authority is not a regular file")
            if hasattr(os, "getuid") and before.st_uid != os.getuid():
                fail("authority is not owner-owned")
            if os.name != "nt" and stat.S_IMODE(before.st_mode) != 0o600:
                fail("authority must have mode 0600")
            with os.fdopen(descriptor, "rb", closefd=False) as stream:
                raw = stream.read()
            if _identity(before) != _identity(os.fstat(descriptor)) or _identity(before) != _identity(os.stat(path, follow_symlinks=False)):
                fail("authority changed while reading")
        finally:
            os.close(descriptor)
    except OSError as error:
        fail(f"authority unavailable: {error.__class__.__name__}")
    digest = hashlib.sha256(raw).hexdigest()
    if digest != REGISTRY_SHA256.lower():
        fail("digest pin mismatch")
    try:
        value = json.loads(raw, object_pairs_hook=_object)
    except (ValueError, UnicodeError):
        fail("malformed JSON")
    if not isinstance(value, dict) or set(value) != {"schema_version", "network_id", "registry_epoch", "nodes"}:
        fail("snapshot fields are not exact")
    if value["schema_version"] != SCHEMA or value["network_id"] != NETWORK_ID:
        fail("schema or network mismatch")
    epoch = value["registry_epoch"]
    if not isinstance(epoch, str) or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}", epoch):
        fail("registry epoch is malformed")
    nodes = value["nodes"]
    if not isinstance(nodes, list) or len(nodes) != len(NODE_IDS):
        fail("snapshot must contain exactly five nodes")
    peers = {}
    for node in nodes:
        if not isinstance(node, dict) or set(node) != {"node_name", "node_id", "peer_id"}:
            fail("node fields are not exact")
        name = node["node_name"]
        if not isinstance(name, str) or name not in NODE_IDS or name in peers or node["node_id"] != NODE_IDS[name]:
            fail("node identity mismatch or duplicate")
        peer = node["peer_id"]
        if not isinstance(peer, str) or not re.fullmatch(r"[A-Za-z0-9]{1,256}", peer) or peer in peers.values():
            fail("peer identity is malformed or duplicate")
        peers[name] = peer
    return MappingProxyType({"sha256": digest, "registry_epoch": epoch, "peers": MappingProxyType(peers)})
