#!/usr/bin/env bash
set -euo pipefail

repo_root="${OASIS7_STANDALONE_TOOL_REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$repo_root"

manifests=()
while IFS= read -r manifest; do
  manifests+=("$manifest")
done < <(git ls-files "tools/**/Cargo.toml" | sort)

if [[ "${#manifests[@]}" -eq 0 ]]; then
  echo "error: no tracked standalone tool manifests found under tools/" >&2
  exit 1
fi

checked=0
for manifest in "${manifests[@]}"; do
  lockfile="$(dirname "$manifest")/Cargo.lock"
  if [[ ! -f "$lockfile" ]]; then
    echo "error: standalone tool lockfile missing: $lockfile" >&2
    exit 1
  fi
  if ! git ls-files --error-unmatch "$lockfile" >/dev/null 2>&1; then
    echo "error: standalone tool lockfile is not tracked: $lockfile" >&2
    exit 1
  fi

  echo "checking standalone tool lockfile: $manifest"
  env -u RUSTC_WRAPPER cargo metadata \
    --manifest-path "$manifest" \
    --locked \
    --format-version 1 >/dev/null
  checked=$((checked + 1))
done

while IFS= read -r lockfile; do
  manifest="$(dirname "$lockfile")/Cargo.toml"
  if ! git ls-files --error-unmatch "$manifest" >/dev/null 2>&1; then
    echo "error: standalone tool manifest missing for lockfile: $lockfile" >&2
    exit 1
  fi
done < <(git ls-files "tools/**/Cargo.lock" | sort)

echo "ok: standalone tool lockfiles are locked and manifest-consistent ($checked manifests)"
