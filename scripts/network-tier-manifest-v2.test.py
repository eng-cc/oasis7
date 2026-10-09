#!/usr/bin/env python3
"""Policy validation and fail-closed legacy consumers for planned persistent worlds."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
V1 = ROOT / "doc/testing/templates/network-tier-public-testnet.example.json"
V2 = ROOT / "doc/testing/templates/network-tier-persistent-preview-planned.example.json"

class PlannedManifestTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.path = Path(self.temp.name) / "manifest.json"

    def write(self, data):
        # Keep file references relative to the original template directory.
        for key in ("genesis_ref", "bootstrap_peer_ref"):
            data["runtime_refs"][key] = str((V2.parent / data["runtime_refs"][key]).resolve())
        self.path.write_text(json.dumps(data))

    def run_script(self, script, *args):
        return subprocess.run([str(ROOT / "scripts" / script), *args], cwd=ROOT, text=True, capture_output=True)

    def test_v1_cannot_smuggle_v2_policies(self):
        for field in ("release_policy", "world_policy", "authority_policy"):
            for value in (None, json.loads(V2.read_text())[field]):
                with self.subTest(field=field, value=value):
                    data = json.loads(V1.read_text())
                    data[field] = value
                    self.write(data)
                    result = self.run_script("network-tier-manifest.sh", "validate", "--manifest", str(self.path))
                    self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_v2_duplicate_policy_fields_rejected(self):
        self.write(json.loads(V2.read_text()))
        source = self.path.read_text()
        for original, duplicate in (
            ('"profile_version": 1', '"profile_version": 0, "profile_version": 1'),
            ('"activation": "planned"', '"activation": "live", "activation": "planned"'),
            ('"schema_version": "oasis7.network_tier_manifest.v2"', '"schema_version": "oasis7.network_tier_manifest.v1", "schema_version": "oasis7.network_tier_manifest.v2"'),
        ):
            with self.subTest(original=original):
                self.assertIn(original, source)
                self.path.write_text(source.replace(original, duplicate))
                result = self.run_script("network-tier-manifest.sh", "validate", "--manifest", str(self.path))
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertIn("duplicate", result.stderr)

    def test_v2_planned_valid_and_never_ready(self):
        self.write(json.loads(V2.read_text()))
        result = self.run_script("network-tier-manifest.sh", "validate", "--manifest", str(self.path))
        self.assertEqual(result.returncode, 0, result.stderr)
        validated = json.loads(result.stdout)
        self.assertEqual(validated["validate_result"], "pass")
        self.assertTrue(validated["schema_valid"])
        self.assertFalse(validated["runtime_supported"])
        self.assertEqual(validated["authority_activation"], "planned")
        result = self.run_script("network-tier-exit-review.sh", "--manifest", str(self.path))
        self.assertEqual(result.returncode, 0, result.stderr)
        exit_summary = json.loads(result.stdout)
        self.assertEqual(exit_summary["exit_review_readiness"], "planned_authority_not_activated")
        self.assertFalse(exit_summary["runtime_supported"])
        self.assertEqual(exit_summary["manifest_schema_version"], "oasis7.network_tier_manifest.v2")
        for field in ("release_policy", "world_policy", "authority_policy"):
            self.assertEqual(exit_summary[field], json.loads(V2.read_text())[field])
        result = self.run_script("network-tier-public-testnet-readiness.sh", "--manifest", str(self.path), "--out-dir", str(Path(self.temp.name) / "readiness"))
        self.assertEqual(result.returncode, 0, result.stderr)
        summary = json.loads(result.stdout)
        self.assertFalse(summary["live_candidate_allowed"])
        self.assertEqual(summary["readiness_verdict"], "block")
        self.assertFalse(summary["runtime_supported"])
        self.assertEqual(summary["manifest_schema_version"], "oasis7.network_tier_manifest.v2")
        for field in ("release_policy", "world_policy", "authority_policy"):
            self.assertEqual(summary[field], json.loads(V2.read_text())[field])
        self.assertIn("planned_authority_not_activated", summary["manifest_blockers"])

    def test_v2_cannot_reuse_legacy_proof_consumers(self):
        self.write(json.loads(V2.read_text()))
        for script in ("network-tier-external-verifier-light-client-lite.sh", "network-tier-validator-finality-proof.sh", "network-tier-light-client-continuity-window.sh"):
            with self.subTest(script=script):
                # Must reject the authority declaration before other proof work.
                result = self.run_script(script, "--manifest", str(self.path))
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("planned authority is not activated", result.stderr)
        lanes = Path(self.temp.name) / "green-lanes.tsv"
        lanes.write_text("public_rpc_ready\truntime_engineer\tpass\tlegacy-proof.json\tall green legacy evidence\n")
        result = self.run_script("network-tier-public-testnet-readiness.sh", "--manifest", str(self.path), "--lanes-tsv", str(lanes), "--out-dir", str(Path(self.temp.name) / "green-readiness"))
        self.assertEqual(result.returncode, 0, result.stderr)
        summary = json.loads(result.stdout)
        self.assertFalse(summary["live_candidate_allowed"])
        self.assertEqual(summary["readiness_verdict"], "block")
        self.assertEqual(summary["lane_count"], 0, "legacy pass lanes cannot verify planned authority")

    def test_v2_never_enters_deployment(self):
        self.write(json.loads(V2.read_text()))
        target = Path(self.temp.name) / "must-not-exist"
        for script, args in (
            ("p2p-public-testnet-local-node-install.sh", ["--source-manifest", str(self.path), "--node-root", str(target)]),
            ("p2p-public-testnet-local-observer-sync.sh", ["apply", "--manifest-source", str(self.path), "--manifest-path", str(target / "manifest.json"), "--backup-dir", str(target)]),
        ):
            with self.subTest(script=script):
                result = self.run_script(script, *args)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("planned authority is not activated", result.stderr)
                self.assertFalse(target.exists())
        stage = Path(self.temp.name) / "stage"
        config = stage / "config"
        config.mkdir(parents=True)
        for name in ("bundle", "genesis", "manifest"):
            value = json.loads(self.path.read_text()) if name == "manifest" else {}
            (config / f"public-testnet-governed-bootstrap-{name}-2026-06-06.json").write_text(json.dumps(value))
        (config / "public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt").write_text("placeholder")
        runtime = Path(self.temp.name) / "runtime.exe"
        runtime.write_text("must not run")
        result = self.run_script("p2p-public-testnet-build-deployment-stage.sh", "--runtime-build-ref", str(runtime),
            "--bootstrap-peers-file", str(V2.parent / "public-testnet-bootstrap.example.txt"),
            "--base-manifest", str(self.path), "--out-dir", str(target))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("planned authority is not activated", result.stderr)
        self.assertFalse(target.exists())
        result = subprocess.run(["python3", str(ROOT / "scripts/p2p-public-testnet-stage-windows-governed-closure.py"), "--stage-dir", str(stage), "--runtime-build-ref", str(runtime), "--out-dir", str(target), "--bash-executable", "bash"], cwd=ROOT, text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("planned authority is not activated", result.stderr)
        self.assertFalse(target.exists())

    def test_v2_contradictory_live_claims_rejected(self):
        for claim in ("mainnet_live", "MAINNET_READY", "production_oc_settlement", "controlled_single_authority_live", "persistent_world_live", "distributed_finality"):
            with self.subTest(claim=claim):
                data = json.loads(V2.read_text())
                data["claims_policy"]["allowed_claims"].append(claim)
                self.write(data)
                result = self.run_script("network-tier-manifest.sh", "validate", "--manifest", str(self.path))
                self.assertNotEqual(result.returncode, 0)

    def test_v2_create_roundtrip(self):
        result = self.run_script("network-tier-manifest.sh", "create", "--manifest", str(self.path),
            "--schema-version", "oasis7.network_tier_manifest.v2", "--tier", "public_testnet", "--status", "planned",
            "--network-id", "example", "--chain-id", "oasis7-public-testnet-example",
            "--release-candidate-bundle-ref", "planned-bundle.json",
            "--genesis-ref", str(V2.parent / "public-testnet-genesis.example.json"),
            "--bootstrap-peer-ref", str(V2.parent / "public-testnet-bootstrap.example.txt"),
            "--rpc-ref", "https://example.invalid/rpc", "--explorer-ref", "https://example.invalid/explorer",
            "--governance-mode", "shared_ops", "--validator-admission", "shared_allowlist", "--target-validator-count", "1",
            "--allow-observer-nodes", "true", "--faucet-mode", "operator_grant", "--value-semantics", "preview",
            "--reset-policy", "frozen", "--release-stage", "limited_preview", "--world-id", "oasis7-public-testnet-example",
            "--world-retention", "persistent", "--authority-profile", "controlled_single_authority",
            "--authority-profile-version", "1", "--authority-activation", "planned",
            "--allowed-claim", "public_testnet", "--denied-claim", "mainnet_live", "--denied-claim", "production_oc_settlement")
        self.assertEqual(result.returncode, 0, result.stderr)
        data = json.loads(self.path.read_text())
        self.assertNotIn("reset_policy", data["token_policy"])
        self.assertEqual(data["world_policy"]["reset_policy"], "frozen")
        self.assertEqual(data["authority_policy"]["activation"], "planned")

    def test_v2_unsupported_combinations_rejected(self):
        mutations = [
            ("status", "live"), ("tier", "mainnet"),
            ("release_policy.stage", "production"),
            ("world_policy.world_id", "other-world"), ("world_policy.world_id", " "), ("world_policy.reset_policy", "resettable"),
            ("world_policy.retention", "ephemeral"),
            ("authority_policy.activation", "active"), ("authority_policy.profile", "bft"),
            ("authority_policy.profile_version", True), ("authority_policy.profile_version", "1"), ("authority_policy.profile_version", 1.0), ("authority_policy.profile_version", 2),
            ("validator_policy.target_validator_count", True), ("validator_policy.target_validator_count", 2**64), ("token_policy.symbol", " "),
            ("endpoint_policy.faucet_ref", ""),
            ("release_policy.extra", "ignored"), ("world_policy.extra", "ignored"), ("authority_policy.extra", "ignored"),
            ("token_policy.reset_policy", None), ("token_policy.reset_policy", "frozen"), ("token_policy.value_semantics", "production"),
            ("token_policy.faucet_mode", "guarded_testnet_faucet"),
        ]
        for field, value in mutations:
            with self.subTest(field=field, value=value):
                data = json.loads(V2.read_text())
                parts = field.split(".")
                obj = data if len(parts) == 1 else data[parts[0]]
                obj[parts[-1]] = value
                self.write(data)
                result = self.run_script("network-tier-manifest.sh", "validate", "--manifest", str(self.path))
                self.assertNotEqual(result.returncode, 0, result.stdout)

if __name__ == "__main__":
    unittest.main()
