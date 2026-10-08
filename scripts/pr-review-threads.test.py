#!/usr/bin/env python3
import importlib.util
from pathlib import Path
import contextlib
import io
import sys
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('threads', Path(__file__).with_name('pr-review-threads.py'))
threads = importlib.util.module_from_spec(spec)
spec.loader.exec_module(threads)

def page(nodes, more=False, cursor=None):
    return {'data': {'repository': {'pullRequest': {'reviewThreads': {'nodes': nodes, 'pageInfo': {'hasNextPage': more, 'endCursor': cursor}}}}}}

class PaginationTests(unittest.TestCase):
    @patch.object(threads, 'gh')
    def test_all_pages_are_read(self, gh):
        gh.side_effect = [page([{'id': 'first'}], True, 'next'), page([{'id': 'second'}])]
        self.assertEqual([t['id'] for t in threads.read_threads('owner', 'repo', 1)], ['first', 'second'])
        self.assertIn('cursor=next', gh.call_args.args)

    @patch.object(threads, 'gh', return_value=page([], True, None))
    def test_invalid_cursor_fails(self, gh):
        with self.assertRaisesRegex(RuntimeError, 'pagination'):
            threads.read_threads('owner', 'repo', 1)

    @patch.object(threads, 'gh', side_effect=RuntimeError('API error'))
    def test_query_error_propagates(self, gh):
        with self.assertRaisesRegex(RuntimeError, 'API error'):
            threads.read_threads('owner', 'repo', 1)

    @patch.object(threads, 'gh')
    def test_mutation_requires_an_explicit_pr(self, gh):
        with patch.object(sys, 'argv', ['review', '--resolve-all-unresolved']), contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit) as error:
                threads.main()
        self.assertEqual(error.exception.code, 2)
        gh.assert_not_called()

    @patch.object(threads, 'gh')
    def test_unverified_mutation_propagates_failure(self, gh):
        unresolved = page([{'id': 't1', 'isResolved': False}])
        gh.side_effect = [{'number': 7, 'url': 'https://github.com/owner/repo/pull/7'}, unresolved,
                          {'data': {'resolveReviewThread': {'thread': {'id': 't1', 'isResolved': False}}}}, unresolved]
        with patch.object(sys, 'argv', ['review', '7', '--resolve-thread', 't1']), contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit) as error:
                threads.main()
        self.assertEqual(error.exception.code, 1)

if __name__ == '__main__':
    unittest.main()
