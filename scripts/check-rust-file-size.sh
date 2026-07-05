#!/usr/bin/env bash
set -euo pipefail

repo_root=${OASIS7_RUST_FILE_SIZE_REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}
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

current_scan_tmp=$(mktemp)
current_structural_tmp=$(mktemp)
cleanup() {
  rm -f \
    "$current_scan_tmp" \
    "$current_structural_tmp"
}
trap cleanup EXIT

python3 - "$RUST_FILE_LINE_LIMIT" "$STRUCTURAL_SLICE_PATTERN" "$current_scan_tmp" "$current_structural_tmp" <<'PY'
from __future__ import annotations

from pathlib import Path
import re
import subprocess
import sys


line_limit = int(sys.argv[1])
structural_slice_pattern = re.compile(sys.argv[2])
oversized_output = Path(sys.argv[3])
structural_output = Path(sys.argv[4])
include_re = re.compile(r'include!\("([^"]+)"\)')


def classify_rust_file_kind(path: str) -> str:
    base = Path(path).name
    if "/tests/" in f"/{path}" or path.endswith("/tests.rs") or "tests" in base or base.endswith("_tests.rs"):
        return "test"
    return "code"


def path_matches_structural_slice_pattern(path: str) -> bool:
    return structural_slice_pattern.search(path) is not None


tracked = subprocess.run(
    ["git", "ls-files", "-z", "crates/**/*.rs", "tools/**/*.rs"],
    check=True,
    stdout=subprocess.PIPE,
)
paths = [path.decode("utf-8") for path in tracked.stdout.split(b"\0") if path]
oversized_entries: list[tuple[str, str, str]] = []
structural_entries: list[tuple[str, str, str]] = []

for path in paths:
    file_path = Path(path)
    data = file_path.read_bytes()
    line_count = data.count(b"\n")
    if line_count > line_limit:
        oversized_entries.append((classify_rust_file_kind(path), path, str(line_count)))

    if path_matches_structural_slice_pattern(path):
        structural_entries.append(("slice_file", path, "-"))

    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError:
        text = data.decode("utf-8", errors="ignore")
    for match in include_re.finditer(text):
        include_target = match.group(1)
        if path_matches_structural_slice_pattern(include_target):
            structural_entries.append(("include_target", path, include_target))

oversized_entries.sort(key=lambda entry: (entry[0], entry[1]))
structural_entries.sort(key=lambda entry: (entry[0], entry[1]))
oversized_output.write_text(
    "".join("\t".join(entry) + "\n" for entry in oversized_entries),
    encoding="utf-8",
)
structural_output.write_text(
    "".join("\t".join(entry) + "\n" for entry in structural_entries),
    encoding="utf-8",
)
PY

if [[ -s "$current_structural_tmp" ]]; then
  echo "check-rust-file-size: current structural slicing scan must be empty:"
  cat "$current_structural_tmp"
  fail "split_part/include-based structural slicing entries must be retired before merge"
fi

if [[ -s "$current_scan_tmp" ]]; then
  echo "check-rust-file-size: current oversized scan must be empty:"
  cat "$current_scan_tmp"
  fail "oversized Rust files must be reduced below ${RUST_FILE_LINE_LIMIT} lines before merge"
fi

code_count=$(awk -F '\t' '$1 == "code" {count++} END {print count + 0}' "$current_scan_tmp")
test_count=$(awk -F '\t' '$1 == "test" {count++} END {print count + 0}' "$current_scan_tmp")
slice_file_count=$(awk -F '\t' '$1 == "slice_file" {count++} END {print count + 0}' "$current_structural_tmp")
include_target_count=$(awk -F '\t' '$1 == "include_target" {count++} END {print count + 0}' "$current_structural_tmp")
echo "check-rust-file-size: oversized code files=${code_count}, test files=${test_count}, structural slice files=${slice_file_count}, include targets=${include_target_count}, limit=${RUST_FILE_LINE_LIMIT}"

if (( failures > 0 )); then
  exit 1
fi

echo "check-rust-file-size: OK"
