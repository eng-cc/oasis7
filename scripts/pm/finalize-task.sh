#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

usage() {
  cat <<'EOF'
Usage: ./scripts/pm/finalize-task.sh --task-uid <uid> --pr <number> [options]

Run the canonical merged-PR terminal lifecycle as one resumable, fail-closed
operation. Existing receipt validators and crash journals remain authoritative.

Options:
  --task-uid <uid>                  Bound task UID
  --pr <number>                     Bound merged pull request
  --repo-root <path>                Canonical default worktree (default: current repository root)
  --preflight                       Validate terminal identity without mutating state
  --cleanup=defer                   Complete delivery and defer this invocation's cleanup attempt
  --cleanup-only                    Retry resources after v2 delivery was read back
  --patch-equivalence-receipt <p>  Reuse an existing canonical squash/rebase proof
  --resume                          Resume the same durable task/PR identity (default behavior)
  --json                            Print a machine-readable result
  -h, --help                        Show this help
EOF
}

fail() { echo "finalize-task: $*" >&2; exit 1; }
task_uid="" pr_number="" repo_root="$ROOT_DIR" supplied_patch="" resume=0 preflight=0 output_json=0 cleanup_defer=0 cleanup_only=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --task-uid) task_uid="${2:-}"; shift 2 ;;
    --pr) pr_number="${2:-}"; shift 2 ;;
    --repo-root) repo_root="${2:-}"; shift 2 ;;
    --patch-equivalence-receipt) supplied_patch="${2:-}"; shift 2 ;;
    --preflight) preflight=1; shift ;;
    --cleanup=defer) cleanup_defer=1; shift ;;
    --cleanup-only) cleanup_only=1; shift ;;
    --resume) resume=1; shift ;;
    --json) output_json=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) fail "unknown argument: $1" ;;
  esac
done
[[ "$task_uid" =~ ^task_[0-9a-f]{32}$ ]] || fail "invalid --task-uid"
[[ "$pr_number" =~ ^[1-9][0-9]*$ ]] || fail "invalid --pr"
[[ "$cleanup_defer" == 0 || "$cleanup_only" == 0 ]] || fail "--cleanup=defer and --cleanup-only are mutually exclusive"
[[ "$preflight" == 0 || "$cleanup_only" == 0 ]] || fail "--preflight cannot be combined with --cleanup-only"
repo_root="$(git -C "$repo_root" rev-parse --show-toplevel)" || fail "invalid --repo-root"
SCRIPT_DIR="$repo_root/scripts/pm"
[[ -x "$SCRIPT_DIR/finalize-task.sh" ]] || fail "--repo-root does not contain the terminal orchestrator"
mapping="$repo_root/.pm/github-project-sync/tasks.json"
[[ -f "$mapping" ]] || fail "canonical task mapping is unavailable"
receipt_root_args=(--default-worktree "$repo_root" --task-uid "$task_uid")
receipt_root="$(python3 "$SCRIPT_DIR/canonical-receipt-root.py" "${receipt_root_args[@]}")" \
  || fail "cannot resolve canonical receipt root"
merge_receipt="$receipt_root/merge-receipt.json"
terminal_receipt="$receipt_root/terminal-cleanup-receipt.json"
protocol_selector="$(python3 - "$mapping" "$task_uid" <<'PY'
import hashlib,json,pathlib,re,sys
record=(json.loads(pathlib.Path(sys.argv[1]).read_text(encoding='utf-8')).get('tasks') or {}).get(sys.argv[2]) or {}
types=record.get('phase_receipt_type') or {}; digests=record.get('phase_receipt_sha256') or {}
v2_type=types.get('post_merge_done')
digest=digests.get('post_merge_done')
if v2_type=='oasis7_terminal_delivery':
    print('v2' if isinstance(digest,str) and re.fullmatch(r'[0-9a-f]{64}',digest) else 'conflict')
    raise SystemExit(0)
