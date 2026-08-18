#!/usr/bin/env python3
"""Create and validate immutable validator-pair package/world provenance.

The receipt intentionally contains hashes and public metadata only.  It never
reads or emits validator node-keypair contents.  Cryptographic signing is
performed by the external package-attestation workflow; this helper verifies
that the resulting detached receipt is present and marked verified.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, NoReturn


SCHEMA = "oasis7.validator_pair_rebuild_provenance.v1"
TRUST_ROOT_SCHEMA = "oasis7.validator_pair_provenance_trust_root.v1"
REQUIRED_GOVERNED_KEYS = {"manifest", "genesis", "registry", "bootstrap", "world"}
HEX64 = re.compile(r"^[0-9a-fA-F]{64}$")
HEX40 = re.compile(r"^[0-9a-fA-F]{40}$")


def die(message: str) -> NoReturn:
    raise SystemExit(f"error: validator-pair provenance: {message}")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def tree_digest(path: Path) -> tuple[str, int, int]:
    if not path.is_dir() or path.is_symlink():
        die(f"world reference must be a real directory: {path}")
    digest = hashlib.sha256()
    count = 0
    total = 0
    for child in sorted((item for item in path.rglob("*") if item.is_file()), key=lambda p: p.relative_to(path).as_posix()):
        if child.is_symlink():
            die(f"world reference contains symlink: {child}")
        relative = child.relative_to(path).as_posix()
        child_sha = sha256_file(child)
        size = child.stat().st_size
        digest.update(relative.encode("utf-8"))
        digest.update(b"\0")
        digest.update(child_sha.encode("ascii"))
        digest.update(b"\0")
        digest.update(str(size).encode("ascii"))
        digest.update(b"\n")
        count += 1
        total += size
    if count == 0:
        die(f"world reference is empty: {path}")
    return digest.hexdigest(), count, total


def metadata(path: Path, *, allow_directory: bool = False) -> dict[str, Any]:
    if path.is_symlink() or not path.exists():
        die(f"provenance reference missing or symlinked: {path}")
    stat = path.stat()
    if path.is_file():
        return {"path": str(path), "sha256": sha256_file(path), "size_bytes": stat.st_size, "kind": "file"}
    if allow_directory and path.is_dir():
        digest, count, total = tree_digest(path)
        return {"path": str(path), "sha256_tree": digest, "file_count": count, "total_bytes": total, "kind": "directory"}
    die(f"provenance reference is not a regular file: {path}")


def parse_buildinfo(path: Path) -> dict[str, str]:
    if not path.is_file():
        die(f"missing BUILDINFO: {path}")
    values: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        key, separator, value = line.partition("=")
        if separator:
            values[key.strip()] = value.strip()
    for key in ("run_id", "commit", "package_version"):
        if not values.get(key):
            die(f"BUILDINFO missing {key}")
    if not HEX40.fullmatch(values["commit"]):
        die("BUILDINFO commit must be a 40-hex commit")
    return values


def find_runtime(package_dir: Path) -> Path:
    candidates = [
        package_dir / "oasis7_chain_runtime",
        package_dir / "bin" / "oasis7_chain_runtime",
    ]
    candidates.extend(sorted(package_dir.rglob("oasis7_chain_runtime")))
    for candidate in candidates:
        if candidate.is_file() and not candidate.is_symlink():
            return candidate
    die(f"package runtime not found under {package_dir}")


def verify_checksums(package_dir: Path, runtime: Path) -> tuple[str, str]:
    sums_candidates = [package_dir / "SHA256SUMS"] + sorted(package_dir.glob("*-SHA256SUMS"))
    sums_path = next((candidate for candidate in sums_candidates if candidate.is_file()), None)
    if sums_path is None:
        die(f"package SHA256SUMS missing under {package_dir}")
    runtime_sha = sha256_file(runtime)
    covered = False
    for raw in sums_path.read_text(encoding="utf-8").splitlines():
        fields = raw.strip().split()
        if len(fields) < 2:
            continue
        expected, name = fields[0].lower(), fields[-1].lstrip("*")
        if not HEX64.fullmatch(expected):
            die("SHA256SUMS contains a malformed digest")
        listed = Path(name)
        if listed.is_absolute() or ".." in listed.parts:
            die("SHA256SUMS contains a path outside the package")
        candidate = package_dir / listed
        if not candidate.is_file() or candidate.is_symlink():
            die(f"SHA256SUMS references missing file: {name}")
        actual = sha256_file(candidate)
        if expected != actual:
            die(f"SHA256SUMS mismatch: {name}")
        if Path(name).name == runtime.name:
            covered = True
    if not covered:
        die("SHA256SUMS does not cover runtime")
    return sha256_file(sums_path), sums_path.name


def canonical_without_digest(payload: dict[str, Any]) -> bytes:
    body = {key: value for key, value in payload.items() if key != "binding_digest"}
    return json.dumps(body, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode("utf-8")


def canonical_without_root_digest(payload: dict[str, Any]) -> bytes:
    body = {key: value for key, value in payload.items() if key != "root_digest"}
    return json.dumps(body, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode("utf-8")


def load_trust_root(path: Path) -> dict[str, Any]:
    if path.is_symlink() or not path.is_file():
        die(f"trust root must be a regular file: {path}")
    try:
        root = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        die(f"cannot read trust root: {error}")
    if not isinstance(root, dict) or root.get("schema_version") != TRUST_ROOT_SCHEMA:
        die("unsupported trust root schema")
    root_digest = root.get("root_digest")
    if not isinstance(root_digest, str) or not HEX64.fullmatch(root_digest):
        die("trust root root_digest must be 64-hex")
    actual_digest = hashlib.sha256(canonical_without_root_digest(root)).hexdigest()
    if root_digest.lower() != actual_digest:
        die("trust root digest mismatch")
    allowlist = root.get("allowlist")
    if not isinstance(allowlist, list) or not allowlist:
        die("trust root allowlist is required")
    normalized: list[dict[str, str]] = []
    for entry in allowlist:
        if not isinstance(entry, dict):
            die("trust root allowlist entry must be an object")
        signer_id = entry.get("signer_id")
        algorithm = entry.get("algorithm")
        key_sha = entry.get("public_key_sha256")
        if not isinstance(signer_id, str) or not signer_id.strip() or not isinstance(algorithm, str) or not algorithm.strip() or not isinstance(key_sha, str) or not HEX64.fullmatch(key_sha):
            die("trust root allowlist entry is malformed")
        normalized_entry = {"signer_id": signer_id, "algorithm": algorithm, "public_key_sha256": key_sha.lower()}
        if entry.get("public_key_hex") is not None:
            raw_hex = entry.get("public_key_hex")
            if not isinstance(raw_hex, str) or not re.fullmatch(r"[0-9a-fA-F]{64}", raw_hex):
                die("trust root public_key_hex must be 32-byte hex")
            normalized_entry["public_key_hex"] = raw_hex.lower()
        normalized.append(normalized_entry)
    return {"schema_version": TRUST_ROOT_SCHEMA, "path": str(path.resolve()), "allowlist": normalized, "root_digest": root_digest.lower()}


def receipt_file(receipt_path: Path, raw_path: Any, field: str) -> Path:
    if not isinstance(raw_path, str) or not raw_path.strip():
        die(f"signature.{field} must be a file path")
    path = Path(raw_path)
    if not path.is_absolute():
        path = receipt_path.parent / path
    elif not path.exists():
        closure = receipt_path.with_name(receipt_path.name + ".closure.json")
        if closure.is_file():
            try:
                mapping = json.loads(closure.read_text(encoding="utf-8")).get("mapping", {})
            except (OSError, json.JSONDecodeError):
                mapping = {}
            mapped = mapping.get(str(raw_path))
            if isinstance(mapped, str):
                path = receipt_path.parent / mapped
        staged_fallback = receipt_path.parent / path.name
        if not path.exists() and staged_fallback.exists():
            path = staged_fallback
    if path.is_symlink() or not path.is_file():
        die(f"signature.{field} missing or symlinked: {path}")
    return path.resolve()


def verify_detached_signature(receipt_path: Path, receipt: dict[str, Any], signature: dict[str, Any]) -> None:
    if signature.get("algorithm") != "openssl-rsa-sha256":
        die("signature.algorithm must be openssl-rsa-sha256")
    signature_path = receipt_file(receipt_path, signature.get("signature_ref"), "signature_ref")
    public_key_path = receipt_file(receipt_path, signature.get("public_key_ref"), "public_key_ref")
    public_key_sha = signature.get("public_key_sha256")
    if not isinstance(public_key_sha, str) or not HEX64.fullmatch(public_key_sha):
        die("signature.public_key_sha256 must be 64-hex")
    if public_key_sha.lower() != sha256_file(public_key_path):
        die("signature public key hash mismatch")
    with tempfile.TemporaryDirectory(prefix="oasis7-provenance-verify-") as temp_dir:
        payload_path = Path(temp_dir) / "payload"
        payload_path.write_bytes(canonical_without_digest(receipt))
        try:
            result = subprocess.run(
                [
                    "openssl",
                    "dgst",
                    "-sha256",
                    "-verify",
                    str(public_key_path),
                    "-signature",
                    str(signature_path),
                    str(payload_path),
                ],
                check=False,
                capture_output=True,
                timeout=10,
            )
        except (FileNotFoundError, subprocess.TimeoutExpired) as error:
            die(f"detached signature verifier unavailable: {error.__class__.__name__}")
    if result.returncode != 0:
        die("detached signature verification failed")


def validate_receipt(receipt_path: Path, package_dir: Path, trust_root_path: Path | None = None) -> dict[str, Any]:
    try:
        receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        die(f"cannot read receipt: {error}")
    if receipt.get("schema_version") != SCHEMA:
        die("unsupported schema_version")
    if not receipt.get("network_id") or not receipt.get("chain_id"):
        die("network_id and chain_id are required")
    signature = receipt.get("signature")
    if not isinstance(signature, dict) or signature.get("status") != "verified":
        die("verified detached signature receipt is required")
    for field in ("signer_id", "algorithm", "signature_ref", "public_key_ref"):
        if not isinstance(signature.get(field), str) or not signature[field].strip():
            die(f"signature.{field} is required")
    digest = receipt.get("binding_digest")
    if not isinstance(digest, str) or not HEX64.fullmatch(digest):
        die("binding_digest must be 64-hex")
    actual_digest = hashlib.sha256(canonical_without_digest(receipt)).hexdigest()
    if digest.lower() != actual_digest:
        die("binding_digest mismatch")
    verify_detached_signature(receipt_path, receipt, signature)
    trust_root = load_trust_root(trust_root_path) if trust_root_path is not None else None
    if trust_root is not None:
        candidate = {
            "signer_id": signature["signer_id"],
            "algorithm": signature["algorithm"],
            "public_key_sha256": str(signature.get("public_key_sha256", "")).lower(),
        }
        if candidate not in trust_root["allowlist"]:
            die("signature signer is not in the governed trusted signer allowlist")

    package = receipt.get("package")
    if not isinstance(package, dict):
        die("package binding is required")
    runtime = find_runtime(package_dir)
    buildinfo_candidates = [package_dir / "BUILDINFO"] + sorted(package_dir.glob("*-BUILDINFO"))
    buildinfo_path = next((candidate for candidate in buildinfo_candidates if candidate.is_file()), None)
    if buildinfo_path is None:
        die("BUILDINFO missing")
    buildinfo = parse_buildinfo(buildinfo_path)
    sums_sha, _ = verify_checksums(package_dir, runtime)
    runtime_sha = sha256_file(runtime)
    if package.get("runtime_sha256") != runtime_sha or int(package.get("runtime_size_bytes", -1)) != runtime.stat().st_size:
        die("package runtime hash/size binding mismatch")
    if (
        package.get("commit") != buildinfo.get("commit")
        or str(package.get("run_id")) != buildinfo.get("run_id")
        or package.get("package_version") != buildinfo.get("package_version")
    ):
        die("package BUILDINFO commit/run/package-version binding mismatch")
    if package.get("buildinfo_sha256") != sha256_file(buildinfo_path) or package.get("sha256sums_sha256") != sums_sha:
        die("package BUILDINFO/SHA256SUMS binding mismatch")

    governed = receipt.get("governed")
    if not isinstance(governed, dict):
        die("governed binding is required")
    if set(governed) != REQUIRED_GOVERNED_KEYS:
        missing = sorted(REQUIRED_GOVERNED_KEYS - set(governed))
        extra = sorted(set(governed) - REQUIRED_GOVERNED_KEYS)
        die(f"governed keys must be exact; missing={missing} extra={extra}")
    for key, value in governed.items():
        if not isinstance(value, dict) or not isinstance(value.get("path"), str):
            die(f"governed.{key} is malformed")
        path = Path(value["path"])
        if not path.is_absolute():
            path = receipt_path.parent / path
        elif not path.exists():
            closure = receipt_path.with_name(receipt_path.name + ".closure.json")
            mapping: dict[str, Any] = {}
            if closure.is_file():
                try:
                    mapping = json.loads(closure.read_text(encoding="utf-8")).get("mapping", {})
                except (OSError, json.JSONDecodeError):
                    mapping = {}
            mapped = mapping.get(value["path"])
            if isinstance(mapped, str):
                path = receipt_path.parent / mapped
            staged_fallback = receipt_path.parent / path.name
            if not path.exists() and staged_fallback.exists():
                path = staged_fallback
        actual = metadata(path, allow_directory=value.get("kind") == "directory")
        expected_sha = value.get("sha256") or value.get("sha256_tree")
        actual_sha = actual.get("sha256") or actual.get("sha256_tree")
        if expected_sha != actual_sha or int(value.get("size_bytes", value.get("total_bytes", -1))) != int(actual.get("size_bytes", actual.get("total_bytes", -2))):
            die(f"governed.{key} hash/size binding mismatch")
    return {
        "schema_version": SCHEMA,
        "binding_digest": digest.lower(),
        "package": package,
        "governed": governed,
        "signature": {"signer_id": signature["signer_id"], "algorithm": signature["algorithm"]},
        "network_id": receipt["network_id"],
        "chain_id": receipt["chain_id"],
        "trusted_root": trust_root,
    }


def create(args: argparse.Namespace) -> int:
    package_dir = Path(args.package_dir).resolve()
    runtime = find_runtime(package_dir)
    buildinfo_candidates = [package_dir / "BUILDINFO"] + sorted(package_dir.glob("*-BUILDINFO"))
    buildinfo_path = next((candidate for candidate in buildinfo_candidates if candidate.is_file()), None)
    if buildinfo_path is None:
        die("BUILDINFO missing")
    buildinfo = parse_buildinfo(buildinfo_path)
    sums_sha, _ = verify_checksums(package_dir, runtime)
    governed: dict[str, Any] = {}
    for key, raw_path in {
        "manifest": args.manifest,
        "genesis": args.genesis,
        "registry": args.registry,
        "bootstrap": args.bootstrap,
        "world": args.world,
    }.items():
        governed[key] = metadata(Path(raw_path).resolve(), allow_directory=key == "world")
    payload: dict[str, Any] = {
        "schema_version": SCHEMA,
        "network_id": args.network_id,
        "chain_id": args.chain_id,
        "package": {
            "run_id": buildinfo["run_id"],
            "commit": buildinfo["commit"],
            "package_version": buildinfo["package_version"],
            "runtime_sha256": sha256_file(runtime),
            "runtime_size_bytes": runtime.stat().st_size,
            "buildinfo_sha256": sha256_file(buildinfo_path),
            "sha256sums_sha256": sums_sha,
        },
        "governed": governed,
        "signature": {
            "status": "verified" if args.verified_signature else "unverified",
            "signer_id": args.signer_id,
            "algorithm": args.signature_algorithm,
            "signature_ref": args.signature_ref,
            "public_key_ref": args.public_key_ref,
        },
    }
    if args.public_key_ref:
        public_key_path = Path(args.public_key_ref).resolve()
        if public_key_path.is_symlink() or not public_key_path.is_file():
            die(f"--public-key-ref must point to a regular file: {public_key_path}")
        payload["signature"]["public_key_sha256"] = sha256_file(public_key_path)
    if args.verified_signature and not args.public_key_ref:
        die("--public-key-ref is required with --verified-signature")
    payload["binding_digest"] = hashlib.sha256(canonical_without_digest(payload)).hexdigest()
    output = Path(args.output).resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(payload, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"schema_version": SCHEMA, "binding_digest": payload["binding_digest"], "output": str(output)}))
    return 0


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    sub = root.add_subparsers(dest="mode", required=True)
    create_parser = sub.add_parser("create")
    create_parser.add_argument("--package-dir", required=True)
    for option in ("manifest", "genesis", "registry", "bootstrap", "world"):
        create_parser.add_argument(f"--{option}", required=True)
    create_parser.add_argument("--network-id", required=True)
    create_parser.add_argument("--chain-id", required=True)
    create_parser.add_argument("--output", required=True)
    create_parser.add_argument("--signer-id", required=True)
    create_parser.add_argument("--signature-ref", required=True)
    create_parser.add_argument("--public-key-ref")
    create_parser.add_argument("--signature-algorithm", default="openssl-rsa-sha256")
    create_parser.add_argument("--verified-signature", action="store_true")
    validate_parser = sub.add_parser("validate")
    validate_parser.add_argument("--provenance", required=True)
    validate_parser.add_argument("--package-dir", required=True)
    validate_parser.add_argument("--trust-root")
    return root


def main() -> int:
    args = parser().parse_args()
    if args.mode == "create":
        return create(args)
    summary = validate_receipt(
        Path(args.provenance).resolve(),
        Path(args.package_dir).resolve(),
        Path(args.trust_root).resolve() if args.trust_root else None,
    )
    print(json.dumps(summary, ensure_ascii=True, sort_keys=True))
    return 0


if __name__ == "__main__":
    main()
