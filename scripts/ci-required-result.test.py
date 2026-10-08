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
        self.plan = {'groups': ['baseline', 'windows'], 'matrices': {
            'baseline': {'include': [{'group': 'baseline'}]}, 'rust': {'include': []},
            'windows': {'include': [{'group': 'windows'}]}}}
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

    def test_illegal_empty_duplicate_unknown_matrices(self):
        changes = [lambda p: p['groups'].append('unknown'), lambda p: p['groups'].append('baseline'),
                   lambda p: p['groups'].remove('baseline'), lambda p: p['matrices']['windows'].update(include=[]),
                   lambda p: p['matrices'].pop('rust'), lambda p: p['matrices']['rust'].update(include=[{'group': 'rust'}])]
        for change in changes:
            plan = copy.deepcopy(self.plan)
            change(plan)
            with self.assertRaises(ValueError):
                result.validate(plan, self.needs, self.jobs)


if __name__ == '__main__':
    unittest.main()
