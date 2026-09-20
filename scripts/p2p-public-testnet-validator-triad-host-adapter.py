#!/usr/bin/env python3
"""Governed FD-only transport for the existing public-testnet validator pair.

The adapter is intentionally narrower than the retired rebuild shell path. It
accepts only a transaction-bound ``triad_staggered`` callback, uses the fixed
pair inventory and pinned host keys, and authenticates SSH through inherited
file descriptors. It never accepts a plaintext credential value or falls back
to ambient SSH credentials. Missing identity-v2 admission or remote evidence
is a capability blocker.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import importlib.util
import json
import os
import re
import shlex
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path
from typing import Any, NoReturn
from urllib.parse import urlsplit


ROOT = Path(__file__).resolve().parents[1]
EXECUTOR_PATH = ROOT / "scripts" / "p2p-public-testnet-validator-pair-rebuild.py"
INVENTORY_PATH = ROOT / "scripts" / "public-testnet-validator-pair-inventory.v1.json"
TRIAD_INVENTORY_PATH = ROOT / "scripts" / "public-testnet-validator-triad-inventory.v1.json"
TRIAD_EVIDENCE_ROOT = ROOT / "doc" / "testing" / "evidence"
STACK_ROOT = "/opt/oasis7/p2p-testnet"
KNOWN_HOSTS_PATH = Path(STACK_ROOT) / "config" / "public-testnet-validator-pair-known-hosts"
PACKAGE_DEB_NAME = "oasis7-linux-x64.deb"
OPS_TOOLS_NAME = "oasis7-linux-x64-ops-tools.tar.gz"
BUILDINFO_NAME = "linux-x64-BUILDINFO"
PACKAGE_SUMS_NAME = "linux-x64-SHA256SUMS"
OPS_SUMS_NAME = "linux-x64-ops-tools-SHA256SUMS"
PUBLIC_MANIFEST_NAME = "public-testnet-governance-public-signers-2026-06-05.json"
TRIAD_INVENTORY_SHA256 = "b983bd9df4f29bf7a0e32dd9d1d56323d85d16d4cf572c10bc8c567908565739"
TRIAD_SOURCE_REGISTRY = "public-testnet-governed-bootstrap-validator-triad-registry-2026-09-15.json"
TRIAD_BOOTSTRAP_PEERS = "public-testnet-governed-bootstrap-validator-triad-bootstrap-peers-2026-09-15.txt"
TRIAD_SOURCE_REGISTRY_SHA256 = "1296accdac21371a017797b29503aa6658ed600f71049db0599ae3d12c651510"
TRIAD_BOOTSTRAP_PEERS_SHA256 = "5e62e5b132fe083c18c637213baefebad161c675e2243b2e02dc8bcd5f70401c"
TRIAD_GENERATED_REGISTRY_SHA256 = "1290818e16b4d5f6fa1929a18e0d6c73295d0ff86e8c7ee8ee58e670874199fd"
TRIAD_GENERATED_REGISTRY_SEMANTIC_SHA256 = "82b3b705cc72173b34fc738f9cff00cba2f4cab0f05ca32ca58bf4dfd5ea7228"
RESET_SURFACES = (
    "data/execution-records",
    "data/execution-world",
    "data/execution-world-simulator-mirror",
    "data/storage",
    "data/runtime-root",
    "data/replication-root",
    "output/chain-runtime",
    "output/node-distfs",
)
MUTATION_ORDER = ("storage-205", "sequencer-204")
PHASES = {"staggered-preflight", "staggered-storage", "staggered-sequencer", "staggered-rollback"}
BACKUP_PHASES = {"staggered-storage-backup", "staggered-sequencer-backup"}
PHASES |= BACKUP_PHASES
ROLE_ALIASES = {"storage-205": "storage", "sequencer-204": "sequencer"}
SHARED_FD_ENV = "OASIS7_TRIAD_ADAPTER_SHARED_FD"
STORAGE_FD_ENV = "OASIS7_TRIAD_ADAPTER_STORAGE_FD"
SEQUENCER_FD_ENV = "OASIS7_TRIAD_ADAPTER_SEQUENCER_FD"
TRANSPORT_ENV = "OASIS7_TRIAD_ADAPTER_CREDENTIAL_TRANSPORT"
HEX64 = re.compile(r"^[0-9a-fA-F]{64}$")
SAFE_VERSION = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_.+:~-]*$")
MIN_REMOTE_BACKUP_INODES = 128


class AdapterExit(SystemExit):
    """Fail closed with a stable process exit code and inspectable reason."""

    def __init__(self, message: str):
        self.message = message
        super().__init__(2)

    def __str__(self) -> str:
        return self.message


def fail(message: str) -> NoReturn:
    print(f"error: triad host adapter: {message}", file=sys.stderr)
    raise AdapterExit(message)


def load_executor() -> Any:
    spec = importlib.util.spec_from_file_location("oasis7_triad_adapter_executor", EXECUTOR_PATH)
    if spec is None or spec.loader is None:
        fail("repository executor is unavailable")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


EXECUTOR = load_executor()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def regular_file(path: Path, label: str) -> Path:
    try:
        metadata = os.lstat(path)
    except OSError as error:
        fail(f"{label} unavailable: {error.__class__.__name__}")
    if not stat.S_ISREG(metadata.st_mode):
        fail(f"{label} must be a regular file")
    return path.resolve()


def safe_tree(path: Path, label: str) -> Path:
    if path.is_symlink() or not path.is_dir():
        fail(f"{label} must be a real directory")
    for entry in path.rglob("*"):
        if entry.is_symlink() or not (entry.is_file() or entry.is_dir()):
            fail(f"{label} contains an unsupported entry")
    return path.resolve()


def load_json(path: Path, label: str) -> dict[str, Any]:
    regular_file(path, label)
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        fail(f"{label} is malformed: {error.__class__.__name__}")
    if not isinstance(value, dict):
        fail(f"{label} must be a JSON object")
    return value


def credential_fds() -> dict[str, int]:
    if os.environ.get(TRANSPORT_ENV) != "fd-only-v1":
        fail("capability_blocked: adapter FD-only transport is not bound")
    if "SSHPASS" in os.environ or any(key.endswith("_SSHPASS") for key in os.environ):
        fail("capability_blocked: plaintext credential environment is forbidden")
    shared = os.environ.get(SHARED_FD_ENV)
    storage = os.environ.get(STORAGE_FD_ENV)
    sequencer = os.environ.get(SEQUENCER_FD_ENV)
    if shared is not None and (storage is not None or sequencer is not None):
        fail("adapter credential descriptor mapping is ambiguous")
    if shared is None and (storage is None or sequencer is None):
        fail("capability_blocked: one shared or two role-specific credential descriptors are required")
    raw = {"shared": shared} if shared is not None else {"storage-205": storage, "sequencer-204": sequencer}
    values: dict[str, int] = {}
    for role, text in raw.items():
        if not isinstance(text, str) or not re.fullmatch(r"[0-9]+", text):
            fail(f"adapter credential descriptor is malformed for {role}")
        value = int(text)
        if value <= 2:
            fail(f"adapter credential descriptor is reserved for {role}")
        try:
            os.fstat(value)
        except OSError:
            fail(f"capability_blocked: adapter credential descriptor is unavailable for {role}")
        values[role] = value
    return values


def validate_identity_v2(transaction: dict[str, Any]) -> None:
    proof = transaction.get("proof")
    admission = proof.get("identity_v2") if isinstance(proof, dict) else None
    if not isinstance(admission, dict) or admission.get("verified") is not True:
        fail("capability_blocked: identity-v2 admission is unavailable or unverified")
    if admission.get("status") != "verified":
        fail("capability_blocked: identity-v2 admission status is not verified")
    receipts = admission.get("receipts")
    if not isinstance(receipts, list) or not receipts:
        fail("capability_blocked: authenticated identity-v2 receipts are required")
    for item in receipts:
        if not isinstance(item, dict) or not isinstance(item.get("path"), str):
            fail("capability_blocked: identity-v2 receipt path is missing")
        if not isinstance(item.get("sha256"), str) or not HEX64.fullmatch(item["sha256"]):
            fail("capability_blocked: identity-v2 receipt digest is missing")
        path = regular_file(Path(item["path"]), "identity-v2 receipt")
        if item["sha256"].lower() != sha256_file(path):
            fail("capability_blocked: identity-v2 receipt digest mismatch")
        value = load_json(path, "identity-v2 receipt")
        if value.get("schema_version") != "oasis7.identity_v2_verification_receipt.v1":
            fail("capability_blocked: identity-v2 receipt schema is not the governed verification receipt")
        if (
            value.get("mode") != "current_admission"
            or value.get("verified") is not True
            or value.get("apply_authorized") is not True
            or value.get("historical_only") is not False
        ):
            fail("capability_blocked: identity-v2 receipt is not current-admission evidence")


def load_inventory() -> tuple[dict[str, Any], Path]:
    triad_inventory = regular_file(TRIAD_INVENTORY_PATH, "triad inventory")
    if sha256_file(triad_inventory) != TRIAD_INVENTORY_SHA256:
        fail("triad inventory digest is not the fixed governed value")
    for filename, expected in (
        (TRIAD_SOURCE_REGISTRY, TRIAD_SOURCE_REGISTRY_SHA256),
        (TRIAD_BOOTSTRAP_PEERS, TRIAD_BOOTSTRAP_PEERS_SHA256),
    ):
        evidence = regular_file(TRIAD_EVIDENCE_ROOT / filename, f"triad evidence {filename}")
        if sha256_file(evidence) != expected:
            fail(f"triad evidence digest mismatch for {filename}")
    inventory = load_json(INVENTORY_PATH, "pair inventory")
    if inventory.get("schema_version") != EXECUTOR.DEPLOYMENT_INVENTORY_SCHEMA or inventory.get("repository") != "eng-cc/oasis7" or inventory.get("network_tier") != "public_testnet":
        fail("pair inventory is not governed public-testnet truth")
    if inventory.get("stack_root") != STACK_ROOT or inventory.get("known_hosts_path") != str(KNOWN_HOSTS_PATH):
        fail("pair inventory root or known-hosts binding drifted")
    nodes = inventory.get("nodes")
    if not isinstance(nodes, dict) or set(nodes) != set(MUTATION_ORDER):
        fail("pair inventory must contain exactly storage-205 and sequencer-204")
    known_hosts = EXECUTOR._direct_canonical_known_hosts(str(KNOWN_HOSTS_PATH), "adapter pinned known-hosts")
    for role in MUTATION_ORDER:
        expected = EXECUTOR.HUMAN_DIRECT_SSH_CANONICAL[role]
        node = nodes[role]
        if not isinstance(node, dict) or any(node.get(key) != expected[key] for key in ("host", "root", "service", "host_key_fingerprint")):
            fail(f"pair inventory binding mismatch for {role}")
        EXECUTOR._direct_known_host_pin(known_hosts, node["host"], node["host_key_fingerprint"], role)
    return inventory, known_hosts


def validate_package(transaction: dict[str, Any]) -> dict[str, Any]:
    package = transaction.get("package")
    if not isinstance(package, dict):
        fail("package binding is missing")
    directory = safe_tree(Path(str(package.get("directory", ""))), "package directory")
    package_tree = EXECUTOR.inventory_tree(directory)
    if package.get("package_sha256") != package_tree.get("sha256"):
        fail("package directory provenance digest mismatch")
    version = package.get("version")
    commit = package.get("commit")
    run_id = str(package.get("run_id", ""))
    if not isinstance(version, str) or not SAFE_VERSION.fullmatch(version) or ".." in version:
        fail("package version is unsafe")
    if not isinstance(commit, str) or not re.fullmatch(r"[0-9a-fA-F]{40}", commit):
        fail("package commit binding is malformed")
    if not run_id or not run_id.isdigit():
        fail("package run id binding is malformed")
    buildinfo = regular_file(directory / BUILDINFO_NAME, "package BUILDINFO")
    info: dict[str, str] = {}
    for line in buildinfo.read_text(encoding="utf-8").splitlines():
        key, separator, value = line.partition("=")
        if separator:
            info[key] = value
    for key, expected in (("commit", commit), ("package_version", version), ("run_id", run_id), ("platform", "linux-x64")):
        if info.get(key) != expected:
            fail(f"package BUILDINFO {key} binding mismatch")
    files = {name: regular_file(directory / name, f"package asset {name}") for name in (PACKAGE_DEB_NAME, OPS_TOOLS_NAME)}
    for sums_name, target_name in ((PACKAGE_SUMS_NAME, PACKAGE_DEB_NAME), (OPS_SUMS_NAME, OPS_TOOLS_NAME)):
        sums = regular_file(directory / sums_name, f"package checksum file {sums_name}")
        matches = []
        for line in sums.read_text(encoding="utf-8").splitlines():
            fields = line.split()
            if len(fields) == 2 and fields[1].lstrip("*") == target_name:
                matches.append(fields[0])
        if matches != [sha256_file(files[target_name])]:
            fail(f"package checksum binding mismatch for {target_name}")
    runtime_relpath = package.get("runtime_relpath")
    runtime_sha256 = package.get("runtime_sha256")
    runtime_size_bytes = package.get("runtime_size_bytes")
    if not isinstance(runtime_relpath, str) or not runtime_relpath or Path(runtime_relpath).is_absolute() or ".." in Path(runtime_relpath).parts:
        fail("package runtime path binding is malformed")
    runtime = regular_file(directory / runtime_relpath, "package runtime")
    if not isinstance(runtime_sha256, str) or sha256_file(runtime) != runtime_sha256:
        fail("package runtime digest binding mismatch")
    if not isinstance(runtime_size_bytes, int) or runtime.stat().st_size != runtime_size_bytes:
        fail("package runtime size binding mismatch")
    return {"directory": directory, "version": version, "commit": commit, "run_id": run_id, "files": files}


def validate_governed(transaction: dict[str, Any]) -> dict[str, Path]:
    network = transaction.get("network")
    governed = network.get("governed") if isinstance(network, dict) else None
    if not isinstance(governed, dict) or set(governed) != {"manifest", "genesis", "registry", "bootstrap", "world"}:
        fail("governed deployment-stage inputs are incomplete")
    result: dict[str, Path] = {}
    for key, value in governed.items():
        if not isinstance(value, dict) or not isinstance(value.get("path"), str):
            fail(f"governed {key} binding is malformed")
        path = safe_tree(Path(value["path"]), f"governed {key}") if value.get("kind") == "directory" else regular_file(Path(value["path"]), f"governed {key}")
        expected = value.get("sha256") or value.get("sha256_tree")
        if not isinstance(expected, str) or not HEX64.fullmatch(expected):
            fail(f"governed {key} digest is missing")
        if value.get("kind") != "directory" and sha256_file(path) != expected:
            fail(f"governed {key} digest mismatch")
        if value.get("kind") == "directory":
            actual = EXECUTOR.inventory_tree(path)
            if actual.get("sha256") != expected and actual.get("sha256_tree") != expected:
                fail(f"governed {key} tree digest mismatch")
        if key == "registry":
            if sha256_file(path) != TRIAD_GENERATED_REGISTRY_SHA256:
                fail("triad governed registry digest is not the fixed generated value")
            registry = load_json(path, "triad governed registry")
            validators = registry.get("validators")
            expected_ids = {
                "triad-testnet-storage",
                "triad-testnet-sequencer",
                "triad-testnet-validator-47",
            }
            if not isinstance(validators, list) or len(validators) != 3:
                fail("triad governed registry must contain exactly three validators")
            actual_ids = {
                item.get("node_id")
                for item in validators
                if isinstance(item, dict) and isinstance(item.get("node_id"), str)
            }
            if len(actual_ids) != 3 or actual_ids != expected_ids:
                fail("triad governed registry must contain exactly three validators")
            if any(
                not isinstance(item, dict)
                or not isinstance(item.get("finality_signer_public_key"), str)
                or not isinstance(item.get("stake"), int)
                for item in validators
            ):
                fail("triad governed registry validator bindings are incomplete")
            canonical = {
                "signer_bindings": {
                    f"governance.finality.v1.{item['node_id']}": str(item["finality_signer_public_key"]).lower()
                    for item in validators
                },
                "slot_id": registry.get("slot_id"),
                "threshold": registry.get("threshold"),
                "threshold_bps": registry.get("threshold_bps"),
                "validator_stakes": {
                    f"governance.finality.v1.{item['node_id']}": item["stake"]
                    for item in validators
                },
            }
            semantic = hashlib.sha256(
                json.dumps(canonical, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
            ).hexdigest()
            if semantic != TRIAD_GENERATED_REGISTRY_SEMANTIC_SHA256:
                fail("triad governed registry semantic digest is not the fixed generated value")
        result[key] = path
    return result


def validate_transaction(transaction: dict[str, Any], phase: str) -> tuple[dict[str, Any], Path, dict[str, Any], dict[str, Path]]:
    if transaction.get("execution_mode") != "triad_staggered":
        fail("capability_blocked: adapter accepts only execution_mode=triad_staggered")
    if phase not in PHASES:
        fail("unsupported adapter phase")
    binding = transaction.get("adapter_binding")
    if not isinstance(binding, dict) or binding.get("phase") != phase:
        fail("adapter binding is missing or phase-mismatched")
    if binding.get("credential_transport") != "fd-only-v1":
        fail("capability_blocked: adapter credential transport is not fd-only-v1")
    if binding.get("transaction_id") != transaction.get("transaction_id") or binding.get("plan_digest") != transaction.get("plan_digest"):
        fail("adapter transaction/plan binding mismatch")
    if binding.get("repository_executable") != EXECUTOR.repository_executable_identity():
        fail("adapter executor provenance mismatch")
    if not isinstance(transaction.get("transaction_id"), str) or not transaction["transaction_id"].strip():
        fail("transaction id is missing")
    validate_identity_v2(transaction)
    package = validate_package(transaction)
    governed = validate_governed(transaction)
    inventory, known_hosts = load_inventory()
    return inventory, known_hosts, package, governed


def clean_environment() -> dict[str, str]:
    environment = os.environ.copy()
    for key in list(environment):
        if key == "SSHPASS" or key.endswith("_SSHPASS"):
            environment.pop(key, None)
    return environment


class FixedSSH:
    def __init__(self, inventory: dict[str, Any], known_hosts: Path, fds: dict[str, int]):
        self.inventory = inventory
        self.known_hosts = known_hosts
        self.fds = fds
        self.sshpass = shutil.which("sshpass")
        self.ssh = shutil.which("ssh")
        if not self.sshpass or not self.ssh:
            fail("capability_blocked: ssh and sshpass are required")

    def fd_for(self, role: str) -> int:
        return self.fds.get(role, self.fds.get("shared", -1))

    def _argv(self, role: str, remote: str) -> list[str]:
        fd = self.fd_for(role)
        if fd <= 2:
            fail(f"credential descriptor is not bound for {role}")
        node = self.inventory["nodes"][role]
        return [
            self.sshpass, "-d", str(fd), self.ssh,
            "-o", "HostKeyAlgorithms=ssh-ed25519",
            "-o", "StrictHostKeyChecking=yes",
            "-o", f"UserKnownHostsFile={self.known_hosts}",
            "-o", "GlobalKnownHostsFile=/dev/null",
            "-o", "PreferredAuthentications=password",
            "-o", "PubkeyAuthentication=no",
            "-o", "NumberOfPasswordPrompts=1",
            "-o", "ControlMaster=no", "-o", "ControlPath=none",
            "-o", "LogLevel=ERROR", node["host"], remote,
        ]

    def command(self, role: str, remote: str, timeout: int = 60) -> str:
        fd = self.fd_for(role)
        try:
            result = subprocess.run(
                self._argv(role, remote), check=False, text=True, capture_output=True,
                timeout=timeout, env=clean_environment(), pass_fds=(fd,),
            )
        except (OSError, subprocess.TimeoutExpired) as error:
            fail(f"capability_blocked: strict SSH failed for {role}: {error.__class__.__name__}")
        if result.returncode != 0:
            fail(f"capability_blocked: strict SSH command failed for {role}")
        return result.stdout.strip()

    def remote_forensic_backup(
        self, role: str, transaction_id: str, required_bytes: int, required_inodes: int
    ) -> dict[str, Any]:
        """Capture a durable, remote-only forensic backup before mutation.

        The command is deliberately generated from the fixed inventory and the
        canonical reset-surface set.  It never receives a credential value in
        argv or the environment; ``command`` carries the inherited FD through
        the pinned ``_argv`` path.  A partial backup is left as a hidden
        ``.partial`` directory and is never promoted to the transaction-bound
        final path.
        """
        if not isinstance(transaction_id, str) or not SAFE_VERSION.fullmatch(transaction_id) or ".." in transaction_id:
            fail("capability_blocked: remote backup transaction id is unsafe")
        if (
            isinstance(required_bytes, bool)
            or not isinstance(required_bytes, int)
            or required_bytes <= 0
            or isinstance(required_inodes, bool)
            or not isinstance(required_inodes, int)
            or required_inodes < MIN_REMOTE_BACKUP_INODES
        ):
            fail("capability_blocked: remote backup code-owned capacity threshold is malformed")
        node = self.inventory["nodes"][role]
        backup_root = f"{STACK_ROOT}/backups/{transaction_id}"
        surfaces_json = json.dumps(list(RESET_SURFACES), ensure_ascii=True, separators=(",", ":"))
        remote = (f'''set -euo pipefail
python3 - {shlex.quote(STACK_ROOT)} {shlex.quote(backup_root)} {shlex.quote(role)} {shlex.quote(ROLE_ALIASES[role])} {shlex.quote(node["service"])} {shlex.quote(transaction_id)} {required_bytes} {required_inodes} <<'PY'
'''
        + r'''
import hashlib
import json
import os
import shutil
import stat
import subprocess
import sys
from pathlib import Path

root = Path(sys.argv[1])
backup_root = Path(sys.argv[2])
role = sys.argv[3]
role_alias = sys.argv[4]
service = sys.argv[5]
transaction_id = sys.argv[6]
required_bytes = int(sys.argv[7])
required_inodes = int(sys.argv[8])
reset_surfaces = json.loads(__SURFACES_JSON__)
partial_root = backup_root.parent / ("." + backup_root.name + ".partial")

def fail(message):
    raise SystemExit(message)

if root.is_symlink() or not root.is_dir():
    fail("remote backup root is not a real directory")
backup_parent = root / "backups"
if backup_root.parent != backup_parent or backup_parent.is_symlink() or not backup_parent.is_dir():
    fail("remote backup path escapes the fixed stack root")
if backup_root.exists() or backup_root.is_symlink() or partial_root.exists() or partial_root.is_symlink():
    fail("remote backup transaction path already exists")
try:
    readback = subprocess.run(
        [
            str(root / "current/bin/service-readback"),
            "--read-only", "--role", role_alias, "--root", str(root), "--service", service,
        ], check=False, capture_output=True, text=True, timeout=30,
    )
except (OSError, subprocess.TimeoutExpired) as error:
    fail("remote backup service readback failed: " + error.__class__.__name__)
if readback.returncode != 0:
    fail("remote backup service readback returned a non-zero status")
try:
    service_value = json.loads(readback.stdout)
except json.JSONDecodeError:
    fail("remote backup service readback is not JSON")
if not isinstance(service_value, dict) or service_value.get("independently_observed") is not True:
    fail("remote backup service readback is not independently observed")
if (
    service_value.get("active") is not True
    or service_value.get("running") is not True
    or service_value.get("service_state") != "running"
):
    fail("remote backup target is not live before capture")
try:
    usage = shutil.disk_usage(root)
    statvfs = os.statvfs(root)
    free_bytes = int(usage.free)
    free_inodes = int(statvfs.f_favail)
except OSError as error:
    fail("remote backup capacity read failed: " + error.__class__.__name__)
if free_bytes < required_bytes or free_inodes < required_inodes:
    fail("remote backup capacity is below the code-owned threshold")

def metadata(path, relative):
    value = {"path": relative}
    info = path.lstat()
    if stat.S_ISLNK(info.st_mode) or not (stat.S_ISREG(info.st_mode) or stat.S_ISDIR(info.st_mode)):
        fail("remote reset surface contains a symlink or special entry: " + relative)
    value.update({"kind": "file" if stat.S_ISREG(info.st_mode) else "directory", "mode": stat.S_IMODE(info.st_mode), "uid": info.st_uid, "gid": info.st_gid})
    if stat.S_ISREG(info.st_mode):
        digest = hashlib.sha256()
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(chunk)
        value.update({"size_bytes": info.st_size, "sha256": digest.hexdigest()})
    return value

def copy_tree(source, destination, relative):
    value = metadata(source, relative)
    if source.is_dir():
        destination.mkdir(parents=True, exist_ok=False)
        for child in sorted(source.iterdir(), key=lambda item: item.name):
            child_relative = relative + "/" + child.name
            value.setdefault("children", []).append(copy_tree(child, destination / child.name, child_relative))
    else:
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, destination, follow_symlinks=False)
    return value

partial_root.mkdir(parents=True)
snapshot = partial_root / "snapshot"
snapshot.mkdir()
entries = []
try:
    for surface in reset_surfaces:
        source = root / surface
        destination = snapshot / surface
        if not source.exists() and not source.is_symlink():
            entries.append({"path": surface, "kind": "absent"})
            continue
        entries.append(copy_tree(source, destination, surface))
    manifest = {
        "schema_version": "oasis7.validator_pair_rebuild_remote_backup_manifest.v1",
        "role": role,
        "transaction_id": transaction_id,
        "root": str(root),
        "backup_root": str(backup_root),
        "reset_surfaces": reset_surfaces,
        "entries": entries,
        "backup_non_seed": {
            "forensic_only": True,
            "seed_eligible": False,
            "restore_deleted_chain_state": False,
        },
    }
    manifest_path = partial_root / "manifest.json"
    payload = json.dumps(manifest, ensure_ascii=True, sort_keys=True, separators=(",", ":")) + "\n"
    with manifest_path.open("w", encoding="utf-8") as handle:
        handle.write(payload)
        handle.flush()
        os.fsync(handle.fileno())
    directory_fd = os.open(str(partial_root), os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(directory_fd)
    finally:
        os.close(directory_fd)
    same_filesystem = os.stat(root).st_dev == os.stat(partial_root).st_dev
    if not same_filesystem:
        fail("remote backup is not on the fixed stack filesystem")
    os.replace(partial_root, backup_root)
    parent_fd = os.open(str(backup_root.parent), os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(parent_fd)
    finally:
        os.close(parent_fd)
    final_manifest = backup_root / "manifest.json"
    digest = hashlib.sha256(final_manifest.read_bytes()).hexdigest()
    print(json.dumps({
        "schema_version": "oasis7.validator_pair_rebuild_remote_backup_receipt.v1",
        "role": role,
        "transaction_id": transaction_id,
        "remote_target": True,
        "credential_transport": "fd-only-v1",
        "remote_root": str(root),
        "backup_root": str(backup_root),
        "manifest": str(final_manifest),
        "manifest_sha256": digest,
        "reset_surface_manifest_sha256": digest,
        "reset_surfaces": reset_surfaces,
        "capacity": {
            "verified": True,
            "same_filesystem": True,
            "available_bytes": free_bytes,
            "free_bytes": free_bytes,
            "required_bytes": required_bytes,
            "free_inodes": free_inodes,
            "required_inodes": required_inodes,
        },
        "backup_non_seed": {
            "forensic_only": True,
            "seed_eligible": False,
            "restore_deleted_chain_state": False,
        },
    }, ensure_ascii=True, sort_keys=True, separators=(",", ":")))
except BaseException:
    shutil.rmtree(partial_root, ignore_errors=True)
    raise
PY
''').replace("__SURFACES_JSON__", repr(surfaces_json))
        raw = self.command(role, remote, timeout=300)
        try:
            value = json.loads(raw)
        except json.JSONDecodeError:
            fail(f"capability_blocked: remote backup receipt is not JSON for {role}")
        if not isinstance(value, dict):
            fail(f"capability_blocked: remote backup receipt is not an object for {role}")
        # The remote command proves the backup contents; the fixed local
        # inventory binds that proof to the pinned host identity.  Keep this
        # binding explicit so resume cannot adopt a receipt from another host.
        value["remote_host"] = node["host"]
        return value

    @staticmethod
    def _tar_filter(info: tarfile.TarInfo) -> tarfile.TarInfo:
        if info.issym() or info.islnk() or not (info.isfile() or info.isdir()):
            raise tarfile.TarError("adapter archive rejects symlink or special entry")
        return info

    def archive(self, role: str, control_script: str, entries: list[tuple[Path, str]]) -> str:
        fd = self.fd_for(role)
        remote = (
            "set -euo pipefail; stage=$(mktemp -d /tmp/oasis7-triad-adapter.XXXXXX); "
            "trap 'rm -rf -- \"$stage\"' EXIT; tar -xzf - -C \"$stage\"; "
            "ASSET_ROOT=\"$stage\" bash \"$stage/control.sh\""
        )
        process: subprocess.Popen[bytes] | None = None
        try:
            process = subprocess.Popen(
                self._argv(role, remote), stdin=subprocess.PIPE,
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=clean_environment(), pass_fds=(fd,),
            )
            assert process.stdin is not None
            with tarfile.open(fileobj=process.stdin, mode="w|gz") as archive:
                control_fd, control_name = tempfile.mkstemp(prefix="oasis7-adapter-control-")
                os.close(control_fd)
                control_path = Path(control_name)
                try:
                    control_path.write_text(control_script, encoding="utf-8")
                    control_path.chmod(0o700)
                    archive.add(control_path, arcname="control.sh", recursive=False)
                    for source, arcname in entries:
                        archive.add(source, arcname=arcname, recursive=True, filter=self._tar_filter)
                finally:
                    control_path.unlink(missing_ok=True)
            process.stdin.close()
            # Let communicate drain both pipes concurrently; reading stdout
            # then stderr can deadlock if a remote command fills stderr while
            # its stdout remains quiet.
            process.stdin = None
            stdout_bytes, stderr_bytes = process.communicate(timeout=300)
            stdout = stdout_bytes.decode("utf-8", errors="replace")
            stderr = stderr_bytes.decode("utf-8", errors="replace")
            returncode = process.returncode
        except (OSError, tarfile.TarError, subprocess.TimeoutExpired) as error:
            if process is not None:
                process.kill()
            fail(f"capability_blocked: package/stage transport failed for {role}: {error.__class__.__name__}")
        if returncode != 0:
            del stderr
            fail(f"target adapter operation failed for {role}")
        return stdout


def readback(transport: FixedSSH, inventory: dict[str, Any], role: str) -> dict[str, Any]:
    node = inventory["nodes"][role]
    command = " ".join(
        [
            shlex.quote(STACK_ROOT + "/current/bin/service-readback"),
            "--read-only", "--role", ROLE_ALIASES[role],
            "--root", shlex.quote(STACK_ROOT),
            "--service", shlex.quote(node["service"]),
        ]
    )
    raw = transport.command(role, command)
    try:
        value = json.loads(raw)
    except json.JSONDecodeError:
        fail(f"remote service-readback is not JSON for {role}")
    if not isinstance(value, dict) or value.get("independently_observed") is not True:
        fail(f"remote service-readback is malformed for {role}")
    return value


def health_url_for_role(transaction: dict[str, Any], role: str) -> str:
    proof = transaction.get("proof")
    if not isinstance(proof, dict):
        fail("transaction health proof is missing")
    key = "storage_health_url" if role == "storage-205" else "sequencer_health_url"
    value = proof.get(key)
    if not isinstance(value, str) or not value:
        fail(f"{key} must be a local /healthz endpoint")
    try:
        parsed_url = urlsplit(value)
        hostname = parsed_url.hostname
    except ValueError:
        fail(f"{key} must be a local /healthz endpoint")
    if (
        parsed_url.scheme != "http"
        or hostname not in {"127.0.0.1", "localhost"}
        or parsed_url.path != "/healthz"
        or parsed_url.query
        or parsed_url.fragment
        or parsed_url.username is not None
        or parsed_url.password is not None
    ):
        fail(f"{key} must be a local /healthz endpoint")
    return value


def probe_healthz(transport: FixedSSH, role: str, health_url: str) -> dict[str, Any]:
    """Read the role's transaction-bound health endpoint over the pinned host."""
    command = f"curl --fail --silent --show-error --max-time 5 {shlex.quote(health_url)}"
    raw = transport.command(role, command)
    try:
        value = json.loads(raw)
    except json.JSONDecodeError:
        fail(f"remote /healthz is not JSON for {role}")
    if not isinstance(value, dict):
        fail(f"remote /healthz is not an object for {role}")
    required_health = ("ok", "ready", "last_error", "nrestarts", "oom_panic_segfault")
    if any(key not in value for key in required_health):
        fail(f"remote /healthz evidence is incomplete for {role}")
    if not isinstance(value["ok"], bool) or not isinstance(value["ready"], bool):
        fail(f"remote /healthz boolean fields are malformed for {role}")
    if value["last_error"] is not None and not isinstance(value["last_error"], str):
        fail(f"remote /healthz last_error field is malformed for {role}")
    if isinstance(value["nrestarts"], bool) or not isinstance(value["nrestarts"], int):
        fail(f"remote /healthz nrestarts field is malformed for {role}")
    if not isinstance(value["oom_panic_segfault"], bool):
        fail(f"remote /healthz crash field is malformed for {role}")
    if value["ok"] is not True or value["ready"] is not True or value["last_error"] is not None:
        fail(f"remote health readiness gate failed for {role}")
    if value["nrestarts"] != 0 or value["oom_panic_segfault"] is not False:
        fail(f"remote health restart/crash gate failed for {role}")
    return value


