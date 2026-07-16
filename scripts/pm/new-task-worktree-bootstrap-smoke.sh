#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
SOURCE_ROOT="$ROOT_DIR"

OUTPUT_JSON=0
KEEP_TEMP=0

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/new-task-worktree-bootstrap-smoke.sh [--json] [--keep-temp]

Create a temporary task worktree, bootstrap a GitHub-backed task inside it through
`new-task-worktree.sh --pm-*`, and assert that the source worktree stays
unchanged while the target worktree receives the mapping record, start metadata,
a copied canonical main-worktree `config.toml`, and an ignored `target` symlink
to the repo-family shared cargo cache. When the canonical source checkout has no
local `config.toml`, this smoke seeds a temporary fixture so the copy path is
still covered.

Options:
  --json       Print machine-readable JSON summary
  --keep-temp  Keep the temporary directory for inspection
  -h, --help   Show help
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)
      OUTPUT_JSON=1
      shift
      ;;
    --keep-temp)
      KEEP_TEMP=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "new-task-worktree-bootstrap-smoke: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

TMPDIR="$(mktemp -d)"
FIXTURE_ROOT="$TMPDIR/repo"
mkdir -p "$FIXTURE_ROOT"
(cd "$SOURCE_ROOT" && git ls-files -co --exclude-standard -z | tar --null -T - -cf -) | tar -xf - -C "$FIXTURE_ROOT"
git -C "$FIXTURE_ROOT" init -q -b main
git -C "$FIXTURE_ROOT" config user.email test@example.com
git -C "$FIXTURE_ROOT" config user.name Test
git -C "$FIXTURE_ROOT" add .
git -C "$FIXTURE_ROOT" commit -qm "fixture snapshot"
git -C "$FIXTURE_ROOT" update-ref refs/remotes/origin/main HEAD
ROOT_DIR="$FIXTURE_ROOT"
WORKTREE_PATH="$TMPDIR/worktree"
BRANCH_NAME="task/smoke-task-worktree-pm-bootstrap-$$-$(date +%s)"
SOURCE_STATUS_BEFORE="$(git -C "$ROOT_DIR" status --short)"
GIT_COMMON_DIR="$(git -C "$ROOT_DIR" rev-parse --path-format=absolute --git-common-dir)"
CANONICAL_REPO_ROOT="$(cd "$GIT_COMMON_DIR/.." && pwd -P)"
CANONICAL_CONFIG_PATH="$CANONICAL_REPO_ROOT/config.toml"
CREATED_CANONICAL_CONFIG_FIXTURE=0
mkdir -p "$TMPDIR/bin"

