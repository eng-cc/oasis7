#!/usr/bin/env python3
"""Contract tests for deterministic five-node identity-v2 evidence aggregation."""

from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import unittest
from types import SimpleNamespace
from unittest import mock


ROOT = Path(__file__).resolve().parents[1]
AGGREGATOR = ROOT / "scripts" / "p2p-public-testnet-identity-v2-evidence-aggregate.py"
PLANNER = ROOT / "scripts" / "p2p-public-testnet-full-network-clean-room.py"
ADAPTER = ROOT / "scripts" / "p2p-public-testnet-full-network-clean-room-adapter.py"
PLANNER_TEST = ROOT / "scripts" / "p2p-public-testnet-full-network-clean-room.test.py"


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise AssertionError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def write_json(path: Path, value: object) -> None:
    payload = json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")
    path.write_bytes(payload)
    path.chmod(0o600)


def descriptor(path: Path) -> dict[str, object]:
    payload = path.read_bytes()
    return {"path": str(path), "sha256": hashlib.sha256(payload).hexdigest(), "size_bytes": len(payload)}


class EvidenceAggregateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        # Retain the planner fixture's real-crypto cache across aggregate tests;
        # reimporting this module per test creates a fresh fixture class/cache.
        cls.planner_tests = load_module(PLANNER_TEST, "aggregate_planner_tests")
        fixture_class = cls.planner_tests.FullNetworkCleanRoomPlanTests
        cls.addClassCleanup(fixture_class.tearDownClass)
        fixture_class.setUpClass()

    def setUp(self) -> None:
        self.planner = load_module(PLANNER, "aggregate_planner")
        self.adapter = load_module(ADAPTER, "aggregate_adapter")
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        fixture = self.planner_tests.FullNetworkCleanRoomPlanTests("runTest")
        fixture.setUp()
        self.addCleanup(fixture.tearDown)
        (self.root / "artifacts").mkdir(mode=0o700)
        self.full_map, self.request = fixture._network_binding_evidence_fixture(
            self.root / "artifacts", context_network_id=self.planner.CANONICAL_NETWORK_ID
        )
        self.input_paths: list[Path] = []
        for index, entry in enumerate(self.full_map["entries"]):
            path = self.root / f"input-{index}.json"
            single = dict(self.full_map)
            single["entries"] = [copy.deepcopy(entry)]
            write_json(path, single)
            self.input_paths.append(path)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def run_aggregate(self, input_paths: list[Path], output: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(AGGREGATOR), *(arg for path in input_paths for arg in ("--input-map", str(path))), "--out", str(output)],
            text=True,
            capture_output=True,
            check=False,
        )

    def _aggregate_authority_fixture(self, root):
        aggregate = load_module(AGGREGATOR, "aggregate_authority_regression")
        names = ("trust.json", "registry.json", "verifier", "trust-key", "provider-key", "provider-adapter")
        paths = {name: root / name for name in names}
        for path in paths.values():
            path.write_bytes(b"synthetic authority anchor")
            path.chmod(0o600)
        write_json(paths["trust.json"], {"allowlist": [{"public_key_ref": str(paths["trust-key"])}]})
        write_json(paths["registry.json"], {
            "trust_config_path": str(paths["trust.json"]),
            "providers": [{"public_key_ref": str(paths["provider-key"]), "adapter_path": str(paths["provider-adapter"])}],
            "verifier": {"executable_path": str(paths["verifier"])},
        })
        tool = root / "pinned-tool"
        tool.write_bytes(b"#!/bin/sh\nexit 0\n")
        tool.chmod(0o700)
        for stem, path in (("VERIFY_TOOL", tool), ("TRUST_CONFIG", paths["trust.json"]),
                           ("PROVIDER_REGISTRY", paths["registry.json"])):
            setattr(aggregate.PLANNER, f"IDENTITY_V2_{stem}_PATH", path)
            setattr(aggregate.PLANNER, f"IDENTITY_V2_{stem}_SHA256", hashlib.sha256(path.read_bytes()).hexdigest())
        return aggregate, paths

    def _aggregate_main(self, aggregate, output):
        return aggregate.main([*(arg for path in self.input_paths for arg in ("--input-map", str(path))),
                               "--out", str(output)])

    def test_authority_output_aliases_preserve_anchors(self):
        for index, name in enumerate(("trust.json", "registry.json", "verifier", "trust-key", "provider-key", "provider-adapter")):
            for alias in ("direct", "hardlink", "symlink"):
                with self.subTest(anchor=name, alias=alias):
                    root = self.root / f"authority-{index}-{alias}"
                    root.mkdir(mode=0o700)
                    aggregate, paths = self._aggregate_authority_fixture(root)
                    output = paths[name] if alias == "direct" else root / "output-alias"
                    if alias == "hardlink":
                        os.link(paths[name], output)
                    elif alias == "symlink":
                        output.symlink_to(paths[name])
                    before = {path: (path.read_bytes(), stat.S_IMODE(path.stat().st_mode)) for path in paths.values()}
                    entries = set(root.iterdir())
                    with self.assertRaisesRegex(SystemExit, r"alias|output|authority"):
                        self._aggregate_main(aggregate, output)
                    self.assertEqual(before, {path: (path.read_bytes(), stat.S_IMODE(path.stat().st_mode)) for path in paths.values()})
                    self.assertEqual(set(root.iterdir()), entries, "rejection must leave no publication residue")

    def test_authority_closure_failure_blocks_publication(self):
        for failure in ("valid", "missing", "malformed", "pin-mismatch", "relative-reference"):
            with self.subTest(closure=failure):
                root = self.root / f"closure-{failure}"
                root.mkdir(mode=0o700)
                aggregate, paths = self._aggregate_authority_fixture(root)
                registry = paths["registry.json"]
                if failure == "missing":
                    registry.unlink()
                elif failure in ("malformed", "pin-mismatch"):
                    registry.write_bytes(b"not JSON")
                    if failure == "malformed":
                        aggregate.PLANNER.IDENTITY_V2_PROVIDER_REGISTRY_SHA256 = hashlib.sha256(registry.read_bytes()).hexdigest()
                elif failure == "relative-reference":
                    value = json.loads(registry.read_text())
                    value["verifier"]["executable_path"] = "relative-verifier"
                    write_json(registry, value)
                    aggregate.PLANNER.IDENTITY_V2_PROVIDER_REGISTRY_SHA256 = hashlib.sha256(registry.read_bytes()).hexdigest()
                output = root / "aggregate.json"
                output.write_bytes(b"existing output")
                output.chmod(0o600)
                before = {path: (path.read_bytes(), stat.S_IMODE(path.stat().st_mode)) for path in root.iterdir()}
                if failure == "valid":
                    self.assertEqual(self._aggregate_main(aggregate, output), 0)
                    self.assertEqual(json.loads(output.read_text())["schema_version"], aggregate.IDENTITY_V2_EVIDENCE_SCHEMA)
                    self.assertEqual(stat.S_IMODE(output.stat().st_mode), 0o600)
                else:
                    with self.assertRaises(SystemExit):
                        self._aggregate_main(aggregate, output)
                    self.assertEqual(before, {path: (path.read_bytes(), stat.S_IMODE(path.stat().st_mode)) for path in root.iterdir()})

    def test_aggregate_five_maps_is_canonical_repeatable_and_preserves_inputs(self) -> None:
        authority_root = self.root / "happy-path-authority"
        authority_root.mkdir(mode=0o700)
        module, _ = self._aggregate_authority_fixture(authority_root)
        before = {path: hashlib.sha256(path.read_bytes()).hexdigest() for path in self.input_paths}
        output_a = self.root / "out-a.json"
        output_b = self.root / "out-b.json"
        self.assertEqual(module.main([
            *(arg for path in reversed(self.input_paths) for arg in ("--input-map", str(path))),
            "--out", str(output_a),
        ]), 0)
        self.assertEqual(self._aggregate_main(module, output_b), 0)
        self.assertEqual(output_a.read_bytes(), output_b.read_bytes())
        aggregate = json.loads(output_a.read_text(encoding="utf-8"))
        self.assertEqual([entry["node_name"] for entry in aggregate["entries"]], list(self.planner.NODE_ORDER))
        self.assertEqual(stat.S_IMODE(output_a.stat().st_mode), 0o600)
        self.assertEqual(self.planner._identity_v2_evidence_map(aggregate, self.request)[0], aggregate)
        self.assertEqual(self.adapter.IDENTITY_V2_EVIDENCE_SCHEMA, self.planner.IDENTITY_V2_EVIDENCE_SCHEMA)
        adapter_admission = self.adapter._current_identity_v2_admission(
            {
                "identity_v2_evidence": aggregate,
                "plan_digest": "plan-digest-fixture",
                "transaction_id": "transaction-fixture",
            },
            {"apply_authorized": False},
            aggregate,
            "current_admission",
        )
        self.assertEqual(adapter_admission["status"], "identity-v2-admission-validated")
        self.assertEqual(adapter_admission["identity_v2_mode"], "current_admission")
        self.assertEqual(before, {path: hashlib.sha256(path.read_bytes()).hexdigest() for path in self.input_paths})

    def test_rejects_missing_duplicate_unexpected_and_metadata_mismatch(self) -> None:
        output = self.root / "rejected.json"
        self.assertNotEqual(self.run_aggregate(self.input_paths[:4], output).returncode, 0)

        duplicate = list(self.input_paths)
        duplicate[1] = self._rewrite_map("duplicate.json", duplicate[0], lambda value: value)
        self.assertNotEqual(self.run_aggregate(duplicate, output).returncode, 0)

        unexpected = self._rewrite_map(
            "unexpected.json", self.input_paths[0], lambda value: self._change_entry(value, node_name="attacker-node")
        )
        self.assertNotEqual(self.run_aggregate([unexpected, *self.input_paths[1:]], output).returncode, 0)

        mixed = self._rewrite_map("mixed.json", self.input_paths[0], lambda value: self._change_top(value, task_uid="other-task"))
        self.assertNotEqual(self.run_aggregate([mixed, *self.input_paths[1:]], output).returncode, 0)

        context_path = self.root / "different-context.json"
        context_path.write_bytes(b'{"capture_window_id":"different-window","network_id":"oasis7-public-testnet-governed-20260606"}')
        context_path.chmod(0o600)
        mixed_context = self._rewrite_map(
            "mixed-context.json", self.input_paths[0], lambda value: self._change_top(value, context=descriptor(context_path))
        )
        self.assertNotEqual(self.run_aggregate([mixed_context, *self.input_paths[1:]], output).returncode, 0)

    def test_rejects_artifact_cross_pairing_without_touching_existing_output(self) -> None:
        output = self.root / "sentinel.json"
        output.write_bytes(b"sentinel")
        output.chmod(0o600)
        cross_paired = self._rewrite_map(
            "cross-paired.json",
            self.input_paths[0],
            lambda value: self._change_entry(value, raw_v1=self.full_map["entries"][1]["raw_v1"]),
        )
        result = self.run_aggregate([cross_paired, *self.input_paths[1:]], output)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(output.read_bytes(), b"sentinel")

    def test_rejects_output_aliases_to_retained_evidence_and_preserves_bytes(self) -> None:
        """Publication cannot replace any retained descriptor or inode alias."""
        retained_paths = [
            Path(self.full_map["context"]["path"]),
            Path(self.full_map["plan_intent"]["path"]),
            *(
                Path(entry[artifact]["path"])
                for entry in self.full_map["entries"]
                for artifact in self.planner.IDENTITY_V2_EVIDENCE_ARTIFACT_FIELDS
            ),
        ]
        for retained in retained_paths:
            with self.subTest(alias=retained.name):
                before = retained.read_bytes()
                result = self.run_aggregate(self.input_paths, retained)
                self.assertNotEqual(result.returncode, 0, result.stderr)
                self.assertRegex(result.stderr.lower(), r"alias|retained|output")
                self.assertEqual(retained.read_bytes(), before)

        input_alias = self.input_paths[0]
        before = input_alias.read_bytes()
        result = self.run_aggregate(self.input_paths, input_alias)
        self.assertNotEqual(result.returncode, 0, result.stderr)
        self.assertRegex(result.stderr.lower(), r"alias|input|output")
        self.assertEqual(input_alias.read_bytes(), before)

        context = Path(self.full_map["context"]["path"])
        symlink = self.root / "context-output-symlink.json"
        symlink.symlink_to(context)
        before = context.read_bytes()
        result = self.run_aggregate(self.input_paths, symlink)
        self.assertNotEqual(result.returncode, 0, result.stderr)
        self.assertRegex(result.stderr.lower(), r"alias|retained|output")
        self.assertEqual(context.read_bytes(), before)

        hardlink = self.root / "context-output-hardlink.json"
        os.link(context, hardlink)
        before = context.read_bytes()
        result = self.run_aggregate(self.input_paths, hardlink)
        self.assertNotEqual(result.returncode, 0, result.stderr)
        self.assertRegex(result.stderr.lower(), r"alias|retained|inode|output")
        self.assertEqual(context.read_bytes(), before)
        self.assertEqual(hardlink.read_bytes(), before)

    def _rewrite_map(self, name: str, source: Path, transform) -> Path:
        value = json.loads(source.read_text(encoding="utf-8"))
        return_value = transform(value)
        if return_value is not None:
            value = return_value
        path = self.root / name
        write_json(path, value)
        return path

    @staticmethod
    def _change_entry(value: dict[str, object], **changes: object) -> dict[str, object]:
        value = copy.deepcopy(value)
        value["entries"][0].update(changes)  # type: ignore[index]
        return value

    @staticmethod
    def _change_top(value: dict[str, object], **changes: object) -> dict[str, object]:
        value = copy.deepcopy(value)
        value.update(changes)
        return value


