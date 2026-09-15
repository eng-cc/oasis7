#!/usr/bin/env python3
"""Validate the public, digest-bound authority for the third-validator candidate.

This is a repository-only contract.  It checks the public identity material
staged on validator-47 and the exact 3x100/quorum/governance semantics without
reading private keys or contacting any host.  The historical two-validator
manifest/genesis remain immutable inputs; the candidate cannot be promoted
until a fresh stage regenerates them and records new bytes/digests.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]
INVENTORY = ROOT / "scripts/public-testnet-validator-triad-inventory.v1.json"
AUTHORITY = ROOT / "doc/testing/evidence/public-testnet-validator-triad-authority-2026-09-15.json"
SOURCE_REGISTRY = ROOT / "doc/testing/evidence/public-testnet-governed-bootstrap-validator-triad-registry-2026-09-15.json"
PEERS = ROOT / "doc/testing/evidence/public-testnet-governed-bootstrap-validator-triad-bootstrap-peers-2026-09-15.txt"
BASE_MANIFEST = ROOT / "doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json"
BASE_GENESIS = ROOT / "doc/testing/evidence/public-testnet-governed-bootstrap-genesis-2026-06-06.json"

WORLD_ID = "oasis7-public-testnet-governed-20260606"
VALIDATOR_47 = {
    "node_id": "triad-testnet-validator-47",
    "stake": 100,
    "root_public_key": "b21137667506c6c9d5eb30e2cefac73950396d1a58e70665cfdb5afec8943ec6",
    "finality_public_key": "cf8c9c2b5637d20d0efa585f0fb7f503b19a1aaba02fb807637e67ed40919fc2",
    "finality_signer_public_key": "cf8c9c2b5637d20d0efa585f0fb7f503b19a1aaba02fb807637e67ed40919fc2",
    "libp2p_peer_id": "12D3KooWCdQLY6Qm9sWqPqEhJTmPdY3Ykw1w5QnTh7qmSgYDazQZ",
}


def load_json(path: Path) -> dict[str, object]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise AssertionError(f"expected object: {path}")
    return value


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class PublicTestnetValidatorTriadAuthorityTest(unittest.TestCase):
    def test_inventory_freezes_public_identity_and_three_equal_validator_policy(self) -> None:
        inventory = load_json(INVENTORY)
        self.assertEqual(inventory["schema_version"], "oasis7.public_testnet_validator_triad_inventory.v1")
        self.assertEqual(inventory["network_tier"], "public_testnet")
        self.assertEqual(inventory["topology"], "three_equal_validator")
        self.assertEqual(
            inventory["validator_set"],
            {
                "count": 3,
                "stakes": [100, 100, 100],
                "total_stake": 300,
                "required_stake": 200,
                "quorum": {"numerator": 2, "denominator": 3},
            },
        )
        self.assertEqual(
            inventory["governance"],
            {"signer_count": 3, "threshold": 2, "threshold_bps": 6667},
        )
        node = inventory["nodes"]["validator-47"]  # type: ignore[index]
        self.assertEqual(node["node_id"], VALIDATOR_47["node_id"])
        self.assertEqual(node["stake"], VALIDATOR_47["stake"])
        for field in ("root_public_key", "finality_public_key", "finality_signer_public_key", "libp2p_peer_id"):
            self.assertEqual(node[field], VALIDATOR_47[field])

    def test_registry_is_exact_three_member_public_finality_authority(self) -> None:
        registry = load_json(SOURCE_REGISTRY)
        self.assertEqual(registry["network_tier"], "public_testnet")
        self.assertEqual(registry["topology"], "three_equal_validator")
        self.assertEqual(registry["threshold"], 2)
        self.assertEqual(registry["quorum"], {"numerator": 2, "denominator": 3, "total_stake": 300, "required_stake": 200})
        self.assertEqual(registry["governance"], {"signer_count": 3, "threshold": 2, "threshold_bps": 6667})
        validators = registry["validators"]
        self.assertEqual(len(validators), 3)
        self.assertEqual([entry["stake"] for entry in validators], [100, 100, 100])
        validator_47 = next(entry for entry in validators if entry["node_id"] == VALIDATOR_47["node_id"])
        for field in ("finality_public_key", "finality_signer_public_key", "root_public_key", "libp2p_peer_id"):
            self.assertEqual(validator_47[field], VALIDATOR_47[field])

    def test_authority_digest_chain_matches_repository_bytes(self) -> None:
        authority = load_json(AUTHORITY)
        self.assertEqual(authority["schema_version"], "oasis7.public_testnet_validator_triad_authority.v1")
        self.assertEqual(authority["status"], "candidate_cold_cutover_hold")
        self.assertEqual(authority["world_id"], WORLD_ID)
        self.assertEqual(authority["chain_id"], WORLD_ID)
        bindings = authority["bindings"]
        expected = {
            "inventory": INVENTORY,
            "source_registry": SOURCE_REGISTRY,
            "manifest": BASE_MANIFEST,
            "genesis": BASE_GENESIS,
            "bootstrap_peers": PEERS,
        }
        for name, path in expected.items():
            self.assertEqual(bindings[name]["sha256"], sha256(path), name)  # type: ignore[index]
        inventory = load_json(INVENTORY)
        inventory_authority = inventory["authority"]
        self.assertEqual(
            bindings["registry"]["sha256"],
            inventory_authority["generated_registry_sha256"],  # type: ignore[index]
        )
        self.assertEqual(
            bindings["registry"]["semantic_sha256"],
            inventory_authority["generated_registry_semantic_sha256"],  # type: ignore[index]
        )
        self.assertEqual(bindings["registry"]["role"], "generated_deployment_registry")  # type: ignore[index]
        self.assertEqual(bindings["source_registry"]["role"], "source_registry_input")  # type: ignore[index]
        self.assertEqual(bindings["manifest"]["role"], "historical_base_input")  # type: ignore[index]
        self.assertTrue(bindings["manifest"]["requires_regeneration_for_triad"])  # type: ignore[index]
        self.assertEqual(bindings["genesis"]["role"], "historical_base_input")  # type: ignore[index]
        self.assertTrue(bindings["genesis"]["requires_regeneration_for_triad"])  # type: ignore[index]

    def test_bootstrap_peer_authority_contains_existing_pair_and_staged_validator(self) -> None:
        peers = PEERS.read_text(encoding="utf-8").splitlines()
        self.assertEqual(len(peers), 3)
        self.assertTrue(any("39.104.204.172/tcp/6831" in peer for peer in peers))
        self.assertTrue(any("39.104.205.67/tcp/6832" in peer for peer in peers))
        self.assertTrue(any("47.111.225.27/tcp/6834" in peer for peer in peers))
        self.assertTrue(any(VALIDATOR_47["libp2p_peer_id"] in peer for peer in peers))

    def test_inventory_authority_refs_and_digests_are_current(self) -> None:
        inventory = load_json(INVENTORY)
        authority = inventory["authority"]
        self.assertEqual(
            authority["source_registry_ref"],
            "doc/testing/evidence/public-testnet-governed-bootstrap-validator-triad-registry-2026-09-15.json",
        )
        self.assertEqual(authority["source_registry_sha256"], sha256(SOURCE_REGISTRY))
        self.assertEqual(
            authority["bootstrap_peer_ref"],
            "doc/testing/evidence/public-testnet-governed-bootstrap-validator-triad-bootstrap-peers-2026-09-15.txt",
        )
        self.assertEqual(authority["bootstrap_peer_sha256"], sha256(PEERS))


if __name__ == "__main__":
    unittest.main(verbosity=2)
