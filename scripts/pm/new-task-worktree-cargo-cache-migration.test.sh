#!/usr/bin/env bash
# Contract: resuming a task worktree may replace only its legacy target symlink.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMPDIR="$(mktemp -d)"
REPO="$TMPDIR/repo"
TARGET="$TMPDIR/resumed-worktree"
FAKE_BIN="$TMPDIR/fake-bin"
BRANCH="task/engineering-cache-migration-fixture"
OLD_CACHE="$TMPDIR/.oasis7-cache/cargo-target/git-legacy/rustc-fixture"
PYTHON_BIN="$("$ROOT_DIR/scripts/pm/find-python-with-module.sh" ast)"

cleanup() {
  set +e
  git -C "$REPO" worktree remove --force "$TARGET" >/dev/null 2>&1 || true
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

mkdir -p "$REPO/scripts/pm" "$FAKE_BIN"
cp "$ROOT_DIR/scripts/new-task-worktree.sh" "$REPO/scripts/new-task-worktree.sh"
cp "$ROOT_DIR/scripts/worktree-harness-lib.sh" "$REPO/scripts/worktree-harness-lib.sh"
cp "$ROOT_DIR/scripts/cargo-dev.sh" "$REPO/scripts/cargo-dev.sh"
cp "$ROOT_DIR/scripts/pm/pm_store.py" "$REPO/scripts/pm/pm_store.py"
cat >"$REPO/python-shim" <<'PY'
#!/usr/bin/env python3
import os
import runpy
import sys

arguments = sys.argv[1:]
if not arguments:
    raise SystemExit("python-shim: missing arguments")
sys.argv = arguments
if arguments[0] == "-":
    source = sys.stdin.read()
    if os.environ.get("FAIL_OS_REPLACE") == "1" and "os.replace(" in source:
        def injected_failure(*_args, **_kwargs):
            raise OSError("injected os.replace failure")
        os.replace = injected_failure
    exec(compile(source, "<python-shim>", "exec"), {"__name__": "__main__"})
else:
    runpy.run_path(arguments[0], run_name="__main__")
PY
cat >"$REPO/scripts/pm/find-python-with-module.sh" <<SH
#!/usr/bin/env bash
printf '%s\\n' '$REPO/python-shim'
SH
cat >"$REPO/scripts/pm/terminal-tombstone-guard.py" <<'PY'
raise SystemExit(0)
PY
cat >"$REPO/scripts/pm/loop-bootstrap.py" <<'PY'
#!/usr/bin/env python3
import subprocess
import sys

root = sys.argv[sys.argv.index('--root') + 1]
print(subprocess.check_output(['git', '-C', root, 'rev-parse', 'HEAD'], text=True).strip())
PY
cat >"$REPO/scripts/pm/new-task.sh" <<'SH'
#!/usr/bin/env bash
echo 'intentional post-cache-bootstrap stop: migration fixture does not contact PM' >&2
exit 77
SH
cat >"$FAKE_BIN/cargo" <<'SH'
#!/usr/bin/env bash
printf '%s\n' 'cargo 1.0.0-fixture'
SH
chmod +x "$REPO/scripts/new-task-worktree.sh" "$REPO/scripts/cargo-dev.sh" \
  "$REPO/scripts/pm/find-python-with-module.sh" "$REPO/scripts/pm/loop-bootstrap.py" \
  "$REPO/scripts/pm/new-task.sh" "$REPO/python-shim" "$FAKE_BIN/cargo"

git -C "$REPO" init -q -b main
git -C "$REPO" config user.email test@example.invalid
git -C "$REPO" config user.name 'Cargo cache migration fixture'
printf '%s\n' fixture >"$REPO/source.txt"
git -C "$REPO" add .
git -C "$REPO" commit -qm fixture
git -C "$REPO" worktree add -qb "$BRANCH" "$TARGET" main

mkdir -p "$OLD_CACHE"
printf '%s\n' 'must-survive-migration' >"$OLD_CACHE/sentinel.txt"
ln -s "$OLD_CACHE" "$TARGET/target"
OLD_CACHE_REAL="$("$PYTHON_BIN" - "$OLD_CACHE" <<'PY'
import os
import sys
print(os.path.realpath(sys.argv[1]))
PY
)"

