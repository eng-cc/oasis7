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
import tarfile
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


VALID_HEALTH_URL = "http://127.0.0.1:6632/healthz"
_MISSING = object()


class HealthProbeTransport:
    """Fake SSH transport separating service-readback from an actual /healthz probe."""

    def __init__(
        self,
        *,
        service_payload: dict[str, object] | None = None,
        health_payload: dict[str, object] | object = _MISSING,
        health_raw: str | object = _MISSING,
    ) -> None:
        self.calls: list[tuple[str, str]] = []
        self.service_payload = service_payload or {
            "independently_observed": True,
            "active": True,
            "running": True,
            "service_state": "running",
            "listeners": [],
        }
        self.health_payload = health_payload
        self.health_raw = health_raw

    def command(self, role: str, remote: str, timeout: int = 60) -> str:
        del timeout
        self.calls.append((role, remote))
        if "service-readback" in remote:
            return json.dumps(self.service_payload)
        if "/healthz" in remote:
            if self.health_raw is not _MISSING:
                return str(self.health_raw)
            if self.health_payload is _MISSING:
                raise SystemExit("health probe unavailable")
            return json.dumps(self.health_payload)
        if "sha256sum" in remote:
            return "a" * 64
        raise AssertionError(f"unexpected remote command: {remote}")


def healthy_health_payload() -> dict[str, object]:
    return {
        "ok": True,
        "ready": True,
        "last_error": None,
        "nrestarts": 0,
        "oom_panic_segfault": False,
    }


def valid_remote_backup_entry(
    *,
    role: str = "storage-205",
    transaction_id: str = "fixture-transaction",
) -> dict[str, object]:
    expected = ADAPTER.EXECUTOR.HUMAN_DIRECT_SSH_CANONICAL[role]
    capacity = {
        "verified": True,
        "available_bytes": 4096,
        "required_bytes": 1024,
        "free_bytes": 4096,
        "required_inodes": 16,
        "free_inodes": 16,
    }
    return {
        "role": role,
        "transaction_id": transaction_id,
        "remote_target": True,
        "remote_host": expected["host"],
        "remote_root": ADAPTER.STACK_ROOT,
        "manifest": f"{ADAPTER.STACK_ROOT}/backups/{transaction_id}/manifest.json",
        "manifest_sha256": "a" * 64,
        "reset_surface_manifest_sha256": "b" * 64,
        "reset_surfaces": list(ADAPTER.RESET_SURFACES),
        "capacity": capacity,
        "backup_non_seed": {
            "forensic_only": True,
            "seed_eligible": False,
            "restore_deleted_chain_state": False,
        },
    }


def helper_package() -> dict[str, object]:
    return {
        "version": "fixture-version",
        "commit": "a" * 40,
        "run_id": "123",
        "runtime_sha256": "b" * 64,
        "runtime_size_bytes": 123,
        "helper_sha256": {
            name: f"{index:064x}"
            for index, name in enumerate(ADAPTER.OPS_HELPERS, 1)
        },
    }


def helper_receipt(role: str, package: dict[str, object]) -> dict[str, object]:
    return {
        "preflight_verified": True,
        "package_identity_verified": True,
        "runtime_identity_verified": True,
        "runtime_executable": True,
        "repair_rebuild_helper_executable": True,
        "generated_world_dir_contract": True,
        "governance_registry_importer_executable": True,
        "helper_identity_verified": True,
        "python_available": True,
        "tar_available": True,
        "systemd_available": True,
        "process_inspection_available": True,
        "role": role,
        "service": f"oasis7-triad-{role.split('-')[0]}.service",
        "runtime_sha256": package["runtime_sha256"],
        "helper_sha256": package["helper_sha256"],
    }


