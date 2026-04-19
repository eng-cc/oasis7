#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

readonly RUST_FILE_LINE_LIMIT=1200
readonly OVERSIZED_BASELINE_FILE="doc/.governance/rust-oversized-file-baseline.tsv"
readonly STRUCTURAL_SLICE_PATTERN='(^|[_/])(split_part[0-9]+|part[0-9]+|impl_part[0-9]+)\.rs$'

usage() {
  cat <<'USAGE'
Usage: ./scripts/check-rust-file-size.sh [--write-baseline]

Checks:
  1. Scan tracked Rust source/test files under crates/ and identify files > 1200 lines.
  2. Require the current oversized-file baseline file to match the current scan exactly.
  3. When a previous baseline exists, reject any newly introduced oversized file path.
  4. When a touched oversized Rust file already exceeded 1200 lines, require its current line count to shrink.
  5. Require the current split-part/include!-based structural slicing scan to be empty.

Options:
  --write-baseline             Rewrite doc/.governance/rust-oversized-file-baseline.tsv from current scan.
  -h, --help                   Show this help.
USAGE
}

write_baseline=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --write-baseline)
      write_baseline=1
      ;;
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

sort_tsv_prefix_file() {
  local file="$1"
  cut -f1-2 "$file" | sort -t $'\t' -k1,1 -k2,2
}

strip_comment_and_blank_lines() {
  local input_file="$1"
  local output_file="$2"
  grep -Ev '^[[:space:]]*($|#)' "$input_file" > "$output_file" || true
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

scan_ref_oversized_files() {
  local ref="$1"
  local path line_count kind
  while IFS= read -r path; do
    [[ -n "$path" ]] || continue
    [[ "$path" == crates/*.rs ]] || continue
    line_count=$(git show "${ref}:${path}" | wc -l)
    line_count=${line_count//[[:space:]]/}
    if (( line_count > RUST_FILE_LINE_LIMIT )); then
      kind=$(classify_rust_file_kind "$path")
      printf '%s\t%s\t%s\n' "$kind" "$path" "$line_count"
    fi
  done < <(git ls-tree -r --name-only "$ref")
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
  done < <(git ls-files 'crates/**/*.rs')
}

scan_ref_structural_slice_entries() {
  local ref="$1"
  local path line include_target
  while IFS= read -r path; do
    [[ -n "$path" ]] || continue
    [[ "$path" == crates/*.rs ]] || continue

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
    done < <(git show "${ref}:${path}")
  done < <(git ls-tree -r --name-only "$ref")
}

resolve_compare_ref() {
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

extract_previous_baseline_file() {
  local baseline_ref="$1"
  local baseline_file="$2"
  local out_file="$3"
  if [[ -z "$baseline_ref" ]]; then
    return 1
  fi
  if git show "${baseline_ref}:${baseline_file}" > "$out_file" 2>/dev/null; then
    strip_comment_and_blank_lines "$out_file" "${out_file}.filtered"
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
previous_scan_tmp=$(mktemp)
previous_scan_sorted_tmp=$(mktemp)
current_structural_tmp=$(mktemp)
current_structural_sorted_tmp=$(mktemp)
cleanup() {
  rm -f \
    "$current_scan_tmp" \
    "$current_sorted_tmp" \
    "$baseline_tmp" \
    "$baseline_sorted_tmp" \
    "$previous_baseline_tmp" \
    "$previous_scan_tmp" \
    "$previous_scan_sorted_tmp" \
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

if (( write_baseline == 1 )); then
  {
    echo "# schema: kind<TAB>path<TAB>line_count"
    echo "# kind in {code,test}; line_count is the frozen oversized baseline for tracked Rust files."
    cat "$current_sorted_tmp"
  } > "$OVERSIZED_BASELINE_FILE"
  echo "check-rust-file-size: wrote baseline to ${OVERSIZED_BASELINE_FILE}"
fi

if (( write_baseline == 1 )); then
  exit 0
fi

if [[ ! -f "$OVERSIZED_BASELINE_FILE" ]]; then
  fail "baseline file missing: ${OVERSIZED_BASELINE_FILE}"
else
  strip_comment_and_blank_lines "$OVERSIZED_BASELINE_FILE" "$baseline_tmp"
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

compare_ref=""
if compare_ref=$(resolve_compare_ref); then
  :
else
  compare_ref=""
fi

if [[ -n "$compare_ref" ]]; then
  scan_ref_oversized_files "$compare_ref" > "$previous_scan_tmp"
  sort_tsv_file "$previous_scan_tmp" > "$previous_scan_sorted_tmp"
  new_oversized=$(comm -23 <(sort_tsv_prefix_file "$current_sorted_tmp") <(sort_tsv_prefix_file "$previous_scan_sorted_tmp") || true)
  if [[ -n "$new_oversized" ]]; then
    echo "check-rust-file-size: newly introduced oversized Rust files relative to ${compare_ref}:"
    echo "$new_oversized"
    fail "new oversized Rust files are not allowed"
  fi
else
  echo "check-rust-file-size: bootstrap mode (no compare ref found)"
fi

if [[ -n "$compare_ref" ]]; then
  while IFS=$'\t' read -r status old_path new_path; do
    [[ -n "$status" ]] || continue

    case "$status" in
      M)
        old_path="$old_path"
        new_path="$old_path"
        ;;
      R*)
        ;;
      *)
        continue
        ;;
    esac

    [[ "$old_path" == crates/*.rs ]] || continue
    if ! git cat-file -e "${compare_ref}:${old_path}" 2>/dev/null; then
      continue
    fi

    previous_line_count=$(git show "${compare_ref}:${old_path}" | wc -l)
    previous_line_count=${previous_line_count//[[:space:]]/}
    if (( previous_line_count <= RUST_FILE_LINE_LIMIT )); then
      continue
    fi

    if [[ ! -f "$new_path" ]]; then
      continue
    fi

    current_line_count=$(wc -l < "$new_path")
    current_line_count=${current_line_count//[[:space:]]/}
    if (( current_line_count >= previous_line_count )); then
      fail "touched oversized Rust file must shrink: ${old_path} (${previous_line_count} -> ${current_line_count})"
    fi
  done < <(git diff --name-status --find-renames "$compare_ref" -- 'crates/**/*.rs')
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
