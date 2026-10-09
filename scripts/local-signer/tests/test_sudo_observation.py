import subprocess
import sys
from pathlib import Path
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import macos_host as host


class SudoObservation(unittest.TestCase):
    def observe(self, code, stdout="", stderr=""):
        backend = host.MacOSHost(runtime_identity={"test": "test"})
        result = subprocess.CompletedProcess([], code, stdout, stderr)
        with patch.object(host.subprocess, "run", return_value=result), patch.object(host.socket, "gethostname", return_value="Mac.bbrouter"):
            return backend.read_sudo_policy("oasis7-codex")

    def test_native_no_grants_is_safe_only_before_installation(self):
        for channel in ("stdout", "stderr"):
            observed = self.observe(1, **{channel: "User oasis7-codex is not allowed to run sudo on Mac.\n"})
            self.assertTrue(host.sudo_policy_safe(observed, 401, 401, "/worker"))
            self.assertFalse(host.sudo_policy_safe(observed, 401, 401, "/worker", require_worker=True))

    def test_errors_and_ambiguous_denials_are_not_no_grants(self):
        denial = "User oasis7-codex is not allowed to run sudo on Mac.\n"
        for code, out, err in ((0, denial, ""), (2, "", denial), (1, "", denial.replace("oasis7-codex", "other")),
                               (1, "", denial.replace("Mac.", "Other.")), (1, "", denial + "sudo: policy plugin failed\n"),
                               (1, "unexpected", denial), (1, "", denial + "\n"), (1, denial + "\n", ""),
                               (1, "", "sudo: a password is required\n"), (1, "", "")):
            with self.subTest(code=code, out=out, err=err):
                self.assertFalse(host.sudo_policy_safe(self.observe(code, out, err), 401, 401, "/worker"))

    def test_effective_worker_rule_still_required_after_installation(self):
        output = 'User may run the following commands:\n (#401 : #401) NOPASSWD: NOSETENV: /worker ""\n'
        self.assertTrue(host.sudo_policy_safe(self.observe(0, output), 401, 401, "/worker", require_worker=True))
        self.assertFalse(host.sudo_policy_safe(self.observe(0, "User may run the following commands:\n (ALL) NOPASSWD: ALL"), 401, 401, "/worker"))

    def test_invalid_hostname_and_inconsistent_observation_fail_closed(self):
        denial = "User oasis7-codex is not allowed to run sudo on Mac.\n"
        for code, out, err, caller, hostname in ((0, "", denial, "oasis7-codex", "Mac"),
                                                (1, "", denial, "other", "Mac"),
                                                (1, "", denial, "oasis7-codex", ""),
                                                (1, "", denial, "oasis7-codex", "Mac\n")):
            observed = host.SudoPolicyObservation(code, out, err, caller, hostname)
            self.assertFalse(host.sudo_policy_safe(observed, 401, 401, "/worker"))


if __name__ == "__main__":
    unittest.main()