legacy=((record.get('phase_receipts') or {}).get('post_merge_done') or {})
if legacy.get('receipt_type')=='oasis7_terminal_cleanup':
    print('v1' if isinstance(digest,str) and re.fullmatch(r'[0-9a-f]{64}',digest) else 'conflict')
else:
    print('new')
PY
)" || fail "cannot read terminal protocol selector"
[[ "$protocol_selector" != conflict ]] || fail "terminal protocol selector is malformed"
allow_missing_task_worktree=0
[[ "$protocol_selector" == v1 || "$protocol_selector" == v2 || "$cleanup_only" == 1 ]] && allow_missing_task_worktree=1
identity_json="$(python3 - "$repo_root" "$mapping" "$task_uid" "$pr_number" "$allow_missing_task_worktree" <<'PY'
import json
import pathlib
import re
import subprocess
import sys

repo_root, mapping_path, task_uid, pr_number, allow_missing_worktree = sys.argv[1:]
root = pathlib.Path(repo_root).resolve()
record = (json.loads(pathlib.Path(mapping_path).read_text(encoding="utf-8")).get("tasks") or {}).get(task_uid) or {}
bound = {
    "task_uid": str(record.get("task_uid") or ""),
    "pr_number": str(record.get("pr_number") or ""),
    "repository": str(record.get("repository") or ""),
    "issue_number": str(record.get("issue_number") or ""),
    "issue_url": str(record.get("issue_url") or ""),
    "pr_url": str(record.get("pr_url") or ""),
    "canonical_worktree": str(record.get("canonical_worktree") or ""),
    "task_branch": str(record.get("task_branch") or ""),
    "default_branch": str(record.get("default_branch") or ""),
    "owner_role": str(record.get("owner_role") or ""),
}
blockers = []

def blocker(message):
    if message not in blockers:
        blockers.append(message)

if not record:
    blocker("task identity: task UID is absent from canonical mapping")
if bound["task_uid"] != task_uid:
    blocker("task identity: task UID mismatch")
if bound["pr_number"] != pr_number:
    blocker("task/PR mismatch: mapping PR does not match requested PR")
for key in ("issue_number", "issue_url", "pr_url", "canonical_worktree", "task_branch", "default_branch", "owner_role", "repository"):
    if not bound[key]:
        blocker(f"task identity: task truth missing {key}")
if bound["repository"] and not re.fullmatch(r"[^/\s]+/[^/\s]+", bound["repository"]):
    blocker("repository mismatch: task repository identity is malformed")

def github_object_identity(url, kind):
    if not url:
        return None
    match = re.search(
        rf"^https?://github\.com/([^/\s]+/[^/\s?#]+)/{kind}/(\d+)(?:$|[?#])",
        url,
        re.IGNORECASE,
    )
    return (match.group(1), match.group(2)) if match else None

issue_identity = github_object_identity(bound["issue_url"], "issues")
if not issue_identity:
    blocker("task identity: Issue URL is malformed or unsupported (GitHub Issue URL required)")
elif issue_identity[0] != bound["repository"]:
    blocker("repository mismatch: task Issue URL belongs to a different repository")
elif issue_identity[1] != bound["issue_number"]:
    blocker("task identity: Issue URL does not match issue number")
pr_identity = github_object_identity(bound["pr_url"], "pulls?")
if bound["pr_url"]:
    if not pr_identity:
        blocker("task identity: PR URL is malformed or unsupported (GitHub PR URL required)")
    pr_match = re.search(r"/pulls?/(\d+)(?:$|[?#])", bound["pr_url"])
    if not pr_match or pr_match.group(1) != pr_number:
        blocker("task/PR mismatch: task PR URL does not match requested PR")
if pr_identity and pr_identity[0] != bound["repository"]:
    blocker("repository mismatch: task PR URL belongs to a different repository")

def git(path, *args):
    try:
        return subprocess.check_output(["git", "-C", str(path), *args], text=True, stderr=subprocess.DEVNULL).strip()
    except (OSError, subprocess.CalledProcessError):
        return ""

