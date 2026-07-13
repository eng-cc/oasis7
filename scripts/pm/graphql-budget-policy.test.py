#!/usr/bin/env python3
from pathlib import Path
root = Path(__file__).resolve().parents[2]
workflow = (root / "scripts/pm/github-project-workflow.py").read_text()
watch = (root / "scripts/pm/pr-watch-loop.sh").read_text()
audit = (root / "scripts/pm/audit-pr-watch-issues.py").read_text()
sync = (root / "scripts/pm/github-project-sync.py").read_text()
assert "rateLimit { remaining resetAt }" in workflow
assert "graphql_budget_insufficient" in workflow and "resumable" in workflow
assert 'PM_PR_WATCH_INTERVAL_SECONDS:-60' in watch
assert 'PM_PR_WATCH_MAX_INTERVAL_SECONDS:-300' in watch
assert 'parser.add_argument("--task-uid"' in audit
assert '_PROJECT_CONTEXT_CACHE' in sync
assert ':unchanged' in sync
print("graphql-budget-policy.test: OK")
