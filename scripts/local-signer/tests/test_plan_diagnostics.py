import importlib.util
from pathlib import Path
import sys
import unittest

SOURCE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SOURCE))
import installer
spec = importlib.util.spec_from_file_location("diagnostic_cli", SOURCE / "install-release.py")
cli = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cli)

class PlanDiagnostics(unittest.TestCase):
    def test_typed_guard_reason_and_cause_errno(self):
        try:
            try:
                raise PermissionError(13, "sensitive/path")
            except PermissionError as cause:
                raise installer.InstallError("INSTALLATION_DRIFT", "path or filesystem unobservable") from cause
        except installer.InstallError as error:
            value = cli.blocked_output(error, installer)
        self.assertEqual(value["reason"], "path or filesystem unobservable")
        self.assertEqual(value["cause_errno"], 13)
        self.assertEqual(value["exception_type"], "InstallError")
        self.assertFalse(value["host_mutated"])
        self.assertNotIn("sensitive/path", str(value))

    def test_unexpected_exception_does_not_echo_values(self):
        try:
            raise RuntimeError("SECRET raw host output")
        except RuntimeError as error:
            value = cli.blocked_output(error, installer)
        self.assertEqual(value["exception_type"], "RuntimeError")
        self.assertNotIn("SECRET", str(value))
        self.assertNotIn("reason", value)
        self.assertTrue(value["failure_location"].endswith("test_unexpected_exception_does_not_echo_values"))

    def test_preflight_flags_are_safe_booleans(self):
        error = installer.InstallError("INSTALLATION_DRIFT", "host isolation preflight blocked")
        error.preflight_flags = {"safe": False, "sudo_safe": True}
        self.assertEqual(cli.blocked_output(error, installer)["preflight_flags"], error.preflight_flags)
