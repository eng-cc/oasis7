#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"; TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/bin" "$TMP/root/.pm/github-project-sync" "$TMP/root/.pm/tasks"
mkdir -p "$TMP/root/scripts/pm"
cp "$ROOT/scripts/pm/github-project-task.py" "$ROOT/scripts/pm/github-project-sync.py" "$ROOT/scripts/pm/github-project-workflow.py" "$ROOT/scripts/pm/workflow-durable-store.py" "$TMP/root/scripts/pm/"
printf '{"version":1,"project":{"owner":"eng-cc","number":1,"repo":"eng-cc/oasis7"},"tasks":{}}\n' >"$TMP/root/.pm/github-project-sync/tasks.json"
cat >"$TMP/bin/gh" <<'SH'
#!/usr/bin/env bash
echo "$*" >>"$CALL_LOG"
if [[ "$*" == api\ graphql* ]]; then
 case "$RATE_MODE" in
  unavailable) exit 9;;
  unknown) printf '{"data":{"rateLimit":{"remaining":null,"resetAt":""}}}\n';;
  low) printf '{"data":{"rateLimit":{"remaining":99,"resetAt":"2099-01-01T00:00:00Z"}}}\n';;
  ok) printf '{"data":{"rateLimit":{"remaining":5000,"resetAt":"2099-01-01T00:00:00Z"}}}\n';;
 esac; exit 0
fi
if [[ "$*" == issue\ list* ]]; then printf '[]\n'; exit 0; fi
if [[ "$*" == "project view 1 --owner eng-cc --format json" ]]; then printf '{"id":"PROJECT_ID"}\n'; exit 0; fi
if [[ "$*" == "project field-list 1 --owner eng-cc --format json" ]]; then printf '{"fields":[]}\n'; exit 0; fi
echo "unexpected: $*" >&2; exit 8
SH
chmod +x "$TMP/bin/gh"; export PATH="$TMP/bin:$PATH" CALL_LOG="$TMP/calls"
for tool in audit sync; do
 : >"$CALL_LOG"; set +e
 if [[ "$tool" == audit ]]; then python3 "$ROOT/scripts/pm/audit-pr-watch-issues.py" "$TMP/root" --json >"$TMP/default" 2>&1
 else python3 "$ROOT/scripts/pm/github-project-sync.py" "$TMP/root" --repo eng-cc/oasis7 --project-owner eng-cc --project-number 1 --dry-run --json >"$TMP/default" 2>&1; fi
 rc=$?; set -e; [[ $rc -ne 0 ]]; grep -q 'task-uid.*required' "$TMP/default"
 for RATE_MODE in unavailable unknown low ok; do
  export RATE_MODE; : >"$CALL_LOG"; set +e
  if [[ "$tool" == audit ]]; then python3 "$ROOT/scripts/pm/audit-pr-watch-issues.py" "$TMP/root" --global-maintenance --json >"$TMP/out"
  else python3 "$ROOT/scripts/pm/github-project-sync.py" "$TMP/root" --repo eng-cc/oasis7 --project-owner eng-cc --project-number 1 --global-maintenance --dry-run --json >"$TMP/out"; fi
  rc=$?; set -e
  if [[ "$RATE_MODE" == ok ]]; then [[ $rc == 0 ]]; grep -q '"status": "ok"\|"selected_count": 0' "$TMP/out"
  else [[ $rc == 2 ]]; grep -q '"resumable": true' "$TMP/out"; ! grep -Eq 'issue list|project view' "$CALL_LOG"; fi
 done
done

# The workflow sync wrapper must preserve the same fail-closed contract while
# delegating the one live budget preflight to the shared sync child.
: >"$CALL_LOG"; set +e
python3 "$TMP/root/scripts/pm/github-project-workflow.py" "$TMP/root" --json sync >"$TMP/wrapper-default" 2>&1
rc=$?; set -e
if [[ $rc == 0 ]] || ! grep -q 'sync requires --task-uid by default' "$TMP/wrapper-default" || [[ -s "$CALL_LOG" ]]; then
 echo "wrapper default scope guard failed" >&2; exit 1
fi
for RATE_MODE in unavailable unknown low ok; do
 export RATE_MODE; : >"$CALL_LOG"; set +e
 python3 "$TMP/root/scripts/pm/github-project-workflow.py" "$TMP/root" --json sync --global-maintenance >"$TMP/wrapper-out" 2>&1
 rc=$?; set -e
 rate_calls="$(grep -c 'api graphql' "$CALL_LOG" || true)"
 if [[ "$RATE_MODE" == unavailable ]]; then
  # The child may retry transport failures, but the wrapper must not add a
  # second preflight layer or reach broad work.
  [[ "$rate_calls" -ge 1 ]] || { echo "wrapper skipped child preflight" >&2; exit 1; }
 elif [[ "$rate_calls" != 1 ]]; then
  echo "wrapper must delegate exactly one rate-limit preflight for $RATE_MODE" >&2; exit 1
 fi
 if [[ "$RATE_MODE" == ok ]]; then
  if [[ $rc != 0 ]] || ! grep -q 'project view 1 --owner eng-cc --format json' "$CALL_LOG"; then
   echo "wrapper did not reach broad child for ok budget" >&2; exit 1
  fi
 else
  if [[ $rc != 2 ]] || ! grep -q '"resumable": true' "$TMP/wrapper-out" || grep -Eq 'issue list|project view' "$CALL_LOG"; then
   echo "wrapper failed to block broad work for $RATE_MODE" >&2; exit 1
  fi
 fi
done
echo "graphql-global-maintenance-command.test: OK"
