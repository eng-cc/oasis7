#!/usr/bin/env python3
"""RED process contracts for the identity-v2 sidecar/planner/adapter bridge.

The production CLIs currently expose only the legacy callback/in-process seams.
These tests specify the smallest executable bridge: the sidecar must invoke the
four real signing-tool commands, the planner must ingest an explicit evidence
map while retaining the full envelope, and the adapter must admit only a
current, correctly paired map before any mutation.  The fixture provider is
ephemeral and test-local; no production offline flag, credential, node, or
network is involved.
"""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
SIDECAR = ROOT / "scripts" / "p2p-public-testnet-identity-receipt-v2.py"
PLANNER = ROOT / "scripts" / "p2p-public-testnet-full-network-clean-room.py"
ADAPTER = ROOT / "scripts" / "p2p-public-testnet-full-network-clean-room-adapter.py"
TOOL = ROOT / "scripts" / "p2p-public-testnet-identity-v2-signing-tool.py"
TOOL_TEST = ROOT / "scripts" / "p2p-public-testnet-identity-v2-signing-tool.test.py"
PLANNER_TEST = ROOT / "scripts" / "p2p-public-testnet-full-network-clean-room.test.py"
ADAPTER_TEST = ROOT / "scripts" / "p2p-public-testnet-full-network-clean-room-adapter.test.py"


