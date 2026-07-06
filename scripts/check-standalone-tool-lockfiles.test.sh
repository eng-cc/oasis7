#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
script_path="$repo_root/scripts/check-standalone-tool-lockfiles.sh"
tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT
real_git="$(command -v git)"

fixture_repo="$tmp_dir/repo"
mkdir -p "$fixture_repo/tools/valid_tool/src"
cd "$fixture_repo"
git init -q
git config user.email "test@example.invalid"
git config user.name "Test User"

cat >tools/valid_tool/Cargo.toml <<'TOML'
[package]
name = "valid_tool"
version = "0.1.0"
edition = "2021"

[dependencies]
TOML
cat >tools/valid_tool/src/main.rs <<'RS'
fn main() {}
RS
env -u RUSTC_WRAPPER cargo generate-lockfile --manifest-path tools/valid_tool/Cargo.toml
git add tools/valid_tool/Cargo.toml tools/valid_tool/Cargo.lock tools/valid_tool/src/main.rs
git commit -q -m "valid tool"

valid_out="$tmp_dir/valid.out"
fake_bin="$tmp_dir/fake-bin"
mkdir -p "$fake_bin"
cat >"$fake_bin/git" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "ls-files" ]]; then
  echo "git ls-files $*" >>"$OASIS7_GIT_CALL_LOG"
fi
exec "$OASIS7_REAL_GIT" "$@"
SH
chmod +x "$fake_bin/git"

OASIS7_STANDALONE_TOOL_REPO_ROOT="$fixture_repo" \
  OASIS7_REAL_GIT="$real_git" \
  OASIS7_GIT_CALL_LOG="$tmp_dir/git-calls.log" \
  PATH="$fake_bin:$PATH" \
  "$script_path" >"$valid_out"
grep -q "ok: standalone tool lockfiles are locked and manifest-consistent (1 manifests)" "$valid_out"
test "$(grep -c "^git ls-files " "$tmp_dir/git-calls.log")" -eq 1

mkdir -p tools/missing_lock/src
cat >tools/missing_lock/Cargo.toml <<'TOML'
[package]
name = "missing_lock"
version = "0.1.0"
edition = "2021"

[dependencies]
TOML
cat >tools/missing_lock/src/main.rs <<'RS'
fn main() {}
RS
git add tools/missing_lock/Cargo.toml tools/missing_lock/src/main.rs
if OASIS7_STANDALONE_TOOL_REPO_ROOT="$fixture_repo" "$script_path" >"$tmp_dir/missing-lock.out" 2>&1; then
  echo "expected manifest without tracked lockfile to fail" >&2
  exit 1
fi
grep -q "standalone tool lockfile missing: tools/missing_lock/Cargo.lock" "$tmp_dir/missing-lock.out"
git reset -q --hard HEAD

mkdir -p tools/orphan_lock
cp tools/valid_tool/Cargo.lock tools/orphan_lock/Cargo.lock
git add tools/orphan_lock/Cargo.lock
if OASIS7_STANDALONE_TOOL_REPO_ROOT="$fixture_repo" "$script_path" >"$tmp_dir/orphan-lock.out" 2>&1; then
  echo "expected orphan lockfile to fail" >&2
  exit 1
fi
grep -q "standalone tool manifest missing for lockfile: tools/orphan_lock/Cargo.lock" "$tmp_dir/orphan-lock.out"

echo "check-standalone-tool-lockfiles.test: OK"
