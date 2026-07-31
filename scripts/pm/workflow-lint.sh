#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${PM_ROOT_DIR:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: ./scripts/pm/workflow-lint.sh [--task-uid <task_uid>] [--allow-unbound] [--phase current|pr-ready|post-pr]

Static consistency checks for the current task:
- GitHub Issue task UID and Project mapping bind the current worktree/task
- GitHub task issue evidence comments include claim/review/closeout records
- post-PR evidence chain is task-local
USAGE
}

TASK_UID=""
ALLOW_UNBOUND=0
PHASE="pr-ready"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --task-uid) TASK_UID="${2:-}"; shift 2 ;;
    --allow-unbound) ALLOW_UNBOUND=1; shift ;;
    --phase) PHASE="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "workflow-lint: unknown arg: $1" >&2; usage >&2; exit 2 ;;
  esac
done
[[ "$PHASE" == "current" || "$PHASE" == "pr-ready" || "$PHASE" == "post-pr" ]] || { echo "workflow-lint: --phase must be current, pr-ready, or post-pr" >&2; exit 2; }

python3 - "$ROOT_DIR" "$TASK_UID" "$ALLOW_UNBOUND" "$PHASE" <<'PY'
from __future__ import annotations
import json
import pathlib
import re
import subprocess
import sys

root = pathlib.Path(sys.argv[1])
explicit_uid = sys.argv[2].strip()
allow_unbound = sys.argv[3] == "1"
phase = sys.argv[4]
sys.path.insert(0, str(root / "scripts" / "pm"))
from pm_store_docio import load_mapping_document  # type: ignore

try:
    branch = subprocess.check_output(
        ["git", "branch", "--show-current"],
        text=True,
        cwd=root,
        stderr=subprocess.DEVNULL,
    ).strip()
except subprocess.CalledProcessError:
    branch = ""
worktree_name = root.name
ACTIVE_STATUSES = {"candidate", "committed", "blocked", "ready", "pr_watch"}


def parse_task(path: pathlib.Path) -> dict[str, object]:
    fields = dict(load_mapping_document(path))
    fields["path"] = str(path.relative_to(root))
    return fields


def parse_issue_body_task_fields(body: str) -> dict[str, object]:
    fields: dict[str, object] = {}
    for key in ("owner_role", "module", "status", "priority", "worktree_hint"):
        match = re.search(rf"^- {re.escape(key)}: `([^`]+)`$", body, re.MULTILINE)
        if match:
            fields[key] = match.group(1)
    return fields


def github_issue_task(explicit_uid: str) -> dict[str, object] | None:
    repo = "eng-cc/oasis7"
    try:
        search_payload = subprocess.check_output(
            [
                "gh",
                "issue",
                "list",
                "-R",
                repo,
                "--search",
                f"{explicit_uid} in:body",
                "--json",
                "number,url,title,state",
                "--limit",
                "5",
            ],
            text=True,
            cwd=root,
            stderr=subprocess.PIPE,
            timeout=180,
        )
        search_hits = json.loads(search_payload)
    except (subprocess.CalledProcessError, FileNotFoundError, subprocess.TimeoutExpired, json.JSONDecodeError):
        return None
    if not isinstance(search_hits, list):
        return None
    matches = [
        hit for hit in search_hits
        if isinstance(hit, dict) and str(hit.get("url") or "").endswith(f"/issues/{hit.get('number')}")
    ]
    if len(matches) != 1:
        return None
    issue_number = str(matches[0].get("number") or "")
    if not issue_number:
        return None
    try:
        issue_payload = subprocess.check_output(
            ["gh", "issue", "view", issue_number, "-R", repo, "--json", "body,comments,number,title,url"],
            text=True,
            cwd=root,
            stderr=subprocess.PIPE,
            timeout=180,
        )
        issue = json.loads(issue_payload)
    except (subprocess.CalledProcessError, FileNotFoundError, subprocess.TimeoutExpired, json.JSONDecodeError):
        return None
    body = str(issue.get("body") or "")
    if explicit_uid not in body:
        return None
    task = parse_issue_body_task_fields(body)
    task.update({
        "task_uid": explicit_uid,
        "title": str(issue.get("title") or matches[0].get("title") or ""),
        "issue_number": int(issue_number),
        "issue_url": str(issue.get("url") or matches[0].get("url") or ""),
        "path": "github-issue-search",
        "github_project_mapping": {
            "repo": repo,
            "issue_number": int(issue_number),
            "issue_url": str(issue.get("url") or matches[0].get("url") or ""),
        },
        "_github_source": "issue_search",
    })
    comments = issue.get("comments") or []
    if isinstance(comments, list):
        task["evidence_comments"] = [
            str(comment.get("url") or "")
            for comment in comments
            if isinstance(comment, dict) and explicit_uid in str(comment.get("body") or "")
        ]
        if any(
            isinstance(comment, dict)
            and explicit_uid in str(comment.get("body") or "")
            and "<!-- oasis7-pm-claim-verification -->" in str(comment.get("body") or "")
            for comment in comments
        ):
            task["last_claim_verification_at"] = "github_issue_comment"
    return task


