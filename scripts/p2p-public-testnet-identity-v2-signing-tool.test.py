#!/usr/bin/env python3
"""RED contract tests for the proposed identity-receipt-v2 executable.

These tests intentionally target the not-yet-present
``p2p-public-testnet-identity-v2-signing-tool.py``.  All authority material is
created inside a temporary directory.  The provider fixture keeps its private
key in that temporary custody boundary and signs with the host's OpenSSL 3
Ed25519 implementation; it is never a deployment provider or a readiness
fixture.

The test-only provider protocol is deliberately narrow: the executable must
resolve ``--provider-ref`` through the pinned registry, then invoke the pinned
adapter with a request JSON containing the payload path and signed context.
There is no caller-selected command, endpoint, private-key option, or
``verified=true`` shortcut.
"""

from __future__ import annotations

import hashlib
import json
import os
import stat
import subprocess
import sys
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
TOOL = ROOT / "scripts" / "p2p-public-testnet-identity-v2-signing-tool.py"
OPENSSL = Path("/opt/homebrew/bin/openssl")
PREFIX = b"OASIS7-IDENTITY-RECEIPT-V2\0"
NETWORK_ID = "oasis7-public-testnet-governed-20260606"
TRUST_ROOT_ID = "oasis7-public-testnet-governance-root-v1"
VERIFIER_ID = "governed-receipt-verifier"
SIGNER_ID = "identity-v2-ephemeral-test-signer"
PROVIDER_ID = "ephemeral-test-custody"
ROTATION_EPOCH = "identity-v2-rotation-0001"
TASK_UID = "task_174f0a5a87394012b071171cc4a52372"
NODE_ID = "triad-testnet-sequencer"
PEER_ID = "12D3KooWIdentityV2TestPeer"

# Test-only child process loader. It imports the real tool and adapter, then
# redirects only their authority constants to temporary files. The production
# CLI is still exercised through its unchanged ``main(argv)`` entrypoint, and
# no production environment variable or command-line bypass is introduced.
CHILD_HARNESS = r'''
import importlib.util
import os
import sys
from pathlib import Path

tool_path = Path(sys.argv[1])
governance_root = Path(sys.argv[2])
trust_config = Path(sys.argv[3])
provider_registry = Path(sys.argv[4])
trusted_config_sha256 = sys.argv[5]
trusted_registry_sha256 = sys.argv[6]
adapter_path = tool_path.with_name("p2p-public-testnet-full-network-clean-room-adapter.py")


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise SystemExit("cannot load test harness module")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


adapter = load("oasis7_identity_v2_adapter", adapter_path)
adapter.CANONICAL_TRUST_ROOT_PATH = str(governance_root)
adapter.CANONICAL_TRUST_ROOT_FILE_SHA256 = "f278bc8f060cd6777d68f086fc3131edc5d6b5a6080bde09208ba69a69e3ef66"
adapter.CANONICAL_TRUST_ROOT_OWNER_UID = os.getuid()
adapter.CANONICAL_TRUST_ROOT_MODE = "0600"

tool = load("oasis7_identity_v2_signing_tool", tool_path)
tool.DEPLOYED_TRUST_CONFIG = trust_config
tool.DEPLOYED_PROVIDER_REGISTRY = provider_registry
tool.PINNED_TRUST_CONFIG_SHA256 = trusted_config_sha256
tool.PINNED_PROVIDER_REGISTRY_SHA256 = trusted_registry_sha256
tool.DEPLOYED_GOVERNANCE_ROOT = governance_root
raise SystemExit(tool.main(sys.argv[7:]))
'''


