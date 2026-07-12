#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

FIXTURE="$TMPDIR/fixture"
mkdir -p "$FIXTURE/scripts/pm" "$FIXTURE/.pm/github-project-sync"
cp "$ROOT_DIR/scripts/pm/task-closeout.sh" "$FIXTURE/scripts/pm/task-closeout.sh"
chmod +x "$FIXTURE/scripts/pm/task-closeout.sh"
cat >"$FIXTURE/scripts/pm/github-project-workflow.sh" <<'EOF'
#!/usr/bin/env bash
printf '{"status":"ok","errors":[],"warnings":[]}'
EOF
cat >"$FIXTURE/scripts/pm/claim-ready.sh" <<'EOF'
#!/usr/bin/env bash
printf '{"claim_type":"task_complete","status":"verified","allowed_to_claim":true,"verification_exit_code":0}'
EOF
cat >"$FIXTURE/scripts/pm/github-project-task.py" <<'EOF'
#!/usr/bin/env python3
import json, os
from pathlib import Path
Path(os.environ["MUTATION_LOG"]).write_text("mutated\n", encoding="utf-8")
print(json.dumps({"task_uid":"task_11111111111111111111111111111111","status":"done","issue_url":"https://example.invalid/1"}))
EOF
chmod +x "$FIXTURE/scripts/pm/github-project-workflow.sh" "$FIXTURE/scripts/pm/claim-ready.sh" "$FIXTURE/scripts/pm/github-project-task.py"
cat >"$FIXTURE/.pm/github-project-sync/tasks.json" <<'EOF'
{"version":1,"tasks":{"task_11111111111111111111111111111111":{"task_uid":"task_11111111111111111111111111111111","completion_mode":"non_pr_task","non_pr_completion_evidence":"persisted fixture truth"}}}
EOF

assert_rejected_without_profile() {
  local command="$1"
  local slug="$2"
  if MUTATION_LOG="$TMPDIR/$slug.mutation" PM_ROOT_DIR="$FIXTURE" \
    "$FIXTURE/scripts/pm/task-closeout.sh" --role tpm \
      --task-uid task_11111111111111111111111111111111 \
      --to-status done --claim-type task_complete \
      --verify-command "$command" --json \
      >"$TMPDIR/$slug.out" 2>"$TMPDIR/$slug.err"; then
    echo "expected closeout verification '$command' without a named repository profile to fail" >&2
    exit 1
  fi
  [[ ! -e "$TMPDIR/$slug.mutation" ]]
  grep -Eiq 'verification.profile|named.*profile|codex_subagent_role_fit|workflow_behavior|vacuous' "$TMPDIR/$slug.err"
}

assert_rejected_without_profile 'exit 0' exit_zero
assert_rejected_without_profile 'echo ok' echo_ok
assert_rejected_without_profile './scripts/verify-something.sh' unnamed_repo_command

MUTATION_LOG="$TMPDIR/profile.mutation" PM_ROOT_DIR="$FIXTURE" \
  "$FIXTURE/scripts/pm/task-closeout.sh" --role tpm \
    --task-uid task_11111111111111111111111111111111 \
    --to-status done --claim-type task_complete \
    --verification-profile codex_subagent_role_fit \
    --json >"$TMPDIR/profile.out"
test -f "$TMPDIR/profile.mutation"

echo "task-closeout-profile.test: OK"
