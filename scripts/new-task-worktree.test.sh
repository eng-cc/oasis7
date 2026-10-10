#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
REPO="$TMP/repo"
mkdir -p "$REPO/scripts"
cp "$ROOT/scripts/"{new-task-worktree.sh,worktree-harness-lib.sh,cargo-dev.sh,cargo-cache.py,find-python-with-module.sh} "$REPO/scripts/"
git -C "$REPO" init -q
git -C "$REPO" config user.email test@example.com
git -C "$REPO" config user.name test
printf 'tracked\n' >"$REPO/file"
git -C "$REPO" add .
git -C "$REPO" commit -qm initial
"$REPO/scripts/new-task-worktree.sh" docs isolated --path "$TMP/new" --init-docs --json >"$TMP/result"
python3 - "$TMP/result" "$TMP/new" <<'CHECK'
import json,os,sys
p=json.load(open(sys.argv[1]))
assert p['branch']=='codex/docs-isolated'
assert os.path.realpath(p['worktree_path'])==os.path.realpath(sys.argv[2])
assert 'pm_task' not in p
assert p['head'] and p['setup_status']=='complete'
assert set(p['doc_checks'])=={'prd'}
assert os.path.islink(sys.argv[2]+'/target')
CHECK
printf 'user work\n' >"$TMP/new/file"
if "$REPO/scripts/new-task-worktree.sh" docs isolated --path "$TMP/new" --json >/dev/null 2>&1; then
  echo 'duplicate path unexpectedly accepted' >&2; exit 1
fi
test "$(cat "$TMP/new/file")" = 'user work'
printf 'dirty\n' >"$REPO/file"
if "$REPO/scripts/new-task-worktree.sh" docs dirty --path "$TMP/dirty" >/dev/null 2>&1; then
  echo 'dirty source unexpectedly accepted' >&2; exit 1
fi
test ! -e "$TMP/dirty"
git -C "$REPO" checkout -- file
printf 'canonical config' >"$REPO/config.toml"
printf 'config.toml\n' >>"$REPO/.git/info/exclude"
mkdir -p "$REPO/.git/hooks"
cat >"$REPO/.git/hooks/post-checkout" <<'HOOK'
#!/usr/bin/env bash
printf 'hook user material' > hook-note
printf 'hook commit' > hook-commit
git add hook-commit
git commit -qm hook
HOOK
chmod +x "$REPO/.git/hooks/post-checkout"
# Force a real mkdir failure through the configured cache root.
mkdir -p "$TMP/bin"
REAL_MKDIR="$(command -v mkdir)"
cat >"$TMP/bin/mkdir" <<'MKDIR'
#!/usr/bin/env bash
case "$*" in *cargo-target*) echo 'fixture mkdir failure' >&2; exit 1;; esac
exec "$TEST_REAL_MKDIR" "$@"
MKDIR
chmod +x "$TMP/bin/mkdir"
set +e
TEST_REAL_MKDIR="$REAL_MKDIR" PATH="$TMP/bin:$PATH" "$REPO/scripts/new-task-worktree.sh" docs hook --path "$TMP/hook" --json >"$TMP/failure"
code=$?
set -e
test "$code" = 1
python3 - "$TMP/failure" "$TMP/hook" <<'CHECK'
import json,subprocess,sys
from pathlib import Path
p=json.load(open(sys.argv[1]));w=Path(sys.argv[2])
assert p['worktree_created'] and p['setup_status']=='failed'
assert p['failed_step']=='cache_mkdir'
assert (w/'hook-note').read_text()=='hook user material'
assert subprocess.check_output(['git','-C',str(w),'log','-1','--format=%s'],text=True).strip()=='hook'
CHECK
head="$(git -C "$TMP/hook" rev-parse HEAD)"
printf 'user config' >"$TMP/hook/config.toml"
git -C "$REPO" config --local oasis7.task-worktree-family-name user-family
family_root="$(git -C "$REPO" config --local --get oasis7.task-worktrees-root)"
"$REPO/scripts/new-task-worktree.sh" docs hook --path "$TMP/hook" --resume-setup --worktrees-root "$TMP/alternate" --json >"$TMP/resumed"
test "$(git -C "$REPO" config --local --get oasis7.task-worktree-family-name)" = user-family
test "$(git -C "$REPO" config --local --get oasis7.task-worktrees-root)" = "$family_root"
"$REPO/scripts/new-task-worktree.sh" docs hook --path "$TMP/hook" --resume-setup --json >/dev/null
test "$(git -C "$TMP/hook" rev-parse HEAD)" = "$head"
test "$(cat "$TMP/hook/config.toml")" = 'user config'
test "$(cat "$TMP/hook/hook-note")" = 'hook user material'
rm "$TMP/hook/target"
mkdir "$TMP/hook/target"
printf 'keep' >"$TMP/hook/target/user"
if "$REPO/scripts/new-task-worktree.sh" docs hook --path "$TMP/hook" --resume-setup --json >/dev/null 2>&1; then exit 1; fi
test "$(cat "$TMP/hook/target/user")" = keep
for extra in '--branch codex/wrong' '--with-harness'; do
  if "$REPO/scripts/new-task-worktree.sh" docs hook --path "$TMP/hook" --resume-setup $extra >/dev/null 2>&1; then exit 1; fi
done
echo 'new-task-worktree.test: OK'
