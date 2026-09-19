#!/usr/bin/env python3
"""RED contract tests for the production governed triad host adapter.

These tests deliberately exercise the production adapter/executor modules,
not temporary fake adapters.  They must fail until the adapter rejects
marker-only identity admission, synthesized health, stale registry inputs,
and local-only backup evidence, and until direct resume re-observation has an
explicit credential seam.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
EXECUTOR_PATH = ROOT / "scripts" / "p2p-public-testnet-validator-pair-rebuild.py"
ADAPTER_PATH = ROOT / "scripts" / "p2p-public-testnet-validator-triad-host-adapter.py"


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"unable to load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


EXECUTOR = load_module(EXECUTOR_PATH, "triad_production_negative_executor")
ADAPTER = load_module(ADAPTER_PATH, "triad_production_negative_adapter")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class GovernedTriadProductionNegativeTests(unittest.TestCase):
    def test_resume_adapter_fd_only_fails_before_direct_observation(self) -> None:
        """Adapter-only resume must not enter direct SSH without a credential seam."""
        with tempfile.TemporaryDirectory(prefix="oasis7-triad-resume-credential-") as temp_dir:
            credential = Path(temp_dir) / "credential"
            credential.write_text("fixture-only\n", encoding="utf-8")
            descriptor = os.open(credential, os.O_RDONLY)
            try:
                args = argparse.Namespace(
                    human_direct_ssh_request="fixture-request.json",
                    known_hosts="fixture-known-hosts",
                    credential_env=None,
                    credential_fd=None,
                    adapter_credential_fd=descriptor,
                    adapter_storage_credential_fd=None,
                    adapter_sequencer_credential_fd=None,
                )
                calls: list[object] = []

                def unexpected_direct_observation(*_args: object, **_kwargs: object) -> dict[str, object]:
                    calls.append(True)
                    return {}

                with patch.object(EXECUTOR, "run_human_direct_ssh", unexpected_direct_observation):
                    with self.assertRaisesRegex(SystemExit, r"(?i)credential"):
                        EXECUTOR._direct_reobserve_from_args(args)
                self.assertEqual(calls, [])
            finally:
                os.close(descriptor)

    def test_marker_only_identity_v2_admission_is_rejected(self) -> None:
        transaction = {
            "proof": {
                "identity_v2": {
                    "verified": True,
                    "status": "verified",
                    # A marker without authenticated receipt bytes is not authority.
                }
            }
        }
        with self.assertRaisesRegex(SystemExit, r"(?i)identity-v2|receipt"):
            ADAPTER.validate_identity_v2(transaction)

    def test_synthesized_health_is_rejected(self) -> None:
        class ReadbackOnlyTransport:
            def command(self, role: str, remote: str, timeout: int = 60) -> str:
                del role, timeout
                if "service-readback" in remote:
                    return json.dumps(
                        {
                            "independently_observed": True,
                            "active": True,
                            "running": True,
                            "service_state": "running",
                            "listeners": [],
                        }
                    )
                if "sha256sum" in remote:
                    return "a" * 64
                raise AssertionError(f"unexpected remote command: {remote}")

        inventory = {
            "nodes": {
                "storage-205": {"service": "oasis7-triad-storage.service"},
            }
        }
        with self.assertRaisesRegex(SystemExit, r"(?i)health"):
            ADAPTER.observe_node(
                ReadbackOnlyTransport(),
                inventory,
                "storage-205",
                "/fixture/storage-205",
                running=True,
            )

    def test_stale_registry_digest_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory(prefix="oasis7-triad-stale-registry-") as temp_dir:
            root = Path(temp_dir)
            files = {}
            for name in ("manifest", "genesis", "bootstrap"):
                path = root / name
                path.write_text(f"{name}-fixture\n", encoding="utf-8")
                files[name] = {"path": str(path), "kind": "file", "sha256": sha256(path)}
            registry = root / "stale-registry.json"
            registry.write_text('{"validators": []}\n', encoding="utf-8")
            files["registry"] = {
                "path": str(registry),
                "kind": "file",
                "sha256": sha256(registry),
            }
            world = root / "world"
            world.mkdir()
            (world / "fixture.txt").write_text("world\n", encoding="utf-8")
            files["world"] = {
                "path": str(world),
                "kind": "directory",
                "sha256": EXECUTOR.inventory_tree(world)["sha256"],
            }
            with self.assertRaisesRegex(SystemExit, r"(?i)registry|governed"):
                ADAPTER.validate_governed({"network": {"governed": files}})

    def test_pair_only_registry_is_rejected_as_triad_admission(self) -> None:
        with tempfile.TemporaryDirectory(prefix="oasis7-triad-pair-only-registry-") as temp_dir:
            root = Path(temp_dir)
            files = {}
            for name in ("manifest", "genesis", "bootstrap"):
                path = root / name
                path.write_text(f"{name}-fixture\n", encoding="utf-8")
                files[name] = {"path": str(path), "kind": "file", "sha256": sha256(path)}
            registry = root / "pair-only-registry.json"
            registry.write_text(
                json.dumps(
                    {
                        "schema_version": "oasis7.public_testnet_validator_registry.v1",
                        "validators": [
                            {"node_id": "storage-205", "stake": 100},
                            {"node_id": "sequencer-204", "stake": 100},
                        ],
                    }
                )
                + "\n",
                encoding="utf-8",
            )
            files["registry"] = {
                "path": str(registry),
                "kind": "file",
                "sha256": sha256(registry),
            }
            world = root / "world"
            world.mkdir()
            (world / "fixture.txt").write_text("world\n", encoding="utf-8")
            files["world"] = {
                "path": str(world),
                "kind": "directory",
                "sha256": EXECUTOR.inventory_tree(world)["sha256"],
            }
            with self.assertRaisesRegex(SystemExit, r"(?i)triad|three|validator"):
                ADAPTER.validate_governed({"network": {"governed": files}})

    def test_reset_script_requires_verified_stop_before_reset(self) -> None:
        with tempfile.TemporaryDirectory(prefix="oasis7-triad-stop-proof-") as temp_dir:
            root = Path(temp_dir)
            package = {
                "version": "fixture-version",
                "commit": "a" * 40,
                "run_id": "123",
                "files": {
                    name: root / name
                    for name in (ADAPTER.PACKAGE_DEB_NAME, ADAPTER.OPS_TOOLS_NAME)
                },
            }
            transaction = {
                "nodes": {
                    "storage-205": {"service": "oasis7-triad-storage.service"},
                },
                "network": {"network_id": "fixture-network", "chain_id": "fixture-chain"},
                "proof": {"storage_health_url": "http://127.0.0.1/healthz"},
            }
            script, _ = ADAPTER.control_script(
                transaction,
                "staggered-storage",
                package,
                {},
                "storage-205",
            )
            reset_start = script.index("for surface in")
            pre_reset = script[:reset_start]
            self.assertNotIn('systemctl stop "$service" || true', pre_reset)
            self.assertIn("service_state", pre_reset)

    def test_local_only_backup_is_rejected_before_destructive_script_generation(self) -> None:
        with tempfile.TemporaryDirectory(prefix="oasis7-triad-local-backup-") as temp_dir:
            root = Path(temp_dir)
            package_files = {
                name: root / name
                for name in (ADAPTER.PACKAGE_DEB_NAME, ADAPTER.OPS_TOOLS_NAME)
            }
            package = {
                "version": "fixture-version",
                "commit": "a" * 40,
                "run_id": "123",
                "files": package_files,
            }
            local_backup = root / "local-backup-manifest.json"
            local_backup.write_text("local-only\n", encoding="utf-8")
            transaction = {
                "nodes": {
                    "storage-205": {"service": "oasis7-triad-storage.service"},
                },
                "network": {"network_id": "fixture-network", "chain_id": "fixture-chain"},
                "proof": {"storage_health_url": "http://127.0.0.1/healthz"},
                "backup": {
                    "storage-205": {
                        "manifest": str(local_backup),
                        "remote_target": False,
                    }
                },
            }
            with self.assertRaisesRegex(SystemExit, r"(?i)remote|backup"):
                ADAPTER.control_script(
                    transaction,
                    "staggered-storage",
                    package,
                    {},
                    "storage-205",
                )


if __name__ == "__main__":
    unittest.main(verbosity=2)