BINDING="$TMPDIR/binding.json"
cat >"$BINDING" <<JSON
{"loop":"code","request_key":"migration-fixture","manual_request_ref":"issue-3867","task_uid":"task_13380b7cf3de4f0ca0dbe567ae48638c","repo":"eng-cc/oasis7","owner_role":"repository_health_engineer","worktree":"$TARGET","branch":"$BRANCH"}
JSON

set +e
(
  cd "$REPO"
  PATH="$FAKE_BIN:$PATH" ./scripts/new-task-worktree.sh engineering cache-migration \
    --branch "$BRANCH" \
    --path "$TARGET" \
    --pm-owner-role repository_health_engineer \
    --pm-title 'cache migration fixture' \
    --pm-source-ref doc/engineering/workflow/source-of-truth.md \
    --pm-acceptance 'legacy target symlink migrates without deleting its cache' \
    --pm-loop code \
    --pm-loop-binding "$BINDING" \
    --pm-request-key migration-fixture \
    --pm-manual-request-ref issue-3867
) >"$TMPDIR/bootstrap.out" 2>"$TMPDIR/bootstrap.err"
rc=$?
set -e
if [[ "$rc" != 77 ]]; then
  echo "expected fixture to stop at the fake PM boundary with exit 77, got $rc" >&2
  cat "$TMPDIR/bootstrap.err" >&2
  exit 1
fi

EXPECTED_TARGET="$(cd "$TARGET" && PATH="$FAKE_BIN:$PATH" ./scripts/cargo-dev.sh --print-target-dir)"
ACTUAL_TARGET="$("$PYTHON_BIN" - "$TARGET/target" <<'PY'
import os
import sys
print(os.path.realpath(sys.argv[1]))
PY
)"
if [[ "$ACTUAL_TARGET" != "$EXPECTED_TARGET" ]]; then
  echo "resume migration did not install the isolated target symlink: $ACTUAL_TARGET != $EXPECTED_TARGET" >&2
  exit 1
fi
if [[ "$ACTUAL_TARGET" == "$OLD_CACHE_REAL" ]]; then
  echo 'resume migration retained the legacy target namespace' >&2
  exit 1
fi
if [[ "$(<"$OLD_CACHE/sentinel.txt")" != must-survive-migration ]]; then
  echo 'resume migration deleted or changed data in the legacy cache' >&2
  exit 1
fi

unlink "$TARGET/target"
ln -s "$OLD_CACHE" "$TARGET/target"
set +e
(
  cd "$REPO"
  FAIL_OS_REPLACE=1 PATH="$FAKE_BIN:$PATH" ./scripts/new-task-worktree.sh engineering cache-migration \
    --branch "$BRANCH" \
    --path "$TARGET" \
    --pm-owner-role repository_health_engineer \
    --pm-title 'cache migration failure fixture' \
    --pm-source-ref doc/engineering/workflow/source-of-truth.md \
    --pm-acceptance 'injected atomic migration failure preserves the old link' \
    --pm-loop code \
    --pm-loop-binding "$BINDING" \
    --pm-request-key migration-fixture \
    --pm-manual-request-ref issue-3867
) >"$TMPDIR/failure.out" 2>"$TMPDIR/failure.err"
failure_rc=$?
set -e
if [[ "$failure_rc" == "0" ]] || ! grep -Fq 'failed to migrate existing target symlink' "$TMPDIR/failure.err"; then
  echo 'injected os.replace failure did not fail at the migration boundary' >&2
  cat "$TMPDIR/failure.err" >&2
  exit 1
fi
FAILURE_TARGET="$("$PYTHON_BIN" - "$TARGET/target" <<'PY'
import os
import sys
print(os.path.realpath(sys.argv[1]))
PY
)"
if [[ "$FAILURE_TARGET" != "$OLD_CACHE_REAL" ]]; then
  echo "migration failure changed the old target link: $FAILURE_TARGET" >&2
  exit 1
fi
if [[ "$(<"$OLD_CACHE/sentinel.txt")" != must-survive-migration ]]; then
  echo 'migration failure changed data in the old cache' >&2
  exit 1
fi
if find "$TARGET" -maxdepth 1 -name 'target.migration.*' -print -quit | grep -q .; then
  echo 'migration failure left a target.migration residue' >&2
  exit 1
fi

echo "new-task-worktree-cargo-cache-migration.test: OK"
