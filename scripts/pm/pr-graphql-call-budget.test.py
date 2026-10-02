#!/usr/bin/env python3
"""Auxiliary source guard; runtime PR budget evidence lives in graphql-budget-red.test."""

from __future__ import annotations

import ast
import pathlib
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
GATE_PATH = ROOT / "scripts/pm/pr-lifecycle-gate.py"


def function_source(name: str) -> str:
    source = GATE_PATH.read_text(encoding="utf-8")
    tree = ast.parse(source)
    for node in tree.body:
        if isinstance(node, ast.FunctionDef) and node.name == name:
            return ast.get_source_segment(source, node) or ""
    raise AssertionError(f"missing {name}")


class PRHotPathSourceContracts(unittest.TestCase):
    def test_load_live_uses_effective_strict_snapshot_without_cli_or_legacy_graphql(self):
        body = function_source("load_live")
        self.assertIn("_load_effective_helper(effective, \"github_pr_snapshot\")", body)
        self.assertIn("fetch_pr_snapshot", body)
        self.assertIn("fresh=True", body)
        self.assertNotIn("subprocess", body)
        self.assertNotIn("graphql_pages", body)
        self.assertNotIn("graphql_pr_snapshot", body)

    def test_final_identity_uses_separate_uncached_snapshot_adapter(self):
        body = function_source("read_pr_identity")
        self.assertIn("fetch_pr_identity", body)
        self.assertNotIn("subprocess", body)
        self.assertNotIn("load_live(", body)

    def test_cli_view_and_raw_graphql_bypasses_are_absent(self):
        source = GATE_PATH.read_text(encoding="utf-8")
        self.assertNotIn("gh pr view", source)
        self.assertNotIn("gh repo view", source)
        self.assertNotIn('"gh", "api", "graphql"', source)


if __name__ == "__main__":
    unittest.main(verbosity=2)
