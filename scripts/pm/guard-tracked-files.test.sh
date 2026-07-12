#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-guard-tracked-files-test.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

new_repo() {
  local name="$1"
  REPO="$TMP_DIR/$name/repo"
  STATE="$TMP_DIR/$name/state"
  TRACKED="$REPO/.pm/roles/tpm/backlog/committed.yaml"
  mkdir -p "$(dirname "$TRACKED")"
  printf 'baseline\n' > "$TRACKED"
  git -C "$REPO" init -q
  git -C "$REPO" add .pm/roles/tpm/backlog/committed.yaml
}

new_repo tracked-edit
python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" snapshot \
  --root "$REPO" --state "$STATE" --pathspec .pm/roles

printf 'legitimate concurrent edit\n' > "$TRACKED"
if python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" check \
  --root "$REPO" --state "$STATE" --pathspec .pm/roles \
  >"$TMP_DIR/tracked.out" 2>"$TMP_DIR/tracked.err"; then
  echo "guard-tracked-files.test: expected tracked edit detection" >&2
  exit 1
fi
grep -F "tracked projection drift: .pm/roles/tpm/backlog/committed.yaml" \
  "$TMP_DIR/tracked.err" >/dev/null
if [[ "$(cat "$TRACKED")" != "legitimate concurrent edit" ]]; then
  echo "guard-tracked-files.test: fail-only guard overwrote a legitimate concurrent edit" >&2
  exit 1
fi
printf 'concurrent tracked edit detected and preserved\n'

new_repo untracked-artifact
UNTRACKED="$REPO/.pm/roles/tpm/backlog/test-artifact.yaml"
python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" snapshot \
  --root "$REPO" --state "$STATE" --pathspec .pm/roles
printf 'artifact\n' > "$UNTRACKED"
if python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" check \
  --root "$REPO" --state "$STATE" --pathspec .pm/roles \
  >"$TMP_DIR/untracked.out" 2>"$TMP_DIR/untracked.err"; then
  echo "guard-tracked-files.test: expected untracked artifact detection" >&2
  exit 1
fi
grep -F "new untracked projection artifact: .pm/roles/tpm/backlog/test-artifact.yaml" \
  "$TMP_DIR/untracked.err" >/dev/null
printf 'new untracked artifact detected\n'

new_repo staged-new
STAGED_NEW="$REPO/.pm/roles/tpm/backlog/staged-new.yaml"
python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" snapshot \
  --root "$REPO" --state "$STATE" --pathspec .pm/roles
printf 'staged artifact\n' > "$STAGED_NEW"
git -C "$REPO" add .pm/roles/tpm/backlog/staged-new.yaml
if python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" check \
  --root "$REPO" --state "$STATE" --pathspec .pm/roles \
  >"$TMP_DIR/staged-new.out" 2>"$TMP_DIR/staged-new.err"; then
  echo "guard-tracked-files.test: expected staged-new path detection" >&2
  exit 1
fi
grep -F "new index projection path: .pm/roles/tpm/backlog/staged-new.yaml" \
  "$TMP_DIR/staged-new.err" >/dev/null
printf 'new staged path detected\n'

new_repo untracked-to-staged
UNTRACKED_STAGED="$REPO/.pm/roles/tpm/backlog/untracked-to-staged.yaml"
printf 'initial untracked\n' > "$UNTRACKED_STAGED"
python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" snapshot \
  --root "$REPO" --state "$STATE" --pathspec .pm/roles
git -C "$REPO" add .pm/roles/tpm/backlog/untracked-to-staged.yaml
if python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" check \
  --root "$REPO" --state "$STATE" --pathspec .pm/roles \
  >"$TMP_DIR/untracked-to-staged.out" 2>"$TMP_DIR/untracked-to-staged.err"; then
  echo "guard-tracked-files.test: expected untracked-to-staged path detection" >&2
  exit 1
fi
grep -F "new index projection path: .pm/roles/tpm/backlog/untracked-to-staged.yaml" \
  "$TMP_DIR/untracked-to-staged.err" >/dev/null
printf 'untracked-to-staged transition detected\n'

new_repo staged-removal
python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" snapshot \
  --root "$REPO" --state "$STATE" --pathspec .pm/roles
