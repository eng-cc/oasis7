#!/usr/bin/env python3
"""Regression tests separating advisory findings from operational/integrity errors."""
import contextlib
import importlib.util
import io
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent

def load(name):
    spec = importlib.util.spec_from_file_location(name.replace('.', '_').replace('-', '_'), HERE / name)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module

fixture = load('product-doc-content-check.test.py')
overlay = load('product-doc-governance-check.py')
from doc_content_warnings import report_findings

class WarningTests(unittest.TestCase):
    def setUp(self):
        self.root, self.base, self.head = fixture.make_repo()
    def tearDown(self):
        shutil.rmtree(self.root)
    def invoke(self, name, *args):
        return subprocess.run([sys.executable,str(HERE/name),'--repo-root',str(self.root),*args],capture_output=True,text=True)
    def test_product_findings_warn_strict_fail(self):
        p = self.invoke('product-doc-content-check.py','--full-corpus','--warn-only')
        self.assertEqual(p.returncode,0,p.stdout+p.stderr)
        self.assertIn('::warning::product-doc-content:',p.stdout)
        self.assertNotEqual(self.invoke('product-doc-content-check.py','--full-corpus').returncode,0)
    def test_core_findings_warn_strict_fail(self):
        p = self.invoke('doc-core-content-check.py','--full-corpus','--warn-only')
        self.assertEqual(p.returncode,0,p.stdout+p.stderr)
        self.assertIn('missing-status-field',p.stdout)
        self.assertIn('including legacy REQ blocks',p.stdout)
        self.assertNotEqual(self.invoke('doc-core-content-check.py','--full-corpus').returncode,0)
    def test_system_findings_warn(self):
        file = self.root/'doc/system/broken.design.md'
        file.parent.mkdir()
        file.write_text('# Missing required trace tables\n')
        p = self.invoke('system-design-traceability-check.py','--base',self.base,'--head',self.head,'--worktree','--warn-only')
        self.assertEqual(p.returncode,0,p.stdout+p.stderr)
        self.assertIn('::warning::system-design-traceability:',p.stdout)
    def test_invalid_range_is_error(self):
        for name in ('product-doc-content-check.py','system-design-traceability-check.py','doc-core-content-check.py'):
            self.assertNotEqual(self.invoke(name,'--base','not-a-sha','--head',self.head,'--warn-only').returncode,0)
    def test_unreadable_utf8_is_error(self):
        (self.root/fixture.TOPIC).write_bytes(b'\xff')
        for name in ('product-doc-content-check.py','doc-core-content-check.py'):
            self.assertNotEqual(self.invoke(name,'--full-corpus','--warn-only').returncode,0)
    def test_symlink_corpus_still_fails(self):
        link = self.root/'doc/product/world-rules-core-gameplay/link.prd.md'
        link.symlink_to(self.root/fixture.TOPIC)
        for name in ('product-doc-content-check.py','doc-core-content-check.py'):
            p = self.invoke(name,'--full-corpus','--warn-only')
            self.assertNotEqual(p.returncode,0)
            self.assertIn('symlink-not-allowed',p.stdout)
    def test_annotation_escape_and_security(self):
        output=io.StringIO()
        with contextlib.redirect_stdout(output):
            self.assertEqual(report_findings(['file\n::error::injected%'],True),0)
            self.assertEqual(report_findings(['link escapes repository'],True),1)
            self.assertEqual(report_findings(['reference escapes the repository'],True),1)
        self.assertIn('%0A::error::injected%25',output.getvalue())
    def test_overlay_content_and_integrity_partition(self):
        for code, expected in [('section-contract',0),('traceability-contract',0),('metadata-content',0),('traceability-authority-format',0),('entry-contract',1),('metadata-contract',1),('topic-index-contract',1),('traceability-authority',1)]:
            with patch.object(overlay,'check',return_value=[f'product-doc-governance: {code}: fixture']),patch.object(sys,'argv',['checker','--warn-only']),contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(overlay.main(),expected,code)
    def test_overlay_exception_not_suppressed(self):
        with patch.object(overlay,'check',side_effect=OSError('read failed')),patch.object(sys,'argv',['checker','--warn-only']):
            with self.assertRaises(OSError): overlay.main()
    def scoped(self, name):
        return self.invoke(name, '--base', self.base, '--head', self.head, '--worktree')
    def test_new_and_materially_changed_documents_block(self):
        path = self.root / fixture.TOPIC
        path.write_text(path.read_text() + "\nNew product obligation.\n")
        self.assertNotEqual(self.scoped('doc-core-content-check.py').returncode, 0)
        path = self.root / 'doc/product/world-rules-core-gameplay/new.prd.md'
        path.write_text('# Missing required structure\n')
        self.assertNotEqual(self.scoped('product-doc-content-check.py').returncode, 0)
    def test_untouched_legacy_is_not_scoped_blocker(self):
        for name in ('product-doc-content-check.py', 'doc-core-content-check.py'):
            self.assertEqual(self.scoped(name).returncode, 0)
    def test_confirmed_formatting_only_skipped(self):
        path = self.root / fixture.TOPIC
        path.write_text('\n' + '\n'.join(line+'  ' for line in path.read_text().splitlines()) + '\n<!-- review note -->\n')
        for name in ('product-doc-content-check.py', 'doc-core-content-check.py'):
            p = self.scoped(name)
            self.assertEqual(p.returncode, 0, p.stdout+p.stderr)
    def test_tailoring_strict_cli_valid_and_invalid(self):
        path = self.root/'doc/system/tailored.design.md'
        path.parent.mkdir()
        text = '# 小型接口设计\n- 结构裁剪：理由：只改展示；范围：只读接口；风险：不改变持久化；映射：[问题目标](#接口说明)、[上游约束](#接口说明)、[边界结构](#接口说明)、[验证](#接口说明)、[决策风险](#接口说明)\n## 接口说明\n输入输出及验证仍由既有合同覆盖。\n'
        path.write_text(text)
        p = self.scoped('doc-core-content-check.py')
        self.assertEqual(p.returncode,0,p.stdout+p.stderr)
        self.assertIn('::notice::',p.stdout)
        path.write_text(text.replace('映射：[问题目标](#接口说明)、[上游约束](#接口说明)、[边界结构](#接口说明)、[验证](#接口说明)、[决策风险](#接口说明)', '映射：[验证](#接口说明)'))
        self.assertNotEqual(self.scoped('doc-core-content-check.py').returncode,0)
        path.write_text(text.replace('风险：不改变持久化；',''))
        self.assertNotEqual(self.scoped('doc-core-content-check.py').returncode,0)
    def test_retired_topic_skips_active_status_rules(self):
        path = self.root/fixture.TOPIC
        path.write_text(path.read_text().replace('- 生命周期：`active`', '- 生命周期：`retired`'))
        p = self.scoped('doc-core-content-check.py')
        self.assertEqual(p.returncode,0,p.stdout+p.stderr)
        self.assertIn('skipped active core/status rules',p.stdout)
    def test_root_prd_status_is_scoped(self):
        path = self.root/'doc/product/world-rules-core-gameplay/prd.md'
        path.write_text(path.read_text() + '\n### 功能：新增能力\n缺少状态摘要。\n')
        p = self.scoped('doc-core-content-check.py')
        self.assertNotEqual(p.returncode, 0)
        self.assertIn('missing-status-field', p.stdout)
    def test_overlay_enforces_source_not_linked_target(self):
        path = self.root/fixture.TOPIC
        path.write_text(path.read_text()+'\nNew scope.\n')
        argv = ['checker', '--repo-root', str(self.root), '--enforce-changed', '--base', self.base, '--head', self.head, '--worktree']
        for source, result in [(fixture.TOPIC,1),('doc/product/world-rules-core-gameplay/legacy.prd.md',0)]:
            error = f'product-doc-governance: section-contract: {source}: linked {fixture.TOPIC}'
            with patch.object(overlay,'check',return_value=[error]), patch.object(sys,'argv',argv), contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(overlay.main(), result)

    def test_overlay_empty_new_file_and_unknown_source_fail_closed(self):
        path = 'doc/product/world-rules-core-gameplay/empty.prd.md'
        (self.root/path).write_text('<!-- only comment -->\n')
        argv = ['checker','--repo-root',str(self.root),'--enforce-changed','--base',self.base,'--head',self.head,'--worktree']
        for detail in (f'{path}: missing section', 'unclassifiable source'):
            with patch.object(overlay,'check',return_value=[f'product-doc-governance: section-contract: {detail}']),patch.object(sys,'argv',argv),contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(overlay.main(),1)

    def test_ci_callers_forward_flags(self):
        text=(HERE/'doc-governance-check.sh').read_text()
        self.assertIn('scripts/product-doc-governance-check.py --enforce-changed',text)
        self.assertIn('scripts/system-design-traceability-check.py \\',text)
        self.assertEqual(text.count('scripts/product-doc-content-check.py --warn-only'),1)
        self.assertEqual(text.count('scripts/doc-core-content-check.py --warn-only'),1)
        self.assertIn('document-corpus-inventory-check.py" --repo-root',text)
        self.assertNotIn('continue-on-error',text)

if __name__=='__main__': unittest.main()