origin = git(root, "config", "--get", "remote.origin.url")
origin_match = re.fullmatch(
    r"(?:https?://|ssh://git@|git@)github\.com[:/]([^/\s?#]+/[^/\s?#]+?)(?:\.git)?/?",
    origin.strip(),
    re.IGNORECASE,
)
if not origin_match:
    blocker("repository mismatch: local origin identity is unavailable or unsupported (GitHub origin required)")
elif origin_match.group(1) != bound["repository"]:
    blocker("repository mismatch: task repository differs from the local origin")

def common_dir(path):
    raw = git(path, "rev-parse", "--git-common-dir")
    if not raw:
        return ""
    return str((path / raw if not pathlib.Path(raw).is_absolute() else pathlib.Path(raw)).resolve())

task_path = pathlib.Path(bound["canonical_worktree"]).expanduser()
if allow_missing_worktree == "1":
    pass
elif not task_path.exists():
    blocker("worktree mismatch: canonical task worktree is missing")
else:
    task_path = task_path.resolve()
    root_common = common_dir(root)
    task_common = common_dir(task_path)
    if not task_common:
        blocker("repository mismatch: canonical task worktree is not a Git worktree")
    elif root_common and task_common != root_common:
        blocker("repository mismatch: task worktree belongs to a different Git repository")

registered = {}
current_path = ""
for line in (git(root, "worktree", "list", "--porcelain") + "\n").splitlines():
    if line.startswith("worktree "):
        current_path = str(pathlib.Path(line[9:]).resolve())
    elif line.startswith("branch refs/heads/") and current_path:
        registered[current_path] = line.removeprefix("branch refs/heads/")
    elif not line:
        current_path = ""
if allow_missing_worktree != "1" and task_path.exists() and str(task_path) not in registered:
    blocker("worktree mismatch: canonical task worktree is detached or unregistered")
if allow_missing_worktree != "1" and task_path.exists():
    actual_branch = git(task_path, "symbolic-ref", "--short", "HEAD")
    if actual_branch != bound["task_branch"]:
        blocker(f"branch mismatch: task worktree is {actual_branch or 'detached'}, expected {bound['task_branch']}")
actual_default = git(root, "symbolic-ref", "--short", "HEAD")
if not actual_default:
    blocker("branch mismatch: default worktree is detached")
elif actual_default != bound["default_branch"]:
    blocker(f"branch mismatch: default worktree is {actual_default}, expected {bound['default_branch']}")

payload = {
    "status": "ready" if not blockers else "blocked",
    "identity_status": "bound" if not blockers else "blocked",
    **bound,
    "pr_number": int(pr_number),
    "repo_root": str(root),
    "blockers": blockers,
    "next_command": ([] if blockers else [
        "./scripts/pm/finalize-task.sh", "--repo-root", str(root), "--task-uid", task_uid,
        "--pr", pr_number, "--resume", "--json",
    ]),
}
print(json.dumps(payload, sort_keys=True))
PY
)"
identity_status="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["status"])' <<<"$identity_json")"
if [[ "$identity_status" != "ready" ]]; then
  if [[ "$output_json" == 1 ]]; then
    python3 - "$identity_json" <<'PY'
import json,sys
p=json.loads(sys.argv[1]); blockers=p.get('blockers',[])
print(json.dumps({**p,"delivery_blockers":blockers,"cleanup_blockers":[]},sort_keys=True))
PY
  else
    python3 -c 'import json,sys; print("finalize-task: " + "; ".join(json.load(sys.stdin)["blockers"]))' <<<"$identity_json" >&2
  fi
  exit 1
fi

task_worktree="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["canonical_worktree"])' <<<"$identity_json")"
task_branch="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["task_branch"])' <<<"$identity_json")"
owner_role="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["owner_role"])' <<<"$identity_json")"

