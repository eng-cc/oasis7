#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

content_probe="doc/testing/templates/unified-world-code-scan-content-probe.template.tsv"
path_probe="doc/testing/templates/shared""-network-regression-probe.template.tsv"

cleanup() {
  rm -f "$content_probe" "$path_probe"
}
trap cleanup EXIT

assert_fails_with() {
  local expected="$1"
  shift
  local status=0
  local output
  output="$("$@" 2>&1)" || status=$?
  if [[ "$status" -eq 0 ]]; then
    echo "expected command to fail: $*" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
  if [[ "$output" != *"$expected"* ]]; then
    echo "expected failure output to contain: $expected" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
}

cleanup

./scripts/unified-world-code-terminology-scan.sh >/dev/null

legacy_content="shared""_network should not re-enter active templates"
printf 'mode\tclaim\nprobe\t%s\n' "$legacy_content" > "$content_probe"
assert_fails_with "$content_probe" ./scripts/unified-world-code-terminology-scan.sh
assert_fails_with "$legacy_content" ./scripts/unified-world-code-terminology-scan.sh
rm -f "$content_probe"

printf 'mode\tclaim\nprobe\tclean contents\n' > "$path_probe"
assert_fails_with "$path_probe: legacy terminology in path name" ./scripts/unified-world-code-terminology-scan.sh
rm -f "$path_probe"

./scripts/unified-world-code-terminology-scan.sh >/dev/null

echo "unified-world-code-terminology-scan.test: OK"
