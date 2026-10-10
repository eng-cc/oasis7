"""Production command contracts with recording mocks; never execute host tools."""
import importlib.util
import sys
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch

SOURCE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SOURCE))
import macos_host as host
import installer as api
spec = importlib.util.spec_from_file_location("repair_cli_fixture", SOURCE / "install-release.py")
cli = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cli)

class HostRepairContract(unittest.TestCase):
    def setUp(self):
        self.backend = host.MacOSHost(runtime_identity={"fixture": "isolated"})
        self.plan = {"caller": {"uid": 504, "name": "oasis7-codex"}, "release_id": "repair-fixture", "installation_id": "install-fixture"}

    def test_disable_is_exact_include_replace_before_global_and_effective_check(self):
        events = []
        with patch.object(self.backend, "atomic_root_file", side_effect=lambda *a, **k: events.append(("write", a, k))), patch.object(self.backend, "run", side_effect=lambda a: events.append(("run", a))), patch.object(self.backend, "read_sudo_policy", return_value=SimpleNamespace(no_grants=True)):
            self.backend.disable_sudo(self.plan)
        self.assertEqual(events[0], ("write", (host.SUDO, b"# oasis7 local signer binding repair: worker authorization disabled\n", 0o440), {"replace": True}))
        self.assertEqual(events[1], ("run", ["/usr/sbin/visudo", "-c"]))

    def test_disable_rejects_remaining_effective_authority(self):
        with patch.object(self.backend, "atomic_root_file"), patch.object(self.backend, "run"), patch.object(self.backend, "read_sudo_policy", return_value=SimpleNamespace(no_grants=False)):
            with self.assertRaises(api.InstallError): self.backend.disable_sudo(self.plan)

    def doctor_result(self, **changes):
        value = dict(command="doctor", schema_version="oasis7.local_signer_doctor.v1", installation_id="install-fixture", ready=False, status_code="AUTHORIZATION_DENIED")
        value.update(changes)
        return SimpleNamespace(stdout=api.canonical_bytes(value), stderr=b"AUTHORIZATION_DENIED: operation failed\n", returncode=3)

    def test_actual_caller_command_is_numeric_and_clean_and_doctor_only(self):
        with patch.object(host.subprocess, "run", return_value=self.doctor_result()) as run:
            proof = self.backend.verify_caller_doctor(self.plan)
        self.assertEqual(proof["exit_code"], 3)
        self.assertEqual(run.call_args.args[0], ["/usr/bin/sudo", "-n", "-u", "#504", "/usr/bin/env", "-i", "PATH=/usr/bin:/bin:/usr/sbin:/sbin", "LANG=C", "LC_ALL=C", "/usr/local/libexec/oasis7-local-signer/repair-fixture/oasis7_local_signer", "doctor"])
        self.assertEqual(run.call_args.kwargs["stdin"], host.subprocess.DEVNULL)

    def test_doctor_rejects_ready_or_foreign_identity_and_unknown_fields(self):
        for change in ({"ready": True}, {"installation_id": "foreign"}, {"extra": True}, {"status_code": "PERSISTENCE_FAILED"}):
            with self.subTest(change=change), patch.object(host.subprocess, "run", return_value=self.doctor_result(**change)):
                with self.assertRaises(api.InstallError): self.backend.verify_caller_doctor(self.plan)

    def test_original_installation_consumer_rejects_any_repair_receipt(self):
        with patch.object(host.os.path, "lexists", return_value=True):
            with self.assertRaises(api.InstallError) as caught: self.backend.completed_installation()
        self.assertEqual(caught.exception.code, "RECOVERY_REQUIRED")

    def test_cli_repair_modes_are_mutually_exclusive(self):
        base = ["apply", "--release-dir", "/fixture", "--expected-manifest-sha256", "a" * 64, "--plan", "/old-plan", "--expected-plan-sha256", "b" * 64]
        for mode in ("--binding-repair-plan-out", "--binding-repair-apply", "--binding-repair-check", "--binding-repair-quarantine"):
            parsed = cli.parser().parse_args(base + [mode, "/repair-plan"])
            self.assertEqual(getattr(parsed, mode[2:].replace("-", "_")), "/repair-plan")
        with self.assertRaises(SystemExit): cli.parser().parse_args(base + ["--completion-only", "--binding-repair-apply", "/repair-plan"])

if __name__ == "__main__": unittest.main()