task_dir = root / ".pm" / "tasks"
github_backed = False
tasks: list[dict[str, object]] = []
if explicit_uid:
    task_path = task_dir / f"{explicit_uid}.yaml"
    tasks = [parse_task(task_path)] if task_path.is_file() else []
    if not tasks:
        mapping_path = root / ".pm/github-project-sync/tasks.json"
        if mapping_path.is_file():
            mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
            record = (mapping.get("tasks") or {}).get(explicit_uid)
            if isinstance(record, dict):
                github_backed = True
                task = dict(record)
                project = mapping.get("project") or {}
                task["task_uid"] = explicit_uid
                task["path"] = str(mapping_path.relative_to(root))
                task["github_project_mapping"] = {
                    "repo": str(project.get("repo") or "eng-cc/oasis7"),
                    "issue_number": task.get("issue_number"),
                    "issue_url": task.get("issue_url"),
                }
                tasks = [task]
        if not tasks:
            task = github_issue_task(explicit_uid)
            if task is not None:
                github_backed = True
                tasks = [task]
else:
    mapping_path = root / ".pm/github-project-sync/tasks.json"
    if mapping_path.is_file():
        mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
        project = mapping.get("project") or {}
        repo_name = str(project.get("repo") or "eng-cc/oasis7")
        for uid, record in sorted((mapping.get("tasks") or {}).items()):
            record = dict(record)
            record["task_uid"] = uid
            record["path"] = str(mapping_path.relative_to(root))
            record["github_project_mapping"] = {
                "repo": repo_name,
                "issue_number": record.get("issue_number"),
                "issue_url": record.get("issue_url"),
            }
            tasks.append(record)
        github_backed = bool(tasks)
    if not tasks:
        tasks = [parse_task(p) for p in sorted(task_dir.glob("task_*.yaml"))]
if not tasks:
    if explicit_uid:
        raise SystemExit(f"workflow-lint: task mapping not found for --task-uid {explicit_uid}")
    raise SystemExit("workflow-lint: no GitHub Project task mapping found for this worktree")

def worktree_hint_matches(raw_hint: object) -> bool:
    hint = str(raw_hint or "")
    if hint in {branch, worktree_name}:
        return True
    if not hint:
        return False
    hint_path = pathlib.Path(hint).expanduser()
    candidates = {hint_path.name}
    if hint_path.is_absolute():
        try:
            candidates.add(str(hint_path.resolve()))
            candidates.add(str(hint_path.resolve().relative_to(root.parent)))
        except (OSError, ValueError):
            candidates.add(str(hint_path))
    return worktree_name in candidates or str(root.resolve()) in candidates

if explicit_uid:
    bound = [t for t in tasks if t.get("task_uid") == explicit_uid]
else:
    by_hint = [t for t in tasks if worktree_hint_matches(t.get("worktree_hint"))]
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
pr_hits: list[str] = []

def check(cond, bad):
    if not cond: errors.append(bad)


def file_contains_text(path: pathlib.Path, needle: str) -> bool:
    with path.open("r", encoding="utf-8") as handle:
        return any(needle in line for line in handle)


def unresolved_fallback_paths(task_uid: str) -> list[str]:
    fallback_dir = root / ".pm" / "scratch" / task_uid / "fallback-evidence"
    if not fallback_dir.is_dir():
        return []
    return [
        str(path.relative_to(root))
        for path in sorted(fallback_dir.glob("*.md"))
        if not path.name.endswith(".replayed.md")
    ]


def issue_number_from_url(value: object) -> str:
    match = re.search(r"/issues/(\d+)(?:$|[?#])", str(value or ""))
    return match.group(1) if match else ""


def issue_comments_via_rest(repo: str, issue_number: str) -> list[dict[str, object]]:
    owner, _, name = repo.partition("/")
    if not owner or not name:
        raise subprocess.CalledProcessError(2, ["gh", "api", "invalid-repo"])
    payload = subprocess.check_output(
        [
            "gh",
            "api",
            f"repos/{owner}/{name}/issues/{issue_number}/comments",
            "--paginate",
        ],
        text=True,
        cwd=root,
        stderr=subprocess.PIPE,
        timeout=180,
    )
    if not payload.strip():
        return []
    decoder = json.JSONDecoder()
    comments: list[dict[str, object]] = []
    idx = 0
    while idx < len(payload):
        while idx < len(payload) and payload[idx].isspace():
            idx += 1
        if idx >= len(payload):
            break
        page, next_idx = decoder.raw_decode(payload, idx)
        if isinstance(page, list):
            comments.extend(comment for comment in page if isinstance(comment, dict))
        idx = next_idx
    return comments