cat > "$TMPDIR/bin/gh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%q ' "$@" >> "$GH_CALL_LOG"
printf '\n' >> "$GH_CALL_LOG"
case "$*" in
  "issue create -R eng-cc/oasis7 --title "*)
    printf 'https://github.com/eng-cc/oasis7/issues/2401\n'
    ;;
  "issue edit 2401 -R eng-cc/oasis7 --body-file "*)
    printf '%s\n' '--- issue edit body ---' >> "$GH_EDIT_BODY_LOG"
    cat "${@: -1}" >> "$GH_EDIT_BODY_LOG"
    printf '\n' >> "$GH_EDIT_BODY_LOG"
    printf 'edited\n'
    ;;
  "issue comment 2401 -R eng-cc/oasis7 --body-file "*)
    n=$(( $(wc -l < "$GH_COMMENT_LOG") + 1 ))
    printf 'comment-%s\n' "$n" >> "$GH_COMMENT_LOG"
    printf 'https://github.com/eng-cc/oasis7/issues/2401#issuecomment-%s\n' "$n"
    ;;
  "project item-add 1 --owner eng-cc --url https://github.com/eng-cc/oasis7/issues/2401 --format json")
    printf '{"id":"ITEM_ID","content":{"url":"https://github.com/eng-cc/oasis7/issues/2401"}}\n'
    ;;
  "project view 1 --owner eng-cc --format json")
    printf '{"id":"PROJECT_ID","number":1,"title":"oasis7 Engineering PM","url":"https://github.com/users/eng-cc/projects/1"}\n'
    ;;
  "project field-list 1 --owner eng-cc --format json")
    cat <<'JSON'
{"fields":[
{"id":"FIELD_STATUS","name":"Status","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_TODO","name":"Todo"},{"id":"OPT_IN_PROGRESS","name":"In Progress"},{"id":"OPT_BLOCKED_STATUS","name":"Blocked"},{"id":"OPT_READY","name":"Ready / PR"},{"id":"OPT_PR_WATCH","name":"PR Watch"},{"id":"OPT_DONE_STATUS","name":"Done"}]},
{"id":"FIELD_TASK_UID","name":"Task UID","type":"ProjectV2Field"},
{"id":"FIELD_OWNER_ROLE","name":"Owner Role","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_TPM","name":"tpm"},{"id":"OPT_PRODUCER","name":"producer_system_designer"}]},
{"id":"FIELD_MODULE","name":"Module","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_ENGINEERING","name":"engineering"}]},
{"id":"FIELD_PM_STATUS","name":"PM Status","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_CANDIDATE","name":"candidate"},{"id":"OPT_COMMITTED","name":"committed"},{"id":"OPT_READY_PM","name":"ready"},{"id":"OPT_PR_WATCH_PM","name":"pr_watch"},{"id":"OPT_DONE","name":"done"}]},
{"id":"FIELD_WORKFLOW_PHASE","name":"Workflow Phase","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_EXECUTION","name":"execution"},{"id":"OPT_CLOSEOUT","name":"closeout"},{"id":"OPT_PR_WATCH_PHASE","name":"pr_watch"},{"id":"OPT_DONE_PHASE","name":"done"}]},
{"id":"FIELD_PRIORITY","name":"Priority","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_P2","name":"P2"}]},
{"id":"FIELD_BLOCKED","name":"Blocked Reason","type":"ProjectV2Field"},
{"id":"FIELD_WORKTREE","name":"Canonical Worktree","type":"ProjectV2Field"},
{"id":"FIELD_PR","name":"PR","type":"ProjectV2Field"},
{"id":"FIELD_TIER","name":"Test Tier Required","type":"ProjectV2SingleSelectField","options":[{"id":"OPT_NA","name":"n/a"}]},
{"id":"FIELD_UPDATED","name":"Last PM Update","type":"ProjectV2Field"}]}
JSON
    ;;
  project\ item-edit*)
    printf '{}\n'
    ;;
  *)
    echo "unexpected gh invocation: $*" >&2
    exit 9
    ;;
esac
SH
chmod +x "$TMPDIR/bin/gh"
export PATH="$TMPDIR/bin:$PATH"
export GH_CALL_LOG="$TMPDIR/gh-calls.log"
export GH_COMMENT_LOG="$TMPDIR/gh-comments.log"
export GH_EDIT_BODY_LOG="$TMPDIR/issue-body-edited.md"
: > "$GH_CALL_LOG"
: > "$GH_COMMENT_LOG"
: > "$GH_EDIT_BODY_LOG"

if [[ ! -f "$CANONICAL_CONFIG_PATH" ]]; then
  cat >"$CANONICAL_CONFIG_PATH" <<'EOF'
[llm]
model = "smoke-model"
base_url = "https://example.invalid/v1"
api_key = "smoke-api-key"
EOF
  CREATED_CANONICAL_CONFIG_FIXTURE=1
fi

cleanup() {
  set +e
  if [[ "$CREATED_CANONICAL_CONFIG_FIXTURE" == "1" ]]; then
    rm -f "$CANONICAL_CONFIG_PATH"
  fi
  if [[ "$KEEP_TEMP" == "1" ]]; then
    return
  fi
  if [[ -d "$WORKTREE_PATH" ]]; then
    git -C "$ROOT_DIR" worktree remove --force "$WORKTREE_PATH" >/dev/null 2>&1 || true
  fi
  git -C "$ROOT_DIR" branch -D "$BRANCH_NAME" >/dev/null 2>&1 || true
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

BOOTSTRAP_JSON="$(
  cd "$ROOT_DIR" &&
  ./scripts/new-task-worktree.sh engineering smoke-task-worktree-pm-bootstrap \
    --allow-dirty-source \
    --branch "$BRANCH_NAME" \
    --path "$WORKTREE_PATH" \
    --pm-owner-role producer_system_designer \
    --pm-title "smoke bootstrap task" \
    --pm-source-ref doc/engineering/project.md \
    --pm-doc-ref doc/engineering/prd.md \
    --pm-related-prd PRD-ENGINEERING-021 \
    --pm-acceptance "bootstrap created committed task in target worktree" \
    --json
)"

