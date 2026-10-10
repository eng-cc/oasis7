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

    def test_macos_success_status_with_exact_denial_is_initially_safe(self):
        denial = "User oasis7-codex is not allowed to run sudo on Mac.\n"
        observed = self.observe(0, denial)
        self.assertTrue(host.sudo_policy_safe(observed, 401, 401, "/worker"))
        self.assertFalse(host.sudo_policy_safe(observed, 401, 401, "/worker", require_worker=True))
        for out, err in ((denial, "sudo: policy plugin failed\n"), (denial + "\n", ""),
                         ("", "sudo: a password is required\n"), (denial.replace("Mac.", "Other."), "")):
            with self.subTest(out=out, err=err):
                self.assertFalse(host.sudo_policy_safe(self.observe(0, out, err), 401, 401, "/worker"))

    def test_errors_and_ambiguous_denials_are_not_no_grants(self):
        denial = "User oasis7-codex is not allowed to run sudo on Mac.\n"
        for code, out, err in ((2, denial, ""), (2, "", denial), (1, "", denial.replace("oasis7-codex", "other")),
                               (1, "", denial.replace("Mac.", "Other.")), (1, "", denial + "sudo: policy plugin failed\n"),
                               (1, "unexpected", denial), (1, "", denial + "\n"), (1, denial + "\n", ""),
                               (1, "", "sudo: a password is required\n"), (1, "", "")):
            with self.subTest(code=code, out=out, err=err):
                self.assertFalse(host.sudo_policy_safe(self.observe(code, out, err), 401, 401, "/worker"))

    def test_effective_worker_rule_still_required_after_installation(self):
        output = 'User may run the following commands:\n (#401 : #401) NOPASSWD: NOSETENV: /worker ""\n'
        self.assertTrue(host.sudo_policy_safe(self.observe(0, output), 401, 401, "/worker", require_worker=True))
        self.assertFalse(host.sudo_policy_safe(self.observe(0, "User may run the following commands:\n (ALL) NOPASSWD: ALL"), 401, 401, "/worker"))

    def native_detailed(self, defaults='env_reset, env_keep+="HOME MAIL", env_keep+=SSH_AUTH_SOCK, lecture_file=/etc/sudo_lecture, !log_allowed'):
        return ('Matching Defaults entries for oasis7-codex on Mac:\n    ' + defaults + '\n\n'
                'User oasis7-codex may run the following commands on Mac:\n\n'
                'Sudoers entry: /private/etc/sudoers.d/oasis7-local-signer\n'
                '    RunAsUsers: #401\n    RunAsGroups: #401\n'
                '    Options: !setenv, !authenticate\n    Commands:\n'
                '        /worker\n        ""\n')

    def test_native_detailed_rule_and_evidenced_defaults(self):
        self.assertTrue(host.sudo_policy_safe(self.observe(0, self.native_detailed()), 401, 401, '/worker', require_worker=True))

    def test_actual_macos_wrapped_defaults_fixture(self):
        defaults = ('env_reset, env_keep+=BLOCKSIZE, env_keep+="COLORFGBG COLORTERM", '
                    'env_keep+=__CF_USER_TEXT_ENCODING, env_keep+="CHARSET LANG LANGUAGE LC_ALL LC_COLLATE\n    LC_CTYPE", '
                    'env_keep+="LC_MESSAGES LC_MONETARY LC_NUMERIC LC_TIME", env_keep+="LINES\n    COLUMNS", '
                    'env_keep+=LSCOLORS, env_keep+=SSH_AUTH_SOCK, env_keep+=TZ, '
                    'env_keep+="DISPLAY XAUTHORIZATION XAUTHORITY", env_keep+="EDITOR VISUAL", '
                    'env_keep+="HOME MAIL", lecture_file=/etc/sudo_lecture, !log_allowed')
        self.assertTrue(host.sudo_policy_safe(self.observe(0, self.native_detailed(defaults)), 401, 401, '/worker', require_worker=True))

    def test_detailed_policy_rejects_dangerous_defaults_and_extra_authority(self):
        valid = self.native_detailed()
        bad = [self.native_detailed('env_reset, env_keep+=' + name) for name in
               ('SUDO_UID', 'DYLD_LIBRARY_PATH', 'PATH', 'PYTHONPATH', 'TMPDIR')]
        bad += [self.native_detailed('!env_reset'), self.native_detailed('env_reset, exempt_group=staff'),
                valid.replace('!setenv, !authenticate', 'setenv, !authenticate'),
                valid.replace('RunAsUsers: #401', 'RunAsUsers: ALL'),
                valid.replace('/worker\n        ""', '/worker argument'),
                valid + valid[valid.index('Sudoers entry:'):],
                valid.replace('/worker', '/other'), valid.replace('oasis7-codex', 'other')]
        for out in bad:
            with self.subTest(out=out):
                self.assertFalse(host.sudo_policy_safe(self.observe(0, out), 401, 401, '/worker', require_worker=True))

    def test_invalid_hostname_and_inconsistent_observation_fail_closed(self):
        denial = "User oasis7-codex is not allowed to run sudo on Mac.\n"
        for code, out, err, caller, hostname in ((2, "", denial, "oasis7-codex", "Mac"),
                                                (1, "", denial, "other", "Mac"),
                                                (1, "", denial, "oasis7-codex", ""),
                                                (1, "", denial, "oasis7-codex", "Mac\n")):
            observed = host.SudoPolicyObservation(code, out, err, caller, hostname)
            self.assertFalse(host.sudo_policy_safe(observed, 401, 401, "/worker"))


if __name__ == "__main__":
    unittest.main()
