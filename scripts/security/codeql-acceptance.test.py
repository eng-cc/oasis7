#!/usr/bin/env python3
"""Independent offline boundary acceptance. No fixture proves hosted scans."""
import copy
import importlib.util
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


planner_tests = load('acceptance_planner_fixture', HERE / 'codeql-plan.test.py')
advisory_tests = load('acceptance_advisory_fixture', ROOT / 'scripts/pm/codeql-advisory.test.py')
workflow_tests = load('acceptance_workflow_fixture', HERE / 'codeql-workflow.test.py')
advisory = advisory_tests.module


class AcceptanceTests(unittest.TestCase):
    def planner(self):
        fixture = planner_tests.PlannerTests()
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        return fixture

    def provider(self):
        fixture = advisory_tests.AdvisoryTests()
        fixture.setUp()
        return fixture

    def test_CQ_T08_removed_embedding_still_routes_old_consumer(self):
        f = self.planner()
        f.write('src/lib.rs', '// removed include\n')
        (f.root / 'data/help.md').unlink()
        f.commit()
        result = f.run_plan()
        self.assertIn('embedded or possible build input: data/help.md', result['reasons']['rust-repo'])

    def test_CQ_T10_analysis_checkout_distinct_from_source(self):
        f = self.planner()
        f.write('tool.py'); source = f.commit()
        f.write('docs/guide.md', 'merge checkout only'); checkout = f.commit()
        result = f.run_plan(head=source, checkout=checkout)
        self.assertEqual(result['identity']['source_head'], source)
        self.assertEqual(result['identity']['checkout'], checkout)
        self.assertEqual(result['completeness']['diff_paths'], ['tool.py'])
        with self.assertRaisesRegex(ValueError, 'unreadable'):
            f.run_plan(checkout='0' * 40)

    def test_CQ_T13_frontend_embedding_has_both_consumers(self):
        f = self.planner()
        f.write('src/lib.rs', 'const PAGE: &str = include_str!("../web/index.html");')
        f.write('web/index.html', '<script>before()</script>'); f.base = f.commit()
        f.write('web/index.html', '<script>after()</script>'); f.commit()
        self.assertEqual(f.run_plan()['selected_units'], ['javascript-repo', 'rust-repo'])

    def test_CQ_T13_build_script_consumed_markdown(self):
        f = self.planner()
        f.write('build.rs', 'fn main() { std::fs::read_to_string("docs/guide.md").unwrap(); }')
        f.base = f.commit()
        f.write('docs/guide.md', 'changed build input'); f.commit()
        self.assertEqual(f.run_plan()['selected_units'], ['rust-repo'])

    def test_CQ_T05_workflow_consumed_shell(self):
        f = self.planner()
        f.write('.github/workflows/test.yml', 'jobs:\n  check:\n    steps:\n      - run: ./scripts/check.sh\n')
        f.write('scripts/check.sh', 'echo before'); f.base = f.commit()
        f.write('scripts/check.sh', 'echo after'); f.commit()
        self.assertEqual(f.run_plan()['selected_units'], ['actions-repo'])

    def test_CQ_T12_bootstrap_executes_no_candidate_command(self):
        outputs, calls = workflow_tests.WorkflowTests().plan(trusted=False)
        self.assertEqual(len(__import__('json').loads(outputs['matrix'])['include']), 4)
        self.assertFalse(any(command[0] == 'python3' for command, _ in calls))

    def test_CQ_T15_PR_cannot_opt_into_extended(self):
        outputs, _ = workflow_tests.WorkflowTests().plan(profile='extended')
        self.assertEqual(outputs['profile'], 'default')
        self.assertTrue(all(not r['queries'] for r in __import__('json').loads(outputs['matrix'])['include']))

    def test_CQ_T26_provider_owner_and_run_repository_are_authorities(self):
        for endpoint, field, value in (
                ('apps/github-actions', 'owner', {'login': 'attacker', 'type': 'Organization'}),
                ('apps/github-actions', 'slug', 'same-name-app')):
            f = self.provider(); f.responses[endpoint][field] = value
            self.assertFalse(advisory.explain_unstable(f.data, f.read)['explained'])
        for field, value in [('repository', {'full_name': 'other/repo'}), ('pull_requests', [{'number': 8}]),
                             ('event', 'push'), ('path', '.github/workflows/other.yml')]:
            f = self.provider()
            endpoint = next(k for k in f.responses if '/runs?head_sha=' in k)
            f.responses[endpoint]['workflow_runs'][0][field] = value
            with self.subTest(field=field):
                self.assertFalse(advisory.explain_unstable(f.data, f.read)['explained'])

    def test_CQ_T30_duplicate_suite_and_job_do_not_authorize(self):
        for collection in ('workflow_runs', 'jobs'):
            f = self.provider()
            payload = next(v for v in f.responses.values() if collection in v)
            payload[collection].append(copy.deepcopy(payload[collection][0]))
            payload['total_count'] = 2
            self.assertFalse(advisory.explain_unstable(f.data, f.read)['explained'])

    def test_CQ_T30_anomaly_budget_is_bounded_without_API_reads(self):
        f = self.provider()
        f.data['statusCheckRollup'] = [dict(databaseId=i, name=f'CodeQL / fake-{i}', conclusion='FAILURE') for i in range(13)]
        self.assertFalse(advisory.explain_unstable(f.data, f.read)['explained'])
        self.assertEqual(f.calls, [])

    def test_CQ_T28_required_same_name_wrong_app_stays_required(self):
        f = self.provider()
        f.data['policy_discovery']['required_status_checks'] = [{'context': 'CodeQL / plan', 'app_id': 99}]
        self.assertFalse(advisory.explain_unstable(f.data, f.read)['explained'])
        self.assertEqual(f.calls, [])

    def test_CQ_T24_platform_native_unknown_is_explicit(self):
        f = self.provider()
        f.data['statusCheckRollup'][0].update(name='Code scanning results / CodeQL', checkSuite={'app': {'databaseId': 42}})
        result = advisory.explain_unstable(f.data, f.read)
        self.assertFalse(result['explained'])
        self.assertIn('hosted association required', result['reason'])

    def test_CQ_T37_maintenance_candidate_ref_cannot_activate(self):
        f = self.planner()
        f.git('branch', 'trusted-default', f.base)
        f.write('tool.py'); f.commit()
        with self.assertRaisesRegex(ValueError, 'trusted default branch'):
            f.run_plan(event='workflow_dispatch', mode='baseline', full=True, default_branch='trusted-default')

    def test_CQ_T35_T36_no_privileged_or_cache_fallback(self):
        workflow = workflow_tests.WORKFLOW
        self.assertIn("OASIS7_CODEQL_MODE || 'off'", workflow)
        self.assertIn('dependency-caching: false', workflow)
        self.assertNotIn('actions/cache', workflow)
        self.assertNotIn('pull_request_target', workflow)
        self.assertNotIn('secrets.', workflow)
        self.assertEqual(workflow.count('security-events: write'), 1)
        self.assertNotIn('contents: write', workflow)


if __name__ == '__main__':
    unittest.main()
