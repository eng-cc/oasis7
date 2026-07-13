#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PR="${1:?usage: pr-watch-loop.sh <pr> [--task-uid task_uid]}"; shift
interval="${PM_PR_WATCH_INTERVAL_SECONDS:-60}"
max_interval="${PM_PR_WATCH_MAX_INTERVAL_SECONDS:-600}"
max_polls="${PM_PR_WATCH_MAX_POLLS:-6}"
for value_name in interval max_interval max_polls; do
  value="${!value_name}"
  if [[ ! "$value" =~ ^[1-9][0-9]*$ ]]; then
    printf 'pr-watch-loop: %s must be a positive integer, got %q\n' "$value_name" "$value" >&2
    exit 64
  fi
done
if (( interval > max_interval )); then
  printf 'pr-watch-loop: interval must not exceed max_interval\n' >&2
  exit 64
fi
previous=""
for ((poll = 1; poll <= max_polls; poll++)); do
  snapshot="$(python3 "$SCRIPT_DIR/pr-lifecycle-gate.py" "$PR" "$@" --json)"
  digest="$(printf '%s' "$snapshot" | python3 -c '
import hashlib,json,sys
volatile={"observed_at","verified_at","cache_refreshed_at","last_synced_at","gate_epoch"}
def stable(value):
    if isinstance(value,dict): return {k:stable(v) for k,v in value.items() if k not in volatile and not k.endswith("_receipt")}
    if isinstance(value,list): return [stable(v) for v in value]
    return value
payload=stable(json.load(sys.stdin))
print(hashlib.sha256(json.dumps(payload,sort_keys=True,separators=(",",":")).encode()).hexdigest())
')"
  if [[ "$digest" != "$previous" ]]; then
    printf '%s\n' "$snapshot"
    if [[ -n "$previous" ]]; then
      exit 0
    fi
    previous="$digest"
  fi
  if (( poll == max_polls )); then
    printf '{"status":"external_wait","reason":"stable_pr_watch_bound_exhausted","resume_after_seconds":%s,"polls":%s}\n' "$interval" "$max_polls" >&2
    exit 75
  fi
  sleep "$interval"
  interval=$(( interval * 2 > max_interval ? max_interval : interval * 2 ))
done
