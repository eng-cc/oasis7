#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

readonly RUST_FILE_LINE_LIMIT=1200
readonly OVERSIZED_BASELINE_FILE="doc/.governance/rust-oversized-file-baseline.tsv"

usage() {
  cat <<'USAGE'
Usage: ./scripts/check-rust-file-size.sh [--write-baseline]

Checks:
  1. Scan tracked Rust source/test files under crates/ and identify files > 1200 lines.
  2. Require the current oversized-file baseline file to match the current scan exactly.
  3. When a previous baseline exists, reject any newly introduced oversized file path.

Options:
  --write-baseline   Rewrite doc/.governance/rust-oversized-file-baseline.tsv from current scan.
  -h, --help         Show this help.
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

write_baseline=0
if [[ $# -gt 1 ]]; then
  usage
  exit 1
fi
if [[ "${1:-}" == "--write-baseline" ]]; then
  write_baseline=1
elif [[ $# -eq 1 ]]; then
  usage
  exit 1
fi

failures=0

fail() {
  echo "check-rust-file-size: FAIL: $*"
  failures=$((failures + 1))
}

sort_tsv_file() {
  local file="$1"
  sort -t $'\t' -k1,1 -k2,2 "$file"
}

classify_rust_file_kind() {
  local path="$1"
  local base
  base=$(basename "$path")
  if [[ "$path" == */tests/* || "$path" == */tests.rs || "$base" == *tests*.rs || "$base" == *_tests.rs ]]; then
    printf 'test\n'
  else
    printf 'code\n'
  fi
}

scan_current_oversized_files() {
  local path line_count kind
  while IFS= read -r path; do
    [[ -n "$path" ]] || continue
    line_count=$(wc -l < "$path")
    line_count=${line_count//[[:space:]]/}
    if (( line_count > RUST_FILE_LINE_LIMIT )); then
      kind=$(classify_rust_file_kind "$path")
      printf '%s\t%s\t%s\n' "$kind" "$path" "$line_count"
    fi
  done < <(git ls-files 'crates/**/*.rs')
}

resolve_baseline_ref() {
  if ! git rev-parse --verify HEAD >/dev/null 2>&1; then
    return 1
  fi

  if ! git diff --quiet --ignore-submodules HEAD --; then
    printf 'HEAD\n'
    return 0
  fi

  if git rev-parse --verify HEAD^ >/dev/null 2>&1; then
    printf 'HEAD^\n'
    return 0
  fi

  return 1
}

extract_previous_baseline() {
  local baseline_ref="$1"
  local out_file="$2"
  if [[ -z "$baseline_ref" ]]; then
    return 1
  fi
  if git show "${baseline_ref}:${OVERSIZED_BASELINE_FILE}" > "$out_file" 2>/dev/null; then
    grep -Ev '^[[:space:]]*($|#)' "$out_file" > "${out_file}.filtered"
    mv "${out_file}.filtered" "$out_file"
    return 0
  fi
  return 1
}

current_scan_tmp=$(mktemp)
current_sorted_tmp=$(mktemp)
baseline_tmp=$(mktemp)
baseline_sorted_tmp=$(mktemp)
previous_baseline_tmp=$(mktemp)
cleanup() {
  rm -f "$current_scan_tmp" "$current_sorted_tmp" "$baseline_tmp" "$baseline_sorted_tmp" "$previous_baseline_tmp"
}
trap cleanup EXIT

scan_current_oversized_files > "$current_scan_tmp"
sort_tsv_file "$current_scan_tmp" > "$current_sorted_tmp"

if (( write_baseline == 1 )); then
  {
    echo "# schema: kind<TAB>path<TAB>line_count"
    echo "# kind in {code,test}; line_count is the frozen oversized baseline for tracked Rust files."
    cat "$current_sorted_tmp"
  } > "$OVERSIZED_BASELINE_FILE"
  echo "check-rust-file-size: wrote baseline to ${OVERSIZED_BASELINE_FILE}"
  exit 0
fi

if [[ ! -f "$OVERSIZED_BASELINE_FILE" ]]; then
  fail "baseline file missing: ${OVERSIZED_BASELINE_FILE}"
else
  grep -Ev '^[[:space:]]*($|#)' "$OVERSIZED_BASELINE_FILE" > "$baseline_tmp"
  sort_tsv_file "$baseline_tmp" > "$baseline_sorted_tmp"
fi

if [[ -f "$OVERSIZED_BASELINE_FILE" ]]; then
  unexpected_current=$(comm -23 "$current_sorted_tmp" "$baseline_sorted_tmp" || true)
  stale_baseline=$(comm -13 "$current_sorted_tmp" "$baseline_sorted_tmp" || true)

  if [[ -n "$unexpected_current" ]]; then
    echo "check-rust-file-size: current oversized scan differs from frozen baseline:"
    echo "$unexpected_current"
    fail "current oversized scan contains entries not recorded in the frozen baseline"
  fi

  if [[ -n "$stale_baseline" ]]; then
    echo "check-rust-file-size: frozen baseline contains stale entries:"
    echo "$stale_baseline"
    fail "baseline contains entries that no longer match the current oversized scan"
  fi
fi

baseline_ref=""
if baseline_ref=$(resolve_baseline_ref); then
  :
else
  baseline_ref=""
fi

if extract_previous_baseline "$baseline_ref" "$previous_baseline_tmp"; then
  new_oversized=$(comm -23 "$current_sorted_tmp" <(sort_tsv_file "$previous_baseline_tmp") || true)
  if [[ -n "$new_oversized" ]]; then
    echo "check-rust-file-size: newly introduced oversized Rust files relative to ${baseline_ref}:"
    echo "$new_oversized"
    fail "new oversized Rust files are not allowed"
  fi
else
  echo "check-rust-file-size: bootstrap mode (no previous baseline found)"
fi

code_count=$(awk -F '\t' '$1 == "code" {count++} END {print count + 0}' "$current_sorted_tmp")
test_count=$(awk -F '\t' '$1 == "test" {count++} END {print count + 0}' "$current_sorted_tmp")
echo "check-rust-file-size: oversized code files=${code_count}, test files=${test_count}, limit=${RUST_FILE_LINE_LIMIT}"

if (( failures > 0 )); then
  exit 1
fi

echo "check-rust-file-size: OK"
