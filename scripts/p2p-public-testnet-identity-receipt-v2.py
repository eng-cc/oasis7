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
import subprocess
import tempfile
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any, Callable, NoReturn


SCRIPT_DIR = Path(__file__).resolve().parent
PLANNER_PATH = SCRIPT_DIR / "p2p-public-testnet-full-network-clean-room.py"
IDENTITY_V2_SIGNER_TOOL_PATH = SCRIPT_DIR / "p2p-public-testnet-identity-v2-signing-tool.py"
IDENTITY_V2_SIGNER_TOOL_SHA256: str | None = None
IDENTITY_V2_VERIFIER_TOOL_PATH = SCRIPT_DIR / "p2p-public-testnet-identity-v2-signing-tool.py"
IDENTITY_V2_VERIFIER_TOOL_SHA256: str | None = None
RAW_V1_SCHEMA = "oasis7.identity_receipt.v1"
SYNTHETIC_DIGEST = "a" * 64
SYNTHETIC_SIGNATURE = "b" * 128
HEX64_RE = re.compile(r"^[0-9a-fA-F]{64}$")
SIGNATURE_RE = re.compile(r"^[0-9a-fA-F]{128}$")
OID_RE = re.compile(r"^[0-9a-fA-F]{40,64}$")
SAFE_NAME_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{1,127}$")
MAX_CLOCK_SKEW_SECONDS = 5
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


def _validate_raw_v1(raw_bytes: bytes) -> dict[str, Any]:
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
    if raw["key_mode"] != int("0600", 8):
        die("raw-v1 key_mode must be 0600")
    return raw


def _parse_timestamp(value: Any, label: str) -> datetime:
    if not isinstance(value, str) or not value.strip():
        die(f"{label} must be an RFC3339 UTC timestamp")
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        die(f"{label} must be an RFC3339 UTC timestamp")
    if parsed.tzinfo is None or parsed.utcoffset() is None:
        die(f"{label} must include an explicit UTC offset")
    return parsed.astimezone(timezone.utc)


