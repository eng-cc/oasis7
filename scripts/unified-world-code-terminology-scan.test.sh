#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

content_probe="doc/testing/templates/unified-world-code-scan-content-probe.template.tsv"
path_probe="doc/testing/templates/shared""-network-regression-probe.template.tsv"
sandbox_root=""

cleanup() {
  rm -f "$content_probe" "$path_probe"
  [[ -z "$sandbox_root" ]] || rm -rf "$sandbox_root"
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

make_sandbox() {
  [[ -z "$sandbox_root" ]] || rm -rf "$sandbox_root"
  sandbox_root="$(mktemp -d)"
  mkdir -p "$sandbox_root/scripts/fixtures/document-corpus-v3"
  cp ./scripts/unified-world-code-terminology-scan.sh "$sandbox_root/scripts/"
  cp ./scripts/document_evidence_policy.py "$sandbox_root/scripts/"
  cp -R ./scripts/fixtures/document-corpus-v3/legacy \
    "$sandbox_root/scripts/fixtures/document-corpus-v3/"
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

make_sandbox
printf 'clean additional fixture\n' > \
  "$sandbox_root/scripts/fixtures/document-corpus-v3/legacy/extra.json"
assert_fails_with "fixture file set differs from the fixed manifest" \
  "$sandbox_root/scripts/unified-world-code-terminology-scan.sh"

make_sandbox
linked_payload="$sandbox_root/scripts/fixtures/document-corpus-v3/legacy/document-corpus-inventory.json"
rm -f "$linked_payload"
ln -s inventory.json "$linked_payload"
assert_fails_with "fixture path is not a regular file" \
  "$sandbox_root/scripts/unified-world-code-terminology-scan.sh"

make_sandbox
printf '\n' >> "$sandbox_root/scripts/fixtures/document-corpus-v3/legacy/manifest.json"
assert_fails_with "fixed manifest SHA-256 mismatch" \
  "$sandbox_root/scripts/unified-world-code-terminology-scan.sh"

make_sandbox
printf 'changed payload bytes\n' >> \
  "$sandbox_root/scripts/fixtures/document-corpus-v3/legacy/document-corpus-inventory.json"
assert_fails_with "frozen payload byte count or SHA-256 mismatch" \
  "$sandbox_root/scripts/unified-world-code-terminology-scan.sh"

make_sandbox
misused_reference="doc/testing/evidence/shared""-network-"
printf 'unexpected_active_reference = "%s"\n' "$misused_reference" >> \
  "$sandbox_root/scripts/document_evidence_policy.py"
assert_fails_with "unexpected_active_reference" \
  "$sandbox_root/scripts/unified-world-code-terminology-scan.sh"

make_sandbox
awk '/authority_path = / { print; exit }' ./scripts/document_evidence_policy.py >> \
  "$sandbox_root/scripts/document_evidence_policy.py"
assert_fails_with "exact compatibility reference must occur once; found 2" \
  "$sandbox_root/scripts/unified-world-code-terminology-scan.sh"

echo "unified-world-code-terminology-scan.test: OK"
