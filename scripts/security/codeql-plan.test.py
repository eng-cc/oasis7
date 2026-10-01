#!/usr/bin/env python3
"""Local Git fixtures: these do not prove hosted extraction or scanning."""
import importlib.util
import json
import pathlib
import subprocess
import tempfile
import unittest
from types import SimpleNamespace

HERE = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('planner', HERE / 'codeql-plan.py')
planner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(planner)


class PlannerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = pathlib.Path(self.temp.name)
        self.git('init', '-b', 'main')
        self.git('config', 'user.email', 'fixture@example.test')
        self.git('config', 'user.name', 'Fixture')
        self.write('Cargo.toml', '[workspace]\nmembers=[]\n')
        self.write('src/lib.rs', 'const S: &str = include_str!("../data/help.md");\n')
        self.write('data/help.md', 'embedded')
        self.write('docs/guide.md', 'ordinary')
        self.base = self.commit()

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.root), *args], stderr=subprocess.DEVNULL).decode().strip()

    def write(self, path, value='fixture'):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(value)

    def commit(self):
        self.git('add', '.')
        self.git('commit', '-m', 'fixture')
        return self.git('rev-parse', 'HEAD')

    def run_plan(self, **kw):
        args = dict(repo_root=str(self.root), policy=str(HERE / 'codeql-policy.json'), base=self.base,
                    head=self.git('rev-parse', 'HEAD'), checkout=None, mode='observe', event='pull_request',
                    default_branch='main', profile='default', full=False, pr_number=3, max_slots=2)
        args.update(kw)
        return planner.plan(SimpleNamespace(**args))

    def test_CQ_T01_non_consumed_docs(self):
        self.write('docs/guide.md', 'changed'); self.commit()
        self.assertEqual(self.run_plan()['status'], 'not_applicable')

    def test_CQ_T02_T03_T04_T05_language_selection(self):
        for path, unit in [('tools/a.py', 'python-repo'), ('web/a.html', 'javascript-repo'),
                           ('src/new.rs', 'rust-repo'), ('.github/workflows/test.yml', 'actions-repo')]:
            with self.subTest(path=path):
                self.base = self.git('rev-parse', 'HEAD')
                self.write(path); self.commit()
                self.assertEqual(self.run_plan()['selected_units'], [unit])

    def test_CQ_T06_T12_policy_change_full_and_tamper_rejected(self):
        self.write('scripts/security/codeql-policy.json', '{}'); self.commit()
        self.assertEqual(len(self.run_plan()['selected_units']), 4)
        policy = json.loads((HERE / 'codeql-policy.json').read_text())
        policy['excluded_prefixes'].append('src/')
        self.write('candidate.json', json.dumps(policy))
        with self.assertRaisesRegex(ValueError, 'exclusion'):
            self.run_plan(policy=str(self.root / 'candidate.json'))

    def test_CQ_T07_T08_all_manifest_inventory_and_delete(self):
        self.write('standalone/Cargo.toml', '[package]\nname="standalone"\nversion="0.1.0"\n')
        self.write('excluded/Cargo.toml', '[package]\nname="excluded"\nversion="0.1.0"\n')
        self.write('vendor-patch/Cargo.toml', '[package]\nname="patch"\nversion="0.1.0"\n')
        self.base = self.commit()
        (self.root / 'standalone/Cargo.toml').unlink(); self.commit()
        result = self.run_plan()
        self.assertIn('standalone/Cargo.toml', result['completeness']['manifest_inventory']['base'])
        self.assertNotIn('standalone/Cargo.toml', result['completeness']['manifest_inventory']['head'])
        self.assertIn('excluded/Cargo.toml', result['completeness']['manifest_inventory']['head'])
        self.assertIn('vendor-patch/Cargo.toml', result['completeness']['manifest_inventory']['head'])
        self.assertEqual(result['selected_units'], ['rust-repo'])

    def test_CQ_T09_rename_both_sides_special_paths(self):
        self.write('old file\n$.py'); self.base = self.commit()
        (self.root / 'old file\n$.py').rename(self.root / 'new file\n$.js'); self.commit()
        result = self.run_plan()
        self.assertEqual(result['selected_units'], ['python-repo', 'javascript-repo'])
        self.assertEqual(len(result['completeness']['diff_paths']), 2)

    def test_CQ_T10_missing_identity(self):
        with self.assertRaisesRegex(ValueError, 'unreadable'):
            self.run_plan(base='0' * 40)

    def test_CQ_T11_complete_over_300_diff(self):
        for i in range(350): self.write(f'docs/{i}.md')
        self.write('last.py'); self.commit()
        result = self.run_plan()
        self.assertEqual(len(result['completeness']['diff_paths']), 351)
        self.assertEqual(result['selected_units'], ['python-repo'])

    def test_CQ_T13_embedded_document_and_build_data(self):
        self.write('data/help.md', 'changed'); self.commit()
        self.assertEqual(self.run_plan()['selected_units'], ['rust-repo'])
        self.base = self.git('rev-parse', 'HEAD')
        self.write('build.rs', 'fn main() {}'); self.base = self.commit()
        self.write('web/data.json', '{}'); self.commit()
        self.assertEqual(len(self.run_plan()['selected_units']), 4)

    def test_CQ_T14_vendor_and_exclusions(self):
        self.write('third_party/a.rs'); self.write('target/out.rs'); self.commit()
        self.assertEqual(self.run_plan()['selected_units'], [])
        self.base = self.git('rev-parse', 'HEAD')
        self.write('vendor-patched/a.rs'); self.commit()
        self.assertEqual(self.run_plan()['selected_units'], ['rust-repo'])

    def test_CQ_T15_T37_modes_profiles_and_slots(self):
        self.write('a.py'); self.commit()
        self.assertEqual(self.run_plan(mode='off')['status'], 'disabled')
        self.assertEqual(self.run_plan(mode='baseline')['status'], 'disabled')
        result = self.run_plan(profile='extended', max_slots=1)
        row = result['matrix']['include'][0]
        self.assertEqual(row['category'], 'oasis7/python-repo/extended')
        self.assertEqual(row['queries'], 'security-extended')
        self.assertEqual(row['slot'], 0)
        self.assertEqual(len(self.run_plan(event='schedule', mode='baseline', full=True)['selected_units']), 4)

    def test_unknown_and_symlink_fail_closed(self):
        self.write('new.config'); self.commit()
        self.assertEqual(len(self.run_plan()['selected_units']), 4)
        (self.root / 'link.py').symlink_to('/outside'); self.commit()
        with self.assertRaisesRegex(ValueError, 'symbolic link'): self.run_plan()

    def test_dynamic_include_and_invalid_cli_configuration(self):
        self.write('src/lib.rs', 'include_bytes!(concat!(env!("OUT_DIR"), "/data"));')
        self.base = self.commit()
        self.write('docs/guide.md', 'possible input'); self.commit()
        self.assertEqual(self.run_plan()['selected_units'], ['rust-repo'])
        result = subprocess.run(['python3', str(HERE / 'codeql-plan.py'), '--mode', 'invalid'], capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b'invalid choice', result.stderr)


if __name__ == '__main__': unittest.main()
