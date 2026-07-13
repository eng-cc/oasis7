#!/usr/bin/env python3
"""RED contracts for bounded GitHub GraphQL use in hot PM paths."""

from __future__ import annotations

import ast
import pathlib
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]


def function_source(path: pathlib.Path, function_name: str) -> str:
    source = path.read_text(encoding="utf-8")
    tree = ast.parse(source)
    for node in tree.body:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) and node.name == function_name:
            return ast.get_source_segment(source, node) or ""
    raise AssertionError(f"missing function {function_name} in {path}")


class GraphqlBudgetContracts(unittest.TestCase):
    def test_one_task_refresh_uses_one_project_graphql_read(self) -> None:
        body = function_source(ROOT / "scripts/pm/github-project-task.py", "command_refresh_task")
        project_reads = body.count("recover_project_mapping(") + body.count('"gh", "api", "graphql"')
        self.assertLessEqual(
            project_reads,
            1,
            "one-task refresh must reconcile issue and Project fields with at most one GraphQL read",
        )

    def test_pr_watch_uses_one_batched_graphql_read_per_poll(self) -> None:
        body = function_source(ROOT / "scripts/pm/pr-lifecycle-gate.py", "load_live")
        self.assertLessEqual(
            body.count("graphql_pages("),
            1,
            "one PR-watch poll must batch comments, reviews, threads, and checks into one GraphQL read",
        )

    def test_terminal_closeout_runs_selected_task_audit_once(self) -> None:
        body = (ROOT / "scripts/pm/task-closeout.sh").read_text(encoding="utf-8")
        audit_calls = body.count('github-project-workflow.sh" --json audit --task-uid')
        self.assertLessEqual(
            audit_calls,
            1,
            "terminal closeout must reuse one selected-task audit instead of repeating its GraphQL read",
        )


if __name__ == "__main__":
    unittest.main()
