"""Human-scoped workflow tools executed as code under test, never release authority."""
import json
import re
import subprocess
from pathlib import Path, PurePosixPath

MARKER = "Workflow Maintenance Authority:"
FIELDS = {"repository", "task_uid", "issue_number", "pr_number", "purpose",
          "allowed_write_paths", "allowed_tool_paths"}
TOOL_PATHS = (
    "scripts/pm/workflow_maintenance.py", "scripts/pm/loop-ci.py",
    "scripts/pm/loop_ci_content.py", "scripts/pm/loop_policy.py",
    "scripts/pm/loop_contracts.py", "scripts/pm/loop_traceability.py",
    "scripts/pm/loop_approval_authority.py", "scripts/pm/loop_leaf_result.py",
    "scripts/pm/pr_projection_publication.py", "scripts/pm/pr_projection_journal.py",
    "scripts/pm/portable_file_lock.py", "scripts/pm/projection_publication_contract.py",
    "scripts/pm/workflow-impact-projection.py", "scripts/plan-rust-required-scope.py",
    "scripts/product-doc-content-check.py", "scripts/product_doc_markdown.py",
)

def _unique(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError("duplicate maintenance field")
        value[key] = item
    return value

def _paths(value):
    if (not isinstance(value, list) or not value or any(not isinstance(p, str) for p in value)
            or len(set(value)) != len(value)):
        raise ValueError("maintenance paths must be nonempty and unique")
    for path in value:
        if (not isinstance(path, str) or not path.isascii() or not path.strip()
                or PurePosixPath(path).is_absolute() or ".." in PurePosixPath(path).parts
                or str(PurePosixPath(path)) != path or any(c in path for c in "*?[]\\\n\r")):
            raise ValueError("maintenance path must be exact and repository relative")
    return value

def parse_maintenance_authority(body):
    if not isinstance(body, str) or body.count(MARKER) != 1:
        raise ValueError("one maintenance scope record is required")
    payload = body.split(MARKER, 1)[1].strip()
    match = re.fullmatch(r"```json\s*\n(.*?)\n```", payload, re.S)
    if match:
        payload = match[1]
    value = json.loads(payload, object_pairs_hook=_unique)
    if not isinstance(value, dict) or set(value) != FIELDS:
        raise ValueError("maintenance scope has unsupported fields")
    if (not re.fullmatch(r"[^/\s]+/[^/\s]+", str(value["repository"]))
            or not re.fullmatch(r"task_[0-9a-f]{32}", str(value["task_uid"]))
            or any(type(value[k]) is not int or value[k] < 1 for k in ("issue_number", "pr_number"))
            or value["purpose"] != "candidate-tool-verification"):
        raise ValueError("maintenance scope identity is invalid")
    _paths(value["allowed_write_paths"]); _paths(value["allowed_tool_paths"])
    return value

def maintenance_comment_id(pr_body):
    lines = re.findall(r"(?m)^Workflow Maintenance Authority:[^\n]*$", pr_body or "")
    if not lines:
        return None
    if len(lines) != 1:
        raise ValueError("duplicate maintenance locator")
    match = re.fullmatch(r"Workflow Maintenance Authority: ([1-9][0-9]*)", lines[0])
    if not match:
        raise ValueError("malformed maintenance locator")
    return int(match[1])

def validate_maintenance_authority(comment_readback, author_permission, live_task, live_pr,
                                   event_head_oid, required_write_paths=(), required_tool_paths=()):
    value = parse_maintenance_authority(comment_readback.get("body"))
    repo, issue, number = value["repository"], value["issue_number"], value["pr_number"]
    actor = comment_readback.get("user") or {}
    comment_id = comment_readback.get("id")
    if (type(comment_id) is not int or comment_id < 1 or actor.get("type") != "User"
            or not actor.get("login")
            or comment_readback.get("issue_url") != f"https://api.github.com/repos/{repo}/issues/{issue}"
            or comment_readback.get("html_url") != f"https://github.com/{repo}/issues/{issue}#issuecomment-{comment_id}"
            or not comment_readback.get("created_at")
            or comment_readback.get("created_at") != comment_readback.get("updated_at")):
        raise ValueError("maintenance comment server identity is invalid or edited")
    permission_user = author_permission.get("user") or {}
    if (permission_user.get("login") != actor["login"]
            or not (author_permission.get("permission") == "admin"
                    or (author_permission.get("permissions") or {}).get("admin") is True)):
        raise ValueError("maintenance scope author lacks current admin permission")
    body = live_task.get("body") or ""
    hold = re.findall(r"(?m)^- merge_hold_active: `([^`]+)`$", body)
    hold_lines = re.findall(r"(?m)^- merge_hold_active:[^\n]*$", body)
    if (live_task.get("number") != issue or str(live_task.get("state")).lower() != "open"
            or live_task.get("html_url") != f"https://github.com/{repo}/issues/{issue}"
            or re.findall(r"(?m)^task_uid: ([^\n]+)$", body) != [value["task_uid"]]
            or len(hold_lines) != len(hold) or len(hold) > 1 or (hold and hold != ["false"])):
        raise ValueError("maintenance Task identity or hold is invalid")
    head, base = live_pr.get("head") or {}, live_pr.get("base") or {}
    pr_body = live_pr.get("body") or ""
    if (not re.fullmatch(r"[0-9a-f]{40,64}", str(event_head_oid))
            or head.get("sha") != event_head_oid or live_pr.get("number") != number
            or live_pr.get("html_url") != f"https://github.com/{repo}/pull/{number}"
            or live_pr.get("state") != "open" or live_pr.get("merged_at") is not None
            or live_pr.get("merged", False) is not False
            or not isinstance(live_pr.get("draft"), bool)
            or (head.get("repo") or {}).get("full_name") != repo
            or (base.get("repo") or {}).get("full_name") != repo
            or not head.get("ref") or not base.get("ref")
            or len(re.findall(r"(?m)^Task:[^\n]*$", pr_body)) != 1
            or re.findall(r"(?m)^Task: (task_[0-9a-f]{32})$", pr_body) != [value["task_uid"]]
            or re.findall(r"(?m)^Refs #([1-9][0-9]*)$", pr_body) != [str(issue)]
            or set(re.findall(r"task_[0-9a-f]{32}", pr_body)) != {value["task_uid"]}):
        raise ValueError("maintenance live PR/event identity mismatch")
    if (not set(required_write_paths).issubset(value["allowed_write_paths"])
            or not set(required_tool_paths).issubset(value["allowed_tool_paths"])):
        raise ValueError("maintenance scope does not cover the selected paths")
    return {**value, "comment_id": comment_id, "tool_revision": event_head_oid}

def _gh(*args):
    return json.loads(subprocess.check_output(["gh", "api", *args], text=True))

def read_maintenance_authority(repository, comment_id, task_uid, pr_number, event_head_oid,
                               required_write_paths=(), required_tool_paths=()):
    comment = _gh(f"repos/{repository}/issues/comments/{comment_id}")
    value = parse_maintenance_authority(comment.get("body"))
    if (value["repository"], value["task_uid"], value["pr_number"]) != (repository, task_uid, pr_number):
        raise ValueError("maintenance locator selects another Task or PR")
    actor = (comment.get("user") or {}).get("login")
    permission = _gh(f"repos/{repository}/collaborators/{actor}/permission")
    task = _gh(f"repos/{repository}/issues/{value['issue_number']}")
    pr = _gh(f"repos/{repository}/pulls/{pr_number}")
    repository_info = _gh(f"repos/{repository}")
    if (pr.get("base") or {}).get("ref") != repository_info.get("default_branch"):
        raise ValueError("maintenance PR does not target the live default branch")
    return validate_maintenance_authority(comment, permission, task, pr, event_head_oid,
                                          required_write_paths, required_tool_paths)

def validate_candidate_tool_root(tool_root, target_root, authority, *, execution_revision=None,
                                 required_tool_paths=TOOL_PATHS):
    tool, target = Path(tool_root).resolve(), Path(target_root).resolve()
    def git(root, *args):
        return subprocess.check_output(["git", "-C", str(root), *args])
    revision = execution_revision or authority["tool_revision"]
    if not re.fullmatch(r"[0-9a-f]{40,64}", str(revision)):
        raise ValueError("maintenance execution revision is malformed")
    if (git(tool, "rev-parse", "HEAD").decode().strip() != revision
            or git(tool, "rev-parse", "--path-format=absolute", "--git-common-dir")
            != git(target, "rev-parse", "--path-format=absolute", "--git-common-dir")):
        raise ValueError("maintenance tool checkout identity mismatch")
    for relative in required_tool_paths:
        if relative not in authority["allowed_tool_paths"]:
            raise ValueError("maintenance tool closure is not authorized")
        path = tool / relative
        entry = git(tool, "ls-tree", revision, "--", relative).decode().split()
        if not entry:
            raise ValueError("maintenance tool is absent from the selected revision")
        mode = entry[0]
        if mode not in {"100644", "100755"} or path.is_symlink() or not path.resolve().is_relative_to(tool):
            raise ValueError("maintenance tool mode/path is unsafe")
        if path.read_bytes() != git(tool, "show", f"{revision}:{relative}"):
            raise ValueError("maintenance tool bytes changed")
    if git(tool, "ls-files", "--others", "--", "scripts/pm", ":(exclude)**/__pycache__/**").strip():
        raise ValueError("maintenance tool import shadow")
    return authority