SMOKE_TASK_UID="$(BOOTSTRAP_JSON="$BOOTSTRAP_JSON" python3 -c 'import json,os; print(json.loads(os.environ["BOOTSTRAP_JSON"])["pm_task"]["task_uid"])')"
PM_ROOT_DIR="$WORKTREE_PATH" "$ROOT_DIR/scripts/pm/append-execution-log.sh" \
  --task-uid "$SMOKE_TASK_UID" \
  --role producer_system_designer \
  --completed "new-task-worktree smoke appended structured evidence" \
  --pending "none" \
  --action "exercise GitHub-backed evidence append from absolute worktree_hint" \
  --validation-command "mapping/evidence assertion" \
  --expected-result "GitHub-backed mapping accepts the bootstrapped task mapping and evidence sink" \
  --actual-result "append command wrote a complete GitHub issue evidence comment before mapping assertion" \
  --blocker-next-action "none" \
  --json >/dev/null

SUMMARY_JSON="$(
  BOOTSTRAP_JSON="$BOOTSTRAP_JSON" python3 - "$ROOT_DIR" "$WORKTREE_PATH" "$SOURCE_STATUS_BEFORE" <<'PY'
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

root = Path(sys.argv[1])
worktree = Path(sys.argv[2]).resolve()
source_status_before = sys.argv[3]
payload = json.loads(__import__("os").environ["BOOTSTRAP_JSON"])
pm_task = payload.get("pm_task")
if not pm_task:
    raise SystemExit("pm_task summary missing from new-task-worktree bootstrap output")
snapshot_path = Path(pm_task.get("bootstrap_snapshot_path") or "")
if not snapshot_path.is_file():
    raise SystemExit(f"bootstrap snapshot missing: {snapshot_path}")
snapshot = json.loads(snapshot_path.read_text(encoding="utf-8"))
if snapshot.get("digest") != pm_task.get("bootstrap_snapshot_digest"):
    raise SystemExit("bootstrap snapshot digest does not match worktree summary")
if snapshot.get("task", {}).get("uid") != pm_task["task_uid"]:
    raise SystemExit("bootstrap snapshot is bound to the wrong task UID")
live_head = subprocess.check_output(["git", "-C", str(worktree), "rev-parse", "HEAD"], text=True).strip()
if snapshot.get("git", {}).get("head") != live_head:
    raise SystemExit("bootstrap snapshot is not bound to the worktree HEAD")
subprocess.run(
    [
        str(worktree / "scripts/pm/bootstrap-task-snapshot.py"), "validate",
        "--repo-root", str(worktree), "--task-uid", pm_task["task_uid"],
        "--request-identity", "smoke bootstrap task",
    ],
    cwd=worktree,
    check=True,
    stdout=subprocess.DEVNULL,
)
config_summary = payload.get("config")
if not config_summary:
    raise SystemExit("config summary missing from new-task-worktree output")
cargo_cache_summary = payload.get("cargo_cache")
if not cargo_cache_summary:
    raise SystemExit("cargo_cache summary missing from new-task-worktree output")

mapping_path = worktree / ".pm/github-project-sync/tasks.json"
if not mapping_path.is_file():
    raise SystemExit(f"bootstrapped GitHub Project mapping missing: {mapping_path}")
mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
record = (mapping.get("tasks") or {}).get(pm_task["task_uid"])
if not record:
    raise SystemExit("bootstrapped task uid missing from GitHub Project mapping")
if record.get("status") != "committed":
    raise SystemExit(f"bootstrapped task did not move to committed: {record.get('status')}")
if not record.get("last_started_at"):
    raise SystemExit("bootstrapped task missing workflow-report start timestamp")
