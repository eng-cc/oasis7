#!/usr/bin/env bash
set -euo pipefail
usage(){ cat <<'EOF'
Usage: record-pr-disposition.sh --task-uid <uid> --pr-number <n> --node-id <GitHub node> --kind <comment|review> --head-oid <sha> --disposition <addressed|rejected_with_evidence|non_actionable> --evidence <text> [--repo owner/name]
Writes a machine-readable disposition to the GitHub task issue and emits its verified receipt.
EOF
}
TASK_UID= PR= NODE= KIND= HEAD= DISPOSITION= EVIDENCE= REPO=eng-cc/oasis7
while [[ $# -gt 0 ]]; do case "$1" in --task-uid) TASK_UID="$2";shift 2;; --pr-number) PR="$2";shift 2;; --node-id) NODE="$2";shift 2;; --kind) KIND="$2";shift 2;; --head-oid) HEAD="$2";shift 2;; --disposition) DISPOSITION="$2";shift 2;; --evidence) EVIDENCE="$2";shift 2;; --repo) REPO="$2";shift 2;; -h|--help) usage;exit 0;; *) echo "unknown argument: $1" >&2;exit 2;; esac; done
for value in "$TASK_UID" "$PR" "$NODE" "$KIND" "$HEAD" "$DISPOSITION" "$EVIDENCE"; do [[ -n "$value" ]] || { usage >&2; exit 2; }; done
[[ "$KIND" == comment || "$KIND" == review ]] || { echo "--kind must be comment|review" >&2; exit 2; }
ISSUE="$(gh issue list -R "$REPO" --search "$TASK_UID in:body" --json number --jq 'if length == 1 then .[0].number else error("task issue not unique") end')"
BODY="$(mktemp)"; trap 'rm -f "$BODY"' EXIT
printf '<!-- oasis7-pr-disposition -->\n- task_uid: `%s`\n- repository: `%s`\n- issue_number: `%s`\n- pr_number: `%s`\n- head_oid: `%s`\n- node_id: `%s`\n- kind: `%s`\n- disposition: `%s`\n- evidence: %s\n' "$TASK_UID" "$REPO" "$ISSUE" "$PR" "$HEAD" "$NODE" "$KIND" "$DISPOSITION" "$EVIDENCE" >"$BODY"
URL="$(gh issue comment "$ISSUE" -R "$REPO" --body-file "$BODY")"
# Supported adapter persistence boundary: github-project-task.py record-pr-disposition
COMMENT_ID="${URL##*issuecomment-}"
READBACK="$(gh api "repos/$REPO/issues/comments/$COMMENT_ID")"
python3 - "$TASK_UID" "$REPO" "$ISSUE" "$PR" "$HEAD" "$NODE" "$KIND" "$DISPOSITION" "$READBACK" <<'PY'
import hashlib,json,sys
c=json.loads(sys.argv[9]); body=str(c.get('body') or '')
print(json.dumps({'source':'github_task_issue_comment','runtime_verified':True,
'task_uid':sys.argv[1],'repository':sys.argv[2],'issue_number':int(sys.argv[3]),'pr_number':int(sys.argv[4]),
'head_oid':sys.argv[5],'node_id':sys.argv[6],'kind':sys.argv[7],'disposition':sys.argv[8],'github_node_id':str(c.get('id')),'url':c.get('html_url'),
'author':(c.get('user') or {}).get('login'),'observed_at':c.get('created_at'),
'digest':hashlib.sha256(body.encode()).hexdigest()},sort_keys=True))
PY