git -C "$REPO" rm --cached -q .pm/roles/tpm/backlog/committed.yaml
if python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" check \
  --root "$REPO" --state "$STATE" --pathspec .pm/roles \
  >"$TMP_DIR/staged-removal.out" 2>"$TMP_DIR/staged-removal.err"; then
  echo "guard-tracked-files.test: expected staged-removal path detection" >&2
  exit 1
fi
grep -F "removed index projection path: .pm/roles/tpm/backlog/committed.yaml" \
  "$TMP_DIR/staged-removal.err" >/dev/null
printf 'staged path removal detected\n'

new_repo regular-to-symlink
python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" snapshot \
  --root "$REPO" --state "$STATE" --pathspec .pm/roles
printf 'baseline\n' >"$REPO/same-content.txt"
rm "$TRACKED"
ln -s "$REPO/same-content.txt" "$TRACKED"
if python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" check \
  --root "$REPO" --state "$STATE" --pathspec .pm/roles \
  >"$TMP_DIR/regular-to-symlink.out" 2>"$TMP_DIR/regular-to-symlink.err"; then
  echo "guard-tracked-files.test: expected regular-to-symlink detection" >&2
  exit 1
fi
grep -F "tracked projection drift: .pm/roles/tpm/backlog/committed.yaml" \
  "$TMP_DIR/regular-to-symlink.err" >/dev/null
printf 'regular-to-symlink transition detected despite identical content\n'

new_repo symlink-retarget
printf 'first\n' >"$REPO/first.txt"
printf 'second\n' >"$REPO/second.txt"
rm "$TRACKED"
ln -s "$REPO/first.txt" "$TRACKED"
git -C "$REPO" add .pm/roles/tpm/backlog/committed.yaml
python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" snapshot \
  --root "$REPO" --state "$STATE" --pathspec .pm/roles
rm "$TRACKED"
ln -s "$REPO/second.txt" "$TRACKED"
if python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" check \
  --root "$REPO" --state "$STATE" --pathspec .pm/roles \
  >"$TMP_DIR/symlink-retarget.out" 2>"$TMP_DIR/symlink-retarget.err"; then
  echo "guard-tracked-files.test: expected symlink retarget detection" >&2
  exit 1
fi
grep -F "tracked projection drift: .pm/roles/tpm/backlog/committed.yaml" \
  "$TMP_DIR/symlink-retarget.err" >/dev/null
printf 'symlink retarget detected\n'

new_repo worktree-mode
python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" snapshot \
  --root "$REPO" --state "$STATE" --pathspec .pm/roles
chmod +x "$TRACKED"
if python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" check \
  --root "$REPO" --state "$STATE" --pathspec .pm/roles \
  >"$TMP_DIR/worktree-mode.out" 2>"$TMP_DIR/worktree-mode.err"; then
  echo "guard-tracked-files.test: expected worktree mode-only detection" >&2
  exit 1
fi
grep -F "tracked projection drift: .pm/roles/tpm/backlog/committed.yaml" \
  "$TMP_DIR/worktree-mode.err" >/dev/null
printf 'worktree mode-only drift detected\n'

new_repo index-mode
python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" snapshot \
  --root "$REPO" --state "$STATE" --pathspec .pm/roles
chmod +x "$TRACKED"
git -C "$REPO" add .pm/roles/tpm/backlog/committed.yaml
if python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" check \
  --root "$REPO" --state "$STATE" --pathspec .pm/roles \
  >"$TMP_DIR/index-mode.out" 2>"$TMP_DIR/index-mode.err"; then
  echo "guard-tracked-files.test: expected index mode/oid/path detection" >&2
  exit 1
fi
grep -F "index projection mode/oid/path drift" "$TMP_DIR/index-mode.err" >/dev/null
printf 'exact index mode drift detected\n'

expect_filesystem_drift() {
  local label="$1"
  local expected="$2"
  if python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" check \
    --root "$REPO" --state "$STATE" --pathspec .pm \
    >"$TMP_DIR/$label.out" 2>"$TMP_DIR/$label.err"; then
    echo "guard-tracked-files.test: expected filesystem drift: $label" >&2
    exit 1
  fi
  grep -F "$expected" "$TMP_DIR/$label.err" >/dev/null
}

