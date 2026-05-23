#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

readonly RUST_FILE_LINE_LIMIT=1200
readonly STRUCTURAL_SLICE_PATTERN='(^|[_/])(split_part[0-9]+|part[0-9]+|impl_part[0-9]+)\.rs$'

usage() {
  cat <<'USAGE'
Usage: ./scripts/check-rust-file-size.sh

Checks:
  1. Scan tracked first-party Rust source/test files under crates/ and tools/ and identify files > 1200 lines.
  2. Require the current oversized Rust scan to be empty.
  3. Require the current split-part/include!-based structural slicing scan to be empty.

Options:
  -h, --help                   Show this help.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage
      exit 1
      ;;
  esac
  shift
done

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

path_matches_structural_slice_pattern() {
  local path="$1"
  [[ "$path" =~ $STRUCTURAL_SLICE_PATTERN ]]
}

scan_tracked_first_party_rust_files() {
  git ls-files 'crates/**/*.rs' 'tools/**/*.rs'
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
  done < <(scan_tracked_first_party_rust_files)
}

scan_current_structural_slice_entries() {
  local path line include_target
  while IFS= read -r path; do
    [[ -n "$path" ]] || continue

    if path_matches_structural_slice_pattern "$path"; then
      printf 'slice_file\t%s\t-\n' "$path"
    fi

    while IFS= read -r line; do
      if [[ "$line" =~ include!\(\"([^\"]+)\"\) ]]; then
        include_target="${BASH_REMATCH[1]}"
        if path_matches_structural_slice_pattern "$include_target"; then
          printf 'include_target\t%s\t%s\n' "$path" "$include_target"
        fi
      fi
    done < "$path"
  done < <(scan_tracked_first_party_rust_files)
}

current_scan_tmp=$(mktemp)
current_sorted_tmp=$(mktemp)
current_structural_tmp=$(mktemp)
current_structural_sorted_tmp=$(mktemp)
cleanup() {
  rm -f \
    "$current_scan_tmp" \
    "$current_sorted_tmp" \
    "$current_structural_tmp" \
    "$current_structural_sorted_tmp"
}
trap cleanup EXIT

scan_current_oversized_files > "$current_scan_tmp"
sort_tsv_file "$current_scan_tmp" > "$current_sorted_tmp"
scan_current_structural_slice_entries > "$current_structural_tmp"
sort_tsv_file "$current_structural_tmp" > "$current_structural_sorted_tmp"

if [[ -s "$current_structural_sorted_tmp" ]]; then
  echo "check-rust-file-size: current structural slicing scan must be empty:"
  cat "$current_structural_sorted_tmp"
  fail "split_part/include-based structural slicing entries must be retired before merge"
fi

if [[ -s "$current_sorted_tmp" ]]; then
  echo "check-rust-file-size: current oversized scan must be empty:"
  cat "$current_sorted_tmp"
  fail "oversized Rust files must be reduced below ${RUST_FILE_LINE_LIMIT} lines before merge"
fi

code_count=$(awk -F '\t' '$1 == "code" {count++} END {print count + 0}' "$current_sorted_tmp")
test_count=$(awk -F '\t' '$1 == "test" {count++} END {print count + 0}' "$current_sorted_tmp")
slice_file_count=$(awk -F '\t' '$1 == "slice_file" {count++} END {print count + 0}' "$current_structural_sorted_tmp")
include_target_count=$(awk -F '\t' '$1 == "include_target" {count++} END {print count + 0}' "$current_structural_sorted_tmp")
echo "check-rust-file-size: oversized code files=${code_count}, test files=${test_count}, structural slice files=${slice_file_count}, include targets=${include_target_count}, limit=${RUST_FILE_LINE_LIMIT}"

if (( failures > 0 )); then
  exit 1
fi

echo "check-rust-file-size: OK"
