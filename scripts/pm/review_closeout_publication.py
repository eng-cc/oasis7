#!/usr/bin/env python3
"""Resolve exact C1 identity and reuse its existing publication journal."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Callable
from urllib.parse import quote


REPOSITORY = "eng-cc/oasis7"
TASK_RE = re.compile(r"task_[0-9a-f]{32}\Z")
OID_RE = re.compile(r"[0-9a-f]{40,64}\Z")


class CloseoutPublicationError(ValueError):
    pass


def load_module(root: Path, name: str, relative_path: str) -> Any:
    path = root / "scripts" / "pm" / relative_path
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise CloseoutPublicationError(f"cannot load repository helper: {relative_path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def gh_json(command: list[str], label: str) -> Any:
    try:
        result = subprocess.run(["gh", *command], text=True, capture_output=True, check=False)
    except OSError as exc:
        raise CloseoutPublicationError(f"cannot read live {label}: {exc}") from exc
    if result.returncode:
        detail = result.stderr.strip() or result.stdout.strip() or "command failed"
        raise CloseoutPublicationError(f"live {label} read failed: {detail}")
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise CloseoutPublicationError(f"live {label} read returned invalid JSON") from exc


def require_positive_int(value: object, label: str) -> int:
    if type(value) is not int or value < 1:
        raise CloseoutPublicationError(f"{label} is missing or invalid")
    return value


def c1_bootstrap_epoch(root: Path, task_uid: str, task_record: dict[str, Any],
                       issue_body: str) -> int | None:
    """Derive C1's nullable epoch using the publisher's Task-record contract.

    The review package has its own positive bootstrap epoch, which may come
    from the immutable bootstrap snapshot. C1 predates that distinction for
    legacy non-loop tasks: its writer uses the live loop binding's epoch when
    present, otherwise the canonical Task record's epoch, and permits null.
    Do not replace that historical C1 identity with the review epoch.
    """
    task_module = load_module(root, "review_closeout_task_issue_projection", "github-project-task.py")
    try:
        live_fields = task_module.issue_task_fields(issue_body)
    except (SystemExit, TypeError, ValueError) as exc:
        raise CloseoutPublicationError(f"live Task Issue loop binding is invalid: {exc}") from exc
    live_binding = live_fields.get("loop_binding")
    cached_binding = task_record.get("loop_binding")
    if live_binding != cached_binding:
        raise CloseoutPublicationError("live Task Issue loop binding differs from the canonical Task record")
    if live_binding is not None:
        try:
            binding = task_module.validate_loop_binding(live_binding)
        except (SystemExit, TypeError, ValueError) as exc:
            raise CloseoutPublicationError(f"canonical loop binding is invalid: {exc}") from exc
        if binding.get("task_uid") != task_uid:
            raise CloseoutPublicationError("canonical loop binding belongs to another Task UID")
        epoch = binding.get("bootstrap_epoch")
    else:
        epoch = task_record.get("bootstrap_epoch")
    if epoch is not None and (type(epoch) is not int or epoch < 1):
        raise CloseoutPublicationError("canonical Task C1 bootstrap epoch is invalid")
    return epoch


def c1_authority_locator(comments: list[dict[str, Any]],
                         expected_without_authority: dict[str, Any],
                         publication_module: Any) -> tuple[str, dict[str, Any]]:
    """Find one unedited C1 matching every independently derived field but authority."""
    marker = "<!-- oasis7-ci-publication/v1 -->"
    expected_fields = set(expected_without_authority)
    if "planner_authority_oid" in expected_fields or len(expected_fields) != 12:
        raise CloseoutPublicationError("C1 locator inputs are not the exact twelve-field closure")
    matches: list[tuple[dict[str, Any], dict[str, Any]]] = []
    for comment in comments:
        body = comment.get("body")
        if not isinstance(body, str) or marker not in body:
            continue
        try:
            value = publication_module.parse_publication_comment(body)
        except (TypeError, ValueError):
            continue
        if all(value.get(field) == expected for field, expected in expected_without_authority.items()):
            matches.append((comment, value))
    if len(matches) != 1:
        raise CloseoutPublicationError(
            "C1 planner-authority locator is missing or ambiguous"
        )
    comment, value = matches[0]
    created_at = comment.get("created_at")
    updated_at = comment.get("updated_at")
    if not isinstance(created_at, str) or not created_at or updated_at != created_at:
        raise CloseoutPublicationError("C1 planner-authority locator is edited or lacks exact server timestamps")
    authority = value.get("planner_authority_oid")
    if not isinstance(authority, str) or OID_RE.fullmatch(authority) is None:
        raise CloseoutPublicationError("C1 planner-authority locator is malformed")
    return authority, comment


def verify_planner_authority(root: Path, authority_oid: str, target_ref: str,
                             head: str, scope_oid: str, config_digest: str) -> None:
    """Prove a C1 locator is a canonical default-branch target with exact scope/config."""
    branch = gh_json(
        ["api", f"repos/{REPOSITORY}/branches/{quote(target_ref, safe='')}"],
        "live canonical default-branch tip",
    )
    commit = branch.get("commit") if isinstance(branch, dict) else None
    tip_oid = commit.get("sha") if isinstance(commit, dict) else None
    if (not isinstance(branch, dict) or branch.get("name") != target_ref
            or not isinstance(tip_oid, str) or OID_RE.fullmatch(tip_oid) is None):
        raise CloseoutPublicationError("live canonical default-branch tip is unavailable or malformed")

    def git(*args: str) -> str:
        result = subprocess.run(["git", "-C", str(root), *args],
                                text=True, capture_output=True, check=False)
        if result.returncode:
            detail = result.stderr.strip() or result.stdout.strip() or "git command failed"
            raise CloseoutPublicationError(f"cannot independently verify C1 planner authority: {detail}")
        return result.stdout.strip()

    if git("rev-parse", f"{authority_oid}^{{commit}}") != authority_oid:
        raise CloseoutPublicationError("C1 planner-authority locator is not a canonical commit")
    if git("rev-parse", f"{tip_oid}^{{commit}}") != tip_oid:
        raise CloseoutPublicationError("live canonical default-branch commit is unavailable locally")
    if git("merge-base", "--is-ancestor", authority_oid, tip_oid) == "":
        # --is-ancestor has no stdout on success; the subprocess return status
        # is checked by git() above, including failure.
        pass
    try:
        blob = subprocess.run(
            ["git", "-C", str(root), "show", f"{authority_oid}:scripts/ci-required-scope.v2.json"],
            capture_output=True, check=False,
        )
    except OSError as exc:
        raise CloseoutPublicationError(f"cannot read C1 authority config blob: {exc}") from exc
    if blob.returncode:
        raise CloseoutPublicationError("C1 planner authority has no readable required-scope config blob")
    if "sha256:" + hashlib.sha256(blob.stdout).hexdigest() != config_digest:
        raise CloseoutPublicationError("C1 planner-authority config differs from the validated projection")
    actual_scope = git("merge-base", authority_oid, head)
    if actual_scope != scope_oid:
        raise CloseoutPublicationError("C1 planner-authority merge-base differs from the frozen source scope")


def resolve_c1_planner_authority(root: Path, comments: list[dict[str, Any]],
                                 expected: dict[str, Any], comparison_oid: str,
                                 target_ref: str, head: str, scope_oid: str,
                                 config_digest: str, publication_module: Any) -> str:
    independent_fields = {key: value for key, value in expected.items()
                          if key != "planner_authority_oid"}
    authority_oid, _comment = c1_authority_locator(
        comments, independent_fields, publication_module,
    )
    if authority_oid != comparison_oid:
        verify_planner_authority(root, authority_oid, target_ref, head, scope_oid, config_digest)
    return authority_oid


def exact_issue_scalar(body: str, name: str, *, numeric: bool = False) -> str | None:
    lines = [line for line in body.splitlines() if line.startswith(f"- {name}:")]
    if len(lines) > 1:
        raise CloseoutPublicationError(f"Task Issue {name} binding is ambiguous")
    if not lines:
        return None
    match = re.fullmatch(rf"- {re.escape(name)}:\s*`([^`]+)`", lines[0])
    if match is None:
        raise CloseoutPublicationError(f"Task Issue {name} binding is malformed")
    value = match.group(1)
    if numeric and re.fullmatch(r"[1-9][0-9]*", value) is None:
        raise CloseoutPublicationError(f"Task Issue {name} binding is malformed")
    return value


def all_issue_comments(issue_number: int) -> list[dict[str, Any]]:
    pages = gh_json([
        "api", f"repos/{REPOSITORY}/issues/{issue_number}/comments?per_page=100",
        "--paginate", "--slurp",
    ], "complete Task Issue comment pagination")
    if not isinstance(pages, list) or any(not isinstance(page, list) for page in pages):
        raise CloseoutPublicationError("Task Issue comment pagination is incomplete or malformed")
    comments: list[dict[str, Any]] = []
    for page in pages:
        if any(not isinstance(comment, dict) for comment in page):
            raise CloseoutPublicationError("Task Issue comments contain a malformed row")
        comments.extend(page)
    return comments


def _one_live_pr(root: Path, issue_number: int, task_uid: str,
                 source_ref: str, target_ref: str, asserted_number: int) -> dict[str, Any]:
    owner = REPOSITORY.split("/", 1)[0]
    pages = gh_json([
        "api", f"repos/{REPOSITORY}/pulls?state=all&head={owner}:{source_ref}"
        f"&base={target_ref}&per_page=100", "--paginate", "--slurp",
    ], "complete reciprocal PR pagination")
    if not isinstance(pages, list) or any(not isinstance(page, list) for page in pages):
        raise CloseoutPublicationError("reciprocal PR pagination is incomplete or malformed")
    pulls = [item for page in pages for item in page if isinstance(item, dict)]
    if any(not isinstance(item, dict) for page in pages for item in page):
        raise CloseoutPublicationError("reciprocal PR pagination contains a malformed row")
    same_task: list[dict[str, Any]] = []
    pr_module = load_module(root, "review_closeout_pr_projection_publisher", "pr_projection_publish.py")
    for item in pulls:
        head = item.get("head") if isinstance(item.get("head"), dict) else {}
        base = item.get("base") if isinstance(item.get("base"), dict) else {}
        if (head.get("ref") == source_ref and base.get("ref") == target_ref
                and pr_module.has_exact_task_pr_linkage(item.get("body"), task_uid, issue_number)):
            same_task.append(item)
    if len(same_task) != 1:
        raise CloseoutPublicationError("Task Issue and refs do not resolve one unique reciprocal PR")
    item = same_task[0]
    if type(item.get("number")) is not int or item["number"] != asserted_number:
        raise CloseoutPublicationError("live reciprocal PR differs from the frozen review plan")
    return item


def resolve_context(root: Path, task_uid: str, plan: dict[str, Any]) -> dict[str, Any]:
    """Rebuild the closed C1 identity from live task/PR facts and frozen plan inputs."""
    root = root.resolve(strict=True)
    if not TASK_RE.fullmatch(task_uid):
        raise CloseoutPublicationError("Task UID is malformed")
    identity = plan.get("source_review_identity")
    raw_projection = plan.get("impact_projection")
    if not isinstance(identity, dict) or not isinstance(raw_projection, dict):
        raise CloseoutPublicationError("v2 review plan lacks its frozen identity/projection")
    if (identity.get("task_uid") != task_uid or plan.get("task_uid") != task_uid
            or identity.get("source_head_oid") != plan.get("frozen_head")
            or identity.get("repository") != REPOSITORY):
        raise CloseoutPublicationError("frozen review identity does not match the task/repository")
    require_positive_int(identity.get("bootstrap_epoch"), "frozen bootstrap epoch")
    head = identity.get("source_head_oid")
    scope_oid = identity.get("source_scope_oid")
    comparison_oid = plan.get("comparison_oid")
    if (not isinstance(head, str) or OID_RE.fullmatch(head) is None
            or not isinstance(scope_oid, str) or OID_RE.fullmatch(scope_oid) is None
            or not isinstance(comparison_oid, str) or OID_RE.fullmatch(comparison_oid) is None
            or comparison_oid != scope_oid):
        raise CloseoutPublicationError("frozen comparison or source scope is malformed")
    handoff_module = load_module(root, "review_closeout_handoff_validation", "review_preflight_handoff.py")
    try:
        projection = handoff_module.validate_plan_impact_projection(root, plan, identity)
    except (OSError, TypeError, ValueError) as exc:
        raise CloseoutPublicationError(f"frozen impact projection is not independently valid: {exc}") from exc
    if (projection.get("task_uid") != task_uid or projection.get("source_head_oid") != head
            or projection.get("scope_base_oid") != scope_oid
            or str(projection.get("projection_digest", "")).removeprefix("sha256:")
            != identity.get("input_contract_digest")):
        raise CloseoutPublicationError("impact projection differs from the frozen source-review identity")
    planner_config = projection.get("planner_config_sha256")
    planner_digest = projection.get("planner_digest")
    digest_pattern = r"sha256:[0-9a-f]{64}"
    if (not isinstance(planner_config, str) or re.fullmatch(digest_pattern, planner_config) is None
            or not isinstance(planner_digest, str) or re.fullmatch(digest_pattern, planner_digest) is None
            or projection.get("projection_digest") != plan.get("impact_projection_digest")
            or projection.get("planner_digest") != plan.get("impact_projection_planner_digest")):
        raise CloseoutPublicationError("frozen impact projection lacks exact planner digests")

    mapping_path = root / ".pm" / "github-project-sync" / "tasks.json"
    try:
        mapping = json.loads(mapping_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise CloseoutPublicationError(f"canonical task mapping is unreadable: {exc}") from exc
    tasks = mapping.get("tasks") if isinstance(mapping, dict) else None
    task_record = tasks.get(task_uid) if isinstance(tasks, dict) else None
    if not isinstance(task_record, dict):
        raise CloseoutPublicationError("canonical task mapping has no exact UID")
    issue_number = require_positive_int(task_record.get("issue_number"), "canonical Task Issue number")
    if task_record.get("repository") != REPOSITORY:
        raise CloseoutPublicationError("canonical task mapping repository differs")
    if Path(str(task_record.get("canonical_worktree") or "")).expanduser().resolve() != root:
        raise CloseoutPublicationError("canonical task mapping worktree differs")
    source_ref = task_record.get("task_branch")
    target_ref = task_record.get("default_branch")
    if not isinstance(source_ref, str) or not source_ref.strip() or not isinstance(target_ref, str) or not target_ref.strip():
        raise CloseoutPublicationError("canonical task mapping refs are missing")

    try:
        repository = gh_json(["api", f"repos/{REPOSITORY}"], "canonical repository identity")
    except CloseoutPublicationError:
        raise
    repository_id = require_positive_int(repository.get("id") if isinstance(repository, dict) else None,
                                        "live canonical repository ID")
    if not isinstance(repository, dict) or repository.get("full_name") != REPOSITORY or repository.get("default_branch") != target_ref:
        raise CloseoutPublicationError("live repository default branch or identity differs from task mapping")

    issue = gh_json(["api", f"repos/{REPOSITORY}/issues/{issue_number}"], "canonical Task Issue")
    if not isinstance(issue, dict) or issue.get("number") != issue_number or str(issue.get("state", "")).lower() != "open":
        raise CloseoutPublicationError("canonical Task Issue is missing, closed, or mismatched")
    issue_url = f"https://github.com/{REPOSITORY}/issues/{issue_number}"
    if issue.get("html_url") != issue_url:
        raise CloseoutPublicationError("canonical Task Issue URL differs")
    body = issue.get("body")
    if not isinstance(body, str) or body.count("<!-- oasis7-pm-task -->") != 1:
        raise CloseoutPublicationError("canonical Task Issue marker is missing or duplicated")
    uid_values = re.findall(r"(?m)^task_uid:\s*(task_[0-9a-f]{32})\s*$", body)
    if uid_values != [task_uid]:
        raise CloseoutPublicationError("canonical Task Issue UID differs")
    c1_epoch = c1_bootstrap_epoch(root, task_uid, task_record, body)
    status = exact_issue_scalar(body, "status")
    phase = exact_issue_scalar(body, "workflow_phase")
    if (status != task_record.get("status") or phase != task_record.get("workflow_phase")
            or not status or not phase):
        raise CloseoutPublicationError("live Task Issue status/phase differs from canonical mapping")
    raw_pr_number = exact_issue_scalar(body, "pr_number", numeric=True)
    pr_url = exact_issue_scalar(body, "pr_url")
    if (raw_pr_number is None) != (pr_url is None):
        raise CloseoutPublicationError("Task Issue PR binding is partial")
    issue_pr_number = int(raw_pr_number) if raw_pr_number is not None else None
    if pr_url is not None and pr_url != f"https://github.com/{REPOSITORY}/pull/{issue_pr_number}":
        raise CloseoutPublicationError("Task Issue PR URL/number pair conflicts")

    asserted_pr_number = require_positive_int(identity.get("pr_number"), "frozen review PR number")
    if issue_pr_number is not None and issue_pr_number != asserted_pr_number:
        raise CloseoutPublicationError("live Task Issue PR binding differs from frozen review identity")
    expected_pr_url = f"https://github.com/{REPOSITORY}/pull/{asserted_pr_number}"
    if issue_pr_number != asserted_pr_number or pr_url != expected_pr_url:
        raise CloseoutPublicationError("C1 closeout requires the exact already-bound Task PR number and URL")
    pull = _one_live_pr(root, issue_number, task_uid, source_ref, target_ref, asserted_pr_number)
    head_info = pull.get("head") if isinstance(pull.get("head"), dict) else {}
    base_info = pull.get("base") if isinstance(pull.get("base"), dict) else {}
    user = pull.get("user") if isinstance(pull.get("user"), dict) else {}
    task_author = issue.get("user") if isinstance(issue.get("user"), dict) else {}
    merged = pull.get("merged")
    if type(merged) is not bool:
        merged = pull.get("merged_at") is not None
    if (pull.get("number") != asserted_pr_number or pull.get("state") != "open" or merged
            or head_info.get("ref") != source_ref or head_info.get("sha") != head
            or not isinstance(head_info.get("repo"), dict)
            or head_info["repo"].get("full_name") != REPOSITORY
            or base_info.get("ref") != target_ref
            or not isinstance(base_info.get("repo"), dict)
            or base_info["repo"].get("full_name") != REPOSITORY
            or pull.get("draft") is not True):
        raise CloseoutPublicationError("live PR is not the exact open same-repository frozen draft")
    if (not isinstance(task_author.get("login"), str) or not task_author.get("login")
            or task_author.get("type") != "User" or not isinstance(user.get("login"), str)
            or user.get("type") != "User"):
        raise CloseoutPublicationError("Task Issue or PR author provenance is unavailable")

    comments = all_issue_comments(issue_number)
    publication_module = load_module(root, "review_closeout_c1_publication", "pr_projection_publication.py")
    expected = {
        "repository": REPOSITORY,
        "repository_id": repository_id,
        "task_uid": task_uid,
        # C1's historical Task publication epoch is distinct from the v2
        # review epoch, which may be snapshot-derived for legacy tasks.
        "bootstrap_epoch": c1_epoch,
        "source_repository_id": repository_id,
        "source_ref": source_ref,
        "target_ref": target_ref,
        "source_head_oid": head,
        "source_scope_oid": scope_oid,
        "planner_authority_oid": comparison_oid,
        "planner_config_sha256": planner_config,
        # C1's publication policy digest is the trusted projection planner digest;
        # it is not the legacy loop-policy pin digest.
        "policy_digest": planner_digest,
        "projection_digest": projection["projection_digest"],
    }
    authority_oid = resolve_c1_planner_authority(
        root, comments, expected, comparison_oid, target_ref, head, scope_oid,
        planner_config, publication_module,
    )
    expected["planner_authority_oid"] = authority_oid
    result = publication_module.resolve_task_publication(
        {"complete": True, "repository": REPOSITORY, "issue_number": issue_number,
         "comments": comments},
        expected,
        live_task_author={"login": task_author["login"], "type": task_author["type"]},
        pr_binding={
            "repository": REPOSITORY, "number": asserted_pr_number,
            "url": f"https://github.com/{REPOSITORY}/pull/{asserted_pr_number}",
            "state": pull.get("state"), "merged": merged, "draft": pull.get("draft"),
            "source_ref": head_info.get("ref"), "target_ref": base_info.get("ref"),
            "source_head_oid": head_info.get("sha"), "task_uid": task_uid,
            "issue_number": issue_number, "task_pr_number": issue_pr_number,
            "task_pr_url": pr_url, "pr_author": user.get("login"),
            "pr_author_type": user.get("type"), "created_at": pull.get("created_at"),
            "updated_at": pull.get("updated_at"), "task_status": status,
            "task_phase": phase,
        },
    )
    if result.get("status") != "passed" or not isinstance(result.get("publication"), dict):
        blockers = result.get("blockers")
        detail = "; ".join(str(item) for item in blockers) if isinstance(blockers, list) else "C1 resolution failed"
        raise CloseoutPublicationError(f"exact trusted C1 publication is unavailable: {detail}")
    publication = result["publication"]

    branch = subprocess.run(["git", "-C", str(root), "branch", "--show-current"],
                            text=True, capture_output=True, check=False)
    common = subprocess.run(["git", "-C", str(root), "rev-parse", "--git-common-dir"],
                            text=True, capture_output=True, check=False)
    current_head = subprocess.run(["git", "-C", str(root), "rev-parse", "HEAD^{commit}"],
                                  text=True, capture_output=True, check=False)
    if branch.returncode or common.returncode or current_head.returncode:
        raise CloseoutPublicationError("cannot resolve current source worktree identity")
    if branch.stdout.strip() != source_ref or current_head.stdout.strip() != head:
        raise CloseoutPublicationError("current source branch/HEAD differs from exact C1 review candidate")
    common_dir = Path(common.stdout.strip())
    if not common_dir.is_absolute():
        common_dir = (root / common_dir).resolve()
    else:
        common_dir = common_dir.resolve()
    journal_module = load_module(root, "review_closeout_c1_journal", "pr_projection_journal.py")
    journal = journal_module.open_journal(
        common_dir, publication["repository"], publication["source_ref"],
        publication["publication_id"], task_uid=publication["task_uid"],
        source_head_oid=publication["source_head_oid"],
        scope_base_oid=publication["source_scope_oid"],
        projection_digest=publication["projection_digest"],
    )
    return {"journal": journal, "publication": publication, "issue_number": issue_number,
            "root": root, "task_uid": task_uid, "head": head}


def current_admin_login() -> str:
    current_user = gh_json(["api", "user"], "authenticated GitHub user")
    login = current_user.get("login") if isinstance(current_user, dict) else None
    if not isinstance(login, str) or not login:
        raise CloseoutPublicationError("authenticated GitHub user is unavailable before comment publication")
    permission = gh_json([
        "api", f"repos/{REPOSITORY}/collaborators/{quote(login, safe='')}/permission",
    ], "current comment-publisher permission")
    if not isinstance(permission, dict) or permission.get("permission") != "admin":
        raise CloseoutPublicationError("review comment publication requires current repository-admin permission")
    return login


def publish_comment(context: dict[str, Any], *, action_id: str, kind: str,
                    expected: dict[str, Any], body: str,
                    find_matches: Callable[[list[dict[str, Any]]], list[dict[str, Any]]],
                    verify: Callable[[dict[str, Any]], dict[str, Any]]) -> dict[str, Any]:
    """Publish/reconcile one append action under the candidate C1 journal lock."""
    journal = context["journal"]
    issue_number = int(context["issue_number"])
    with journal.locked():
        actions = [item for item in journal.read()["actions"] if item.get("action_id") == action_id]
        if len(actions) > 1:
            journal.disposition("CONFLICT")
            raise CloseoutPublicationError("duplicate closeout publication actions exist")
        prior = actions[0] if actions else None
        try:
            comments = all_issue_comments(issue_number)
        except CloseoutPublicationError:
            if prior is not None:
                journal.uncertain(action_id, "NETWORK_UNCERTAIN")
                raise CloseoutPublicationError("closeout publication is pending; exact live readback is unavailable")
            raise
        matches = find_matches(comments)
        if len(matches) > 1:
            journal.disposition("CONFLICT")
            raise CloseoutPublicationError("duplicate semantic closeout comments exist")
        if matches:
            if prior is None:
                journal.intent(action_id, kind, expected)
            try:
                verified = verify(matches[0])
            except ValueError as exc:
                journal.uncertain(action_id, "NETWORK_UNCERTAIN")
                raise CloseoutPublicationError(
                    f"closeout publication is pending; exact direct readback failed: {exc}"
                ) from exc
            observed = {"comment_id": verified["comment_id"], "body_digest": verified["body_digest"],
                        "author": verified["author"], "issue_number": issue_number}
            journal.observe(action_id, observed, phase="METADATA_CONFIRMED")
            return {"status": "already_published", **verified}
        if prior is not None:
            journal.uncertain(action_id, "NETWORK_UNCERTAIN")
            raise CloseoutPublicationError(
                "closeout publication is pending; prior append has no unique visible action; refusing duplicate POST"
            )

        # All identity, C1, Issue/PR, pagination, and permission preflight is
        # complete. Persist the existing journal intent immediately before the
        # one non-idempotent append.
        current_admin_login()
        current_head = subprocess.run(
            ["git", "-C", str(context["root"]), "rev-parse", "HEAD^{commit}"],
            text=True, capture_output=True, check=False,
        )
        if current_head.returncode or current_head.stdout.strip() != context["head"]:
            raise CloseoutPublicationError("source HEAD changed before review comment publication")
        journal.intent(action_id, kind, expected)
        try:
            result = subprocess.run([
                "gh", "api", f"repos/{REPOSITORY}/issues/{issue_number}/comments",
                "--method", "POST", "--field", f"body={body}",
            ], text=True, capture_output=True, check=False)
            post_error = result.stderr.strip() if result.returncode else ""
        except OSError as exc:
            post_error = str(exc)
        try:
            comments = all_issue_comments(issue_number)
            matches = find_matches(comments)
        except ValueError as exc:
            journal.uncertain(action_id, "NETWORK_UNCERTAIN")
            suffix = f" ({post_error})" if post_error else ""
            raise CloseoutPublicationError(
                f"closeout publication is pending after possible append; exact live readback failed{suffix}: {exc}"
            ) from exc
        if len(matches) != 1:
            if len(matches) > 1:
                journal.disposition("CONFLICT")
                raise CloseoutPublicationError("closeout append has duplicate semantic readback")
            journal.uncertain(action_id, "NETWORK_UNCERTAIN")
            suffix = f": {post_error}" if post_error else ""
            raise CloseoutPublicationError(
                "closeout publication is pending after possible append; exact live readback is not unique" + suffix
            )
        try:
            verified = verify(matches[0])
        except ValueError as exc:
            journal.uncertain(action_id, "NETWORK_UNCERTAIN")
            raise CloseoutPublicationError(f"closeout publication is pending; direct readback failed: {exc}") from exc
        observed = {"comment_id": verified["comment_id"], "body_digest": verified["body_digest"],
                    "author": verified["author"], "issue_number": issue_number}
        journal.observe(action_id, observed, phase="METADATA_CONFIRMED")
        return {"status": "published", **verified}
