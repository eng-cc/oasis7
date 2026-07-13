#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMPDIR="$(mktemp -d)"; trap 'rm -rf "$TMPDIR"' EXIT
FIXTURE="$TMPDIR/repo"; mkdir -p "$FIXTURE/scripts/pm" "$FIXTURE/.pm/github-project-sync" "$TMPDIR/bin"
cp "$ROOT_DIR/scripts/pm/record-admin-merge-authority.sh" "$FIXTURE/scripts/pm/"
chmod +x "$FIXTURE/scripts/pm/record-admin-merge-authority.sh"
cat >"$FIXTURE/.pm/github-project-sync/tasks.json" <<'JSON'
{"project":{"repo":"eng-cc/oasis7"},"tasks":{"task_11111111111111111111111111111111":{"issue_number":1,"pr_number":9}}}
JSON
cat >"$TMPDIR/bin/gh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
case "$1 $2" in
  "pr view") printf '%s\n' aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa ;;
  "issue comment") body=""; while [[ $# -gt 0 ]]; do [[ "$1" == --body-file ]] && { body="$2"; break; }; shift; done; cp "$body" "$TEST_BODY"; printf 'https://github.com/eng-cc/oasis7/issues/1#issuecomment-77\n' ;;
  "api repos/eng-cc/oasis7/issues/comments/77") cat "$TEST_BODY" ;;
  *) echo "unexpected gh call: $*" >&2; exit 9 ;;
esac
SH
chmod +x "$TMPDIR/bin/gh"
(cd "$FIXTURE" && PATH="$TMPDIR/bin:$PATH" TEST_BODY="$TMPDIR/body" ./scripts/pm/record-admin-merge-authority.sh \
  --task-uid task_11111111111111111111111111111111 --pr-number 9 \
  --requester user --reason 'explicit user authority' --json) >"$TMPDIR/out"
grep -F '"scope": "review_approval_only"' "$TMPDIR/out" >/dev/null
grep -F '<!-- oasis7-admin-merge-authority -->' "$TMPDIR/body" >/dev/null
grep -F -- '- disposition: `authorized`' "$TMPDIR/body" >/dev/null
if (cd "$FIXTURE" && PATH="$TMPDIR/bin:$PATH" TEST_BODY="$TMPDIR/body" ./scripts/pm/record-admin-merge-authority.sh \
  --task-uid task_11111111111111111111111111111111 --pr-number 9 \
  --requester user --reason mismatch --repo other/repo) >"$TMPDIR/mismatch.out" 2>"$TMPDIR/mismatch.err"; then
  echo 'expected mismatched --repo to fail' >&2
  exit 1
fi
grep -F -- '--repo does not match task truth' "$TMPDIR/mismatch.err" >/dev/null
echo 'record-admin-merge-authority.test: OK'
