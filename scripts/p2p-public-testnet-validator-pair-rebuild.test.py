#!/usr/bin/env python3
"""Contract tests for the governed validator pair rebuild executor.

These tests intentionally exercise only local temporary roots.  They are the
RED contract for task #3324 (predecessor task #3318); no live validator, SSH
target, or observer is used.
"""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
import shutil
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
EXECUTOR = ROOT / "scripts" / "p2p-public-testnet-validator-pair-rebuild.py"
PROVENANCE = ROOT / "scripts" / "p2p-public-testnet-validator-pair-provenance.py"
RELEASE_BUNDLE = ROOT / "scripts" / "release-candidate-bundle.sh"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def canonical_digest(value: dict) -> str:
    body = {key: item for key, item in value.items() if key != "canonical_digest"}
    return hashlib.sha256(json.dumps(body, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


class ValidatorPairRebuildContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory(prefix="oasis7-pair-rebuild-contract-")
        self.root = Path(self.tmp.name)
        self.package = self.root / "package"
        self.package.mkdir()
        runtime = self.package / "oasis7_chain_runtime"
        runtime.write_bytes(b"runtime-261\n")
        self.runtime_sha = sha256(runtime)
        self.runtime_size = runtime.stat().st_size
        (self.package / "BUILDINFO").write_text(
            "run_id=3218\ncommit=" + "a" * 40 + "\npackage_version=0.0.0+testnet.261\n",
            encoding="utf-8",
        )
        (self.package / "SHA256SUMS").write_text(
            f"{self.runtime_sha}  oasis7_chain_runtime\n",
            encoding="utf-8",
        )
        self.governed = self.root / "governed"
        self.governed.mkdir()
        for name, content in {
            "manifest.json": '{"network_id":"oasis7-public-testnet-governed-20260606","chain_id":"oasis7-public-testnet-governed-20260606"}\n',
            "genesis.json": '{"world_id":"oasis7-public-testnet-governed-20260606","chain_id":"oasis7-public-testnet-governed-20260606"}\n',
            "registry.json": '{"validators":[{"node_id":"triad-testnet-sequencer"},{"node_id":"triad-testnet-storage"}]}\n',
            "bootstrap.txt": "/ip4/127.0.0.1/tcp/6831/p2p/peer-sequencer-204\n/ip4/127.0.0.1/tcp/6832/p2p/peer-storage-205\n",
            "world.json": '{"snapshot":true}\n',
        }.items():
            (self.governed / name).write_text(content, encoding="utf-8")
        self.provenance = self.root / "provenance.json"
        self.signing_key = self.root / "attestor-key.pem"
        self.public_key = self.root / "attestor-public.pem"
        self.signature = self.root / "attestor-signature.bin"
        subprocess.run(
            [
                "openssl",
                "genpkey",
                "-algorithm",
                "RSA",
                "-pkeyopt",
                "rsa_keygen_bits:2048",
                "-out",
                str(self.signing_key),
            ],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        subprocess.run(
            ["openssl", "pkey", "-in", str(self.signing_key), "-pubout", "-out", str(self.public_key)],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        self._write_provenance()
        self.trust_root = self.root / "provenance-trust-root.json"
        trust_root = {
            "schema_version": "oasis7.validator_pair_provenance_trust_root.v1",
            "allowlist": [
                {
                    "signer_id": "testnet-package-attestor",
                    "algorithm": "openssl-rsa-sha256",
                    "public_key_sha256": sha256(self.public_key),
                }
            ],
        }
        trust_root["root_digest"] = canonical_digest(trust_root)
        self.trust_root.write_text(json.dumps(trust_root, indent=2) + "\n", encoding="utf-8")
        self.identity_receipts = self.root / "identity-receipts.json"
        identity_paths = []
        for role in ("storage-205", "sequencer-204"):
            identity_path = self.root / f"identity-{role}.json"
            signature_path = self.root / f"identity-{role}.sig"
            self._write_attestation(
                identity_path,
                signature_path,
                "oasis7.validator_identity_receipt.v1",
                role,
                f"peer-{role}",
                node_id="triad-testnet-storage" if role == "storage-205" else "triad-testnet-sequencer",
            )
            identity_paths.append(str(identity_path))
        self.identity_receipts.write_text(json.dumps({"receipts": identity_paths}, indent=2) + "\n", encoding="utf-8")
        self.rebuild_proof = self.root / "sequencer-204-rebuild-proof.json"
        self._write_attestation(
            self.rebuild_proof,
            self.root / "sequencer-204-rebuild-proof.sig",
            "oasis7.validator_pair_rebuild_proof.v1",
            "sequencer-204",
            "peer-sequencer-204",
            "triad-testnet-sequencer",
        )
        self.impact = self.root / "consumer-impact.json"
        self.impact.write_text(
            json.dumps(
                {
                    "impact": "none",
                    "evidence_source": "local contract fixture",
                    "timestamp": "2026-08-18T00:00:00Z",
                    "validators_already_stopped": True,
                    "outage_update_channel": "n/a",
                    "recovery_update_checkpoint": "n/a",
                    "producer_wording_approval": "n/a",
                    "decision": "proceed",
                }
            ),
            encoding="utf-8",
        )
        self.nodes = {}
        for role in ("storage-205", "sequencer-204"):
            node = self.root / role
            (node / "data").mkdir(parents=True)
            (node / "config").mkdir()
            (node / "current" / "bin").mkdir(parents=True)
            (node / "current" / "bin" / "oasis7_chain_runtime").write_bytes(b"runtime-old\n")
            (node / "config" / "node.env").write_text("NODE_ID=" + role + "\n", encoding="utf-8")
            self.nodes[role] = node
        self.capacity = self.root / "capacity.json"
        self.capacity.write_text(
            json.dumps(
                {
                    role: {
                        "free_bytes": 10_000_000,
                        "free_inodes": 100_000,
                        "same_filesystem": True,
                    }
                    for role in self.nodes
                }
            ),
            encoding="utf-8",
        )
        self.out = self.root / "out"

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def _write_provenance(self, *, signature: bool = True, extra_governed: Path | None = None) -> None:
        governed = {
            name: {"path": str(path), "sha256": sha256(path), "size_bytes": path.stat().st_size}
            for name, path in (
                ("manifest", self.governed / "manifest.json"),
                ("genesis", self.governed / "genesis.json"),
                ("registry", self.governed / "registry.json"),
                ("bootstrap", self.governed / "bootstrap.txt"),
                ("world", self.governed / "world.json"),
            )
        }
        if extra_governed is not None:
            governed["unexpected"] = {
                "path": str(extra_governed),
                "sha256": sha256(extra_governed),
                "size_bytes": extra_governed.stat().st_size,
            }
        payload = {
            "schema_version": "oasis7.validator_pair_rebuild_provenance.v1",
            "network_id": "oasis7-public-testnet-governed-20260606",
            "chain_id": "oasis7-public-testnet-governed-20260606",
            "package": {
                "run_id": "3218",
                "commit": "a" * 40,
                "package_version": "0.0.0+testnet.261",
                "runtime_sha256": self.runtime_sha,
                "runtime_size_bytes": self.runtime_size,
                "buildinfo_sha256": sha256(self.package / "BUILDINFO"),
                "sha256sums_sha256": sha256(self.package / "SHA256SUMS"),
            },
            "governed": governed,
            "signature": (
                {
                    "status": "verified",
                    "signer_id": "testnet-package-attestor",
                    "algorithm": "openssl-rsa-sha256",
                    "signature_ref": str(self.signature),
                    "public_key_ref": str(self.public_key),
                    "public_key_sha256": sha256(self.public_key),
                }
                if signature
                else None
            ),
        }
        canonical = json.dumps(payload, sort_keys=True, separators=(",", ":"), ensure_ascii=True)
        payload["binding_digest"] = hashlib.sha256(canonical.encode()).hexdigest()
        self.provenance.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
        if signature:
            payload_path = self.root / "provenance-signing-payload.bin"
            payload_path.write_bytes(canonical.encode())
            subprocess.run(
                ["openssl", "dgst", "-sha256", "-sign", str(self.signing_key), "-out", str(self.signature), str(payload_path)],
                check=True,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )

    def _write_attestation(self, path: Path, signature_path: Path, schema: str, role: str, peer_id: str, node_id: str) -> None:
        value = {
            "schema_version": schema,
            "role": role,
            "peer_id": peer_id,
            "node_id": node_id,
            "signer_id": "testnet-package-attestor",
            "algorithm": "openssl-rsa-sha256",
            "public_key_ref": str(self.public_key),
            "public_key_sha256": sha256(self.public_key),
            "signature_ref": str(signature_path),
            "trust_root_digest": json.loads(self.trust_root.read_text(encoding="utf-8"))["root_digest"],
        }
        body = {key: item for key, item in value.items() if key not in {"signature_ref", "public_key_ref"}}
        payload_path = path.with_suffix(path.suffix + ".payload")
        payload_path.write_bytes(json.dumps(body, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode())
        subprocess.run(
            ["openssl", "dgst", "-sha256", "-sign", str(self.signing_key), "-out", str(signature_path), str(payload_path)],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        payload_path.unlink()
        path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")

    def _write_host_adapter(self) -> Path:
        adapter = self.root / "host-adapter.py"
        adapter.write_text(
            """#!/usr/bin/env python3
import json
import sys
from pathlib import Path

transaction = json.loads(Path(sys.argv[sys.argv.index('--transaction') + 1]).read_text())
phase = sys.argv[sys.argv.index('--phase') + 1]
nodes = {}
for role in transaction['mutation_order']:
    nodes[role] = {'active': phase == 'apply', 'running': phase == 'apply'}
    if phase == 'quiesce':
        nodes[role].update({'active': False, 'running': False})
    elif phase == 'backup':
        nodes[role].update({'backup_verified': True})
    elif phase == 'rollback':
        nodes[role].update({'rollback_verified': True})
    else:
        nodes[role].update({
            'healthz_ok': True,
            'nrestarts': 0,
            'oom_panic_segfault': False,
            'runtime_sha256': transaction['package']['runtime_sha256'],
            'runtime_size_bytes': transaction['package']['runtime_size_bytes'],
            'listeners': ['6631', '6831'] if role == 'sequencer-204' else ['6632', '6832'],
            'full_chain_status_called': False,
        })
identity = json.loads(Path(transaction['proof']['identity_receipts_path']).read_text())
identity_paths = identity['receipts']
proof_path = transaction['proof']['sequencer_rebuild_proof_path']
print(json.dumps({
    'schema_version': 'oasis7.validator_pair_rebuild_host_receipt.v2',
    'phase': phase,
    'transaction_id': transaction['transaction_id'],
    'mutation_order': transaction['mutation_order'],
    'startup_order': transaction['startup_order'],
    'nodes': nodes,
    'observer_mutation': False,
    'sequencer_proof_url': transaction['proof']['sequencer_proof_url'],
    'identity_receipts': [{'path': path, 'role': json.loads(Path(path).read_text())['role']} for path in identity_paths] if phase == 'apply' else [],
    'sequencer_rebuild_proof': {'path': proof_path, 'role': 'sequencer-204'} if phase == 'apply' else {},
}))
""",
            encoding="utf-8",
        )
        adapter.chmod(0o755)
        return adapter

    def _base_args(self) -> list[str]:
        return [
            sys.executable,
            str(EXECUTOR),
            "plan",
            "--package-dir",
            str(self.package),
            "--provenance",
            str(self.provenance),
            "--trust-root",
            str(self.trust_root),
            "--identity-receipts",
            str(self.identity_receipts),
            "--sequencer-rebuild-proof",
            str(self.rebuild_proof),
            "--sequencer-proof-url",
            "http://127.0.0.1:6631/v1/chain/rebuild-proof",
            "--consumer-impact-record",
            str(self.impact),
            "--capacity-json",
            str(self.capacity),
            "--node",
            f"storage-205=local:{self.nodes['storage-205']}",
            "--node",
            f"sequencer-204=local:{self.nodes['sequencer-204']}",
            "--out-dir",
            str(self.out),
        ]

    def test_plan_requires_verified_provenance_and_emits_both_orders(self) -> None:
        result = subprocess.run(self._base_args(), text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads(result.stdout)
        self.assertEqual(receipt["schema_version"], "oasis7.validator_pair_rebuild_plan.v1")
        self.assertRegex(receipt["plan_digest"], r"^[0-9a-f]{64}$")
        self.assertEqual(receipt["mutation_order"], ["storage-205", "sequencer-204"])
        self.assertEqual(receipt["startup_order"], ["sequencer-204", "storage-205"])
        self.assertEqual(receipt["phase"], "planned")

    def test_plan_digest_is_stable_and_runtime_transaction_metadata_is_separate(self) -> None:
        first = subprocess.run(self._base_args(), text=True, capture_output=True)
        self.assertEqual(first.returncode, 0, first.stderr)
        first_receipt = json.loads(first.stdout)
        second_out = self.root / "out-second"
        second_args = [arg for arg in self._base_args() if arg not in ("--out-dir", str(self.out))]
        second_args.extend(["--out-dir", str(second_out)])
        second = subprocess.run(second_args, text=True, capture_output=True)
        self.assertEqual(second.returncode, 0, second.stderr)
        second_receipt = json.loads(second.stdout)
        self.assertEqual(first_receipt, second_receipt)
        self.assertNotIn("transaction_id", first_receipt)
        self.assertNotIn("created_at", first_receipt)

    def test_plan_rejects_full_status_url_for_204(self) -> None:
        result = subprocess.run(
            self._base_args() + ["--sequencer-proof-url", "http://127.0.0.1:6631/v1/chain/status"],
            text=True,
            capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("204", result.stderr)

    def test_active_runbook_has_no_sequencer_full_status_command(self) -> None:
        runbook = (ROOT / "doc/p2p/blockchain/public-testnet-governed-bootstrap.runbook.md").read_text(encoding="utf-8")
        self.assertNotIn("6631/v1/chain/status", runbook)
        self.assertNotIn("39.104.204.172:6631/v1/chain/status", runbook)

    def test_plan_rejects_untrusted_signer_even_when_detached_signature_is_valid(self) -> None:
        receipt = json.loads(self.provenance.read_text(encoding="utf-8"))
        receipt["signature"]["signer_id"] = "unlisted-attestor"
        body = {key: value for key, value in receipt.items() if key != "binding_digest"}
        receipt["binding_digest"] = hashlib.sha256(
            json.dumps(body, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest()
        payload_path = self.root / "untrusted-signing-payload.bin"
        payload_path.write_bytes(json.dumps(body, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode())
        subprocess.run(
            ["openssl", "dgst", "-sha256", "-sign", str(self.signing_key), "-out", str(self.signature), str(payload_path)],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        self.provenance.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
        result = subprocess.run(self._base_args(), text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("trusted signer", result.stderr.lower())

    def test_plan_rejects_unexpected_governed_key(self) -> None:
        extra = self.governed / "unexpected.json"
        extra.write_text("{}\n", encoding="utf-8")
        self._write_provenance(extra_governed=extra)
        result = subprocess.run(self._base_args(), text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("governed", result.stderr.lower())

    def test_canonical_current_symlink_is_counted_and_accepted(self) -> None:
        for role, node in self.nodes.items():
            current = node / "current"
            shutil_target = node / "releases" / "known"
            shutil_target.mkdir(parents=True)
            (shutil_target / "bin").mkdir()
            (shutil_target / "bin" / "oasis7_chain_runtime").write_bytes(b"runtime-old\n")
            shutil.rmtree(current)
            current.symlink_to("releases/known", target_is_directory=True)
        result = subprocess.run(self._base_args(), text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_arbitrary_current_symlink_is_rejected(self) -> None:
        current = self.nodes["storage-205"] / "current"
        shutil.rmtree(current)
        current.symlink_to(self.root / "package", target_is_directory=True)
        result = subprocess.run(self._base_args(), text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("current", result.stderr.lower())

    def test_apply_requires_phase_callbacks_and_signed_identity_proof(self) -> None:
        plan = subprocess.run(self._base_args(), text=True, capture_output=True, check=True)
        plan_path = self.out / "transaction.json"
        plan_path.write_text(plan.stdout, encoding="utf-8")
        adapter = self._write_host_adapter()
        result = subprocess.run(
            [sys.executable, str(EXECUTOR), "apply", "--transaction", str(plan_path), "--host-adapter", str(adapter)],
            text=True,
            capture_output=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads(result.stdout)
        self.assertEqual(receipt["phase"], "applied")
        self.assertIn("quiesce_receipt", receipt)
        self.assertIn("backup_receipt", receipt)
        self.assertIn("sequencer_rebuild_proof", receipt["host_receipt"])

    def test_plan_rejects_missing_signature_before_any_node_access(self) -> None:
        self._write_provenance(signature=False)
        result = subprocess.run(self._base_args(), text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("signature", result.stderr.lower())

    def test_provenance_rejects_tampered_signed_body(self) -> None:
        receipt = json.loads(self.provenance.read_text(encoding="utf-8"))
        receipt["package"]["run_id"] = "tampered"
        body = {key: value for key, value in receipt.items() if key != "binding_digest"}
        receipt["binding_digest"] = hashlib.sha256(
            json.dumps(body, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest()
        self.provenance.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
        result = subprocess.run(
            [
                sys.executable,
                str(PROVENANCE),
                "validate",
                "--provenance",
                str(self.provenance),
                "--package-dir",
                str(self.package),
            ],
            text=True,
            capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("detached signature verification failed", result.stderr.lower())

    def test_staged_receipt_falls_back_to_copied_detached_files(self) -> None:
        staged = self.root / "staged-evidence"
        staged.mkdir()
        staged_receipt = staged / self.provenance.name
        staged_receipt.write_bytes(self.provenance.read_bytes())
        (staged / self.signature.name).write_bytes(self.signature.read_bytes())
        (staged / self.public_key.name).write_bytes(self.public_key.read_bytes())
        original_signature = self.signature.read_bytes()
        original_public_key = self.public_key.read_bytes()
        self.signature.unlink()
        self.public_key.unlink()
        result = subprocess.run(
            [
                sys.executable,
                str(PROVENANCE),
                "validate",
                "--provenance",
                str(staged_receipt),
                "--package-dir",
                str(self.package),
            ],
            text=True,
            capture_output=True,
        )
        self.signature.write_bytes(original_signature)
        self.public_key.write_bytes(original_public_key)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_apply_writes_full_manifests_and_preserves_order(self) -> None:
        plan = subprocess.run(self._base_args(), text=True, capture_output=True, check=True)
        plan_path = self.out / "transaction.json"
        plan_path.write_text(plan.stdout, encoding="utf-8")
        adapter = self._write_host_adapter()
        result = subprocess.run(
            [
                sys.executable,
                str(EXECUTOR),
                "apply",
                "--transaction",
                str(plan_path),
                "--host-adapter",
                str(adapter),
            ],
            text=True,
            capture_output=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads(result.stdout)
        self.assertEqual(receipt["phase"], "applied")
        self.assertEqual(receipt["mutation_order"], ["storage-205", "sequencer-204"])
        self.assertTrue(receipt["nodes"]["storage-205"]["backup"]["verified"])
        self.assertTrue(receipt["nodes"]["sequencer-204"]["backup"]["verified"])
        self.assertTrue((self.nodes["storage-205"] / "backups").exists())
        self.assertTrue((self.nodes["sequencer-204"] / "backups").exists())
        self.assertTrue(receipt["host_receipt"]["nodes"]["sequencer-204"]["healthz_ok"])
        self.assertTrue(receipt["capacity_apply"]["storage-205"]["verified"])

    def test_apply_requires_host_gate_receipt(self) -> None:
        plan = subprocess.run(self._base_args(), text=True, capture_output=True, check=True)
        plan_path = self.out / "transaction.json"
        plan_path.write_text(plan.stdout, encoding="utf-8")
        result = subprocess.run(
            [sys.executable, str(EXECUTOR), "apply", "--transaction", str(plan_path)],
            text=True,
            capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("host adapter", result.stderr.lower())
        self.assertFalse((self.nodes["storage-205"] / "backups").exists())

    def test_apply_rechecks_capacity_after_plan(self) -> None:
        plan = subprocess.run(self._base_args(), text=True, capture_output=True, check=True)
        transaction = json.loads(plan.stdout)
        transaction["capacity"]["storage-205"]["required_bytes"] = 10**30
        transaction["plan_digest"] = hashlib.sha256(
            json.dumps({key: value for key, value in transaction.items() if key != "plan_digest"}, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest()
        plan_path = self.out / "transaction.json"
        plan_path.write_text(json.dumps(transaction), encoding="utf-8")
        result = subprocess.run(
            [
                sys.executable,
                str(EXECUTOR),
                "apply",
                "--transaction",
                str(plan_path),
                "--host-adapter",
                str(self._write_host_adapter()),
            ],
            text=True,
            capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("apply-time capacity", result.stderr.lower())
        self.assertFalse((self.nodes["storage-205"] / "backups").exists())

    def test_rollback_restores_the_stopped_snapshot(self) -> None:
        plan = subprocess.run(self._base_args(), text=True, capture_output=True, check=True)
        plan_path = self.out / "transaction.json"
        plan_path.write_text(plan.stdout, encoding="utf-8")
        adapter = self._write_host_adapter()
        applied = subprocess.run(
            [
                sys.executable,
                str(EXECUTOR),
                "apply",
                "--transaction",
                str(plan_path),
                "--host-adapter",
                str(adapter),
            ],
            text=True,
            capture_output=True,
            check=True,
        )
        applied_receipt = json.loads(applied.stdout)
        transaction_id = applied_receipt["transaction_id"]
        self.assertNotEqual(sha256(self.nodes["storage-205"] / "current" / "bin" / "oasis7_chain_runtime"), sha256(self.nodes["storage-205"] / "backups" / transaction_id / "snapshot" / "current" / "bin" / "oasis7_chain_runtime"))
        result = subprocess.run(
            [sys.executable, str(EXECUTOR), "rollback", "--transaction", str(plan_path), "--host-adapter", str(adapter)],
            text=True,
            capture_output=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads(result.stdout)
        self.assertEqual(receipt["phase"], "rolled_back")
        self.assertEqual(sha256(self.nodes["storage-205"] / "current" / "bin" / "oasis7_chain_runtime"), sha256(self.nodes["storage-205"] / "backups" / transaction_id / "snapshot" / "current" / "bin" / "oasis7_chain_runtime"))

    def test_rollback_rejects_tampered_backup_manifest(self) -> None:
        plan = subprocess.run(self._base_args(), text=True, capture_output=True, check=True)
        plan_path = self.out / "transaction.json"
        plan_path.write_text(plan.stdout, encoding="utf-8")
        adapter = self._write_host_adapter()
        applied = subprocess.run(
            [
                sys.executable,
                str(EXECUTOR),
                "apply",
                "--transaction",
                str(plan_path),
                "--host-adapter",
                str(adapter),
            ],
            text=True,
            capture_output=True,
            check=True,
        )
        transaction = json.loads(applied.stdout)
        manifest = Path(transaction["backup"]["storage-205"]["manifest"])
        manifest.write_text(manifest.read_text(encoding="utf-8") + "\n", encoding="utf-8")
        result = subprocess.run(
            [sys.executable, str(EXECUTOR), "rollback", "--transaction", str(plan_path), "--host-adapter", str(adapter)],
            text=True,
            capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("manifest", result.stderr.lower())

    def test_plan_rejects_insufficient_capacity(self) -> None:
        self.capacity.write_text(
            json.dumps(
                {
                    role: {"free_bytes": 1, "free_inodes": 100_000, "same_filesystem": True}
                    for role in self.nodes
                }
            ),
            encoding="utf-8",
        )
        result = subprocess.run(self._base_args(), text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("capacity", result.stderr.lower())

    def test_provenance_create_and_validate_round_trip(self) -> None:
        generated = self.root / "generated-provenance.json"
        create = subprocess.run(
            [
                sys.executable,
                str(PROVENANCE),
                "create",
                "--package-dir",
                str(self.package),
                "--manifest",
                str(self.governed / "manifest.json"),
                "--genesis",
                str(self.governed / "genesis.json"),
                "--registry",
                str(self.governed / "registry.json"),
                "--bootstrap",
                str(self.governed / "bootstrap.txt"),
                "--world",
                str(self.governed / "world.json"),
                "--network-id",
                "oasis7-public-testnet-governed-20260606",
                "--chain-id",
                "oasis7-public-testnet-governed-20260606",
                "--output",
                str(generated),
                "--signer-id",
                "testnet-package-attestor",
                "--signature-ref",
                str(self.signature),
                "--public-key-ref",
                str(self.public_key),
            ],
            text=True,
            capture_output=True,
        )
        self.assertEqual(create.returncode, 0, create.stderr)
        generated_payload = json.loads(generated.read_text(encoding="utf-8"))
        generated_payload["signature"]["status"] = "verified"
        body = {key: value for key, value in generated_payload.items() if key != "binding_digest"}
        generated_payload["binding_digest"] = hashlib.sha256(
            json.dumps(body, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode()
        ).hexdigest()
        generated.write_text(json.dumps(generated_payload, indent=2) + "\n", encoding="utf-8")
        payload_path = self.root / "generated-provenance-signing-payload.bin"
        payload_path.write_bytes(json.dumps(body, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode())
        subprocess.run(
            [
                "openssl",
                "dgst",
                "-sha256",
                "-sign",
                str(self.signing_key),
                "-out",
                str(self.signature),
                str(payload_path),
            ],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        validate = subprocess.run(
            [sys.executable, str(PROVENANCE), "validate", "--provenance", str(generated), "--package-dir", str(self.package)],
            text=True,
            capture_output=True,
        )
        self.assertEqual(validate.returncode, 0, validate.stderr)
        self.assertEqual(json.loads(validate.stdout)["signature"]["signer_id"], "testnet-package-attestor")

    def test_release_bundle_validates_pair_provenance_binding(self) -> None:
        bundle = self.root / "release-bundle.json"
        create = subprocess.run(
            [
                "bash",
                str(RELEASE_BUNDLE),
                "create",
                "--bundle",
                str(bundle),
                "--candidate-id",
                "pair-rebuild-fixture",
                "--runtime-build-ref",
                str(self.package / "oasis7_chain_runtime"),
                "--world-snapshot-ref",
                str(self.governed / "world.json"),
                "--governance-manifest-ref",
                str(self.governed / "manifest.json"),
                "--validator-pair-provenance-ref",
                str(self.provenance),
                "--allow-dirty-worktree",
            ],
            text=True,
            capture_output=True,
        )
        self.assertEqual(create.returncode, 0, create.stderr)
        validate = subprocess.run(
            ["bash", str(RELEASE_BUNDLE), "validate", "--bundle", str(bundle)],
            text=True,
            capture_output=True,
        )
        self.assertEqual(validate.returncode, 0, validate.stderr)

    def test_runtime_verification_receipt_binds_peer_and_exact_raw_proof(self) -> None:
        spec = importlib.util.spec_from_file_location("pair_rebuild_runtime_test", EXECUTOR)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        public_key_hex = "9ea2a4ca4893c1d2fed7482510333464c28a0ddb9db497a1238edb71d4ba286b"
        signature_hex = "22258b4742876b6a997c410b268547b8a40280acbc5ef2ad5a21a4ef034bc463fb728d31ab1ca549e9441c50cf4be587413ece9f9c68067770ce97712096b704"
        claims = {
            "schema_version": "oasis7.rebuild_status.v1",
            "observed_at_unix_ms": 1,
            "node_id": "sequencer-node",
            "world_id": "",
            "ok": True,
            "liveness": {"running": True, "last_error": None},
            "readiness": {"status": "ready", "failed_gates": []},
            "heights": {"committed_height": 1, "network_committed_height": 1, "last_execution_height": 1},
            "network_head": {
                "source": "self",
                "decision": "ready",
                "height": 1,
                "block_hash": "block",
                "execution_block_hash": "execution-block",
                "execution_state_root": "state-root",
                "observed_peer_count": 0,
                "fresh_peer_count": 0,
            },
            "checkpoint": None,
            "local_peer_id": "peer-sequencer",
            "connected_peers": [],
            "connected_peer_count": 0,
        }
        claims_bytes = json.dumps(claims, ensure_ascii=True, separators=(",", ":")).encode()
        self.assertEqual(hashlib.sha256(claims_bytes).hexdigest(), "5acd019055674e080903296b8af0583fec0c86d0243809af187bd2aa11155c76")
        raw_proof = self.root / "raw-rebuild-proof.json"
        raw_value = {
            "schema_version": "oasis7.rebuild_status.v1",
            "observed_at_unix_ms": 1,
            "ok": True,
            "liveness": {"running": True, "last_error": None},
            "readiness": {"status": "ready", "failed_gates": []},
            "heights": {"committed_height": 1, "network_committed_height": 1, "last_execution_height": 1},
            "network_head": {
                "source": "self",
                "decision": "ready",
                "height": 1,
                "block_hash": "block",
                "execution_block_hash": "execution-block",
                "execution_state_root": "state-root",
                "observed_peer_count": 0,
                "fresh_peer_count": 0,
            },
            "checkpoint": None,
            "local_peer_id": "peer-sequencer",
            "connected_peers": [],
            "connected_peer_count": 0,
            "proof": {
                "schema_version": "oasis7.rebuild_proof.v1",
                "signer_id": "sequencer-node",
                "signer_public_key_hex": public_key_hex,
                "signed_payload_sha256": hashlib.sha256(claims_bytes).hexdigest(),
                "signature_hex": signature_hex,
            },
        }
        raw_proof.write_text(json.dumps(raw_value, ensure_ascii=True, separators=(",", ":")), encoding="utf-8")
        receipt = self.root / "runtime-verification-receipt.json"
        receipt_value = {
            "schema_version": "oasis7.rebuild_proof_verification.v1",
            "proof_schema_version": "oasis7.rebuild_proof.v1",
            "signer_id": "sequencer-node",
            "signer_public_key_hex": public_key_hex,
            "signed_payload_sha256": hashlib.sha256(claims_bytes).hexdigest(),
            "local_peer_id": "peer-sequencer",
            "proof_sha256": sha256(raw_proof),
            "verified": True,
        }
        receipt.write_text(json.dumps(receipt_value), encoding="utf-8")
        verifier = self.root / "governed-runtime-verifier.py"
        verifier.write_text(
            "#!/usr/bin/env python3\nimport json\nprint(" + repr(json.dumps(receipt_value)) + ")\n",
            encoding="utf-8",
        )
        verifier.chmod(0o755)
        trusted_root = {"allowlist": [{"signer_id": "sequencer-node", "public_key_hex": public_key_hex}]}
        summary = module.verify_signed_attestation(
            receipt,
            trusted_root,
            "runtime receipt",
            "sequencer-204",
            raw_proof,
            verifier,
        )
        self.assertEqual(summary["peer_id"], "peer-sequencer")
        self.assertEqual(summary["proof_sha256"], sha256(raw_proof))
        forged_verifier = self.root / "forged-runtime-verifier.py"
        forged_receipt = dict(receipt_value)
        forged_receipt["proof_sha256"] = "0" * 64
        forged_verifier.write_text(
            "#!/usr/bin/env python3\nimport json\nprint(" + repr(json.dumps(forged_receipt)) + ")\n",
            encoding="utf-8",
        )
        forged_verifier.chmod(0o755)
        with self.assertRaises(SystemExit):
            module.verify_signed_attestation(
                receipt,
                trusted_root,
                "runtime receipt",
                "sequencer-204",
                raw_proof,
                forged_verifier,
            )
        other_raw_proof = self.root / "other-raw-rebuild-proof.json"
        other_raw_proof.write_bytes(b"another canonical raw signed proof\n")
        with self.assertRaises(SystemExit):
            module.verify_signed_attestation(receipt, trusted_root, "runtime receipt", "sequencer-204", other_raw_proof)
        raw_proof.write_bytes(b"different raw signed proof\n")
        with self.assertRaises(SystemExit):
            module.verify_signed_attestation(receipt, trusted_root, "runtime receipt", "sequencer-204", raw_proof)

    def test_runtime_verification_receipt_rejects_unvalidated_raw_envelope(self) -> None:
        spec = importlib.util.spec_from_file_location("pair_rebuild_runtime_negative_test", EXECUTOR)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        raw_proof = self.root / "malformed-raw-rebuild-proof.json"
        raw_proof.write_text(
            json.dumps({
                "schema_version": "oasis7.rebuild_status.v1",
                "local_peer_id": "peer-sequencer",
                "proof": {
                    "schema_version": "oasis7.rebuild_proof.v1",
                    "signer_id": "sequencer-node",
                    "signer_public_key_hex": "a" * 64,
                    "signed_payload_sha256": "b" * 64,
                    "signature_hex": "c" * 128,
                },
            }),
            encoding="utf-8",
        )
        receipt = self.root / "malformed-runtime-verification-receipt.json"
        receipt.write_text(
            json.dumps({
                "schema_version": "oasis7.rebuild_proof_verification.v1",
                "proof_schema_version": "oasis7.rebuild_proof.v1",
                "signer_id": "sequencer-node",
                "signer_public_key_hex": "a" * 64,
                "signed_payload_sha256": "b" * 64,
                "local_peer_id": "peer-sequencer",
                "proof_sha256": sha256(raw_proof),
                "verified": True,
            }),
            encoding="utf-8",
        )
        trust_root = {"allowlist": [{"signer_id": "sequencer-node", "public_key_hex": "a" * 64}]}
        with self.assertRaises(SystemExit):
            module.verify_signed_attestation(receipt, trust_root, "runtime receipt", "sequencer-204", raw_proof)
        with self.assertRaises(SystemExit):
            module.verify_signed_attestation(receipt, trust_root, "runtime receipt", "sequencer-204")


if __name__ == "__main__":
    unittest.main()