class AggregateFixtureLifecycleTests(unittest.TestCase):
    def test_two_setups_share_baseline_but_isolate_mutations_and_cleanup(self) -> None:
        initializations = []
        fixtures = []

        def fake_load(path, name):
            if path != PLANNER_TEST:
                return SimpleNamespace(CANONICAL_NETWORK_ID="fixture-network")

            class Fixture:
                ready = False

                @classmethod
                def setUpClass(cls):
                    if not cls.ready:
                        initializations.append(cls)
                        cls.ready = True

                @classmethod
                def tearDownClass(cls):
                    cls.ready = False

                def __init__(self, method):
                    self.cleaned = False
                    fixtures.append(self)

                def setUp(self):
                    type(self).setUpClass()

                def tearDown(self):
                    self.cleaned = True

                def _network_binding_evidence_fixture(self, root, **kwargs):
                    return {"entries": [{"node_name": str(i)} for i in range(5)]}, {}

            return SimpleNamespace(FullNetworkCleanRoomPlanTests=Fixture)

        class Lifecycle(EvidenceAggregateTests):
            pass

        with mock.patch.dict(globals(), load_module=fake_load):
            Lifecycle.setUpClass()
            self.addCleanup(Lifecycle.doClassCleanups)
            first, second = Lifecycle("runTest"), Lifecycle("runTest")
            self.addCleanup(first.doCleanups)
            self.addCleanup(second.doCleanups)
            first.setUp()
            self.addCleanup(first.tearDown)
            first.full_map["entries"][0]["node_name"] = "mutated"
            first.planner.CANONICAL_NETWORK_ID = "mutated"
            second.setUp()
            self.addCleanup(second.tearDown)
            self.assertEqual(len(initializations), 1, "crypto baseline must initialize once per aggregate class")
            self.assertIs(first.planner_tests, second.planner_tests)
            self.assertIsNot(first.planner, second.planner)
            self.assertIsNot(first.adapter, second.adapter)
            self.assertEqual(second.planner.CANONICAL_NETWORK_ID, "fixture-network")
            self.assertEqual(second.full_map["entries"][0]["node_name"], "0")
            self.assertNotEqual(first.root, second.root)
            first.tearDown()
            first.doCleanups()
            self.assertFalse(first.root.exists())
            self.assertTrue(second.input_paths[0].exists())
            self.assertTrue(fixtures[0].cleaned)
            self.assertFalse(fixtures[1].cleaned)
            second.tearDown()
            second.doCleanups()
            self.assertFalse(second.root.exists())
            self.assertTrue(fixtures[1].cleaned)
            Lifecycle.doClassCleanups()
            self.assertFalse(initializations[0].ready)


if __name__ == "__main__":
    unittest.main()
