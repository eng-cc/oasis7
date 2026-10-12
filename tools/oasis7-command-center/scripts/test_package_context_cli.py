"""Packaging regression: Cargo artifacts need not live under checkout/target."""
import fnmatch
import re
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("package-context-cli.sh")


class PackageContextCli(unittest.TestCase):
    def run_fixture(self, mode, input_state=None, build_args=None, target=None):
        temporary = tempfile.TemporaryDirectory(prefix="context-package-")
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name) / "checkout"
        package = root / "tools/oasis7-command-center/scripts/package-context-cli.sh"
        package.parent.mkdir(parents=True)
        shutil.copy2(SCRIPT, package)
        desktop = package.with_name("build-desktop.sh")
        shutil.copy2(SCRIPT.with_name("build-desktop.sh"), desktop)
        wrapper = root / "scripts/cargo-dev.sh"
        wrapper.parent.mkdir()
        wrapper.write_text("""#!/usr/bin/env python3
import json, os, sys
expected = ['build','--locked','--release','-p','oasis7_context','--message-format=json-render-diagnostics']
expected += ['--target', os.environ['OASIS7_BUILD_TARGET']]
assert sys.argv[1:] == expected
if os.environ.get('FIXTURE_TARGET'): assert os.environ['OASIS7_BUILD_TARGET'] == os.environ['FIXTURE_TARGET']
assert os.environ['OASIS7_SOURCE_HEAD_OID'] in os.environ['OASIS7_RELEASE_ID']
from pathlib import Path
Path(os.environ['FIXTURE_CARGO_MARKER']).write_text('invoked')
if os.environ['FIXTURE_CARGO_MODE'] == 'changed-input':
    Path('crates/oasis7_context').mkdir(parents=True)
    Path('crates/oasis7_context/build.rs').write_text('fn main() {}')
def artifact(path):
    return dict(reason='compiler-artifact', target=dict(name='oasis7_context',kind=['bin']),executable=path)
mode=os.environ['FIXTURE_CARGO_MODE']
print(json.dumps(dict(reason='compiler-artifact',target=dict(name='oasis7_context',kind=['lib']),executable=None)))
if mode != 'missing':
    print(json.dumps(artifact(os.environ['FIXTURE_ARTIFACT_PATH'])))
if mode == 'ambiguous':
    print(json.dumps(artifact(os.environ['FIXTURE_ARTIFACT_PATH']+'-other')))
if mode == 'failed':
    sys.exit(1)
""")
        wrapper.chmod(0o755)
        artifact = Path(temporary.name) / "shared cache/configured target/release/oasis7_context"
        artifact.parent.mkdir(parents=True)
        data = b"fixture release binary"
        artifact.write_bytes(data)
        def git(*args):
            return subprocess.run(["git", "-C", str(root), *args], check=True, capture_output=True, text=True)
        (root / '.gitignore').write_text('target/\nnode_modules/\ntools/oasis7-command-center/src-tauri/resources/context-cli/\n' + ('crates/**/build.rs\n' if input_state == 'ignored-build-script' else ''))
        git("init")
        git("config", "user.name", "Packaging Fixture")
        git("config", "user.email", "fixture@example.invalid")
        git("add", ".")
        git("-c", "commit.gpgsign=false", "commit", "-m", "Fixture")
        head = git("rev-parse", "HEAD").stdout.strip()
        marker = Path(temporary.name) / 'cargo-invoked'
        if input_state in ('untracked-build-script', 'ignored-build-script'):
            source = root / 'crates/oasis7_context/build.rs'
            source.parent.mkdir(parents=True)
            source.write_text('fn main() {}')
        elif input_state == 'tracked-change':
            wrapper.write_text(wrapper.read_text() + '\n# changed input\n')
        elif input_state == 'ignored-cache':
            cache = root / 'target/cache'
            cache.parent.mkdir()
            cache.write_text('ignored generated cache')
        environment = dict(os.environ, CI="true", CARGO_BUILD_TARGET="invalid-environment-target", FIXTURE_ARTIFACT_PATH=str(artifact), FIXTURE_CARGO_MODE=mode, FIXTURE_CARGO_MARKER=str(marker))
        if build_args is not None:
            shim_dir = Path(temporary.name) / 'fixture-bin'
            shim_dir.mkdir()
            (shim_dir / 'rustc').write_text("#!/bin/sh\necho 'host: aarch64-apple-darwin'\n")
            (shim_dir / 'rustc').chmod(0o755)
            (shim_dir / 'tauri').write_text("#!/usr/bin/env python3\nimport json,os,sys\nfrom pathlib import Path\nPath(os.environ['FIXTURE_TAURI_MARKER']).write_text(json.dumps(dict(args=sys.argv[1:],target=os.environ['OASIS7_BUILD_TARGET'])))\n")
            (shim_dir / 'tauri').chmod(0o755)
            environment['PATH'] = str(shim_dir) + os.pathsep + os.environ['PATH']
            environment['FIXTURE_TAURI_MARKER'] = str(root.parent / 'tauri-invoked')
        if target is not None:
            environment['FIXTURE_TARGET'] = target
        if mode in ('inventory-failure', 'ignored-inventory-failure'):
            shim_dir = Path(temporary.name) / 'fixture-bin'
            shim_dir.mkdir()
            shim = shim_dir / 'git'
            real_git = shutil.which('git')
            shim.write_text("#!/usr/bin/env python3\nimport os,sys\n" +
                "if 'ls-files' in sys.argv and (os.environ['FIXTURE_CARGO_MODE'] == 'inventory-failure' or '--ignored' in sys.argv): sys.exit(7)\n" +
                f"os.execv({real_git!r}, [{real_git!r}] + sys.argv[1:])\n")
            shim.chmod(0o755)
            environment['PATH'] = str(shim_dir) + os.pathsep + os.environ['PATH']
        result = subprocess.run(["bash", str(desktop if build_args is not None else package)] + (build_args or []), cwd=root, env=environment, capture_output=True, text=True)
        return root, data, head, result

    def test_configured_shared_artifact_is_packaged_without_checkout_target(self):
        root, data, head, result = self.run_fixture("success")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((root / "target").exists())
        output = root / "tools/oasis7-command-center/src-tauri/resources/context-cli"
        self.assertEqual((output / "oasis7_context").read_bytes(), data)
        catalog = json.loads((output / "catalog.json").read_text())
        self.assertEqual(catalog["source_head_oid"], head)
        self.assertEqual(catalog["release_id"], "context-" + head)
        self.assertEqual(catalog["sha256"], hashlib.sha256(data).hexdigest())
        self.assertEqual(catalog["size_bytes"], len(data))

    def test_uncommitted_inputs_reject_before_cargo_and_catalog(self):
        for state in ('untracked-build-script', 'ignored-build-script', 'tracked-change'):
            with self.subTest(state=state):
                root, _, _, result = self.run_fixture('success', state)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse((root.parent / 'cargo-invoked').exists())
                self.assertFalse((root / 'tools/oasis7-command-center/src-tauri/resources/context-cli').exists())

    def test_inventory_query_failure_rejects_before_cargo(self):
        for mode in ('inventory-failure', 'ignored-inventory-failure'):
            with self.subTest(mode=mode):
                root, _, _, result = self.run_fixture(mode)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse((root.parent / 'cargo-invoked').exists())
                self.assertFalse((root / 'tools/oasis7-command-center/src-tauri/resources/context-cli').exists())

    def test_input_changes_during_build_never_publish(self):
        root, _, _, result = self.run_fixture('changed-input')
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue((root.parent / 'cargo-invoked').exists())
        self.assertFalse((root / 'tools/oasis7-command-center/src-tauri/resources/context-cli').exists())

    def test_ignored_generated_cache_does_not_invalidate_committed_source(self):
        _, _, _, result = self.run_fixture('success', 'ignored-cache')
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_desktop_workflow_covers_cli_build_driver_inputs(self):
        workflow = (SCRIPT.parents[3] / '.github/workflows/command-center.yml').read_text()
        for event, ending in (('pull_request', '  push:'), ('push', 'permissions:')):
            section = workflow.split('  ' + event + ':', 1)[1].split(ending, 1)[0]
            patterns = re.findall(r"^\s+- '([^']+)'$", section, re.MULTILINE)
            for dependency in ('scripts/cargo-dev.sh', 'scripts/cargo-cache.py',
                               'scripts/find-python-with-module.sh', 'rust-toolchain.toml',
                               '.cargo/config.toml'):
                with self.subTest(event=event, dependency=dependency):
                    self.assertTrue(any(fnmatch.fnmatchcase(dependency, pattern)
                                        for pattern in patterns),
                                    f'{event} must package when only {dependency} changes')

    def test_requested_desktop_target_is_shared_with_cli_and_catalog(self):
        target = 'x86_64-apple-darwin'
        for args in (['--target', target], ['--target=' + target], ['-t', target], ['-t' + target]):
            with self.subTest(args=args):
                root, _, _, result = self.run_fixture('success', build_args=args, target=target)
                self.assertEqual(result.returncode, 0, result.stderr)
                catalog = json.loads((root / 'tools/oasis7-command-center/src-tauri/resources/context-cli/catalog.json').read_text())
                tauri = json.loads((root.parent / 'tauri-invoked').read_text())
                self.assertEqual(catalog['target'], target)
                self.assertEqual(tauri['target'], target)
                self.assertEqual(tauri['args'][-len(args):], args)

    def test_default_desktop_target_is_explicit_host_despite_cargo_environment(self):
        root, _, _, result = self.run_fixture('success', build_args=[], target='aarch64-apple-darwin')
        self.assertEqual(result.returncode, 0, result.stderr)
        catalog = json.loads((root / 'tools/oasis7-command-center/src-tauri/resources/context-cli/catalog.json').read_text())
        tauri = json.loads((root.parent / 'tauri-invoked').read_text())
        self.assertEqual(catalog['target'], 'aarch64-apple-darwin')
        self.assertEqual(tauri['target'], catalog['target'])
        self.assertEqual(tauri['args'][-2:], ['--target', catalog['target']])

    def test_universal_or_invalid_target_rejects_before_either_build(self):
        for args in (['--target', 'universal-apple-darwin'], ['--target='], ['--target'], ['--target', 'x86_64-apple-darwin', '--target=aarch64-apple-darwin'], ['--target=x86_64-apple-darwin', '--target=x86_64-apple-darwin'], ['--', '--target', 'x86_64-apple-darwin'], ['--', '--target=x86_64-apple-darwin']):
            with self.subTest(args=args):
                root, _, _, result = self.run_fixture('success', build_args=args)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse((root.parent / 'cargo-invoked').exists())
                self.assertFalse((root.parent / 'tauri-invoked').exists())
                self.assertFalse((root / 'tools/oasis7-command-center/src-tauri/resources/context-cli').exists())

    def test_missing_ambiguous_and_failed_build_never_publish(self):
        for mode in ("missing", "ambiguous", "failed"):
            with self.subTest(mode=mode):
                root, _, _, result = self.run_fixture(mode)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse((root / "tools/oasis7-command-center/src-tauri/resources/context-cli").exists())


if __name__ == "__main__":
    unittest.main()
