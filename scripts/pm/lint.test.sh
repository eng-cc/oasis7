#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-pm-lint-test.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT
ignored_before="$(find "$ROOT_DIR/scripts/pm" \( -type d -name __pycache__ -o -type f -name '*.pyc' \) -print | sort)"

python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" snapshot \
  --root "$ROOT_DIR" --state "$TMP_DIR/state" --pathspec .pm

output="$($ROOT_DIR/scripts/pm/lint.sh)"
grep -Fx "pm-lint: OK" <<<"$output" >/dev/null

python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" check \
  --root "$ROOT_DIR" --state "$TMP_DIR/state" --pathspec .pm >/dev/null

FIXTURE="$TMP_DIR/concurrent-fixture"
mkdir -p "$FIXTURE"
cp -R "$ROOT_DIR/.pm" "$FIXTURE/.pm"
shopt -s dotglob nullglob
for path in "$ROOT_DIR"/*; do
  name="$(basename "$path")"
  [[ "$name" == ".pm" || "$name" == ".git" ]] && continue
  if [[ "$name" == ".gitignore" ]]; then
    cp "$path" "$FIXTURE/$name"
    continue
  fi
  ln -s "$path" "$FIXTURE/$name"
done
shopt -u dotglob nullglob
git -C "$FIXTURE" init -q
git -C "$FIXTURE" add .pm

python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" snapshot \
  --root "$FIXTURE" --state "$TMP_DIR/concurrent-state" --pathspec .pm

READY="$TMP_DIR/snapshot-ready"
CONTINUE="$TMP_DIR/snapshot-continue"
PM_ROOT_DIR="$FIXTURE" \
PM_LINT_SNAPSHOT_READY_FILE="$READY" \
PM_LINT_CONTINUE_FILE="$CONTINUE" \
  "$ROOT_DIR/scripts/pm/lint.sh" >"$TMP_DIR/concurrent-lint.out" \
  2>"$TMP_DIR/concurrent-lint.err" &
lint_pid=$!
for _ in {1..500}; do
  [[ -f "$READY" ]] && break
  sleep 0.01
done
if [[ ! -f "$READY" ]]; then
  echo "pm-lint.test: lint snapshot readiness hook timed out" >&2
  kill "$lint_pid" 2>/dev/null || true
  wait "$lint_pid" 2>/dev/null || true
  exit 1
fi
printf '\n# concurrent source epoch mutation\n' >>"$FIXTURE/.pm/stage/current.yaml"
: >"$CONTINUE"
wait "$lint_pid"
grep -Fx "pm-lint: OK" "$TMP_DIR/concurrent-lint.out" >/dev/null

if python3 "$ROOT_DIR/scripts/pm/guard-tracked-files.py" check \
  --root "$FIXTURE" --state "$TMP_DIR/concurrent-state" --pathspec .pm \
  >"$TMP_DIR/concurrent-guard.out" 2>"$TMP_DIR/concurrent-guard.err"; then
  echo "pm-lint.test: expected full-.pm guard to detect non-role source mutation" >&2
  exit 1
fi
grep -F "tracked projection drift: .pm/stage/current.yaml" \
  "$TMP_DIR/concurrent-guard.err" >/dev/null

COPY_FIXTURE="$TMP_DIR/during-copy-fixture"
mkdir -p "$COPY_FIXTURE"
cp -R "$ROOT_DIR/.pm" "$COPY_FIXTURE/.pm"
shopt -s dotglob nullglob
for path in "$ROOT_DIR"/*; do
  name="$(basename "$path")"
  [[ "$name" == ".pm" || "$name" == ".git" ]] && continue
  if [[ "$name" == ".gitignore" ]]; then
    cp "$path" "$COPY_FIXTURE/$name"
    continue
  fi
  ln -s "$path" "$COPY_FIXTURE/$name"
done
shopt -u dotglob nullglob

COPY_READY="$TMP_DIR/copy-ready"
COPY_CONTINUE="$TMP_DIR/copy-continue"
PM_ROOT_DIR="$COPY_FIXTURE" \
PM_LINT_COPY_MAX_ATTEMPTS=2 \
PM_LINT_COPY_READY_FILE="$COPY_READY" \
PM_LINT_COPY_CONTINUE_FILE="$COPY_CONTINUE" \
  "$ROOT_DIR/scripts/pm/lint.sh" >"$TMP_DIR/copy-lint.out" \
  2>"$TMP_DIR/copy-lint.err" &
copy_lint_pid=$!
for _ in {1..500}; do
  [[ -f "$COPY_READY" ]] && break
  sleep 0.01
done
if [[ ! -f "$COPY_READY" ]]; then
  echo "pm-lint.test: during-copy readiness hook timed out" >&2
  kill "$copy_lint_pid" 2>/dev/null || true
  wait "$copy_lint_pid" 2>/dev/null || true
  exit 1
fi
chmod +x "$COPY_FIXTURE/.pm/stage/gate.yaml"
: >"$COPY_CONTINUE"
if ! wait "$copy_lint_pid"; then
  echo "pm-lint.test: retry-success fixture unexpectedly failed" >&2
  cat "$TMP_DIR/copy-lint.err" >&2
  exit 1
fi
grep -F "source .pm changed during snapshot attempt 1/2" \
  "$TMP_DIR/copy-lint.err" >/dev/null
grep -Fx "pm-lint: OK" "$TMP_DIR/copy-lint.out" >/dev/null

FAIL_FIXTURE="$TMP_DIR/during-copy-fail-fixture"
cp -R "$COPY_FIXTURE" "$FAIL_FIXTURE"
FAIL_READY="$TMP_DIR/copy-fail-ready"
FAIL_CONTINUE="$TMP_DIR/copy-fail-continue"
PM_ROOT_DIR="$FAIL_FIXTURE" \
PM_LINT_COPY_MAX_ATTEMPTS=1 \
PM_LINT_COPY_READY_FILE="$FAIL_READY" \
PM_LINT_COPY_CONTINUE_FILE="$FAIL_CONTINUE" \
  "$ROOT_DIR/scripts/pm/lint.sh" >"$TMP_DIR/copy-fail-lint.out" \
  2>"$TMP_DIR/copy-fail-lint.err" &
copy_fail_pid=$!
for _ in {1..500}; do
  [[ -f "$FAIL_READY" ]] && break
  sleep 0.01
done
if [[ ! -f "$FAIL_READY" ]]; then
  echo "pm-lint.test: during-copy failure readiness hook timed out" >&2
  kill "$copy_fail_pid" 2>/dev/null || true
  wait "$copy_fail_pid" 2>/dev/null || true
  exit 1
fi
chmod -x "$FAIL_FIXTURE/.pm/stage/gate.yaml"
: >"$FAIL_CONTINUE"
if wait "$copy_fail_pid"; then
  echo "pm-lint.test: expected inconsistent source-copy epoch rejection" >&2
  exit 1
fi
grep -F "source .pm changed during snapshot attempt 1/1" \
  "$TMP_DIR/copy-fail-lint.err" >/dev/null
grep -F "FAIL: could not capture a coherent .pm snapshot" \
  "$TMP_DIR/copy-fail-lint.err" >/dev/null

for symlink_kind in ignored untracked; do
  SYMLINK_FIXTURE="$TMP_DIR/${symlink_kind}-symlink-fixture"
  mkdir -p "$SYMLINK_FIXTURE"
  cp -R "$ROOT_DIR/.pm" "$SYMLINK_FIXTURE/.pm"
  printf 'target\n' >"$SYMLINK_FIXTURE/target.txt"
  if [[ "$symlink_kind" == "ignored" ]]; then
    printf '*.ignored\n' >"$SYMLINK_FIXTURE/.gitignore"
    symlink_path="$SYMLINK_FIXTURE/.pm/cache/source.ignored"
  else
    symlink_path="$SYMLINK_FIXTURE/.pm/local/source-link"
  fi
  mkdir -p "$(dirname "$symlink_path")"
  ln -s "$SYMLINK_FIXTURE/target.txt" "$symlink_path"
  git -C "$SYMLINK_FIXTURE" init -q
  if [[ "$symlink_kind" == "ignored" ]]; then
    git -C "$SYMLINK_FIXTURE" check-ignore -q ".pm/cache/source.ignored"
  else
    git -C "$SYMLINK_FIXTURE" ls-files --others --exclude-standard -- \
      ".pm/local/source-link" | grep -Fx ".pm/local/source-link" >/dev/null
  fi
  if PM_ROOT_DIR="$SYMLINK_FIXTURE" "$ROOT_DIR/scripts/pm/lint.sh" \
    >"$TMP_DIR/${symlink_kind}-symlink.out" \
    2>"$TMP_DIR/${symlink_kind}-symlink.err"; then
    echo "pm-lint.test: expected source .pm $symlink_kind symlink rejection" >&2
    exit 1
  fi
  grep -F "symlinks are forbidden in governed .pm" \
    "$TMP_DIR/${symlink_kind}-symlink.err" >/dev/null
done

ROOT_LINK_FIXTURE="$TMP_DIR/root-pm-symlink-fixture"
ROOT_LINK_TARGET="$TMP_DIR/root-pm-symlink-target"
mkdir -p "$ROOT_LINK_FIXTURE" "$ROOT_LINK_TARGET"
cp -R "$ROOT_DIR/.pm" "$ROOT_LINK_TARGET/.pm-real"
ln -s "$ROOT_LINK_TARGET/.pm-real" "$ROOT_LINK_FIXTURE/.pm"
if "$ROOT_DIR/scripts/pm/tree-manifest.py" \
  --root "$ROOT_LINK_FIXTURE/.pm" --reject-symlinks \
  >"$TMP_DIR/root-pm-tree.out" 2>"$TMP_DIR/root-pm-tree.err"; then
  echo "pm-lint.test: expected tree-manifest root .pm symlink rejection" >&2
  exit 1
fi
grep -F "root path is a forbidden symlink" "$TMP_DIR/root-pm-tree.err" >/dev/null
if PM_ROOT_DIR="$ROOT_LINK_FIXTURE" "$ROOT_DIR/scripts/pm/lint.sh" \
  >"$TMP_DIR/root-pm-lint.out" 2>"$TMP_DIR/root-pm-lint.err"; then
  echo "pm-lint.test: expected lint root .pm symlink rejection" >&2
  exit 1
fi
grep -F "root path is a forbidden symlink" "$TMP_DIR/root-pm-lint.err" >/dev/null

ignored_after="$(find "$ROOT_DIR/scripts/pm" \( -type d -name __pycache__ -o -type f -name '*.pyc' \) -print | sort)"
if [[ "$ignored_before" != "$ignored_after" ]]; then
  echo "pm-lint.test: canonical scripts/pm ignored Python artifacts changed" >&2
  diff -u <(printf '%s\n' "$ignored_before") <(printf '%s\n' "$ignored_after") >&2 || true
  exit 1
fi

printf 'pm-lint coherent copy/snapshot, full-.pm immutability, and pycache isolation: PASS\n'
