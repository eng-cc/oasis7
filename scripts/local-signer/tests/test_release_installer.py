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
            if re.match(r"^(?:export\s+)?(?:APPROVED_BOOTSTRAP_SOURCE|EXPECTED_BOOTSTRAP_SHA256)\s*=", line):
                raise ValueError("runbook must not assign independently approved bootstrap inputs")
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
        return dict(APPROVED_BOOTSTRAP_SOURCE=str(self.bootstrap_source), EXPECTED_BOOTSTRAP_SHA256=self.bootstrap_approval, STAGING=str(self.release), EXPECTED_MANIFEST_SHA256=self.expected, APPROVED_WORK_DIR="/Users/fixture/Documents/keys/oasis7-local-signer", APPROVED_CALLER="fixture", APPROVED_INSTALLATION_ID="fixture-install", APPROVED_DEPLOYMENT_ID="fixture-deployment", NEW_PLAN_FILE=str(self.parent / "plan.json"), INDEPENDENTLY_APPROVED_PLAN_SHA256="0" * 64)

    def intercept_prefix(self, stop_index, variables, lines=None):
        """Interpret the actual restricted shell approval prefix; invoke no OS tool.

        Accepted data grammar deliberately uses fixed native cat/printf/shasum/
        awk and an executable test ... || exit 9. GREEN docs must contain this
        reviewable prefix; every executable line is parsed before invocation,
        so an unknown prelude cannot silently mutate shell inputs. Strings in
        prose or an unchecked assignment do not count. Fake commands never
        exist on disk or in production argv/env.
        """
        import re
        import shlex
        block = self.operator_block() if lines is None else lines
        phase = 0
        for line in block[:stop_index]:
            assignment = re.fullmatch(r'APPROVED_BOOTSTRAP_CODE="\$\((.*)\)"', line)
            if assignment:
                if phase != 0:
                    raise ValueError("bootstrap capture must be the first approval operation")
                command = shlex.split(assignment.group(1))
                self.assertEqual(command, ["/bin/cat", "$APPROVED_BOOTSTRAP_SOURCE"], "bootstrap code must be read as data from independent out-of-stage source")
                source = Path(variables["APPROVED_BOOTSTRAP_SOURCE"])
                if source == self.release or self.release in source.parents:
                    raise ValueError("candidate stage cannot supply trusted bootstrap source")
                variables["APPROVED_BOOTSTRAP_CODE"] = source.read_bytes().decode().rstrip("\n")
                self.operator_events.append("capture-independent-code")
                phase = 1
            elif line.startswith("ACTUAL_BOOTSTRAP_SHA256="):
                if phase != 1:
                    raise ValueError("captured bootstrap code must precede its digest")
                # Shell hash must cover the exact captured variable bytes used
                # by -c, never an independently reopened source pathname.
                expected = 'ACTUAL_BOOTSTRAP_SHA256="$(/usr/bin/printf \'%s\' "$APPROVED_BOOTSTRAP_CODE" | /usr/bin/shasum -a 256 | /usr/bin/awk \'{print $1}\')"'
                self.assertEqual(line, expected)
                self.assertIn("APPROVED_BOOTSTRAP_CODE", variables, "hash must follow real captured code assignment")
                variables["ACTUAL_BOOTSTRAP_SHA256"] = api.digest(variables["APPROVED_BOOTSTRAP_CODE"].encode())
                self.operator_events.append("hash-captured-code")
                phase = 2
            elif line.startswith('test "$ACTUAL_BOOTSTRAP_SHA256"'):
                if phase != 2:
                    raise ValueError("captured bootstrap digest must precede approval comparison")
                self.assertEqual(shlex.split(line), ["test", "$ACTUAL_BOOTSTRAP_SHA256", "=", "$EXPECTED_BOOTSTRAP_SHA256", "||", "exit", "9"])
                if variables.get("ACTUAL_BOOTSTRAP_SHA256") != variables["EXPECTED_BOOTSTRAP_SHA256"]:
                    raise ValueError("independent bootstrap digest approval mismatch")
                self.operator_events.append("check-independent-approval")
                phase = 3
            else:
                tokens = shlex.split(line)
                invocation = tokens[:4] == ["/usr/bin/python3", "-I", "-c", "$APPROVED_BOOTSTRAP_CODE"] and any(operation in tokens for operation in ("plan", "apply"))
                if phase != 3 or not invocation:
                    raise ValueError(f"unsupported executable operator prelude before root invocation: {line}")

    def invoke_operator(self, operation="apply", variables=None, runbook_text=None):
        harness_supplied = variables is None and runbook_text is None
        variables = dict(self.variables() if variables is None else variables)
        lines = self.operator_block(runbook_text)
        index, tokens = next((index, tokens) for index, tokens in self.operator_commands(runbook_text) if operation in tokens)
        try:
            self.intercept_prefix(index, variables, lines)
        except ValueError as exc:
            # These tests require a working trusted inline entry. A frozen
            # source recipe that fails closed is RED, not a setup error.
            if harness_supplied:
                self.fail(f"runbook has no usable independently approved inline bootstrap: {exc}")
            raise
        command = tuple(variables.get(token[1:], token) if token.startswith("$") else token for token in tokens)
        if command[:3] != ("/usr/bin/python3", "-I", "-c"):
            # Current runbook's actual executable command starts candidate path
            # after separate verification. Reproduce that marker behavior only;
            # no shell/Python subprocess or privileged operation is performed.
            return self.legacy_launch()
        self.assertEqual(self.operator_events[-3:], ["capture-independent-code", "hash-captured-code", "check-independent-approval"], "real digest comparison must execute before inline code")
        self.assertTrue(self.bootstrap_code, "independently reviewed inline source bytes are absent")
        self.assertEqual(command[3], self.bootstrap_code, "actual operator -c bytes must equal approved out-of-stage source")
        if self.before_inline_execution:
            self.before_inline_execution()
        from unittest.mock import patch
        namespace = {
            "__name__": "__main__",
            "_bootstrap_acl_query": self.query_acl,
            "_bootstrap_metadata": self.observe_metadata,
            "_bootstrap_execute": self.execute_captured,
            "_bootstrap_after_capture": self.after_capture,
            "_bootstrap_target": "aarch64-apple-darwin",
        }
        with patch.object(sys, "argv", ["-c", *command[4:]]):
            exec(compile(command[3], "<independently-approved-bootstrap>", "exec"), namespace)

    def launch(self):
        return self.invoke_operator()

    def test_production_command_is_exact_isolated_inline_code(self):
        for _, command in self.operator_commands():
            self.assertEqual(command[:4], ["/usr/bin/python3", "-I", "-c", "$APPROVED_BOOTSTRAP_CODE"], "every actual operator command must use independently approved inline captured bytes")
            self.assertEqual(command[command.index("--expected-manifest-sha256") + 1], "$EXPECTED_MANIFEST_SHA256")

    def test_actual_inline_bootstrap_positive_executes_captured_launcher(self):
        self.assertTrue(all(command[2] == "-c" for _, command in self.operator_commands()), "positive must exercise actual inline entry")
        self.launch()
        self.assertEqual(self.executions[0]["launcher"], self.initial_launcher)
        self.assertEqual(self.executions[0]["files"]["install-release.py"], self.initial_launcher)

    def test_runbook_root_commands_use_same_approved_inline_entry(self):
        for operation in ("plan", "apply"):
            self.executions.clear()
            self.operator_events.clear()
            self.invoke_operator(operation)
            self.assertEqual(self.operator_events[:3], ["capture-independent-code", "hash-captured-code", "check-independent-approval"])
            self.assertEqual(self.executions[0]["launcher"], self.initial_launcher)

    def test_wrong_independent_bootstrap_digest_stops_real_snippet(self):
        variables = self.variables()
        variables["EXPECTED_BOOTSTRAP_SHA256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "independent bootstrap digest approval mismatch"):
            self.invoke_operator(variables=variables)
        self.assertEqual(self.operator_events, ["capture-independent-code", "hash-captured-code"])
        self.assertEqual(self.executions, [], "actual digest guard, not prose, must block code startup")

    def test_stage_source_and_self_hash_exports_are_rejected_before_root_invocation(self):
        import re
        runbook = (SOURCE.parents[1] / "doc/p2p/blockchain/local-file-signing-backend.runbook.md").read_text()
        patterns = (
            'export APPROVED_BOOTSTRAP_SOURCE="$STAGING/install-release.py"',
            'export EXPECTED_BOOTSTRAP_SHA256="$(/usr/bin/shasum -a 256 "$STAGING/install-release.py" | /usr/bin/awk \'{print $1}\')"',
            'APPROVED_BOOTSTRAP_SOURCE="$STAGING/install-release.py"',
            'EXPECTED_BOOTSTRAP_SHA256="$(/usr/bin/shasum -a 256 "$APPROVED_BOOTSTRAP_SOURCE" | /usr/bin/awk \'{print $1}\')"',
            'EXPECTED_BOOTSTRAP_SHA256="$(/usr/bin/printf \'%s\' "$APPROVED_BOOTSTRAP_CODE" | /usr/bin/shasum -a 256 | /usr/bin/awk \'{print $1}\')"',
        )
        for injected in patterns:
            with self.subTest(injected=injected):
                modified, count = re.subn(r"(?m)^(/usr/bin/python3 -I .*? plan \\\n)", injected + "\\n\\1", runbook, count=1)
                if count == 0:
                    # Current source still invokes the staged launcher. Inject
                    # before that concrete plan command to test parser behavior.
                    modified, count = re.subn(r"(?m)^(/usr/bin/python3 -I .*? plan \\\n)", injected + "\\n\\1", runbook, count=1)
                self.assertEqual(count, 1, "fixture must prepend the adversarial shell line to the actual plan command")
                with self.assertRaises(ValueError):
                    self.invoke_operator(runbook_text=modified)
                self.assertEqual(self.executions, [], "candidate source/self-hash must not reach root invocation")

    def test_unknown_executable_prelude_is_never_ignored(self):
        import re
        runbook = (SOURCE.parents[1] / "doc/p2p/blockchain/local-file-signing-backend.runbook.md").read_text()
        modified, count = re.subn(r"(?m)^(/usr/bin/python3 -I .*? plan \\\n)", 'printf "unexpected"\\n\\1', runbook, count=1)
        self.assertEqual(count, 1)
        with self.assertRaisesRegex(ValueError, "unsupported executable operator prelude"):
            self.invoke_operator(runbook_text=modified)
        self.assertEqual(self.executions, [])

    def test_candidate_stage_source_cannot_supply_root_inline_code(self):
        variables = self.variables()
        variables["APPROVED_BOOTSTRAP_SOURCE"] = str(self.release / "install-release.py")
        with self.assertRaisesRegex(ValueError, "candidate stage cannot supply trusted bootstrap source"):
            self.invoke_operator(variables=variables)
        self.assertEqual(self.operator_events, [])
        self.assertEqual(self.executions, [], "trusted source must be independent of candidate stage")

    def test_bootstrap_source_path_replacement_cannot_change_executed_inline_bytes(self):
        self.before_inline_execution = lambda: self.bootstrap_source.write_bytes(b"raise RuntimeError('replacement bootstrap')")
        self.invoke_operator()
        self.assertEqual(self.operator_events[:3], ["capture-independent-code", "hash-captured-code", "check-independent-approval"])
        self.assertEqual(self.executions[0]["launcher"], self.initial_launcher)

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
