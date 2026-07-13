#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PR="${1:?usage: pr-watch-loop.sh <pr> [--task-uid task_uid]}"; shift
interval="${PM_PR_WATCH_INTERVAL_SECONDS:-60}"
max_interval="${PM_PR_WATCH_MAX_INTERVAL_SECONDS:-300}"
previous=""
while true; do
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
    previous="$digest"
    interval="${PM_PR_WATCH_INTERVAL_SECONDS:-60}"
  else
    interval=$(( interval * 2 > max_interval ? max_interval : interval * 2 ))
  fi
  sleep "$interval"
done