def observe_node(
    transport: FixedSSH,
    inventory: dict[str, Any],
    role: str,
    plan_root: str,
    *,
    running: bool,
    health_url: str | None = None,
) -> dict[str, Any]:
    # Validate the transaction-bound endpoint before any remote observation so
    # an operator cannot redirect a probe to an untrusted host or path.
    if not isinstance(health_url, str) or not health_url:
        fail(f"remote health URL is missing for {role}")
    try:
        parsed_health_url = urlsplit(health_url)
        hostname = parsed_health_url.hostname
    except ValueError:
        fail(f"remote health URL is not a local /healthz endpoint for {role}")
    if (
        parsed_health_url.scheme != "http"
        or hostname not in {"127.0.0.1", "localhost"}
        or parsed_health_url.path != "/healthz"
        or parsed_health_url.query
        or parsed_health_url.fragment
        or parsed_health_url.username is not None
        or parsed_health_url.password is not None
    ):
        fail(f"remote health URL is not a local /healthz endpoint for {role}")
    value = readback(transport, inventory, role)
    expected_state = (value.get("active") is True and value.get("running") is True and value.get("service_state") == "running")
    if expected_state != running:
        fail(f"remote service state gate failed for {role}")
    health = probe_healthz(transport, role, health_url)
    if health["ok"] is not running:
        fail(f"remote health/service state mismatch for {role}")
    runtime = transport.command(role, f"sha256sum {shlex.quote(STACK_ROOT + '/current/bin/oasis7_chain_runtime')} | awk '{{print $1}}'")
    return {
        "role": role,
        "root": plan_root,
        "remote_root": STACK_ROOT,
        "active": bool(value.get("active")),
        "running": bool(value.get("running")),
        "service_state": value.get("service_state"),
        "independently_observed": True,
        "listeners": [str(item) for item in value.get("listeners", [])],
        "runtime_sha256": runtime,
        "runtime_size_bytes": None,
        "healthz_ok": health["ok"],
        "ready": health["ready"],
        "last_error": health["last_error"],
        "nrestarts": health["nrestarts"],
        "oom_panic_segfault": health["oom_panic_segfault"],
        "full_chain_status_called": False,
    }


