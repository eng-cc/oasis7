#!/usr/bin/env python3
"""Focused closure-localization tests for the Windows Testnet Package helper."""
from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("p2p-public-testnet-stage-windows-governed-closure.py")
SPEC = importlib.util.spec_from_file_location("windows_governed_closure", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
ROLLOUT = Path(__file__).with_name("p2p-public-testnet-package-rollout.py")
ROLLOUT_SPEC = importlib.util.spec_from_file_location("package_rollout", ROLLOUT)
assert ROLLOUT_SPEC and ROLLOUT_SPEC.loader
ROLLOUT_MODULE = importlib.util.module_from_spec(ROLLOUT_SPEC)
ROLLOUT_SPEC.loader.exec_module(ROLLOUT_MODULE)


class WindowsGovernedClosureTests(unittest.TestCase):
    def make_stage(self, root: Path, *, collision: bool = False) -> Path:
        stage = root / "stage"
        config = stage / "config"
        evidence = config / "doc/testing/evidence"
        evidence.mkdir(parents=True)
        (evidence / "governance.json").write_text("{}\n", encoding="utf-8")
        (evidence / "binding.md").write_text("binding\n", encoding="utf-8")
        (stage / "deployment-truth.md").write_text("truth\n", encoding="utf-8")
        world = stage / "generated-world/world"
        world.mkdir(parents=True)
        (world / "world.json").write_text("{}\n", encoding="utf-8")
        sidecar = stage / "generated-world/generated-scenario-world"
        sidecar.mkdir()
        (sidecar / "sidecar.json").write_text("{}\n", encoding="utf-8")
        provenance = stage / "generated-world/world-generation-provenance.json"
        provenance.write_text("{}\n", encoding="utf-8")
        alternate = stage / "other/governance.json"
        alternate.parent.mkdir()
        alternate.write_text('{"different": true}\n', encoding="utf-8")
        genesis_refs = {
            "governance_public_manifest_ref": str(evidence / "governance.json"),
            "binding_notes_ref": str(evidence / "binding.md"),
        }
        (config / "public-testnet-governed-bootstrap-genesis-2026-06-06.json").write_text(
            json.dumps({"governance_bootstrap_refs": genesis_refs}), encoding="utf-8"
        )
        bundle = {
            "track": "public_testnet",
            "world_snapshot": {"ref": str(world), "kind": "directory", "resolved_path": str(world)},
            "generated_world_sidecar": {"ref": str(sidecar), "kind": "directory"},
            "world_generation_provenance": {"ref": str(provenance), "kind": "file"},
            "governance_manifest": {
                "ref": str(alternate if collision else evidence / "governance.json"),
                "kind": "file",
            },
            "network_manifest": None,
            "evidence_refs": [{"ref": str(stage / "deployment-truth.md"), "kind": "file"}],
        }
        (config / "public-testnet-governed-bootstrap-bundle-2026-06-06.json").write_text(
            json.dumps(bundle), encoding="utf-8"
        )
        (config / "public-testnet-governed-bootstrap-manifest-2026-06-06.json").write_text(
            json.dumps({"tier": "public_testnet", "runtime_refs": {}}), encoding="utf-8"
        )
        (config / "public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt").write_text(
            "/ip4/127.0.0.1/tcp/4100/p2p/test\n", encoding="utf-8"
        )
        return stage

    def test_localizes_complete_recursive_closure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            out = root / "windows-governed-closure"
            MODULE.localize(self.make_stage(root), out)
            bundle = json.loads((out / MODULE.NAMES["bundle"]).read_text(encoding="utf-8"))
            genesis = json.loads((out / MODULE.NAMES["genesis"]).read_text(encoding="utf-8"))
            self.assertEqual(bundle["world_snapshot"]["ref"], "generated-world/world")
            self.assertEqual(bundle["generated_world_sidecar"]["ref"], "generated-world/generated-scenario-world")
            self.assertTrue((out / "generated-world/world/world.json").is_file())
            self.assertTrue((out / "doc/testing/evidence/governance.json").is_file())
            self.assertEqual(
                genesis["governance_bootstrap_refs"]["governance_public_manifest_ref"],
                "doc/testing/evidence/governance.json",
            )
            self.assertNotIn("resolved_path", bundle["world_snapshot"])
            platform_dir = out.resolve()
            verified = sorted(
                path.relative_to(platform_dir).as_posix()
                for path in platform_dir.rglob("*")
                if path.is_file()
            )
            governed = ROLLOUT_MODULE.windows_governed_files(platform_dir, verified)
            self.assertGreater(len(governed), 8)

    def test_rejects_localized_basename_collision(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with self.assertRaises(SystemExit):
                MODULE.localize(self.make_stage(root, collision=True), root / "windows-governed-closure")

    def test_rejects_symlinked_bundle_file_reference(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            stage = self.make_stage(root)
            evidence = stage / "config/doc/testing/evidence"
            symlink = evidence / "governance-link.json"
            symlink.symlink_to(evidence / "governance.json")
            bundle_path = stage / "config/public-testnet-governed-bootstrap-bundle-2026-06-06.json"
            bundle = json.loads(bundle_path.read_text(encoding="utf-8"))
            bundle["governance_manifest"]["ref"] = str(symlink)
            bundle_path.write_text(json.dumps(bundle), encoding="utf-8")

            with self.assertRaises(SystemExit):
                MODULE.localize(stage, root / "windows-governed-closure")

    def test_rejects_symlinked_genesis_reference(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            stage = self.make_stage(root)
            evidence = stage / "config/doc/testing/evidence"
            copied = evidence / "binding.md"
            copied.unlink()
            copied.symlink_to(evidence / "governance.json")

            with self.assertRaises(SystemExit):
                MODULE.localize(stage, root / "windows-governed-closure")

    def test_closure_is_checksum_root_and_rollout_platform_dir(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            closure = root / "windows-governed-closure"
            MODULE.localize(self.make_stage(root), closure)
            installer = closure / "oasis7-windows-x64.exe"
            installer.write_text("installer\n", encoding="utf-8")
            buildinfo = closure / "windows-x64-BUILDINFO"
            buildinfo.write_text("package_version=test\n", encoding="utf-8")
            files = sorted(
                path.relative_to(closure).as_posix()
                for path in closure.rglob("*")
                if path.is_file()
            )
            sums = closure / "windows-x64-SHA256SUMS"
            sums.write_text(
                "".join(
                    f"{ROLLOUT_MODULE.sha256_file(closure / name)}  {name}\n"
                    for name in files
                ),
                encoding="utf-8",
            )

            platform_dir = ROLLOUT_MODULE.find_platform_dir(root, "windows-x64").resolve()
            self.assertEqual(platform_dir, closure.resolve())
            self.assertFalse((root / "windows-x64-SHA256SUMS").exists())
            verified = ROLLOUT_MODULE.verify_sha256sums(platform_dir, sums.resolve())
            ROLLOUT_MODULE.require_verified_files(
                "windows-x64", platform_dir, buildinfo.resolve(), installer.resolve(), verified
            )
            self.assertGreater(len(ROLLOUT_MODULE.windows_governed_files(platform_dir, verified)), 8)


if __name__ == "__main__":
    unittest.main()
