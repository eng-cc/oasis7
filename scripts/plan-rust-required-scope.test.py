#!/usr/bin/env python3
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

    def test_additions_only_expand_and_nonempty_matrices(self):
        self.write('doc/readme.md', 'changed')
        head = self.commit()
        plan = selector.select(self.config, self.base, head, additions=['other'])
        self.assertEqual(set(plan['groups']), {'baseline', 'other'})
        for group in self.config['groups']:
            self.assertEqual(bool(plan['matrices'][group]['include']), group in plan['groups'])
        with self.assertRaises(ValueError):
            selector.select(self.config, self.base, head, additions=['unknown'])


if __name__ == '__main__':
    unittest.main()
