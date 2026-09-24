#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
ci_tests="$repo_root/scripts/ci-tests.sh"
clippy_helper=$(sed -n '/^run_cargo_clippy()/,/^}/p' "$ci_tests")

if ! grep -Fqx '    -D warnings' <<<"$clippy_helper"; then
  echo "ci-tests Clippy helper must deny every Rust and Clippy warning" >&2
  exit 1
fi

if [[ $(grep -Fxc '    -D warnings' <<<"$clippy_helper") -ne 1 ]]; then
  echo "ci-tests Clippy helper must define the warning-zero gate exactly once" >&2
  exit 1
fi

echo "ci-tests warning-zero contract: passed"