expected_worktree_hints = {str(worktree), str(worktree.resolve())}
if str(record.get("worktree_hint") or "") not in expected_worktree_hints:
    raise SystemExit("bootstrapped task worktree_hint does not point at target worktree")
if not str(record.get("issue_url") or "").startswith("https://"):
    raise SystemExit("bootstrapped task missing GitHub issue URL")
if not record.get("evidence_comments"):
    raise SystemExit("bootstrapped task missing GitHub issue evidence comments")

source_config_path = Path(config_summary["source_path"])
target_config_path = worktree / "config.toml"
if not config_summary.get("source_exists"):
    raise SystemExit("smoke expected canonical config source fixture to exist")
if not source_config_path.is_file():
    raise SystemExit("reported canonical config source path does not exist")
if not target_config_path.is_file():
    raise SystemExit("target worktree missing copied canonical config.toml")
if source_config_path.read_bytes() != target_config_path.read_bytes():
    raise SystemExit("target worktree config.toml does not match canonical source config")
if not config_summary.get("copied"):
    raise SystemExit("new-task-worktree summary did not report config copy")
config_copied = True

tracked_config = subprocess.run(
    ["git", "-C", str(worktree), "ls-files", "--error-unmatch", "config.toml"],
    text=True,
    capture_output=True,
)
if tracked_config.returncode == 0:
    raise SystemExit("copied config.toml became git-tracked inside target worktree")
ignored_status = subprocess.check_output(
    ["git", "-C", str(worktree), "status", "--short", "--ignored", "--", "config.toml"],
    text=True,
).strip()
if config_copied and ignored_status != "!! config.toml":
    raise SystemExit("copied config.toml is not ignored in target worktree as expected")
if not config_copied and ignored_status:
    raise SystemExit("target worktree reported unexpected config.toml status without canonical source file")

target_path = worktree / "target"
if not target_path.is_symlink():
    raise SystemExit("target worktree target path is not a symlink")
linked_target = target_path.resolve()
reported_shared_target = Path(cargo_cache_summary["shared_target_dir"]).resolve()
if linked_target != reported_shared_target:
    raise SystemExit(
        f"target symlink does not point at reported shared cargo cache: {linked_target} != {reported_shared_target}"
    )
if not reported_shared_target.is_dir():
    raise SystemExit("reported shared cargo cache directory does not exist")
if not cargo_cache_summary.get("linked"):
    raise SystemExit("new-task-worktree summary did not report cargo target link")
printed_shared_target = subprocess.check_output(
    ["./scripts/cargo-dev.sh", "--print-target-dir"],
    cwd=worktree,
    text=True,
).strip()
if Path(printed_shared_target).resolve() != reported_shared_target:
    raise SystemExit("cargo-dev shared target differs from task worktree target symlink")
target_status = subprocess.check_output(
    ["git", "-C", str(worktree), "status", "--short", "--ignored", "--", "target"],
    text=True,
).strip()
if target_status != "!! target":
    raise SystemExit("target symlink is not ignored in target worktree as expected")

source_status_after = subprocess.check_output(
    ["git", "-C", str(root), "status", "--short"],
    text=True,
).rstrip("\n")
if source_status_after != source_status_before:
    raise SystemExit("source worktree status changed during PM bootstrap")

print(
    json.dumps(
        {
            "task_uid": pm_task["task_uid"],
            "task_path": pm_task["task_path"],
            "execution_log_path": pm_task["execution_log_path"],
            "source_status_preserved": True,
            "workflow_started": True,
            "bootstrap_snapshot_path": str(snapshot_path),
            "bootstrap_snapshot_digest": snapshot["digest"],
            "worktree_path": str(worktree),
            "config_copied": config_copied,
            "shared_cargo_target_dir": str(reported_shared_target),
            "target_symlinked": True,
        },
        ensure_ascii=False,
    )
)
PY
)"

if [[ "$OUTPUT_JSON" == "1" ]]; then
  printf '%s\n' "$SUMMARY_JSON"
  exit 0
fi

cat <<INFO
new-task-worktree bootstrap smoke passed
- worktree path: $WORKTREE_PATH
- branch: $BRANCH_NAME
- summary: $SUMMARY_JSON
INFO
