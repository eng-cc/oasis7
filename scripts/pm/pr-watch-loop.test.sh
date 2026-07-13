#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/bin"
cat >"$TMP/bin/python3" <<'SH'
#!/usr/bin/env bash
if [[ "${1:-}" == *pr-lifecycle-gate.py ]]; then
  n=$(cat "$TEST_COUNT" 2>/dev/null || echo 0); n=$((n+1)); echo "$n" >"$TEST_COUNT"
  state=blocked
  [[ "${TEST_NEVER_READY:-0}" != 1 && "$n" -ge 6 ]] && state=ready
  printf '{"status":"%s","gate_epoch":"epoch-%s","readiness_receipt":{"observed_at":"2026-01-01T00:00:0%sZ","identity":"receipt-%s"},"nested":{"approval_only_receipt":{"identity":"nested-%s"}}}\n' "$state" "$n" "$n" "$n" "$n"
  exit 0
fi
exec /usr/bin/python3 "$@"
SH
cat >"$TMP/bin/sleep" <<'SH'
#!/usr/bin/env bash
echo "$1" >>"$TEST_SLEEPS"
SH
chmod +x "$TMP/bin/python3" "$TMP/bin/sleep"

export TEST_COUNT="$TMP/count" TEST_SLEEPS="$TMP/sleeps"
PATH="$TMP/bin:$PATH" PM_PR_WATCH_INTERVAL_SECONDS=60 PM_PR_WATCH_MAX_INTERVAL_SECONDS=600 \
  bash "$ROOT/scripts/pm/pr-watch-loop.sh" 1 >"$TMP/out"
diff -u <(printf '60\n120\n240\n480\n600\n') "$TMP/sleeps"
[[ $(wc -l <"$TMP/out" | tr -d ' ') == 2 ]]
grep -q '"status":"blocked"' "$TMP/out"
grep -q '"status":"ready"' "$TMP/out"

rm -f "$TMP/count" "$TMP/sleeps"
set +e
PATH="$TMP/bin:$PATH" TEST_NEVER_READY=1 PM_PR_WATCH_MAX_POLLS=3 \
  bash "$ROOT/scripts/pm/pr-watch-loop.sh" 1 >"$TMP/stable-out" 2>"$TMP/stable-err"
rc=$?
set -e
[[ "$rc" == 75 ]]
diff -u <(printf '60\n120\n') "$TMP/sleeps"
[[ $(wc -l <"$TMP/stable-out" | tr -d ' ') == 1 ]]
grep -q '"reason":"stable_pr_watch_bound_exhausted"' "$TMP/stable-err"

set +e
PATH="$TMP/bin:$PATH" PM_PR_WATCH_INTERVAL_SECONDS=0 \
  bash "$ROOT/scripts/pm/pr-watch-loop.sh" 1 >"$TMP/invalid-out" 2>"$TMP/invalid-err"
rc=$?
set -e
[[ "$rc" == 64 ]]
grep -q 'interval must be a positive integer' "$TMP/invalid-err"

echo "pr-watch-loop.test: OK"
