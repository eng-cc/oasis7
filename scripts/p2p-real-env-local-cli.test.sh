#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
failures=0

# --help terminates inside the real parser before sampling, filesystem writes,
# curl, systemctl, or SSH. Accepted local values must reach that boundary;
# retired aliases must fail before it, with the normal unknown-option error.
check_flag() {
  local script=$1 local_flag=$2 old_flag=$3 status
  bash "$repo_root/scripts/$script" "$local_flag" fixture-value --help >"$tmp_root/local" 2>&1
  if bash "$repo_root/scripts/$script" "$old_flag" fixture-value --help >"$tmp_root/old" 2>&1; then
    status=0
  else
    status=$?
  fi
  if [[ $status != 2 ]] || ! grep -Fq "unknown option: $old_flag" "$tmp_root/old"; then
    echo "FAIL $script $old_flag: expected unknown-option exit 2, got $status" >&2
    failures=$((failures + 1))
  fi
}

check_flag p2p-real-env-host-monitor.sh --local-service --observer-service
check_flag p2p-real-env-host-monitor.sh --local-storage-path --observer-storage-path
check_flag p2p-real-env-traffic-monitor.sh --local-status-url --observer-status-url
check_flag p2p-real-env-triad-snapshot.sh --local-service --observer-service
check_flag p2p-real-env-triad-snapshot.sh --local-status-url --observer-status-url
check_flag p2p-real-env-triad-snapshot.sh --local-health-url --observer-health-url
check_flag p2p-real-env-triad-snapshot.sh --local-env-file --observer-env-file

[[ $failures == 0 ]] || exit 1
echo "p2p real-env local CLI tests passed"
