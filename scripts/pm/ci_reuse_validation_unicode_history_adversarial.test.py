#!/usr/bin/env python3
"""Adversarial regressions for Unicode in exhaustive validation-run history."""
from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path
from unittest.mock import patch


HERE = Path(__file__).parent
READBACK_TEST_SPEC = importlib.util.spec_from_file_location(
    "ci_reuse_validation_readback_test_for_unicode_history",
    HERE / "ci_reuse_validation_readback.test.py",
)
readback_tests = importlib.util.module_from_spec(READBACK_TEST_SPEC)
assert READBACK_TEST_SPEC.loader is not None
sys.modules[READBACK_TEST_SPEC.name] = readback_tests
READBACK_TEST_SPEC.loader.exec_module(readback_tests)

readback = readback_tests.readback
contract = readback.contract


def run_page(runs, *, total_count=None, has_next=False):
    rows = [dict(run) for run in runs]
    return {
        "total_count": len(rows) if total_count is None else total_count,
        "runs": rows,
        "has_next": has_next,
    }


def authority_and_title(fixture):
    authority = contract.resolve_records_for_readback(fixture["comments"])
    title = contract.expected_run_title(authority)
    assert title.isascii()
    return authority, title


class UnicodeHistoryContractTests(unittest.TestCase):
    def setUp(self):
        self.fixture = readback_tests.readback_fixture()
        self.authority, self.expected_title = authority_and_title(self.fixture)

    def test_complete_history_accepts_unicode_noncandidate_and_selects_exact_ascii_run(self):
        historical = {"id": 699, "display_title": "nightly 日本語 🌱 — прошлый запуск"}
        pages = (run_page([historical, self.fixture["run_api"]]),)

        complete = contract.collect_workflow_runs(pages)
        selected = contract.select_unique_run(complete, self.authority)

        self.assertEqual(2, len(complete))
        self.assertEqual(historical["display_title"], complete[0]["display_title"])
        self.assertEqual(self.fixture["run"]["id"], selected["id"])
        self.assertEqual(self.expected_title, selected["display_title"])
        self.assertTrue(selected["display_title"].isascii())

    def test_complete_history_accepts_long_unicode_noncandidate_and_selects_exact_ascii_run(self):
        long_title = "🌱" * 1100
        self.assertGreater(len(long_title), 1024)
        self.assertGreater(len(long_title.encode("utf-8")), 4096)
        historical = {"id": 698, "display_title": long_title}
        complete = contract.collect_workflow_runs(
            (run_page([historical, self.fixture["run_api"]]),),
        )

        selected = contract.select_unique_run(complete, self.authority)

        self.assertEqual(2, len(complete))
        self.assertEqual(long_title, complete[0]["display_title"])
        self.assertEqual(self.fixture["run"]["id"], selected["id"])
        self.assertEqual(self.expected_title, selected["display_title"])
        self.assertTrue(selected["display_title"].isascii())

    def test_long_unicode_near_candidates_remain_blocking(self):
        validation_id = self.authority.validation_id
        prefix = self.expected_title[: -len(validation_id)]
        long_tail = "🌱" * 1100
        near_candidates = (
            {
                "id": 697,
                "display_title": prefix + "different-request-key-" + long_tail,
            },
            {
                "id": 696,
                "display_title": long_tail + "|" + validation_id,
            },
        )

        for near in near_candidates:
            with self.subTest(run_id=near["id"]):
                self.assertGreater(len(near["display_title"]), 1024)
                self.assertGreater(len(near["display_title"].encode("utf-8")), 4096)
                with self.assertRaisesRegex(
                    contract.ContractError, "another or malformed",
                ):
                    contract.select_unique_run([near], self.authority)
                with self.assertRaisesRegex(
                    contract.ContractError, "unique workflow run ID",
                ):
                    contract.select_unique_run([self.fixture["run_api"], near], self.authority)

    def test_unicode_near_candidates_are_not_ignored_or_normalized(self):
        validation_id = self.authority.validation_id
        prefix = self.expected_title[: -len(validation_id)]
        near_prefix = {
            "id": 701,
            "display_title": prefix + "different-key-🌱",
        }
        near_suffix = {
            "id": 702,
            "display_title": "unrelated history—日本語|" + validation_id,
        }

        for near in (near_prefix, near_suffix):
            with self.subTest(title=near["display_title"]):
                with self.assertRaisesRegex(
                    contract.ContractError, "unique workflow run ID|another or malformed",
                ):
                    contract.select_unique_run([near], self.authority)
                with self.assertRaisesRegex(
                    contract.ContractError, "unique workflow run ID",
                ):
                    contract.select_unique_run([self.fixture["run_api"], near], self.authority)

        confusable_prefix = {
            "id": 703,
            "display_title": self.expected_title.replace("oasis7", "οasis7", 1)
            + "different-key",
        }
        confusable_suffix = {
            "id": 704,
            "display_title": "unrelated history|" + validation_id[:-1] + "０",
        }
        selected = contract.select_unique_run(
            [confusable_prefix, confusable_suffix, self.fixture["run_api"]], self.authority,
        )
        self.assertEqual(self.fixture["run"]["id"], selected["id"])
        for confusable in (confusable_prefix, confusable_suffix):
            with self.subTest(title=confusable["display_title"]):
                with self.assertRaisesRegex(contract.ContractError, "unique workflow run ID"):
                    contract.select_unique_run([confusable], self.authority)

    def test_missing_nonstring_and_malformed_utf8_titles_fail_closed(self):
        malformed_rows = (
            {"id": 705},
            {"id": 706, "display_title": None},
            {"id": 707, "display_title": 42},
            {"id": 708, "display_title": "unpaired surrogate: \ud800"},
        )
        for row in malformed_rows:
            with self.subTest(row=row):
                with self.assertRaises(contract.ContractError):
                    contract.collect_workflow_runs((run_page([row]),))

    def test_total_count_drift_and_duplicate_ids_still_fail_with_unicode_rows(self):
        first = run_page(
            [{"id": 709, "display_title": "history-first"}],
            total_count=2,
            has_next=True,
        )
        second = run_page(
            [{"id": 710, "display_title": "history-second"}],
            total_count=3,
        )
        with self.assertRaisesRegex(contract.ContractError, "total_count changed"):
            contract.collect_workflow_runs((first, second))

        duplicate_pages = (
            run_page(
                [{"id": 711, "display_title": "history-first"}],
                total_count=2,
                has_next=True,
            ),
            run_page([{"id": 711, "display_title": "history-duplicate"}], total_count=2),
        )
        with self.assertRaisesRegex(contract.ContractError, "duplicate run ID"):
            contract.collect_workflow_runs(duplicate_pages)