def parse_markers(output: str) -> dict[str, Any]:
    parsed: dict[str, Any] = {"readbacks": {}, "runtime": {}, "health": {}, "stopped": None, "absence": None}
    for line in output.splitlines():
        if line.startswith("OASIS7_READBACK\t"):
            _, role, payload = line.split("\t", 2)
            try:
                parsed["readbacks"][role] = json.loads(payload)
            except json.JSONDecodeError:
                fail("remote adapter emitted malformed readback")
        elif line.startswith("OASIS7_RUNTIME\t"):
            _, role, digest = line.split("\t", 2)
            parsed["runtime"][role] = digest.strip()
        elif line.startswith("OASIS7_HEALTH\t"):
            _, role, status = line.split("\t", 2)
            try:
                health = json.loads(status)
            except json.JSONDecodeError:
                fail("remote adapter emitted malformed health evidence")
            if not isinstance(health, dict):
                fail("remote adapter emitted non-object health evidence")
            parsed["health"][role] = health
        elif line.startswith("OASIS7_STOPPED\t"):
            try:
                parsed["stopped"] = json.loads(line.split("\t", 1)[1])
            except json.JSONDecodeError:
                fail("remote adapter emitted malformed stopped readback")
        elif line.startswith("OASIS7_ABSENCE\t"):
            try:
                parsed["absence"] = json.loads(line.split("\t", 1)[1])
            except json.JSONDecodeError:
                fail("remote adapter emitted malformed reset receipt")
    return parsed


