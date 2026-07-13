#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMPDIR="$(mktemp -d)"
export OASIS7_PM_TEST_SCRATCH="${OASIS7_PM_TEST_SCRATCH:-$TMPDIR}"
trap 'rm -rf "$TMPDIR"' EXIT
mkdir -p "$TMPDIR/bin"

FIXTURE_ROOT="$TMPDIR/repo"
git init -q -b main "$FIXTURE_ROOT"
git -C "$FIXTURE_ROOT" config user.name "oasis7 smoke"
git -C "$FIXTURE_ROOT" config user.email "smoke@example.invalid"
printf 'fixture\n' >"$FIXTURE_ROOT/README.md"
git -C "$FIXTURE_ROOT" add README.md
git -C "$FIXTURE_ROOT" commit -qm "initial fixture"

REPO=eng-cc/oasis7
TITLE='immutable bootstrap fixture'
OWNER=tpm
WORKTREE="$FIXTURE_ROOT"
KEY="$(python3 - "$REPO" "$TITLE" "$OWNER" "$WORKTREE" <<'PY'
import hashlib,sys
print(hashlib.sha256('\0'.join(sys.argv[1:]).encode()).hexdigest())
PY
)"
JOURNAL_PATH="$OASIS7_PM_TEST_SCRATCH/bootstrap-journal/$KEY.json"
mkdir -p "$(dirname "$JOURNAL_PATH")"
cat >"$JOURNAL_PATH" <<JSON
{"version":1,"task_uid":"task_11111111111111111111111111111111","state":"planned","next_action":"create_issue","request":{"repo":"$REPO","title":"$TITLE","owner_role":"$OWNER","worktree_hint":"$WORKTREE","module":"engineering","priority":"P2","source_refs":["doc/engineering/project.md"],"acceptance":["original acceptance"],"source_signal":"signal-old","source_type":"reflection","severity":"medium","doc_refs":["doc/old.md"],"related_prd":["prd-old"],"handoff_to":["qa_engineer"]}}
JSON
cat >"$TMPDIR/bin/gh" <<'SH'
#!/usr/bin/env bash
touch "${GH_CALLED:?}"
echo "unexpected GitHub call" >&2
exit 73
SH
chmod +x "$TMPDIR/bin/gh"

set +e
GH_CALLED="$TMPDIR/gh-called" PATH="$TMPDIR/bin:$PATH" python3 "$ROOT_DIR/scripts/pm/github-project-task.py" new-task "$FIXTURE_ROOT" \
  --repo "$REPO" --project-owner eng-cc --project-number 1 \
  --owner-role "$OWNER" --title "$TITLE" --module engineering --priority P2 \
  --source-ref doc/engineering/project.md --acceptance 'original acceptance' \
  --source-signal signal-new --source-type bug --severity high \
  --doc-ref doc/new.md --related-prd prd-new --handoff-to repository_health_engineer \
  --worktree-hint "$WORKTREE" --json >"$TMPDIR/out" 2>"$TMPDIR/err"
status=$?
set -e
[[ "$status" -ne 0 ]] || { echo "RED bootstrap-immutable-request: drifted retry was accepted" >&2; exit 1; }
[[ ! -e "$TMPDIR/gh-called" ]] || { echo "RED bootstrap-immutable-request: extended metadata drift reached GitHub" >&2; exit 1; }
grep -Eiq 'immutable request|request drift|source.signal|source.type|severity|doc.ref|related.prd|handoff' "$TMPDIR/err" || {
  echo "RED bootstrap-immutable-request: retry did not reject extended immutable metadata drift" >&2
  exit 1
}

echo "bootstrap-immutable-request.test: OK"
