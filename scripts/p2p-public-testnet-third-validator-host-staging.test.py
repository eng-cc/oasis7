#!/usr/bin/env python3
"""PG-02R-B RED contract for validator-47 host staging.

This file is intentionally tests-only.  It freezes the operator-side
contract required before validator-47 can be staged on the empty host:

* the triad inventory is an authority input and its byte digest is retained;
* the staged node.env is bound to validator-47 identity, role, ports,
  provider capability, world, manifest, and registry truth;
* stale pair identity and target drift are rejected before materialization;
* readback proves disabled plus inactive/dead/no-process/no-listener state; and
* the stable runbook and fresh-host manual provide an executable no-start,
  pair-preserving, cold-cutover, clean-redeploy rollback path.

The current implementation intentionally lacks these validator-47 host
contracts.  The tests must remain immutable through the GREEN implementation;
do not weaken them to accommodate the current pair-only behavior.
"""

from __future__ import annotations

import contextlib
import hashlib
import importlib.util
from importlib.machinery import SourceFileLoader
import io
import json
import os
from pathlib import Path
import re
import stat
import tempfile
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[1]
INVENTORY = ROOT / "scripts/public-testnet-validator-triad-inventory.v1.json"
STAGE = ROOT / "scripts/p2p-public-testnet-build-deployment-stage.sh"
BOOTSTRAP = ROOT / "scripts/p2p-public-testnet-bootstrap-fresh-validator-host.sh"
FLEET_HEALTH = ROOT / "scripts/p2p-public-testnet-fleet-health.py"
SERVICE_READBACK = ROOT / "scripts/service-readback"
RUNBOOK = ROOT / "doc/p2p/blockchain/public-testnet-governed-bootstrap.runbook.md"
MANUAL = ROOT / "doc/testing/manual/public-testnet-fresh-validator-host-bootstrap-2026-07-28.manual.md"

INVENTORY_RELATIVE = "scripts/public-testnet-validator-triad-inventory.v1.json"
VALIDATOR_47_NODE_ID = "triad-testnet-validator-47"
VALIDATOR_47_SERVICE = "oasis7-triad-validator-47.service"
VALIDATOR_47_ENV = {
    "NODE_ID": VALIDATOR_47_NODE_ID,
    "NODE_ROLE": "validator",
    "STATUS_BIND": "0.0.0.0:6634",
    "NODE_GOSSIP_BIND": "0.0.0.0:6834",
    "WORLD_ID": "oasis7-public-testnet-governed-20260606",
    "NETWORK_TIER_MANIFEST_PATH": "config/public-testnet-governed-bootstrap-manifest-2026-06-06.json",
    "GENESIS_VALIDATOR_REGISTRY_PATH": "config/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json",
    "EXECUTION_WORLD_DIR": "staged-world",
}


