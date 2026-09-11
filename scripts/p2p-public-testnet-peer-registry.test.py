#!/usr/bin/env python3
"""Behavioral admission regressions for governed peer-registry authority."""

import hashlib
import importlib.util
import copy
import json
import os
from pathlib import Path
import stat
import tempfile
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parent


def load(name):
    path = ROOT / name
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class PeerRegistryAdmissionTests(unittest.TestCase):
    def test_ancestor_write_modes_reject_with_protected_and_sticky_controls(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            leaf = root / "authority"
            leaf.mkdir(mode=0o700)
            _, registry, _ = self._fixture(leaf)
            expected = registry.REGISTRY_SHA256
            real_stat = Path.stat
            for level in (leaf, root):
                for mode in (0o700, 0o770, 0o702, 0o777, 0o1777):
                    with self.subTest(level=level.name, mode=oct(mode)):
                        level.chmod(mode)
                        try:
                            if mode == 0o700 or (mode == 0o1777 and os.getuid() == 0):
                                self.assertEqual(registry.load_snapshot()["sha256"], expected)
                            else:
                                # Operator-owned sticky directories are not the
                                # root-owned sticky exception.
                                with self.assertRaisesRegex(SystemExit, r"ancestor|director|writ"):
                                    registry.load_snapshot()
                        finally:
                            level.chmod(0o700)
                def root_sticky(path, *args, **kwargs):
                    result = real_stat(path, *args, **kwargs)
                    if path == level:
                        fields = list(result)
                        fields[0], fields[4] = stat.S_IFDIR | 0o1777, 0
                        return os.stat_result(fields)
                    return result
                with self.subTest(level=level.name, control="root-owned-sticky"), mock.patch.object(Path, "stat", root_sticky):
                    self.assertEqual(registry.load_snapshot()["sha256"], expected)

    def test_ancestor_owner_type_and_stat_failures_reject_with_alias_controls(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            leaf = root / "authority"
            leaf.mkdir(mode=0o700)
            _, registry, _ = self._fixture(leaf)
            real_stat = Path.stat
            for level in (leaf, root):
                for corruption in ("foreign-owner", "non-directory", "stat-failure"):
                    def corrupt_stat(path, *args, **kwargs):
                        if path == level and corruption == "stat-failure":
                            raise PermissionError("synthetic ancestor stat failure")
                        result = real_stat(path, *args, **kwargs)
                        if path == level:
                            fields = list(result)
                            if corruption == "foreign-owner":
                                fields[4] = os.getuid() + 1000
                            elif corruption == "non-directory":
                                fields[0] = stat.S_IFREG | 0o700
                            return os.stat_result(fields)
                        return result
                    with self.subTest(level=level.name, corruption=corruption), mock.patch.object(Path, "stat", corrupt_stat):
                        with self.assertRaisesRegex(SystemExit, r"ancestor|director|stat|unavailable"):
                            registry.load_snapshot()
            self.assertEqual(registry.load_snapshot()["sha256"], registry.REGISTRY_SHA256)
        # Exercise both fixed aliases in actual ancestor chains; force their
        # symlink classification so the control is portable across POSIX hosts.
        for alias, temporary_parent in ((Path("/tmp"), "/tmp"), (Path("/var"), "/var/tmp")):
            with self.subTest(control="fixed-system-alias", alias=str(alias)), tempfile.TemporaryDirectory(dir=temporary_parent) as directory:
                _, registry, _ = self._fixture(Path(directory))
                real_is_symlink = Path.is_symlink
                def fixed_alias(path):
                    return path == alias or real_is_symlink(path)
                with mock.patch.object(Path, "is_symlink", fixed_alias):
                    self.assertEqual(registry.load_snapshot()["sha256"], registry.REGISTRY_SHA256)

    def _fixture(self, root):
        planner = load("p2p-public-testnet-full-network-clean-room.py")
        registry = planner._peer_registry_authority()
        registry.REGISTRY_PATH = root / "peers.json"
        value = {"schema_version": registry.SCHEMA, "network_id": registry.NETWORK_ID,
                 "registry_epoch": "peer-epoch-1", "nodes": [
                     {"node_name": name, "node_id": node_id, "peer_id": f"FixturePeer{index}"}
                     for index, (name, node_id) in enumerate(registry.NODE_IDS.items())]}
        self._pin(registry, value)
        return planner, registry, value

    @staticmethod
    def _pin(registry, value):
        raw = json.dumps(value, sort_keys=True).encode()
        registry.REGISTRY_PATH.write_bytes(raw)
        registry.REGISTRY_PATH.chmod(0o600)
        registry.REGISTRY_SHA256 = hashlib.sha256(raw).hexdigest()

    def test_snapshot_admission_controls_and_current_v2_intent(self):
        with tempfile.TemporaryDirectory() as directory:
            planner, registry, value = self._fixture(Path(directory))
            snapshot = registry.load_snapshot()
            self.assertEqual(dict(snapshot["peers"]), {node["node_name"]: node["peer_id"] for node in value["nodes"]})
            with self.assertRaises(TypeError):
                snapshot["peers"]["storage-205"] = "tamper"
            for corruption in ("missing-node", "duplicate-peer", "wrong-node-id", "wrong-network", "extra-field", "bad-epoch", "pin", "mode", "owner"):
                with self.subTest(corruption=corruption):
                    candidate = copy.deepcopy(value)
                    if corruption == "missing-node": candidate["nodes"].pop()
                    elif corruption == "duplicate-peer": candidate["nodes"][1]["peer_id"] = candidate["nodes"][0]["peer_id"]
                    elif corruption == "wrong-node-id": candidate["nodes"][0]["node_id"] = "wrong"
                    elif corruption == "wrong-network": candidate["network_id"] = "wrong"
                    elif corruption == "extra-field": candidate["override"] = True
                    elif corruption == "bad-epoch": candidate["registry_epoch"] = ""
                    self._pin(registry, candidate)
                    if corruption == "pin": registry.REGISTRY_SHA256 = "a" * 64
                    if corruption == "mode": registry.REGISTRY_PATH.chmod(0o644)
                    with mock.patch.object(registry.os, "getuid", return_value=os.getuid() + 1) if corruption == "owner" else mock.patch.dict({}, {}):
                        with self.assertRaises(SystemExit): registry.load_snapshot()
            for corruption in ("missing", "malformed-json", "duplicate-json-field", "symlink"):
                with self.subTest(corruption=corruption):
                    self._pin(registry, value)
                    if corruption == "missing": registry.REGISTRY_PATH.unlink()
                    elif corruption == "symlink":
                        retained = Path(directory) / "retained-registry"
                        registry.REGISTRY_PATH.rename(retained)
                        registry.REGISTRY_PATH.symlink_to(retained)
                    else:
                        raw = b"not-json" if corruption == "malformed-json" else b'{"schema_version":"one","schema_version":"two"}'
                        registry.REGISTRY_PATH.write_bytes(raw)
                        registry.REGISTRY_SHA256 = hashlib.sha256(raw).hexdigest()
                    with self.assertRaises(SystemExit): registry.load_snapshot()
                    if registry.REGISTRY_PATH.is_symlink(): registry.REGISTRY_PATH.unlink()
            self._pin(registry, value)
            intent = planner._canonical_plan_intent("a" * 64)
            self.assertEqual(intent["schema_version"], "oasis7.clean_room_plan_intent.v2")
            self.assertEqual(intent["peer_registry_sha256"], registry.REGISTRY_SHA256)
            self.assertEqual(intent["peer_registry_epoch"], value["registry_epoch"])
            self.assertEqual({node["node_name"]: node["peer_id"] for node in intent["nodes"]}, dict(snapshot["peers"]))

    def test_signing_current_snapshot_binding_and_rotation(self):
        with tempfile.TemporaryDirectory() as directory:
            planner, registry, value = self._fixture(Path(directory))
            signing = load("p2p-public-testnet-identity-v2-signing-tool.py")
            signing._PEER_REGISTRY_MODULE = registry
            context = {"network_id": registry.NETWORK_ID, "rotation_epoch": "distinct-key-epoch"}
            intent = {"schema_version": "oasis7.clean_room_plan_intent.v2",
                      "context_digest": hashlib.sha256(signing.canonical(context)).hexdigest(),
                      "adapter_action": planner.CANONICAL_PLAN_INTENT_ACTION,
                      "peer_registry_sha256": registry.REGISTRY_SHA256,
                      "peer_registry_epoch": value["registry_epoch"],
                      "nodes": [{"node_name": name, "node_id": registry.NODE_IDS[name],
                                 "peer_id": peer, "role": planner.EXPECTED_NODES[name]["role"],
                                 "reset_surface_ids": list(planner.CANONICAL_PLAN_INTENT_RESET_SURFACE_IDS)}
                                for name, peer in sorted(registry.load_snapshot()["peers"].items())]}
            with self.subTest(admission="valid-current"):
                try:
                    admitted = signing.validate_intent(intent, context)
                except signing.ToolError as error:
                    self.fail(f"current pinned v2 intent was rejected: {error}")
                self.assertEqual(admitted, intent)
            for field, replacement in (("peer_registry_sha256", "b" * 64), ("peer_registry_epoch", "old-peer-epoch")):
                tampered = copy.deepcopy(intent)
                tampered[field] = replacement
                with self.assertRaises(signing.ToolError): signing.validate_intent(tampered, context)
            value["registry_epoch"] = "peer-epoch-2"
            value["nodes"][0]["peer_id"] = "RotatedFixturePeer"
            self._pin(registry, value)
            with self.assertRaises(signing.ToolError): signing.validate_intent(intent, context)

    def test_all_writer_peer_registry_aliases(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            planner, registry, _ = self._fixture(root)
            trust, providers = root / "trust.json", root / "providers.json"
            trust.write_text(json.dumps({"allowlist": [{"public_key_ref": str(root / "key") }]}))
            providers.write_text(json.dumps({"trust_config_path": str(trust),
                "providers": [{"adapter_path": str(root / "provider"), "public_key_ref": str(root / "key")}],
                "verifier": {"executable_path": str(root / "verifier")}}))
            planner.IDENTITY_V2_TRUST_CONFIG_PATH = trust
            planner.IDENTITY_V2_PROVIDER_REGISTRY_PATH = providers
            adapter = load("p2p-public-testnet-full-network-clean-room-adapter.py")
            adapter._PLANNER_MODULE = planner
            aggregate = load("p2p-public-testnet-identity-v2-evidence-aggregate.py")
            aggregate.PLANNER = planner
            sidecar = load("p2p-public-testnet-identity-receipt-v2.py")
            sidecar._load_planner = lambda: planner
            signing = load("p2p-public-testnet-identity-v2-signing-tool.py")
            signing._PEER_REGISTRY_MODULE = registry
            guards = {
                "planner": lambda path: planner._reject_plan_output_aliases(path, planner._plan_output_inputs(root / "input", root / "evidence", {})),
                "adapter": lambda path: adapter._reject_journal_input_aliases(path, root / "ledger", {}),
                "aggregate": lambda path: aggregate._check_output_collisions(path, []),
                "sidecar": lambda path: sidecar._reject_output_aliases([(path, "output")], []),
                "signing": lambda path: signing._reject_output_aliases([(path, "output")], []),
            }
            for writer, guard in guards.items():
                for target in registry.protected_paths():
                    for kind in ("direct", "hardlink", "symlink"):
                        with self.subTest(writer=writer, target=target.name, alias=kind):
                            output = target if kind == "direct" else root / f"{writer}-{target.name}-{kind}"
                            if kind == "hardlink": os.link(target, output)
                            elif kind == "symlink": output.symlink_to(target)
                            before = target.read_bytes()
                            with self.assertRaisesRegex((SystemExit, adapter.AdapterError, signing.ToolError), r"alias|symlink|protected"):
                                guard(output)
                            self.assertEqual(target.read_bytes(), before)

    def test_unprovisioned_planner_cannot_emit_synthetic_peer_intent(self):
        planner = load("p2p-public-testnet-full-network-clean-room.py")
        with self.assertRaisesRegex(SystemExit, r"peer|registry|provision"):
            planner._canonical_plan_intent("a" * 64)

    def test_signing_rejects_legacy_intent_without_registry_binding(self):
        signing = load("p2p-public-testnet-identity-v2-signing-tool.py")
        context = {"network_id": "synthetic-network", "rotation_epoch": "key-epoch"}
        intent = {
            "schema_version": "oasis7.clean_room_plan_intent.v1",
            "context_digest": hashlib.sha256(signing.canonical(context)).hexdigest(),
            "adapter_action": "public-testnet-governed-rebuild",
            "nodes": [{"node_name": "synthetic-node", "node_id": "synthetic-node",
                       "peer_id": "synthetic-peer", "role": "validator", "reset_surface_ids": ["world"]}],
        }
        with self.assertRaisesRegex(signing.ToolError, r"intent|schema|registry"):
            signing.validate_intent(intent, context)

    def test_planner_output_inventory_protects_shared_peer_authority(self):
        planner = load("p2p-public-testnet-full-network-clean-room.py")
        protected = planner._plan_output_inputs(ROOT / "input", ROOT / "evidence", {})
        for target in (ROOT / "p2p-public-testnet-peer-registry.py",
                       Path("/operator/truth/managed-peer-registry.json")):
            with self.subTest(target=str(target)):
                with self.assertRaisesRegex(SystemExit, r"alias|authority"):
                    planner._reject_plan_output_aliases(target, protected)

    def test_sidecar_output_guard_protects_shared_peer_authority(self):
        sidecar = load("p2p-public-testnet-identity-receipt-v2.py")
        for target in (ROOT / "p2p-public-testnet-peer-registry.py",
                       Path("/operator/truth/managed-peer-registry.json")):
            with self.subTest(target=str(target)):
                with self.assertRaisesRegex(SystemExit, r"alias|protected"):
                    sidecar._reject_output_aliases([(target, "output")], [])


if __name__ == "__main__":
    unittest.main()
