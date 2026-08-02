#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HELPER="$ROOT_DIR/scripts/full-escalation-receipt.py"
WORKFLOW="$ROOT_DIR/.github/workflows/rust.yml"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

grep -Fq -- '- full_escalation' "$WORKFLOW"
grep -Fq "inputs.run_mode == 'full_escalation'" "$WORKFLOW"

HEAD="0123456789abcdef0123456789abcdef01234567"
COMMON=(
  --trigger workflow_dispatch
  --repository eng-cc/oasis7
  --run-id 42
  --run-attempt 3
  --actor human-operator
  --task-uid task_142b0428658248948da8910aa8b6a2f9
  --pr-number 2833
  --escalation-reason history_defect
  --evidence-url https://github.com/eng-cc/oasis7/issues/2832#issuecomment-123
  --ref refs/heads/task-2832
  --expected-head "$HEAD"
  --actual-head "$HEAD"
  --workflow-commit "$HEAD"
  --command 'CI_VERBOSE=1 ./scripts/ci-tests.sh full'
  --started-at 2026-08-02T00:00:00Z
  --finished-at 2026-08-02T00:01:00Z
  --conclusion success
)

python3 "$HELPER" receipt "${COMMON[@]}" --output "$TMP_DIR/receipt.json"
python3 - "$TMP_DIR/receipt.json" <<'PY'
import json, sys
receipt=json.load(open(sys.argv[1], encoding="utf-8"))
expected={
  "schema":"oasis7-full-escalation-receipt-v1",
  "repository":"eng-cc/oasis7",
  "run_id":42,
  "run_attempt":3,
  "actor":"human-operator",
  "task_uid":"task_142b0428658248948da8910aa8b6a2f9",
  "pr_number":2833,
  "escalation_reason":"history_defect",
  "evidence_url":"https://github.com/eng-cc/oasis7/issues/2832#issuecomment-123",
  "expected_head":"0123456789abcdef0123456789abcdef01234567",
  "actual_head":"0123456789abcdef0123456789abcdef01234567",
  "command":"CI_VERBOSE=1 ./scripts/ci-tests.sh full",
  "conclusion":"success",
}
for key, value in expected.items():
    if receipt.get(key) != value:
        raise SystemExit(f"receipt {key} mismatch: {receipt.get(key)!r}")
PY

if python3 "$HELPER" validate "${COMMON[@]}" --actual-head ffffffffffffffffffffffffffffffffffffffff >"$TMP_DIR/mismatch.out" 2>"$TMP_DIR/mismatch.err"; then
  echo "expected mismatched head to fail before full execution" >&2
  exit 1
fi
grep -F 'expected_head does not match actual_head' "$TMP_DIR/mismatch.err"

if python3 "$HELPER" validate "${COMMON[@]}" --trigger schedule >"$TMP_DIR/trigger.out" 2>"$TMP_DIR/trigger.err"; then
  echo "expected non-manual trigger to fail authorization validation" >&2
  exit 1
fi
grep -F 'trigger must be workflow_dispatch' "$TMP_DIR/trigger.err"

WORKFLOW="$ROOT_DIR/.github/workflows/rust.yml"
grep -Fq "if: github.event_name == 'schedule'" "$WORKFLOW"
grep -Fq "if: github.event_name == 'workflow_dispatch' && inputs.run_mode == 'full_regression'" "$WORKFLOW"
for input in task_uid pr_number expected_head escalation_reason evidence_url; do
  grep -Fq "      $input:" "$WORKFLOW"
done
grep -Fq 'steps.full.outcome' "$WORKFLOW"
grep -Fq 'name: oasis7-full-escalation-receipt-v1' "$WORKFLOW"

echo "full-escalation-receipt.test: OK"
