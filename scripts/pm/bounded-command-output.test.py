#!/usr/bin/env python3

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("bounded-command-output.py")
TASK_UID = "task_11111111111111111111111111111111"


class BoundedCommandOutputTests(unittest.TestCase):
    def run_wrapper(self, root: Path, *arguments: str) -> subprocess.CompletedProcess[bytes]:
        return subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--repo-root",
                str(root),
                "--task-uid",
                TASK_UID,
                "--label",
                "focused-test",
                *arguments,
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )

    def test_preserves_full_streams_and_reports_explicit_truncation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            stdout = b"one\ntwo\nthree\nfour\nfive\n"
            stderr = b"warning-one\nwarning-two\n"
            result = self.run_wrapper(
                root,
                "--head-lines",
                "1",
                "--tail-lines",
                "1",
                "--max-bytes",
                "1024",
                "--",
                sys.executable,
                "-c",
                "import os; os.write(1, %r); os.write(2, %r)" % (stdout, stderr),
            )
            self.assertEqual(result.returncode, 0, result.stderr.decode())
            summary = json.loads(result.stdout)
            self.assertTrue(summary["truncated"])
            self.assertTrue(summary["stdout"]["truncated"])
            self.assertEqual(summary["stdout"]["summary"], "one\nfive\n")
            self.assertEqual(summary["stderr"]["summary"], stderr.decode())
            for name, expected in (("stdout", stdout), ("stderr", stderr)):
                artifact = root / summary[name]["artifact"]
                self.assertEqual(artifact.read_bytes(), expected)
                self.assertEqual(summary[name]["sha256"], hashlib.sha256(expected).hexdigest())
            self.assertEqual((root / ".pm/scratch/.gitignore").read_text(), "*\n")

    def test_utf8_byte_bound_and_nonzero_status_passthrough(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            payload = "你好世界\n尾巴\n".encode()
            result = self.run_wrapper(
                root,
                "--head-lines",
                "10",
                "--tail-lines",
                "10",
                "--max-bytes",
                "7",
                "--",
                sys.executable,
                "-c",
                "import os,sys; os.write(1, %r); sys.exit(23)" % payload,
            )
            self.assertEqual(result.returncode, 23)
            summary = json.loads(result.stdout)
            self.assertEqual(summary["exit_status"], 23)
            self.assertLessEqual(summary["stdout"]["summary_bytes"], 7)
            self.assertTrue(summary["stdout"]["truncated"])
            self.assertEqual((root / summary["stdout"]["artifact"]).read_bytes(), payload)

    def test_rejects_unsafe_task_or_label_before_running(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for option, value in (("--task-uid", "../escape"), ("--label", "../escape")):
                command = [
                    sys.executable,
                    str(SCRIPT),
                    "--repo-root",
                    str(root),
                    "--task-uid",
                    TASK_UID,
                    "--label",
                    "safe",
                    option,
                    value,
                    "--",
                    sys.executable,
                    "-c",
                    "raise SystemExit(0)",
                ]
                result = subprocess.run(command, capture_output=True, check=False)
                self.assertEqual(result.returncode, 2)
            self.assertFalse((root / ".pm").exists())

    def test_missing_command_maps_to_shell_compatible_status(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            result = self.run_wrapper(root, "--", "definitely-not-an-oasis7-command")
            self.assertEqual(result.returncode, 127)
            summary = json.loads(result.stdout)
            self.assertEqual(summary["exit_status"], 127)
            self.assertIn("command not found", summary["stderr"]["summary"])

    @unittest.skipIf(os.name == "nt", "POSIX signal status contract")
    def test_signal_maps_to_shell_compatible_status(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            result = self.run_wrapper(
                root,
                "--",
                sys.executable,
                "-c",
                "import os,signal; os.kill(os.getpid(), signal.SIGTERM)",
            )
            self.assertEqual(result.returncode, 143)
            self.assertEqual(json.loads(result.stdout)["exit_status"], 143)


if __name__ == "__main__":
    unittest.main()