new_repo ignored-edit
printf '*.ignored\n' >"$REPO/.gitignore"
IGNORED="$REPO/.pm/cache/state.ignored"
mkdir -p "$(dirname "$IGNORED")"
printf 'ignored baseline\n' >"$IGNORED"
python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" snapshot \
  --root "$REPO" --state "$STATE" --pathspec .pm
printf 'ignored changed\n' >"$IGNORED"
expect_filesystem_drift ignored-edit \
  "filesystem projection lstat/content drift: .pm/cache/state.ignored"

new_repo ignored-removal
printf '*.ignored\n' >"$REPO/.gitignore"
IGNORED="$REPO/.pm/cache/state.ignored"
mkdir -p "$(dirname "$IGNORED")"
printf 'ignored baseline\n' >"$IGNORED"
python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" snapshot \
  --root "$REPO" --state "$STATE" --pathspec .pm
rm "$IGNORED"
expect_filesystem_drift ignored-removal \
  "removed filesystem projection path: .pm/cache/state.ignored"

new_repo ignored-symlink-retarget
printf '*.ignored\n' >"$REPO/.gitignore"
printf 'first\n' >"$REPO/first.txt"
printf 'second\n' >"$REPO/second.txt"
IGNORED="$REPO/.pm/cache/state.ignored"
mkdir -p "$(dirname "$IGNORED")"
ln -s "$REPO/first.txt" "$IGNORED"
python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" snapshot \
  --root "$REPO" --state "$STATE" --pathspec .pm
rm "$IGNORED"
ln -s "$REPO/second.txt" "$IGNORED"
expect_filesystem_drift ignored-symlink-retarget \
  "filesystem projection lstat/content drift: .pm/cache/state.ignored"

new_repo baseline-untracked-edit
BASELINE_UNTRACKED="$REPO/.pm/local/state.yaml"
mkdir -p "$(dirname "$BASELINE_UNTRACKED")"
printf 'baseline untracked\n' >"$BASELINE_UNTRACKED"
python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" snapshot \
  --root "$REPO" --state "$STATE" --pathspec .pm
printf 'changed untracked\n' >"$BASELINE_UNTRACKED"
expect_filesystem_drift baseline-untracked-edit \
  "filesystem projection lstat/content drift: .pm/local/state.yaml"

new_repo baseline-untracked-removal
BASELINE_UNTRACKED="$REPO/.pm/local/state.yaml"
mkdir -p "$(dirname "$BASELINE_UNTRACKED")"
printf 'baseline untracked\n' >"$BASELINE_UNTRACKED"
python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" snapshot \
  --root "$REPO" --state "$STATE" --pathspec .pm
rm "$BASELINE_UNTRACKED"
expect_filesystem_drift baseline-untracked-removal \
  "removed filesystem projection path: .pm/local/state.yaml"

new_repo baseline-untracked-type
printf 'same content\n' >"$REPO/type-target.txt"
BASELINE_UNTRACKED="$REPO/.pm/local/state.yaml"
mkdir -p "$(dirname "$BASELINE_UNTRACKED")"
printf 'same content\n' >"$BASELINE_UNTRACKED"
python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" snapshot \
  --root "$REPO" --state "$STATE" --pathspec .pm
rm "$BASELINE_UNTRACKED"
ln -s "$REPO/type-target.txt" "$BASELINE_UNTRACKED"
expect_filesystem_drift baseline-untracked-type \
  "filesystem projection lstat/content drift: .pm/local/state.yaml"

new_repo baseline-untracked-retarget
printf 'first\n' >"$REPO/first.txt"
printf 'second\n' >"$REPO/second.txt"
BASELINE_UNTRACKED="$REPO/.pm/local/state.yaml"
mkdir -p "$(dirname "$BASELINE_UNTRACKED")"
ln -s "$REPO/first.txt" "$BASELINE_UNTRACKED"
python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" snapshot \
  --root "$REPO" --state "$STATE" --pathspec .pm
rm "$BASELINE_UNTRACKED"
ln -s "$REPO/second.txt" "$BASELINE_UNTRACKED"
expect_filesystem_drift baseline-untracked-retarget \
  "filesystem projection lstat/content drift: .pm/local/state.yaml"

printf 'ignored and baseline-untracked filesystem drift detected\n'

printf 'guard-tracked-files.test: PASS\n'
