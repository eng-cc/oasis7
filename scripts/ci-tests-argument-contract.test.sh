#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPT="$ROOT_DIR/scripts/ci-tests.sh"
header="$(sed -n '1,45p' "$SCRIPT")"

if ! grep -Fq 'if [[ $# -eq 0 ]]; then' <<<"$header"; then
  echo "ci-tests must reject an omitted tier before any test command can run" >&2
  exit 1
fi

if ! grep -Fq 'Default: none (explicit tier required)' <<<"$header"; then
  echo "ci-tests usage must state that an explicit tier is required" >&2
  exit 1
fi

if ! grep -Fq 'commit|required|full|full-core|full-support) ;;' <<<"$header"; then
  echo "ci-tests must retain every explicit tier" >&2
  exit 1
fi

echo "ci-tests-argument-contract.test: OK"
