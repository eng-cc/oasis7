#!/usr/bin/env python3
"""Contract tests for the governed adapter FD-only credential transport.

The child is a fake host adapter.  It records only descriptor metadata and
reads fixture-only bytes from inherited descriptors, then exits non-zero so
the executor's failure journal is exercised without contacting a host.
"""

from __future__ import annotations

import importlib.util
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
EXECUTOR = ROOT / "scripts" / "p2p-public-testnet-validator-pair-rebuild.py"
CONTRACT_TEST = ROOT / "scripts" / "p2p-public-testnet-validator-triad-host-adapter-contract.test.py"


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"unable to load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


CONTRACT = load_module(CONTRACT_TEST, "triad_host_adapter_contract_fixture")
MODULE = CONTRACT.MODULE


class GovernedFdOnlyCredentialContractTests(unittest.TestCase):
    def setUp(self) -> None:
        # Reuse the existing hermetic triad plan fixture without inheriting its
        # test methods.  This keeps the FD slice focused and avoids duplicating
        # identity, runtime, peer, rollback, and phase evidence construction.
        self.fixture = CONTRACT.GovernedTriadHostAdapterContractTests(methodName="runTest")
        self.fixture.setUp()
        self.tmp = self.fixture.tmp
        self.root = self.fixture.root
        self.adapter = self.root / "fake-fd-only-governed-adapter.py"
        self.log = self.root / "fd-only-adapter.json"
        self._write_fake_adapter()

    def tearDown(self) -> None:
        self.fixture.tearDown()

    def _write_fake_adapter(self) -> None:
        self.adapter.write_text(
            f"""#!{os.sys.executable}
import json
import os
import sys
from pathlib import Path

args = sys.argv[1:]
markers = [
    "single-adapter-fd-secret",
    "storage-adapter-fd-secret",
    "sequencer-adapter-fd-secret",
    "ambient-sshpass-secret",
    "ambient-storage-secret",
    "ambient-sequencer-secret",
]
serialized = json.dumps(args, ensure_ascii=True) + json.dumps(dict(os.environ), ensure_ascii=True)
secret_env_keys = [
    key for key, value in os.environ.items()
    if key == "SSHPASS" or key.endswith("_SSHPASS") or any(marker in value for marker in markers)
]
argv_leaked = any(marker in serialized for marker in markers)
fd_values = {{}}
for name in (
    "OASIS7_TRIAD_ADAPTER_SHARED_FD",
    "OASIS7_TRIAD_ADAPTER_STORAGE_FD",
    "OASIS7_TRIAD_ADAPTER_SEQUENCER_FD",
):
    raw = os.environ.get(name)
    if raw is None:
        continue
    try:
        descriptor = int(raw)
        fd_values[name] = {{"fd": descriptor, "value": os.read(descriptor, 4096).decode("utf-8")}}
    except (OSError, UnicodeDecodeError, ValueError) as error:
        fd_values[name] = {{"error": error.__class__.__name__}}
Path({str(self.log)!r}).write_text(
    json.dumps({{
        "argv": args,
        "environment_transport": os.environ.get("OASIS7_TRIAD_ADAPTER_CREDENTIAL_TRANSPORT"),
        "secret_env_keys": secret_env_keys,
        "argv_leaked": argv_leaked,
        "fd_values": fd_values,
    }}, sort_keys=True) + "\\n",
    encoding="utf-8",
)
if argv_leaked or secret_env_keys or any("error" in value for value in fd_values.values()):
    raise SystemExit(91)
raise SystemExit(17)
""",
            encoding="utf-8",
        )
        self.adapter.chmod(0o700)

    def _invoke_failure(self, direct_args: object) -> dict[str, object]:
        transaction = self.fixture._base_plan()
        path = self.root / "fd-only.transaction.json"
        path.write_text(json.dumps(transaction, sort_keys=True) + "\n", encoding="utf-8")
        with patch.dict(
            os.environ,
            {
                "SSHPASS": "ambient-sshpass-secret",
                "PUBLIC_TESTNET_STORAGE_SSHPASS": "ambient-storage-secret",
                "PUBLIC_TESTNET_SEQUENCER_SSHPASS": "ambient-sequencer-secret",
            },
            clear=False,
        ):
            with self.assertRaises(SystemExit) as raised:
                MODULE.run_host_adapter(self.adapter, path, transaction, "staggered-storage", direct_args)
        persisted = json.loads(path.read_text(encoding="utf-8"))
        self.assertRegex(str(raised.exception), r"(?i)(host adapter failed|exit 17)")
        self.assertEqual(persisted["adapter_callback"]["status"], "failed")
        return json.loads(self.log.read_text(encoding="utf-8"))

    def test_single_shared_fd_is_passed_and_password_values_are_scrubbed(self) -> None:
        secret = self.root / "single-adapter-secret"
        secret.write_text("single-adapter-fd-secret\n", encoding="utf-8")
        fd = os.open(secret, os.O_RDONLY)
        try:
            direct_args = MODULE.argparse.Namespace(
                adapter_credential_fd=fd,
                adapter_storage_credential_fd=None,
                adapter_sequencer_credential_fd=None,
                credential_env=None,
                credential_fd=None,
            )
            record = self._invoke_failure(direct_args)
        finally:
            os.close(fd)
        self.assertEqual(record["environment_transport"], "fd-only-v1")
        self.assertEqual(record["secret_env_keys"], [])
        self.assertFalse(record["argv_leaked"])
        self.assertEqual(set(record["fd_values"]), {"OASIS7_TRIAD_ADAPTER_SHARED_FD"})
        self.assertEqual(record["fd_values"]["OASIS7_TRIAD_ADAPTER_SHARED_FD"]["value"], "single-adapter-fd-secret\n")

    def test_role_specific_fds_are_passed_without_crossing(self) -> None:
        storage_secret = self.root / "storage-adapter-secret"
        sequencer_secret = self.root / "sequencer-adapter-secret"
        storage_secret.write_text("storage-adapter-fd-secret\n", encoding="utf-8")
        sequencer_secret.write_text("sequencer-adapter-fd-secret\n", encoding="utf-8")
        storage_fd = os.open(storage_secret, os.O_RDONLY)
        sequencer_fd = os.open(sequencer_secret, os.O_RDONLY)
        try:
            direct_args = MODULE.argparse.Namespace(
                adapter_credential_fd=None,
                adapter_storage_credential_fd=storage_fd,
                adapter_sequencer_credential_fd=sequencer_fd,
                credential_env=None,
                credential_fd=None,
            )
            record = self._invoke_failure(direct_args)
        finally:
            os.close(storage_fd)
            os.close(sequencer_fd)
        values = record["fd_values"]
        self.assertEqual(
            set(values),
            {"OASIS7_TRIAD_ADAPTER_STORAGE_FD", "OASIS7_TRIAD_ADAPTER_SEQUENCER_FD"},
        )
        self.assertEqual(values["OASIS7_TRIAD_ADAPTER_STORAGE_FD"]["value"], "storage-adapter-fd-secret\n")
        self.assertEqual(values["OASIS7_TRIAD_ADAPTER_SEQUENCER_FD"]["value"], "sequencer-adapter-fd-secret\n")
        self.assertNotEqual(
            values["OASIS7_TRIAD_ADAPTER_STORAGE_FD"]["fd"],
            values["OASIS7_TRIAD_ADAPTER_SEQUENCER_FD"]["fd"],
        )
        self.assertEqual(record["secret_env_keys"], [])
        self.assertFalse(record["argv_leaked"])

    def test_incomplete_role_specific_fd_mapping_is_durable_failure(self) -> None:
        secret = self.root / "incomplete-role-secret"
        secret.write_text("storage-adapter-fd-secret\n", encoding="utf-8")
        storage_fd = os.open(secret, os.O_RDONLY)
        transaction = self.fixture._base_plan()
        path = self.root / "incomplete-role.transaction.json"
        path.write_text(json.dumps(transaction, sort_keys=True) + "\n", encoding="utf-8")
        try:
            direct_args = MODULE.argparse.Namespace(
                adapter_credential_fd=None,
                adapter_storage_credential_fd=storage_fd,
                adapter_sequencer_credential_fd=None,
                credential_env=None,
                credential_fd=None,
            )
            with self.assertRaisesRegex(SystemExit, r"(?i)(role-specific|cover storage and sequencer)"):
                MODULE.run_host_adapter(self.adapter, path, transaction, "staggered-storage", direct_args)
        finally:
            os.close(storage_fd)
        persisted = json.loads(path.read_text(encoding="utf-8"))
        self.assertEqual(persisted["adapter_callback"]["status"], "failed")
        self.assertNotEqual(persisted["adapter_callback"].get("status"), "in_flight")


if __name__ == "__main__":
    unittest.main(verbosity=2)
