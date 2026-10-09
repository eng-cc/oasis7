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
        self.facts.update(caller_uid=501, caller_gid=20, signer_uid=499, signer_gid=499,
                          runtime_identity={"schema_version": "fixture.runtime.v1", "runtime_cdhash": "a" * 40})
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
        self.assertEqual(result.returncode, 9, result.stderr)
        self.assertEqual(json.loads(result.stdout)["status"], "BLOCKED", "direct Python startup lacks the separately approved native gate")
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


class BootstrapACLFixtureMixin:
    """ACL-R1: independently trusted bootstrap must execute captured approved bytes.

    Before the new callable exists, legacy_launch models the current documented
    operator digest-check followed by Python opening the staging launcher path.
    It deliberately uses the real existing release verifier, then reopens the
    launcher as that current startup does. It is not a production fallback.
    This mixin is not a discovered test class and requires no staged helper.
    The sole production suite executes the actual operator inline bytes, with
    importer-only observation/execute seams and no production fake flags.
    """

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.parent = Path(self.temporary.name).resolve()
        self.release = self.parent / "approved-r1"
        self.release.mkdir()
        self.initial_launcher = b"execution_marker = 'approved-launcher'\n"
        self.replacement_launcher = b"execution_marker = 'replacement-launcher'\n"
        entries = []
        for name in api.FILES:
            raw = self.initial_launcher if name == "install-release.py" else ("approved:" + name).encode()
            (self.release / name).write_bytes(raw)
            entries.append(dict(name=name, size_bytes=len(raw), sha256=api.digest(raw)))
        manifest = dict(schema_version=api.RELEASE_SCHEMA, release_id="approved-r1", target="aarch64-apple-darwin", source_revision="a" * 40, installation_schema_version="oasis7.local_signer_installation.v3", control_schema_version="oasis7.local_signer_control.v1", files=sorted(entries, key=lambda entry: entry["name"]))
        raw = api.canonical_bytes(manifest)
        (self.release / "manifest.json").write_bytes(raw)
        self.expected = api.digest(raw)
        spec = importlib.util.spec_from_file_location("acl_bootstrap_source", SOURCE / "install-release.py")
        self.source = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.source)
        self.executions = []
        self.acl_queries = []
        self.acl_override = {}
        self.metadata_override = {}
        self.after_capture = None

    @staticmethod
    def identity(path):
        metadata = Path(path).stat()
        return metadata.st_dev, metadata.st_ino

    def query_acl(self, fd):
        import os
        metadata = os.fstat(fd)
        identity = (metadata.st_dev, metadata.st_ino)
        self.acl_queries.append(identity)
        result = self.acl_override.get(identity, ())
        if isinstance(result, Exception):
            raise result
        return result

    def observe_metadata(self, fd):
        import os
        import stat
        from types import SimpleNamespace
        info = os.fstat(fd)
        # Pure recording fixture: model root-owned POSIX-safe metadata without
        # changing a real owner/mode/ACL or enabling privileged test execution.
        observed = {name: getattr(info, name) for name in ("st_dev", "st_ino", "st_size", "st_nlink", "st_mtime_ns", "st_ctime_ns")}
        observed.update(st_uid=0, st_gid=0, st_mode=(stat.S_IFDIR | 0o755) if stat.S_ISDIR(info.st_mode) else (stat.S_IFREG | 0o444))
        observed.update(self.metadata_override.get((info.st_dev, info.st_ino), {}))
        return SimpleNamespace(**observed)

    def execute_captured(self, launcher_bytes, captured_files):
        namespace = {}
        exec(compile(launcher_bytes, "<captured-test-launcher>", "exec"), namespace)
        self.executions.append(dict(marker=namespace["execution_marker"], launcher=launcher_bytes, files=captured_files))

    def legacy_launch(self):
        # Existing reviewed source protects member digests; the operator recipe
        # still opens candidate launcher by pathname after a separate check.
        approved = api.validate_release(self.release, self.expected, "aarch64-apple-darwin")
        if self.after_capture:
            self.after_capture()
        self.execute_captured((self.release / "install-release.py").read_bytes(), approved["verified_bytes"])

    def assert_preexec_rejection(self):
        try:
            self.launch()
        except (ValueError, OSError, api.InstallError):
            pass
        self.assertEqual(self.executions, [], "unsafe/unobservable trust must reject before candidate code executes")

    def test_no_acl_positive_executes_exact_approved_bytes(self):
        self.launch()
        self.assertEqual(len(self.executions), 1)
        self.assertEqual(self.executions[0]["launcher"], self.initial_launcher)
        self.assertEqual(self.executions[0]["marker"], "approved-launcher")
        self.assertEqual(self.executions[0]["files"]["install-release.py"], self.initial_launcher)

    def test_positive_reads_acl_for_release_and_parent_descriptors(self):
        self.launch()
        self.assertIn(self.identity(self.release), self.acl_queries, "release POSIX metadata cannot substitute for an observed ACL")
        self.assertIn(self.identity(self.parent), self.acl_queries, "parent directory replacement rights must be inspected before exec")

    def test_positive_reads_acl_for_every_ancestor_descriptor(self):
        self.launch()
        for path in self.release.parents:
            self.assertIn(self.identity(path), self.acl_queries, "all bootstrap ancestor ACLs must be observable before exec")

    def test_replacement_acl_on_release_rejects_before_exec(self):
        self.acl_override[self.identity(self.release)] = ({"principal": "user:fixture-attacker", "permissions": ("add_file", "delete_child")},)
        self.assert_preexec_rejection()

    def test_replacement_acl_on_parent_rejects_before_exec(self):
        self.acl_override[self.identity(self.parent)] = ({"principal": "user:fixture-attacker", "permissions": ("add_subdirectory", "delete_child")},)
        self.assert_preexec_rejection()

    def test_unobservable_acl_denied_fails_closed(self):
        self.acl_override[self.identity(self.release)] = PermissionError("fixture ACL query denied")
        self.assert_preexec_rejection()

    def test_malformed_acl_evidence_fails_closed(self):
        self.acl_override[self.identity(self.release)] = "malformed fixture ACL result"
        self.assert_preexec_rejection()

    def test_path_replacement_after_capture_cannot_change_executed_launcher(self):
        def replace():
            original = self.release / "install-release.py"
            original.rename(self.release / "old-launcher")
            original.write_bytes(self.replacement_launcher)
        self.after_capture = replace
        self.launch()
        self.assertEqual(self.executions[0]["launcher"], self.initial_launcher, "launch must consume retained verified bytes, never reopen candidate pathname")
        self.assertEqual(self.executions[0]["marker"], "approved-launcher")

    def test_parent_replacement_after_capture_preserves_all_executed_bytes(self):
        initial_module = (self.release / "installer.py").read_bytes()
        def replace():
            old = self.parent / "previous-approved-release"
            self.release.rename(old)
            self.release.mkdir()
            (self.release / "install-release.py").write_bytes(self.replacement_launcher)
            (self.release / "installer.py").write_bytes(b"replacement module")
        self.after_capture = replace
        self.launch()
        self.assertEqual(self.executions[0]["launcher"], self.initial_launcher)
        self.assertEqual(self.executions[0]["files"]["installer.py"], initial_module)

    def test_root_owner_is_a_new_preexec_requirement(self):
        import stat
        self.metadata_override[self.identity(self.release)] = dict(st_uid=501, st_mode=stat.S_IFDIR | 0o755)
        self.assert_preexec_rejection()

    def test_nonwritable_parent_is_a_new_preexec_requirement(self):
        import stat
        self.metadata_override[self.identity(self.parent)] = dict(st_mode=stat.S_IFDIR | 0o777)
        self.assert_preexec_rejection()

    def test_symlink_member_preexec_guard_remains(self):
        member = self.release / "installer.py"
        original = self.parent / "outside-module"
        member.rename(original)
        member.symlink_to(original)
        self.assert_preexec_rejection()

    def test_member_digest_preexec_guard_remains(self):
        (self.release / "installer.py").write_bytes(b"changed approved module")
        self.assert_preexec_rejection()

    def test_manifest_digest_preexec_guard_remains(self):
        self.expected = "0" * 64
        self.assert_preexec_rejection()


