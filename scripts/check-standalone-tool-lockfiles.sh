#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

lockfiles=()
while IFS= read -r lockfile; do
  lockfiles+=("$lockfile")
done < <(git ls-files "tools/**/Cargo.lock" | sort)

if [[ "${#lockfiles[@]}" -eq 0 ]]; then
  echo "error: no tracked standalone tool lockfiles found under tools/" >&2
  exit 1
fi

checked=0
for lockfile in "${lockfiles[@]}"; do
  manifest="$(dirname "$lockfile")/Cargo.toml"
  lockfile="$(dirname "$manifest")/Cargo.lock"
  if [[ ! -f "$lockfile" ]]; then
    echo "error: standalone tool lockfile missing: $lockfile" >&2
    exit 1
  fi
  if [[ ! -f "$manifest" ]]; then
    echo "error: standalone tool manifest missing for lockfile: $lockfile" >&2
    exit 1
  fi

  echo "checking standalone tool lockfile: $manifest"
  env -u RUSTC_WRAPPER cargo metadata \
    --manifest-path "$manifest" \
    --locked \
    --format-version 1 >/dev/null
  checked=$((checked + 1))
done

echo "ok: standalone tool lockfiles are locked and manifest-consistent ($checked manifests)"
