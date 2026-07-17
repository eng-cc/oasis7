#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: record-admin-merge-authority.sh --task-uid <uid> --pr-number <n> --requester <identity> --reason <text> [--repo owner/name] [--json]

Optionally record and read back a GitHub task-issue audit note for the default
review-approval-only repository admin merge path. Repository standing policy,
not this note, selects that path. The note binds the live PR head and never
authorizes bypassing another gate.
USAGE
}

TASK_UID=""; PR_NUMBER=""; REQUESTER=""; REASON=""; REPO=""; JSON=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --task-uid) TASK_UID="${2:-}"; shift 2 ;;
    --pr-number) PR_NUMBER="${2:-}"; shift 2 ;;
    --requester) REQUESTER="${2:-}"; shift 2 ;;
    --reason) REASON="${2:-}"; shift 2 ;;
    --repo) REPO="${2:-}"; shift 2 ;;
    --json) JSON=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "error: unknown argument: $1" >&2; exit 2 ;;
  esac
done
[[ -n "$TASK_UID" && -n "$PR_NUMBER" && -n "$REQUESTER" && -n "$REASON" ]] || { usage >&2; exit 2; }

eval "$(python3 - "$TASK_UID" <<'PY'
import json, shlex, sys
from pathlib import Path
uid=sys.argv[1]
p=json.loads(Path('.pm/github-project-sync/tasks.json').read_text(encoding='utf-8'))
r=(p.get('tasks') or {}).get(uid) or {}
print('MAPPED_REPO='+shlex.quote(str((p.get('project') or {}).get('repo') or 'eng-cc/oasis7')))
print('ISSUE_NUMBER='+shlex.quote(str(r.get('issue_number') or '')))
print('RECORDED_PR='+shlex.quote(str(r.get('pr_number') or '')))
PY
)"
if [[ -n "$REPO" && "$REPO" != "$MAPPED_REPO" ]]; then
  echo "error: --repo does not match task truth" >&2
  exit 3
fi
REPO="$MAPPED_REPO"
[[ -n "$ISSUE_NUMBER" ]] || { echo "error: task has no GitHub issue mapping" >&2; exit 3; }
[[ "$RECORDED_PR" == "$PR_NUMBER" ]] || { echo "error: PR does not match task truth" >&2; exit 3; }

HEAD_OID="$(gh pr view "$PR_NUMBER" -R "$REPO" --json headRefOid --jq .headRefOid)"
[[ "$HEAD_OID" =~ ^[0-9a-f]{40}$ ]] || { echo "error: live PR head readback is invalid" >&2; exit 3; }
BODY="$(mktemp)"; trap 'rm -f "$BODY"' EXIT
printf '<!-- oasis7-admin-merge-authority -->\n- task_uid: `%s`\n- repository: `%s`\n- issue_number: `%s`\n- pr_number: `%s`\n- head_oid: `%s`\n- node_id: `admin_merge_authority`\n- kind: `admin_merge_authority`\n- disposition: `authorized`\n- requester: `%s`\n- scope: `review_approval_only`\n- reason: `%s`\n' \
  "$TASK_UID" "$REPO" "$ISSUE_NUMBER" "$PR_NUMBER" "$HEAD_OID" "$REQUESTER" "$REASON" >"$BODY"
COMMENT_URL="$(gh issue comment "$ISSUE_NUMBER" -R "$REPO" --body-file "$BODY")"
COMMENT_ID="${COMMENT_URL##*issuecomment-}"
[[ "$COMMENT_ID" =~ ^[0-9]+$ ]] || { echo "error: comment URL readback is invalid" >&2; exit 3; }
READ_BODY="$(gh api "repos/$REPO/issues/comments/$COMMENT_ID" --jq .body)"
[[ "$READ_BODY" == "$(cat "$BODY")" ]] || { echo "error: authority comment readback mismatch" >&2; exit 3; }

if [[ "$JSON" == 1 ]]; then
  python3 - "$TASK_UID" "$PR_NUMBER" "$HEAD_OID" "$COMMENT_URL" <<'PY'
import json,sys
print(json.dumps({'task_uid':sys.argv[1],'pr_number':int(sys.argv[2]),'head_oid':sys.argv[3],'comment_url':sys.argv[4],'scope':'review_approval_only'},sort_keys=True))
PY
else
  printf '%s\n' "$COMMENT_URL"
fi
