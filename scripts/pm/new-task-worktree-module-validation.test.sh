#!/usr/bin/env bash
# Cross-platform test contract: fixture bootstrap remains valid on Windows Git Bash and Linux/macOS shells.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

REPO="$TMPDIR/repo"
TARGET="$TMPDIR/task-worktree"
PM_BOOTSTRAP_MARKER="$TMPDIR/pm-bootstrap-invoked"
mkdir -p "$REPO/scripts/pm"
cp "$ROOT_DIR/scripts/new-task-worktree.sh" "$REPO/scripts/new-task-worktree.sh"
cp "$ROOT_DIR/scripts/worktree-harness-lib.sh" "$REPO/scripts/worktree-harness-lib.sh"
cp "$ROOT_DIR/scripts/pm/find-python-with-module.sh" "$REPO/scripts/pm/find-python-with-module.sh"
cp "$ROOT_DIR/scripts/pm/pm_store.py" "$REPO/scripts/pm/pm_store.py"
chmod +x "$REPO/scripts/new-task-worktree.sh"

cat >"$REPO/scripts/cargo-dev.sh" <<'EOF'
#!/usr/bin/env bash
[[ "${1:-}" == "--print-target-dir" ]]
printf '%s\n' "${TEST_SHARED_TARGET:?}"
EOF
cat >"$REPO/scripts/pm/new-task.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
: >"${PM_BOOTSTRAP_MARKER:?}"
echo "PM bootstrap must not run for an unsupported module" >&2
exit 73
EOF
chmod +x "$REPO/scripts/cargo-dev.sh" "$REPO/scripts/pm/new-task.sh"

git -C "$REPO" init -q -b main
git -C "$REPO" config user.email test@example.invalid
git -C "$REPO" config user.name Test
git -C "$REPO" add .
git -C "$REPO" commit -qm fixture

set +e
(
  cd "$REPO"
  PM_BOOTSTRAP_MARKER="$PM_BOOTSTRAP_MARKER" TEST_SHARED_TARGET="$TMPDIR/shared-target" \
    ./scripts/new-task-worktree.sh game-design invalid-project-module \
      --branch task/invalid-project-module \
      --path "$TARGET" \
      --pm-owner-role tpm \
      --pm-title "invalid Project module fixture" \
      --pm-source-ref doc/engineering/project.md
) >"$TMPDIR/bootstrap.out" 2>"$TMPDIR/bootstrap.err"
status=$?
set -e

failures=0
if [[ "$status" -eq 0 ]]; then
  echo "expected unsupported PM Module to exit nonzero" >&2
  failures=$((failures + 1))
fi
if [[ -e "$TARGET" ]]; then
  echo "unsupported PM Module created a worktree before validation: $TARGET" >&2
  failures=$((failures + 1))
fi
if [[ -e "$PM_BOOTSTRAP_MARKER" ]]; then
  echo "unsupported PM Module invoked the PM/GitHub bootstrap before validation" >&2
  failures=$((failures + 1))
fi
if git -C "$REPO" show-ref --verify --quiet refs/heads/task/invalid-project-module; then
  echo "unsupported PM Module created the requested branch before validation" >&2
  failures=$((failures + 1))
fi
while IFS= read -r supported_module; do
  if ! grep -Fq "$supported_module" "$TMPDIR/bootstrap.err"; then
    echo "unsupported Module error did not list supported value: $supported_module" >&2
    failures=$((failures + 1))
  fi
done < <(python3 - "$ROOT_DIR/scripts/pm/pm_store.py" <<'PY'
import ast
import pathlib
import sys

tree = ast.parse(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
for node in tree.body:
    if isinstance(node, ast.Assign) and any(
        isinstance(target, ast.Name) and target.id == "TASK_MODULE_VALUES"
        for target in node.targets
    ):
        print("\n".join(sorted(ast.literal_eval(node.value))))
        raise SystemExit(0)
raise SystemExit("TASK_MODULE_VALUES is missing")
PY
)

if [[ "$failures" -ne 0 ]]; then
  echo "new-task-worktree module validation regression: $failures assertion(s) failed" >&2
  echo "command stderr:" >&2
  cat "$TMPDIR/bootstrap.err" >&2
  exit 1
fi

echo "new-task-worktree-module-validation.test: OK"
