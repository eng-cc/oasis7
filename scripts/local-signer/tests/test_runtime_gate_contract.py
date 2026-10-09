"""RED contracts for the independently approved pre-Python runtime gate."""
import re
import shlex
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

import test_release_installer as existing

SOURCE = Path(existing.__file__).resolve().parents[1]
RUNBOOK = SOURCE.parents[1] / "doc/p2p/blockchain/local-file-signing-backend.runbook.md"


class RecordingRuntimeHost(existing.FakeHost):
    def __init__(self):
        super().__init__()
        self.facts["runtime_identity"] = {
            "python_version": "3.9.6",
            "executable_cdhash": "a" * 40,
            "framework_cdhash": "b" * 40,
        }
        self.installed_validation_calls = 0

    def validate_installed(self, plan, release):
        self.installed_validation_calls += 1
        return super().validate_installed(plan, release)


class RuntimeIdentityRepeatContract(unittest.TestCase):
    """A complete receipt cannot make runtime identity drift look unchanged."""

    def setUp(self):
        existing.InstallerContract.setUp(self)

    write_manifest = existing.InstallerContract.write_manifest
    verified = existing.InstallerContract.verified

    def test_complete_repeat_rejects_runtime_identity_drift_before_readback(self):
        host = RecordingRuntimeHost()
        release = self.verified()
        plan = existing.api.plan_installation(self.request, release, host)
        planned_identity = plan["observations"][0]["facts"]["runtime_identity"]
        self.assertEqual(planned_identity, host.facts["runtime_identity"])
        plan_sha = existing.api.digest(existing.api.canonical_bytes(plan))

        first = existing.api.apply_installation(release, plan, plan_sha, host)
        self.assertEqual(first["status"], "INSTALLED_UNREADY")
        self.assertEqual(host.last_journal["stage"], "complete")

        host.facts["runtime_identity"] = {
            "python_version": "3.9.6",
            "executable_cdhash": "c" * 40,
            "framework_cdhash": "b" * 40,
        }
        events_before_repeat = list(host.events)
        host.installed_validation_calls = 0

        with self.assertRaises(existing.api.InstallError) as caught:
            existing.api.apply_installation(release, plan, plan_sha, host)

        self.assertEqual(caught.exception.code, "INSTALLATION_DRIFT")
        self.assertEqual(host.installed_validation_calls, 0)
        self.assertEqual(host.events, events_before_repeat)

class MissingNativeGateProofContract(unittest.TestCase):
    """The reviewed bootstrap must reject missing gate evidence pre-capture."""

    def test_missing_gate_proof_rejects_before_capture_or_execute(self):
        fixture = existing.ProductionInlineBootstrapContract(
            "test_actual_inline_bootstrap_positive_executes_captured_launcher"
        )
        fixture.setUp()
        try:
            self.assertIsInstance(fixture, existing.BootstrapACLFixtureMixin)
            after_capture = []
            namespace = {
                "__name__": "__main__",
                "_bootstrap_acl_query": fixture.query_acl,
                "_bootstrap_metadata": fixture.observe_metadata,
                "_bootstrap_execute": fixture.execute_captured,
                "_bootstrap_after_capture": lambda: after_capture.append(True),
                "_bootstrap_target": "aarch64-apple-darwin",
            }
            argv = [
                "-c", "plan",
                "--release-dir", str(fixture.release),
                "--expected-manifest-sha256", fixture.expected,
            ]

            with patch.object(sys, "argv", argv):
                with self.assertRaisesRegex(
                    (ValueError, existing.api.InstallError),
                    "(?i)(gate|runtime|attestation)",
                ):
                    exec(compile(fixture.bootstrap_code, "<approved-bootstrap-test>", "exec"), namespace)

            self.assertEqual(fixture.acl_queries, [], "missing proof must reject before opening/capturing stage descriptors")
            self.assertEqual(after_capture, [])
            self.assertEqual(fixture.executions, [], "missing proof must reject before _bootstrap_execute")
        finally:
            fixture.doCleanups()


def executable_operator_lines(text):
    blocks = []
    for block in re.findall(r"^```sh\s*\n(.*?)^```", text, re.MULTILINE | re.DOTALL):
        flattened = block.replace("\\\n", " ")
        lines = [line.strip() for line in flattened.splitlines() if line.strip() and not line.lstrip().startswith("#")]
        blocks.append(lines)
    candidates = []
    for lines in blocks:
        operations = []
        for index, line in enumerate(lines):
            tokens = shlex.split(line)
            if "plan" in tokens or "apply" in tokens:
                operations.append((index, tokens))
        if operations:
            candidates.append((lines, operations))
    if len(candidates) != 1:
        raise AssertionError("operator plan/apply commands must use one fenced shell block")
    return candidates[0]


class OperatorGateInvocationContract(unittest.TestCase):
    """The documented root entry must scrub env and check independent digests."""

    def test_plan_and_apply_use_clean_env_approved_gate(self):
        lines, operations = executable_operator_lines(RUNBOOK.read_text())
        self.assertEqual(["plan" if "plan" in tokens else "apply" for _, tokens in operations], ["plan", "apply"])
        expected_prefix = [
            "/usr/bin/env", "-i",
            "PATH=/usr/bin:/bin:/usr/sbin:/sbin", "LANG=C", "LC_ALL=C",
            "$APPROVED_RUNTIME_GATE",
        ]
        for index, command in operations:
            with self.subTest(operation="plan" if "plan" in command else "apply"):
                self.assertEqual(command[:len(expected_prefix)], expected_prefix)
                self.assertEqual(
                    command[1:5], ["-i", "PATH=/usr/bin:/bin:/usr/sbin:/sbin", "LANG=C", "LC_ALL=C"],
                    "the native gate must start only after env -i has removed inherited DYLD_* and other variables",
                )

    def test_gate_and_bootstrap_digests_are_independent_and_checked_before_launch(self):
        lines, operations = executable_operator_lines(RUNBOOK.read_text())
        for index, _command in operations:
            preceding = lines[:index]
            with self.subTest(operation="plan" if "plan" in _command else "apply"):
                self.assertTrue(
                    any("ACTUAL_RUNTIME_GATE_SHA256" in line and "shasum -a 256" in line and "$APPROVED_RUNTIME_GATE" in line for line in preceding),
                    "the gate artifact must be hashed before launch",
                )
                self.assertIn(
                    'test "$ACTUAL_RUNTIME_GATE_SHA256" = "$EXPECTED_RUNTIME_GATE_SHA256" || exit 9',
                    preceding,
                )
                self.assertTrue(
                    any("ACTUAL_BOOTSTRAP_SHA256" in line and "shasum -a 256" in line and "$APPROVED_BOOTSTRAP_CODE" in line for line in preceding),
                    "captured bootstrap bytes must be hashed before launch",
                )
                self.assertIn(
                    'test "$ACTUAL_BOOTSTRAP_SHA256" = "$EXPECTED_BOOTSTRAP_SHA256" || exit 9',
                    preceding,
                )

        executable_lines = [line for block in re.findall(r"^```sh\s*\n(.*?)^```", RUNBOOK.read_text(), re.MULTILINE | re.DOTALL) for line in block.splitlines()]
        for expected in ("EXPECTED_RUNTIME_GATE_SHA256", "EXPECTED_BOOTSTRAP_SHA256"):
            with self.subTest(expected_digest=expected):
                self.assertFalse(
                    any(re.match(rf"\s*(?:export\s+)?{expected}=", line) for line in executable_lines),
                    "expected digests must arrive as independent approval inputs, not be self-derived in the runbook",
                )


if __name__ == "__main__":
    unittest.main()