class ProductionInlineBootstrapContract(BootstrapACLFixtureMixin, unittest.TestCase):
    """Require the real operator entry's exact -I -c bytes, not an unused helper.

    Seams exist only in an importer-created execution namespace. No CLI/env
    option selects fake root metadata, ACLs, target or candidate execution.
    """

    def setUp(self):
        super().setUp()
        import ast
        tree = ast.parse((SOURCE / "install-release.py").read_text())
        sources = [node.value for node in tree.body if isinstance(node, ast.Assign) and any(isinstance(name, ast.Name) and name.id == "TRUSTED_BOOTSTRAP" for name in node.targets)]
        self.bootstrap_code = ast.literal_eval(sources[0]) if sources else ""
        # This is a synthetic test approval injected by the harness. The
        # operator snippet must consume these independent inputs as supplied;
        # it must never derive or rebind either one from staged candidate data.
        self.bootstrap_source = self.parent / "independently-reviewed-bootstrap.txt"
        self.bootstrap_source.write_bytes(self.bootstrap_code.encode())
        self.bootstrap_approval = api.digest(self.bootstrap_code.encode())
        self.runtime_gate = self.parent / "independently-approved-runtime-gate"
        self.runtime_gate.write_bytes(b"synthetic native gate artifact")
        self.runtime_gate_approval = api.digest(self.runtime_gate.read_bytes())
        self.before_inline_execution = None
        self.operator_events = []

    def executable_blocks(self, text=None):
        import re
        if text is None:
            text = (SOURCE.parents[1] / "doc/p2p/blockchain/local-file-signing-backend.runbook.md").read_text()
        blocks = []
        for block in re.findall(r"^```sh\s*\n(.*?)^```", text, re.MULTILINE | re.DOTALL):
            lines = []
            for line in block.replace("\\\n", " ").splitlines():
                line = line.strip()
                if line and not line.startswith("#"):
                    lines.append(line)
            blocks.append(lines)
        return blocks

    def executable_lines(self, text=None):
        return [line for block in self.executable_blocks(text) for line in block]

    def operator_block(self, text=None):
        import shlex
        blocks = self.executable_blocks(text)
        candidates = []
        # Plan and apply must be in one reviewed shell block so the approval
        # guard and invocation share one auditable execution context.
        for block_index, block in enumerate(blocks):
            operations = []
            for index, line in enumerate(block):
                tokens = shlex.split(line)
                if "plan" in tokens or "apply" in tokens:
                    operations.append((index, tokens))
            if operations:
                candidates.append((block_index, block, operations))
        self.assertEqual(len(candidates), 1, "all executable operator plan/apply commands must be in one shell block")
        _, block, operations = candidates[0]
        # The human-approved source and expected digest are external inputs.
        # Reject rebinding them anywhere in executable runbook blocks, even in
        # an earlier block or with a shell export/command substitution.
        import re
        for line in self.executable_lines(text):
            if re.match(r"^(?:export\s+)?(?:APPROVED_RUNTIME_GATE|EXPECTED_RUNTIME_GATE_SHA256|APPROVED_BOOTSTRAP_SOURCE|EXPECTED_BOOTSTRAP_SHA256)\s*=", line):
                raise ValueError("runbook must not assign independently approved gate or bootstrap inputs")
        return block

    def operator_commands(self, text=None):
        import shlex
        # Parse every executable plan/apply line, including alternate Python,
        # sudo-prefix or variable executables. Prose/comments cannot satisfy it.
        block = self.operator_block(text)
        commands = []
        for index, line in enumerate(block):
            tokens = shlex.split(line)
            if "plan" in tokens or "apply" in tokens:
                commands.append((index, tokens))
        self.assertEqual(["plan" if "plan" in tokens else "apply" for _, tokens in commands], ["plan", "apply"], "all executable operator plan/apply commands must be represented")
        return commands

    def variables(self):
        return dict(APPROVED_RUNTIME_GATE=str(self.runtime_gate), EXPECTED_RUNTIME_GATE_SHA256=self.runtime_gate_approval, APPROVED_BOOTSTRAP_SOURCE=str(self.bootstrap_source), EXPECTED_BOOTSTRAP_SHA256=self.bootstrap_approval, STAGING=str(self.release), EXPECTED_MANIFEST_SHA256=self.expected, APPROVED_WORK_DIR="/Users/fixture/Documents/keys/oasis7-local-signer", APPROVED_CALLER="fixture", APPROVED_INSTALLATION_ID="fixture-install", APPROVED_DEPLOYMENT_ID="fixture-deployment", NEW_PLAN_FILE=str(self.parent / "plan.json"), INDEPENDENTLY_APPROVED_PLAN_SHA256="0" * 64)

    def runtime_attestation_bytes(self):
        override = getattr(self, "runtime_attestation_override", None)
        if override is not None:
            return override
        return api.canonical_bytes({
            "apple_anchor": "apple",
            "bootstrap_sha256": self.bootstrap_approval,
            "dependency_policy_id": "clt-python39-apple-dyld-v1",
            "framework_cdhash": "a43551195b8d2eefd9356d81c1098ffc9e0a8b47",
            "framework_dev": "1",
            "framework_ino": "3",
            "framework_mode": str(0o040555),
            "framework_path": "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9",
            "framework_resource_seal": "valid",
            "framework_uid": "0",
            "os_build": "fixture-build",
            "policy_id": "clt-python3.9-v1",
            "requirement_id": "com.apple.python3",
            "runtime_arch": "arm64",
            "runtime_cdhash": "77e5dcc021cbfa7e2c3940b5ea150e3da037f3cf",
            "runtime_dev": "1",
            "runtime_ino": "2",
            "runtime_mode": str(0o100555),
            "runtime_path": "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/bin/python3.9",
            "runtime_uid": "0",
            "runtime_version": "3.9",
            "schema_version": "oasis7.local-signer.runtime-attestation.v1",
            "system_volume_trust": "current-booted-apple-os",
        })

    def test_runtime_attestation_schema_and_fixed_identity_reject_before_stage_capture(self):
        import json
        valid = json.loads(self.runtime_attestation_bytes())
        cases = {
            "duplicate": b'{"apple_anchor":"apple","apple_anchor":"apple"}\n',
            "extra": api.canonical_bytes(dict(valid, unexpected="value")),
            "wrong type": api.canonical_bytes(dict(valid, runtime_uid=0)),
            "noncanonical": (json.dumps(valid, sort_keys=True, indent=2) + "\n").encode(),
            "missing final LF": api.canonical_bytes(valid).rstrip(b"\n"),
            "oversized": b"{" + b" " * 4096 + b"}\n",
            "wrong fixed CDHash": api.canonical_bytes(dict(valid, runtime_cdhash="0" * 40)),
            "unsafe mode": api.canonical_bytes(dict(valid, runtime_mode=str(0o100777))),
            "unsafe OS build": api.canonical_bytes(dict(valid, os_build="build\\n")),
        }
        for label, raw in cases.items():
            with self.subTest(label=label):
                self.acl_queries.clear()
                self.executions.clear()
                self.runtime_attestation_override = raw
                with self.assertRaisesRegex(ValueError, "(?i)(runtime|attestation)"):
                    self.launch()
                self.assertEqual(self.acl_queries, [], "runtime proof must reject before stage descriptor ACL queries")
                self.assertEqual(self.executions, [], "runtime proof must reject before captured candidate execution")
        del self.runtime_attestation_override

    def test_fd3_attestation_requires_fifo_effective_owner_and_readonly_access(self):
        import ast
        import fcntl
        import json
        import os
        import re
        import stat
        import types
        from unittest.mock import Mock, patch
        tree = ast.parse(self.bootstrap_code)
        function = next(node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name == "read_runtime_attestation")
        module = ast.Module(body=[function], type_ignores=[])
        evidence = self.runtime_attestation_bytes()
        for label, mode, uid, access in (
            ("not fifo", stat.S_IFREG | 0o600, 501, os.O_RDONLY),
            ("wrong owner", stat.S_IFIFO | 0o600, 502, os.O_RDONLY),
            ("not readonly", stat.S_IFIFO | 0o600, 501, os.O_WRONLY),
        ):
            with self.subTest(label=label):
                fake_os = Mock()
                fake_os.fstat.return_value = types.SimpleNamespace(st_mode=mode, st_uid=uid)
                fake_os.geteuid.return_value = 501
                fake_os.O_ACCMODE = os.O_ACCMODE
                fake_os.O_RDONLY = os.O_RDONLY
                fake_fcntl = types.SimpleNamespace(F_GETFL=fcntl.F_GETFL)
                fake_fcntl.fcntl = Mock(return_value=access)
                namespace = {
                    "os": fake_os, "fcntl": fake_fcntl, "stat": stat,
                    "RUNTIME_PATH": "/fixed/runtime", "FRAMEWORK_PATH": "/fixed/framework",
                    "RUNTIME_ATTESTATION_FIELDS": (), "RUNTIME_FIXED": {},
                    "pairs": lambda items: dict(items), "json": json, "re": re,
                    "MappingProxyType": types.MappingProxyType,
                }
                exec(compile(module, "<read-runtime-attestation>", "exec"), namespace)
                with self.assertRaisesRegex(ValueError, "descriptor is unsafe"):
                    namespace["read_runtime_attestation"](False)
                fake_os.read.assert_not_called()
                fake_os.close.assert_called_once_with(3)

    def test_runtime_import_path_rejects_relative_entry_before_realpath(self):
        import ast
        import fcntl
        import json
        import os
        import re
        import stat
        import types
        from unittest.mock import patch
        tree = ast.parse(self.bootstrap_code)
        function = next(node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name == "read_runtime_attestation")
        module = ast.Module(body=[function], type_ignores=[])
        sys_fixture = types.SimpleNamespace(
            platform="darwin",
            executable="/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/bin/python3.9",
            version_info=(3, 9, 6),
            flags=types.SimpleNamespace(isolated=1, no_site=1, dont_write_bytecode=1),
            path=["relative-entry"],
        )
        with patch.object(os, "fstat", return_value=types.SimpleNamespace(st_mode=stat.S_IFIFO | 0o600, st_uid=os.geteuid())), \
             patch.object(os, "read", side_effect=[self.runtime_attestation_bytes(), b""]), \
             patch.object(os, "close"), patch.object(fcntl, "fcntl", return_value=os.O_RDONLY), \
             patch.object(os.path, "realpath") as realpath:
            namespace = {
                "os": os, "fcntl": fcntl, "stat": stat, "sys": sys_fixture,
                "RUNTIME_PATH": "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/bin/python3.9",
                "FRAMEWORK_PATH": "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9",
                "RUNTIME_ATTESTATION_FIELDS": (
                    "apple_anchor", "bootstrap_sha256", "dependency_policy_id", "framework_cdhash",
                    "framework_dev", "framework_ino", "framework_mode", "framework_path",
                    "framework_resource_seal", "framework_uid", "os_build", "policy_id",
                    "requirement_id", "runtime_arch", "runtime_cdhash", "runtime_dev",
                    "runtime_ino", "runtime_mode", "runtime_path", "runtime_uid",
                    "runtime_version", "schema_version", "system_volume_trust",
                ),
                "RUNTIME_FIXED": {
                    "apple_anchor": "apple", "dependency_policy_id": "clt-python39-apple-dyld-v1",
                    "framework_cdhash": "a43551195b8d2eefd9356d81c1098ffc9e0a8b47",
                    "framework_path": "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9",
                    "framework_resource_seal": "valid", "policy_id": "clt-python3.9-v1",
                    "requirement_id": "com.apple.python3", "runtime_cdhash": "77e5dcc021cbfa7e2c3940b5ea150e3da037f3cf",
                    "runtime_path": "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/bin/python3.9",
                    "runtime_version": "3.9", "schema_version": "oasis7.local-signer.runtime-attestation.v1",
                    "system_volume_trust": "current-booted-apple-os",
                },
                "pairs": lambda items: dict(items), "json": json, "re": re,
                "MappingProxyType": __import__("types").MappingProxyType,
                "_verify_runtime_names": lambda _record: None,
            }
            exec(compile(module, "<read-runtime-attestation>", "exec"), namespace)
            with self.assertRaisesRegex(ValueError, "runtime import path is not absolute"):
                namespace["read_runtime_attestation"](False)
            realpath.assert_not_called()

    def intercept_prefix(self, stop_index, variables, lines=None):
        """Interpret the approved gate/bootstrap digest checks; invoke no OS tool."""
        import re
        import shlex
        block = self.operator_block() if lines is None else lines
        phase = 0
        for line in block[:stop_index]:
            if line.startswith("ACTUAL_RUNTIME_GATE_SHA256="):
                expected = 'ACTUAL_RUNTIME_GATE_SHA256="$(/usr/bin/shasum -a 256 "$APPROVED_RUNTIME_GATE" | /usr/bin/awk \'{print $1}\')"'
                if phase != 0:
                    raise ValueError("gate digest must be the first approval check")
                self.assertEqual(line, expected)
                gate = Path(variables["APPROVED_RUNTIME_GATE"])
                if not gate.is_file() or gate.is_symlink():
                    raise ValueError("approved native gate must be an independent regular file")
                variables["ACTUAL_RUNTIME_GATE_SHA256"] = api.digest(gate.read_bytes())
                self.operator_events.append("hash-approved-gate")
                phase = 1
            elif line.startswith('test "$ACTUAL_RUNTIME_GATE_SHA256"'):
                if phase != 1:
                    raise ValueError("gate digest must precede its approval comparison")
                self.assertEqual(shlex.split(line), ["test", "$ACTUAL_RUNTIME_GATE_SHA256", "=", "$EXPECTED_RUNTIME_GATE_SHA256", "||", "exit", "9"])
                if variables["ACTUAL_RUNTIME_GATE_SHA256"] != variables["EXPECTED_RUNTIME_GATE_SHA256"]:
                    raise ValueError("independent runtime gate digest approval mismatch")
                self.operator_events.append("check-approved-gate")
                phase = 2
            elif line.startswith('APPROVED_BOOTSTRAP_CODE="$(/bin/cat '):
                if phase != 2:
                    raise ValueError("bootstrap capture must follow the gate digest approval")
                self.assertEqual(line, 'APPROVED_BOOTSTRAP_CODE="$(/bin/cat "$APPROVED_BOOTSTRAP_SOURCE")"')
                source = Path(variables["APPROVED_BOOTSTRAP_SOURCE"])
                if source == self.release or self.release in source.parents:
                    raise ValueError("candidate stage cannot supply trusted bootstrap source")
                raw = source.read_bytes()
                if raw.endswith(b"\n"):
                    raise ValueError("bootstrap source must have no trailing line terminator")
                variables["APPROVED_BOOTSTRAP_CODE"] = raw.decode()
                self.operator_events.append("capture-independent-code")
                phase = 3
            elif line.startswith("ACTUAL_BOOTSTRAP_SHA256="):
                if phase != 3:
                    raise ValueError("captured bootstrap code must precede its digest")
                # Shell hash must cover the exact captured variable bytes used
                # by -c, never an independently reopened source pathname.
                expected = 'ACTUAL_BOOTSTRAP_SHA256="$(/usr/bin/printf \'%s\' "$APPROVED_BOOTSTRAP_CODE" | /usr/bin/shasum -a 256 | /usr/bin/awk \'{print $1}\')"'
                self.assertEqual(line, expected)
                self.assertIn("APPROVED_BOOTSTRAP_CODE", variables, "hash must follow real captured code assignment")
                variables["ACTUAL_BOOTSTRAP_SHA256"] = api.digest(variables["APPROVED_BOOTSTRAP_CODE"].encode())
                self.operator_events.append("hash-captured-code")
                phase = 4
            elif line.startswith('test "$ACTUAL_BOOTSTRAP_SHA256"'):
                if phase != 4:
                    raise ValueError("captured bootstrap digest must precede approval comparison")
                self.assertEqual(shlex.split(line), ["test", "$ACTUAL_BOOTSTRAP_SHA256", "=", "$EXPECTED_BOOTSTRAP_SHA256", "||", "exit", "9"])
                if variables.get("ACTUAL_BOOTSTRAP_SHA256") != variables["EXPECTED_BOOTSTRAP_SHA256"]:
                    raise ValueError("independent bootstrap digest approval mismatch")
                self.operator_events.append("check-approved-bootstrap")
                phase = 5
            else:
                tokens = shlex.split(line)
                invocation = tokens[:6] == ["/usr/bin/env", "-i", "PATH=/usr/bin:/bin:/usr/sbin:/sbin", "LANG=C", "LC_ALL=C", "$APPROVED_RUNTIME_GATE"] and "--bootstrap-source" in tokens and "--expected-bootstrap-sha256" in tokens and "--" in tokens and any(operation in tokens for operation in ("plan", "apply"))
                if phase != 5 or not invocation:
                    raise ValueError(f"unsupported executable operator prelude before approved gate invocation: {line}")

    def invoke_operator(self, operation="apply", variables=None, runbook_text=None):
        harness_supplied = variables is None and runbook_text is None
        variables = dict(self.variables() if variables is None else variables)
        lines = self.operator_block(runbook_text)
        index, tokens = next((index, tokens) for index, tokens in self.operator_commands(runbook_text) if operation in tokens)
        try:
            self.intercept_prefix(index, variables, lines)
        except ValueError as exc:
            # These tests require the supported gate contract, not a legacy
            # direct-interpreter command.
            if harness_supplied:
                self.fail(f"runbook has no usable independently approved runtime gate: {exc}")
            raise
        command = tuple(variables.get(token[1:], token) if token.startswith("$") else token for token in tokens)
        expected_prefix = ("/usr/bin/env", "-i", "PATH=/usr/bin:/bin:/usr/sbin:/sbin", "LANG=C", "LC_ALL=C", str(self.runtime_gate))
        self.assertEqual(command[:6], expected_prefix, "the approved gate must start with a clean environment")
        self.assertEqual(self.operator_events[:5], ["hash-approved-gate", "check-approved-gate", "capture-independent-code", "hash-captured-code", "check-approved-bootstrap"], "both independent digest comparisons must execute before gate startup")
        self.assertIn("check-approved-gate", self.operator_events, "independent native gate digest must be checked before invocation")
        self.assertTrue(self.bootstrap_code, "independently reviewed inline source bytes are absent")
        self.assertEqual(command[command.index("--bootstrap-source") + 1], str(self.bootstrap_source))
        self.assertEqual(command[command.index("--expected-bootstrap-sha256") + 1], self.bootstrap_approval)
        operation_args = command[command.index("--") + 1:]
        if self.before_inline_execution:
            self.before_inline_execution()
        if api.digest(self.bootstrap_source.read_bytes()) != self.bootstrap_approval:
            raise ValueError("native gate bootstrap-source digest mismatch")
        from unittest.mock import patch
        namespace = {
            "__name__": "__main__",
            "_bootstrap_acl_query": self.query_acl,
            "_bootstrap_metadata": self.observe_metadata,
            "_bootstrap_execute": self.execute_captured,
            "_bootstrap_after_capture": self.after_capture,
            "_bootstrap_target": "aarch64-apple-darwin",
            "_bootstrap_runtime_attestation": self.runtime_attestation_bytes,
        }
        with patch.object(sys, "argv", ["-c", *operation_args]):
            exec(compile(self.bootstrap_code, "<independently-approved-bootstrap>", "exec"), namespace)

    def launch(self):
        return self.invoke_operator()

    def test_production_command_uses_clean_env_and_approved_gate(self):
        for _, command in self.operator_commands():
            self.assertEqual(command[:6], ["/usr/bin/env", "-i", "PATH=/usr/bin:/bin:/usr/sbin:/sbin", "LANG=C", "LC_ALL=C", "$APPROVED_RUNTIME_GATE"], "every actual operator command must clear the environment before loading the gate")
            self.assertEqual(command[command.index("--bootstrap-source") + 1], "$APPROVED_BOOTSTRAP_SOURCE")
            self.assertEqual(command[command.index("--expected-bootstrap-sha256") + 1], "$EXPECTED_BOOTSTRAP_SHA256")
            self.assertEqual(command[command.index("--") + 1], "plan" if "plan" in command else "apply")
            self.assertEqual(command[command.index("--expected-manifest-sha256") + 1], "$EXPECTED_MANIFEST_SHA256")

    def test_actual_inline_bootstrap_positive_executes_captured_launcher(self):
        self.assertTrue(all(command[0] == "/usr/bin/env" and command[1] == "-i" for _, command in self.operator_commands()), "positive must exercise actual clean-environment gate entry")
        self.launch()
        self.assertEqual(self.executions[0]["launcher"], self.initial_launcher)
        self.assertEqual(self.executions[0]["files"]["install-release.py"], self.initial_launcher)

    def test_runbook_root_commands_use_same_approved_inline_entry(self):
        for operation in ("plan", "apply"):
            self.executions.clear()
            self.operator_events.clear()
            self.invoke_operator(operation)
            self.assertEqual(self.operator_events[:5], ["hash-approved-gate", "check-approved-gate", "capture-independent-code", "hash-captured-code", "check-approved-bootstrap"])
            self.assertEqual(self.executions[0]["launcher"], self.initial_launcher)

    def test_wrong_independent_bootstrap_digest_stops_real_snippet(self):
        variables = self.variables()
        variables["EXPECTED_BOOTSTRAP_SHA256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "independent bootstrap digest approval mismatch"):
            self.invoke_operator(variables=variables)
        self.assertEqual(self.operator_events, ["hash-approved-gate", "check-approved-gate", "capture-independent-code", "hash-captured-code"])
        self.assertEqual(self.executions, [], "actual digest guard, not prose, must block code startup")

    def test_stage_source_and_self_hash_exports_are_rejected_before_root_invocation(self):
        runbook = (SOURCE.parents[1] / "doc/p2p/blockchain/local-file-signing-backend.runbook.md").read_text()
        marker = '/usr/bin/env -i PATH=/usr/bin:/bin:/usr/sbin:/sbin LANG=C LC_ALL=C "$APPROVED_RUNTIME_GATE" \\\n'
        patterns = (
            'export APPROVED_BOOTSTRAP_SOURCE="$STAGING/install-release.py"',
            'export EXPECTED_BOOTSTRAP_SHA256="$(/usr/bin/shasum -a 256 "$STAGING/install-release.py" | /usr/bin/awk \'{print $1}\')"',
            'APPROVED_BOOTSTRAP_SOURCE="$STAGING/install-release.py"',
            'EXPECTED_BOOTSTRAP_SHA256="$(/usr/bin/shasum -a 256 "$APPROVED_BOOTSTRAP_SOURCE" | /usr/bin/awk \'{print $1}\')"',
            'EXPECTED_BOOTSTRAP_SHA256="$(/usr/bin/printf \'%s\' "$APPROVED_BOOTSTRAP_CODE" | /usr/bin/shasum -a 256 | /usr/bin/awk \'{print $1}\')"',
        )
        for injected in patterns:
            with self.subTest(injected=injected):
                self.assertEqual(runbook.count(marker), 2)
                modified = runbook.replace(marker, injected + "\n" + marker, 1)
                with self.assertRaises(ValueError):
                    self.invoke_operator(runbook_text=modified)
                self.assertEqual(self.executions, [], "candidate source/self-hash must not reach root invocation")

    def test_unknown_executable_prelude_is_never_ignored(self):
        runbook = (SOURCE.parents[1] / "doc/p2p/blockchain/local-file-signing-backend.runbook.md").read_text()
        marker = '/usr/bin/env -i PATH=/usr/bin:/bin:/usr/sbin:/sbin LANG=C LC_ALL=C "$APPROVED_RUNTIME_GATE" \\\n'
        self.assertEqual(runbook.count(marker), 2)
        modified = runbook.replace(marker, 'printf "unexpected"\n' + marker, 1)
        with self.assertRaisesRegex(ValueError, "unsupported executable operator prelude"):
            self.invoke_operator(runbook_text=modified)
        self.assertEqual(self.executions, [])

    def test_candidate_stage_source_cannot_supply_root_inline_code(self):
        variables = self.variables()
        variables["APPROVED_BOOTSTRAP_SOURCE"] = str(self.release / "install-release.py")
        with self.assertRaisesRegex(ValueError, "candidate stage cannot supply trusted bootstrap source"):
            self.invoke_operator(variables=variables)
        self.assertEqual(self.operator_events, ["hash-approved-gate", "check-approved-gate"])
        self.assertEqual(self.executions, [], "trusted source must be independent of candidate stage")

    def test_bootstrap_source_path_replacement_cannot_change_executed_inline_bytes(self):
        self.before_inline_execution = lambda: self.bootstrap_source.write_bytes(b"raise RuntimeError('replacement bootstrap')")
        with self.assertRaisesRegex(ValueError, "native gate bootstrap-source digest mismatch"):
            self.invoke_operator()
        self.assertEqual(self.executions, [], "the native gate independently reopens and hashes the protected bootstrap source")

    def test_complete_snapshot_contains_manifest_and_every_member_once(self):
        expected = {name: (self.release / name).read_bytes() for name in (*api.FILES, "manifest.json")}
        def replace_noncode_paths():
            for name in ("manifest.json", "oasis7_local_signer_worker", "oasis7_local_signer_admin", "installer.py", "macos_host.py"):
                path = self.release / name
                path.rename(self.release / ("previous-" + name))
                path.write_bytes(b"replacement after capture")
        self.after_capture = replace_noncode_paths
        self.launch()
        self.assertEqual(self.executions[0]["files"], expected, "downstream code/package/install input must be the complete closed captured snapshot, never path rereads")

    def test_acl_observed_for_manifest_and_every_member_descriptor(self):
        identities = [self.identity(self.release / name) for name in (*api.FILES, "manifest.json")]
        self.launch()
        for identity in identities:
            self.assertIn(identity, self.acl_queries, "native ACL admission applies to retained manifest/binary/module descriptors as well as ancestors")

    def test_binary_digest_preexec_guard_remains(self):
        (self.release / "oasis7_local_signer_worker").write_bytes(b"tampered worker")
        self.assert_preexec_rejection()

    def test_same_fd_capture_drift_rejects_before_candidate_exec(self):
        import os
        from unittest.mock import patch
        path = self.release / "installer.py"
        selected = self.identity(path)
        original_read = os.read
        changed = False
        def drift(fd, amount):
            nonlocal changed
            info = os.fstat(fd)
            if not changed and (info.st_dev, info.st_ino) == selected:
                path.rename(self.release / "previous-installer.py")
                path.write_bytes(b"changed module path while descriptor retained")
                changed = True
            return original_read(fd, amount)
        with patch.object(os, "read", drift):
            self.assert_preexec_rejection()

    def test_direct_candidate_path_root_main_is_explicitly_unsupported(self):
        from unittest.mock import patch
        import io
        import contextlib
        events = []
        def forbidden_path_read(*_args):
            events.append("candidate-module-path-read")
            raise ValueError("fixture path-read tripwire")
        command = [str(SOURCE / "install-release.py"), "apply", "--release-dir", str(self.release), "--expected-manifest-sha256", self.expected, "--plan", str(self.parent / "approved-plan"), "--expected-plan-sha256", "0" * 64]
        output = io.StringIO()
        with patch.object(sys, "argv", command), patch.object(self.source.os, "geteuid", return_value=0), patch.object(self.source, "approved_modules", forbidden_path_read), contextlib.redirect_stdout(output):
            status = self.source.main()
        self.assertEqual(status, 9)
        self.assertEqual(events, [], "direct root path entry must stop before candidate modules or installer admission")
        self.assertEqual(json.loads(output.getvalue())["code"], "TRUSTED_BOOTSTRAP_REQUIRED")


class NativeBootstrapACLContract(unittest.TestCase):
    """Exercise the shipped ACL parser's native branch without host ACL calls."""

    @classmethod
    def setUpClass(cls):
        import ast
        cls.bootstrap = next(
            ast.literal_eval(node.value)
            for node in ast.parse((SOURCE / "install-release.py").read_text(encoding="utf-8")).body
            if isinstance(node, ast.Assign)
            and any(isinstance(name, ast.Name) and name.id == "TRUSTED_BOOTSTRAP" for name in node.targets)
        )
        bootstrap_tree = ast.parse(cls.bootstrap)
        cls.acl_empty_node = next(
            node for node in bootstrap_tree.body
            if isinstance(node, ast.FunctionDef) and node.name == "acl_empty"
        )

    @staticmethod
    def u32(value):
        import sys
        return int(value).to_bytes(4, sys.byteorder)

    @classmethod
    def volume_reply(cls, valid=0x400, capability=0x400):
        values = (0, capability, 0, 0, 0, valid, 0, 0)
        return cls.u32(36) + b"".join(cls.u32(value) for value in values)

    @classmethod
    def absent_acl_reply(cls):
        # total, five returned-attribute groups, and an all-zero attrreference
        return cls.u32(32) + cls.u32(0x80000000) + bytes(16) + bytes(8)

    @classmethod
    def filesec_reply(cls, count=0xFFFFFFFF, flags=0, aces=b""):
        import struct
        filesec = cls.u32(0x012CC16D) + bytes(32) + cls.u32(count) + cls.u32(flags) + aces
        total = 4 + 20 + 8 + len(filesec)
        return (
            cls.u32(total)
            + cls.u32(0x80400000) + bytes(16)
            + struct.pack("=iI", 8, len(filesec))
            + filesec
        )

    class FakeFunction:
        def __init__(self, replies):
            self.replies = list(replies)
            self.calls = []
            self.argtypes = None
            self.restype = None

        def __call__(self, fd, request_ptr, output, capacity, options):
            import ctypes
            import struct
            request = ctypes.string_at(request_ptr, 24)
            bitmapcount, reserved = struct.unpack_from("=HH", request)
            groups = struct.unpack_from("=5I", request, 4)
            self.calls.append((fd, bitmapcount, reserved, *groups, capacity, options))
            if not self.replies:
                raise AssertionError("unexpected native ACL query")
            reply = self.replies.pop(0)
            if isinstance(reply, tuple):
                ctypes.set_errno(reply[1])
                return reply[0]
            if len(reply) > capacity:
                raise AssertionError("native ACL fixture exceeds output capacity")
            ctypes.memmove(output, reply, len(reply))
            return 0

    class FakeLibrary:
        def __init__(self, replies):
            self.fgetattrlist = NativeBootstrapACLContract.FakeFunction(replies)

    @classmethod
    def execute_native_acl_query(cls, replies, fd=123):
        import ast
        import ctypes
        import errno
        import sys
        from types import SimpleNamespace
        from unittest.mock import patch

        namespace = {"ctypes": ctypes, "errno": errno, "sys": SimpleNamespace(platform="darwin", byteorder=sys.byteorder)}
        exec(compile(ast.Module(body=[cls.acl_empty_node], type_ignores=[]), str(SOURCE / "install-release.py"), "exec"), namespace)
        library = cls.FakeLibrary(replies)
        with patch.object(ctypes, "CDLL", return_value=library) as load_native_library:
            namespace["acl_empty"](fd, seams=False)
        load_native_library.assert_called_once_with("/usr/lib/libSystem.B.dylib", use_errno=True)
        return library.fgetattrlist.calls

    @staticmethod
    def expected_calls(fd=123):
        return (
            (fd, 5, 0, 0, 0x80020000, 0, 0, 0, 64, 4),
            (fd, 5, 0, 0x80400000, 0, 0, 0, 0, 8192, 4),
        )

    def test_native_acl_absent_and_noacl_sentinel_use_exact_requests(self):
        for security_reply in (self.absent_acl_reply(), self.filesec_reply()):
            with self.subTest(security_reply=security_reply[4:8]):
                calls = self.execute_native_acl_query((self.volume_reply(), security_reply))
                self.assertEqual(tuple(calls), self.expected_calls())

    def test_native_acl_query_errors_fail_closed_even_with_zero_errno(self):
        import errno

        for error in (errno.EIO, 0):
            with self.subTest(query="volume", errno=error):
                with self.assertRaises(OSError):
                    self.execute_native_acl_query(((-1, error),))
            with self.subTest(query="security", errno=error):
                with self.assertRaises(OSError):
                    self.execute_native_acl_query((self.volume_reply(), (-1, error)))

    def test_native_acl_requires_capability_and_complete_security_framing(self):
        for valid, capability in ((0, 0x400), (0x400, 0)):
            with self.subTest(valid=valid, capability=capability):
                with self.assertRaisesRegex(ValueError, "does not prove extended-security support"):
                    self.execute_native_acl_query((self.volume_reply(valid, capability), self.absent_acl_reply()))
        with self.assertRaisesRegex(ValueError, "extended-security response is truncated"):
            self.execute_native_acl_query((self.volume_reply(), self.u32(20) + bytes(16)))

    def test_native_acl_rejects_present_empty_and_populated_filesec(self):
        import sys

        for security_reply in (self.filesec_reply(count=0), self.filesec_reply(count=1, aces=bytes(24))):
            with self.subTest(filesec_count=int.from_bytes(security_reply[68:72], sys.byteorder)):
                with self.assertRaisesRegex(ValueError, "contains an ACL"):
                    self.execute_native_acl_query((self.volume_reply(), security_reply))
