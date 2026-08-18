#!/usr/bin/env python3
"""Run and verify a clean-root signed checkpoint closure probe.

This deliberately owns the receipt boundary: rollout callers never pass an
operator-authored receipt.  The runtime writes its receipt only after it has
installed a full network-fetched closure in this probe's fresh replication
root; this helper validates that receipt and records immutable package facts.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import secrets
import shutil
import signal
import socket
import subprocess
import sys
import tarfile
import tempfile
import time
import urllib.request
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = "oasis7.checkpoint_closure_verification_receipt.v1"
RESULT_SCHEMA = "oasis7.observer_checkpoint_closure_probe.v1"


def die(message: str) -> None:
    raise SystemExit(f"error: checkpoint closure probe: {message}")


def allocate_loopback_tcp_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.bind(("127.0.0.1", 0))
        return int(listener.getsockname()[1])


def allocate_udp_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as listener:
        listener.bind(("0.0.0.0", 0))
        return int(listener.getsockname()[1])


def clean_room_network_overrides(
    status_port: int, gossip_port: int
) -> dict[str, str]:
    return {
        "STATUS_BIND": f"127.0.0.1:{status_port}",
        "NODE_GOSSIP_BIND": f"0.0.0.0:{gossip_port}",
        "REPLICATION_NETWORK_LISTEN_ADDRS_CSV": "/ip4/127.0.0.1/tcp/0",
        "TRAFFIC_MONITOR_ENABLE": "0",
    }


def runtime_log_excerpt(app_root: Path) -> str:
    runtime_log = app_root / "logs/chain-runtime.log"
    if not runtime_log.is_file():
        return ""
    output = runtime_log.read_text(encoding="utf-8", errors="replace")
    if len(output) > 8192:
        output = (
            output[:4096]
            + "\n... runtime log truncated ...\n"
            + output[-4096:]
        )
    return output.strip()


def probe_status_excerpt(status_port: int) -> str:
    try:
        with urllib.request.urlopen(
            f"http://127.0.0.1:{status_port}/v1/chain/status", timeout=3
        ) as response:
            status = json.load(response)
    except Exception as error:  # diagnostic only; the receipt remains authoritative
        return f"unavailable:{error}"
    replication = status.get("replication", {})
    consensus = status.get("consensus", {})
    return json.dumps(
        {
            "running": status.get("running"),
            "last_error": status.get("last_error"),
            "readiness": status.get("readiness"),
            "sync": status.get("sync"),
            "checkpoint": status.get("chain_proof", {}).get(
                "latest_execution_checkpoint"
            ),
            "connected_peers": replication.get("connected_peers"),
            "peer_healths": replication.get("peer_healths"),
            "recent_errors": replication.get("recent_errors"),
            "network_head": consensus.get("network_head"),
        },
        sort_keys=True,
        separators=(",", ":"),
    )


def bind_probe_runtime_metadata(
    app_root: Path, runtime: Path, buildinfo: dict[str, str]
) -> None:
    runtime_sha256 = sha256(runtime)
    runtime_size = runtime.stat().st_size
    package_version = buildinfo["package_version"]
    commit = buildinfo["commit"]
    run_id = buildinfo["run_id"]
    updated_by = (
        "p2p-observer-checkpoint-closure-probe "
        f"{package_version} (run {run_id}, commit {commit})"
    )
    artifact_ref = (
        f"testnet-package-linux-x64-{package_version}/"
        "oasis7-linux-x64.deb!/opt/oasis7/bin/oasis7_chain_runtime"
    )
    bundle_paths = sorted(
        (app_root / "config").rglob(
            "public-testnet-governed-bootstrap-bundle-2026-06-06.json"
        )
    )
    if not bundle_paths:
        die("governed probe config has no public-testnet bootstrap bundle")
    for bundle_path in bundle_paths:
        data = json.loads(bundle_path.read_text(encoding="utf-8"))
        runtime_build = data.setdefault("runtime_build", {})
        runtime_build.update(
            {
                "git_commit": commit,
                "kind": "file",
                "path": str(runtime),
                "resolved_path": str(runtime.resolve()),
                "ref": artifact_ref,
                "sha256": runtime_sha256,
                "size_bytes": runtime_size,
                "package_version": package_version,
                "run_id": run_id,
                "updated_by": updated_by,
            }
        )
        data["git_commit"] = commit
        data["updated_by"] = updated_by
        bundle_path.write_text(
            json.dumps(data, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for block in iter(lambda: f.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def canonical_json(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode()


def validate_runtime_receipt(receipt: Any, nonce: str) -> dict[str, Any]:
    if not isinstance(receipt, dict) or receipt.get("schema_version") != SCHEMA:
        die("runtime receipt has invalid schema_version")
    if receipt.get("probe_nonce") != nonce:
        die("runtime receipt nonce mismatch")
    if not isinstance(receipt.get("world_id"), str) or not receipt["world_id"]:
        die("runtime receipt missing world_id")
    if not isinstance(receipt.get("height"), int) or receipt["height"] <= 0:
        die("runtime receipt has invalid height")
    hashes = ("execution_block_hash", "execution_state_root", "manifest_hash")
    if any(not isinstance(receipt.get(k), str) or not receipt[k] for k in hashes):
        die("runtime receipt missing checkpoint bindings")
    objects = receipt.get("objects")
    observations = receipt.get("fetch_observations")
    if not isinstance(objects, list) or not objects or not isinstance(observations, list):
        die("runtime receipt missing closure objects or fetch observations")
    if len(objects) != len(observations):
        die("runtime receipt object/observation count mismatch")
    for index, (obj, observed) in enumerate(zip(objects, observations)):
        if not isinstance(obj, dict) or not isinstance(observed, dict):
            die(f"runtime receipt entry {index} is not an object")
        expected_hash = obj.get("expected_content_hash")
        expected_size = obj.get("expected_size_bytes")
        if (not isinstance(expected_hash, str) or not expected_hash or
                not isinstance(expected_size, int) or expected_size < 0):
            die(f"runtime receipt entry {index} has invalid expected binding")
        if (obj.get("observed_content_hash") != expected_hash or
                obj.get("observed_size_bytes") != expected_size or
                observed.get("source") != "network_fetch" or
                observed.get("content_hash") != expected_hash or
                observed.get("observed_content_hash") != expected_hash or
                observed.get("observed_size_bytes") != expected_size or
                observed.get("response_found") is not True or
                observed.get("signed_request") is not True):
            die(f"runtime receipt entry {index} fails hash/size/network/signed binding")
        candidates = observed.get("connected_candidate_ids")
        if not isinstance(candidates, list) or not candidates or not all(isinstance(x, str) and x for x in candidates):
            die(f"runtime receipt entry {index} has no connected candidates")
    return receipt


def read_env(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line or line.lstrip().startswith("#") or "=" not in line:
            continue
        k, v = line.split("=", 1)
        result[k.strip()] = v.strip().strip('"')
    return result


def write_env(path: Path, values: dict[str, str]) -> None:
    path.write_text("".join(f"{k}={v}\n" for k, v in sorted(values.items())), encoding="utf-8")


def normalize_clean_room_environment(
    governed_env: dict[str, str], source_root: Path, app_root: Path
) -> dict[str, str]:
    """Rebind governed config references and mutable state to the fresh root.

    ``node.env`` is sourced by the node-start shell script, but the probe never
    sources it: its values must therefore be expanded deliberately before that
    script sees them.  Only the copied ``config/`` subtree remains governed
    startup truth; every state root is newly allocated under ``app_root``.
    """
    result = dict(governed_env)
    source_config = (source_root / "config").resolve()
    for key in (
        "CONFIG_PATH",
        "NETWORK_TIER_MANIFEST_PATH",
        "GENESIS_VALIDATOR_REGISTRY_PATH",
    ):
        value = result.get(key)
        if not value:
            continue
        expanded = value.replace("${STACK_ROOT}", str(source_root)).replace(
            "$STACK_ROOT", str(source_root)
        )
        candidate = Path(expanded)
        try:
            relative = candidate.resolve().relative_to(source_config)
        except (OSError, ValueError):
            die(f"governed {key} must resolve under source config")
        result[key] = str(app_root / "config" / relative)
    result.update(
        {
            "STACK_ROOT": str(app_root),
            "APP_ROOT": str(app_root),
            "RUNTIME_ROOT": str(app_root / "runtime-root"),
            "EXECUTION_WORLD_DIR": str(app_root / "world"),
            "EXECUTION_RECORDS_DIR": str(app_root / "execution-records"),
            "STORAGE_ROOT": str(app_root / "storage"),
            "REPLICATION_ROOT": str(app_root / "replication-root"),
        }
    )
    return result


def clean_room_launch_environment(env: dict[str, str], nonce: str) -> dict[str, str]:
    """Return the minimal process environment; do not inherit host state."""
    return {
        "PATH": os.defpath,
        **env,
        "OASIS7_CHECKPOINT_PROBE_NONCE": nonce,
    }


def package_file(package_dir: Path, name: str, label: str) -> Path:
    matches = sorted(package_dir.rglob(name))
    if len(matches) != 1 or matches[0].is_symlink() or not matches[0].is_file():
        die(f"package must contain exactly one {label}: {name}")
    return matches[0]


def validate_ops_tools_archive(archive_path: Path, destination: Path) -> None:
    with tarfile.open(archive_path, "r:gz") as archive:
        members = archive.getmembers()
        for member in members:
            target = (destination / member.name).resolve()
            try:
                target.relative_to(destination.resolve())
            except ValueError:
                die(f"ops-tools archive member escapes extraction root: {member.name}")
            if not (member.isdir() or member.isreg()):
                die(f"ops-tools archive contains non-regular member: {member.name}")
        for member in members:
            target = destination / member.name
            if member.isdir():
                target.mkdir(parents=True, exist_ok=True)
                continue
            target.parent.mkdir(parents=True, exist_ok=True)
            source = archive.extractfile(member)
            if source is None:
                die(f"ops-tools archive member cannot be read: {member.name}")
            with source, target.open("xb") as output:
                shutil.copyfileobj(source, output)
    manifest = destination / "oasis7-linux-x64-ops-tools/.oasis7-ops-tools-manifest.json"
    sums = destination / "oasis7-linux-x64-ops-tools/SHA256SUMS"
    if not manifest.is_file() or not sums.is_file():
        die("ops-tools archive missing manifest or SHA256SUMS")
    for raw in sums.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line:
            continue
        expected, separator, relative = line.partition("  ")
        if not separator:
            parts = line.split(maxsplit=1)
            if len(parts) != 2:
                die("ops-tools SHA256SUMS contains malformed entry")
            expected, relative = parts
        target = destination / "oasis7-linux-x64-ops-tools" / relative.lstrip("*")
        if target.is_symlink() or not target.is_file() or sha256(target) != expected.lower():
            die(f"ops-tools checksum verification failed: {relative}")
    for name in (
        "oasis7_world_repair_rebuild",
        "oasis7_governance_registry_import",
        "oasis7_governance_registry_audit",
    ):
        if not (destination / "oasis7-linux-x64-ops-tools/bin" / name).is_file():
            die(f"ops-tools archive missing required executable: {name}")


def package_runtime(package_dir: Path, app_root: Path) -> Path:
    if package_dir.is_symlink() or not package_dir.is_dir():
        die("package directory must be a non-symlink directory")
    deb = package_file(package_dir, "oasis7-linux-x64.deb", "linux Debian package")
    ops_tools = package_file(package_dir, "oasis7-linux-x64-ops-tools.tar.gz", "linux ops-tools archive")
    app_root.mkdir(parents=True, exist_ok=True)
    deb_stage = app_root / "_deb-extract"
    completed = subprocess.run(
        ["dpkg-deb", "--extract", str(deb), str(deb_stage)],
        text=True,
        capture_output=True,
    )
    if completed.returncode:
        die(f"cannot extract linux Debian package: {completed.stderr.strip()}")
    source = deb_stage / "opt/oasis7"
    if not source.is_dir() or source.is_symlink():
        die("linux Debian package is missing a regular /opt/oasis7 player bundle")
    for path in source.rglob("*"):
        if path.is_symlink() or not (path.is_file() or path.is_dir()):
            die(f"linux Debian package contains unsupported player entry: {path}")
    ops_destination = app_root / "_ops-extract"
    validate_ops_tools_archive(ops_tools, ops_destination)
    release = app_root / "release/oasis7-linux-x64"
    shutil.copytree(source, release)
    shutil.rmtree(deb_stage)
    shutil.rmtree(ops_destination)
    candidates = list(release.rglob("oasis7_chain_runtime"))
    if len(candidates) != 1 or not candidates[0].is_file():
        die("linux Debian package has no unique runtime executable")
    candidates[0].chmod(candidates[0].stat().st_mode | 0o111)
    current = app_root / "current"
    current.symlink_to(release)
    return candidates[0]


def run_probe(args: argparse.Namespace) -> dict[str, Any]:
    if args.timeout_secs <= 0:
        die("timeout must be greater than zero")
    if args.manifest.is_symlink() or not args.manifest.is_file():
        die("manifest must be a regular non-symlink file")
    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    nodes = manifest.get("nodes", []) if isinstance(manifest, dict) else []
    observer = next((n for n in nodes if isinstance(n, dict) and n.get("name") not in {"sequencer", "storage"}), None)
    if observer is None:
        die("manifest has no observer")
    observer_name = observer.get("name")
    if not isinstance(observer_name, str) or not observer_name:
        die("observer must have a non-empty name")
    source_root = Path(str(observer.get("node_root") or ""))
    source_env = source_root / "config/node.env"
    if source_env.is_symlink() or not source_env.is_file():
        die("observer node_root must provide config/node.env for governed probe")
    buildinfos = list(args.package_dir.rglob("linux-x64-BUILDINFO"))
    if len(buildinfos) != 1 or buildinfos[0].is_symlink() or not buildinfos[0].is_file():
        die("package must contain exactly one regular linux BUILDINFO")
    buildinfo = {
        key: value
        for line in buildinfos[0].read_text(encoding="utf-8").splitlines()
        if "=" in line
        for key, value in [line.split("=", 1)]
    }
    for key in ("commit", "package_version", "run_id"):
        if not buildinfo.get(key):
            die(f"BUILDINFO missing {key}")
    with tempfile.TemporaryDirectory(prefix="oasis7-checkpoint-probe-") as temp:
        app_root = Path(temp) / "app"
        shutil.copytree(source_root / "config", app_root / "config")
        runtime = package_runtime(args.package_dir, app_root)
        # Copy only governed startup truth; all mutable roots are fresh below.
        for name in ("node.env", "config.json", "bootstrap-peers.txt"):
            source = source_root / "config" / name
            if source.is_file():
                shutil.copy2(source, app_root / "config" / name)
        bind_probe_runtime_metadata(app_root, runtime, buildinfo)
        env = read_env(source_env)
        world_id = env.get("WORLD_ID", "")
        if not world_id:
            die("governed observer env has no WORLD_ID")
        env = normalize_clean_room_environment(env, source_root, app_root)
        network_manifest = env.get("NETWORK_TIER_MANIFEST_PATH", "")
        network_manifest_sha256 = None
        if network_manifest:
            network_path = Path(network_manifest)
            if network_path.is_symlink() or not network_path.is_file():
                die("network tier manifest must be a regular non-symlink file")
            network_manifest_sha256 = sha256(network_path)
        nonce = secrets.token_urlsafe(32)
        status_port = allocate_loopback_tcp_port()
        gossip_port = allocate_udp_port()
        env.update({
            "BIN": str(runtime), "NODE_ID": "checkpoint-closure-probe",
            "NODE_ROLE": "observer",
        })
        env.update(clean_room_network_overrides(status_port, gossip_port))
        write_env(app_root / "config/node.env", env)
        launch_env = clean_room_launch_environment(env, nonce)
        proc = subprocess.Popen([str(ROOT / "scripts/p2p-triad-node-start.sh")], env=launch_env)
        try:
            deadline = time.monotonic() + args.timeout_secs
            receipt_path: Path | None = None
            while time.monotonic() < deadline:
                matches = list((app_root / "replication-root/checkpoint-verification").glob("*.json"))
                if matches:
                    receipt_path = max(matches, key=lambda p: p.stat().st_mtime)
                    break
                if proc.poll() is not None:
                    runtime_tail = runtime_log_excerpt(app_root)
                    detail = f"; runtime_log_tail={runtime_tail}" if runtime_tail else ""
                    die(
                        f"probe runtime exited before receipt (status={proc.returncode})"
                        f"{detail}"
                    )
                time.sleep(1)
            if receipt_path is None:
                runtime_tail = runtime_log_excerpt(app_root)
                status_detail = probe_status_excerpt(status_port)
                detail = (
                    f"; probe_status={status_detail}; runtime_log_tail={runtime_tail}"
                )
                die(f"timed out waiting for fresh runtime receipt{detail}")
            receipt = validate_runtime_receipt(json.loads(receipt_path.read_text(encoding="utf-8")), nonce)
        finally:
            proc.send_signal(signal.SIGTERM)
            try: proc.wait(timeout=15)
            except subprocess.TimeoutExpired: proc.kill(); proc.wait()
        result = {"schema_version": RESULT_SCHEMA, "runtime_receipt": receipt,
                  "input_bindings": {"rollout_manifest_sha256": sha256(args.manifest),
                    "observer_name": observer_name, "world_id": world_id,
                    "network_tier_manifest_sha256": network_manifest_sha256,
                    "buildinfo": {key: buildinfo[key] for key in ("commit", "package_version", "run_id")}},
                  "package_runtime_sha256": sha256(runtime), "package_runtime_path": runtime.name,
                  "generated_at_unix_ms": int(time.time() * 1000)}
        result["canonical_digest"] = hashlib.sha256(canonical_json(result)).hexdigest()
        return result


def main() -> int:
    p = argparse.ArgumentParser(description="Execute a clean-root signed checkpoint closure probe")
    p.add_argument("--manifest", type=Path, required=True)
    p.add_argument("--package-dir", type=Path, required=True)
    p.add_argument("--out", type=Path)
    p.add_argument("--timeout-secs", type=int, default=180)
    p.add_argument("--validate-receipt", type=Path, help="test-only validator for a runtime-produced receipt")
    p.add_argument("--nonce", help="required with --validate-receipt")
    args = p.parse_args()
    if args.validate_receipt:
        if not args.nonce: die("--validate-receipt requires --nonce")
        receipt = validate_runtime_receipt(json.loads(args.validate_receipt.read_text()), args.nonce)
        print(json.dumps(receipt, sort_keys=True))
        return 0
    if not args.out: die("--out is required when executing a probe")
    result = run_probe(args)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_bytes(canonical_json(result) + b"\n")
    print(json.dumps(result, sort_keys=True))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