class GovernedTriadProductionNegativeTests(unittest.TestCase):
    def test_ops_helper_archive_digest_binding_is_exact(self) -> None:
        with tempfile.TemporaryDirectory(prefix="oasis7-triad-ops-helper-archive-") as temp_dir:
            root = Path(temp_dir)
            bundle = root / "oasis7-linux-x64-ops-tools"
            (bundle / "bin").mkdir(parents=True)
            tools = []
            for name in ADAPTER.OPS_HELPERS:
                path = bundle / "bin" / name
                path.write_bytes((name + " fixture").encode())
                tools.append(
                    {
                        "path": f"bin/{name}",
                        "sha256": sha256(path),
                        "sizeBytes": path.stat().st_size,
                    }
                )
            (bundle / ".oasis7-ops-tools-manifest.json").write_text(
                json.dumps({"opsToolsSchemaVersion": 1, "tools": tools}) + "\n",
                encoding="utf-8",
            )
            (bundle / "SHA256SUMS").write_text(
                "".join(f"{item['sha256']}  ./{item['path']}\n" for item in tools),
                encoding="utf-8",
            )
            archive_path = root / ADAPTER.OPS_TOOLS_NAME
            with tarfile.open(archive_path, "w:gz") as archive:
                archive.add(bundle, arcname=bundle.name)
            observed = ADAPTER.ops_helper_digests(archive_path)
            self.assertEqual(set(observed), set(ADAPTER.OPS_HELPERS))
            self.assertEqual(observed["oasis7_world_repair_rebuild"], tools[0]["sha256"])

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

    def test_preflight_uses_separate_fixed_healthz_probe(self) -> None:
        transport = HealthProbeTransport(health_payload=healthy_health_payload())
        inventory = {
            "nodes": {
                "storage-205": {"service": "oasis7-triad-storage.service"},
            }
        }
        observed = ADAPTER.observe_node(
            transport,
            inventory,
            "storage-205",
            "/fixture/storage-205",
            running=True,
            health_url=VALID_HEALTH_URL,
        )
        self.assertTrue(observed["healthz_ok"])
        self.assertTrue(observed["ready"])
        self.assertEqual(observed["last_error"], None)
        self.assertEqual(observed["nrestarts"], 0)
        self.assertFalse(observed["oom_panic_segfault"])
        service_calls = [remote for _, remote in transport.calls if "service-readback" in remote]
        health_calls = [remote for _, remote in transport.calls if "/healthz" in remote]
        self.assertEqual(len(service_calls), 1)
        self.assertEqual(len(health_calls), 1)
        self.assertNotEqual(service_calls[0], health_calls[0])
        self.assertIn(VALID_HEALTH_URL, health_calls[0])

    def test_preflight_requires_exact_packaged_helper_identity_before_remote_probe(self) -> None:
        class NoRemoteTransport:
            def __init__(self) -> None:
                self.calls: list[tuple[str, str]] = []

            def command(self, role: str, remote: str, timeout: int = 60) -> str:
                self.calls.append((role, remote))
                raise AssertionError("remote preflight must not run without helper identity")

        transport = NoRemoteTransport()
        inventory = {
            "nodes": {
                "storage-205": {
                    "service": "oasis7-triad-storage.service",
                }
            }
        }
        package = helper_package()
        package.pop("helper_sha256")
        with self.assertRaisesRegex(SystemExit, r"(?i)(helper|identity|package)"):
            ADAPTER.preflight_helper_capabilities(transport, inventory, "storage-205", package)
        self.assertEqual(transport.calls, [])

    def test_preflight_rejects_helper_mismatch_or_tool_unreadiness_before_mutation(self) -> None:
        package = helper_package()
        inventory = {
            "nodes": {
                "storage-205": {
                    "service": "oasis7-triad-storage.service",
                }
            }
        }

        class PreflightTransport:
            def __init__(self, receipt: dict[str, object]) -> None:
                self.receipt = receipt
                self.calls: list[tuple[str, str]] = []

            def command(self, role: str, remote: str, timeout: int = 60) -> str:
                del timeout
                self.calls.append((role, remote))
                self.assert_read_only(remote)
                return json.dumps(self.receipt)

            @staticmethod
            def assert_read_only(remote: str) -> None:
                if any(token in remote for token in ("systemctl stop", "systemctl start", "rm -rf", "reset")):
                    raise AssertionError("preflight helper probe contains mutation")

        bad_hash = helper_receipt("storage-205", package)
        bad_hash["helper_sha256"] = dict(package["helper_sha256"])
        bad_hash["helper_sha256"]["oasis7_world_repair_rebuild"] = "f" * 64
        with self.assertRaisesRegex(SystemExit, r"(?i)(helper|identity)"):
            ADAPTER.preflight_helper_capabilities(
                PreflightTransport(bad_hash), inventory, "storage-205", package
            )

        unavailable = helper_receipt("storage-205", package)
        unavailable["python_available"] = False
        with self.assertRaisesRegex(SystemExit, r"(?i)(readiness|python|preflight)"):
            ADAPTER.preflight_helper_capabilities(
                PreflightTransport(unavailable), inventory, "storage-205", package
            )

    def test_absent_healthz_fails_closed(self) -> None:
        transport = HealthProbeTransport()
        inventory = {"nodes": {"storage-205": {"service": "oasis7-triad-storage.service"}}}
        with self.assertRaisesRegex(SystemExit, r"(?i)health"):
            ADAPTER.observe_node(
                transport,
                inventory,
                "storage-205",
                "/fixture/storage-205",
                running=True,
                health_url=VALID_HEALTH_URL,
            )

    def test_malformed_healthz_fails_closed(self) -> None:
        transport = HealthProbeTransport(health_raw="not-json\n")
        inventory = {"nodes": {"storage-205": {"service": "oasis7-triad-storage.service"}}}
        with self.assertRaisesRegex(SystemExit, r"(?i)health|json"):
            ADAPTER.observe_node(
                transport,
                inventory,
                "storage-205",
                "/fixture/storage-205",
                running=True,
                health_url=VALID_HEALTH_URL,
            )

    def test_false_healthz_fails_closed(self) -> None:
        inventory = {"nodes": {"storage-205": {"service": "oasis7-triad-storage.service"}}}
        for key, value in (
            ("ok", False),
            ("ready", False),
            ("last_error", "fixture failure"),
            ("nrestarts", 1),
            ("oom_panic_segfault", True),
        ):
            with self.subTest(key=key):
                payload = healthy_health_payload()
                payload[key] = value
                transport = HealthProbeTransport(health_payload=payload)
                with self.assertRaisesRegex(SystemExit, r"(?i)health|ready|error|restart|crash"):
                    ADAPTER.observe_node(
                        transport,
                        inventory,
                        "storage-205",
                        "/fixture/storage-205",
                        running=True,
                        health_url=VALID_HEALTH_URL,
                    )

    def test_wrong_healthz_url_is_rejected_before_remote_probe(self) -> None:
        transport = HealthProbeTransport(health_payload=healthy_health_payload())
        inventory = {"nodes": {"storage-205": {"service": "oasis7-triad-storage.service"}}}
        with self.assertRaisesRegex(SystemExit, r"(?i)health|url|local"):
            ADAPTER.observe_node(
                transport,
                inventory,
                "storage-205",
                "/fixture/storage-205",
                running=True,
                health_url="https://untrusted.example/healthz",
            )
        self.assertEqual(transport.calls, [])

    def test_healthz_port_is_role_bound_before_remote_probe(self) -> None:
        cases = (
            ("storage-205", None),
            ("storage-205", "http://127.0.0.1/healthz"),
            ("storage-205", "http://127.0.0.1:6631/healthz"),
            ("sequencer-204", "http://127.0.0.1/healthz"),
            ("sequencer-204", "http://127.0.0.1:6632/healthz"),
        )
        for role, value in cases:
            with self.subTest(role=role, value=value):
                transport = HealthProbeTransport(health_payload=healthy_health_payload())
                inventory = {"nodes": {role: {"service": f"oasis7-triad-{role.split('-')[0]}.service"}}}
                with self.assertRaisesRegex(SystemExit, r"(?i)(health|port|required)"):
                    ADAPTER.observe_node(
                        transport,
                        inventory,
                        role,
                        f"/fixture/{role}",
                        running=True,
                        health_url=value,
                    )
                self.assertEqual(transport.calls, [])

    def test_health_state_mismatch_is_rejected(self) -> None:
        transport = HealthProbeTransport(
            service_payload={
                "independently_observed": True,
                "active": False,
                "running": False,
                "service_state": "stopped",
                "listeners": [],
            },
            health_payload=healthy_health_payload(),
        )
        inventory = {"nodes": {"storage-205": {"service": "oasis7-triad-storage.service"}}}
        with self.assertRaisesRegex(SystemExit, r"(?i)state|running|active"):
            ADAPTER.observe_node(
                transport,
                inventory,
                "storage-205",
                "/fixture/storage-205",
                running=True,
                health_url=VALID_HEALTH_URL,
            )

    def test_service_readback_health_defaults_are_not_authority(self) -> None:
        transport = HealthProbeTransport(
            service_payload={
                "independently_observed": True,
                "active": True,
                "running": True,
                "service_state": "running",
                "listeners": [],
                "healthz_ok": True,
                "ready": True,
                "last_error": None,
                "nrestarts": 0,
                "oom_panic_segfault": False,
            }
        )
        inventory = {"nodes": {"storage-205": {"service": "oasis7-triad-storage.service"}}}
        with self.assertRaisesRegex(SystemExit, r"(?i)health"):
            ADAPTER.observe_node(
                transport,
                inventory,
                "storage-205",
                "/fixture/storage-205",
                running=True,
                health_url=VALID_HEALTH_URL,
            )
        self.assertTrue(any("/healthz" in remote for _, remote in transport.calls))

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
                "proof": {"storage_health_url": "http://127.0.0.1:6632/healthz"},
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
                "proof": {"storage_health_url": "http://127.0.0.1:6632/healthz"},
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

    def test_remote_backup_receipt_rejects_unbound_or_short_evidence(self) -> None:
        cases = [
            (
                "seed eligible",
                lambda entry: entry["backup_non_seed"].update(seed_eligible=True),
            ),
            (
                "missing reset-surface manifest",
                lambda entry: entry.pop("reset_surface_manifest_sha256"),
            ),
            (
                "short free inodes",
                lambda entry: entry["capacity"].update(free_inodes=15),
            ),
            (
                "role mismatch",
                lambda entry: entry.update(role="sequencer-204"),
            ),
            (
                "stale transaction binding",
                lambda entry: entry.update(transaction_id="stale-transaction"),
            ),
        ]
        for label, mutate in cases:
            with self.subTest(label=label):
                entry = valid_remote_backup_entry()
                mutate(entry)
                transaction = {
                    "transaction_id": "fixture-transaction",
                    "backup": {"storage-205": entry},
                }
                with self.assertRaisesRegex(
                    SystemExit,
                    r"(?i)(remote|backup|capacity|seed|surface|transaction|role|inode)",
                ):
                    ADAPTER.require_remote_backup(transaction, "storage-205")


if __name__ == "__main__":
    unittest.main(verbosity=2)
