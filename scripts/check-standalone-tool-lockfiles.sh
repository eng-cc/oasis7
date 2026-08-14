#!/usr/bin/env bash
set -euo pipefail

repo_root="${OASIS7_STANDALONE_TOOL_REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$repo_root"

tracked_standalone_file_set=$'\n'
manifests=()
lockfiles=()
while IFS= read -r tracked_file; do
  tracked_standalone_file_set+="$tracked_file"$'\n'
  case "$tracked_file" in
    */Cargo.toml) manifests+=("$tracked_file") ;;
    */Cargo.lock) lockfiles+=("$tracked_file") ;;
  esac
done < <(git ls-files \
  "tools/**/Cargo.toml" \
  "tools/**/Cargo.lock" \
  "crates/oasis7_builtin_wasm_modules/*/Cargo.toml" \
  "crates/oasis7_builtin_wasm_modules/*/Cargo.lock" | sort)

is_tracked_standalone_file() {
  local needle="$1"
  [[ "$tracked_standalone_file_set" == *$'\n'"$needle"$'\n'* ]]
}

if [[ "${#manifests[@]}" -eq 0 ]]; then
  echo "error: no tracked standalone Cargo manifests found" >&2
  exit 1
fi

checked=0
for manifest in "${manifests[@]}"; do
  lockfile="$(dirname "$manifest")/Cargo.lock"
  if [[ ! -f "$lockfile" ]]; then
    echo "error: standalone tool lockfile missing: $lockfile" >&2
    exit 1
  fi
  if ! is_tracked_standalone_file "$lockfile"; then
    echo "error: standalone lockfile is not tracked: $lockfile" >&2
    exit 1
  fi

  echo "checking standalone lockfile: $manifest"
  env -u RUSTC_WRAPPER cargo metadata \
    --manifest-path "$manifest" \
    --locked \
    --format-version 1 >/dev/null
  checked=$((checked + 1))
done

for lockfile in "${lockfiles[@]}"; do
  manifest="$(dirname "$lockfile")/Cargo.toml"
  if ! is_tracked_standalone_file "$manifest"; then
    echo "error: standalone manifest missing for lockfile: $lockfile" >&2
    exit 1
  fi
done

echo "ok: standalone lockfiles are locked and manifest-consistent ($checked manifests)"
