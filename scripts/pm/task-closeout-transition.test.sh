#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

FIXTURE="$TMPDIR/fixture"
mkdir -p "$FIXTURE/scripts/pm"
cp "$ROOT_DIR/scripts/pm/task-closeout.sh" "$FIXTURE/scripts/pm/task-closeout.sh"
chmod +x "$FIXTURE/scripts/pm/task-closeout.sh"

cat >"$FIXTURE/scripts/pm/github-project-workflow.sh" <<'EOF'
#!/usr/bin/env bash
printf '{"status":"ok","errors":[],"warnings":[]}'
EOF
cat >"$FIXTURE/scripts/pm/claim-ready.sh" <<'EOF'
#!/usr/bin/env bash
printf '{"claim_type":"fixture","status":"verified","allowed_to_claim":true,"verification_exit_code":0}'
EOF
cat >"$FIXTURE/scripts/pm/github-project-task.py" <<'EOF'
#!/usr/bin/env python3
import json
import os
from pathlib import Path

Path(os.environ["MUTATION_LOG"]).write_text("remote closeout attempted\n", encoding="utf-8")
print(json.dumps({
    "task_uid": "task_11111111111111111111111111111111",
    "status": "ready",
    "issue_url": "https://example.invalid/issues/1",
}))
EOF
chmod +x "$FIXTURE/scripts/pm/github-project-workflow.sh" \
  "$FIXTURE/scripts/pm/claim-ready.sh" "$FIXTURE/scripts/pm/github-project-task.py"

assert_vacuous_transition_rejected() {
  local target_status="$1"
  local claim_type="$2"
  local verify_command="${3:-true}"
  local slug="$(printf '%s' "$verify_command" | tr -cd '[:alnum:]')"
  local stderr_file="$TMPDIR/${target_status}-${slug}.err"
  local mutation_log="$TMPDIR/${target_status}-${slug}.mutation"

  if MUTATION_LOG="$mutation_log" PM_ROOT_DIR="$FIXTURE" \
    "$FIXTURE/scripts/pm/task-closeout.sh" \
      --role tpm \
      --task-uid task_11111111111111111111111111111111 \
      --to-status "$target_status" \
      --claim-type "$claim_type" \
      --verify-command "$verify_command" \
      --json >"$TMPDIR/${target_status}.out" 2>"$stderr_file"; then
    echo "expected $target_status closeout with arbitrary true verification to fail" >&2
    exit 1
  fi

  if [[ -e "$mutation_log" ]]; then
    echo "vacuous $target_status verification must fail before remote closeout mutation" >&2
    exit 1
  fi
  grep -Eiq 'transition-specific|verification.*profile|vacuous|arbitrary.*command|review packet|merged PR|non-PR' "$stderr_file"
}

assert_vacuous_transition_rejected ready ready_for_pr
assert_vacuous_transition_rejected ready ready_for_pr "exit 0"
assert_vacuous_transition_rejected ready ready_for_pr "echo ok"
assert_vacuous_transition_rejected done task_complete

echo "task-closeout-transition.test: OK"