def control_script(transaction: dict[str, Any], phase: str, package: dict[str, Any], governed: dict[str, Path], role: str) -> tuple[str, list[tuple[Path, str]]]:
    if phase not in {"staggered-storage", "staggered-sequencer", "staggered-rollback"}:
        fail("control script requested for unsupported phase")
    node = transaction.get("nodes", {}).get(role)
    if not isinstance(node, dict):
        fail(f"transaction node binding is missing for {role}")
    service = str(node.get("service") or ("oasis7-triad-storage.service" if role == "storage-205" else "oasis7-triad-sequencer.service"))
    if service not in {"oasis7-triad-storage.service", "oasis7-triad-sequencer.service"}:
        fail("transaction service binding is not fixed")
    if isinstance(transaction.get("backup"), dict):
        require_remote_backup(transaction, role)
    entries: list[tuple[Path, str]] = []
    for helper in (
        "p2p-public-testnet-package-node-upgrade.sh",
        "p2p-verify-linux-package-bundle.py",
        "p2p-safe-extract-tar.py",
        "p2p-safe-validate-deb-tree.py",
        "p2p-rebuild-linux-bundle-checksums.py",
    ):
        entries.append((ROOT / "scripts" / helper, f"scripts/{helper}"))
    if phase != "staggered-rollback":
        entries.extend((source, f"governed/{key}") for key, source in governed.items())
        entries.extend(
            [
                (package["files"][PACKAGE_DEB_NAME], f"package/{PACKAGE_DEB_NAME}"),
                (package["files"][OPS_TOOLS_NAME], f"package/{OPS_TOOLS_NAME}"),
                (TRIAD_INVENTORY_PATH, "triad/public-testnet-validator-triad-inventory.v1.json"),
                (TRIAD_EVIDENCE_ROOT / PUBLIC_MANIFEST_NAME, f"triad/evidence/{PUBLIC_MANIFEST_NAME}"),
            ]
        )
        for source in sorted(TRIAD_EVIDENCE_ROOT.glob("public-testnet-governed-bootstrap*")):
            entries.append((source, f"triad/evidence/{source.name}"))
    network = transaction.get("network", {})
    network_id = str(network.get("network_id") or "")
    chain_id = str(network.get("chain_id") or network_id)
    health_url = "" if phase == "staggered-rollback" else health_url_for_role(transaction, role)
    reset = " ".join(shlex.quote(item) for item in RESET_SURFACES)
    phase_literal = shlex.quote(phase)
    role_literal = shlex.quote(ROLE_ALIASES[role])
    script = f'''#!/usr/bin/env bash
set -euo pipefail
asset_root="${{ASSET_ROOT:?asset root missing}}"
root={shlex.quote(STACK_ROOT)}
service={shlex.quote(service)}
role={role_literal}
phase={phase_literal}
verify_stopped() {{
  value=$("$root/current/bin/service-readback" --read-only --role "$role" --root "$root" --service "$service")
  printf 'OASIS7_STOPPED\\t%s\\n' "$value"
  python3 - "$value" <<'PY'
import json, sys
value = json.loads(sys.argv[1])
if value.get('active') is not False or value.get('running') is not False or value.get('service_state') != 'stopped' or value.get('independently_observed') is not True:
    raise SystemExit('service_state stop proof is not verified')
PY
}}
emit_final() {{
  value=$("$root/current/bin/service-readback" --read-only --role "$role" --root "$root" --service "$service")
  printf 'OASIS7_READBACK\\t%s\\t%s\\n' "$role" "$value"
  printf 'OASIS7_RUNTIME\\t%s\\t%s\\n' "$role" "$(sha256sum "$root/current/bin/oasis7_chain_runtime" | awk '{{print $1}}')"
  health=$(curl --fail --silent --show-error --max-time 5 {shlex.quote(health_url)})
  health=$(HEALTH_JSON="$health" python3 - <<'PY'
import json, os
value = json.loads(os.environ['HEALTH_JSON'])
if not isinstance(value, dict):
    raise SystemExit('health evidence is not an object')
required = ('ok', 'ready', 'last_error', 'nrestarts', 'oom_panic_segfault')
if any(key not in value for key in required):
    raise SystemExit('health evidence is incomplete')
if not isinstance(value['ok'], bool) or not isinstance(value['ready'], bool):
    raise SystemExit('health boolean evidence is malformed')
if value['last_error'] is not None and not isinstance(value['last_error'], str):
    raise SystemExit('health last_error evidence is malformed')
if isinstance(value['nrestarts'], bool) or not isinstance(value['nrestarts'], int):
    raise SystemExit('health restart evidence is malformed')
if not isinstance(value['oom_panic_segfault'], bool):
    raise SystemExit('health crash evidence is malformed')
if value['ok'] is not True or value['ready'] is not True or value['last_error'] is not None:
    raise SystemExit('health readiness gate failed')
if value['nrestarts'] != 0 or value['oom_panic_segfault'] is not False:
    raise SystemExit('health restart/crash gate failed')
print(json.dumps(value, ensure_ascii=True, sort_keys=True, separators=(',', ':')))
PY
  )
  printf 'OASIS7_HEALTH\\t%s\\t%s\\n' "$role" "$health"
}}
systemctl stop "$service"
verify_stopped
if [[ "$phase" == "staggered-rollback" ]]; then
  for surface in {reset}; do
    path="$root/$surface"
    [[ ! -L "$path" ]] || exit 1
    rm -rf -- "$path"
    mkdir -p -- "$path"
  done
  [[ ! -L "$root/staged-world" ]] && rm -rf -- "$root/staged-world" || exit 1
  [[ ! -L "$root/staged-governed" ]] && rm -rf -- "$root/staged-governed" || exit 1
  printf 'OASIS7_ABSENCE\\t%s\\n' '{{"absent":true,"target_set_sha256":"{EXECUTOR.reset_surface_digest()}"}}'
  exit 0
fi
for surface in {reset}; do
  path="$root/$surface"
  [[ ! -L "$path" ]] || exit 1
  rm -rf -- "$path"
  mkdir -p -- "$path"
done
[[ ! -L "$root/staged-world" ]] && rm -rf -- "$root/staged-world" || exit 1
[[ ! -L "$root/staged-governed" ]] && rm -rf -- "$root/staged-governed" || exit 1
mkdir -p -- "$root/staged-world" "$root/staged-governed" "$root/config/doc/testing/evidence"
bash "$asset_root/scripts/p2p-public-testnet-package-node-upgrade.sh" \\
  --node-root "$root" --package-deb "$asset_root/package/{PACKAGE_DEB_NAME}" \\
  --ops-tools-tar "$asset_root/package/{OPS_TOOLS_NAME}" \\
  --package-version {shlex.quote(package["version"])} --commit {shlex.quote(package["commit"])} \\
  --run-id {shlex.quote(package["run_id"])} \\
  --artifact-ref {shlex.quote("testnet-package-linux-x64-" + package["version"] + "/" + PACKAGE_DEB_NAME + "!/opt/oasis7/bin/oasis7_chain_runtime")}
cp -- "$asset_root/governed/manifest" "$root/config/public-testnet-governed-bootstrap-manifest-2026-06-06.json"
cp -- "$asset_root/governed/genesis" "$root/config/public-testnet-governed-bootstrap-genesis-2026-06-06.json"
cp -- "$asset_root/governed/registry" "$root/config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json"
cp -- "$asset_root/governed/bootstrap" "$root/config/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt"
cp -- "$asset_root/triad/public-testnet-validator-triad-inventory.v1.json" "$root/config/public-testnet-validator-triad-inventory.v1.json"
for evidence in "$asset_root"/triad/evidence/public-testnet-governed-bootstrap* "$asset_root/triad/evidence/{PUBLIC_MANIFEST_NAME}"; do
  [[ -e "$evidence" ]] || continue
  cp -R -- "$evidence" "$root/config/doc/testing/evidence/"
done
cp -R -- "$asset_root/governed/world/." "$root/staged-world/"
"$root/current/bin/oasis7_world_repair_rebuild" \\
  --generated-world-dir "$root/staged-world" --output-world-dir "$root/data/execution-world" \\
  --world-id {shlex.quote(network_id)} --chain-id {shlex.quote(chain_id)} \\
  --resource-commit-height 0 --resource-commit-hash genesis
if [[ -d "$root/staged-world/generated-scenario-world" ]]; then
  cp -R -- "$root/staged-world/generated-scenario-world" "$root/data/execution-world/"
fi
if [[ -f "$root/staged-world/world-generation-provenance.json" ]]; then
  cp -- "$root/staged-world/world-generation-provenance.json" "$root/data/execution-world/world-generation-provenance.json"
fi
python3 - "$root/config/node.env" "$root/config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json" "$root/config/public-testnet-validator-triad-inventory.v1.json" <<'PY'
import hashlib, json, os, sys
from pathlib import Path
env_path, registry_path, inventory_path = map(Path, sys.argv[1:])
lines = env_path.read_text(encoding='utf-8').splitlines()
values = {{line.split('=', 1)[0]: line.split('=', 1)[1] for line in lines if '=' in line and not line.startswith('#')}}
role = {json.dumps(ROLE_ALIASES[role])}
expected_id = 'triad-testnet-' + role
if values.get('NODE_ID') != expected_id or values.get('NODE_ROLE') != role:
    raise SystemExit('node.env role identity mismatch')
registry = json.loads(registry_path.read_text(encoding='utf-8'))
signers = ','.join(f"{{item['node_id']}}:{{item['finality_signer_public_key']}}" for item in registry['validators'])
registry_sha = hashlib.sha256(registry_path.read_bytes()).hexdigest()
inventory_sha = hashlib.sha256(inventory_path.read_bytes()).hexdigest()
kept = []
skip = {{'NODE_VALIDATOR_SIGNERS_CSV', 'GENESIS_VALIDATOR_REGISTRY_SHA256', 'GENESIS_VALIDATOR_REGISTRY_SEMANTIC_SHA256', 'DEPLOYMENT_INVENTORY_PATH', 'DEPLOYMENT_INVENTORY_SHA256', 'P2P_NODE_ROLE'}}
for line in lines:
    if '=' in line and line.split('=', 1)[0] in skip:
        continue
    kept.append(line)
kept += [f'NODE_VALIDATOR_SIGNERS_CSV={{signers}}', f'GENESIS_VALIDATOR_REGISTRY_SHA256={{registry_sha}}', 'GENESIS_VALIDATOR_REGISTRY_SEMANTIC_SHA256=' + {shlex.quote(TRIAD_GENERATED_REGISTRY_SEMANTIC_SHA256)!r}, 'P2P_NODE_ROLE=' + ('full_storage' if role == 'storage' else 'sequencer'), 'DEPLOYMENT_INVENTORY_PATH=config/public-testnet-validator-triad-inventory.v1.json', f'DEPLOYMENT_INVENTORY_SHA256={{inventory_sha}}']
tmp = env_path.with_name('.node.env.adapter.tmp')
tmp.write_text('\\n'.join(kept) + '\\n', encoding='utf-8')
os.chmod(tmp, env_path.stat().st_mode & 0o777)
os.replace(tmp, env_path)
PY
public_manifest="$root/config/doc/testing/evidence/{PUBLIC_MANIFEST_NAME}"
[[ -f "$public_manifest" && ! -L "$public_manifest" ]] || exit 1
"$root/current/bin/oasis7_governance_registry_import" --world-dir "$root/data/execution-world" --public-manifest "$public_manifest"
systemctl daemon-reload
systemctl start "$service"
emit_final
printf 'OASIS7_ABSENCE\\t%s\\n' '{{"absent":true,"target_set_sha256":"{EXECUTOR.reset_surface_digest()}"}}'
'''
    return script, entries


