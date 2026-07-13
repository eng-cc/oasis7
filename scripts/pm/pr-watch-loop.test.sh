#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/bin"
cat >"$TMP/bin/python3" <<'SH'
#!/usr/bin/env bash
if [[ "${1:-}" == *pr-lifecycle-gate.py ]]; then
  n=$(cat "$TEST_COUNT" 2>/dev/null || echo 0); n=$((n+1)); echo "$n" >"$TEST_COUNT"
  state=blocked; [[ "$n" -ge 5 ]] && state=ready
  printf '{"status":"%s","gate_epoch":"epoch-%s","readiness_receipt":{"observed_at":"2026-01-01T00:00:0%sZ","identity":"receipt-%s"},"nested":{"approval_only_receipt":{"identity":"nested-%s"}}}\n' "$state" "$n" "$n" "$n" "$n"
  exit 0
fi
exec /usr/bin/python3 "$@"
SH
cat >"$TMP/bin/sleep" <<'SH'
#!/usr/bin/env bash
echo "$1" >>"$TEST_SLEEPS"
[[ $(wc -l <"$TEST_SLEEPS") -ge 5 ]] && exit 19
exit 0
SH
chmod +x "$TMP/bin/python3" "$TMP/bin/sleep"
export TEST_COUNT="$TMP/count" TEST_SLEEPS="$TMP/sleeps"
set +e
PATH="$TMP/bin:$PATH" PM_PR_WATCH_INTERVAL_SECONDS=60 PM_PR_WATCH_MAX_INTERVAL_SECONDS=300 \
  bash "$ROOT/scripts/pm/pr-watch-loop.sh" 1 >"$TMP/out"
rc=$?
set -e
[[ "$rc" == 19 ]]
diff -u <(printf '60\n120\n240\n300\n60\n') "$TMP/sleeps"
[[ $(wc -l <"$TMP/out" | tr -d ' ') == 2 ]]
grep -q '"status":"blocked"' "$TMP/out"
grep -q '"status":"ready"' "$TMP/out"
echo "pr-watch-loop.test: OK"
