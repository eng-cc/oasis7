#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

staged_rust_files=()
while IFS= read -r file; do
  staged_rust_files+=("$file")
done < <(git diff --cached --name-only --diff-filter=ACMR -- '*.rs')

if [[ ${#staged_rust_files[@]} -gt 0 ]]; then
  echo "+ env -u RUSTC_WRAPPER rustfmt --edition 2021 ${staged_rust_files[*]}"
  env -u RUSTC_WRAPPER rustfmt --edition 2021 "${staged_rust_files[@]}"
  echo "+ git add ${staged_rust_files[*]}"
  git add "${staged_rust_files[@]}"
else
  echo "+ no staged Rust files, skip rustfmt"
fi

./scripts/ci-tests.sh commit