def load_module(name: str, path: Path) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise AssertionError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def canonical(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode("utf-8")


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.write_bytes(canonical(value))


def descriptor(path: Path) -> dict[str, Any]:
    payload = path.read_bytes()
    return {"path": str(path), "sha256": digest_bytes(payload), "size_bytes": len(payload)}


V2_ARTIFACT_FIELDS = (
    "raw_v1",
    "prepare_manifest",
    "payload",
    "provider_attestation",
    "unsigned_envelope",
    "signed_envelope",
    "verification",
)


def clean_env() -> dict[str, str]:
    return {"PATH": os.environ.get("PATH", ""), "PYTHONIOENCODING": "utf-8"}


class SidecarVerifierBoundaryTests(unittest.TestCase):
    """Isolated execution-boundary tests; child outputs are synthetic, not custody."""

    def test_verifier_replacement_never_promotes_evidence(self):
        for phase in ("before", "after"):
            for replacement in ("inode", "digest", "unsafe-mode"):
                with self.subTest(phase=phase, replacement=replacement), tempfile.TemporaryDirectory() as directory:
                    root = Path(directory).resolve()
                    module = load_module("sidecar_boundary", SIDECAR)
                    signer, verifier = root / "signer", root / "verifier"
                    for path in (signer, verifier):
                        path.write_bytes(b"fixture executable")
                        path.chmod(0o700)
                    module.IDENTITY_V2_SIGNER_TOOL_PATH = signer
                    module.IDENTITY_V2_SIGNER_TOOL_SHA256 = digest_bytes(signer.read_bytes())
                    inputs = {}
                    for key in ("raw_v1", "template", "context", "plan_intent", "trust_config", "provider_registry"):
                        inputs[key] = root / key
                        inputs[key].write_bytes(b"{}")
                    write_json(inputs["provider_registry"], {"verifier": {
                        "executable_path": str(verifier), "executable_sha256": digest_bytes(verifier.read_bytes())}})
                    args = SimpleNamespace(**inputs, signer_tool=signer, verifier_tool=verifier,
                        provider_ref="fixture", out=root / "output", evidence_map_out=root / "map", evidence_dir=root / "retained")
                    args.out.write_bytes(b"prior envelope")
                    args.evidence_map_out.write_bytes(b"prior map")
                    calls = []
                    def replace():
                        if replacement == "inode":
                            other = root / "replacement"
                            other.write_bytes(verifier.read_bytes())
                            other.chmod(0o700)
                            os.replace(other, verifier)
                        elif replacement == "unsafe-mode":
                            verifier.chmod(0o777)
                        else:
                            verifier.write_bytes(b"changed executable")
                    def child(argv, **kwargs):
                        command = argv[1]
                        calls.append(command)
                        if command == "verify":
                            final = {"authenticated": True, "verified": True,
                                "network_id": module.CANONICAL_NETWORK_ID, "node_id": "triad-testnet-storage"}
                            Path(argv[argv.index("--out") + 1]).write_bytes(canonical(final))
                            Path(argv[argv.index("--verification-out") + 1]).write_bytes(b"{}")
                            if phase == "after":
                                replace()
                        else:
                            for flag in ("--payload-out", "--manifest-out", "--signature-out", "--attestation-out", "--out"):
                                if flag in argv:
                                    Path(argv[argv.index(flag) + 1]).write_bytes(b"{}")
                            if command == "assemble" and phase == "before":
                                replace()
                        return SimpleNamespace(returncode=0, stderr="")
                    with patch.object(module.subprocess, "run", side_effect=child):
                        with self.assertRaises(SystemExit):
                            module._bridge_create(args)
                    self.assertEqual(args.out.read_bytes(), b"prior envelope")
                    self.assertEqual(args.evidence_map_out.read_bytes(), b"prior map")
                    self.assertEqual(list(args.evidence_dir.iterdir()), [])
                    self.assertEqual(calls.count("verify"), 0 if phase == "before" else 1)


class IdentityV2CliBridgeTests(unittest.TestCase):
    """Process-level RED tests for the proposed CLI contract."""

    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="oasis7-identity-v2-bridge-")
        self.root = Path(self.temp.name)

        # Reuse the approved S2 fixture harness only to make real, independently
        # verified Ed25519 artifacts.  It patches authority constants in child
        # processes, never in production and never through an offline bypass.
        signing_module = load_module("identity_v2_signing_tool_contract", TOOL_TEST)
        planner_module = load_module("full_network_clean_room_contract", PLANNER_TEST)
        self.signing = signing_module.IdentityV2SigningToolContractTests("runTest")
        self.signing.setUp()
        self.planner = planner_module.FullNetworkCleanRoomPlanTests("runTest")
        self.planner.setUp()
        shutil.copyfile(self.planner.module._peer_registry_authority().REGISTRY_PATH, self.signing.peer_registry)
        self.signing.peer_registry.chmod(0o600)
        self.request = self.planner._input()

        self._align_signing_context()
        self.artifacts = self._make_signed_artifacts()
        self.evidence_map = self._write_evidence_map()
        self.input_path = self.root / "clean-room-input.json"
        write_json(self.input_path, self.request)
        self.authority_path = self.root / "authority.json"

    def tearDown(self) -> None:
        self.planner.tearDown()
        self.signing.tearDown()
        self.temp.cleanup()

    def _align_signing_context(self) -> None:
        """Bind the fixture context to the planner's task/head/window."""
        context = json.loads(self.signing.context.read_text(encoding="utf-8"))
        context["capture_window_id"] = self.request["capture_window_id"]
        write_json(self.signing.context, context)
        self.signing.context_digest = digest_bytes(canonical(context))

        intent_nodes = []
        for node in sorted(self.request["nodes"], key=lambda item: str(item["name"])):
            name = str(node["name"])
            intent_nodes.append(
                {
                    "node_name": name,
                    "node_id": str(node["node_id"]),
                    "peer_id": self.planner.module.CANONICAL_PEER_REGISTRY[name],
                    "role": str(node["role"]),
                    "reset_surface_ids": ["config", "execution", "world"],
                }
            )
        write_json(
            self.signing.intent,
            {
                "schema_version": "oasis7.clean_room_plan_intent.v2",
                "peer_registry_sha256": digest_bytes(self.signing.peer_registry.read_bytes()),
                "peer_registry_epoch": json.loads(self.signing.peer_registry.read_text())["registry_epoch"],
                "context_digest": self.signing.context_digest,
                "adapter_action": "public-testnet-governed-rebuild",
                "nodes": intent_nodes,
            },
        )
        self.signing.plan_digest = digest_bytes(self.signing.intent.read_bytes())

    def _write_node_raw_and_template(self, node: dict[str, Any]) -> None:
        name = str(node["name"])
        node_id = str(node["node_id"])
        peer_id = self.planner.module.CANONICAL_PEER_REGISTRY[name]
        raw = {
            "schema_version": "oasis7.identity_receipt.v1",
            "node_id": node_id,
            "peer_id": peer_id,
            "key_path": f"config/{node_id}-node-keypair.toml",
            "key_sha256": "7" * 64,
            "key_size_bytes": 128,
            "key_mode": 0o600,
            "key_uid": 0,
            "key_gid": 0,
        }
        # Preserve the deliberately non-canonical raw capture: its exact bytes
        # are hashed and carried in the evidence map, not silently normalized.
        self.signing.raw.write_bytes((json.dumps(raw, indent=2) + "\n").encode("utf-8"))
        self.signing.raw_digest = digest_bytes(self.signing.raw.read_bytes())

        context = json.loads(self.signing.context.read_text(encoding="utf-8"))
        template = {
            "domain_separator": "oasis7.identity_receipt.v2/signature/v1",
            "schema_version": "oasis7.identity_receipt.v2",
            "signer_id": "identity-v2-ephemeral-test-signer",
            "verifier_id": "governed-receipt-verifier",
            "trust_root_id": "oasis7-public-testnet-governance-root-v1",
            "network_id": self.planner.module.CANONICAL_NETWORK_ID,
            "task_uid": context["task_uid"],
            "head_oid": context["head_oid"],
            "frozen_head_oid": context["head_oid"],
            "plan_digest": self.signing.plan_digest,
            "context_digest": self.signing.context_digest,
            "capture_window_id": context["capture_window_id"],
            "rotation_epoch": context["rotation_epoch"],
            "issued_at": context["issued_at"],
            "expires_at": context["expires_at"],
            "node_id": node_id,
            "peer_id": peer_id,
            "key_sha256": raw["key_sha256"],
            "key_size_bytes": raw["key_size_bytes"],
            "key_mode": "0600",
            "key_uid": raw["key_uid"],
            "key_gid": raw["key_gid"],
            "signed_payload_sha256": self.signing.raw_digest,
        }
        write_json(self.signing.template, template)

    def _make_signed_artifacts(self) -> dict[str, dict[str, Path]]:
        artifacts: dict[str, dict[str, Path]] = {}
        for node in self.request["nodes"]:
            name = str(node["name"])
            self._write_node_raw_and_template(node)
            stem = "bridge-" + name
            payload, manifest, _, attestation, envelope = self.signing._prepare_sign_assemble(stem)
            verified, verification = self.signing._verify(envelope)
            # The S2 fixture reuses its raw and verification output paths for
            # each case. Preserve node-specific bytes before the next case can
            # overwrite those paths; context and plan-intent remain immutable
            # shared inputs for the whole managed-node capture.
            raw_v1 = self.root / f"{name}.identity-receipt.v1.raw"
            template = self.root / f"{name}.identity-v2.unsigned-template.json"
            prepare_manifest = self.root / f"{name}.identity-v2.prepare.json"
            payload_copy = self.root / f"{name}.identity-v2.payload.bin"
            provider_attestation = self.root / f"{name}.identity-v2.provider-attestation.json"
            unsigned_envelope = self.root / f"{name}.identity-v2.unsigned.json"
            signed_envelope = self.root / f"{name}.identity-v2.signed.json"
            verified_envelope = self.root / f"{name}.identity-v2.verified.json"
            verification_receipt = self.root / f"{name}.identity-v2.verification.json"
            shutil.copyfile(self.signing.raw, raw_v1)
            shutil.copyfile(self.signing.template, template)
            shutil.copyfile(manifest, prepare_manifest)
            shutil.copyfile(payload, payload_copy)
            shutil.copyfile(attestation, provider_attestation)
            shutil.copyfile(envelope, unsigned_envelope)
            shutil.copyfile(verified, signed_envelope)
            shutil.copyfile(verified, verified_envelope)
            shutil.copyfile(verification, verification_receipt)
            artifacts[name] = {
                "raw_v1": raw_v1,
                "template": template,
                "prepare_manifest": prepare_manifest,
                "payload": payload_copy,
                "provider_attestation": provider_attestation,
                "unsigned_envelope": unsigned_envelope,
                "signed_envelope": signed_envelope,
                "verified_envelope": verified_envelope,
                "verification": verification_receipt,
            }
        return artifacts

    def _write_evidence_map(self) -> Path:
        return self._write_v2_evidence_map_fixture()

    def _forged_evidence_map(self) -> tuple[Path, dict[str, Any]]:
        """Forge one signature while recomputing every recorded byte digest."""
        value = json.loads(self.evidence_map.read_text(encoding="utf-8"))
        target = next(item for item in value["entries"] if item["node_name"] == "storage-205")
        original_envelope = Path(target["signed_envelope"]["path"])
        envelope = json.loads(original_envelope.read_text(encoding="utf-8"))
        envelope["signature_hex"] = "c" * 128
        signed_fields = {
            field: envelope[field]
            for field in self.planner.module.IDENTITY_V2_SIGNED_FIELDS
        }
        envelope["canonical_digest"] = digest_bytes(
            canonical({**signed_fields, "signature_hex": envelope["signature_hex"]})
        )

        forged_dir = self.root / "forged-identity-v2"
        forged_dir.mkdir(mode=0o700)
        forged_envelope = forged_dir / "storage-205.envelope.json"
        write_json(forged_envelope, envelope)
        forged_envelope.chmod(0o600)

        original_verification = Path(target["verification"]["path"])
        verification = json.loads(original_verification.read_text(encoding="utf-8"))
        verification["envelope_sha256"] = digest_bytes(forged_envelope.read_bytes())
        forged_verification = forged_dir / "storage-205.verification.json"
        write_json(forged_verification, verification)
        forged_verification.chmod(0o600)

        target["signed_envelope"] = descriptor(forged_envelope)
        target["verification"] = descriptor(forged_verification)
        forged_map = self.root / "forged-identity-v2-evidence-map.json"
        write_json(forged_map, value)
        return forged_map, value

    def _write_v2_evidence_map_fixture(self) -> Path:
        """Build a private, complete v2 retention map from real fixture artifacts."""
        existing = sorted(self.root.glob("retained-identity-v2-*"))
        retention = self.root / f"retained-identity-v2-{len(existing)}"
        retention.mkdir(mode=0o700)
        context = retention / "context.json"
        intent = retention / "plan-intent.json"
        shutil.copyfile(self.signing.context, context)
        shutil.copyfile(self.signing.intent, intent)
        context.chmod(0o600)
        intent.chmod(0o600)
        evidence: dict[str, Any] = {
            "schema_version": "oasis7.identity_v2_evidence_map.v2",
            "network_id": self.planner.module.CANONICAL_NETWORK_ID,
            "task_uid": self.request["task_uid"],
            "head_oid": self.request["head_oid"],
            "context": descriptor(context),
            "plan_intent": descriptor(intent),
            "entries": [],
        }
        for node in self.request["nodes"]:
            name = str(node["name"])
            source = self.artifacts[name]
            entry: dict[str, Any] = {
                "node_name": name,
                "node_id": str(node["node_id"]),
                "peer_id": self.planner.module.CANONICAL_PEER_REGISTRY[name],
            }
            for field in V2_ARTIFACT_FIELDS:
                retained = retention / f"{name}.{field}"
                shutil.copyfile(source[field], retained)
                retained.chmod(0o600)
                entry[field] = descriptor(retained)
            evidence["entries"].append(entry)
        path = retention / "identity-v2-evidence-map.json"
        write_json(path, evidence)
        path.chmod(0o600)
        return path

    def _tool_wrapper(self, marker: Path | None = None) -> Path:
        log = self.root / "signing-tool-commands.log"
        wrapper_name = "identity-v2-signing-tool-wrapper.py" if marker is None else f"untrusted-{marker.stem}.py"
        wrapper = self.root / wrapper_name
        harness = load_module("identity_v2_signing_tool_contract_for_wrapper", TOOL_TEST).CHILD_HARNESS
        peer_setup = (f"tool._peer_registry_authority().REGISTRY_PATH = Path({str(self.signing.peer_registry)!r})\n"
                      f"tool._peer_registry_authority().REGISTRY_SHA256 = {digest_bytes(self.signing.peer_registry.read_bytes())!r}\n")
        harness = harness.replace("raise SystemExit(tool.main", peer_setup + "raise SystemExit(tool.main")
        marker_statement = (
            f"Path({str(marker)!r}).write_text('invoked\\n', encoding='utf-8')"
            if marker is not None
            else ""
        )
        source = f'''#!/usr/bin/env python3
import subprocess, sys
from pathlib import Path

TOOL = {str(TOOL)!r}
ROOT = {str(self.signing.governance_root)!r}
TRUST = {str(self.signing.trust)!r}
LOG = Path({str(log)!r})
HARNESS = {harness!r}
REGISTRY = {str(self.signing.registry)!r}
TRUST_DIGEST = {digest_bytes(self.signing.trust.read_bytes())!r}
REGISTRY_DIGEST = {digest_bytes(self.signing.registry.read_bytes())!r}
{marker_statement}
if not sys.argv[1:]:
    raise SystemExit("missing signing-tool command")
LOG.open("a", encoding="utf-8").write(sys.argv[1] + "\\n")
result = subprocess.run(
    [sys.executable, "-c", HARNESS, TOOL, ROOT, TRUST, REGISTRY,
     TRUST_DIGEST, REGISTRY_DIGEST, *sys.argv[1:]],
    cwd={str(ROOT)!r},
    env={{"PATH": __import__("os").environ.get("PATH", ""), "PYTHONIOENCODING": "utf-8"}},
)
raise SystemExit(result.returncode)
'''
        wrapper.write_text(source, encoding="utf-8")
        wrapper.chmod(wrapper.stat().st_mode | stat.S_IXUSR)
        self.command_log = log
        return wrapper

    def _run(self, script: Path, *args: str) -> subprocess.CompletedProcess[str]:
        if script == SIDECAR:
            script = self._sidecar_harness()
        elif script == PLANNER:
            script = self._planner_harness()
        elif script == ADAPTER:
            script = self._adapter_harness()
        return subprocess.run(
            [sys.executable, str(script), *args],
            cwd=ROOT,
            env=clean_env(),
            capture_output=True,
            text=True,
        )

    def _sidecar_harness(self) -> Path:
        """Run the real sidecar with isolated, test-only tool pins."""
        trusted_wrapper = self._tool_wrapper()
        harness = self.root / "identity-v2-sidecar-harness.py"
        trusted_digest = digest_bytes(trusted_wrapper.read_bytes())
        source = f'''#!/usr/bin/env python3
import importlib.util
import sys
from pathlib import Path

def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise SystemExit("cannot load sidecar test harness module")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module

sidecar = load("identity_v2_bridge_sidecar", {str(SIDECAR)!r})
sidecar.IDENTITY_V2_SIGNER_TOOL_PATH = Path({str(trusted_wrapper)!r})
sidecar.IDENTITY_V2_SIGNER_TOOL_SHA256 = {trusted_digest!r}
sidecar.IDENTITY_V2_VERIFIER_TOOL_PATH = Path({str(self.signing.verifier)!r})
sidecar.IDENTITY_V2_VERIFIER_TOOL_SHA256 = {digest_bytes(self.signing.verifier.read_bytes())!r}
raise SystemExit(sidecar.main())
'''
        harness.write_text(source, encoding="utf-8")
        harness.chmod(harness.stat().st_mode | stat.S_IXUSR)
        return harness

    def _planner_harness(self) -> Path:
        """Run the real planner with isolated, test-only admission anchors."""
        verifier_tool = self._tool_wrapper()
        harness = self.root / "identity-v2-planner-harness.py"
        source = f'''#!/usr/bin/env python3
import importlib.util
import sys
from pathlib import Path

def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise SystemExit("cannot load planner test harness module")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module

planner = load("identity_v2_bridge_planner", {str(PLANNER)!r})
planner._peer_registry_authority().REGISTRY_PATH = Path({str(self.signing.peer_registry)!r})
planner._peer_registry_authority().REGISTRY_SHA256 = {digest_bytes(self.signing.peer_registry.read_bytes())!r}
planner.IDENTITY_V2_VERIFY_TOOL_PATH = Path({str(verifier_tool)!r})
planner.IDENTITY_V2_VERIFY_TOOL_SHA256 = {digest_bytes(verifier_tool.read_bytes())!r}
planner.IDENTITY_V2_TRUST_CONFIG_PATH = Path({str(self.signing.trust)!r})
planner.IDENTITY_V2_TRUST_CONFIG_SHA256 = {digest_bytes(self.signing.trust.read_bytes())!r}
planner.IDENTITY_V2_PROVIDER_REGISTRY_PATH = Path({str(self.signing.registry)!r})
planner.IDENTITY_V2_PROVIDER_REGISTRY_SHA256 = {digest_bytes(self.signing.registry.read_bytes())!r}
{self._semantic_authority_fixture_source()}
raise SystemExit(planner.main(sys.argv[1:]))
'''
        harness.write_text(source, encoding="utf-8")
        harness.chmod(harness.stat().st_mode | stat.S_IXUSR)
        return harness

    def _semantic_authority_fixture_source(self) -> str:
        # Genuine semantic signatures use the planner fixture's ephemeral key;
        # the child relocates the same live root pins without a production seam.
        authority = self.planner.module._semantic_signing_authority()._load_adapter_module()
        return "\n".join([
            "semantic_authority = planner._semantic_signing_authority()._load_adapter_module()",
            *[f"semantic_authority.{field} = {getattr(authority, field)!r}" for field in (
                "CANONICAL_TRUST_ROOT_PATH", "CANONICAL_TRUST_ROOT_FILE_SHA256",
                "CANONICAL_TRUST_ROOT_DIGEST", "CANONICAL_TRUST_ROOT_OWNER_UID")],
        ])

    def _adapter_harness(self) -> Path:
        """Run the real adapter against the same patched planner instance."""
        verifier_tool = self._tool_wrapper()
        harness = self.root / "identity-v2-adapter-harness.py"
        source = f'''#!/usr/bin/env python3
import importlib.util
import sys
from pathlib import Path

def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise SystemExit("cannot load adapter test harness module")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module

planner = load("identity_v2_bridge_adapter_planner", {str(PLANNER)!r})
planner._peer_registry_authority().REGISTRY_PATH = Path({str(self.signing.peer_registry)!r})
planner._peer_registry_authority().REGISTRY_SHA256 = {digest_bytes(self.signing.peer_registry.read_bytes())!r}
planner.IDENTITY_V2_VERIFY_TOOL_PATH = Path({str(verifier_tool)!r})
planner.IDENTITY_V2_VERIFY_TOOL_SHA256 = {digest_bytes(verifier_tool.read_bytes())!r}
planner.IDENTITY_V2_TRUST_CONFIG_PATH = Path({str(self.signing.trust)!r})
planner.IDENTITY_V2_TRUST_CONFIG_SHA256 = {digest_bytes(self.signing.trust.read_bytes())!r}
planner.IDENTITY_V2_PROVIDER_REGISTRY_PATH = Path({str(self.signing.registry)!r})
planner.IDENTITY_V2_PROVIDER_REGISTRY_SHA256 = {digest_bytes(self.signing.registry.read_bytes())!r}
adapter = load("identity_v2_bridge_adapter", {str(ADAPTER)!r})
adapter._PLANNER_MODULE = planner
{self._semantic_authority_fixture_source()}
raise SystemExit(adapter.main(sys.argv[1:]))
'''
        harness.write_text(source, encoding="utf-8")
        harness.chmod(harness.stat().st_mode | stat.S_IXUSR)
        return harness

    def _adapter_authority(self, plan: dict[str, Any]) -> dict[str, Any]:
        """Reuse the adapter contract test's plan-bound authority fixture."""
        adapter_module = load_module("identity_v2_bridge_adapter", ADAPTER)
        fixture_module = load_module("identity_v2_bridge_adapter_fixture", ADAPTER_TEST)
        fixture = fixture_module.FullNetworkCleanRoomAdapterTests("runTest")
        fixture.adapter = adapter_module
        return fixture._authority(plan=plan)

    def _sidecar_args(
        self,
        output: Path,
        evidence_map_out: Path,
        wrapper: Path,
        *,
        signer_tool: Path | None = None,
        verifier_tool: Path | None = None,
        evidence_dir: Path | None = None,
    ) -> list[str]:
        item = self.artifacts["sequencer-204"]
        evidence_dir = evidence_dir or self.root / "sidecar-evidence-root"
        return [
            "--raw-v1", str(item["raw_v1"]),
            "--template", str(item["template"]),
            "--out", str(output),
            "--context", str(self.signing.context),
            "--plan-intent", str(self.signing.intent),
            "--trust-config", str(self.signing.trust),
            "--provider-registry", str(self.signing.registry),
            "--provider-ref", "ephemeral-test-custody",
            "--signer-tool", str(signer_tool or wrapper),
            "--verifier-tool", str(verifier_tool or self.signing.verifier),
            "--evidence-map-out", str(evidence_map_out),
            "--evidence-dir", str(evidence_dir),
        ]

    def test_sidecar_retains_transaction_unique_v2_evidence_bundle(self) -> None:
        wrapper = self._tool_wrapper()
        evidence_root = self.root / "evidence-root"
        first_output = self.root / "first-verified-envelope.json"
        first_map = self.root / "first-evidence-map.json"
        first = self._run(
            SIDECAR,
            *self._sidecar_args(
                first_output,
                first_map,
                wrapper,
                evidence_dir=evidence_root,
            ),
        )
        self.assertEqual(first.returncode, 0, first.stderr)
        first_value = json.loads(first_map.read_text(encoding="utf-8"))
        self.assertEqual(first_value["schema_version"], "oasis7.identity_v2_evidence_map.v2")
        self.assertEqual(first_value["network_id"], self.planner.module.CANONICAL_NETWORK_ID)
        first_entry = first_value["entries"][0]
        self.assertEqual(
            set(first_entry) - {"node_name", "node_id", "peer_id"},
            set(V2_ARTIFACT_FIELDS),
        )
        first_paths = [Path(first_entry[field]["path"]) for field in V2_ARTIFACT_FIELDS]
        self.assertEqual(len({path.parent for path in first_paths}), 1)
        first_transaction_dir = first_paths[0].parent
        self.assertTrue(first_transaction_dir.is_dir())
        self.assertEqual(stat.S_IMODE(first_transaction_dir.stat().st_mode), 0o700)
        self.assertEqual(first_transaction_dir.stat().st_uid, os.getuid())
        for path in first_paths:
            self.assertTrue(path.is_file() and not path.is_symlink())
            self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o600)
            self.assertEqual(path.stat().st_uid, os.getuid())
            field = next(field for field in V2_ARTIFACT_FIELDS if Path(first_entry[field]["path"]) == path)
            self.assertEqual(descriptor(path), first_entry[field])

        second_output = self.root / "second-verified-envelope.json"
        second_map = self.root / "second-evidence-map.json"
        second = self._run(
            SIDECAR,
            *self._sidecar_args(
                second_output,
                second_map,
                wrapper,
                evidence_dir=evidence_root,
            ),
        )
        self.assertEqual(second.returncode, 0, second.stderr)
        second_value = json.loads(second_map.read_text(encoding="utf-8"))
        second_transaction_dir = Path(second_value["entries"][0][V2_ARTIFACT_FIELDS[0]]["path"]).parent
        self.assertNotEqual(first_transaction_dir, second_transaction_dir)

    def test_planner_rejects_missing_v2_retained_artifact(self) -> None:
        map_path = self._write_v2_evidence_map_fixture()
        baseline = self._run(
            PLANNER,
            "--input", str(self.input_path),
            "--identity-v2-evidence-map", str(map_path),
            "--out", str(self.root / "valid-v2-plan.json"),
            "--json",
        )
        self.assertEqual(baseline.returncode, 0, baseline.stderr)
        value = json.loads(map_path.read_text(encoding="utf-8"))
        missing = Path(value["entries"][0]["prepare_manifest"]["path"])
        missing.unlink()
        write_json(map_path, value)
        output = self.root / "missing-v2-plan.json"
        result = self._run(
            PLANNER,
            "--input", str(self.input_path),
            "--identity-v2-evidence-map", str(map_path),
            "--out", str(output),
            "--json",
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr.lower(), r"prepare|missing|artifact|evidence")
        self.assertFalse(output.exists())

    def test_planner_rejects_tampered_v2_retained_artifact_digest(self) -> None:
        map_path = self._write_v2_evidence_map_fixture()
        baseline = self._run(
            PLANNER,
            "--input", str(self.input_path),
            "--identity-v2-evidence-map", str(map_path),
            "--out", str(self.root / "valid-v2-plan.json"),
            "--json",
        )
        self.assertEqual(baseline.returncode, 0, baseline.stderr)
        value = json.loads(map_path.read_text(encoding="utf-8"))
        tampered = Path(value["entries"][0]["provider_attestation"]["path"])
        tampered.write_bytes(tampered.read_bytes() + b"tampered\n")
        output = self.root / "tampered-v2-plan.json"
        result = self._run(
            PLANNER,
            "--input", str(self.input_path),
            "--identity-v2-evidence-map", str(map_path),
            "--out", str(output),
            "--json",
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr.lower(), r"digest|tamper|artifact|evidence")
        self.assertFalse(output.exists())

    def test_sidecar_orchestrates_four_commands_and_retains_exact_evidence(self) -> None:
        wrapper = self._tool_wrapper()
        output = self.root / "sidecar-verified-envelope.json"
        evidence_map_out = self.root / "sidecar-evidence-map.json"
        self.signing.verifier_invocation_marker.unlink(missing_ok=True)
        self.assertFalse(self.signing.verifier_invocation_marker.exists())
        result = self._run(SIDECAR, *self._sidecar_args(output, evidence_map_out, wrapper))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            self.command_log.read_text(encoding="utf-8").splitlines(),
            ["prepare", "sign", "assemble"],
        )
        self.assertTrue(
            self.signing.verifier_invocation_marker.exists(),
            "sidecar did not invoke the registry-selected verifier",
        )
        produced = json.loads(evidence_map_out.read_text(encoding="utf-8"))
        self.assertEqual(produced["schema_version"], "oasis7.identity_v2_evidence_map.v2")
        self.assertEqual(produced["network_id"], self.planner.module.CANONICAL_NETWORK_ID)
        for field, path in (("context", self.signing.context), ("plan_intent", self.signing.intent)):
            self.assertEqual(produced[field]["sha256"], digest_bytes(path.read_bytes()))
            self.assertEqual(produced[field]["size_bytes"], path.stat().st_size)
        entry = produced["entries"][0]
        for field in V2_ARTIFACT_FIELDS:
            retained = Path(entry[field]["path"])
            self.assertEqual(entry[field], descriptor(retained))
            if field == "signed_envelope":
                self.assertEqual(retained.read_bytes(), output.read_bytes())
        self.assertTrue(json.loads(output.read_text(encoding="utf-8"))["verified"])

    def test_planner_ingests_explicit_map_and_emits_verified_legacy_projection(self) -> None:
        output = self.root / "planned.json"
        result = self._run(
            PLANNER,
            "--input", str(self.input_path),
            "--identity-v2-evidence-map", str(self.evidence_map),
            "--out", str(output),
            "--json",
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        plan = json.loads(output.read_text(encoding="utf-8"))
        self.assertEqual(plan["identity_v2_evidence"]["schema_version"], "oasis7.identity_v2_evidence_map.v2")
        self.assertEqual(plan["identity_v2_evidence"]["network_id"], self.planner.module.CANONICAL_NETWORK_ID)
        self.assertEqual(
            plan["identity_v2_evidence"]["context"]["sha256"],
            digest_bytes(self.signing.context.read_bytes()),
        )
        self.assertEqual(len(plan["identity_v2_evidence"]["entries"]), 5)
        for node in plan["nodes"]:
            receipt = node["identity_receipt"]
            self.assertEqual(set(receipt), {
                "schema_version", "authenticated", "verified", "signer_id", "verifier_id",
                "trust_root_id", "signed_payload_sha256", "signature_hex", "canonical_digest",
                "node_id", "peer_id", "key_sha256", "key_size_bytes", "key_mode", "key_uid",
                "key_gid", "capture_window_id", "rotation_epoch", "issued_at", "expires_at",
            })
            self.assertTrue(receipt["authenticated"] and receipt["verified"])

    def test_legacy_input_without_identity_v2_evidence_is_rejected(self) -> None:
        output = self.root / "legacy-without-identity-v2-plan.json"
        result = subprocess.run(
            [
                sys.executable,
                str(PLANNER),
                "--input", str(self.input_path),
                "--out", str(output),
            ],
            cwd=ROOT,
            env=clean_env(),
            capture_output=True,
            text=True,
        )
        self.assertNotEqual(result.returncode, 0, "legacy input bypassed identity-v2 admission")
        self.assertRegex(result.stderr.lower(), r"identity.?v2|evidence|admission|verif")
        self.assertFalse(output.exists())

    def test_adapter_current_admission_consumes_map_without_mutation(self) -> None:
        plan = self.root / "planned.json"
        planner_result = self._run(
            PLANNER,
            "--input", str(self.input_path),
            "--identity-v2-evidence-map", str(self.evidence_map),
            "--out", str(plan),
            "--json",
        )
        self.assertEqual(planner_result.returncode, 0, planner_result.stderr)
        write_json(self.authority_path, self._adapter_authority(json.loads(plan.read_text(encoding="utf-8"))))
        journal = self.root / "adapter.journal.jsonl"
        ledger = self.root / "credential-nonce-ledger.jsonl"
        result = self._run(
            ADAPTER,
            "--plan", str(plan),
            "--authority", str(self.authority_path),
            "--journal", str(journal),
            "--ledger", str(ledger),
            "--identity-v2-evidence-map", str(self.evidence_map),
            "--identity-v2-mode", "current_admission",
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        response = json.loads(result.stdout)
        self.assertEqual(response["identity_v2_mode"], "current_admission")
        self.assertFalse(response["provider_mutation_performed"])
        self.assertEqual(
            response["identity_v2_evidence"]["context"]["sha256"],
            digest_bytes(self.signing.context.read_bytes()),
        )
        self.assertFalse(journal.exists(), "dry-run must not create a durable mutation journal")

    def test_forged_signature_is_rejected_by_planner_and_adapter_before_artifacts(self) -> None:
        forged_map_path, forged_map = self._forged_evidence_map()

        planner_output = self.root / "forged-planner-output.json"
        planner_result = self._run(
            PLANNER,
            "--input", str(self.input_path),
            "--identity-v2-evidence-map", str(forged_map_path),
            "--out", str(planner_output),
            "--json",
        )
        with self.subTest(boundary="planner"):
            self.assertNotEqual(planner_result.returncode, 0, "forged signature unexpectedly passed planner")
            self.assertRegex(planner_result.stderr.lower(), r"signature|crypto|verif|auth|binding")
            self.assertFalse(planner_output.exists())

        valid_plan_path = self.root / "valid-plan-for-adapter.json"
        valid_plan_result = self._run(
            PLANNER,
            "--input", str(self.input_path),
            "--identity-v2-evidence-map", str(self.evidence_map),
            "--out", str(valid_plan_path),
            "--json",
        )
        self.assertEqual(valid_plan_result.returncode, 0, valid_plan_result.stderr)
        forged_plan = json.loads(valid_plan_path.read_text(encoding="utf-8"))
        forged_plan["identity_v2_evidence"] = forged_map
        forged_plan["plan_digest"] = digest_bytes(
            canonical({key: item for key, item in forged_plan.items() if key != "plan_digest"})
        )
        forged_plan_path = self.root / "forged-adapter-plan.json"
        write_json(forged_plan_path, forged_plan)
        forged_authority_path = self.root / "forged-adapter-authority.json"
        write_json(forged_authority_path, self._adapter_authority(forged_plan))
        journal = self.root / "forged-adapter.journal.jsonl"
        ledger = self.root / "forged-adapter.ledger.jsonl"
        adapter_result = self._run(
            ADAPTER,
            "--plan", str(forged_plan_path),
            "--authority", str(forged_authority_path),
            "--journal", str(journal),
            "--ledger", str(ledger),
            "--identity-v2-evidence-map", str(forged_map_path),
            "--identity-v2-mode", "current_admission",
        )
        self.assertNotEqual(adapter_result.returncode, 0, "forged signature unexpectedly passed adapter")
        self.assertRegex(adapter_result.stderr.lower(), r"signature|crypto|verif|auth|evidence")
        self.assertFalse(journal.exists())
        self.assertFalse(ledger.exists())

    def test_sidecar_rejects_unpinned_signer_and_verifier_before_invocation(self) -> None:
        trusted_wrapper = self._tool_wrapper()
        cases = (
            ("signer", "signer_tool"),
            ("verifier", "verifier_tool"),
        )
        for label, tool_role in cases:
            with self.subTest(tool_role=tool_role):
                marker = self.root / f"{label}-invoked.marker"
                untrusted_wrapper = self._tool_wrapper(marker)
                output = self.root / f"{label}-unpinned-envelope.json"
                evidence_map_out = self.root / f"{label}-unpinned-evidence-map.json"
                kwargs = {tool_role: untrusted_wrapper}
                result = self._run(
                    SIDECAR,
                    *self._sidecar_args(
                        output,
                        evidence_map_out,
                        trusted_wrapper,
                        **kwargs,
                    ),
                )
                self.assertNotEqual(result.returncode, 0, f"unpinned {tool_role} unexpectedly succeeded")
                self.assertRegex(result.stderr.lower(), r"pin|digest|govern|allow|trust")
                self.assertFalse(marker.exists(), f"unpinned {tool_role} was invoked")
                self.assertFalse(output.exists())
                self.assertFalse(evidence_map_out.exists())

    def test_sidecar_rejects_output_aliases_without_mutating_protected_inputs(self) -> None:
        trusted_wrapper = self._tool_wrapper()
        item = self.artifacts["sequencer-204"]
        protected = (
            ("raw-v1", item["raw_v1"]),
            ("template", item["template"]),
            ("context", self.signing.context),
            ("plan-intent", self.signing.intent),
            ("trust-config", self.signing.trust),
            ("provider-registry", self.signing.registry),
            ("signer-tool", trusted_wrapper),
            ("verifier-tool", self.signing.verifier),
        )
        for label, protected_path in protected:
            with self.subTest(protected=label):
                before = protected_path.read_bytes()
                evidence_map_out = self.root / f"{label}-alias-evidence-map.json"
                result = self._run(
                    SIDECAR,
                    *self._sidecar_args(protected_path, evidence_map_out, trusted_wrapper),
                )
                self.assertNotEqual(result.returncode, 0, f"{label} alias unexpectedly succeeded")
                self.assertRegex(result.stderr.lower(), r"alias|protected|output")
                self.assertTrue(protected_path.is_file())
                self.assertEqual(protected_path.read_bytes(), before)
                self.assertFalse(evidence_map_out.exists())

    def test_sidecar_rejects_hardlink_output_alias_without_mutation(self) -> None:
        trusted_wrapper = self._tool_wrapper()
        raw_path = self.artifacts["sequencer-204"]["raw_v1"]
        hardlink = self.root / "raw-v1-hardlink"
        os.link(raw_path, hardlink)
        before = raw_path.read_bytes()
        evidence_map_out = self.root / "hardlink-alias-evidence-map.json"
        result = self._run(
            SIDECAR,
            *self._sidecar_args(hardlink, evidence_map_out, trusted_wrapper),
        )
        self.assertNotEqual(result.returncode, 0, "hardlink output alias unexpectedly succeeded")
        self.assertRegex(result.stderr.lower(), r"alias|protected|output")
        self.assertEqual(raw_path.read_bytes(), before)
        self.assertEqual(hardlink.read_bytes(), before)
        self.assertFalse(evidence_map_out.exists())

    def test_sidecar_rejects_evidence_map_alias_without_mutating_protected_input(self) -> None:
        trusted_wrapper = self._tool_wrapper()
        raw_path = self.artifacts["sequencer-204"]["raw_v1"]
        before = raw_path.read_bytes()
        output = self.root / "independent-envelope-output.json"
        result = self._run(
            SIDECAR,
            *self._sidecar_args(output, raw_path, trusted_wrapper),
        )
        self.assertNotEqual(result.returncode, 0, "evidence-map raw alias unexpectedly succeeded")
        self.assertRegex(result.stderr.lower(), r"alias|protected|output")
        self.assertEqual(raw_path.read_bytes(), before)
        self.assertFalse(output.exists())

    def test_sidecar_rejects_output_pair_alias_without_mutating_existing_output(self) -> None:
        trusted_wrapper = self._tool_wrapper()
        output = self.root / "aliased-output.json"
        sentinel = b"previous verified output\n"
        output.write_bytes(sentinel)
        result = self._run(
            SIDECAR,
            *self._sidecar_args(output, output, trusted_wrapper),
        )
        self.assertNotEqual(result.returncode, 0, "output pair alias unexpectedly succeeded")
        self.assertRegex(result.stderr.lower(), r"alias|output")
        self.assertEqual(output.read_bytes(), sentinel)

    def test_sidecar_preserves_existing_outputs_when_validation_fails(self) -> None:
        trusted_wrapper = self._tool_wrapper()
        invalid_raw = self.root / "invalid.raw-v1"
        invalid_bytes = b"not-json\n"
        invalid_raw.write_bytes(invalid_bytes)
        output = self.root / "previous-output.json"
        evidence_map_out = self.root / "previous-evidence-map.json"
        output_bytes = b"previous envelope\n"
        evidence_bytes = b"previous evidence map\n"
        output.write_bytes(output_bytes)
        evidence_map_out.write_bytes(evidence_bytes)
        args = self._sidecar_args(output, evidence_map_out, trusted_wrapper)
        raw_index = args.index("--raw-v1")
        args[raw_index + 1] = str(invalid_raw)
        result = self._run(SIDECAR, *args)
        self.assertNotEqual(result.returncode, 0, "invalid raw-v1 unexpectedly succeeded")
        self.assertRegex(result.stderr.lower(), r"raw|json|signing-tool|invalid")
        self.assertEqual(invalid_raw.read_bytes(), invalid_bytes)
        self.assertEqual(output.read_bytes(), output_bytes)
        self.assertEqual(evidence_map_out.read_bytes(), evidence_bytes)

    def test_sidecar_rejects_registry_provider_authority_aliases(self) -> None:
        trusted_wrapper = self._tool_wrapper()
        registry = json.loads(self.signing.registry.read_text(encoding="utf-8"))
        authority_paths: list[tuple[str, Path]] = []
        for index, provider in enumerate(registry["providers"]):
            for field in ("adapter_path", "public_key_ref"):
                authority_paths.append((f"provider-{index}-{field}", Path(provider[field])))
        for label, authority_path in authority_paths:
            with self.subTest(authority=label):
                before = authority_path.read_bytes()
                before_mode = stat.S_IMODE(authority_path.stat().st_mode)
                evidence_map_out = self.root / f"{label}-evidence-map.json"
                try:
                    result = self._run(
                        SIDECAR,
                        *self._sidecar_args(authority_path, evidence_map_out, trusted_wrapper),
                    )
                    self.assertNotEqual(result.returncode, 0, f"{label} alias unexpectedly succeeded")
                    self.assertRegex(result.stderr.lower(), r"alias|protected|output")
                    self.assertEqual(authority_path.read_bytes(), before)
                    self.assertFalse(evidence_map_out.exists())
                finally:
                    authority_path.write_bytes(before)
                    authority_path.chmod(before_mode)

    def test_sidecar_restores_output_pair_when_evidence_map_write_fails(self) -> None:
        trusted_wrapper = self._tool_wrapper()
        output = self.root / "previous-output.json"
        evidence_root = self.root / "unwritable-evidence-root"
        evidence_root.mkdir(mode=0o700)
        evidence_map_out = evidence_root / "previous-evidence-map.json"
        output_bytes = b"previous envelope\n"
        evidence_bytes = b"previous evidence map\n"
        output.write_bytes(output_bytes)
        evidence_map_out.write_bytes(evidence_bytes)
        evidence_root.chmod(0o500)
        try:
            result = self._run(
                SIDECAR,
                *self._sidecar_args(output, evidence_map_out, trusted_wrapper),
            )
        finally:
            evidence_root.chmod(0o700)
        self.assertNotEqual(result.returncode, 0, "unwritable evidence-map parent unexpectedly succeeded")
        self.assertRegex(result.stderr.lower(), r"permission|evidence|write|output")
        self.assertEqual(output.read_bytes(), output_bytes)
        self.assertEqual(evidence_map_out.read_bytes(), evidence_bytes)

    def test_cross_pair_or_missing_evidence_rejects_before_any_mutation(self) -> None:
        bad_map = self.root / "cross-pair-map.json"
        value = json.loads(self.evidence_map.read_text(encoding="utf-8"))
        value["entries"][0]["peer_id"] = "12D3KooWcross-paired-peer"
        write_json(bad_map, value)
        output = self.root / "rejected-plan.json"
        result = self._run(
            PLANNER,
            "--input", str(self.input_path),
            "--identity-v2-evidence-map", str(bad_map),
            "--out", str(output),
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr.lower(), r"evidence|cross|pair|peer")
        self.assertFalse(output.exists())
        self.assertFalse((self.root / "mutation.journal").exists())


if __name__ == "__main__":
    unittest.main()