if [[ "$preflight" == 1 ]]; then
  # Unselected preflight is the original mutation-free premerge identity check.
  # A postmerge proof cannot exist while that reciprocal PR is still OPEN.
  # Explicit delivered-v2 preflight instead revalidates its required proof.
  if [[ "$protocol_selector" == v2 ]]; then
    python3 "$SCRIPT_DIR/readiness_transport.py" --repo-root "$repo_root" --task-uid "$task_uid" >/dev/null \
      || fail "readiness read-only preflight failed"
  fi
  cleanup_blockers='[]'
  if [[ "$protocol_selector" == v2 ]]; then
    cleanup_preflight_rc=0
    if cleanup_preflight="$($SCRIPT_DIR/post-merge-cleanup.sh --repo-root "$repo_root" --task-uid "$task_uid" --delivery --preflight --json)"; then
      :
    else
      cleanup_preflight_rc=$?
    fi
    cleanup_blockers="$(python3 - "$cleanup_preflight" "$cleanup_preflight_rc" <<'PY'
import json,sys
raw, returncode = sys.argv[1], int(sys.argv[2])
blockers = []
if not raw.strip():
    blockers.append("cleanup preflight helper returned no result")
else:
    try:
        result = json.loads(raw)
    except (json.JSONDecodeError, TypeError):
        blockers.append("cleanup preflight helper returned invalid JSON")
    else:
        values = result.get("cleanup_blockers") if isinstance(result, dict) else None
        if not isinstance(values, list) or any(not isinstance(value, str) or not value for value in values):
            blockers.append("cleanup preflight helper returned invalid cleanup blockers")
        else:
            blockers.extend(values)
if returncode:
    blockers.append(f"cleanup preflight helper exited with status {returncode}")
print(json.dumps(blockers))
PY
)"
  fi
  if [[ "$output_json" == 1 ]]; then
    python3 - "$identity_json" "$cleanup_blockers" <<'PY'
import json,sys
p=json.loads(sys.argv[1]); cleanup=json.loads(sys.argv[2])
print(json.dumps({**p,"status":"ready","identity_status":"bound",
                  "delivery_blockers":[],"cleanup_blockers":cleanup},sort_keys=True))
PY
  else
    echo "finalize-task preflight: ready $task_uid PR #$pr_number"
  fi
  exit 0
fi

run_cleanup() {
  local cleanup_rc=0 cleanup_json
  cleanup_json="$("$SCRIPT_DIR/post-merge-cleanup.sh" --repo-root "$repo_root" --task-uid "$task_uid" --delivery --json)" || cleanup_rc=$?
  if [[ -z "$cleanup_json" ]]; then
    cleanup_json='{"status":"blocked","cleanup_blockers":["cleanup helper returned no result"]}'
    [[ "$cleanup_rc" != 0 ]] || cleanup_rc=1
  fi
  printf '%s\n' "$cleanup_json"
  return "$cleanup_rc"
}

if [[ "$cleanup_only" == 1 ]]; then
  [[ "$protocol_selector" == v2 ]] || fail "--cleanup-only requires a mapped v2 delivery receipt"
  [[ -z "$supplied_patch" ]] || fail "--cleanup-only does not accept patch-equivalence input"
  python3 "$SCRIPT_DIR/post-merge-finalize.py" --repo-root "$repo_root" --task-uid "$task_uid" --delivery --preflight --json >/dev/null \
    || fail "delivery proof must be read back before cleanup-only"
  cleanup_rc=0
  cleanup_json="$(run_cleanup)" || cleanup_rc=$?
  if [[ "$output_json" == 1 ]]; then
    python3 - "$cleanup_json" "$task_uid" "$pr_number" "$receipt_root" <<'PY'
import json,sys
c=json.loads(sys.argv[1]); print(json.dumps({"status":c.get("status"),"task_uid":sys.argv[2],
"pr_number":int(sys.argv[3]),"receipt_root":sys.argv[4],"delivery":{"state":"complete","protocol_version":2},
"cleanup_state":c.get("cleanup_state"),"cleanup":c.get("cleanup"),"cleanup_blockers":c.get("cleanup_blockers",[])},sort_keys=True))
PY
  else
    echo "finalize-task: cleanup-only $task_uid PR #$pr_number"
  fi
  exit "$cleanup_rc"
fi