def github_issue_comments(task: dict[str, object]) -> list[str]:
    mapping_info = task.get("github_project_mapping")
    repo = ""
    if isinstance(mapping_info, dict):
        repo = str(mapping_info.get("repo") or "")
    repo = repo or "eng-cc/oasis7"
    issue_number = str(task.get("issue_number") or "") or issue_number_from_url(task.get("issue_url"))
    if not issue_number:
        errors.append("GitHub-backed task missing issue number for live issue-comment audit")
        return []
    try:
        payload = subprocess.check_output(
            ["gh", "issue", "view", issue_number, "-R", repo, "--json", "comments"],
            text=True,
            cwd=root,
            stderr=subprocess.PIPE,
            timeout=180,
        )
    except (subprocess.CalledProcessError, FileNotFoundError, subprocess.TimeoutExpired) as exc:
        try:
            comments = issue_comments_via_rest(repo, issue_number)
        except (subprocess.CalledProcessError, FileNotFoundError, subprocess.TimeoutExpired, json.JSONDecodeError) as rest_exc:
            errors.append(f"GitHub-backed task issue comments unreadable for live audit: {exc}; REST fallback failed: {rest_exc}")
            return []
    else:
        try:
            comments = json.loads(payload).get("comments") or []
        except json.JSONDecodeError as exc:
            errors.append(f"GitHub-backed task issue comments JSON invalid: {exc}")
            return []
    return [str(comment.get("body") or "") for comment in comments if isinstance(comment, dict)]


def comment_has(markers: tuple[str, ...], comments: list[str], task_uid: str) -> bool:
    task_marker = f"Task UID: {task_uid}"
    return any(task_marker in comment and all(marker in comment for marker in markers) for comment in comments)

if github_backed:
    log_path = str(task.get("execution_log_path") or "")
    if phase == "current" and log_path and not log_path.startswith(("http://", "https://")):
        elog = root / log_path
        check(elog.is_file(), f"execution log missing: {elog.relative_to(root)}; fix: run workflow-report --phase start or update execution_log_path")
        if elog.exists():
            et = elog.read_text(encoding="utf-8")
            heading_re = re.compile(r"^## \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} CST / [a-z_][a-z0-9_]*$", re.MULTILINE)
            headings = list(heading_re.finditer(et))
            entries = []
            for idx, match in enumerate(headings):
                start = match.end()
                end = headings[idx + 1].start() if idx + 1 < len(headings) else len(et)
                entries.append(et[start:end])
            required_fields = ["完成内容:", "遗留事项:", "Action:", "Validation Command:", "Expected Result:", "Actual Result:", "Blocker / Next Action:"]
            check(bool(entries), "execution log missing real entries; fix: use ./scripts/pm/append-execution-log.sh to add a timestamped entry")
            if entries:
                complete_entry_found = any(all(fld in entry for fld in required_fields) for entry in entries)
                missing = [fld for fld in required_fields if not any(fld in entry for entry in entries)]
                check(complete_entry_found, f"execution log missing Actual Result or one complete structured entry; fix: use ./scripts/pm/append-execution-log.sh or補齊 execution log fields ({', '.join(missing) if missing else 'fields split across entries'})")
    if phase in {"pr-ready", "post-pr"}:
        fallback_paths = unresolved_fallback_paths(uid)
        check(not fallback_paths, "unreplayed fallback evidence exists: " + ",".join(fallback_paths))
        source = str(task.get("_github_source") or "mapping")
        check(bool(task.get("issue_number") or task.get("issue_url")), "GitHub-backed task missing issue handle")
        if source == "mapping":
            check(bool(task.get("project_item_id")), "GitHub-backed task missing project_item_id in .pm/github-project-sync/tasks.json")
        check(bool(task.get("evidence_comments")), "GitHub-backed task missing issue evidence comment links")
        claim_records = task.get("claim_verifications")
        has_verified_claim = isinstance(claim_records, list) and any(
            isinstance(item, dict) and str(item.get("status") or "") == "verified"
            for item in claim_records
        )
        check(has_verified_claim or bool(task.get("last_claim_verification_at")),
              "GitHub-backed task missing verified claim-ready evidence")
        if phase == "pr-ready":
            check(str(task.get("status") or "") in {"ready", "pr_watch", "done"},
                  "GitHub-backed task status must be ready/pr_watch/done for pr-ready lint")
        if phase == "post-pr":
            check(str(task.get("status") or "") in {"pr_watch", "done"},
                  "GitHub-backed task status must be pr_watch/done for post-pr lint")
        comments = github_issue_comments(task)
        if comments:
            check(comment_has(("<!-- oasis7-pm-claim-verification -->",), comments, uid),
                  "GitHub issue comments missing claim-ready verification marker")
            check(comment_has(("Pre-PR Local Role Review: passed",), comments, uid),
                  "GitHub issue comments missing passed pre-PR local role review packet")
            check(comment_has(("Evidence Phase: pre_pr_ready",), comments, uid) or comment_has(("Evidence Phase: pr_watch",), comments, uid),
                  "GitHub issue comments missing pre-PR-ready or PR-watch evidence marker")
    if phase == "post-pr":
        check(bool(task.get("pr_url") or task.get("pull_request_url") or task.get("pr_number")),
              "GitHub-backed task missing PR evidence link")
    if errors:
        print(f"workflow-lint: FAIL ({uid})")
        for e in errors:
            print(f"- {e}")
        raise SystemExit(1)
    print(f"workflow-lint: OK ({uid}, phase={phase}, github-backed)")
    for hit in list(task.get("evidence_comments") or [])[:5]:
        print(f"- evidence: {hit}")
    raise SystemExit(0)

