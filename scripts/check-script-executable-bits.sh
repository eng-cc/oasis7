#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

required_scripts=(
  "scripts/check-script-executable-bits.sh"
  "scripts/release-gate.sh"
  "scripts/release-gate-web-strict.sh"
  "scripts/viewer-primary-web-entry-regression.sh"
  "scripts/viewer-software-safe-step-regression.sh"
  "scripts/release-prepare-bundle.sh"
  "scripts/build-game-launcher-bundle.sh"
  "scripts/package-native-installer.sh"
  "scripts/validate-release-platform-entrypoints.sh"
)

failures=0

for path in "${required_scripts[@]}"; do
  tracked_mode=$(git ls-files --stage -- "$path" | awk 'NR==1 { print $1 }')
  if [[ -z "$tracked_mode" ]]; then
    echo "error: required release script is not tracked: $path" >&2
    failures=1
    continue
  fi
  if [[ "$tracked_mode" != "100755" ]]; then
    echo "error: required release script must be tracked as executable (100755): $path (actual: $tracked_mode)" >&2
    failures=1
  fi
  if [[ ! -x "$path" ]]; then
    echo "error: required release script is not executable in worktree: $path" >&2
    failures=1
  fi
done

if [[ "$failures" -ne 0 ]]; then
  exit 1
fi

echo "ok: required release scripts are tracked and executable"
