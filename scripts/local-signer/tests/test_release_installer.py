"""Behavioral source contract; fixture backend is importer injection only."""
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest

SOURCE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SOURCE))
import installer as api


class FakeHost:
    def __init__(self):
        self.events = []
        self.facts = {"platform": "darwin", "target": "aarch64-apple-darwin", "root": True, "safe": True, "acl_safe": True, "sudo_safe": True, "identity_available": True}
        # TPM-approved additive fixture correction: identities must be observed, never inferred.
        self.facts.update(caller_uid=501, caller_gid=20, signer_uid=499, signer_gid=499)
        self.fault = None
        self.receipt = None
        self.last_journal = None

    def observe(self, request, release):
        return dict(self.facts)

    def lock(self):
        from contextlib import nullcontext
        return nullcontext()

    def journal(self, record):
        self.events.append(("journal", record))
        self.last_journal = record
        if record.get("stage") == "complete":
            self.receipt = record

    def apply_action(self, kind, value):
        self.events.append((kind, value))
        if self.fault == kind:
            raise OSError("injected durable-stage failure")

    def completed_installation(self):
        return self.last_journal

    def validate_installed(self, plan, release):
        return self.facts.get("installed_safe", True)

    def __getattr__(self, name):
        names = {"create_identity": "identity", "create_layout": "layout", "publish_release": "release", "publish_binding": "binding", "validate_installation": "validated", "publish_sudo": "sudo", "report": "complete"}
        if name in names:
            return lambda value: self.apply_action(names[name], value)
        raise AttributeError(name)


