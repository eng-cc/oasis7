#!/usr/bin/env python3
import importlib.util
from pathlib import Path
import subprocess
import sys
import unittest

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

if __name__ == '__main__': unittest.main()