def base_receipt(transaction: dict[str, Any], phase: str) -> dict[str, Any]:
    binding = transaction["adapter_binding"]
    evidence = binding.get("evidence_bindings")
    if not isinstance(evidence, dict):
        fail("adapter evidence binding is missing")
    return {
        "schema_version": "oasis7.validator_pair_rebuild_host_receipt.v2",
        "phase": phase,
        "execution_mode": "triad_staggered",
        "transaction_id": transaction["transaction_id"],
        "plan_digest": binding["plan_digest"],
        "evidence_bindings": evidence,
        "repository_executable": binding["repository_executable"],
        "captured_at": dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z"),
        "mutation_order": list(MUTATION_ORDER),
        "startup_order": list(transaction.get("startup_order", MUTATION_ORDER)),
        "observer_mutation": False,
        "max_simultaneously_stopped_validators": 1,
        "identity_receipts": evidence.get("identity_receipts"),
        "sequencer_rebuild_proof": evidence.get("sequencer_rebuild_proof"),
        "package": EXECUTOR._expected_adapter_package_binding(transaction),
    }


def require_remote_backup(transaction: dict[str, Any], role: str) -> None:
    """Require an executor-produced receipt bound to the remote mutation host.

    The adapter cannot safely infer a remote backup from local filesystem
    paths.  Until the executor records this explicit binding, the governed
    route remains capability-blocked before any archive or control script is
    created.
    """
    backup = transaction.get("backup")
    entry = backup.get(role) if isinstance(backup, dict) else None
    if not isinstance(entry, dict) or entry.get("remote_target") is not True:
        fail(f"capability_blocked: remote backup evidence is required before {role} mutation")
    if entry.get("schema_version") != "oasis7.validator_pair_rebuild_remote_backup_receipt.v1":
        fail(f"capability_blocked: remote backup receipt schema is unsupported for {role}")
    transaction_id = transaction.get("transaction_id")
    if (
        not isinstance(transaction_id, str)
        or not transaction_id.strip()
        or not SAFE_VERSION.fullmatch(transaction_id)
        or ".." in transaction_id
    ):
        fail("capability_blocked: remote backup transaction binding is missing or unsafe")
    if entry.get("transaction_id") != transaction_id:
        fail(f"capability_blocked: remote backup transaction binding is stale for {role}")
    if entry.get("role") != role:
        fail(f"capability_blocked: remote backup role binding is mismatched for {role}")
    if entry.get("credential_transport") != "fd-only-v1":
        fail(f"capability_blocked: remote backup credential transport is not FD-only for {role}")
    expected = EXECUTOR.HUMAN_DIRECT_SSH_CANONICAL[role]
    if entry.get("remote_host") != expected["host"] or entry.get("remote_root") != STACK_ROOT:
        fail(f"capability_blocked: remote backup host/root binding is not fixed for {role}")
    expected_manifest = f"{STACK_ROOT}/backups/{transaction_id}/manifest.json"
    if entry.get("manifest") != expected_manifest:
        fail(f"capability_blocked: remote backup manifest path is not transaction-bound for {role}")
    manifest_sha256 = entry.get("manifest_sha256")
    if not isinstance(manifest_sha256, str) or not HEX64.fullmatch(manifest_sha256):
        fail(f"capability_blocked: remote backup manifest digest is missing for {role}")
    reset_surface_manifest_sha256 = entry.get("reset_surface_manifest_sha256")
    if not isinstance(reset_surface_manifest_sha256, str) or not HEX64.fullmatch(reset_surface_manifest_sha256):
        fail(f"capability_blocked: remote reset-surface manifest digest is missing for {role}")
    if entry.get("reset_surfaces") != list(RESET_SURFACES):
        fail(f"capability_blocked: remote reset-surface binding is not canonical for {role}")
    non_seed = entry.get("backup_non_seed")
    if (
        not isinstance(non_seed, dict)
        or non_seed.get("forensic_only") is not True
        or non_seed.get("seed_eligible") is not False
        or non_seed.get("restore_deleted_chain_state") is not False
    ):
        fail(f"capability_blocked: remote backup is not forensic-only non-seed evidence for {role}")
    capacity = entry.get("capacity")
    if (
        not isinstance(capacity, dict)
        or capacity.get("verified") is not True
        or capacity.get("same_filesystem") is not True
    ):
        fail(f"capability_blocked: remote backup capacity evidence is missing for {role}")
    numeric_fields = ("available_bytes", "free_bytes", "required_bytes", "free_inodes", "required_inodes")
    if any(
        isinstance(capacity.get(field), bool)
        or not isinstance(capacity.get(field), int)
        or capacity[field] < 0
        for field in numeric_fields
    ):
        fail(f"capability_blocked: remote backup capacity evidence is malformed for {role}")
    if (
        capacity["available_bytes"] < capacity["required_bytes"]
        or capacity["free_bytes"] < capacity["required_bytes"]
        or capacity["free_inodes"] < capacity["required_inodes"]
    ):
        fail(f"capability_blocked: remote backup capacity is insufficient for {role}")
    if entry.get("reset_surface_manifest_sha256") != manifest_sha256:
        fail(f"capability_blocked: remote reset-surface manifest is not bound for {role}")
    expected_backup_root = f"{STACK_ROOT}/backups/{transaction_id}"
    if entry.get("backup_root") != expected_backup_root:
        fail(f"capability_blocked: remote backup root is not transaction-bound for {role}")
    planned = transaction.get("capacity", {}).get(role) if isinstance(transaction.get("capacity"), dict) else None
    if isinstance(planned, dict):
        if (
            capacity.get("required_bytes") != planned.get("required_bytes")
            or capacity.get("required_inodes") != planned.get("required_inodes")
            or planned.get("required_inodes", 0) < MIN_REMOTE_BACKUP_INODES
        ):
            fail(f"capability_blocked: remote backup capacity threshold is not code-bound for {role}")