def _validate_context(context: dict[str, Any], raw: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(context, dict):
        die("governed signing context must be an object")
    required = {"task_uid", "frozen_head_oid", "plan_digest", "node_id", "peer_id"}
    if set(context) != required:
        missing = sorted(required - set(context))
        extra = sorted(set(context) - required)
        die(f"governed signing context fields are not exact (missing={missing}, extra={extra})")
    for field in required:
        if not isinstance(context[field], str) or not context[field].strip():
            die(f"governed signing context {field} must be a non-empty string")
    if SAFE_NAME_RE.fullmatch(context["task_uid"].replace("_", "-")) is None:
        die("governed signing context task_uid is not a safe identifier")
    if OID_RE.fullmatch(context["frozen_head_oid"]) is None:
        die("governed signing context frozen_head_oid is not a commit oid")
    if context["node_id"] != raw["node_id"] or context["peer_id"] != raw["peer_id"]:
        die("governed signing context identity does not match exact raw-v1 identity")
    return context


def _validate_template(
    template: dict[str, Any], planner: Any, raw: dict[str, Any] | None = None
) -> dict[str, Any]:
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
    issued_at = _parse_timestamp(template.get("issued_at"), "v2 template.issued_at")
    expires_at = _parse_timestamp(template.get("expires_at"), "v2 template.expires_at")
    if expires_at <= issued_at:
        die("v2 template freshness window is inverted")
    now = datetime.now(timezone.utc)
    if expires_at <= now:
        die("v2 template freshness window is stale")
    if issued_at > now + timedelta(seconds=MAX_CLOCK_SKEW_SECONDS):
        die("v2 template.issued_at is in the future")
    if SAFE_NAME_RE.fullmatch(template["capture_window_id"]) is None:
        die("v2 template capture_window_id is not a safe identifier")
    if template["rotation_epoch"] != planner.CANONICAL_ROTATION_EPOCH:
        die("v2 template rotation_epoch is not the governed rotation epoch")
    if raw is not None:
        identity_fields = ("node_id", "peer_id", "key_sha256", "key_size_bytes", "key_uid", "key_gid")
        for field in identity_fields:
            raw_value = raw[field]
            template_value = template[field]
            if field == "key_sha256":
                matches = str(raw_value).lower() == str(template_value).lower()
            else:
                matches = raw_value == template_value
            if not matches:
                die(f"v2 template {field} is not bound to exact raw-v1 identity")
        if format(raw["key_mode"], "04o") != template["key_mode"]:
            die("v2 template key_mode is not bound to exact raw-v1 identity")
    if signed_payload == SYNTHETIC_DIGEST and signature == SYNTHETIC_SIGNATURE:
        die("v2 template contains the reserved synthetic digest/signature pair")
    return template


def _canonical_signing_payload(envelope: dict[str, Any], context: dict[str, Any]) -> bytes:
    """Return the one immutable payload used by both signer and verifier.

    Signature and canonical digest are intentionally omitted: both are derived
    from this finalized canonical body and therefore cannot introduce a
    circular signing dependency.  The governed context is included in the
    signed bytes even though it remains an adapter-side binding at admission.
    """
    payload = {
        key: value
        for key, value in envelope.items()
        if key not in {"signature_hex", "canonical_digest"}
    }
    payload["governed_context"] = context
    return json.dumps(payload, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode(
        "utf-8"
    )


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


def _regular_executable(path_value: str | Path, label: str) -> Path:
    """Validate a tool executable without turning it into caller authority."""
    path = Path(path_value)
    _regular_file(path, label)
    if not os.access(path, os.X_OK):
        die(f"{label} must be executable: {path}")
    return path


def _pinned_tool(
    path_value: str | Path,
    expected_path_value: str | Path,
    expected_sha256: str | None,
    label: str,
) -> Path:
    """Require an exact code-owned tool path and independently pinned bytes."""
    path = _regular_executable(path_value, label)
    expected_path = Path(expected_path_value)
    if path.resolve() != expected_path.resolve():
        die(f"{label} is not the code-owned pinned path")
    if expected_sha256 is None or HEX64_RE.fullmatch(expected_sha256) is None:
        die(f"{label} digest pin is not provisioned")
    try:
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
    except OSError as error:
        die(f"cannot read {label}: {error.__class__.__name__}")
    if actual != expected_sha256.lower():
        die(f"{label} digest pin does not match its bytes")
    return path


def _descriptor(path: Path, label: str) -> dict[str, Any]:
    _regular_file(path, label)
    try:
        value = path.read_bytes()
    except OSError as error:
        die(f"cannot read {label}: {error.__class__.__name__}")
    # Evidence retains the caller's exact path spelling; the signing tool
    # independently resolves paths while validating authority bindings.
    return {"path": str(path), "sha256": hashlib.sha256(value).hexdigest(), "size_bytes": len(value)}


def _clear_derived_output(path: Path, label: str) -> None:
    if not path.exists():
        return
    if path.is_symlink() or not path.is_file():
        die(f"{label} must be an absent regular output or a regular file")
    try:
        path.unlink()
    except OSError as error:
        die(f"cannot clear stale {label}: {error.__class__.__name__}")


def _run_signing_command(tool: Path, command: str, arguments: list[str]) -> None:
    """Run only the fixed file-oriented signing-tool vocabulary.

    The executable is an adapter selected by deployment custody.  This
    sidecar supplies every argument and never accepts a shell command,
    endpoint, environment, or provider-specific argument from the caller.
    """
    try:
        completed = subprocess.run(
            [str(tool), command, *arguments],
            cwd=str(SCRIPT_DIR),
            env={"PATH": os.environ.get("PATH", ""), "PYTHONIOENCODING": "utf-8"},
            capture_output=True,
            text=True,
            timeout=60,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        die(f"signing-tool {command} failed: {error.__class__.__name__}")
    if completed.returncode != 0:
        detail = completed.stderr.strip().splitlines()[-1] if completed.stderr.strip() else "no diagnostic"
        die(f"signing-tool {command} failed: {detail[:240]}")


def _bridge_create(args: argparse.Namespace) -> dict[str, Any]:
    """Execute prepare/sign/assemble/verify and retain the complete evidence map."""
    # Pin both executables before clearing or creating any derived output.  A
    # caller may assert the configured path, but cannot choose a tool or hash.
    signer_tool = _pinned_tool(
        args.signer_tool,
        IDENTITY_V2_SIGNER_TOOL_PATH,
        IDENTITY_V2_SIGNER_TOOL_SHA256,
        "signer tool",
    )
    verifier_tool = _pinned_tool(
        args.verifier_tool,
        IDENTITY_V2_VERIFIER_TOOL_PATH,
        IDENTITY_V2_VERIFIER_TOOL_SHA256,
        "verifier tool",
    )
    raw_path = _regular_file(args.raw_v1, "raw-v1")
    template_path = _regular_file(args.template, "v2 template")
    context_path = _regular_file(args.context, "signing context")
    intent_path = _regular_file(args.plan_intent, "plan intent")
    trust_path = _regular_file(args.trust_config, "trust config")
    registry_path = _regular_file(args.provider_registry, "provider registry")
    output_path = Path(args.out)
    evidence_path = Path(args.evidence_map_out)
    _clear_derived_output(output_path, "verified envelope output")
    _clear_derived_output(evidence_path, "evidence-map output")
    # All intermediate files are disposable and private to this transaction.
    with tempfile.TemporaryDirectory(prefix="oasis7-identity-v2-sidecar-") as directory:
        work = Path(directory)
        payload = work / "payload.bin"
        manifest = work / "prepare-manifest.json"
        signature = work / "signature.hex"
        attestation = work / "provider-attestation.json"
        envelope = work / "unsigned-envelope.json"
        verified = work / "verified-envelope.json"
        verification = work / "verification.json"
        common = [
            "--raw-v1", str(raw_path), "--template", str(template_path),
            "--context", str(context_path), "--plan-intent", str(intent_path),
            "--trust-config", str(trust_path), "--provider-registry", str(registry_path),
        ]
        _run_signing_command(
            signer_tool,
            "prepare",
            [*common, "--payload-out", str(payload), "--manifest-out", str(manifest)],
        )
        _run_signing_command(
            signer_tool,
            "sign",
            [
                "--payload", str(payload), "--manifest", str(manifest),
                "--provider-registry", str(registry_path), "--provider-ref", args.provider_ref,
                "--signature-out", str(signature), "--attestation-out", str(attestation),
            ],
        )
        _run_signing_command(
            signer_tool,
            "assemble",
            [
                "--payload", str(payload), "--manifest", str(manifest),
                "--signature", str(signature), "--attestation", str(attestation),
                "--provider-registry", str(registry_path), "--out", str(envelope),
            ],
        )
        _run_signing_command(
            verifier_tool,
            "verify",
            [
                "--mode", "current_admission", "--envelope", str(envelope),
                "--raw-v1", str(raw_path), "--context", str(context_path),
                "--plan-intent", str(intent_path), "--trust-config", str(trust_path),
                "--provider-registry", str(registry_path), "--out", str(verified),
                "--verification-out", str(verification),
            ],
        )
        final_bytes = verified.read_bytes()
        try:
            final_value = json.loads(final_bytes.decode("utf-8"))
        except (UnicodeError, json.JSONDecodeError):
            die("signing-tool verify did not produce a JSON envelope")
        if not isinstance(final_value, dict) or final_value.get("verified") is not True:
            die("signing-tool verify did not produce an authenticated envelope")
        node_name = next(
            (
                name
                for name, expected in _load_planner().EXPECTED_NODES.items()
                if expected.get("node_id") == final_value.get("node_id")
            ),
            None,
        )
        if node_name is None:
            die("verified envelope node is outside the governed fleet")
        # Use the same atomic boundary as the legacy sidecar, but preserve the
        # verifier's exact canonical bytes for evidence-map hashing.
        if output_path.exists() and output_path.is_symlink():
            die(f"output must not be a symlink: {output_path}")
        try:
            output_path.parent.mkdir(parents=True, exist_ok=True)
            with tempfile.NamedTemporaryFile(dir=output_path.parent, prefix=f".{output_path.name}.", delete=False) as temporary:
                temporary_path = Path(temporary.name)
                temporary.write(final_bytes)
                temporary.flush()
                os.fsync(temporary.fileno())
            temporary_path.chmod(0o600)
            os.replace(temporary_path, output_path)
        except OSError as error:
            try:
                temporary_path.unlink(missing_ok=True)
            except (NameError, OSError):
                pass
            die(f"cannot atomically write verified envelope: {error.__class__.__name__}")
        evidence = {
            "schema_version": "oasis7.identity_v2_evidence_map.v1",
            "task_uid": final_value.get("task_uid"),
            "head_oid": final_value.get("head_oid"),
            "context": _descriptor(context_path, "signing context"),
            "plan_intent": _descriptor(intent_path, "plan intent"),
            "entries": [
                {
                    "node_name": node_name,
                    "node_id": final_value.get("node_id"),
                    "peer_id": final_value.get("peer_id"),
                    "raw_v1": _descriptor(raw_path, "raw-v1"),
                    "signed_envelope": _descriptor(output_path, "verified envelope"),
                    "verification": _descriptor(verification, "verification receipt"),
                }
            ],
        }
        try:
            _write_atomically(evidence_path, evidence)
        except SystemExit:
            # Do not leave a verified envelope without the evidence closure.
            _clear_derived_output(output_path, "verified envelope output")
            raise
        return final_value


def create(
    raw_path: Path,
    template_path: Path,
    output_path: Path,
    *,
    context: dict[str, Any] | None = None,
    signer: Callable[[bytes, dict[str, Any]], str] | None = None,
    verifier: Callable[[bytes, dict[str, Any], dict[str, Any]], dict[str, Any]] | None = None,
) -> dict[str, Any]:
    if (signer is None) != (verifier is None):
        die("signer and verifier seams must be supplied together")
    if signer is not None and context is None:
        die("governed signing context is required when signer/verifier seams are used")
    planner = _load_planner()
    _regular_file(raw_path, "raw-v1")
    raw_bytes = raw_path.read_bytes()
    raw = _validate_raw_v1(raw_bytes)
    template = _validate_template(_read_json(template_path, "v2 template"), planner, raw)
    envelope = dict(template)
    envelope["signed_payload_sha256"] = hashlib.sha256(raw_bytes).hexdigest()
    validated_context = _validate_context(context, raw) if context is not None else None
    if signer is not None and verifier is not None:
        # Use callback-local copies so a provider cannot mutate the context or
        # envelope after the finalized bytes have been selected.
        finalized_payload = _canonical_signing_payload(envelope, validated_context)
        try:
            signature = signer(finalized_payload, dict(validated_context))
        except Exception as error:
            die(f"governed signer failed: {error.__class__.__name__}")
        signature = _require_hex(signature, SIGNATURE_RE, "governed signer signature")
        envelope["signature_hex"] = signature
        envelope["canonical_digest"] = planner._canonical_receipt_digest(
            envelope, excluded_fields=frozenset({"peer_id"})
        )
        try:
            verification = verifier(
                finalized_payload, dict(envelope), dict(validated_context)
            )
        except Exception as error:
            die(f"governed verifier failed: {error.__class__.__name__}")
        if not isinstance(verification, dict) or verification.get("verified") is not True:
            die("governed verifier did not verify the finalized canonical payload")
    else:
        # A shape-valid template is useful as a planner input fixture, but a
        # sidecar output without the governed signer/verifier seam is not an
        # admissible authenticated receipt.  Keep the distinction explicit.
        envelope["authenticated"] = False
        envelope["verified"] = False
    if signer is None or verifier is None:
        envelope["canonical_digest"] = planner._canonical_receipt_digest(
            envelope, excluded_fields=frozenset({"peer_id"})
        )
    envelope.pop("key_path", None)
    _write_atomically(output_path, envelope)
    return envelope


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    root.add_argument("--raw-v1", required=True, type=Path)
    root.add_argument("--template", required=True, type=Path)
    root.add_argument("--out", required=True, type=Path)
    # The four-command bridge is deliberately opt-in.  Omitting any bridge
    # option retains the legacy shape-only callback seam for existing callers.
    root.add_argument("--context", type=Path)
    root.add_argument("--plan-intent", type=Path)
    root.add_argument("--trust-config", type=Path)
    root.add_argument("--provider-registry", type=Path)
    root.add_argument("--provider-ref")
    root.add_argument("--signer-tool", type=Path)
    root.add_argument("--verifier-tool", type=Path)
    root.add_argument("--evidence-map-out", type=Path)
    return root


def main() -> int:
    args = parser().parse_args()
    bridge_values = (
        args.context,
        args.plan_intent,
        args.trust_config,
        args.provider_registry,
        args.provider_ref,
        args.signer_tool,
        args.verifier_tool,
        args.evidence_map_out,
    )
    if any(value is not None for value in bridge_values):
        if not all(value is not None for value in bridge_values):
            die("bridge mode requires context, plan-intent, trust/provider config, provider-ref, signer/verifier tools, and evidence-map-out")
        result = _bridge_create(args)
    else:
        result = create(args.raw_v1, args.template, args.out)
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