elog = root / str(task.get("execution_log_path") or "")
check(elog.is_file(), f"execution log missing: {elog.relative_to(root) if str(task.get('execution_log_path') or '') else '(none)'}; fix: run workflow-report --phase start or update execution_log_path")
if elog.exists():
    et = elog.read_text(encoding="utf-8")
    heading_re = re.compile(r"^## \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} CST / [a-z_][a-z0-9_]*$", re.MULTILINE)
    headings = list(heading_re.finditer(et))
    entries = []
    for idx, match in enumerate(headings):
        start = match.end()
        end = headings[idx + 1].start() if idx + 1 < len(headings) else len(et)
        entries.append(et[start:end])
    required_fields = ["完成内容:", "遗留事项:", "Action:", "Validation Command:", "Expected Result:", "Actual Result:", "Blocker / Next Action:"]
    check(bool(entries), "execution log missing real entries; fix: use ./scripts/pm/append-execution-log.sh to add a timestamped entry")
    if entries:
        complete_entry_found = any(all(fld in entry for fld in required_fields) for entry in entries)
        missing = [fld for fld in required_fields if not any(fld in entry for entry in entries)]
        check(complete_entry_found, f"execution log missing Actual Result or one complete structured entry; fix: use ./scripts/pm/append-execution-log.sh or補齊 execution log fields ({', '.join(missing) if missing else 'fields split across entries'})")
    if phase in {"pr-ready", "post-pr"}:
        check("claim-ready.sh" in et or "claim-ready" in et, "execution log missing claim-ready evidence; fix: append claim-ready command/result entry")
        check("task-closeout.sh" in et or "workflow-report.sh --phase close" in et, "execution log missing closeout evidence; fix: append closeout command/result entry")
    if phase == "post-pr":
        pr_markers = ("prepare-task-pr.sh", "gh pr create", "PR URL", "PR #", "https://github.com/")
        has_pr_marker = any(marker in et for marker in pr_markers)
        if has_pr_marker:
            pr_hits.append(str(elog.relative_to(root)))

if phase in {"pr-ready", "post-pr"}:
    check(bool(task.get("last_claim_type")) and bool(task.get("last_verified_at")) and bool(task.get("last_verification_status")),
          "claim-ready record incomplete in task yaml; fix: run ./scripts/pm/claim-ready.sh --claim-type ready_for_pr --verify-command '<cmd>' --task-uid <task_uid>")
    check(bool(task.get("last_closed_at")),
          "closeout record missing last_closed_at; fix: run ./scripts/pm/task-closeout.sh --role <owner_role> --task-uid <task_uid> --verify-command '<cmd>'")

for p in [root / ".pm" / "signals" / "inbox.yaml", root / ".pm" / "signals" / "archive.yaml", root / ".pm" / "working_memory"]:
    if p.is_file() and file_contains_text(p, uid):
        pr_hits.append(str(p.relative_to(root)))
    elif p.is_dir():
        for f in p.glob("*.yaml"):
            if file_contains_text(f, uid):
                pr_hits.append(str(f.relative_to(root)))
if phase == "post-pr":
    check(bool(pr_hits), "PR evidence chain not locatable; fix: append task-local PR evidence to the execution log or .pm signal/memory")

if errors:
    print(f"workflow-lint: FAIL ({uid})")
    for e in errors:
        print(f"- {e}")
    raise SystemExit(1)

print(f"workflow-lint: OK ({uid}, phase={phase})")
for hit in pr_hits[:5]:
    print(f"- evidence: {hit}")
PY
