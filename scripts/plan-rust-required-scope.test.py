#!/usr/bin/env python3
import contextlib
import io
import sys
from unittest.mock import patch
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('selector', HERE / 'plan-rust-required-scope.py')
selector = importlib.util.module_from_spec(spec)
spec.loader.exec_module(selector)


class Selection(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.previous = Path.cwd()
        os.chdir(self.temp.name)
        self.git('init', '-q')
        self.git('config', 'user.email', 'ci@example.invalid')
        self.git('config', 'user.name', 'CI fixture')
        self.write('Cargo.toml', '[workspace]\nmembers=["crates/*"]\n[workspace.dependencies]\ncore={path="crates/core"}\n')
        for package in ['core', 'consumer', 'other']:
            manifest = f'[package]\nname="{package}"\nversion="0.1.0"\n'
            if package == 'consumer':
                manifest += '[target.\'cfg(unix)\'.dev-dependencies]\ncore={workspace=true,optional=true}\n'
            self.write(f'crates/{package}/Cargo.toml', manifest)
            self.write(f'crates/{package}/src/lib.rs', '')
        self.write('doc/readme.md', 'ordinary doc')
        self.base = self.commit()
        groups = ['baseline', 'rust_baseline', 'core', 'consumer', 'other', *selector.PLATFORM_GROUPS]
        self.config = {'groups': groups, 'resources': {g: [] for g in groups},
                       'package_groups': {p: [p] for p in ['core', 'consumer', 'other']},
                       'rules': [{'match': ['doc/**'], 'groups': [], 'reason': 'doc'}]}

    def tearDown(self):
        os.chdir(self.previous)
        self.temp.cleanup()

    def git(self, *args):
        return subprocess.check_output(['git', *args], stderr=subprocess.DEVNULL).decode().strip()

    def write(self, path, content):
        Path(path).parent.mkdir(parents=True, exist_ok=True)
        Path(path).write_text(content)

    def commit(self):
        self.git('add', '-A')
        self.git('commit', '-qm', 'fixture')
        return self.git('rev-parse', 'HEAD')

    def plan(self, head=None):
        return selector.select(self.config, self.base, head or self.commit())

    def test_docs_baseline_only(self):
        self.write('doc/readme.md', 'changed')
        self.assertEqual(self.plan()['groups'], ['baseline'])

    def test_optional_target_dev_reverse_consumer(self):
        self.write('crates/core/src/lib.rs', '// changed')
        self.assertEqual(set(self.plan()['groups']), {'baseline', 'rust_baseline', 'core', 'consumer'})

    def test_deleted_dependency_uses_old_graph(self):
        self.write('crates/consumer/Cargo.toml', '[package]\nname="consumer"\nversion="0.1.0"\n')
        self.write('crates/core/src/lib.rs', '// changed')
        head = self.commit()
        affected = selector.reverse_consumers({'core'}, selector.cargo_graph(self.base)[1])
        self.assertIn('consumer', affected)
        self.assertEqual(self.plan(head)['scope'], 'full')  # manifest changes always conservative

    def test_new_reverse_consumer_and_renamed_dependency(self):
        self.write('crates/other/Cargo.toml', '[package]\nname="other"\nversion="0.1.0"\n[dependencies]\nalias={package="core",path="../core"}\n')
        head = self.commit()
        self.assertIn('other', selector.reverse_consumers({'core'}, selector.cargo_graph(head)[1]))

    def test_move_covers_both_packages(self):
        self.write('crates/core/src/moved.rs', 'unique move contents')
        self.base = self.commit()
        self.git('mv', 'crates/core/src/moved.rs', 'crates/other/src/moved.rs')
        self.assertTrue({'core', 'consumer', 'other'}.issubset(self.plan()['groups']))

    def test_deleted_source_old_owner(self):
        Path('crates/core/src/lib.rs').unlink()
        self.assertIn('consumer', self.plan()['groups'])

    def test_unusual_path_nul_diff_json_safe(self):
        path = 'crates/core/src/space;line\nname.rs'
        self.write(path, 'contents')
        head = self.commit()
        self.assertIn(path, selector.changed_paths(self.base, head))
        self.assertIn('consumer', self.plan(head)['groups'])
        self.assertNotIn('\n', json.dumps(self.plan(head)))

    def test_embedded_document_consumed(self):
        self.write('crates/core/src/lib.rs', 'const D:&str=include_str!("../../../doc/readme.md");')
        self.base = self.commit()
        self.write('doc/readme.md', 'changed consumer input')
        self.assertIn('consumer', self.plan()['groups'])

    def test_build_lock_control_unknown_and_graph_failure_full(self):
        for path in ['Cargo.lock', 'crates/core/build.rs', '.github/workflows/test.yml', 'unknown.file']:
            with self.subTest(path=path):
                self.write(path, 'change')
                self.assertEqual(self.plan()['scope'], 'full')
        self.assertEqual(self.plan('absent-object')['scope'], 'full')

    def test_tested_merge_extra_paths(self):
        self.write('doc/readme.md', 'source doc')
        source = self.commit()
        self.write('crates/other/src/lib.rs', '// actual merge')
        tested = self.commit()
        plan = selector.select(self.config, self.base, source, tested)
        self.assertIn('other', plan['groups'])
        self.assertEqual(plan['scope'], 'full')  # unverifiable merge topology

    def merge_fixture(self, target_path='Cargo.lock', resolution=None, target_content='// target advance'):
        self.git('checkout', '-qb', 'source', self.base)
        self.write('doc/readme.md', 'source doc')
        source = self.commit()
        self.git('checkout', '-qb', 'target', self.base)
        self.write(target_path, target_content)
        target = self.commit()
        self.git('merge', '--no-ff', '-qm', 'merge fixture', source)
        if resolution:
            self.write(*resolution)
            self.git('add', '-A')
            self.git('commit', '--amend', '-qm', 'merge resolution')
        return source, target, self.git('rev-parse', 'HEAD')

    def test_target_lock_advance_does_not_expand_docs_merge(self):
        source, target, tested = self.merge_fixture()
        for base in (self.base, target):
            with self.subTest(base=base):
                plan = selector.select(self.config, base, source, tested)
                self.assertEqual(plan['scope'], 'targeted')
                self.assertEqual(plan['groups'], ['baseline'])

    def test_merge_resolution_keeps_extra_consumer(self):
        source, target, tested = self.merge_fixture(
            resolution=('crates/core/src/lib.rs', '// resolution'))
        plan = selector.select(self.config, self.base, source, tested)
        self.assertEqual(plan['scope'], 'targeted')
        self.assertEqual(set(plan['groups']), {'baseline', 'rust_baseline', 'core', 'consumer'})

    def test_merge_resolution_lock_change_still_full(self):
        source, target, tested = self.merge_fixture(resolution=('Cargo.lock', '// resolution'))
        self.assertEqual(selector.select(self.config, self.base, source, tested)['scope'], 'full')

    def test_wrong_merge_source_and_unrelated_base_fail_closed(self):
        source, target, tested = self.merge_fixture()
        self.assertEqual(selector.select(self.config, self.base, target, tested)['scope'], 'full')
        self.git('checkout', '-qb', 'unrelated', self.base)
        self.write('doc/unrelated.md', 'unrelated event base')
        unrelated = self.commit()
        self.assertEqual(selector.select(self.config, unrelated, source, tested)['scope'], 'full')

    def test_target_graph_keeps_new_reverse_consumer(self):
        source, target, tested = self.merge_fixture(
            'crates/other/Cargo.toml', ('crates/core/src/lib.rs', '// resolution'),
            '[package]\nname="other"\nversion="0.1.0"\n[dependencies]\ncore={path="../core"}\n')
        plan = selector.select(self.config, self.base, source, tested)
        self.assertEqual(plan['scope'], 'targeted')
        self.assertIn('other', plan['groups'])

    def test_source_lock_change_still_full_on_advanced_target(self):
        source, target, tested = self.merge_fixture('crates/other/src/lib.rs')
        self.git('checkout', 'source')
        self.write('Cargo.lock', '// source lock')
        source = self.commit()
        self.git('checkout', 'target')
        self.git('merge', '--no-ff', '-qm', 'merge fixture', source)
        tested = self.git('rev-parse', 'HEAD')
        self.assertEqual(selector.select(self.config, target, source, tested)['scope'], 'full')

    def test_additions_only_expand_and_groups_only(self):
        self.write('doc/readme.md', 'changed')
        head = self.commit()
        plan = selector.select(self.config, self.base, head, additions=['other'])
        self.assertEqual(set(plan['groups']), {'baseline', 'other'})
        self.assertNotIn('matrices', plan)
        with self.assertRaises(ValueError):
            selector.select(self.config, self.base, head, additions=['unknown'])

    def test_github_output_contains_plan_and_flags_without_matrix_outputs(self):
        self.write('doc/readme.md', 'changed')
        head = self.commit()
        Path('config.json').write_text(json.dumps(self.config))
        arguments = ['selector', '--base-ref', self.base, '--head-ref', head,
                     '--config', 'config.json', '--github-output', 'outputs.txt']
        with patch.object(sys, 'argv', arguments), contextlib.redirect_stdout(io.StringIO()):
            selector.main()
        outputs = dict(line.split('=', 1) for line in Path('outputs.txt').read_text().splitlines())
        plan = json.loads(outputs['plan'])
        self.assertEqual(set(plan), {'groups', 'resources', 'reasons', 'scope',
                                    'workspace_support_profiles', 'workspace_support_profile_version'})
        self.assertEqual(outputs['run_baseline'], 'true')
        self.assertEqual(outputs['run_other'], 'false')
        self.assertFalse(any(name.startswith('matrix') for name in outputs))

    def test_real_tool_rules_select_only_related_groups(self):
        config = json.loads((HERE / 'ci-required-scope.json').read_text())
        cases = {
            'resource-cleanup-executor.py': {'baseline', 'workflow_governance', 'fleet_health'},
            'new-task-worktree.sh': {'baseline', 'cargo_tooling_contracts', 'fleet_health'},
            'worktree-gc-report.sh': {'baseline', 'workflow_governance'},
            'pr-review-thread-closeout.py': {'baseline', 'workflow_governance'},
            'cargo-dev.sh': {'baseline', 'cargo_tooling_contracts'},
        }
        for filename, expected in cases.items():
            with self.subTest(filename=filename):
                self.write('scripts/' + filename, 'fixture')
                head = self.commit()
                plan = selector.select(config, self.base, head)
                self.assertEqual(set(plan['groups']), expected)
                self.assertEqual(plan['scope'], 'targeted')
                self.base = head


    def test_local_signer_precise_consumers(self):
        config = json.loads((HERE / 'ci-required-scope.json').read_text())
        common = {'baseline', 'packaging_contracts', 'macos_package_contract', 'fleet_health'}
        cases = {
            'scripts/local-signer/tests/test_future.py': common,
            'doc/p2p/blockchain/local-file-signing-backend.runbook.md': common,
            **{'scripts/local-signer/' + name: common | {'workspace_support'}
               for name in ['installer.py', 'macos_host.py', 'install-release.py', 'package-release.py']},
        }
        for path, expected in cases.items():
            with self.subTest(path=path):
                self.write(path, 'fixture')
                head = self.commit()
                plan = selector.select(config, self.base, head)
                self.assertEqual(set(plan['groups']), expected)
                self.assertEqual(plan['scope'], 'targeted')
                self.base = head
        self.write('scripts/local-signer/new-production.py', 'fixture')
        self.assertEqual(selector.select(config, self.base, self.commit())['scope'], 'full')

    def test_local_signer_deletion_and_rename(self):
        config = json.loads((HERE / 'ci-required-scope.json').read_text())
        self.write('scripts/local-signer/installer.py', 'unique production fixture')
        self.base = self.commit()
        Path('scripts/local-signer/tests').mkdir()
        self.git('mv', 'scripts/local-signer/installer.py', 'scripts/local-signer/tests/test_moved.py')
        head = self.commit()
        self.assertEqual(set(selector.select(config, self.base, head)['groups']),
                         {'baseline', 'packaging_contracts', 'macos_package_contract', 'fleet_health', 'workspace_support'})
        self.base = head
        Path('scripts/local-signer/tests/test_moved.py').unlink()
        self.assertEqual(set(selector.select(config, self.base, self.commit())['groups']),
                         {'baseline', 'packaging_contracts', 'macos_package_contract', 'fleet_health'})

    def test_local_signer_crate_keeps_reverse_consumers(self):
        config = json.loads((HERE / 'ci-required-scope.json').read_text())
        self.write('crates/oasis7_local_signer/Cargo.toml', '[package]\nname="oasis7_local_signer"\nversion="0.1.0"\n')
        self.write('crates/oasis7_local_signer/src/lib.rs', '')
        self.write('crates/consumer/Cargo.toml', '[package]\nname="consumer"\nversion="0.1.0"\n[dependencies]\noasis7_local_signer={path="../oasis7_local_signer"}\n')
        config['package_groups']['consumer'] = ['node']
        self.base = self.commit()
        self.write('crates/oasis7_local_signer/src/lib.rs', '// changed')
        plan = selector.select(config, self.base, self.commit())
        self.assertEqual(plan['scope'], 'targeted')
        self.assertTrue({'workspace_support', 'node', 'rust_baseline', 'packaging_contracts',
                         'macos_package_contract', 'fleet_health'}.issubset(plan['groups']))

    def test_local_signer_combination_and_security_overlay(self):
        config = json.loads((HERE / 'ci-required-scope.json').read_text())
        for path in ['scripts/local-signer/installer.py', 'scripts/local-signer/tests/test_fixture.py',
                     'doc/p2p/blockchain/local-file-signing-backend.runbook.md', 'scripts/security/fixture.py']:
            self.write(path, 'fixture')
        self.assertEqual(set(selector.select(config, self.base, self.commit())['groups']),
                         {'baseline', 'packaging_contracts', 'macos_package_contract', 'fleet_health',
                          'workspace_support', 'workflow_governance'})

    def test_agent_api_and_harness_route_through_support_and_oasis7_consumers(self):
        config = json.loads((HERE / 'ci-required-scope.json').read_text())
        packages = ('oasis7_agent_api', 'oasis7_agent_harness', 'oasis7')
        for package in packages:
            self.write(f'crates/{package}/Cargo.toml',
                       f'[package]\nname="{package}"\nversion="0.1.0"\n')
            self.write(f'crates/{package}/src/lib.rs', '')
        base = self.commit()
        manifests = {name: (f'crates/{name}', {}) for name in packages}
        reverse = {
            'oasis7_agent_api': {'oasis7_agent_harness', 'oasis7'},
            'oasis7_agent_harness': {'oasis7'},
            'oasis7': set(),
        }
        graph = (manifests, reverse, {})
        expected = {'baseline', 'rust_baseline', 'workspace_support',
                    'oasis7_required', 'launcher_web', 'formal_process'}
        with patch.object(selector, 'cargo_graph', return_value=graph):
            for package in ('oasis7_agent_api', 'oasis7_agent_harness'):
                with self.subTest(package=package):
                    self.write(f'crates/{package}/src/lib.rs', '// changed')
                    plan = selector.select(config, base, self.commit())
                    self.assertEqual(plan['scope'], 'targeted')
                    expected_profiles = {'oasis7_agent_harness'}
                    if package == 'oasis7_agent_api':
                        expected_profiles.add('oasis7_agent_api')
                    self.assertEqual(set(plan['workspace_support_profiles']), expected_profiles)
                    self.assertEqual(set(plan['groups']), expected)
                    base = self.git('rev-parse', 'HEAD')

    def test_game_client_routes_through_support_and_server_consumer(self):
        config = json.loads((HERE / 'ci-required-scope.json').read_text())
        packages = ('oasis7_client_api', 'oasis7_game_client', 'oasis7')
        for package in packages:
            self.write(f'crates/{package}/Cargo.toml',
                       f'[package]\nname="{package}"\nversion="0.1.0"\n')
            self.write(f'crates/{package}/src/lib.rs', '')
        base = self.commit()
        manifests = {name: (f'crates/{name}', {}) for name in packages}
        graph = (manifests, {
            'oasis7_client_api': {'oasis7_game_client', 'oasis7'},
            'oasis7_game_client': {'oasis7'}, 'oasis7': set(),
        }, {})
        with patch.object(selector, 'cargo_graph', return_value=graph):
            for package in ('oasis7_client_api', 'oasis7_game_client'):
                with self.subTest(package=package):
                    self.write(f'crates/{package}/src/lib.rs', '// changed')
                    plan = selector.select(config, base, self.commit())
                    self.assertEqual(plan['scope'], 'targeted')
                    expected_profiles = {'oasis7_game_client'}
                    if package == 'oasis7_client_api':
                        expected_profiles.add('oasis7_client_api')
                    self.assertEqual(set(plan['workspace_support_profiles']), expected_profiles)
                    self.assertEqual(set(plan['groups']), {'baseline', 'rust_baseline',
                                     'workspace_support', 'oasis7_required', 'launcher_web'})
                    base = self.git('rev-parse', 'HEAD')

    def test_deleted_agent_harness_source_uses_base_consumer_graph(self):
        config = json.loads((HERE / 'ci-required-scope.json').read_text())
        packages = ('oasis7_agent_api', 'oasis7_agent_harness', 'oasis7')
        for package in packages:
            self.write(f'crates/{package}/Cargo.toml',
                       f'[package]\nname="{package}"\nversion="0.1.0"\n')
            self.write(f'crates/{package}/src/lib.rs', '')
        base = self.commit()
        old_manifests = {name: (f'crates/{name}', {}) for name in packages}
        old_reverse = {
            'oasis7_agent_api': {'oasis7_agent_harness', 'oasis7'},
            'oasis7_agent_harness': {'oasis7'},
            'oasis7': set(),
        }
        new_manifests = {'oasis7': ('crates/oasis7', {})}
        new_reverse = {'oasis7': set()}
        Path('crates/oasis7_agent_harness/src/lib.rs').unlink()
        head = self.commit()

        def graph_for(ref):
            return ((old_manifests, old_reverse, {}) if ref == base
                    else (new_manifests, new_reverse, {}))

        with patch.object(selector, 'cargo_graph', side_effect=graph_for):
            plan = selector.select(config, base, head)
        self.assertEqual(plan['scope'], 'targeted')
        self.assertEqual(set(plan['groups']), {'baseline', 'rust_baseline', 'workspace_support',
                                               'oasis7_required', 'launcher_web', 'formal_process'})


if __name__ == '__main__':
    unittest.main()