def load_module(path: Path, name: str):
    loader = SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    if spec is None or spec.loader is None:
        raise AssertionError(f"cannot load readback target: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def write_fake_readback_tools(bin_dir: Path, *, unit_file_state: str) -> None:
    """Create deterministic read-only command fixtures; never touch a host."""
    systemctl = bin_dir / "systemctl"
    systemctl.write_text(
        "#!/bin/sh\n"
        "set -eu\n"
        "case \"${1:-}\" in\n"
        "  show)\n"
        "    printf '%s\\n' \\\n"
        "      LoadState=loaded \\\n"
        "      ActiveState=inactive \\\n"
        "      SubState=dead \\\n"
        f"      UnitFileState={unit_file_state} \\\n"
        "      NRestarts=0\n"
        "    ;;\n"
        f"  is-enabled) printf '%s\\n' {unit_file_state} ;;\n"
        "  is-active) printf '%s\\n' inactive ;;\n"
        "  *) exit 2 ;;\n"
        "esac\n",
        encoding="utf-8",
    )
    (bin_dir / "ps").write_text(
        "#!/bin/sh\nprintf '  PID COMMAND\\n'\n", encoding="utf-8"
    )
    (bin_dir / "ss").write_text(
        "#!/bin/sh\nprintf 'State Recv-Q Send-Q Local Address:Port Peer Address:Port\\n'\n",
        encoding="utf-8",
    )
    for path in bin_dir.iterdir():
        path.chmod(path.stat().st_mode | stat.S_IXUSR)


class Validator47HostStagingRedTests(unittest.TestCase):
    def test_stage_identity_binding_is_before_output_materialization(self) -> None:
        """A public identity mismatch must fail before the stage root is touched."""
        source = STAGE.read_text(encoding="utf-8")
        preflight = source.index("validator_47_preflight_identity_receipt")
        output_cleanup = source.index('rm -rf "$out_dir"')
        self.assertLess(preflight, output_cleanup)
        self.assertIn("staged public identity does not match supplied governed signer", source)

    def test_inventory_is_authority_and_digest_is_consumed_by_operator_paths(self) -> None:
        """Stage/bootstrap/health/readback must consume one immutable inventory."""
        # Keep the expected authority shape local and deterministic.  The
        # production inventory is deliberately checked separately so a test
        # fixture cannot silently become an authority substitute.
        fixture_inventory = {
            "schema_version": "oasis7.public_testnet_validator_triad_inventory.v1",
            "network_tier": "public_testnet",
            "topology": "three_equal_validator",
            "nodes": {
                "validator-47": {
                    "node_id": VALIDATOR_47_NODE_ID,
                    "roles": ["validator", "checkpoint_provider", "full_storage_provider"],
                    "service": VALIDATOR_47_SERVICE,
                    "ports": ["6634", "6834"],
                }
            },
        }
        with tempfile.TemporaryDirectory(prefix="oasis7-pg02r-b-inventory-") as temp_dir:
            fixture_path = Path(temp_dir) / "triad-inventory.json"
            fixture_path.write_text(
                json.dumps(fixture_inventory, sort_keys=True, separators=(",", ":")) + "\n",
                encoding="utf-8",
            )
            fixture_digest = hashlib.sha256(fixture_path.read_bytes()).hexdigest()
        self.assertEqual(len(fixture_digest), 64)

        self.assertTrue(
            INVENTORY.is_file(),
            f"RED: missing governed triad inventory authority: {INVENTORY}",
        )
        self.assertFalse(INVENTORY.is_symlink(), "triad inventory authority must not be a symlink")
        value = json.loads(INVENTORY.read_text(encoding="utf-8"))
        self.assertEqual(value.get("schema_version"), fixture_inventory["schema_version"])
        self.assertEqual(value.get("network_tier"), "public_testnet")
        self.assertEqual(value.get("topology"), "three_equal_validator")

        for path in (STAGE, BOOTSTRAP, FLEET_HEALTH, SERVICE_READBACK):
            source = path.read_text(encoding="utf-8")
            self.assertTrue(
                INVENTORY_RELATIVE in source,
                f"RED: {path.name} does not consume the triad inventory authority",
            )
            self.assertTrue(
                re.search(r"(?i)(sha256|digest)", source),
                f"RED: {path.name} does not retain an inventory digest binding",
            )

    def test_stage_contract_renders_validator47_node_env_bindings(self) -> None:
        """The deployment stage must emit a complete validator-47 env contract."""
        source = STAGE.read_text(encoding="utf-8")
        self.assertTrue("node.env" in source, "RED: stage does not render or validate node.env")
        for key, value in VALIDATOR_47_ENV.items():
            self.assertTrue(
                key in source,
                f"RED: stage does not bind validator-47 node.env field {key}",
            )
            self.assertTrue(
                value in source,
                f"RED: stage does not bind validator-47 node.env value {key}={value}",
            )
        self.assertTrue(
            VALIDATOR_47_SERVICE in source,
            "RED: stage does not bind the validator-47 systemd service",
        )

    def test_identity_import_is_explicit_byte_preserving_and_registry_bound(self) -> None:
        """The final stack must import, never regenerate, validator-47 identity."""
        stage_source = STAGE.read_text(encoding="utf-8")
        bootstrap_source = BOOTSTRAP.read_text(encoding="utf-8")
        readback_source = SERVICE_READBACK.read_text(encoding="utf-8")
        for marker in (
            "--validator-47-identity-dir",
            "identity-receipt.json",
            "identity-receipt",
            "regular file",
            "mode 0600",
            "ownership",
            "bytes changed",
            "does not match governed registry",
        ):
            self.assertTrue(
                marker in stage_source or marker in bootstrap_source or marker in readback_source,
                f"identity import contract missing marker {marker}",
            )
        self.assertIn("--identity-dir", bootstrap_source)
        self.assertIn("install -m 0600", bootstrap_source)
        self.assertIn("identity-receipt", bootstrap_source)
        self.assertIn("identity_descriptor", readback_source)
        self.assertIn("finality_public_key", readback_source)
        self.assertIn("validator-47 public identity", readback_source)

    def test_bootstrap_rejects_stale_pair_identity_and_target_drift(self) -> None:
        """A stale pair env must fail closed for node, role, port, and world drift."""
        # This fixture describes the exact kind of stale pair input that must
        # never be promoted as validator-47.  It is data-only and never sent
        # to a host or provider.
        stale_pair_env = dict(VALIDATOR_47_ENV)
        stale_pair_env.update(
            {
                "NODE_ID": "triad-testnet-storage",
                "NODE_ROLE": "storage",
                "STATUS_BIND": "0.0.0.0:6632",
                "NODE_GOSSIP_BIND": "0.0.0.0:6832",
                "WORLD_ID": "stale-pair-world",
            }
        )
        with tempfile.TemporaryDirectory(prefix="oasis7-pg02r-b-stale-") as temp_dir:
            env_path = Path(temp_dir) / "node.env"
            env_path.write_text(
                "\n".join(f"{key}={value}" for key, value in stale_pair_env.items()) + "\n",
                encoding="utf-8",
            )
            self.assertTrue(env_path.is_file())
            self.assertIn("triad-testnet-storage", env_path.read_text(encoding="utf-8"))

        source = BOOTSTRAP.read_text(encoding="utf-8")
        self.assertTrue(VALIDATOR_47_NODE_ID in source, "RED: bootstrap has no validator-47 target")
        self.assertTrue(VALIDATOR_47_SERVICE in source, "RED: bootstrap has no validator-47 service target")
        self.assertNotIn("systemctl start", source, "validator-47 staging must remain no-start")
        self.assertNotIn("systemctl enable", source, "validator-47 staging must remain disabled")
        for marker in (
            "NODE_ID",
            "NODE_ROLE",
            "6634",
            "6834",
            "checkpoint_provider",
            "full_storage_provider",
            "WORLD_ID",
            "NETWORK_TIER_MANIFEST_PATH",
            "GENESIS_VALIDATOR_REGISTRY_PATH",
            "EXECUTION_WORLD_DIR",
            "mismatch",
        ):
            self.assertTrue(
                marker in source,
                f"RED: bootstrap does not reject stale validator-47 binding field {marker}",
            )

    def test_readback_proves_disabled_inactive_no_process_and_no_listener(self) -> None:
        """Readback must expose disabled state and reject an enabled unit."""
        with tempfile.TemporaryDirectory(prefix="oasis7-pg02r-b-readback-") as temp_dir:
            root = Path(temp_dir) / "stack"
            root.mkdir()
            config = root / "config"
            config.mkdir()
            config.chmod(0o700)
            inventory_bytes = INVENTORY.read_bytes()
            (config / INVENTORY.name).write_bytes(inventory_bytes)
            (config / "node-keypair.toml").write_text("already-staged-key\n", encoding="utf-8")
            (config / "node-keypair.toml").chmod(0o600)
            public_receipt = {
                "schema_version": "oasis7.identity_provision.v1",
                "node_id": VALIDATOR_47_NODE_ID,
                "root_public_key": "aa" * 32,
                "finality_public_key": "bb" * 32,
                "libp2p_peer_id": "12D3KooWValidator47",
            }
            receipt_bytes = (json.dumps(public_receipt, sort_keys=True) + "\n").encode("utf-8")
            (config / "identity-receipt.json").write_bytes(receipt_bytes)
            (config / "identity-receipt.json").chmod(0o600)
            registry = {
                "validators": [
                    {"node_id": VALIDATOR_47_NODE_ID, "finality_signer_public_key": "bb" * 32}
                ]
            }
            (config / "public-testnet-governed-bootstrap-validator-registry-2026-06-06.json").write_text(
                json.dumps(registry), encoding="utf-8"
            )
            (config / "node.env").write_text(
                "\n".join(
                    [
                        "NODE_ID=triad-testnet-validator-47",
                        "NODE_ROLE=storage",
                        "P2P_NODE_ROLE=full_storage",
                        "CHECKPOINT_PROVIDER=1",
                        "FULL_STORAGE_PROVIDER=1",
                        "IDENTITY_KEY_PATH=config/node-keypair.toml",
                        "IDENTITY_RECEIPT_PATH=config/identity-receipt.json",
                        f"DEPLOYMENT_INVENTORY_SHA256={hashlib.sha256(inventory_bytes).hexdigest()}",
                        f"IDENTITY_KEY_SHA256={hashlib.sha256((config / 'node-keypair.toml').read_bytes()).hexdigest()}",
                        f"IDENTITY_RECEIPT_SHA256={hashlib.sha256(receipt_bytes).hexdigest()}",
                    ]
                )
                + "\n",
                encoding="utf-8",
            )
            (config / "node.env").chmod(0o600)
            bin_dir = Path(temp_dir) / "bin"
            bin_dir.mkdir()
            write_fake_readback_tools(bin_dir, unit_file_state="disabled")
            module = load_module(SERVICE_READBACK, "service_readback_pg02r_b")
            module.CANONICAL_ROOT = str(root)
            args = [
                "--read-only",
                "--role",
                "validator-47",
                "--root",
                str(root),
                "--service",
                VALIDATOR_47_SERVICE,
            ]
            output = io.StringIO()
            with (
                mock.patch.dict(os.environ, {"PATH": f"{bin_dir}:{os.environ['PATH']}"}),
                contextlib.redirect_stdout(output),
                contextlib.redirect_stderr(io.StringIO()),
            ):
                try:
                    result = module.main(args)
                except SystemExit as exc:
                    raise AssertionError(
                        "RED: readback rejected the validator-47 role before proving "
                        f"the no-start contract (exit {exc.code})"
                    ) from None
            self.assertEqual(result, 0)
            payload = json.loads(output.getvalue())
            self.assertEqual(payload["unit_file_state"], "disabled")
            self.assertFalse(payload["active"])
            self.assertFalse(payload["running"])
            self.assertEqual(payload["service_state"], "stopped")
            self.assertEqual(payload["listeners"], [])
            self.assertTrue(payload["no_process"])
            self.assertTrue(payload["no_listener"])

            write_fake_readback_tools(bin_dir, unit_file_state="enabled")
            enabled_module = load_module(SERVICE_READBACK, "service_readback_pg02r_b_enabled")
            enabled_module.CANONICAL_ROOT = str(root)
            with mock.patch.dict(os.environ, {"PATH": f"{bin_dir}:{os.environ['PATH']}"}):
                with self.assertRaises(SystemExit):
                    enabled_module.main(args)

    def test_bootstrap_and_readback_receipt_carries_no_start_proof(self) -> None:
        """The receipt path must retain independent no-start observations."""
        bootstrap_source = BOOTSTRAP.read_text(encoding="utf-8")
        readback_source = SERVICE_READBACK.read_text(encoding="utf-8")
        for marker in (
            "UnitFileState",
            "disabled",
            "inactive",
            "no_process",
            "no_listener",
            "listeners",
            "service-readback",
        ):
            self.assertTrue(
                marker in bootstrap_source or marker in readback_source,
                f"RED: no-start receipt/readback does not prove {marker}",
            )

    def test_runbook_and_manual_define_executable_validator47_recovery_path(self) -> None:
        """Stable operator docs must cover no-start, cutover, and clean redeploy."""
        command_markers = (
            "p2p-public-testnet-bootstrap-fresh-validator-host.sh",
            "--node-id triad-testnet-validator-47",
            f"--service-name {VALIDATOR_47_SERVICE}",
        )
        contract_patterns = {
            "validator-47": r"validator-47",
            "no-start": r"no[- ]start",
            "existing pair": r"existing pair",
            "cold cutover": r"cold[- ]cutover",
            "staggered": r"staggered",
            "clean redeploy": r"clean[- ]redeploy",
            "pair preservation": r"pair[- ]preserv",
        }
        for path in (RUNBOOK, MANUAL):
            source = path.read_text(encoding="utf-8").lower()
            for marker in command_markers:
                self.assertTrue(
                    marker.lower() in source,
                    f"RED: {path.name} lacks executable validator-47 command {marker}",
                )
            for marker, pattern in contract_patterns.items():
                self.assertTrue(
                    re.search(pattern, source),
                    f"RED: {path.name} lacks executable validator-47 contract marker {marker}",
                )


if __name__ == "__main__":
    unittest.main(verbosity=2)
