#!/usr/bin/env python3
"""Bind an already-signed identity envelope to exact raw runtime bytes.

The runtime's ``identity_receipt.v1`` output is raw metadata only.  This
sidecar does not create keys or signatures: it validates the existing
signer/verifier/trust-root seam, hashes the exact bytes supplied by the
runtime, and atomically emits the governed ``identity_receipt.v2`` envelope.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import re
import tempfile
from pathlib import Path
from typing import Any, NoReturn


SCRIPT_DIR = Path(__file__).resolve().parent
PLANNER_PATH = SCRIPT_DIR / "p2p-public-testnet-full-network-clean-room.py"
RAW_V1_SCHEMA = "oasis7.identity_receipt.v1"
SYNTHETIC_DIGEST = "a" * 64
SYNTHETIC_SIGNATURE = "b" * 128
HEX64_RE = re.compile(r"^[0-9a-fA-F]{64}$")
SIGNATURE_RE = re.compile(r"^[0-9a-fA-F]{128}$")
RAW_V1_FIELDS = frozenset(
    {
        "schema_version",
        "node_id",
        "peer_id",
        "key_path",
        "key_sha256",
        "key_size_bytes",
        "key_mode",
        "key_uid",
        "key_gid",
    }
)


def die(message: str) -> NoReturn:
    raise SystemExit(f"error: identity receipt v2 sidecar: {message}")


def _load_planner() -> Any:
    spec = importlib.util.spec_from_file_location("full_network_clean_room", PLANNER_PATH)
    if spec is None or spec.loader is None:
        die(f"cannot load governed verifier seam: {PLANNER_PATH}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _regular_file(path: Path, label: str) -> Path:
    if path.is_symlink() or not path.is_file():
        die(f"{label} must be a regular non-symlink file: {path}")
    return path


def _read_json(path: Path, label: str) -> dict[str, Any]:
    _regular_file(path, label)
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        die(f"cannot read {label}: {error.__class__.__name__}")
    if not isinstance(value, dict):
        die(f"{label} must contain a JSON object")
    return value


def _require_hex(value: Any, pattern: re.Pattern[str], label: str) -> str:
    if not isinstance(value, str) or pattern.fullmatch(value) is None:
        die(f"{label} has invalid hexadecimal shape")
    if not any(character != "0" for character in value):
        die(f"{label} must not be all zeroes")
    return value.lower()


def _validate_raw_v1(raw_bytes: bytes) -> None:
    if not raw_bytes:
        die("raw-v1 input must not be empty")
    try:
        raw = json.loads(raw_bytes.decode("utf-8"))
    except (UnicodeError, json.JSONDecodeError) as error:
        die(f"raw-v1 input is not valid UTF-8 JSON: {error.__class__.__name__}")
    if not isinstance(raw, dict):
        die("raw-v1 input must contain a JSON object")
    if raw.get("schema_version") != RAW_V1_SCHEMA:
        die("raw-v1 input must use oasis7.identity_receipt.v1")
    if set(raw) != RAW_V1_FIELDS:
        missing = sorted(RAW_V1_FIELDS - set(raw))
        extra = sorted(set(raw) - RAW_V1_FIELDS)
        die(f"raw-v1 fields are not exact (missing={missing}, extra={extra})")
    for field in ("node_id", "peer_id", "key_path"):
        if not isinstance(raw.get(field), str) or not raw[field].strip():
            die(f"raw-v1 {field} must be a non-empty string")
    _require_hex(raw.get("key_sha256"), HEX64_RE, "raw-v1.key_sha256")
    for field in ("key_size_bytes", "key_mode", "key_uid", "key_gid"):
        value = raw.get(field)
        if not isinstance(value, int) or isinstance(value, bool) or value < 0:
            die(f"raw-v1 {field} must be a non-negative integer")
        if field == "key_size_bytes" and value == 0:
            die("raw-v1 key_size_bytes must be positive")


def _validate_template(template: dict[str, Any], planner: Any) -> dict[str, Any]:
    if set(template) != planner.IDENTITY_RECEIPT_FIELDS:
        missing = sorted(planner.IDENTITY_RECEIPT_FIELDS - set(template))
        extra = sorted(set(template) - planner.IDENTITY_RECEIPT_FIELDS)
        die(f"v2 template fields are not exact (missing={missing}, extra={extra})")
    if template.get("schema_version") != planner.IDENTITY_RECEIPT_SCHEMA:
        die("v2 template schema is unsupported")
    if template.get("authenticated") is not True or template.get("verified") is not True:
        die("v2 template must be authenticated and independently verified")
    if template.get("signer_id") not in planner.CANONICAL_SIGNER_ALLOWLIST:
        die("v2 template signer is outside the governed signer allowlist")
    if template.get("verifier_id") != planner.CANONICAL_VERIFIER_ID:
        die("v2 template verifier is not the governed verifier")
    if template.get("trust_root_id") != planner.CANONICAL_TRUST_ROOT_ID:
        die("v2 template trust root is not the governed root")
    for field in (
        "node_id",
        "peer_id",
        "key_sha256",
        "key_mode",
        "capture_window_id",
        "rotation_epoch",
        "issued_at",
        "expires_at",
    ):
        if not isinstance(template.get(field), str) or not template[field].strip():
            die(f"v2 template {field} must be a non-empty string")
    for field in ("key_size_bytes", "key_uid", "key_gid"):
        value = template.get(field)
        if not isinstance(value, int) or isinstance(value, bool) or value < 0:
            die(f"v2 template {field} must be a non-negative integer")
        if field == "key_size_bytes" and value == 0:
            die("v2 template key_size_bytes must be positive")
    _require_hex(template.get("key_sha256"), HEX64_RE, "v2 template.key_sha256")
    signed_payload = _require_hex(
        template.get("signed_payload_sha256"), HEX64_RE, "v2 template.signed_payload_sha256"
    )
    signature = _require_hex(template.get("signature_hex"), SIGNATURE_RE, "v2 template.signature_hex")
    _require_hex(template.get("canonical_digest"), HEX64_RE, "v2 template.canonical_digest")
    if signed_payload == SYNTHETIC_DIGEST and signature == SYNTHETIC_SIGNATURE:
        die("v2 template contains the reserved synthetic digest/signature pair")
    return template


def _write_atomically(path: Path, value: dict[str, Any]) -> None:
    if path.exists() and path.is_symlink():
        die(f"output must not be a symlink: {path}")
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.NamedTemporaryFile(
            mode="w", encoding="utf-8", dir=path.parent, prefix=f".{path.name}.", delete=False
        ) as temporary:
            temporary_path = Path(temporary.name)
            temporary.write(json.dumps(value, ensure_ascii=True, sort_keys=True, indent=2))
            temporary.write("\n")
            temporary.flush()
            os.fsync(temporary.fileno())
        temporary_path.chmod(0o600)
        os.replace(temporary_path, path)
    except OSError as error:
        try:
            temporary_path.unlink(missing_ok=True)
        except (NameError, OSError):
            pass
        die(f"cannot atomically write output: {error.__class__.__name__}")


def create(raw_path: Path, template_path: Path, output_path: Path) -> dict[str, str]:
    planner = _load_planner()
    _regular_file(raw_path, "raw-v1")
    raw_bytes = raw_path.read_bytes()
    _validate_raw_v1(raw_bytes)
    template = _validate_template(_read_json(template_path, "v2 template"), planner)
    envelope = dict(template)
    envelope["signed_payload_sha256"] = hashlib.sha256(raw_bytes).hexdigest()
    envelope["canonical_digest"] = planner._canonical_receipt_digest(
        envelope, excluded_fields=frozenset({"peer_id"})
    )
    envelope.pop("key_path", None)
    _write_atomically(output_path, envelope)
    return {
        "schema_version": planner.IDENTITY_RECEIPT_SCHEMA,
        "signed_payload_sha256": envelope["signed_payload_sha256"],
        "canonical_digest": envelope["canonical_digest"],
        "output": str(output_path),
    }


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    root.add_argument("--raw-v1", required=True, type=Path)
    root.add_argument("--template", required=True, type=Path)
    root.add_argument("--out", required=True, type=Path)
    return root


def main() -> int:
    args = parser().parse_args()
    print(json.dumps(create(args.raw_v1, args.template, args.out), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
