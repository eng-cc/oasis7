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


def clean_env() -> dict[str, str]:
    return {"PATH": os.environ.get("PATH", ""), "PYTHONIOENCODING": "utf-8"}


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
                "schema_version": "oasis7.clean_room_plan_intent.v1",
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
            _, _, _, _, envelope = self.signing._prepare_sign_assemble(stem)
            verified, verification = self.signing._verify(envelope)
            # The S2 fixture reuses its raw and verification output paths for
            # each case. Preserve node-specific bytes before the next case can
            # overwrite those paths; context and plan-intent remain immutable
            # shared inputs for the whole managed-node capture.
            raw_v1 = self.root / f"{name}.identity-receipt.v1.raw"
            template = self.root / f"{name}.identity-v2.unsigned-template.json"
            verified_envelope = self.root / f"{name}.identity-v2.verified.json"
            verification_receipt = self.root / f"{name}.identity-v2.verification.json"
            shutil.copyfile(self.signing.raw, raw_v1)
            shutil.copyfile(self.signing.template, template)
            shutil.copyfile(verified, verified_envelope)
            shutil.copyfile(verification, verification_receipt)
            artifacts[name] = {
                "raw_v1": raw_v1,
                "template": template,
                "signed_envelope": envelope,
                "verified_envelope": verified_envelope,
                "verification": verification_receipt,
            }
        return artifacts

    def _write_evidence_map(self) -> Path:
        context = self.signing.context
        intent = self.signing.intent
        evidence = {
            "schema_version": "oasis7.identity_v2_evidence_map.v1",
            "task_uid": self.request["task_uid"],
            "head_oid": self.request["head_oid"],
            "context": descriptor(context),
            "plan_intent": descriptor(intent),
            "entries": [],
        }
        for node in self.request["nodes"]:
            name = str(node["name"])
            item = self.artifacts[name]
            evidence["entries"].append(
                {
                    "node_name": name,
                    "node_id": str(node["node_id"]),
                    "peer_id": self.planner.module.CANONICAL_PEER_REGISTRY[name],
                    "raw_v1": descriptor(item["raw_v1"]),
                    "signed_envelope": descriptor(item["signed_envelope"]),
                    "verification": descriptor(item["verification"]),
                }
            )
        path = self.root / "identity-v2-evidence-map.json"
        write_json(path, evidence)
        return path

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
        forged_dir.mkdir()
        forged_envelope = forged_dir / "storage-205.envelope.json"
        write_json(forged_envelope, envelope)

        original_verification = Path(target["verification"]["path"])
        verification = json.loads(original_verification.read_text(encoding="utf-8"))
        verification["envelope_sha256"] = digest_bytes(forged_envelope.read_bytes())
        forged_verification = forged_dir / "storage-205.verification.json"
        write_json(forged_verification, verification)

        target["signed_envelope"] = descriptor(forged_envelope)
        target["verification"] = descriptor(forged_verification)
        forged_map = self.root / "forged-identity-v2-evidence-map.json"
        write_json(forged_map, value)
        return forged_map, value

    def _tool_wrapper(self, marker: Path | None = None) -> Path:
        log = self.root / "signing-tool-commands.log"
        wrapper_name = "identity-v2-signing-tool-wrapper.py" if marker is None else f"untrusted-{marker.stem}.py"
        wrapper = self.root / wrapper_name
        harness = load_module("identity_v2_signing_tool_contract_for_wrapper", TOOL_TEST).CHILD_HARNESS
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
sidecar.IDENTITY_V2_VERIFIER_TOOL_PATH = Path({str(trusted_wrapper)!r})
sidecar.IDENTITY_V2_VERIFIER_TOOL_SHA256 = {trusted_digest!r}
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
planner.IDENTITY_V2_VERIFY_TOOL_PATH = Path({str(verifier_tool)!r})
planner.IDENTITY_V2_VERIFY_TOOL_SHA256 = {digest_bytes(verifier_tool.read_bytes())!r}
planner.IDENTITY_V2_TRUST_CONFIG_PATH = Path({str(self.signing.trust)!r})
planner.IDENTITY_V2_TRUST_CONFIG_SHA256 = {digest_bytes(self.signing.trust.read_bytes())!r}
planner.IDENTITY_V2_PROVIDER_REGISTRY_PATH = Path({str(self.signing.registry)!r})
planner.IDENTITY_V2_PROVIDER_REGISTRY_SHA256 = {digest_bytes(self.signing.registry.read_bytes())!r}
raise SystemExit(planner.main(sys.argv[1:]))
'''
        harness.write_text(source, encoding="utf-8")
        harness.chmod(harness.stat().st_mode | stat.S_IXUSR)
        return harness

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
planner.IDENTITY_V2_VERIFY_TOOL_PATH = Path({str(verifier_tool)!r})
planner.IDENTITY_V2_VERIFY_TOOL_SHA256 = {digest_bytes(verifier_tool.read_bytes())!r}
planner.IDENTITY_V2_TRUST_CONFIG_PATH = Path({str(self.signing.trust)!r})
planner.IDENTITY_V2_TRUST_CONFIG_SHA256 = {digest_bytes(self.signing.trust.read_bytes())!r}
planner.IDENTITY_V2_PROVIDER_REGISTRY_PATH = Path({str(self.signing.registry)!r})
planner.IDENTITY_V2_PROVIDER_REGISTRY_SHA256 = {digest_bytes(self.signing.registry.read_bytes())!r}
adapter = load("identity_v2_bridge_adapter", {str(ADAPTER)!r})
adapter._PLANNER_MODULE = planner
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
    ) -> list[str]:
        item = self.artifacts["sequencer-204"]
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
            "--verifier-tool", str(verifier_tool or wrapper),
            "--evidence-map-out", str(evidence_map_out),
        ]

    def test_sidecar_orchestrates_four_commands_and_retains_exact_evidence(self) -> None:
        wrapper = self._tool_wrapper()
        output = self.root / "sidecar-verified-envelope.json"
        evidence_map_out = self.root / "sidecar-evidence-map.json"
        result = self._run(SIDECAR, *self._sidecar_args(output, evidence_map_out, wrapper))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            self.command_log.read_text(encoding="utf-8").splitlines(),
            ["prepare", "sign", "assemble", "verify"],
        )
        produced = json.loads(evidence_map_out.read_text(encoding="utf-8"))
        self.assertEqual(produced["schema_version"], "oasis7.identity_v2_evidence_map.v1")
        for field, path in (("context", self.signing.context), ("plan_intent", self.signing.intent)):
            self.assertEqual(produced[field], descriptor(path))
        entry = produced["entries"][0]
        self.assertEqual(entry["raw_v1"], descriptor(self.artifacts["sequencer-204"]["raw_v1"]))
        self.assertEqual(entry["signed_envelope"], descriptor(output))
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
        self.assertEqual(plan["identity_v2_evidence"]["schema_version"], "oasis7.identity_v2_evidence_map.v1")
        self.assertEqual(plan["identity_v2_evidence"]["context"], descriptor(self.signing.context))
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
        self.assertEqual(response["identity_v2_evidence"]["context"], descriptor(self.signing.context))
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
            self.assertRegex(planner_result.stderr.lower(), r"signature|crypto|verif|auth")
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