class InstallerContract(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.bundle = self.root / "release"
        self.bundle.mkdir()
        entries = []
        for name in sorted(api.FILES):
            data = ("fixed:" + name).encode()
            (self.bundle / name).write_bytes(data)
            entries.append({"name": name, "size_bytes": len(data), "sha256": api.digest(data)})
        self.manifest = {"schema_version": api.RELEASE_SCHEMA, "release_id": "r1", "target": "aarch64-apple-darwin", "source_revision": "a" * 40, "installation_schema_version": "oasis7.local_signer_installation.v3", "control_schema_version": "oasis7.local_signer_control.v1", "files": entries}
        self.write_manifest()
        self.request = {"installation_id": "i1", "deployment_id": "d1", "store_dir": "/Library/Application Support/oasis7-local-signer", "work_dir": "/Users/fixture/Documents/keys/oasis7-local-signer", "caller_user": "fixture", "signer_user": "_oasis7_signer"}

    def write_manifest(self):
        self.raw = api.canonical_bytes(self.manifest)
        (self.bundle / "manifest.json").write_bytes(self.raw)
        self.sha = api.digest(self.raw)

    def verified(self):
        try:
            return api.validate_release(self.bundle, self.sha, "aarch64-apple-darwin")
        except api.InstallError as error:
            if error.code == "UNSUPPORTED_PLATFORM_OR_FS":
                self.fail("approved fixture release verification is not implemented")
            raise

    def assert_blocked(self, action, code="INVALID_RELEASE"):
        with self.assertRaises(api.InstallError) as caught:
            action()
        self.assertEqual(caught.exception.code, code)

    def test_exact_release_bytes_are_verified(self):
        release = self.verified()
        self.assertEqual(release["manifest_sha256"], self.sha)
        self.assertEqual(set(release["verified_bytes"]), set(api.FILES))

    def test_independent_expected_digest_required(self):
        self.assert_blocked(lambda: api.validate_release(self.bundle, "0" * 64, "aarch64-apple-darwin"))

    def test_mutated_binary_rejected(self):
        (self.bundle / api.FILES[0]).write_bytes(b"changed")
        self.assert_blocked(self.verified)

    def test_symlink_and_hardlink_rejected(self):
        path = self.bundle / api.FILES[0]
        original = self.root / "external"
        original.write_bytes(path.read_bytes())
        path.unlink()
        path.symlink_to(original)
        self.assert_blocked(self.verified)
        path.unlink()
        import os
        os.link(original, path)
        self.assert_blocked(self.verified)

    def test_unknown_duplicate_and_extra_members_rejected(self):
        self.manifest["unexpected"] = True
        self.write_manifest()
        self.assert_blocked(self.verified)
        del self.manifest["unexpected"]
        self.write_manifest()
        duplicate = self.raw[:-2] + b',"release_id":"evil"}\n'
        (self.bundle / "manifest.json").write_bytes(duplicate)
        self.assert_blocked(lambda: api.validate_release(self.bundle, api.digest(duplicate), "aarch64-apple-darwin"))
        self.write_manifest()
        (self.bundle / "extra").write_bytes(b"extra")
        self.assert_blocked(self.verified)

    def test_wrong_target_rejected(self):
        self.assert_blocked(lambda: api.validate_release(self.bundle, self.sha, "x86_64-apple-darwin"))

    def test_plan_has_zero_mutations_and_binds_approval(self):
        host = FakeHost()
        plan = api.plan_installation(self.request, self.verified(), host)
        self.assertEqual(host.events, [])
        self.assertFalse(plan["signing_enabled"])
        self.assertEqual(plan["manifest_sha256"], self.sha)
        self.assertEqual(plan["schema_version"], api.PLAN_SCHEMA)

    def test_unsafe_ancestry_acl_sudo_account_block_before_effects(self):
        release = self.verified()
        for fact in ("safe", "acl_safe", "sudo_safe", "identity_available"):
            with self.subTest(fact=fact):
                host = FakeHost()
                host.facts[fact] = False
                self.assert_blocked(lambda: api.plan_installation(self.request, release, host), "INSTALLATION_DRIFT")
                self.assertEqual(host.events, [])

    def test_apply_disabled_and_sudo_last(self):
        host = FakeHost()
        release = self.verified()
        plan = api.plan_installation(self.request, release, host)
        result = api.apply_installation(release, plan, api.digest(api.canonical_bytes(plan)), host)
        self.assertEqual(result["status"], "INSTALLED_UNREADY")
        self.assertFalse(result["signing_enabled"])
        effects = [name for name, _ in host.events if name != "journal"]
        self.assertEqual(effects, list(api.ACTIONS))
        self.assertFalse(any(name in effects for name in ("key", "policy", "grant", "enable")))

    def test_plan_hash_and_live_drift_refuse_apply(self):
        host = FakeHost()
        release = self.verified()
        plan = api.plan_installation(self.request, release, host)
        self.assert_blocked(lambda: api.apply_installation(release, plan, "0" * 64, host), "INSTALLATION_DRIFT")
        self.assertEqual(host.events, [])
        host.facts["safe"] = False
        self.assert_blocked(lambda: api.apply_installation(release, plan, api.digest(api.canonical_bytes(plan)), host), "INSTALLATION_DRIFT")
        self.assertEqual(host.events, [])

    def test_fault_returns_partial_recovery_and_retry_cannot_resume(self):
        for point in api.ACTIONS[:-1]:
            with self.subTest(point=point):
                host = FakeHost()
                release = self.verified()
                plan = api.plan_installation(self.request, release, host)
                host.fault = point
                result = api.apply_installation(release, plan, api.digest(api.canonical_bytes(plan)), host)
                self.assertEqual(result["status"], "RECOVERY_REQUIRED")
                self.assertFalse(result["signing_enabled"])
                before = len(host.events)
                again = api.apply_installation(release, plan, api.digest(api.canonical_bytes(plan)), host)
                self.assertEqual(again["status"], "RECOVERY_REQUIRED")
                self.assertEqual(len(host.events), before)

    def test_complete_repeat_validates_original_plan_and_installed_facts(self):
        host = FakeHost()
        release = self.verified()
        plan = api.plan_installation(self.request, release, host)
        sha = api.digest(api.canonical_bytes(plan))
        api.apply_installation(release, plan, sha, host)
        before = len(host.events)
        host.facts["identity_available"] = False
        self.assertEqual(api.apply_installation(release, plan, sha, host)["status"], "VERIFIED_UNCHANGED")
        self.assertEqual(len(host.events), before)
        host.facts["installed_safe"] = False
        self.assert_blocked(lambda: api.apply_installation(release, plan, sha, host), "INSTALLATION_DRIFT")

    def test_linux_apply_refuses_before_mutation(self):
        host = FakeHost()
        host.facts["platform"] = "linux"
        self.assert_blocked(lambda: api.plan_installation(self.request, self.verified(), host), "UNSUPPORTED_PLATFORM_OR_FS")
        self.assertEqual(host.events, [])

    def test_packaging_is_create_only_and_roundtrips(self):
        binary = self.root / "binaries"
        modules = self.root / "modules"
        binary.mkdir()
        modules.mkdir()
        for name in api.FILES:
            destination = modules if name.endswith(".py") else binary
            (destination / name).write_bytes((self.bundle / name).read_bytes())
        output = self.root / "packaged"
        kwargs = dict(release_id="r1", target="aarch64-apple-darwin", source_revision="a" * 40)
        try:
            result = api.package_release(binary, modules, output, **kwargs)
        except api.InstallError:
            self.fail("fixed release packaging is not implemented")
        self.assertEqual(api.validate_release(output, result["manifest_sha256"], kwargs["target"])["manifest"]["release_id"], "r1")
        self.assert_blocked(lambda: api.package_release(binary, modules, output, **kwargs))

    def test_production_has_no_fake_backend_selector(self):
        spec = importlib.util.spec_from_file_location("installer_cli", SOURCE / "install-release.py")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        parser = module.parser()
        choices = [action for action in parser._actions if isinstance(action, __import__("argparse")._SubParsersAction)][0].choices
        options = {name for command in choices.values() for action in command._actions for name in action.option_strings}
        self.assertFalse(options & {"--fake", "--backend", "--fixture-root", "--test-mode"})
        self.assertNotIn("environ", (SOURCE / "install-release.py").read_text())

    def test_root_entrypoint_uses_fixed_isolated_interpreter(self):
        source = (SOURCE / "install-release.py").read_text()
        self.assertEqual(source.splitlines()[0], "#!/usr/bin/python3 -I")
        self.assertNotIn("/usr/bin/env", source)
        from macos_host import MacOSHost
        self.assertTrue(hasattr(MacOSHost, "validate_interpreter"), "production must validate protected interpreter identity before installation effects")


if __name__ == "__main__":
    unittest.main()


class SupplementalContract(InstallerContract):
    def test_missing_or_malformed_identity_evidence_blocks(self):
        release = self.verified()
        for field in ("caller_uid", "signer_uid", "signer_gid"):
            for bad in (None, True, -1, 0, "499"):
                host = FakeHost()
                host.facts[field] = bad
                self.assert_blocked(lambda: api.plan_installation(self.request, release, host), "INSTALLATION_DRIFT")
                self.assertEqual(host.events, [])
    def test_nonroot_has_zero_effects(self):
        host = FakeHost()
        release = self.verified()
        plan = api.plan_installation(self.request, release, host)
        host.facts["root"] = False
        self.assert_blocked(lambda: api.apply_installation(release, plan, api.digest(api.canonical_bytes(plan)), host), "UNSUPPORTED_PLATFORM_OR_FS")
        self.assertEqual(host.events, [])

    def test_final_report_and_complete_journal_faults_not_complete(self):
        for point in ("complete", "complete_journal"):
            host = FakeHost()
            original = host.journal
            if point == "complete":
                host.fault = "complete"
            else:
                def fail_final(record):
                    original(record)
                    if record["stage"] == "complete":
                        raise OSError("final fsync failed")
                host.journal = fail_final
            release = self.verified()
            plan = api.plan_installation(self.request, release, host)
            result = api.apply_installation(release, plan, api.digest(api.canonical_bytes(plan)), host)
            self.assertEqual(result["status"], "RECOVERY_REQUIRED")
            self.assertNotEqual(host.last_journal["stage"], "complete")

    def test_size_id_and_plan_grammar_boundaries(self):
        for size in (True, -1, 0, 128 * 1024 * 1024 + 1):
            self.manifest["files"][0]["size_bytes"] = size
            self.write_manifest()
            self.assert_blocked(self.verified)
        self.manifest["files"][0]["size_bytes"] = len((self.bundle / self.manifest["files"][0]["name"]).read_bytes())
        self.manifest["release_id"] = "../../evil"
        self.write_manifest()
        self.assert_blocked(self.verified)

    def test_same_descriptor_regular_read(self):
        from unittest.mock import patch
        path = self.root / "read.bin"
        path.write_bytes(b"approved")
        original = api.os.read
        moved = False
        def replace_after_open(fd, amount):
            nonlocal moved
            if not moved:
                path.rename(self.root / "old.bin")
                path.write_bytes(b"attacker")
                moved = True
            return original(fd, amount)
        with patch.object(api.os, "read", replace_after_open):
            # Rename changes ctime, so stricter same-FD validation rejects the race.
            self.assert_blocked(lambda: api.read_file(path, 1024))

    def test_backend_fixed_commands_and_conservative_parsers(self):
        import macos_host as host
        commands = host.identity_commands(dict(name="_oasis7_signer", uid=499, gid=498))
        self.assertTrue(all(command[0] == "/usr/bin/dscl" for command in commands))
        self.assertIn(["/usr/bin/dscl", ".", "-create", "/Users/_oasis7_signer", "PrimaryGroupID", "498"], commands)
        self.assertTrue(host.acl_safe("drwx------ 2 root wheel 64 date path"))
        self.assertFalse(host.acl_safe("drwx------+ 2 root wheel 64 date path\n 0: user:evil allow write"))
        self.assertFalse(host.acl_safe(""))
        self.assertFalse(host.sudo_policy_safe("User may run the following commands:\n (ALL) NOPASSWD: ALL", 499, 498, "/worker"))
        self.assertFalse(host.sudo_policy_safe("UNOBSERVABLE", 499, 498, "/worker"))
        self.assertTrue(host.sudo_policy_safe('User may run the following commands:\n ( #499 : #498 ) NOPASSWD: /worker', 499, 498, "/worker") is False)
        self.assertTrue(host.sudo_policy_safe('User may run the following commands:\n ( #499 : #498 ) PASSWD: /bin/ls', 499, 498, "/worker"))

    def test_actual_isolated_cli_hostile_environment(self):
        import subprocess
        import os
        hostile = self.root / "hostile"
        hostile.mkdir()
        marker = self.root / "injected"
        (hostile / "sitecustomize.py").write_text("from pathlib import Path; Path(" + repr(str(marker)) + ").write_text('bad')")
        environment = dict(os.environ, PYTHONPATH=str(hostile), PYTHONHOME=str(hostile))
        result = subprocess.run(["/usr/bin/python3", "-I", str(SOURCE / "install-release.py"), "--help"], env=environment, capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("plan", result.stdout)
        self.assertFalse(marker.exists())
        result = subprocess.run(["/usr/bin/python3", "-I", str(SOURCE / "install-release.py"), "apply", "--release-dir", str(self.bundle), "--expected-manifest-sha256", self.sha, "--plan", str(self.root / "missing-plan"), "--expected-plan-sha256", "0" * 64], env=environment, capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 9)
        self.assertEqual(json.loads(result.stdout)["status"], "BLOCKED")
        self.assertFalse(marker.exists())

    def test_real_packaged_loader_and_cli_without_host_observation(self):
        import subprocess
        import os
        binary = self.root / "loader-binaries"
        binary.mkdir()
        for name in api.FILES:
            if not name.endswith(".py"):
                (binary / name).write_bytes(b"fixture-binary")
        package = self.root / "loader-release"
        import platform
        target = "aarch64-apple-darwin" if platform.machine() == "arm64" else "x86_64-apple-darwin"
        packaged = api.package_release(binary, SOURCE, package, release_id="loader-r1", target=target, source_revision="a" * 40)
        argv = ["/usr/bin/python3", "-I", str(package / "install-release.py"), "apply", "--release-dir", str(package), "--expected-manifest-sha256", packaged["manifest_sha256"], "--plan", str(self.root / "intentionally-absent-plan"), "--expected-plan-sha256", "0" * 64]
        result = subprocess.run(argv, env={"PATH": "/invalid", "PYTHONPATH": "/invalid", "PYTHONHOME": "/invalid"}, capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 9, result.stderr)
        self.assertEqual(json.loads(result.stdout)["status"], "BLOCKED")
        self.assertFalse((self.root / "intentionally-absent-plan").exists())


def generate_boundary_fixture(output):
    """Importer-only source fixture from the same config builder and layout as production."""
    from types import SimpleNamespace
    import macos_host
    plan = dict(installation_id="fixture-install", deployment_id="fixture-deployment", release_id="fixture-release", store_dir="/Library/Application Support/oasis7-local-signer", caller=dict(uid=501, work_dir="/Users/fixture/Documents/keys/oasis7-local-signer"), signer=dict(uid=499, gid=499))
    release = {"verified_bytes": {"oasis7_local_signer_worker": b"fixture-worker"}}
    config = api.build_installation_config(plan, release, SimpleNamespace(st_dev=1, st_ino=2), SimpleNamespace(st_dev=1, st_ino=3))
    value = dict(installation_config=config, inventory=[dict(relative=str(path.relative_to(plan["store_dir"])), owner_uid=uid, owner_gid=gid, mode=mode) for path, uid, gid, mode in macos_host.layout(plan)], initial_files=[])
    Path(output).write_bytes(api.canonical_bytes(value))
    return value
