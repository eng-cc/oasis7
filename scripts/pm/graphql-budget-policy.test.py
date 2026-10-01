#!/usr/bin/env python3
import importlib.util
from pathlib import Path
import re
root = Path(__file__).resolve().parents[2]
workflow = (root / "scripts/pm/github-project-workflow.py").read_text()
api = (root / "scripts/pm/github_api.py").read_text()
gate = (root / "scripts/pm/pr-lifecycle-gate.py").read_text()
observation_path = root / "scripts/pm/github_observation.py"
observation_spec = importlib.util.spec_from_file_location("github_observation_budget_policy_test", observation_path)
observation = importlib.util.module_from_spec(observation_spec)
observation_spec.loader.exec_module(observation)
audit = (root / "scripts/pm/audit-pr-watch-issues.py").read_text()
sync = (root / "scripts/pm/github-project-sync.py").read_text()
assert "load_sync_module().broad_rate_limit_guard()" in workflow
assert "client.rate_limit_guard(" in sync
budget_query = re.search(
    r"query RateLimitBudget\s*\{[^}]*rateLimit\s*\{([^}]*)\}", api,
)
assert budget_query is not None, "shared client no longer issues the bounded GraphQL budget query"
budget_fields = set(budget_query.group(1).split())
assert {"remaining", "resetAt"} <= budget_fields, budget_fields
assert "graphql_budget_insufficient" in api and "resumable" in api
assert (observation.DEFAULT_MIN_INTERVAL, observation.DEFAULT_MAX_INTERVAL,
        observation.DEFAULT_MAX_POLLS, observation.DEFAULT_MAX_UNCHANGED_POLLS) == (60, 600, 6, 1)
assert 'observation_module.watch(' in gate
assert all(name in gate for name in (
    'PM_PR_WATCH_INTERVAL_SECONDS', 'PM_PR_WATCH_MAX_INTERVAL_SECONDS',
    'PM_PR_WATCH_MAX_POLLS', 'PM_PR_WATCH_MAX_UNCHANGED_POLLS',
))
observation_source = observation_path.read_text()
assert 'stable_pr_watch_bound_exhausted' in observation_source
assert 'stable_pr_watch_unchanged_budget_exhausted' in observation_source
assert 'parser.add_argument("--task-uid"' in audit
assert '_PROJECT_CONTEXT_CACHE' in sync
assert ':unchanged' in sync
print("graphql-budget-policy.test: OK")
