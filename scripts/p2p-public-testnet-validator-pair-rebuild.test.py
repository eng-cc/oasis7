#!/usr/bin/env python3
"""Contract tests for the governed validator pair rebuild executor.

These tests intentionally exercise only local temporary roots.  They are the
RED contract for task #3318; no live validator, SSH target, or observer is
used.
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
import tempfile
import unittest
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
            "bootstrap.txt": "/ip4/127.0.0.1/tcp/6831/p2p/12D3KooWSequencer\n",
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

    def _write_provenance(self, *, signature: bool = True) -> None:
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

    def _write_host_adapter(self) -> Path:
        adapter = self.root / "host-adapter.py"
        adapter.write_text(
            """#!/usr/bin/env python3
import json
import sys
from pathlib import Path

transaction = json.loads(Path(sys.argv[sys.argv.index('--transaction') + 1]).read_text())
nodes = {}
for role in transaction['mutation_order']:
    nodes[role] = {
        'active': True,
        'running': True,
        'healthz_ok': True,
        'nrestarts': 0,
        'oom_panic_segfault': False,
        'runtime_sha256': transaction['package']['runtime_sha256'],
        'runtime_size_bytes': transaction['package']['runtime_size_bytes'],
        'listeners': ['6631', '6831'] if role == 'sequencer-204' else ['6632', '6832'],
        'full_chain_status_called': False,
    }
print(json.dumps({
    'schema_version': 'oasis7.validator_pair_rebuild_host_receipt.v1',
    'transaction_id': transaction['transaction_id'],
    'mutation_order': transaction['mutation_order'],
    'startup_order': transaction['startup_order'],
    'nodes': nodes,
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
        self.assertEqual(receipt["schema_version"], "oasis7.validator_pair_rebuild_transaction.v1")
        self.assertEqual(receipt["mutation_order"], ["storage-205", "sequencer-204"])
        self.assertEqual(receipt["startup_order"], ["sequencer-204", "storage-205"])
        self.assertEqual(receipt["phase"], "planned")

    def test_plan_rejects_full_status_url_for_204(self) -> None:
        result = subprocess.run(
            self._base_args() + ["--sequencer-proof-url", "http://127.0.0.1:6631/v1/chain/status"],
            text=True,
            capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("204", result.stderr)

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
        transaction["canonical_digest"] = canonical_digest(transaction)
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
        subprocess.run(
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
            check=True,
        )
        self.assertNotEqual(sha256(self.nodes["storage-205"] / "current" / "bin" / "oasis7_chain_runtime"), sha256(self.nodes["storage-205"] / "backups" / json.loads(plan.stdout)["transaction_id"] / "snapshot" / "current" / "bin" / "oasis7_chain_runtime"))
        result = subprocess.run(
            [sys.executable, str(EXECUTOR), "rollback", "--transaction", str(plan_path)],
            text=True,
            capture_output=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads(result.stdout)
        self.assertEqual(receipt["phase"], "rolled_back")
        self.assertEqual(sha256(self.nodes["storage-205"] / "current" / "bin" / "oasis7_chain_runtime"), sha256(self.nodes["storage-205"] / "backups" / json.loads(plan.stdout)["transaction_id"] / "snapshot" / "current" / "bin" / "oasis7_chain_runtime"))

    def test_rollback_rejects_tampered_backup_manifest(self) -> None:
        plan = subprocess.run(self._base_args(), text=True, capture_output=True, check=True)
        plan_path = self.out / "transaction.json"
        plan_path.write_text(plan.stdout, encoding="utf-8")
        applied = subprocess.run(
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
            check=True,
        )
        transaction = json.loads(applied.stdout)
        manifest = Path(transaction["backup"]["storage-205"]["manifest"])
        manifest.write_text(manifest.read_text(encoding="utf-8") + "\n", encoding="utf-8")
        result = subprocess.run(
            [sys.executable, str(EXECUTOR), "rollback", "--transaction", str(plan_path)],
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


if __name__ == "__main__":
    unittest.main()
