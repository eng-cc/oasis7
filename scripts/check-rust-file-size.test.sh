#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
script_path="$repo_root/scripts/check-rust-file-size.sh"
tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT

fixture_repo="$tmp_dir/repo"
mkdir -p "$fixture_repo/crates/demo/src" "$fixture_repo/tools/tool/src"
cd "$fixture_repo"
git init -q
git config user.email "test@example.invalid"
git config user.name "Test User"

cat >crates/demo/src/lib.rs <<'RS'
pub fn ok() {}
RS
cat >tools/tool/src/main.rs <<'RS'
fn main() {}
RS
git add crates/demo/src/lib.rs tools/tool/src/main.rs
git commit -q -m "valid rust files"

valid_out="$tmp_dir/valid.out"
OASIS7_RUST_FILE_SIZE_REPO_ROOT="$fixture_repo" "$script_path" >"$valid_out"
grep -q "oversized code files=0, test files=0, structural slice files=0, include targets=0, limit=1200" "$valid_out"
grep -q "check-rust-file-size: OK" "$valid_out"

python3 - <<'PY'
from pathlib import Path

Path("crates/demo/src/large.rs").write_text(
    "".join(f"// line {i}\n" for i in range(1201)),
    encoding="utf-8",
)
PY
git add crates/demo/src/large.rs
if OASIS7_RUST_FILE_SIZE_REPO_ROOT="$fixture_repo" "$script_path" >"$tmp_dir/oversized.out" 2>&1; then
  echo "expected oversized Rust file to fail" >&2
  exit 1
fi
grep -q $'code\tcrates/demo/src/large.rs\t1201' "$tmp_dir/oversized.out"
grep -q "oversized code files=1, test files=0, structural slice files=0, include targets=0, limit=1200" "$tmp_dir/oversized.out"
git reset -q --hard HEAD

cat >crates/demo/src/split_part1.rs <<'RS'
pub fn split() {}
RS
git add crates/demo/src/split_part1.rs
if OASIS7_RUST_FILE_SIZE_REPO_ROOT="$fixture_repo" "$script_path" >"$tmp_dir/slice-file.out" 2>&1; then
  echo "expected structural slice file to fail" >&2
  exit 1
fi
grep -q $'slice_file\tcrates/demo/src/split_part1.rs\t-' "$tmp_dir/slice-file.out"
grep -q "structural slice files=1, include targets=0" "$tmp_dir/slice-file.out"
git reset -q --hard HEAD

cat >crates/demo/src/lib.rs <<'RS'
include!("impl_part2.rs");
pub fn ok() {}
RS
git add crates/demo/src/lib.rs
if OASIS7_RUST_FILE_SIZE_REPO_ROOT="$fixture_repo" "$script_path" >"$tmp_dir/include-target.out" 2>&1; then
  echo "expected structural include target to fail" >&2
  exit 1
fi
grep -q $'include_target\tcrates/demo/src/lib.rs\timpl_part2.rs' "$tmp_dir/include-target.out"
grep -q "structural slice files=0, include targets=1" "$tmp_dir/include-target.out"

echo "check-rust-file-size.test: OK"