class UnicodeHistoryReadbackTests(unittest.TestCase):
    def setUp(self):
        self.fixture = readback_tests.readback_fixture()

    def _readback(self, api):
        with patch.object(
            readback,
            "_trusted_inventory",
            return_value=(self.fixture["context"], "6" * 40, "7" * 40),
        ):
            return readback.read_validation(api)

    def test_initial_and_final_full_enumerations_accept_unrelated_unicode_history(self):
        class API(readback_tests.FakeReadbackAPI):
            def __init__(self, fixture):
                super().__init__(fixture)
                self.unicode_history_pages = 0

            def workflow_run_pages(self, workflow_id, repository_id):
                pages = super().workflow_run_pages(workflow_id, repository_id)
                page = pages[0]
                rows = list(page["runs"])
                rows.insert(0, {"id": 799, "display_title": "nightly 日本語 🌱 — പഴയ run"})
                self.unicode_history_pages += 1
                return (run_page(rows),)

        api = API(self.fixture)
        envelope = self._readback(api)

        self.assertEqual(readback.contract.READBACK_SCHEMA, envelope["schema"])
        self.assertEqual(self.fixture["run"]["id"], envelope["run_id"])
        self.assertEqual(2, api.run_listing_count)
        self.assertEqual(2, api.unicode_history_pages)
        self.assertEqual(2, api.run_direct_count)
        self.assertEqual(0, api.write_calls)

    def test_initial_and_final_full_enumerations_accept_long_unicode_history(self):
        long_title = "🌱" * 1100
        self.assertGreater(len(long_title), 1024)
        self.assertGreater(len(long_title.encode("utf-8")), 4096)

        class API(readback_tests.FakeReadbackAPI):
            def __init__(self, fixture):
                super().__init__(fixture)
                self.long_unicode_history_pages = 0

            def workflow_run_pages(self, workflow_id, repository_id):
                pages = super().workflow_run_pages(workflow_id, repository_id)
                page = pages[0]
                rows = list(page["runs"])
                rows.insert(0, {"id": 795, "display_title": long_title})
                self.long_unicode_history_pages += 1
                return (run_page(rows),)

        api = API(self.fixture)
        envelope = self._readback(api)

        self.assertEqual(readback.contract.READBACK_SCHEMA, envelope["schema"])
        self.assertEqual(self.fixture["run"]["id"], envelope["run_id"])
        self.assertEqual(2, api.run_listing_count)
        self.assertEqual(2, api.long_unicode_history_pages)
        self.assertEqual(2, api.run_direct_count)
        self.assertEqual(0, api.write_calls)

    def test_final_enumeration_blocks_unicode_prefix_near_candidate(self):
        class API(readback_tests.FakeReadbackAPI):
            def workflow_run_pages(self, workflow_id, repository_id):
                pages = super().workflow_run_pages(workflow_id, repository_id)
                if self.run_listing_count == 2:
                    page = pages[0]
                    authority, title = authority_and_title(self.fixture)
                    validation_id = authority.validation_id
                    prefix = title[: -len(validation_id)]
                    near = {
                        "id": 800,
                        "display_title": prefix + "another-request-key-🌱",
                    }
                    return (run_page([*page["runs"], near]),)
                return pages

        api = API(self.fixture)
        with self.assertRaisesRegex(contract.ContractError, "unique workflow run ID"):
            self._readback(api)
        self.assertEqual(2, api.run_listing_count)
        self.assertEqual(1, api.run_direct_count)
        self.assertEqual(0, api.write_calls)

    def test_latest_attempt_advance_still_blocks_after_unicode_history_acceptance(self):
        class API(readback_tests.FakeReadbackAPI):
            def __init__(self, fixture):
                super().__init__(fixture)
                self.advance_second_attempt = True
                self.unicode_history_pages = 0

            def workflow_run_pages(self, workflow_id, repository_id):
                pages = super().workflow_run_pages(workflow_id, repository_id)
                page = pages[0]
                rows = list(page["runs"])
                rows.insert(0, {"id": 801, "display_title": "nightly 日本語 🌱 — run"})
                self.unicode_history_pages += 1
                return (run_page(rows),)

        api = API(self.fixture)
        with self.assertRaisesRegex(readback.ReadbackError, "latest R/A"):
            self._readback(api)
        self.assertEqual(2, api.run_listing_count)
        self.assertEqual(2, api.unicode_history_pages)
        self.assertEqual(2, api.run_direct_count)
        self.assertEqual(0, api.write_calls)


if __name__ == "__main__":
    unittest.main()