def remote_backup_thresholds(transaction: dict[str, Any], role: str) -> tuple[int, int]:
    capacity = transaction.get("capacity")
    planned = capacity.get(role) if isinstance(capacity, dict) else None
    if not isinstance(planned, dict):
        fail(f"capability_blocked: code-owned remote backup capacity threshold is missing for {role}")
    required_bytes = planned.get("required_bytes")
    required_inodes = planned.get("required_inodes")
    if (
        isinstance(required_bytes, bool)
        or not isinstance(required_bytes, int)
        or required_bytes <= 0
        or isinstance(required_inodes, bool)
        or not isinstance(required_inodes, int)
        or required_inodes < MIN_REMOTE_BACKUP_INODES
    ):
        fail(f"capability_blocked: code-owned remote backup capacity threshold is malformed for {role}")
    return required_bytes, required_inodes


def backup_phase_role(phase: str) -> str:
    if phase == "staggered-storage-backup":
        return "storage-205"
    if phase == "staggered-sequencer-backup":
        return "sequencer-204"
    fail(f"unsupported remote backup phase: {phase}")


def run_phase(transaction: dict[str, Any], phase: str) -> dict[str, Any]:
    inventory, known_hosts, package, governed = validate_transaction(transaction, phase)
    fds = credential_fds()
    transport = FixedSSH(inventory, known_hosts, fds)
    plan_nodes = transaction.get("nodes")
    if not isinstance(plan_nodes, dict) or any(role not in plan_nodes for role in MUTATION_ORDER):
        fail("transaction node bindings are missing")
    if phase == "staggered-preflight":
        nodes = {
            role: observe_node(
                transport,
                inventory,
                role,
                str(plan_nodes[role]["root"]),
                running=True,
                health_url=health_url_for_role(transaction, role),
            )
            for role in MUTATION_ORDER
        }
        for role, node in nodes.items():
            if node["runtime_sha256"] != transaction["package"].get("runtime_sha256"):
                fail(f"preflight runtime identity mismatch for {role}")
        receipt = base_receipt(transaction, phase)
        receipt.update({"staggered_phase": "preflight", "live_baseline": True, "nodes": nodes})
        for node in nodes.values():
            node["preflight_observer_mutation"] = False
        return receipt
    if phase in BACKUP_PHASES:
        target = backup_phase_role(phase)
        peer = "sequencer-204" if target == "storage-205" else "storage-205"
        plan_nodes = transaction.get("nodes")
        if not isinstance(plan_nodes, dict) or any(role not in plan_nodes for role in MUTATION_ORDER):
            fail("transaction node bindings are missing")
        existing = transaction.get("backup")
        if isinstance(existing, dict) and target in existing:
            fail(f"remote backup is already bound for {target}")
        peer_before = observe_node(
            transport,
            inventory,
            peer,
            str(plan_nodes[peer]["root"]),
            running=True,
            health_url=health_url_for_role(transaction, peer),
        )
        target_before = observe_node(
            transport,
            inventory,
            target,
            str(plan_nodes[target]["root"]),
            running=True,
            health_url=health_url_for_role(transaction, target),
        )
        required_bytes, required_inodes = remote_backup_thresholds(transaction, target)
        entry = transport.remote_forensic_backup(
            target,
            str(transaction["transaction_id"]),
            required_bytes,
            required_inodes,
        )
        bound = dict(transaction)
        bound["backup"] = {target: entry}
        require_remote_backup(bound, target)
        target_receipt = dict(target_before)
        target_receipt.update(entry)
        target_receipt["backup_verified"] = True
        receipt = base_receipt(transaction, phase)
        receipt.update(
            {
                "staggered_phase": "remote_backup",
                "backup_role": target,
                "live_peer_role": peer,
                "backup_before_stop": True,
                "target_stopped_before_reset": False,
                "reset_started_after_target_stop": False,
                "backup_receipt": entry,
                "nodes": {target: target_receipt, peer: peer_before},
            }
        )
        return receipt
    if phase == "staggered-rollback":
        failed = transaction.get("staggered_failed_role") or transaction.get("staggered_active_role")
        completed = transaction.get("staggered_completed_roles")
        if failed not in MUTATION_ORDER and isinstance(completed, list) and completed:
            failed = completed[-1]
        if failed not in MUTATION_ORDER:
            fail("rollback target role is not transaction-bound")
        require_remote_backup(transaction, failed)
        peer = "sequencer-204" if failed == "storage-205" else "storage-205"
        peer_before = observe_node(
            transport,
            inventory,
            peer,
            str(plan_nodes[peer]["root"]),
            running=True,
            health_url=health_url_for_role(transaction, peer),
        )
        script, entries = control_script(transaction, phase, package, governed, failed)
        markers = parse_markers(transport.archive(failed, script, entries))
        stopped = markers.get("stopped")
        if not isinstance(stopped, dict) or stopped.get("active") is not False or stopped.get("running") is not False or stopped.get("service_state") != "stopped":
            fail("rollback target was not left stopped")
        peer_after = observe_node(
            transport,
            inventory,
            peer,
            str(plan_nodes[peer]["root"]),
            running=True,
            health_url=health_url_for_role(transaction, peer),
        )
        target = {
            "role": failed, "root": str(plan_nodes[failed]["root"]), "active": False,
            "running": False, "service_state": "stopped", "independently_observed": True,
            "listeners": [], "runtime_sha256": transaction["package"].get("runtime_sha256"),
            "healthz_ok": False, "nrestarts": 0, "oom_panic_segfault": False,
            "full_chain_status_called": False,
        }
        receipt = base_receipt(transaction, phase)
        receipt.update({"staggered_phase": "target_only_cleanup", "failed_role": failed, "target_only_cleanup": True, "restore_deleted_chain_state": False, "nodes": {failed: target, peer: peer_after}})
        return receipt
    target = "storage-205" if phase == "staggered-storage" else "sequencer-204"
    peer = "sequencer-204" if target == "storage-205" else "storage-205"
    require_remote_backup(transaction, target)
    peer_before = observe_node(
        transport,
        inventory,
        peer,
        str(plan_nodes[peer]["root"]),
        running=True,
        health_url=health_url_for_role(transaction, peer),
    )
    script, entries = control_script(transaction, phase, package, governed, target)
    markers = parse_markers(transport.archive(target, script, entries))
    stopped = markers.get("stopped")
    if not isinstance(stopped, dict) or stopped.get("active") is not False or stopped.get("running") is not False or stopped.get("service_state") != "stopped":
        fail("target stop-before-reset proof is missing")
    if not isinstance(markers.get("absence"), dict) or markers["absence"].get("absent") is not True:
        fail("target reset absence proof is missing")
    alias = ROLE_ALIASES[target]
    payload = markers.get("readbacks", {}).get(alias)
    runtime = markers.get("runtime", {}).get(alias)
    health = markers.get("health", {}).get(alias)
    if not isinstance(payload, dict) or not isinstance(runtime, str) or not isinstance(health, dict):
        fail("target post-start readback is incomplete")
    if (
        not isinstance(health.get("ok"), bool)
        or not isinstance(health.get("ready"), bool)
        or (health.get("last_error") is not None and not isinstance(health.get("last_error"), str))
        or isinstance(health.get("nrestarts"), bool)
        or not isinstance(health.get("nrestarts"), int)
        or not isinstance(health.get("oom_panic_segfault"), bool)
    ):
        fail("target health evidence is malformed")
    if health.get("ok") is not True or health.get("ready") is not True or health.get("last_error") is not None:
        fail("target health readiness gate failed")
    if health.get("nrestarts") != 0 or health.get("oom_panic_segfault") is not False:
        fail("target health restart/crash gate failed")
    if payload.get("active") is not True or payload.get("running") is not True or payload.get("service_state") != "running":
        fail("target readiness readback failed")
    listeners = {str(item) for item in payload.get("listeners", [])}
    if not EXECUTOR.EXPECTED_LISTENERS[target].issubset(listeners):
        fail(f"target listener readback is incomplete for {target}")
    target_node = {
        "role": target, "root": str(plan_nodes[target]["root"]), "active": True,
        "running": True, "service_state": "running", "independently_observed": True,
        "listeners": sorted(listeners), "runtime_sha256": runtime, "healthz_ok": health["ok"],
        "ready": health["ready"], "last_error": health["last_error"],
        "nrestarts": health["nrestarts"], "oom_panic_segfault": health["oom_panic_segfault"],
        "full_chain_status_called": False,
        "post_delete_absence": {"absent": True, "target_set": list(RESET_SURFACES), "target_set_sha256": EXECUTOR.reset_surface_digest()},
    }
    if runtime != transaction["package"].get("runtime_sha256"):
        fail(f"target runtime identity mismatch for {target}")
    peer_after = observe_node(
        transport,
        inventory,
        peer,
        str(plan_nodes[peer]["root"]),
        running=True,
        health_url=health_url_for_role(transaction, peer),
    )
    if peer_before["running"] is not True or peer_after["running"] is not True:
        fail("live peer preservation readback failed")
    receipt = base_receipt(transaction, phase)
    receipt.update({
        "staggered_phase": "member_cutover", "target_role": target, "live_peer_role": peer,
        "live_peer_readback": True, "rebuilt_member_readiness": True,
        "target_stopped_before_reset": {"active": False, "running": False, "service_state": "stopped", "independently_observed": True},
        "reset_started_after_target_stop": True, "nodes": {target: target_node, peer: peer_after},
    })
    return receipt


def main() -> int:
    parser = argparse.ArgumentParser(allow_abbrev=False)
    parser.add_argument("--phase", choices=sorted(PHASES), required=True)
    parser.add_argument("--transaction", required=True)
    args = parser.parse_args()
    transaction = load_json(Path(args.transaction).expanduser(), "transaction")
    receipt = run_phase(transaction, args.phase)
    print(json.dumps(receipt, ensure_ascii=True, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
