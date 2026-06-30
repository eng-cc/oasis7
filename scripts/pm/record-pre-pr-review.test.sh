#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

TMPDIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

TEST_REPO="$TMPDIR/repo"
mkdir -p "$TEST_REPO/scripts/pm"
cp "$ROOT_DIR/scripts/pm/record-pre-pr-review.sh" "$TEST_REPO/scripts/pm/record-pre-pr-review.sh"
chmod +x "$TEST_REPO/scripts/pm/record-pre-pr-review.sh"

git -C "$TEST_REPO" init -q -b main
printf 'base\n' > "$TEST_REPO/README.md"
git -C "$TEST_REPO" add README.md scripts/pm/record-pre-pr-review.sh
git -C "$TEST_REPO" -c user.name="oasis7 smoke" -c user.email="smoke@example.invalid" commit -q -m "base"
git -C "$TEST_REPO" branch base

printf 'changed\n' >> "$TEST_REPO/README.md"
git -C "$TEST_REPO" add README.md
git -C "$TEST_REPO" -c user.name="oasis7 smoke" -c user.email="smoke@example.invalid" commit -q -m "change"

if "$TEST_REPO/scripts/pm/record-pre-pr-review.sh" \
  --task-uid task_11111111111111111111111111111111 \
  --roles repository_health_engineer \
  --verification "helper -> smoke -> observed" \
  --residual-risk "fixture risk" \
  --comparison-ref refs/heads/base \
  --print-only >"$TMPDIR/missing.out" 2>"$TMPDIR/missing.err"; then
  echo "expected missing review evidence to fail" >&2
  exit 1
fi
grep -q -- "--review-evidence is required" "$TMPDIR/missing.err"

printf 'dirty\n' >> "$TEST_REPO/README.md"
if "$TEST_REPO/scripts/pm/record-pre-pr-review.sh" \
  --task-uid task_11111111111111111111111111111111 \
  --roles repository_health_engineer \
  --review-evidence "repository_health_engineer: no_findings; smoke" \
  --review-verdicts "repository_health_engineer scope/spec compliance=approved; role quality/risk=approved" \
  --finding-disposition-evidence "smoke evidence" \
  --verification "helper -> smoke -> observed" \
  --residual-risk "fixture risk" \
  --comparison-ref refs/heads/base \
  --print-only >"$TMPDIR/dirty.out" 2>"$TMPDIR/dirty.err"; then
  echo "expected dirty worktree to fail" >&2
  exit 1
fi
grep -q "working tree is dirty" "$TMPDIR/dirty.err"
git -C "$TEST_REPO" checkout -- README.md

"$TEST_REPO/scripts/pm/record-pre-pr-review.sh" \
  --task-uid task_11111111111111111111111111111111 \
  --roles repository_health_engineer \
  --review-evidence "repository_health_engineer: no_findings; smoke" \
  --review-verdicts "repository_health_engineer scope/spec compliance=approved; role quality/risk=approved" \
  --finding-disposition-evidence "smoke evidence" \
  --verification "helper -> smoke -> observed" \
  --residual-risk "fixture risk" \
  --visual-evidence "screenshot/model review: smoke visual evidence" \
  --ops-evidence "readiness/rollback/runbook/operator evidence: smoke ops evidence" \
  --liveops-evidence "messaging/release-note/player/community evidence: smoke liveops evidence" \
  --comparison-ref refs/heads/base \
  --print-only >"$TMPDIR/packet.out"

grep -q "Pre-PR Local Role Review: passed" "$TMPDIR/packet.out"
grep -q "Reviewed Changed Paths: README.md" "$TMPDIR/packet.out"
grep -q "Finding Disposition Evidence: smoke evidence" "$TMPDIR/packet.out"
grep -q "Visual Evidence: screenshot/model review: smoke visual evidence" "$TMPDIR/packet.out"
grep -q "Ops Evidence: readiness/rollback/runbook/operator evidence: smoke ops evidence" "$TMPDIR/packet.out"
grep -q "LiveOps Evidence: messaging/release-note/player/community evidence: smoke liveops evidence" "$TMPDIR/packet.out"

echo "record-pre-pr-review.test: OK"
