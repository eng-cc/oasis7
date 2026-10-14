#!/usr/bin/env python3
import importlib.util
from pathlib import Path
import subprocess
import sys
import unittest
import os
import tempfile
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('doc_core', HERE / 'doc-core-content-check.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class ContentTests(unittest.TestCase):
    def test_bad_structure(self):
        self.assertTrue(module.check_core('doc/product/a/x.prd.md', '# Empty'))
    def test_valid_structure(self):
        text = '\n'.join('## '+x for x in ['文档身份','玩家问题与目标','范围与 Non-Goals','玩家体验与决策','产品要求','验收与证据','设计取舍与未决问题'])
        self.assertEqual(module.check_core('doc/product/a/x.prd.md', text), [])
    def test_partial_card(self):
        text = '### REQ-DEMO-001\n- 系统设计状态：部分覆盖\n- 实现状态：部分实现\n'
        findings = module.check_status('a.prd.md', text)
        self.assertTrue(any('missing-status-field' in x for x in findings))
        self.assertTrue(any('partial-missing-scope' in x for x in findings))
    def test_unknown_is_honest(self):
        text = '### REQ-DEMO-001\n' + '\n'.join('- '+k+'：'+v for k,v in [('系统设计状态','未核实'),('实现状态','未核实'),('系统设计依据','尚未核查'),('实现与验证依据','尚未核查'),('核对日期与基线','尚未核查'),('剩余范围','整个功能待核查')])
        self.assertEqual(module.check_status('a.prd.md', text), [])
    def test_illegal_enum(self):
        self.assertTrue(any('invalid-status' in x for x in module.check_status('a.prd.md', '### REQ-X-1\n- 实现状态：merged')))
    def test_verified_needs_evidence(self):
        self.assertTrue(any('missing-status-evidence' in x for x in module.check_status('a.prd.md', '### REQ-X-1\n- 实现状态：已验证')))
    def test_comment_and_code_do_not_count(self):
        self.assertEqual(module.check_status('a.prd.md', '```\n### REQ-X-1\n```'), [])
    def test_cropping_is_not_silent_pass(self):
        self.assertTrue(any('invalid-structure-tailoring' in x for x in module.check_core('doc/system/x.design.md','- 结构裁剪：理由：小改；范围：接口；风险：低；映射：现有接口节')))
    def test_not_applicable_requires_reason_and_scope(self):
        errors = module.check_status('a.prd.md', '### REQ-X-1\n- 系统设计状态：不适用')
        self.assertTrue(any('not-applicable-missing-reason' in x for x in errors))
        self.assertFalse(any('invalid-status' in x for x in errors))
    def test_invalid_invocation(self):
        p = subprocess.run([sys.executable, str(HERE/'doc-core-content-check.py'), '--warn-only','--base','bad'],capture_output=True,text=True)
        self.assertNotEqual(p.returncode,0)


class FullCorpusTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.git('init', '-q')
        self.git('config', 'user.email', 'fixture@example.invalid')
        self.git('config', 'user.name', 'Fixture')
        self.put('seed', 'fixture')
        self.commit()

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.root), *args], text=True).strip()

    def put(self, path, text):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)
        return target

    def commit(self):
        self.git('add', '.')
        self.git('commit', '-qm', 'fixture')
        return self.git('rev-parse', 'HEAD')

    def run_checker(self, name, *args):
        return subprocess.run([sys.executable, str(HERE / (name + '.py')), '--repo-root', str(self.root), *args], capture_output=True, text=True)

    def test_full_reports_untouched_professional_and_scoped_stays_strict(self):
        self.put('doc/system/legacy.design.md', '# Legacy')
        base = self.commit()
        for name in ('system-design-traceability-check', 'doc-core-content-check'):
            scoped = self.run_checker(name, '--base', base, '--head', base)
            self.assertEqual(scoped.returncode, 0, scoped.stdout + scoped.stderr)
            full = self.run_checker(name, '--full-corpus')
            self.assertEqual(full.returncode, 0, full.stdout + full.stderr)
            self.assertIn('::warning::', full.stdout)
            self.assertIn('doc/system/legacy.design.md', full.stdout)
        self.put('doc/system/legacy.design.md', '# Substantive change')
        head = self.commit()
        for name in ('system-design-traceability-check', 'doc-core-content-check'):
            strict = self.run_checker(name, '--base', base, '--head', head)
            self.assertEqual(strict.returncode, 1, strict.stdout + strict.stderr)

    def test_inventory_excludes_product_fiction_overlays_but_keeps_retired(self):
        self.put('doc/product/a.design.md', '# Product')
        self.put('doc/novel/a.design.md', '# Novel')
        self.put('site/story/a.design.md', '# Story')
        self.put('doc/engineering/doc-governance/system-design-writing-standard.design.md', '# Norm')
        self.put('doc/system/retired.design.md', '# Retired\n## 文档身份\n- 生命周期：retired')
        docs = module.system.collect_full_corpus(self.root)
        self.assertEqual([p for p, _ in docs], ['doc/system/retired.design.md'])
        full = self.run_checker('doc-core-content-check', '--full-corpus')
        self.assertEqual(full.returncode, 0, full.stdout + full.stderr)
        self.assertIn('product=1 professional=1', full.stdout)
        self.assertIn('doc/system/retired.design.md', full.stdout)

    def test_conflicting_arguments_fail_even_warn_only(self):
        for name in ('system-design-traceability-check', 'doc-core-content-check'):
            for conflict in (('--base', 'bad'), ('--head', 'bad'), ('--base', ''), ('--head', ''), ('--worktree',)):
                p = self.run_checker(name, '--full-corpus', '--warn-only', *conflict)
                self.assertEqual(p.returncode, 2)

    def test_symlink_source_and_parent_hard_fail(self):
        target = self.put('outside/design', '# hidden')
        link = self.root / 'doc/system/link.design.md'
        link.parent.mkdir(parents=True)
        link.symlink_to(target)
        for name in ('system-design-traceability-check', 'doc-core-content-check'):
            p = self.run_checker(name, '--full-corpus', '--warn-only')
            self.assertEqual(p.returncode, 2, p.stdout + p.stderr)
        link.unlink()
        (self.root / 'doc/system/linked').symlink_to(target.parent, target_is_directory=True)
        for name in ('system-design-traceability-check', 'doc-core-content-check'):
            p = self.run_checker(name, '--full-corpus')
            self.assertEqual(p.returncode, 2, p.stdout + p.stderr)

    def test_product_parent_symlink_hard_fail_before_read(self):
        outside = self.root / 'external'
        outside.mkdir()
        (self.root / 'doc').mkdir()
        (self.root / 'doc/product').symlink_to(outside, target_is_directory=True)
        p = self.run_checker('doc-core-content-check', '--full-corpus')
        self.assertEqual(p.returncode, 2)

    def test_read_and_walk_errors_are_not_findings(self):
        self.put('doc/system/a.design.md', '# Legacy')
        with patch.object(Path, 'read_text', side_effect=PermissionError('denied')):
            with self.assertRaises(PermissionError):
                module.system.collect_full_corpus(self.root)
        def broken_walk(*args, **kwargs):
            kwargs['onerror'](PermissionError('walk denied'))
            return iter(())
        with patch.object(module.system.os, 'walk', broken_walk):
            with self.assertRaises(PermissionError):
                module.system.collect_full_corpus(self.root)
        (self.root / 'doc/system/a.design.md').write_bytes(b'\xff')
        for name in ('system-design-traceability-check', 'doc-core-content-check'):
            p = self.run_checker(name, '--full-corpus', '--warn-only')
            self.assertEqual(p.returncode, 2, p.stdout + p.stderr)

    def test_full_traceability_rejects_unsafe_reference(self):
        spec = importlib.util.spec_from_file_location('trace_fixture', HERE / 'system-design-traceability-check.test.py')
        fixture = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(fixture)
        self.put(fixture.DESIGN, fixture.DESIGN_HEADER)
        self.put(fixture.PRODUCT, fixture.PRODUCT_TEXT)
        with tempfile.TemporaryDirectory() as outside:
            foreign = Path(outside) / 'sample.test.py'
            foreign.write_text('secret')
            target = self.root / fixture.TEST_SOURCE
            target.parent.mkdir(parents=True)
            target.symlink_to(foreign)
            p = self.run_checker('system-design-traceability-check', '--full-corpus', '--warn-only')
            self.assertEqual(p.returncode, 2, p.stdout + p.stderr)
            self.assertIn('unsafe corpus target', p.stderr)

    def test_missing_dependency_is_hard_error(self):
        for name in ('system-design-traceability-check', 'doc-core-content-check'):
            p = subprocess.run([sys.executable, '-S', str(HERE / (name + '.py')), '--repo-root', str(self.root), '--full-corpus', '--warn-only'], capture_output=True, text=True, env={**os.environ, 'PYTHONPATH': ''})
            self.assertNotEqual(p.returncode, 0)
            self.assertRegex(p.stderr + p.stdout, 'markdown[-_]it')

    def test_reference_escape_is_hard_error_but_missing_is_finding(self):
        with tempfile.TemporaryDirectory() as outside:
            foreign = Path(outside) / 'secret'
            foreign.write_text('outside')
            link = self.root / 'link'
            link.symlink_to(foreign)
            with self.assertRaises(OSError):
                module.system.full_corpus_read(self.root, link)
        self.assertIsNone(module.system.full_corpus_read(self.root, self.root / 'absent'))
        with self.assertRaises(OSError):
            module.system.full_corpus_read(self.root, self.root / 'absent', required=True)

if __name__ == '__main__': unittest.main()

