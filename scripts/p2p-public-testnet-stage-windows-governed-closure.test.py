#!/usr/bin/env python3
"""Focused closure-localization tests for the Windows Testnet Package helper."""
from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


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
        governance = {
            "entries": [
                {
                    "slot_id": "ops.rollback.on_call.v1",
                    "signer_id": "public-testnet-rollback-on-call-01",
                    "scheme": "ed25519",
                    "threshold": 1,
                    "public_key_hex": "9dfad9943645344153bfd0efa982cf4dec8f09aa7d1a3146e65883fd4c997657",
                },
                {
                    "slot_id": "governance.rollback.v1",
                    "signer_id": "public-testnet-rollback-governance-01",
                    "scheme": "ed25519",
                    "threshold": 1,
                    "public_key_hex": "d9f35c8fc0e0e5df53475cc7059f2f38ab901ee39a5c9c464f65b09ef811bf4a",
                },
            ]
        }
        (evidence / "governance.json").write_text(
            json.dumps(governance), encoding="utf-8"
        )
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

    def make_repo_relative_bundle_refs(self, stage: Path) -> Path:
        bundle_path = stage / "config/public-testnet-governed-bootstrap-bundle-2026-06-06.json"
        bundle = json.loads(bundle_path.read_text(encoding="utf-8"))
        for field in (
            "world_snapshot",
            "generated_world_sidecar",
            "world_generation_provenance",
            "governance_manifest",
        ):
            metadata = bundle[field]
            source = Path(metadata["ref"])
            metadata["ref"] = f"output/testnet-packages/assets/stage/{source.relative_to(stage).as_posix()}"
            metadata["resolved_path"] = str(source)
        for metadata in bundle["evidence_refs"]:
            source = Path(metadata["ref"])
            metadata["ref"] = f"output/testnet-packages/assets/stage/{source.relative_to(stage).as_posix()}"
            metadata["resolved_path"] = str(source)
        bundle_path.write_text(json.dumps(bundle), encoding="utf-8")
        return bundle_path

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
            localized_governance = json.loads(
                (out / "doc/testing/evidence/governance.json").read_text(encoding="utf-8")
            )
            self.assertEqual(
                [entry["slot_id"] for entry in localized_governance["entries"]],
                ["ops.rollback.on_call.v1", "governance.rollback.v1"],
            )
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

    def test_localizes_repo_relative_bundle_refs_from_absolute_resolved_paths(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            stage = self.make_stage(root)
            self.make_repo_relative_bundle_refs(stage)
            out = root / "windows-governed-closure"

            MODULE.localize(stage, out)

            bundle = json.loads((out / MODULE.NAMES["bundle"]).read_text(encoding="utf-8"))
            self.assertEqual(bundle["world_snapshot"]["ref"], "generated-world/world")
            self.assertNotIn("resolved_path", bundle["world_snapshot"])

    def test_normalizes_git_bash_and_windows_absolute_paths(self) -> None:
        self.assertEqual(
            MODULE.normalize_absolute_ref("/d/a/oasis7/stage/generated-world/world"),
            "D:/a/oasis7/stage/generated-world/world",
        )
        windows_ref = r"D:\a\oasis7\stage\generated-world\world"
        self.assertEqual(MODULE.normalize_absolute_ref(windows_ref), windows_ref)
        self.assertIsNone(MODULE.normalize_absolute_ref("output/stage/generated-world/world"))

    def test_rejects_resolved_path_outside_stage(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            stage = self.make_stage(root)
            bundle_path = self.make_repo_relative_bundle_refs(stage)
            outside = root / "outside-world"
            outside.mkdir()
            (outside / "world.json").write_text("{}\n", encoding="utf-8")
            bundle = json.loads(bundle_path.read_text(encoding="utf-8"))
            bundle["world_snapshot"]["resolved_path"] = str(outside)
            bundle_path.write_text(json.dumps(bundle), encoding="utf-8")

            with self.assertRaises(SystemExit) as raised:
                MODULE.localize(stage, root / "windows-governed-closure")
            self.assertIn("escapes staged deployment closure", str(raised.exception))

    def test_rejects_symlinked_resolved_path(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            stage = self.make_stage(root)
            bundle_path = self.make_repo_relative_bundle_refs(stage)
            world_link = stage / "generated-world/world-link"
            world_link.symlink_to(stage / "generated-world/world", target_is_directory=True)
            bundle = json.loads(bundle_path.read_text(encoding="utf-8"))
            bundle["world_snapshot"]["resolved_path"] = str(world_link)
            bundle_path.write_text(json.dumps(bundle), encoding="utf-8")

            with self.assertRaises(SystemExit) as raised:
                MODULE.localize(stage, root / "windows-governed-closure")
            self.assertIn("contains symlink component", str(raised.exception))

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

    def test_main_invokes_the_explicit_git_bash_executable(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            runtime = root / "oasis7_chain_runtime.exe"
            runtime.write_text("runtime\n", encoding="utf-8")
            out_dir = root / "windows-governed-closure"
            git_bash = r"C:\Program Files\Git\bin\bash.exe"
            invoked: list[str] = []

            def stage(command: list[str], **_: object) -> None:
                invoked.extend(command)
                stage_dir = Path(command[command.index("--out-dir") + 1])
                self.make_stage(stage_dir.parent)

            with (
                mock.patch.object(MODULE.shutil, "which", return_value="jq"),
                mock.patch.object(MODULE, "validator_keys", return_value=("sequencer", "storage")),
                mock.patch.object(MODULE.subprocess, "run", side_effect=stage),
                mock.patch.object(
                    sys,
                    "argv",
                    [
                        str(SCRIPT),
                        "--runtime-build-ref",
                        str(runtime),
                        "--out-dir",
                        str(out_dir),
                        "--bash-executable",
                        git_bash,
                    ],
                ),
            ):
                self.assertEqual(MODULE.main(), 0)

            self.assertEqual(invoked[:2], [git_bash, str(MODULE.STAGE_SCRIPT)])


if __name__ == "__main__":
    unittest.main()
