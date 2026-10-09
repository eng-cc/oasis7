#!/usr/bin/env python3
import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('result', Path(__file__).with_name('ci-required-result.py'))
result = importlib.util.module_from_spec(spec)
spec.loader.exec_module(result)


class Results(unittest.TestCase):
    def setUp(self):
        self.jobs = {'baseline': 'baseline', 'rust': 'rust', 'windows': 'windows'}
        self.plan = {'groups': ['baseline', 'windows']}
        self.needs = {'select': {'result': 'success'}, 'baseline': {'result': 'success'},
                      'rust': {'result': 'skipped'}, 'windows': {'result': 'success'}}

    def test_expected_success(self):
        self.assertTrue(result.validate(self.plan, self.needs, self.jobs))
        self.needs['rust']['result'] = 'success'
        self.assertTrue(result.validate(self.plan, self.needs, self.jobs))

    def test_selected_failure_cancelled_missing_skipped_unknown(self):
        for value in ['failure', 'cancelled', 'skipped', 'unknown', None]:
            with self.subTest(value=value), self.assertRaises(ValueError):
                self.needs['windows'] = {'result': value}
                result.validate(self.plan, self.needs, self.jobs)
        del self.needs['windows']
        with self.assertRaises(ValueError):
            result.validate(self.plan, self.needs, self.jobs)

    def test_select_failure_or_skip(self):
        for value in ['failure', 'cancelled', 'skipped', None]:
            self.needs['select'] = {'result': value}
            with self.assertRaises(ValueError):
                result.validate(self.plan, self.needs, self.jobs)

    def test_unselected_failure_is_not_hidden(self):
        for value in ['failure', 'cancelled', 'unknown', None]:
            self.needs['rust'] = {'result': value}
            with self.assertRaises(ValueError):
                result.validate(self.plan, self.needs, self.jobs)

    def test_added_workflow_dependency_must_explicitly_succeed(self):
        # Adding another gate dependency must never hide its failed or absent result.
        invalid = [{'result': value} for value in ('failure', 'cancelled', 'skipped', 'unknown', None)]
        invalid.extend([{}, {'result': 1}, None, [], 'success', True])
        for entry in invalid:
            with self.subTest(entry=entry), self.assertRaises(ValueError):
                self.needs['new-required-dependency'] = entry
                result.validate(self.plan, self.needs, self.jobs)
        self.needs['new-required-dependency'] = {'result': 'success'}
        self.assertTrue(result.validate(self.plan, self.needs, self.jobs))
        self.assertEqual(self.needs['rust']['result'], 'skipped')

    def test_invalid_groups_fail_closed(self):
        for groups in (None, {}, 'baseline', [], ['baseline', 'unknown'],
                       ['baseline', 'baseline'], ['windows'], ['baseline', {}]):
            with self.subTest(groups=groups), self.assertRaises(ValueError):
                result.validate({'groups': groups}, self.needs, self.jobs)

    def test_extra_legacy_plan_fields_do_not_change_results(self):
        self.plan['matrices'] = {'legacy': 'unused'}
        self.assertTrue(result.validate(self.plan, self.needs, self.jobs))


if __name__ == '__main__':
    unittest.main()