def canonical(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode(
        "utf-8"
    )


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def digest_file(path: Path) -> str:
    return digest_bytes(path.read_bytes())


def write_json(path: Path, value: Any) -> None:
    path.write_bytes(canonical(value))


def provider_source(private_key: Path) -> str:
    """Return an isolated custody adapter using real OpenSSL Ed25519."""

    return f'''#!/usr/bin/env python3
import argparse
import hashlib
import json
import subprocess
from pathlib import Path

OPENSSL = {str(OPENSSL)!r}
PRIVATE_KEY = {str(private_key)!r}

parser = argparse.ArgumentParser()
parser.add_argument("--request", required=True)
parser.add_argument("--signature-out", required=True)
parser.add_argument("--attestation-out", required=True)
args = parser.parse_args()
request = json.loads(Path(args.request).read_text(encoding="utf-8"))
payload_path = Path(request["payload_path"])
payload = payload_path.read_bytes()
signature_path = Path(args.signature_out)
subprocess.run(
    [OPENSSL, "pkeyutl", "-sign", "-inkey", PRIVATE_KEY, "-rawin", "-in", str(payload_path), "-out", str(signature_path)],
    check=True,
)
signature = signature_path.read_bytes()
if len(signature) != 64:
    raise SystemExit("test provider produced a non-Ed25519 signature")
attestation = {{
    "schema_version": "oasis7.identity_v2_provider_attestation.v1",
    "provider_id": request["provider_id"],
    "request_id": request["request_id"],
    "signer_id": request["signer_id"],
    "public_key_sha256": request["public_key_sha256"],
    "algorithm": "ed25519",
    "canonical_payload_sha256": hashlib.sha256(payload).hexdigest(),
    "signature_sha256": hashlib.sha256(signature).hexdigest(),
    "context_digest": request["context_digest"],
    "rotation_epoch": request["rotation_epoch"],
    "capture_window_id": request["capture_window_id"],
    "issued_at": request["issued_at"],
    "detached_provider_authentication_proof": "test-only-openssl:" + hashlib.sha256(signature).hexdigest(),
}}
Path(args.attestation_out).write_bytes(json.dumps(attestation, sort_keys=True, separators=(",", ":")).encode("utf-8"))
'''


class IdentityV2SigningToolContractTests(unittest.TestCase):
    """Acceptance contract for the future prepare/sign/assemble/verify tool."""

    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="oasis7-identity-v2-test-")
        self.root = Path(self.temp.name)
        self.raw = self.root / "raw-v1.json"
        self.context = self.root / "context.json"
        self.intent = self.root / "plan-intent.json"
        self.template = self.root / "unsigned-template.json"
        self.trust = self.root / "identity-v2-trust.json"
        self.registry = self.root / "identity-v2-provider-registry.json"
        self.public_key = self.root / "public-key.raw"
        self.public_key_pem = self.root / "public-key.pem"
        self.private_key = self.root / "ephemeral-custody-key.pem"
        self.provider = self.root / "ephemeral-provider.py"
        self.verifier = self.root / "pinned-verifier-marker"
        self.governance_root = self.root / "governance-root.json"
        self.now = datetime.now(timezone.utc).replace(microsecond=0)
        self.capture_start = (self.now - timedelta(seconds=10)).isoformat().replace("+00:00", "Z")
        self.capture_end = (self.now + timedelta(minutes=10)).isoformat().replace("+00:00", "Z")
        self.issued_at = self.now.isoformat().replace("+00:00", "Z")
        self.expires_at = (self.now + timedelta(minutes=5)).isoformat().replace("+00:00", "Z")

        self._generate_ephemeral_ed25519_material()
        self._write_provider()
        self.governance_root.write_bytes(
            (ROOT / "scripts" / "fixtures" / "oasis7-governance-root.v1.json").read_bytes()
        )
        self.governance_root.chmod(0o600)
        self._write_context_and_intent()
        self._write_raw_v1()
        self._write_trust_and_registry()
        self._write_template()

        # RED guard: all behavioral tests must fail because the production
        # executable is missing, not because a fixture silently skipped.
        self.assertTrue(
            TOOL.is_file(),
            "RED: missing proposed executable scripts/p2p-public-testnet-identity-v2-signing-tool.py",
        )

    def tearDown(self) -> None:
        self.temp.cleanup()

    def _generate_ephemeral_ed25519_material(self) -> None:
        self.assertTrue(OPENSSL.is_file(), f"test prerequisite missing: {OPENSSL}")
        self._openssl("genpkey", "-algorithm", "ED25519", "-out", str(self.private_key))
        self._openssl("pkey", "-in", str(self.private_key), "-pubout", "-out", str(self.public_key_pem))
        der = self.root / "public-key.der"
        self._openssl("pkey", "-in", str(self.private_key), "-pubout", "-outform", "DER", "-out", str(der))
        der_bytes = der.read_bytes()
        self.assertGreaterEqual(len(der_bytes), 32)
        self.public_key.write_bytes(der_bytes[-32:])

    def _openssl(self, *args: str) -> None:
        result = subprocess.run([str(OPENSSL), *args], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)

    def _write_provider(self) -> None:
        self.provider.write_text(provider_source(self.private_key), encoding="utf-8")
        self.provider.chmod(self.provider.stat().st_mode | stat.S_IXUSR)
        self.verifier.write_text("test-only verifier marker\n", encoding="utf-8")
        self.verifier.chmod(self.verifier.stat().st_mode | stat.S_IXUSR)

    def _write_context_and_intent(self) -> None:
        context = {
            "schema_version": "oasis7.identity_v2_context.v1",
            "network_id": NETWORK_ID,
            "task_uid": TASK_UID,
            "head_oid": "e" * 40,
            "capture_window_id": "capture-window-identity-v2-001",
            "capture_start": self.capture_start,
            "capture_end": self.capture_end,
            "rotation_epoch": ROTATION_EPOCH,
            "issued_at": self.issued_at,
            "expires_at": self.expires_at,
        }
        write_json(self.context, context)
        context_digest = digest_file(self.context)
        intent = {
            "schema_version": "oasis7.clean_room_plan_intent.v1",
            "context_digest": context_digest,
            "adapter_action": "public-testnet-governed-rebuild",
            "nodes": [
                {
                    "node_name": NODE_ID,
                    "node_id": NODE_ID,
                    "peer_id": PEER_ID,
                    "role": "validator",
                    "reset_surface_ids": ["config", "execution", "world"],
                }
            ],
        }
        write_json(self.intent, intent)
        self.context_digest = context_digest
        self.plan_digest = digest_file(self.intent)

    def _write_raw_v1(self) -> None:
        # Deliberate noncanonical whitespace and key order prove that the raw
        # digest binds bytes, while admission still validates its parsed shape.
        raw = {
            "schema_version": "oasis7.identity_receipt.v1",
            "node_id": NODE_ID,
            "peer_id": PEER_ID,
            "key_path": "config/node-keypair.toml",
            "key_sha256": "1" * 64,
            "key_size_bytes": 321,
            "key_mode": 384,
            "key_uid": 0,
            "key_gid": 0,
        }
        self.raw.write_bytes((json.dumps(raw, ensure_ascii=True, indent=2) + "\n").encode("utf-8"))
        self.raw_digest = digest_file(self.raw)

    def _write_trust_and_registry(self) -> None:
        trust = {
            "schema_version": "oasis7.identity_v2_trust_config.v1",
            "network_id": NETWORK_ID,
            "trust_root_id": TRUST_ROOT_ID,
            "verifier_id": VERIFIER_ID,
            "algorithm": "ed25519",
            "rotation_epoch": ROTATION_EPOCH,
            "allowlist": [
                {
                    "signer_id": SIGNER_ID,
                    "public_key_ref": str(self.public_key),
                    "public_key_sha256": digest_file(self.public_key),
                    "status": "active",
                    "valid_from": self.capture_start,
                    "valid_until": self.expires_at,
                }
            ],
            "revocations": [],
        }
        write_json(self.trust, trust)
        registry = {
            "schema_version": "oasis7.identity_v2_provider_registry.v1",
            "trust_config_path": str(self.trust),
            "trust_config_sha256": digest_file(self.trust),
            "providers": [
                {
                    "provider_id": PROVIDER_ID,
                    "adapter_path": str(self.provider),
                    "adapter_sha256": digest_file(self.provider),
                    "signer_id": SIGNER_ID,
                    "public_key_ref": str(self.public_key),
                    "public_key_sha256": digest_file(self.public_key),
                    "algorithm": "ed25519",
                }
            ],
            "verifier": {
                "executable_path": str(self.verifier),
                "executable_sha256": digest_file(self.verifier),
            },
        }
        write_json(self.registry, registry)

    def _write_template(self) -> None:
        write_json(
            self.template,
            {
                "domain_separator": "oasis7.identity_receipt.v2/signature/v1",
                "schema_version": "oasis7.identity_receipt.v2",
                "signer_id": SIGNER_ID,
                "verifier_id": VERIFIER_ID,
                "trust_root_id": TRUST_ROOT_ID,
                "task_uid": TASK_UID,
                "head_oid": "e" * 40,
                "frozen_head_oid": "e" * 40,
                "plan_digest": self.plan_digest,
                "context_digest": self.context_digest,
                "capture_window_id": "capture-window-identity-v2-001",
                "rotation_epoch": ROTATION_EPOCH,
                "issued_at": self.issued_at,
                "expires_at": self.expires_at,
                "node_id": NODE_ID,
                "peer_id": PEER_ID,
                "key_sha256": "1" * 64,
                "key_size_bytes": 321,
                "key_mode": "0600",
                "key_uid": 0,
                "key_gid": 0,
                "signed_payload_sha256": self.raw_digest,
            },
        )

    def _env(self) -> dict[str, str]:
        # Do not expose the caller's credential-bearing environment to the
        # future tool or its provider.  The fixture uses only its own paths.
        return {"PATH": os.environ.get("PATH", ""), "PYTHONIOENCODING": "utf-8"}

    def _run(
        self,
        *args: str,
        authority_pins: tuple[str, str] | None = None,
    ) -> subprocess.CompletedProcess[bytes]:
        trusted_config_sha256, trusted_registry_sha256 = authority_pins or (
            digest_file(self.trust),
            digest_file(self.registry),
        )
        return subprocess.run(
            [
                sys.executable,
                "-c",
                CHILD_HARNESS,
                str(TOOL),
                str(self.governance_root),
                str(self.trust),
                str(self.registry),
                trusted_config_sha256,
                trusted_registry_sha256,
                *args,
            ],
            cwd=ROOT,
            env=self._env(),
            capture_output=True,
        )

    def _run_unpatched(self, *args: str) -> subprocess.CompletedProcess[bytes]:
        return subprocess.run(
            [sys.executable, str(TOOL), *args],
            cwd=ROOT,
            env=self._env(),
            capture_output=True,
        )

    def _prepare_args(self, payload: Path, manifest: Path) -> list[str]:
        return [
            "prepare",
            "--raw-v1",
            str(self.raw),
            "--template",
            str(self.template),
            "--context",
            str(self.context),
            "--plan-intent",
            str(self.intent),
            "--trust-config",
            str(self.trust),
            "--provider-registry",
            str(self.registry),
            "--payload-out",
            str(payload),
            "--manifest-out",
            str(manifest),
        ]

    def _prepare(self, stem: str = "flow") -> tuple[Path, Path]:
        payload = self.root / f"{stem}.payload.bin"
        manifest = self.root / f"{stem}.prepare.json"
        result = self._run(*self._prepare_args(payload, manifest))
        self._assert_success(result)
        return payload, manifest

    def _sign(self, payload: Path, manifest: Path, stem: str = "flow") -> tuple[Path, Path]:
        signature = self.root / f"{stem}.signature.hex"
        attestation = self.root / f"{stem}.attestation.json"
        result = self._run(
            "sign",
            "--payload",
            str(payload),
            "--manifest",
            str(manifest),
            "--provider-registry",
            str(self.registry),
            "--provider-ref",
            PROVIDER_ID,
            "--signature-out",
            str(signature),
            "--attestation-out",
            str(attestation),
        )
        self._assert_success(result)
        return signature, attestation

    def _assemble(
        self, payload: Path, manifest: Path, signature: Path, attestation: Path, stem: str = "flow"
    ) -> Path:
        envelope = self.root / f"{stem}.envelope.json"
        result = self._run(
            "assemble",
            "--payload",
            str(payload),
            "--manifest",
            str(manifest),
            "--signature",
            str(signature),
            "--attestation",
            str(attestation),
            "--provider-registry",
            str(self.registry),
            "--out",
            str(envelope),
        )
        self._assert_success(result)
        return envelope

    def _verify(self, envelope: Path, raw: Path | None = None, mode: str = "current_admission") -> tuple[Path, Path]:
        verified = self.root / f"{mode}.verified.json"
        receipt = self.root / f"{mode}.verification.json"
        result = self._run(
            "verify",
            "--mode",
            mode,
            "--envelope",
            str(envelope),
            "--raw-v1",
            str(raw or self.raw),
            "--context",
            str(self.context),
            "--plan-intent",
            str(self.intent),
            "--trust-config",
            str(self.trust),
            "--provider-registry",
            str(self.registry),
            "--out",
            str(verified),
            "--verification-out",
            str(receipt),
        )
        self._assert_success(result)
        return verified, receipt

    def _assert_success(self, result: subprocess.CompletedProcess[bytes]) -> None:
        self.assertEqual(
            result.returncode,
            0,
            "expected executable success; stderr=" + result.stderr.decode("utf-8", "replace"),
        )

    def _assert_rejected_no_output(self, result: subprocess.CompletedProcess[bytes], *outputs: Path) -> None:
        self.assertNotEqual(result.returncode, 0, "malformed/tampered input unexpectedly succeeded")
        for output in outputs:
            self.assertFalse(output.exists(), f"rejected operation created output: {output}")

    def _prepare_sign_assemble(self, stem: str = "flow") -> tuple[Path, Path, Path, Path, Path]:
        payload, manifest = self._prepare(stem)
        signature, attestation = self._sign(payload, manifest, stem)
        envelope = self._assemble(payload, manifest, signature, attestation, stem)
        return payload, manifest, signature, attestation, envelope

    def test_prepare_is_deterministic_and_binds_exact_raw_context_and_intent(self) -> None:
        payload_a, manifest_a = self._prepare("a")
        payload_b, manifest_b = self._prepare("b")
        self.assertEqual(payload_a.read_bytes(), payload_b.read_bytes())
        first = json.loads(manifest_a.read_text(encoding="utf-8"))
        second = json.loads(manifest_b.read_text(encoding="utf-8"))
        for field in (
            "canonical_payload_sha256",
            "raw_v1_sha256",
            "context_digest",
            "plan_digest",
            "task_uid",
            "head_oid",
            "node_id",
            "peer_id",
            "capture_window_id",
            "rotation_epoch",
            "trust_config_sha256",
            "provider_registry_sha256",
            "public_key_sha256",
            "verifier_executable_sha256",
        ):
            self.assertEqual(first[field], second[field], field)
        self.assertEqual(first["raw_v1_sha256"], self.raw_digest)
        self.assertEqual(first["context_digest"], self.context_digest)
        self.assertEqual(first["plan_digest"], self.plan_digest)

    def test_real_ed25519_sign_assemble_verify_preserves_one_immutable_payload(self) -> None:
        payload, manifest, signature, attestation, envelope = self._prepare_sign_assemble()
        payload_bytes = payload.read_bytes()
        signature_bytes = bytes.fromhex(signature.read_text(encoding="utf-8").strip())
        self.assertEqual(len(signature_bytes), 64)
        self._openssl_verify(payload, signature_bytes)
        verified, verification = self._verify(envelope)
        self.assertEqual(payload.read_bytes(), payload_bytes)
        self.assertEqual(json.loads(manifest.read_text())["canonical_payload_sha256"], digest_bytes(payload_bytes))
        self.assertEqual(json.loads(attestation.read_text())["canonical_payload_sha256"], digest_bytes(payload_bytes))
        output = json.loads(verified.read_text(encoding="utf-8"))
        receipt = json.loads(verification.read_text(encoding="utf-8"))
        self.assertTrue(output["authenticated"])
        self.assertTrue(output["verified"])
        self.assertTrue(receipt["apply_authorized"])
        self.assertEqual(receipt["canonical_payload_sha256"], digest_bytes(payload_bytes))

    def _openssl_verify(self, payload: Path, signature_bytes: bytes) -> None:
        signature = self.root / "independent-check.signature"
        signature.write_bytes(signature_bytes)
        result = subprocess.run(
            [
                str(OPENSSL),
                "pkeyutl",
                "-verify",
                "-pubin",
                "-inkey",
                str(self.public_key_pem),
                "-rawin",
                "-in",
                str(payload),
                "-sigfile",
                str(signature),
            ],
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_assemble_requires_explicit_unchanged_payload(self) -> None:
        payload, manifest, signature, attestation, _ = self._prepare_sign_assemble()
        envelope = self.root / "tampered-payload.envelope.json"
        original = payload.read_bytes()
        payload.write_bytes(original[:-1] + bytes([original[-1] ^ 1]))
        result = self._run(
            "assemble",
            "--payload",
            str(payload),
            "--manifest",
            str(manifest),
            "--signature",
            str(signature),
            "--attestation",
            str(attestation),
            "--provider-registry",
            str(self.registry),
            "--out",
            str(envelope),
        )
        self._assert_rejected_no_output(result, envelope)

    def test_current_admission_rejects_retired_key_but_historical_is_non_authorizing(self) -> None:
        _, _, _, _, envelope = self._prepare_sign_assemble("retired")
        trust = json.loads(self.trust.read_text(encoding="utf-8"))
        trust["allowlist"][0]["status"] = "retired"
        write_json(self.trust, trust)
        registry = json.loads(self.registry.read_text(encoding="utf-8"))
        registry["trust_config_sha256"] = digest_file(self.trust)
        write_json(self.registry, registry)
        current_verified = self.root / "retired-current.verified.json"
        current_receipt = self.root / "retired-current.verification.json"
        current = self._run(
            "verify",
            "--mode",
            "current_admission",
            "--envelope",
            str(envelope),
            "--raw-v1",
            str(self.raw),
            "--context",
            str(self.context),
            "--plan-intent",
            str(self.intent),
            "--trust-config",
            str(self.trust),
            "--provider-registry",
            str(self.registry),
            "--out",
            str(current_verified),
            "--verification-out",
            str(current_receipt),
        )
        self._assert_rejected_no_output(current, current_verified, current_receipt)
        historical_verified, historical_receipt = self._verify(envelope, mode="historical_audit")
        historical = json.loads(historical_verified.read_text(encoding="utf-8"))
        receipt = json.loads(historical_receipt.read_text(encoding="utf-8"))
        self.assertTrue(historical["historical_only"])
        self.assertFalse(historical["apply_authorized"])
        self.assertTrue(receipt["historical_only"])
        self.assertFalse(receipt["apply_authorized"])

    def test_current_admission_rejects_revoked_key_but_historical_receipt_cannot_apply(self) -> None:
        _, _, _, _, envelope = self._prepare_sign_assemble("revoked")
        trust = json.loads(self.trust.read_text(encoding="utf-8"))
        trust["allowlist"][0]["status"] = "revoked"
        trust["revocations"] = [
            {
                "signer_id": SIGNER_ID,
                "effective_at": self.expires_at,
                "reason": "test-only rotation",
            }
        ]
        write_json(self.trust, trust)
        registry = json.loads(self.registry.read_text(encoding="utf-8"))
        registry["trust_config_sha256"] = digest_file(self.trust)
        write_json(self.registry, registry)
        current_verified = self.root / "revoked-current.verified.json"
        current_receipt = self.root / "revoked-current.verification.json"
        current = self._run(
            "verify",
            "--mode",
            "current_admission",
            "--envelope",
            str(envelope),
            "--raw-v1",
            str(self.raw),
            "--context",
            str(self.context),
            "--plan-intent",
            str(self.intent),
            "--trust-config",
            str(self.trust),
            "--provider-registry",
            str(self.registry),
            "--out",
            str(current_verified),
            "--verification-out",
            str(current_receipt),
        )
        self._assert_rejected_no_output(current, current_verified, current_receipt)
        historical_verified, historical_receipt = self._verify(envelope, mode="historical_audit")
        historical = json.loads(historical_verified.read_text(encoding="utf-8"))
        receipt = json.loads(historical_receipt.read_text(encoding="utf-8"))
        self.assertTrue(historical["historical_only"])
        self.assertFalse(historical["apply_authorized"])
        self.assertTrue(receipt["historical_only"])
        self.assertFalse(receipt["apply_authorized"])

    def test_provider_registry_and_executable_public_key_digests_are_pinned(self) -> None:
        payload, manifest, _, _, envelope = self._prepare_sign_assemble("pins")
        original = self.provider.read_bytes()
        self.provider.write_bytes(original + b"\n# tampered adapter\n")
        signature = self.root / "pins.signature.hex"
        attestation = self.root / "pins.attestation.json"
        result = self._run(
            "sign",
            "--payload",
            str(payload),
            "--manifest",
            str(manifest),
            "--provider-registry",
            str(self.registry),
            "--provider-ref",
            PROVIDER_ID,
            "--signature-out",
            str(signature),
            "--attestation-out",
            str(attestation),
        )
        self._assert_rejected_no_output(result, signature, attestation)

        self.provider.write_bytes(original)
        self.verifier.write_bytes(self.verifier.read_bytes() + b"tamper\n")
        verified = self.root / "verifier-pin.verified.json"
        receipt = self.root / "verifier-pin.verification.json"
        result = self._run(
            "verify",
            "--mode",
            "current_admission",
            "--envelope",
            str(envelope),
            "--raw-v1",
            str(self.raw),
            "--context",
            str(self.context),
            "--plan-intent",
            str(self.intent),
            "--trust-config",
            str(self.trust),
            "--provider-registry",
            str(self.registry),
            "--out",
            str(verified),
            "--verification-out",
            str(receipt),
        )
        self._assert_rejected_no_output(result, verified, receipt)

    def test_no_caller_selected_provider_command_or_endpoint(self) -> None:
        payload, manifest = self._prepare("provider-selection")
        signature = self.root / "rogue.signature.hex"
        attestation = self.root / "rogue.attestation.json"
        result = self._run(
            "sign",
            "--payload",
            str(payload),
            "--manifest",
            str(manifest),
            "--provider-registry",
            str(self.registry),
            "--provider-ref",
            PROVIDER_ID,
            "--provider-command",
            "/tmp/rogue-provider",
            "--provider-endpoint",
            "https://rogue.invalid/sign",
            "--signature-out",
            str(signature),
            "--attestation-out",
            str(attestation),
        )
        self._assert_rejected_no_output(result, signature, attestation)

        unknown_signature = self.root / "unknown-provider.signature.hex"
        unknown_attestation = self.root / "unknown-provider.attestation.json"
        result = self._run(
            "sign",
            "--payload",
            str(payload),
            "--manifest",
            str(manifest),
            "--provider-registry",
            str(self.registry),
            "--provider-ref",
            "/tmp/arbitrary-provider",
            "--signature-out",
            str(unknown_signature),
            "--attestation-out",
            str(unknown_attestation),
        )
        self._assert_rejected_no_output(result, unknown_signature, unknown_attestation)

    def test_template_verified_flags_and_derived_fields_have_no_authority(self) -> None:
        template = json.loads(self.template.read_text(encoding="utf-8"))
        template.update(
            {
                "authenticated": True,
                "verified": True,
                "signature_hex": "a" * 128,
                "canonical_digest": "b" * 64,
            }
        )
        write_json(self.template, template)
        payload = self.root / "template-flags.payload.bin"
        manifest = self.root / "template-flags.prepare.json"
        result = self._run(*self._prepare_args(payload, manifest))
        self._assert_rejected_no_output(result, payload, manifest)

    def test_duplicate_unknown_and_malformed_json_reject_before_any_output(self) -> None:
        cases = {
            "duplicate": b'{"schema_version":"oasis7.identity_v2_context.v1","schema_version":"bad"}',
            "unknown": canonical({"schema_version": "oasis7.identity_v2_context.v1", "unexpected": True}),
            "malformed": b"not-json",
        }
        for name, data in cases.items():
            with self.subTest(case=name):
                context = self.root / f"{name}.context.json"
                context.write_bytes(data)
                payload = self.root / f"{name}.payload.bin"
                manifest = self.root / f"{name}.prepare.json"
                args = self._prepare_args(payload, manifest)
                args[args.index("--context") + 1] = str(context)
                result = self._run(*args)
                self._assert_rejected_no_output(result, payload, manifest)

    def test_wrong_domain_and_cross_pair_raw_context_intent_reject_without_output(self) -> None:
        template = json.loads(self.template.read_text(encoding="utf-8"))
        template["domain_separator"] = "wrong-domain"
        write_json(self.template, template)
        wrong_domain_payload = self.root / "wrong-domain.payload.bin"
        wrong_domain_manifest = self.root / "wrong-domain.prepare.json"
        result = self._run(*self._prepare_args(wrong_domain_payload, wrong_domain_manifest))
        self._assert_rejected_no_output(result, wrong_domain_payload, wrong_domain_manifest)

        # Restore the template, produce a real flow, then pair it with a raw-v1
        # capture belonging to another peer.  The verifier must hash and reject
        # the bytes before it can emit a receipt.
        self._write_template()
        _, _, _, _, envelope = self._prepare_sign_assemble("cross-pair")
        other_raw = self.root / "other-node.raw-v1.json"
        other_raw.write_bytes(
            self.raw.read_bytes().replace(PEER_ID.encode("utf-8"), b"12D3KooWOtherPeer")
        )
        verified = self.root / "cross-pair.verified.json"
        receipt = self.root / "cross-pair.verification.json"
        result = self._run(
            "verify",
            "--mode",
            "current_admission",
            "--envelope",
            str(envelope),
            "--raw-v1",
            str(other_raw),
            "--context",
            str(self.context),
            "--plan-intent",
            str(self.intent),
            "--trust-config",
            str(self.trust),
            "--provider-registry",
            str(self.registry),
            "--out",
            str(verified),
            "--verification-out",
            str(receipt),
        )
        self._assert_rejected_no_output(result, verified, receipt)

    def test_signature_payload_key_and_envelope_tamper_reject_without_output(self) -> None:
        payload, manifest, signature, attestation, envelope = self._prepare_sign_assemble("tamper")
        original = signature.read_text(encoding="utf-8").strip()
        signature.write_text(("0" if original[0] != "0" else "1") + original[1:] + "\n", encoding="utf-8")
        assembled = self.root / "tampered-signature.envelope.json"
        result = self._run(
            "assemble",
            "--payload",
            str(payload),
            "--manifest",
            str(manifest),
            "--signature",
            str(signature),
            "--attestation",
            str(attestation),
            "--provider-registry",
            str(self.registry),
            "--out",
            str(assembled),
        )
        self._assert_rejected_no_output(result, assembled)

        tampered_envelope = self.root / "tampered-envelope.json"
        envelope_value = json.loads(envelope.read_text(encoding="utf-8"))
        envelope_value["node_id"] = "triad-testnet-storage"
        write_json(tampered_envelope, envelope_value)
        verified = self.root / "tampered-envelope.verified.json"
        receipt = self.root / "tampered-envelope.verification.json"
        result = self._run(
            "verify",
            "--mode",
            "current_admission",
            "--envelope",
            str(tampered_envelope),
            "--raw-v1",
            str(self.raw),
            "--context",
            str(self.context),
            "--plan-intent",
            str(self.intent),
            "--trust-config",
            str(self.trust),
            "--provider-registry",
            str(self.registry),
            "--out",
            str(verified),
            "--verification-out",
            str(receipt),
        )
        self._assert_rejected_no_output(result, verified, receipt)

    def test_wrong_present_governance_root_rejects_before_provider_command(self) -> None:
        """A present root with altered content cannot reach custody signing."""
        payload, manifest = self._prepare("root-hardening")
        original = self.governance_root.read_bytes()
        marker = b'"root_id": "oasis7-public-testnet-governance-root-v1"'
        self.assertIn(marker, original)
        self.governance_root.write_bytes(
            original.replace(marker, b'"root_id": "wrong-test-root"', 1)
        )
        self.governance_root.chmod(0o600)
        signature = self.root / "root-hardening.signature.hex"
        attestation = self.root / "root-hardening.attestation.json"
        result = self._run(
            "sign",
            "--payload",
            str(payload),
            "--manifest",
            str(manifest),
            "--provider-registry",
            str(self.registry),
            "--provider-ref",
            PROVIDER_ID,
            "--signature-out",
            str(signature),
            "--attestation-out",
            str(attestation),
        )
        self._assert_rejected_no_output(result, signature, attestation)

    def test_signer_validity_interval_must_cover_issuance_in_both_modes(self) -> None:
        """A signer becoming valid after issuance cannot authorize either mode."""
        _, _, _, _, envelope = self._prepare_sign_assemble("validity-at-issuance")
        trust = json.loads(self.trust.read_text(encoding="utf-8"))
        trust["allowlist"][0]["valid_from"] = (
            self.now + timedelta(seconds=1)
        ).isoformat().replace("+00:00", "Z")
        trust["allowlist"][0]["valid_until"] = (
            self.now + timedelta(minutes=9)
        ).isoformat().replace("+00:00", "Z")
        write_json(self.trust, trust)
        registry = json.loads(self.registry.read_text(encoding="utf-8"))
        registry["trust_config_sha256"] = digest_file(self.trust)
        write_json(self.registry, registry)
        for mode in ("current_admission", "historical_audit"):
            with self.subTest(mode=mode):
                verified = self.root / f"validity-{mode}.verified.json"
                receipt = self.root / f"validity-{mode}.verification.json"
                result = self._run(
                    "verify",
                    "--mode",
                    mode,
                    "--envelope",
                    str(envelope),
                    "--raw-v1",
                    str(self.raw),
                    "--context",
                    str(self.context),
                    "--plan-intent",
                    str(self.intent),
                    "--trust-config",
                    str(self.trust),
                    "--provider-registry",
                    str(self.registry),
                    "--out",
                    str(verified),
                    "--verification-out",
                    str(receipt),
                )
                self._assert_rejected_no_output(result, verified, receipt)

    def test_copied_registry_path_is_not_an_independent_deployment_anchor(self) -> None:
        """A caller-copied registry cannot become the deployment registry."""
        copied_registry = self.root / "copied-provider-registry.json"
        copied_registry.write_bytes(self.registry.read_bytes())
        payload = self.root / "copied-registry.payload.bin"
        manifest = self.root / "copied-registry.prepare.json"
        args = self._prepare_args(payload, manifest)
        args[args.index("--provider-registry") + 1] = str(copied_registry)
        result = self._run(*args)
        self._assert_rejected_no_output(result, payload, manifest)

    def test_mutated_trust_config_is_not_accepted_by_caller_updated_registry_digest(self) -> None:
        """The registry's consistency digest cannot replace an independent config pin."""
        trusted_config_sha256 = digest_file(self.trust)
        trusted_registry_sha256 = digest_file(self.registry)
        trust = json.loads(self.trust.read_text(encoding="utf-8"))
        trust["network_id"] = "caller-mutated-network"
        write_json(self.trust, trust)
        registry = json.loads(self.registry.read_text(encoding="utf-8"))
        registry["trust_config_sha256"] = digest_file(self.trust)
        write_json(self.registry, registry)
        payload = self.root / "config-anchor.payload.bin"
        manifest = self.root / "config-anchor.prepare.json"
        result = self._run(
            *self._prepare_args(payload, manifest),
            authority_pins=(trusted_config_sha256, trusted_registry_sha256),
        )
        self._assert_rejected_no_output(result, payload, manifest)

    def test_unpatched_cli_rejects_temporary_authority_without_outputs(self) -> None:
        """The real CLI has no test-only deployment-authority bypass."""
        payload = self.root / "unpatched.payload.bin"
        manifest = self.root / "unpatched.prepare.json"
        result = self._run_unpatched(*self._prepare_args(payload, manifest))
        self._assert_rejected_no_output(result, payload, manifest)


if __name__ == "__main__":
    unittest.main()
