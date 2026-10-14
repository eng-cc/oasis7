"""Independent, offline acceptance tests for requirement-trace.py (stdlib only)."""
import copy
import importlib.util
import contextlib
import datetime as dt
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

SCRIPT = Path(__file__).resolve().parent / 'requirement-trace.py'
ID = 'O7-REQ-276983b3-3187-405e-8453-e2785f01c5e6'
OTHER = 'O7-REQ-7b97fe34-2be1-4b6c-9e8a-54fb03b52c16'
THIRD = 'O7-REQ-e12138fa-bc2c-461a-88b2-c92654e6a175'
REPO = 'eng-cc/oasis7'
STAMP = '2026-10-14T00:00:00Z'


def load_module():
    spec = importlib.util.spec_from_file_location('requirement_trace_under_test', SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class Base(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.m = load_module()

    def parse(self, text, path='doc/test.md'):
        return self.m.parse_markdown(text, {'kind': 'document', 'path': path})

    def ids(self, result, role=None):
        return [r['id'] for r in result['records'] if role is None or r['role'] == role]

    def issue(self, n=1, body=None, kind='issue', **kwargs):
        item = dict(number=n, kind=kind, url=f'https://github.com/{REPO}/{"pull" if kind == "pull_request" else "issues"}/{n}',
                    body=body or f'Trace refs: {ID}', state='closed', updated_at=STAMP)
        item['html_url'] = item['url']
        item.update(kwargs)
        return item

    def snapshot(self, records=()):
        return {'repository': REPO, 'fetched_at': STAMP, 'coverage': {
            'issues': {'complete': True}, 'pull_requests': {'complete': True}}, 'records': list(records)}


class ParserTests(Base):
    def test_exact_uuid_v4(self):
        result = self.parse(f'# Requirement\nTrace ID: {ID}\nTrace refs: {ID}, {OTHER}\n')
        self.assertEqual(self.ids(result, 'declaration'), [ID])
        self.assertEqual(set(self.ids(result, 'reference')), {ID, OTHER})

    def test_invalid_and_token_embedded_ids_do_not_match(self):
        bad = [ID.upper(), ID[:-1], ID + 'f', 'x' + ID, ID + '_suffix', ID.replace('-405e-', '-505e-'),
               ID.replace('-8453-', '-7453-'), ID.replace('276983b3-', '276983b3')]
        for value in bad:
            with self.subTest(value=value):
                self.assertEqual(self.ids(self.parse(f'Trace ID: {value}\n')), [])

    def test_plain_mentions_are_not_formal_relations(self):
        self.assertEqual(self.ids(self.parse(f'This text mentions {ID}.\n[requirement]({ID})\n')), [])

    def test_code_comment_and_blockquote_examples(self):
        text = f'''```md
Trace ID: {ID}
```
~~~markdown
Trace refs: {OTHER}
~~~
<!-- Trace ID: {ID} -->
<!--
Trace refs: {OTHER}
-->
    Trace ID: {ID}
> Trace refs: {OTHER}
`Trace ID: {ID}`
'''
        self.assertEqual(self.ids(self.parse(text)), [])

    def test_example_section_does_not_hide_later_real_section(self):
        for heading in ['Example', 'Examples', 'Template', '示例', '模板']:
            with self.subTest(heading=heading):
                text = f'# Document\n## {heading}\nTrace ID: {OTHER}\n## Real requirement\nTrace ID: {ID}\n'
                self.assertEqual(self.ids(self.parse(text), 'declaration'), [ID])

    def test_template_file_is_not_formal(self):
        for path in ['.github/pull_request_template.md', 'doc/templates/requirement.md']:
            with self.subTest(path=path):
                self.assertEqual(self.ids(self.parse(f'Trace ID: {ID}\n', path)), [])

    def test_backtick_formatted_value_and_label_are_formal(self):
        for line in [f'Trace ID: `{ID}`', f'`Trace ID:` {ID}']:
            with self.subTest(line=line):
                self.assertEqual(self.ids(self.parse(line), 'declaration'), [ID])

    def test_template_manual_prose_can_carry_real_reference(self):
        text = f'# Product documentation templates\n## Requirement cards\nTrace refs: {ID}; this section implements [governance](governance.md#rule).\n```text\nTrace ID: {OTHER}\n```\n'
        result = self.parse(text, 'doc/product-documentation-standard.templates.md')
        self.assertEqual(self.ids(result, 'reference'), [ID])
        self.assertEqual(self.ids(result, 'declaration'), [])

    def test_table_many_to_many_preserves_row_context(self):
        result = self.parse(f'| Requirement | Scope |\n| --- | --- |\n| Trace refs: {ID}, {OTHER} | only rendering |\n')
        self.assertEqual(set(self.ids(result)), {ID, OTHER})
        self.assertTrue(all('only rendering' in r['context'] for r in result['records']))

    def test_explicit_anchor_and_real_link_preserved(self):
        result = self.parse(f'<a id="stable-anchor"></a>\n### Requirement\nTrace ID: {ID}\nTrace refs: {OTHER}; [source](other.md#exact-target)\n')
        self.assertIn('stable-anchor', result['anchors'])
        record = next(r for r in result['records'] if r['id'] == ID)
        self.assertEqual(record['anchor'], 'stable-anchor')
        self.assertIn('other.md#exact-target', json.dumps(result, ensure_ascii=False))

    def test_lifecycle_prose_preserved_without_declaration_inference(self):
        text = f'# Old requirement\nTrace ID: {ID}\n拆分为 {OTHER} 和 {THIRD}，退役；[后继](new.md#new)。需人工核对。\n'
        result = self.parse(text)
        self.assertEqual(self.ids(result, 'declaration'), [ID])
        self.assertIn('拆分为', json.dumps(result, ensure_ascii=False))
        self.assertTrue(result['diagnostics'], 'Unstructured lifecycle text must be reported for review')


class IndexTests(Base):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.addCleanup(self.temp.cleanup)

    def put(self, path, text):
        file = self.root / path
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(text, encoding='utf-8')
        return file

    def build(self, github=None):
        return self.m.build_index(self.root, github=github, repository=REPO, revision='a' * 40)

    def test_one_declaration_many_refs_and_reverse_many_to_many(self):
        self.put('req.md', f'# A\nTrace ID: {ID}\n# B\nTrace ID: {OTHER}\n')
        self.put('design.md', f'# Design\nTrace refs: {ID}, {OTHER}\n')
        self.put('other.md', f'Trace refs: {ID}\n')
        index = self.build(self.snapshot([self.issue()]))
        result = self.m.query_index(index, trace_id=ID)
        self.assertEqual(len([r for r in result['records'] if r['role'] == 'declaration']), 1)
        self.assertGreaterEqual(len([r for r in result['records'] if r['role'] == 'reference']), 3)
        self.assertNotIn('duplicate_declaration', json.dumps(index['diagnostics']))
        reverse = self.m.query_index(index, document='design.md')
        self.assertEqual(set(reverse['ids']), {ID, OTHER})
        reverse_task = self.m.query_index(index, task=f'https://github.com/{REPO}/issues/1')
        self.assertEqual(set(reverse_task['ids']), {ID})

    def test_duplicate_declarations_not_silently_merged(self):
        self.put('a.md', f'Trace ID: {ID}\n')
        self.put('b.md', f'Trace ID: {ID}\n')
        result = self.build()
        self.assertEqual(len(self.ids(result, 'declaration')), 2)
        self.assertIn('duplicate', json.dumps(result['diagnostics']).lower())

    def test_rename_keeps_id_and_changes_location(self):
        old = self.put('old.md', f'<a id="kept"></a>\n# Requirement\nTrace ID: {ID}\n')
        first = self.build()
        old.rename(self.root / 'new.md')
        second = self.build()
        self.assertEqual(self.ids(first), self.ids(second))
        self.assertEqual(second['records'][0]['source']['path'], 'new.md')
        self.assertEqual(second['records'][0]['anchor'], 'kept')

    def test_outside_symlink_not_scanned(self):
        with tempfile.TemporaryDirectory() as outside:
            target = Path(outside) / 'secret.md'
            target.write_text(f'Trace ID: {OTHER}\n')
            (self.root / 'escape.md').symlink_to(target)
            self.put('inside.md', f'Trace ID: {ID}\n')
            self.assertNotIn(OTHER, self.ids(self.build()))

    def test_read_only_rebuild_deterministic_relationships(self):
        file = self.put('req.md', f'# A\nTrace ID: {ID}\n')
        before = file.read_bytes()
        one, two = self.build(), self.build()
        self.assertEqual(one['records'], two['records'])
        self.assertEqual(before, file.read_bytes())
        self.assertEqual(sorted(p.name for p in self.root.iterdir()), ['req.md'])
        self.assertEqual(one['schema_version'], 1)
        self.assertTrue(one['generated_at'])
        serialized = json.dumps(one)
        self.assertIn('a' * 40, serialized)
        self.assertIn(REPO, serialized)

    def test_broken_anchor_and_dangling_ref_diagnostics(self):
        self.put('req.md', f'# Existing\nTrace ID: {ID}\n')
        self.put('design.md', f'Trace refs: {OTHER}; [source](req.md#absent)\n')
        result = self.build()
        text = json.dumps(result['diagnostics']).lower()
        self.assertTrue('anchor' in text or 'location' in text)
        self.assertTrue('dangling' in text or 'unresolved' in text)

    def test_empty_github_search_and_offline_do_not_prove_missing(self):
        self.put('refs.md', f'Trace refs: {ID}\n')
        for snapshot in [None, self.snapshot()]:
            with self.subTest(offline=snapshot is None):
                result = self.m.query_index(self.build(snapshot), trace_id=ID)
                self.assertNotEqual(result.get('status'), 'missing')
                self.assertNotEqual(result.get('coverage'), 'complete')
                self.assertNotIn('"status": "missing"', json.dumps(result))

    def test_github_fields_timestamps_and_merged_record(self):
        result = self.build(self.snapshot([self.issue(kind='pull_request', merged_at=STAMP)]))
        text = json.dumps(result)
        self.assertIn('pull_request', text)
        self.assertIn(STAMP, text)
        self.assertEqual(self.ids(result, 'reference'), [ID])

    def test_snapshot_identity_mismatch_is_rejected(self):
        snapshot = self.snapshot()
        snapshot['repository'] = 'different/repository'
        with self.assertRaises(ValueError):
            self.build(snapshot)

    def test_query_groups_are_explicit_source_classification(self):
        self.put('req.md', f'Trace ID: {ID}\n产品层不适用：工程治理。\n')
        self.put('design.design.md', f'Trace refs: {ID}\n')
        self.put('task-notes.md', f'Trace refs: {ID}\n尚未形成其他任务。\n')
        result = self.m.query_index(self.build(self.snapshot([self.issue()])), trace_id=ID)
        self.assertEqual({k: len(v) for k, v in result['groups'].items()},
                         {'declarations': 1, 'design_references': 1, 'tasks': 1, 'other_document_references': 1})
        self.assertIn('产品层不适用', json.dumps(result, ensure_ascii=False))
        self.assertIn('尚未形成', json.dumps(result, ensure_ascii=False))

    def test_github_comments_and_title_are_not_formal_task_body(self):
        item = self.issue(body='No formal refs here.', title=f'Trace refs: {ID}', comments=[{'body': f'Trace refs: {ID}'}])
        self.assertEqual(self.ids(self.build(self.snapshot([item]))), [])


class CacheCliTests(Base):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.root = self.base / 'repository'
        self.root.mkdir()
        (self.root / 'req.md').write_text(f'Trace ID: {ID}\n')
        self.cache = self.base / 'trace-cache.json'

    def invoke(self, *args):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            result = self.m.main(['--root', str(self.root), '--format', 'json', *args])
        return result, out.getvalue(), err.getvalue()

    def test_default_cli_is_offline_and_json_query_has_time(self):
        with mock.patch.object(self.m, 'fetch_github', side_effect=AssertionError('unexpected network')):
            code, out, err = self.invoke('query', ID)
        self.assertEqual(code, 0, err)
        value = json.loads(out)
        self.assertEqual(value['ids'], [ID])
        self.assertEqual(value['coverage'], 'partial')
        self.assertTrue(value['generated_at'])

    def test_invalid_cli_id_rejected_without_network(self):
        with mock.patch.object(self.m, 'fetch_github', side_effect=AssertionError('unexpected network')):
            code, _, _ = self.invoke('query', ID.upper())
        self.assertEqual(code, 2)

    def test_cache_hit_has_freshness_and_no_fetch(self):
        snapshot = self.snapshot([self.issue()])
        snapshot['fetched_at'] = self.m.utc_now()
        self.cache.write_text(json.dumps(snapshot))
        with mock.patch.object(self.m, 'fetch_github', side_effect=AssertionError('unexpected network')):
            code, out, err = self.invoke('--github', REPO, '--cache', str(self.cache), 'query', ID)
        self.assertEqual(code, 0, err)
        self.assertEqual(json.loads(out)['sources'][1]['freshness']['status'], 'fresh')

    def test_custom_max_age_reconciles_coverage(self):
        old = self.snapshot([self.issue()])
        old['fetched_at'] = (dt.datetime.now(dt.timezone.utc) - dt.timedelta(hours=30)).isoformat()
        self.cache.write_text(json.dumps(old))
        code, out, err = self.invoke('--cache', str(self.cache), '--max-age', '48', 'query', ID)
        self.assertEqual(code, 0, err)
        source = json.loads(out)['sources'][1]
        self.assertEqual(source['freshness']['status'], 'fresh')
        self.assertTrue(source['complete'])

    def test_known_read_failure_remains_partial_despite_complete_listing(self):
        snapshot = self.snapshot([self.issue()])
        snapshot['fetched_at'] = self.m.utc_now()
        snapshot['known_links'] = [{'url': f'https://github.com/{REPO}/issues/99', 'status': 'permission_denied'}]
        self.cache.write_text(json.dumps(snapshot))
        code, out, err = self.invoke('--cache', str(self.cache), 'query', ID)
        self.assertEqual(code, 0, err)
        self.assertEqual(json.loads(out)['coverage'], 'partial')

    def test_stale_cache_without_refresh_stays_partial(self):
        old = self.snapshot([self.issue()])
        old['fetched_at'] = '2000-01-01T00:00:00Z'
        self.cache.write_text(json.dumps(old))
        with mock.patch.object(self.m, 'fetch_github', side_effect=AssertionError('unexpected network')):
            code, out, err = self.invoke('--cache', str(self.cache), 'query', ID)
        self.assertEqual(code, 0, err)
        result = json.loads(out)
        self.assertEqual(result['coverage'], 'partial')
        self.assertEqual(result['sources'][1]['freshness']['status'], 'stale')
        self.assertTrue(any(r['source']['kind'] == 'issue' for r in result['records']))

    def test_refresh_failure_preserves_stale_cache_and_relationships(self):
        old = self.snapshot([self.issue()])
        old['fetched_at'] = '2000-01-01T00:00:00Z'
        self.cache.write_text(json.dumps(old))
        before = self.cache.read_bytes()
        failed = self.snapshot()
        failed['coverage']['issues'] = {'complete': False, 'errors': ['rate_limited']}
        with mock.patch.object(self.m, 'fetch_github', return_value=failed):
            code, out, err = self.invoke('--github', REPO, '--cache', str(self.cache), '--refresh', 'query', ID)
        self.assertEqual(code, 0, err)
        result = json.loads(out)
        self.assertEqual(result['coverage'], 'partial')
        self.assertEqual(result['sources'][1]['freshness']['status'], 'stale')
        self.assertIn('rate_limited', out)
        self.assertTrue(any(r['source']['kind'] == 'issue' for r in result['records']))
        self.assertEqual(before, self.cache.read_bytes())

    def test_cache_cannot_overwrite_source_or_symlink(self):
        target = self.root / 'req.md'
        before = target.read_bytes()
        link = self.base / 'cache-link.json'
        link.symlink_to(target)
        for path in [target, self.root / 'other.json', link]:
            with self.subTest(path=path):
                with self.assertRaises(ValueError):
                    self.m.write_cache(path, self.snapshot(), self.root)
        self.assertEqual(before, target.read_bytes())

    def test_cache_cannot_clobber_unrelated_existing_file(self):
        target = self.base / 'source.json'
        target.write_text('User-owned source, not a derived cache')
        with self.assertRaises(ValueError):
            self.m.write_cache(target, self.snapshot(), self.root)
        self.assertEqual(target.read_text(), 'User-owned source, not a derived cache')

    def test_known_url_not_silently_ignored_with_cache(self):
        self.cache.write_text(json.dumps(self.snapshot()))
        url = f'https://github.com/{REPO}/issues/99'
        with mock.patch.object(self.m, 'fetch_github', return_value=self.snapshot([self.issue(n=99)])) as fetch:
            code, out, err = self.invoke('--github', REPO, '--cache', str(self.cache), '--known-url', url, 'query', ID)
        self.assertEqual(code, 0, err)
        self.assertTrue(fetch.called, 'Explicit known URL must be read, even if a cache already exists')
        self.assertIn(url, out)


class GithubTests(Base):
    def runner_for(self, pages, calls, errors=None):
        errors = errors or {}
        def runner(args, **kwargs):
            calls.append((args, kwargs))
            self.assertIsInstance(args, (list, tuple))
            self.assertFalse(kwargs.get('shell', False))
            self.assertEqual(args[0], 'gh')
            endpoint = next((a for a in args if '/issues' in a or '/pulls' in a), '')
            if endpoint in errors:
                error = errors[endpoint]
                if isinstance(error, Exception):
                    raise error
                return subprocess.CompletedProcess(args, 1, '', error)
            return subprocess.CompletedProcess(args, 0, json.dumps(pages.get(endpoint, [])), '')
        return runner

    def endpoint(self, name, page):
        return f'repos/{REPO}/{name}?state=all&per_page=100&page={page}'

    def test_pagination_closed_issues_and_merged_prs(self):
        calls = []
        records = [self.issue(n=n) for n in range(1, 101)]
        pull = self.issue(n=200, kind='pull_request', merged_at=STAMP)
        pull['html_url'] = pull['url']
        pages = {self.endpoint('issues', 1): records,
                 self.endpoint('issues', 2): [self.issue(n=101)],
                 self.endpoint('pulls', 1): [pull]}
        result = self.m.fetch_github(REPO, runner=self.runner_for(pages, calls))
        self.assertEqual(len(result['records']), 102)
        self.assertTrue(result['coverage']['issues']['complete'])
        self.assertTrue(result['coverage']['pull_requests']['complete'])
        self.assertTrue(any('page=2' in str(args) for args, _ in calls))
        self.assertTrue(all('state=all' in str(args) for args, _ in calls))
        self.assertIn('closed', json.dumps(result))
        self.assertIn('merged', json.dumps(result))

    def test_pull_entries_from_issues_endpoint_not_double_counted(self):
        calls = []
        pull_issue = self.issue(n=2)
        pull_issue['pull_request'] = {'url': f'https://api.github.com/repos/{REPO}/pulls/2'}
        pages = {self.endpoint('issues', 1): [pull_issue],
                 self.endpoint('pulls', 1): [self.issue(n=2, kind='pull_request')]}
        result = self.m.fetch_github(REPO, runner=self.runner_for(pages, calls))
        self.assertEqual(len(result['records']), 1)

    def test_failures_are_partial_and_keep_successful_pages(self):
        failures = ['HTTP 403: API rate limit exceeded', 'HTTP 403: Resource not accessible',
                    'HTTP 401: Bad credentials', subprocess.TimeoutExpired('gh', 1)]
        for failure in failures:
            with self.subTest(error=str(failure)):
                calls = []
                pages = {self.endpoint('issues', 1): [self.issue(n=n) for n in range(1, 101)]}
                errors = {self.endpoint('issues', 2): failure}
                result = self.m.fetch_github(REPO, runner=self.runner_for(pages, calls, errors))
                self.assertFalse(result['coverage']['issues']['complete'])
                self.assertEqual(len(result['records']), 100)
                self.assertNotIn('"status": "missing"', json.dumps(result))

    def test_known_links_cannot_redirect_read_or_execute(self):
        calls = []
        urls = ['https://attacker.invalid/steal', f'https://github.com/{REPO}/issues/2?token=secret',
                'https://github.com/elsewhere/private/issues/1',
                f'https://github.com/{REPO}/issues/../../secret', 'file:///etc/passwd',
                f'https://github.com/{REPO}/issues/1;touch /tmp/SHOULD_NOT_EXIST']
        self.m.fetch_github(REPO, runner=self.runner_for({}, calls), known_urls=urls)
        serialized = json.dumps(calls)
        for token in ['attacker', 'token=secret', 'elsewhere', '../', 'file://', 'touch']:
            self.assertNotIn(token, serialized)

    def test_known_task_direct_read_is_retained(self):
        calls = []
        url = f'https://github.com/{REPO}/issues/99'
        pages = {f'repos/{REPO}/issues/99': self.issue(n=99)}
        result = self.m.fetch_github(REPO, runner=self.runner_for(pages, calls), known_urls=[url])
        self.assertEqual([r['number'] for r in result['records']], [99])
        self.assertEqual(result['known_links'][0]['status'], 'read')

    def test_repeated_page_stops_without_claiming_complete(self):
        calls = []
        page = [self.issue(n=n) for n in range(1, 101)]
        pages = {self.endpoint('issues', 1): page, self.endpoint('issues', 2): page}
        result = self.m.fetch_github(REPO, runner=self.runner_for(pages, calls))
        self.assertFalse(result['coverage']['issues']['complete'])
        self.assertIn('repeated_page', json.dumps(result))
        self.assertLessEqual(len(calls), 3)

    def test_error_text_redacts_credentials(self):
        calls = []
        errors = {self.endpoint('issues', 1): 'HTTP 403 token=SECRET_DO_NOT_LOG'}
        result = self.m.fetch_github(REPO, runner=self.runner_for({}, calls, errors))
        self.assertNotIn('SECRET_DO_NOT_LOG', json.dumps(result))
        self.assertIn('permission_denied', json.dumps(result))

    def test_repository_argument_validation_before_runner(self):
        for value in ['../private', 'eng-cc/oasis7;touch /tmp/x', 'https://evil.invalid/repo', '-o/repo']:
            calls = []
            try:
                self.m.fetch_github(value, runner=self.runner_for({}, calls))
            except ValueError:
                pass
            self.assertEqual(calls, [], value)


if __name__ == '__main__':
    unittest.main()
