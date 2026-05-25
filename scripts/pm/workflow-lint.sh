#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/workflow-lint.sh [--task-uid <task_uid>] [--allow-unbound]

Static consistency checks for the current task:
- exactly one .pm task binding
- project.md task item Trace points to task_uid
- execution log has Action/Validation/Expected/Actual/Blocker/Next Action
- claim-ready + closeout records present
- PR evidence chain locatable
USAGE
}

TASK_UID=""
ALLOW_UNBOUND=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --task-uid) TASK_UID="${2:-}"; shift 2 ;;
    --allow-unbound) ALLOW_UNBOUND=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "workflow-lint: unknown arg: $1" >&2; usage >&2; exit 2 ;;
  esac
done

python3 - "$ROOT_DIR" "$TASK_UID" "$ALLOW_UNBOUND" <<'PY'
from __future__ import annotations
import pathlib
import subprocess
import sys

root = pathlib.Path(sys.argv[1])
explicit_uid = sys.argv[2].strip()
allow_unbound = sys.argv[3] == "1"
sys.path.insert(0, str(root / "scripts" / "pm"))
from pm_store_docio import load_mapping_document  # type: ignore

branch = subprocess.check_output(["git", "branch", "--show-current"], text=True, cwd=root).strip()
worktree_name = root.name
ACTIVE_STATUSES = {"candidate", "committed", "blocked"}


def parse_task(path: pathlib.Path) -> dict[str, object]:
    fields = dict(load_mapping_document(path))
    fields["path"] = str(path.relative_to(root))
    return fields


tasks = [parse_task(p) for p in sorted((root / ".pm" / "tasks").glob("task_*.yaml"))]
if not tasks:
    raise SystemExit("workflow-lint: no .pm/tasks/*.yaml found")

if explicit_uid:
    bound = [t for t in tasks if t.get("task_uid") == explicit_uid]
else:
    by_hint = [t for t in tasks if str(t.get("worktree_hint") or "") in {branch, worktree_name}]
    active = [t for t in by_hint if str(t.get("status") or "") in ACTIVE_STATUSES]
    bound = active or by_hint

if len(bound) != 1:
    if allow_unbound and len(bound) == 0 and not explicit_uid:
        print("workflow-lint: SKIP (no bound task in current worktree)")
        print("fix: task worktree should set worktree_hint, or pass --task-uid for explicit lint")
        raise SystemExit(0)
    msg = [f"workflow-lint: expected exactly one bound task, found {len(bound)}"]
    msg.append("fix: pass --task-uid <task_uid> or set task worktree_hint to current branch/worktree")
    if not explicit_uid:
        msg.append(f"context: branch={branch or '(detached)'} worktree={worktree_name}")
    raise SystemExit("\n".join(msg))

task = bound[0]
uid = str(task.get("task_uid") or "")
errors = []

def check(cond, bad):
    if not cond: errors.append(bad)

project_docs: list[pathlib.Path] = []
for key in ("doc_refs", "source_refs"):
    raw = task.get(key)
    if isinstance(raw, list):
        for item in raw:
            rel = str(item or "")
            if rel.endswith("/project.md"):
                project_docs.append(root / rel)
if not project_docs:
    if (root / "project.md").is_file():
        project_docs = [root / "project.md"]
check(bool(project_docs), "project.md unresolved from task doc_refs/source_refs; fix: add module project.md to task doc_refs")
if project_docs:
    trace_token = f"Trace: .pm/tasks/{uid}.yaml"
    trace_found = any(p.is_file() and trace_token in p.read_text(encoding="utf-8") for p in project_docs)
    check(trace_found, f"project task item lacks Trace for {uid}; fix: add '{trace_token}' in module project.md")

elog = root / str(task.get("execution_log_path") or "")
check(elog.is_file(), f"execution log missing: {elog.relative_to(root) if str(task.get('execution_log_path') or '') else '(none)'}; fix: run workflow-report --phase start or update execution_log_path")
if elog.exists():
    et = elog.read_text(encoding="utf-8")
    for fld in ["Action:", "Validation Command:", "Expected Result:", "Actual Result:", "Blocker:", "Next Action:"]:
        check(fld in et, f"execution log missing '{fld}' entries; fix:补齐 execution log 六字段")
    check("claim-ready.sh" in et or "claim-ready" in et, "execution log missing claim-ready evidence; fix: append claim-ready command/result entry")
    check("task-closeout.sh" in et or "workflow-report.sh --phase close" in et, "execution log missing closeout evidence; fix: append closeout command/result entry")
    pr_markers = ("prepare-task-pr.sh", "gh pr create", "PR evidence", "PR URL")
    check(any(marker in et for marker in pr_markers), "execution log missing PR evidence marker; fix: append prepare-task-pr/PR URL evidence entry")

check(bool(task.get("last_claim_type")) and bool(task.get("last_verified_at")) and bool(task.get("last_verification_status")),
      "claim-ready record incomplete in task yaml; fix: run ./scripts/pm/claim-ready.sh --claim-type ready_for_pr --verify-command '<cmd>' --task-uid <task_uid>")
check(bool(task.get("last_closed_at")),
      "closeout record missing last_closed_at; fix: run ./scripts/pm/task-closeout.sh --role <owner_role> --task-uid <task_uid> --verify-command '<cmd>'")

pr_hits = []
for p in [root / "PR.md", root / ".pm" / "signals" / "inbox.yaml", root / ".pm" / "signals" / "archive.yaml", root / ".pm" / "working_memory"]:
    if p.is_file() and uid in p.read_text(encoding="utf-8"):
        pr_hits.append(str(p.relative_to(root)))
    elif p.is_dir():
        for f in p.glob("*.yaml"):
            if uid in f.read_text(encoding="utf-8"):
                pr_hits.append(str(f.relative_to(root)))
check(bool(pr_hits), "PR evidence chain not locatable; fix: include task_uid in PR body/evidence (PR.md or .pm signal/memory)")

if errors:
    print(f"workflow-lint: FAIL ({uid})")
    for e in errors:
        print(f"- {e}")
    raise SystemExit(1)

print(f"workflow-lint: OK ({uid})")
for hit in pr_hits[:5]:
    print(f"- evidence: {hit}")
PY
