#!/usr/bin/env python3
"""Contract tests for wrapper resume FD forwarding.

The wrapper is run unchanged, while a temporary ``python3`` shim stands in
for the executor.  This verifies shell pre-parsing and forwarding without
performing plan admission, SSH, host mutation, or callback execution.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
WRAPPER = ROOT / "scripts" / "p2p-public-testnet-rebuild-validators.sh"


class GovernedWrapperFdForwardingTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory(prefix="oasis7-wrapper-fd-contract-")
        self.root = Path(self.tmp.name)
        self.bin_dir = self.root / "bin"
        self.bin_dir.mkdir()
        self.log = self.root / "executor-argv.json"
        self.fake_python = self.bin_dir / "python3"
        self.fake_python.write_text(
            f"""#!{sys.executable}
import json
import os
import sys
from pathlib import Path

markers = [
    "wrapper-shared-secret",
    "wrapper-storage-secret",
    "wrapper-sequencer-secret",
]
args = sys.argv[1:]
fd_values = {{}}
for flag in (
    "--adapter-credential-fd",
    "--adapter-storage-credential-fd",
    "--adapter-sequencer-credential-fd",
):
    if flag not in args:
        continue
    try:
        descriptor = int(args[args.index(flag) + 1])
        fd_values[flag] = os.read(descriptor, 4096).decode("utf-8")
    except (IndexError, OSError, UnicodeDecodeError, ValueError) as error:
        fd_values[flag] = error.__class__.__name__
Path({str(self.log)!r}).write_text(
    json.dumps({{
        "argv": args,
        "secret_in_argv": any(marker in json.dumps(args) for marker in markers),
        "fd_values": fd_values,
    }}, sort_keys=True) + "\\n",
    encoding="utf-8",
)
raise SystemExit(42)
""",
            encoding="utf-8",
        )
        self.fake_python.chmod(0o700)
        self.transaction = self.root / "transaction.json"
        self.transaction.write_text("{}\n", encoding="utf-8")
        self.request = self.root / "request.json"
        self.request.write_text("{}\n", encoding="utf-8")
        self.known_hosts = self.root / "known-hosts"
        self.known_hosts.write_text("fixture\n", encoding="utf-8")
        self.adapter = self.root / "adapter.py"
        self.adapter.write_text("#!/usr/bin/env python3\n", encoding="utf-8")
        self.adapter.chmod(0o700)
        self.nonce_ledger = self.root / "nonce-ledger.jsonl"
        self.credential = self.root / "credential"
        self.credential.write_text("wrapper-shared-secret\n", encoding="utf-8")

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def _run_resume(self, extra: list[str], *fds: int) -> subprocess.CompletedProcess[str]:
        args = [
            str(WRAPPER),
            "resume",
            "--execution-mode",
            "triad_staggered",
            "--transaction",
            str(self.transaction),
            "--host-adapter",
            str(self.adapter),
            "--request",
            str(self.request),
            "--known-hosts",
            str(self.known_hosts),
            *extra,
        ]
        env = {
            **os.environ,
            "PATH": f"{self.bin_dir}:{os.environ.get('PATH', '')}",
            "OASIS7_VALIDATOR_PAIR_NONCE_LEDGER": str(self.nonce_ledger),
            "SSHPASS": "wrapper-shared-secret",
            "PUBLIC_TESTNET_STORAGE_SSHPASS": "wrapper-storage-secret",
            "PUBLIC_TESTNET_SEQUENCER_SSHPASS": "wrapper-sequencer-secret",
        }
        return subprocess.run(
            args,
            check=False,
            text=True,
            capture_output=True,
            env=env,
            pass_fds=fds,
        )

    def _executor_record(self) -> dict[str, object] | None:
        if not self.log.exists():
            return None
        return json.loads(self.log.read_text(encoding="utf-8"))

    def test_resume_forwards_shared_adapter_fd_without_secret_logging(self) -> None:
        fd = os.open(self.credential, os.O_RDONLY)
        try:
            result = self._run_resume(["--adapter-credential-fd", str(fd)], fd)
        finally:
            os.close(fd)
        self.assertNotEqual(result.returncode, 0)
        record = self._executor_record()
        self.assertIsNotNone(record, result.stderr)
        assert record is not None
        self.assertIn("resume", record["argv"])
        self.assertIn("--adapter-credential-fd", record["argv"])
        self.assertIn(str(fd), record["argv"])
        self.assertFalse(record["secret_in_argv"])
        self.assertEqual(record["fd_values"]["--adapter-credential-fd"], "wrapper-shared-secret\n")
        self.assertNotIn("wrapper-shared-secret", result.stdout + result.stderr)

    def test_resume_forwards_both_role_specific_adapter_fds(self) -> None:
        storage = self.root / "storage-credential"
        sequencer = self.root / "sequencer-credential"
        storage.write_text("wrapper-storage-secret\n", encoding="utf-8")
        sequencer.write_text("wrapper-sequencer-secret\n", encoding="utf-8")
        storage_fd = os.open(storage, os.O_RDONLY)
        sequencer_fd = os.open(sequencer, os.O_RDONLY)
        try:
            result = self._run_resume(
                [
                    "--adapter-storage-credential-fd",
                    str(storage_fd),
                    "--adapter-sequencer-credential-fd",
                    str(sequencer_fd),
                ],
                storage_fd,
                sequencer_fd,
            )
        finally:
            os.close(storage_fd)
            os.close(sequencer_fd)
        self.assertNotEqual(result.returncode, 0)
        record = self._executor_record()
        self.assertIsNotNone(record, result.stderr)
        assert record is not None
        args = record["argv"]
        self.assertIn("--adapter-storage-credential-fd", args)
        self.assertIn(str(storage_fd), args)
        self.assertIn("--adapter-sequencer-credential-fd", args)
        self.assertIn(str(sequencer_fd), args)
        self.assertEqual(record["fd_values"]["--adapter-storage-credential-fd"], "wrapper-storage-secret\n")
        self.assertEqual(record["fd_values"]["--adapter-sequencer-credential-fd"], "wrapper-sequencer-secret\n")
        self.assertFalse(record["secret_in_argv"])
        self.assertNotIn("wrapper-storage-secret", result.stdout + result.stderr)
        self.assertNotIn("wrapper-sequencer-secret", result.stdout + result.stderr)

    def test_resume_rejects_incomplete_role_mapping_before_executor_callback(self) -> None:
        fd = os.open(self.credential, os.O_RDONLY)
        try:
            result = self._run_resume(["--adapter-storage-credential-fd", str(fd)], fd)
        finally:
            os.close(fd)
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr, r"(?i)(role-specific|storage.*sequencer|incomplete)")
        self.assertIsNone(self._executor_record())
        self.assertNotIn("wrapper-shared-secret", result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
