#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
T="$(mktemp -d)"; trap 'rm -rf "$T"' EXIT
F="$T/repo"; mkdir -p "$F/scripts/pm" "$F/.pm/github-project-sync"
cp "$ROOT/scripts/pm/task-closeout.sh" "$F/scripts/pm/"
git -C "$F" init -q; git -C "$F" config user.email f@x; git -C "$F" config user.name f
touch "$F/tracked"; git -C "$F" add tracked; git -C "$F" commit -qm init
HEAD=$(git -C "$F" rev-parse HEAD)
printf '{}\n' >"$F/slice-ledger.jsonl"; printf '{}\n' >"$F/receipt.json"
printf 'Pre-PR Local Role Review: passed\n- Source Head: %s\n- Review Roles: qa_engineer\n- Slice Ledger: ./slice-ledger.jsonl\n' "$HEAD" >"$F/review.md"
printf '{"version":1,"tasks":{"task_11111111111111111111111111111111":{"completion_mode":"non_pr_task","non_pr_completion_evidence":"fixture"}}}\n' >"$F/.pm/github-project-sync/tasks.json"
cp "$F/review.md" "$T/review.base"; cp "$F/slice-ledger.jsonl" "$T/ledger.base"; cp "$F/receipt.json" "$T/receipt.base"; cp "$F/.pm/github-project-sync/tasks.json" "$T/mapping.base"
cat >"$F/scripts/pm/validate-review-provenance.py" <<'EOF'
#!/usr/bin/env python3
raise SystemExit(0)
EOF
cat >"$F/scripts/pm/github-project-workflow.sh" <<'EOF'
#!/usr/bin/env bash
echo audit >>"$EVENTS"; printf '{"status":"ok"}'
EOF
cat >"$F/scripts/pm/claim-ready.sh" <<'EOF'
#!/usr/bin/env bash
echo claim >>"$EVENTS"
case "${MUTATE_KIND:-}" in
 head) echo x >>tracked; git add tracked; git commit -qm mutate;;
 mapping) echo x >>.pm/github-project-sync/tasks.json;;
 packet) echo x >>"$PACKET";;
 ledger) echo x >>"$LEDGER";;
 receipt) echo x >>"$RECEIPT";;
esac
printf '{"claim_type":"fixture","status":"verified","allowed_to_claim":true,"verification_exit_code":0}'
EOF
cat >"$F/scripts/pm/github-project-task.py" <<'EOF'
#!/usr/bin/env python3
import json,os
open(os.environ['EVENTS'],'a').write('transition\n')
print(json.dumps({'task_uid':'task_11111111111111111111111111111111','status':'done','issue_url':'x'}))
EOF
chmod +x "$F/scripts/pm/"*

cat >"$T/outside-review-plan.json" <<JSON
{"schema":"oasis7-review-plan/v1","task_uid":"task_11111111111111111111111111111111","frozen_head":"$HEAD","roles":["qa_engineer"],"preflight":{"ledger_path":"slice-ledger.jsonl"}}
JSON
ln -s "$T/outside-review-plan.json" "$F/escaping-review-plan.json"

assert_review_plan_path_rejected() {
  local plan_ref="$1" slug="$2"
  printf 'Pre-PR Local Role Review: passed\n- Source Head: %s\n- Review Plan: %s\n- Review Roles: stale_compatibility_role\n- Slice Ledger: n/a; derived from Review Plan\n' \
    "$HEAD" "$plan_ref" >"$F/review-escape.md"
  : >"$T/events"
  set +e
  (cd "$F"; EVENTS="$T/events" PM_ROOT_DIR="$F" \
    ./scripts/pm/task-closeout.sh --role tpm \
      --task-uid task_11111111111111111111111111111111 \
      --to-status ready --claim-type ready_for_pr \
      --verification-profile fixture_repository_state \
      --review-packet-file "$F/review-escape.md" --json >/dev/null 2>"$T/$slug.err")
  local rc=$?
  set -e
  [[ "$rc" != "0" ]] || { echo "expected escaping review plan to fail: $plan_ref" >&2; exit 1; }
  [[ ! -s "$T/events" ]] || { echo "escaping review plan reached lifecycle mutation: $plan_ref" >&2; exit 1; }
  grep -qi "escapes repository root" "$T/$slug.err"
}

assert_review_plan_path_rejected ../outside-review-plan.json traversal
assert_review_plan_path_rejected escaping-review-plan.json symlink

run_case() {
  local status=$1 kind=$2; : >"$T/events"; local args=()
  git -C "$F" reset --hard -q "$HEAD"
  cp "$T/review.base" "$F/review.md"; cp "$T/ledger.base" "$F/slice-ledger.jsonl"; cp "$T/receipt.base" "$F/receipt.json"; cp "$T/mapping.base" "$F/.pm/github-project-sync/tasks.json"
  if [[ "$status" == ready ]]; then args=(--review-packet-file "$F/review.md"); else args=(--pr-receipt "$F/receipt.json"); fi
  set +e
  (cd "$F"; EVENTS="$T/events" MUTATE_KIND="$kind" PACKET="$F/review.md" LEDGER="$F/slice-ledger.jsonl" RECEIPT="$F/receipt.json" PM_ROOT_DIR="$F" \
    ./scripts/pm/task-closeout.sh --role tpm --task-uid task_11111111111111111111111111111111 \
    --to-status "$status" --claim-type "$([[ "$status" == ready ]] && echo ready_for_pr || echo task_complete)" \
    --verification-profile fixture_repository_state "${args[@]}" --json >/dev/null 2>"$T/err")
  local rc=$?; set -e
  if [[ -z "$kind" ]]; then [[ $rc == 0 ]] || { cat "$T/err"; return 1; }; diff -u <(printf 'claim\naudit\ntransition\naudit\n') "$T/events"
  else [[ $rc != 0 ]]; diff -u <(printf 'claim\n') "$T/events"; fi
}
run_case ready ""; run_case done ""
for kind in head mapping packet ledger; do
  run_case ready "$kind"
done
run_case done receipt

# Minimal `{status:ok}` compatibility must be explicitly gated to the fixture
# profile; every live profile must require task_uid, target status, and phase.
python3 - "$ROOT/scripts/pm/task-closeout.sh" <<'PY'
import re,sys
source=open(sys.argv[1],encoding='utf-8').read()
block=re.search(r"POSTCONDITION_AUDIT_JSON=.*?\npython3 - .*?<<'PY'\n(.*?)\nPY", source, re.S)
assert block, "missing postcondition validator"
validator=block.group(1)
assert "VERIFICATION_PROFILE" in validator and "fixture_repository_state" in validator, \
    "minimal postcondition audit compatibility must be fixture-profile-only"
for field in ("task_uid", "target", "workflow_phase"):
    assert field in validator, f"live postcondition validator must require structured {field}"
PY
echo "task-closeout-audit-order.test: OK"