if [[ "$protocol_selector" == v1 ]]; then
  [[ "$cleanup_only" == 0 ]] || fail "legacy v1 cleanup cannot safely identify a recreated resource instance"
  python3 "$SCRIPT_DIR/post-merge-finalize.py" --repo-root "$repo_root" --task-uid "$task_uid" \
    --terminal-receipt "$terminal_receipt" >/dev/null \
    || fail "legacy v1 terminal receipt live readback failed"
  status="already_finalized"
  delivery_state="complete_v1"
  cleanup_state="legacy_v1_unchanged"
  cleanup_json='{}'
else
  [[ -z "$supplied_patch" ]] || fail "patch-equivalence input is not part of v2 delivery finalization"
  # Before merge-receipt creation, task_complete publication or TaskDone,
  # validate the exact delivered human-readiness artifacts read-only.
  python3 "$SCRIPT_DIR/readiness_transport.py" --repo-root "$repo_root" --task-uid "$task_uid" >/dev/null \
    || fail "readiness artifacts must validate before terminal effects"
  python3 "$SCRIPT_DIR/readiness_transport.py" --repo-root "$repo_root" --task-uid "$task_uid" --create >/dev/null \
    || fail "readiness proof could not be created from validated artifacts"
  if [[ "$protocol_selector" != v2 ]]; then
    [[ -d "$task_worktree" ]] || fail "canonical task worktree is missing before task_done; identity mismatch cannot be repaired here"
    (cd "$task_worktree" && python3 "$SCRIPT_DIR/pr-merge-receipt.py" "$pr_number" --json >"$merge_receipt")
    (cd "$task_worktree" && "$SCRIPT_DIR/task-closeout.sh" --role "$owner_role" --task-uid "$task_uid" \
      --to-status "done" --verification-profile repository_required --pr-receipt "$merge_receipt" >/dev/null)
    "$SCRIPT_DIR/refresh-task-cache.sh" --task-uid "$task_uid" --json >/dev/null
  fi
  delivery_json="$(python3 "$SCRIPT_DIR/post-merge-finalize.py" --repo-root "$repo_root" --task-uid "$task_uid" --delivery --json)" \
    || fail "terminal delivery could not be finalized and read back"
  delivery_state="$(python3 -c 'import json,sys; p=json.loads(sys.stdin.read()); d=p.get("delivery") or {}; print("complete" if d.get("state")=="complete" and d.get("protocol_version")==2 else "blocked")' <<<"$delivery_json")"
  [[ "$delivery_state" == complete ]] || fail "producer did not return a complete v2 delivery proof"
  if [[ "$cleanup_defer" == 1 ]]; then
    status="finalized"
    cleanup_state="cleanup_deferred"
    cleanup_json='{"status":"cleanup_deferred","cleanup_state":"cleanup_deferred","cleanup":{},"cleanup_blockers":["cleanup deferred by caller"]}'
  else
    cleanup_rc=0
    cleanup_json="$(run_cleanup)" || cleanup_rc=$?
    status="finalized"
    cleanup_state="$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read()).get("cleanup_state","cleanup_deferred"))' <<<"$cleanup_json")"
  fi
fi

if [[ "$output_json" == 1 ]]; then
  python3 - "$status" "$task_uid" "$pr_number" "$receipt_root" "$resume" "$delivery_state" "$cleanup_state" "$cleanup_json" <<'PY'
import json,sys
cleanup=json.loads(sys.argv[8]) if sys.argv[8] else {}
delivery={"state":"complete","protocol_version":1 if sys.argv[6]=="complete_v1" else 2}
print(json.dumps({"status":sys.argv[1],"task_uid":sys.argv[2],"pr_number":int(sys.argv[3]),
                  "receipt_root":sys.argv[4],"resume":sys.argv[5]=="1","delivery":delivery,
                  "cleanup_state":sys.argv[7],"cleanup":cleanup.get("cleanup",{}),
                  "cleanup_blockers":cleanup.get("cleanup_blockers",[])},sort_keys=True))
PY
else
  echo "finalize-task: $status $task_uid PR #$pr_number ($cleanup_state)"
fi
