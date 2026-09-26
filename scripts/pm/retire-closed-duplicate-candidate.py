#!/usr/bin/env python3
"""Retire a closed duplicate candidate from one GitHub Project task mapping.

The CLI pins its code and persistence helpers to this checkout, while
``--mapping-root`` names the registered target worktree whose cache is being
reconciled. GitHub and Git are read-only evidence sources; the only mutation is
one lock-held atomic rewrite of that target ``tasks.json``.
"""
from __future__ import annotations

import argparse
import copy
import datetime as dt
import hashlib
import importlib.util
import json
import os
import pathlib
import re
import shutil
import stat
import subprocess
import sys
import uuid
from typing import Any


SCHEMA = "oasis7.duplicate-candidate-retirement/v1"
DISPOSITION_MARKER = "<!-- oasis7.duplicate-candidate-disposition/v1 -->"
TASK_UID_RE = re.compile(r"task_[0-9a-f]{32}\Z")
GITHUB_REPO_RE = re.compile(r"[^/\s]+/[^/\s]+\Z")
PROJECT_FIELD_CONFIGURATION_NAME_SELECTION = (
    "... on ProjectV2Field { name } "
    "... on ProjectV2IterationField { name } "
    "... on ProjectV2MultiSelectField { name } "
    "... on ProjectV2SingleSelectField { name }"
)
ROOT = pathlib.Path(__file__).resolve().parents[2]
STORE_PATH = pathlib.Path(__file__).with_name("workflow-durable-store.py")
STORE_SPEC = importlib.util.spec_from_file_location("workflow_durable_store_retirement", STORE_PATH)
if STORE_SPEC is None or STORE_SPEC.loader is None:
    raise RuntimeError(f"cannot load pinned durable store at {STORE_PATH}")
STORE = importlib.util.module_from_spec(STORE_SPEC)
STORE_SPEC.loader.exec_module(STORE)


# ValueError is intentionally the shared exception identity: deterministic
# contract tests and downstream dynamic loaders may load this file under
# distinct module names in one process.
RetirementError = ValueError


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def digest_bytes(value: bytes) -> str:
    return "sha256:" + hashlib.sha256(value).hexdigest()


def canonical_digest(value: Any) -> str:
    return digest_bytes(canonical_bytes(value))


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise RetirementError(message)


def _run(command: list[str], *, cwd: pathlib.Path | None = None) -> str:
    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=60,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise RetirementError(f"could not complete read-only command {command[0]}: {exc}") from exc
    if result.returncode:
        detail = result.stderr.strip() or result.stdout.strip() or f"exit {result.returncode}"
        raise RetirementError(f"read-only command {' '.join(command[:3])} failed: {detail}")
    return result.stdout.strip()


def _gh_json(*arguments: str) -> Any:
    output = _run(["gh", "api", *arguments])
    try:
        return json.loads(output)
    except json.JSONDecodeError as exc:
        raise RetirementError(f"GitHub returned invalid JSON for {arguments[0]}: {exc}") from exc


def _gh_pages(*arguments: str) -> list[Any]:
    output = _run(["gh", "api", *arguments, "--paginate", "--slurp"])
    try:
        payload = json.loads(output)
    except json.JSONDecodeError as exc:
        raise RetirementError(f"GitHub returned invalid paginated JSON: {exc}") from exc
    if isinstance(payload, list):
        return payload
    return [payload]


def _flatten_rest_pages(pages: list[Any], label: str) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for page in pages:
        # gh --paginate --slurp wraps each REST page in an array.
        values = page if isinstance(page, list) else [page]
        if not isinstance(values, list) or any(not isinstance(value, dict) for value in values):
            raise RetirementError(f"{label} pagination returned a malformed page")
        records.extend(values)
    return records


def _graphql_pages(query: str, **variables: str | int) -> list[dict[str, Any]]:
    # A future accidental mutation query must not ride through the read-only
    # authority collector merely because gh accepts the endpoint.
    if not query.lstrip().startswith("query ") or re.search(r"\bmutation\b", query, re.IGNORECASE):
        raise RetirementError("only read-only GraphQL query documents are permitted")
    def decode_pages(output: str) -> list[dict[str, Any]]:
        try:
            value = json.loads(output)
        except json.JSONDecodeError as exc:
            raise RetirementError(f"GitHub GraphQL returned invalid paginated JSON: {exc}") from exc
        if isinstance(value, dict):
            value = [value]
        if not isinstance(value, list) or not value or any(not isinstance(page, dict) for page in value):
            raise RetirementError("GitHub GraphQL pagination returned no complete pages")
        for page in value:
            if page.get("errors"):
                raise RetirementError(f"GitHub GraphQL read failed: {page['errors']}")
        return value

    def connection_info(page: dict[str, Any]) -> dict[str, Any]:
        data = page.get("data")
        if not isinstance(data, dict):
            raise RetirementError("GitHub GraphQL pagination page has no data object")
        if "items(first:" in query:
            organization = data.get("organization")
            user = data.get("user")
            owner = organization if isinstance(organization, dict) else user
            project = owner.get("projectV2") if isinstance(owner, dict) else None
            if not isinstance(project, dict):
                project = data.get("projectV2")
            connection = project.get("items") if isinstance(project, dict) else None
        elif "issues(first:" in query:
            repository = data.get("repository")
            connection = repository.get("issues") if isinstance(repository, dict) else None
        else:
            raise RetirementError("GitHub GraphQL query has no supported paginated connection")
        if not isinstance(connection, dict):
            raise RetirementError("GitHub GraphQL pagination connection is malformed")
        nodes = connection.get("nodes")
        info = connection.get("pageInfo")
        if not isinstance(nodes, list) or any(not isinstance(node, dict) for node in nodes):
            raise RetirementError("GitHub GraphQL pagination nodes are malformed")
        if not isinstance(info, dict) or not isinstance(info.get("hasNextPage"), bool):
            raise RetirementError("GitHub GraphQL pagination metadata is incomplete")
        cursor = info.get("endCursor")
        if cursor is not None and (not isinstance(cursor, str) or not cursor):
            raise RetirementError("GitHub GraphQL pagination cursor is malformed")
        if info["hasNextPage"] and not isinstance(cursor, str):
            raise RetirementError("GitHub GraphQL pagination advertises a next page without a cursor")
        return info

    def request(cursor: str | None = None) -> list[dict[str, Any]]:
        arguments = ["graphql"]
        if cursor is None:
            arguments.extend(["--paginate", "--slurp"])
        for key, value in variables.items():
            arguments.extend(["-F", f"{key}={value}"])
        if cursor is not None:
            arguments.extend(["-F", f"endCursor={cursor}"])
        arguments.extend(["-f", f"query={query}"])
        return decode_pages(_run(["gh", "api", *arguments]))

    pages = request()
    # `gh --paginate` normally consumes the GraphQL pageInfo chain itself.
    # If a transport returns a truncated prefix while its final page still
    # advertises a cursor, explicitly resume from that cursor and validate the
    # rest rather than accepting partial authority or stopping prematurely.
    seen_cursors: set[str] = set()
    while connection_info(pages[-1]).get("hasNextPage") is True:
        cursor = connection_info(pages[-1]).get("endCursor")
        if not isinstance(cursor, str) or not cursor or cursor in seen_cursors:
            raise RetirementError("GitHub GraphQL pagination ended without a fresh continuation cursor")
        if len(seen_cursors) >= 1000:
            raise RetirementError("GitHub GraphQL pagination exceeded the bounded continuation limit")
        seen_cursors.add(cursor)
        pages.extend(request(cursor))
    for page in pages:
        connection_info(page)
    return pages


def _repository_identity(url: str) -> str:
    normalized = url.strip()
    if normalized.startswith("git@github.com:"):
        normalized = normalized[len("git@github.com:"):]
    elif normalized.startswith("ssh://git@github.com/"):
        normalized = normalized[len("ssh://git@github.com/"):]
    elif normalized.startswith("https://github.com/"):
        normalized = normalized[len("https://github.com/"):]
    normalized = normalized.removesuffix(".git").strip("/")
    return normalized if GITHUB_REPO_RE.fullmatch(normalized) else ""


def _normalized_repo(value: Any, fallback: str = "") -> str:
    if isinstance(value, dict):
        value = value.get("full_name") or value.get("nameWithOwner") or ""
    return str(value or fallback)


def _issue_from_rest(raw: Any, repository: str, issue_number: int) -> dict[str, Any]:
    _require(isinstance(raw, dict), f"Issue #{issue_number} read is not an object")
    number = raw.get("number")
    _require(str(number) == str(issue_number), f"Issue #{issue_number} read returned a different number")
    return {
        "repository": _normalized_repo(raw.get("repository"), repository),
        "number": int(number),
        "url": str(raw.get("html_url") or raw.get("url") or f"https://github.com/{repository}/issues/{issue_number}"),
        "state": str(raw.get("state") or "").upper(),
        "state_reason": str(raw.get("state_reason") or raw.get("stateReason") or "").upper() or None,
        "body": str(raw.get("body") or ""),
        "task_uid": _task_uid_from_body(str(raw.get("body") or "")),
    }


def _extract_issue_nodes(page: dict[str, Any]) -> tuple[list[dict[str, Any]], bool | None]:
    data = page.get("data")
    repository = data.get("repository") if isinstance(data, dict) else None
    connection = repository.get("issues") if isinstance(repository, dict) else None
    if isinstance(connection, dict):
        nodes = connection.get("nodes")
        info = connection.get("pageInfo")
        if (
            not isinstance(nodes, list)
            or any(not isinstance(item, dict) for item in nodes)
            or not isinstance(info, dict)
            or not isinstance(info.get("hasNextPage"), bool)
        ):
            raise RetirementError("GitHub Issue pagination connection is malformed")
        cursor = info.get("endCursor")
        if cursor is not None and (not isinstance(cursor, str) or not cursor):
            raise RetirementError("GitHub Issue pagination cursor is malformed")
        if info["hasNextPage"] and not isinstance(cursor, str):
            raise RetirementError("GitHub Issue pagination is incomplete: next-page cursor is missing")
        return [item for item in nodes if isinstance(item, dict)], info.get("hasNextPage")
    raise RetirementError("GitHub Issue pagination did not return an Issues connection")


def _collect_repository_issues(owner: str, name: str, repository: str) -> list[dict[str, Any]]:
    query = (
        "query RetirementIssues($owner: String!, $name: String!, $endCursor: String) { "
        "repository(owner: $owner, name: $name) { issues(first: 100, states: [OPEN, CLOSED], after: $endCursor) { "
        "nodes { number url state stateReason body } pageInfo { hasNextPage endCursor } } } }"
    )
    pages = _graphql_pages(query, owner=owner, name=name)
    found: dict[int, dict[str, Any]] = {}
    has_next_seen: list[bool | None] = []
    for page in pages:
        nodes, has_next = _extract_issue_nodes(page)
        has_next_seen.append(has_next)
        for raw in nodes:
            issue = _issue_from_rest(raw, repository, int(raw.get("number") or 0))
            previous = found.get(issue["number"])
            if previous is not None and previous != issue:
                raise RetirementError(f"conflicting duplicate live Issue #{issue['number']} across pages")
            found[issue["number"]] = issue
    if any(value is True for value in has_next_seen[-1:]):
        raise RetirementError("GitHub Issue pagination ended with hasNextPage=true")
    if not found:
        raise RetirementError("complete GitHub Issue pagination returned no Issues")
    return list(found.values())


def _require_issue_in_complete_collection(
    complete_issues: list[dict[str, Any]], direct_issue: dict[str, Any], label: str,
) -> None:
    issue_number = int(direct_issue.get("number") or 0)
    matches = [issue for issue in complete_issues if int(issue.get("number") or 0) == issue_number]
    _require(len(matches) == 1, f"complete Issue collection does not uniquely include the directly read {label} Issue")
    _require(
        matches[0] == direct_issue,
        f"direct {label} Issue read differs from the complete paginated Issue collection",
    )


def _project_item_fields(raw: dict[str, Any]) -> tuple[dict[str, str], bool]:
    connection = raw.get("fieldValues")
    if not isinstance(connection, dict) or not isinstance(connection.get("nodes"), list):
        raise RetirementError("Project item fieldValues are unavailable")
    info = connection.get("pageInfo")
    if not isinstance(info, dict) or info.get("hasNextPage") is not False:
        raise RetirementError("Project item fieldValues pagination is incomplete")
    fields: dict[str, str] = {}
    for value in connection["nodes"]:
        if not isinstance(value, dict):
            continue
        field = value.get("field")
        name = str(field.get("name") or "") if isinstance(field, dict) else ""
        if not name:
            continue
        fields[name] = str(value.get("name") if value.get("name") is not None else value.get("text") or value.get("value") or "")
    return fields, info.get("hasNextPage") is False


def _project_node_to_item(raw: dict[str, Any], repository: str, project: dict[str, Any]) -> dict[str, Any] | None:
    content = raw.get("content")
    if not isinstance(content, dict) or content.get("__typename") != "Issue":
        return None
    fields, complete = _project_item_fields(raw)
    return {
        "id": str(raw.get("id") or ""),
        "project_id": str(project.get("id") or ""),
        "project_owner": str(project.get("owner") or ""),
        "project_number": int(project.get("number") or 0),
        "repository": _normalized_repo(content.get("repository"), repository),
        "issue_number": int(content.get("number") or 0),
        "issue_url": str(content.get("url") or ""),
        "task_uid": _task_uid_from_body(str(content.get("body") or "")),
        "archived": raw.get("isArchived") is True,
        "fields": fields,
        "_field_values_complete": complete,
    }


def _extract_project_connection(page: dict[str, Any]) -> tuple[dict[str, Any], dict[str, Any]]:
    data = page.get("data")
    organization = data.get("organization") if isinstance(data, dict) else None
    user = data.get("user") if isinstance(data, dict) else None
    owner = organization if isinstance(organization, dict) else user
    project = owner.get("projectV2") if isinstance(owner, dict) else None
    if not isinstance(project, dict):
        data_project = data.get("projectV2") if isinstance(data, dict) else None
        project = data_project if isinstance(data_project, dict) else None
    if not isinstance(project, dict):
        raise RetirementError("canonical GitHub Project was not returned")
    items = project.get("items")
    if isinstance(items, dict) and isinstance(items.get("nodes"), list):
        return project, items
    # Compatibility with query fixture output, while still requiring pageInfo.
    nodes = data.get("nodes") if isinstance(data, dict) else None
    if isinstance(nodes, list):
        return project, {"nodes": nodes, "pageInfo": {"hasNextPage": False, "endCursor": None}}
    raise RetirementError("canonical GitHub Project items connection is malformed")


def _collect_project_items(owner: str, number: int, repository: str) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    def query_for(owner_kind: str) -> str:
        return (
            "query RetirementProject($owner: String!, $number: Int!, $endCursor: String) { "
            + owner_kind
            + "(login: $owner) { projectV2(number: $number) { id number owner { "
            "... on Organization { login } ... on User { login } } "
            "items(first: 100, after: $endCursor) { nodes { id isArchived content { __typename "
            "... on Issue { number url state stateReason body repository { nameWithOwner } } } "
            "fieldValues(first: 100) { nodes { ... on ProjectV2ItemFieldSingleSelectValue { name field { "
            + PROJECT_FIELD_CONFIGURATION_NAME_SELECTION
            + " } } ... on ProjectV2ItemFieldTextValue { text field { "
            + PROJECT_FIELD_CONFIGURATION_NAME_SELECTION
            + " } } } pageInfo { hasNextPage endCursor } } } "
            "pageInfo { hasNextPage endCursor } } } } }"
        )

    query = query_for("organization")
    try:
        pages = _graphql_pages(query, owner=owner, number=number)
    except RetirementError as exc:
        # owner may name a personal account rather than an Organization.
        # Fall back only on GitHub's explicit owner-type resolution error;
        # auth, transport, schema, and pagination failures remain blockers.
        owner_not_organization = f"Could not resolve to an Organization with the login of '{owner}'."
        if owner_not_organization not in str(exc):
            raise
        query = query_for("user")
        pages = _graphql_pages(query, owner=owner, number=number)
    expected_project: dict[str, Any] | None = None
    items: list[dict[str, Any]] = []
    pagination: list[bool | None] = []
    for page in pages:
        raw_project, connection = _extract_project_connection(page)
        project_owner = raw_project.get("owner")
        project_owner = str(project_owner.get("login") or "") if isinstance(project_owner, dict) else str(project_owner or "")
        project = {
            "owner": project_owner,
            "number": int(raw_project.get("number") or 0),
            "id": str(raw_project.get("id") or ""),
            "repo": repository,
        }
        if expected_project is not None and project != expected_project:
            raise RetirementError("GitHub Project identity changed across paginated reads")
        expected_project = project
        info = connection.get("pageInfo")
        if not isinstance(info, dict):
            raise RetirementError("GitHub Project pagination metadata is missing")
        pagination.append(info.get("hasNextPage"))
        nodes = connection.get("nodes")
        if not isinstance(nodes, list):
            raise RetirementError("GitHub Project page has no item nodes")
        for node in nodes:
            if isinstance(node, dict):
                item = _project_node_to_item(node, repository, project)
                if item is not None:
                    items.append(item)
    if not expected_project or not items:
        raise RetirementError("complete GitHub Project read returned no Issue items")
    if pagination[-1] is not False:
        raise RetirementError("GitHub Project pagination ended without hasNextPage=false")
    # gh --paginate retrieves all pages; duplicate nodes across pages are
    # accepted only when they are byte-for-byte the same Project identity.
    by_id: dict[str, dict[str, Any]] = {}
    for item in items:
        if not item["id"]:
            raise RetirementError("GitHub Project item has no server identity")
        previous = by_id.get(item["id"])
        if previous is not None and previous != item:
            raise RetirementError(f"conflicting Project item {item['id']} across pages")
        by_id[item["id"]] = item
    return expected_project, list(by_id.values())


def _task_uid_from_body(body: str) -> str:
    matches = re.findall(r"(?m)^task_uid:\s*(task_[0-9a-f]{32})\s*$", body)
    return matches[0] if len(matches) == 1 else ""


def _issue_uid_markers(issue: dict[str, Any]) -> set[str]:
    body = str(issue.get("body") or "")
    markers = set(re.findall(r"(?m)^task_uid:\s*(task_[0-9a-f]{32})\s*$", body))
    declared = issue.get("task_uid")
    if isinstance(declared, str) and TASK_UID_RE.fullmatch(declared):
        markers.add(declared)
    return markers


def _require_unique_issue_uid(
    issues: list[dict[str, Any]], task_uid: str, issue_number: int, label: str,
) -> dict[str, Any]:
    matches = [issue for issue in issues if task_uid in _issue_uid_markers(issue)]
    _require(len(matches) == 1, f"{label} Issue Task UID is duplicated or ambiguous across Issues")
    _require(
        int(matches[0].get("number") or 0) == issue_number,
        f"{label} Issue Task UID is bound to a different Issue number",
    )
    return matches[0]


def _require_unique_project_uid(
    items: list[dict[str, Any]], repository: str, task_uid: str, issue_number: int, label: str,
) -> dict[str, Any]:
    matches = [item for item in items if item.get("task_uid") == task_uid]
    _require(len(matches) == 1, f"{label} Project Task UID is duplicated or ambiguous across Project items")
    item = matches[0]
    _require(
        item.get("repository") == repository and int(item.get("issue_number") or 0) == issue_number,
        f"{label} Project Task UID is bound to a different Issue identity",
    )
    return item


def _issue_metadata(issue: dict[str, Any]) -> dict[str, str]:
    body = str(issue.get("body") or "")
    task_uid = _task_uid_from_body(body)
    fields = {
        "task_uid": task_uid,
        "status": "",
        "workflow_phase": "",
        "worktree_hint": "",
        "pr_number": "",
        "pr_url": "",
    }
    for key in tuple(fields)[1:]:
        matches = re.findall(rf"(?m)^- {re.escape(key)}:\s*`([^`]*)`\s*$", body)
        if len(matches) == 1:
            fields[key] = matches[0].strip()
    return fields


def _normalize_issue_for_proof(issue: dict[str, Any]) -> dict[str, Any]:
    return {
        "repository": issue["repository"],
        "number": issue["number"],
        "url": issue["url"],
        "state": issue["state"],
        "state_reason": issue["state_reason"],
        "body": issue["body"],
    }


def _comments_for_issue(repo: str, issue_number: int) -> list[dict[str, Any]]:
    pages = _gh_pages(f"repos/{repo}/issues/{issue_number}/comments?per_page=100")
    return _flatten_rest_pages(pages, "Issue comment")


def _normalize_comment(raw: dict[str, Any], issue_number: int) -> dict[str, Any]:
    user = raw.get("user")
    if not isinstance(user, dict):
        user = raw.get("author") if isinstance(raw.get("author"), dict) else {}
    return {
        "id": int(raw.get("id") or 0),
        "issue_number": int(raw.get("issue_number") or raw.get("issue_url_number") or issue_number),
        "user": {"login": str(user.get("login") or "")},
        "body": str(raw.get("body") or ""),
        "html_url": str(raw.get("html_url") or raw.get("url") or ""),
        "created_at": str(raw.get("created_at") or ""),
    }


def _pull_request(raw: dict[str, Any], repository: str) -> dict[str, Any]:
    head = raw.get("head") if isinstance(raw.get("head"), dict) else {}
    base = raw.get("base") if isinstance(raw.get("base"), dict) else {}
    head_repo = head.get("repo") if isinstance(head.get("repo"), dict) else {}
    base_repo = base.get("repo") if isinstance(base.get("repo"), dict) else {}
    body = str(raw.get("body") or "")
    uid = str(raw.get("task_uid") or _task_uid_from_body(body))
    issue_number = raw.get("issue_number")
    if issue_number is None:
        references = re.findall(r"(?i)\b(?:close[sd]?|fix(?:e[sd])?|resolve[sd]?)\s+#(\d+)\b", body)
        issue_number = int(references[0]) if len(set(references)) == 1 else None
    return {
        "number": int(raw.get("number") or 0),
        "state": str(raw.get("state") or "").upper(),
        "repository": _normalized_repo(raw.get("repository"), repository),
        "head_branch": str(raw.get("head_branch") or head.get("ref") or ""),
        "head_repository": _normalized_repo(raw.get("head_repository") or head_repo, repository),
        "base_repository": _normalized_repo(raw.get("base_repository") or base_repo, repository),
        "issue_number": int(issue_number) if issue_number not in (None, "") else None,
        "task_uid": uid,
        "url": str(raw.get("html_url") or raw.get("url") or ""),
        "body": body,
    }


def _worktree_records(root: pathlib.Path, mapping: dict[str, Any], task_uid: str) -> tuple[list[dict[str, Any]], list[dict[str, Any]], dict[str, Any]]:
    raw = _run(["git", "-C", str(root), "worktree", "list", "--porcelain"])
    worktrees: list[dict[str, Any]] = []
    current: dict[str, str] = {}
    for line in raw.splitlines() + [""]:
        if line:
            key, _, value = line.partition(" ")
            current[key] = value
        elif current:
            path = str(pathlib.Path(current.get("worktree", "")).resolve())
            branch_ref = current.get("branch", "")
            branch = branch_ref.removeprefix("refs/heads/") if branch_ref else None
            worktrees.append({"path": path, "registered": True, "branch": branch, "head_oid": current.get("HEAD", "")})
            current = {}
    _require(any(item["path"] == str(root.resolve()) for item in worktrees), "mapping root is not a registered Git worktree")
    origin = _run(["git", "-C", str(root), "config", "--get", "remote.origin.url"])
    origin_repo = _repository_identity(origin)
    _require(bool(origin_repo), "mapping root remote.origin is not a supported GitHub repository")
    common_raw = _run(["git", "-C", str(root), "rev-parse", "--git-common-dir"])
    common = pathlib.Path(common_raw)
    if not common.is_absolute():
        common = (root / common).resolve()
    refs = _run(["git", "-C", str(root), "branch", "--list", "--format=%(refname:short)"])
    branches = [
        line.strip()
        for line in refs.splitlines()
        if line.strip() and line.strip() != "(no branch)"
    ]
    remotes = [line.strip() for line in _run(["git", "-C", str(root), "remote"]).splitlines() if line.strip()]
    if len(remotes) != len(set(remotes)) or any(not re.fullmatch(r"[A-Za-z0-9._-]+", remote) or remote.startswith("-") for remote in remotes):
        raise RetirementError("configured Git remote inventory is malformed or ambiguous")
    remote_branches: list[dict[str, str]] = []
    for remote in remotes:
        raw_remote_refs = _run(["git", "-C", str(root), "ls-remote", "--heads", remote])
        for line in raw_remote_refs.splitlines():
            oid, separator, ref = line.partition("\t")
            if not separator or re.fullmatch(r"[0-9a-fA-F]{40}|[0-9a-fA-F]{64}", oid) is None or not ref.startswith("refs/heads/"):
                raise RetirementError(f"configured remote {remote} returned malformed branch evidence")
            branch = ref.removeprefix("refs/heads/")
            if not branch or _run(["git", "-C", str(root), "check-ref-format", ref]) != "":
                raise RetirementError(f"configured remote {remote} returned an invalid branch name")
            remote_branches.append({"remote": remote, "name": branch, "oid": oid.lower()})
    branch_tips = {
        branch: _run(["git", "-C", str(root), "rev-parse", "--verify", f"refs/heads/{branch}^{{commit}}"])
        for branch in branches
    }
    tasks = mapping.get("tasks") or {}
    branch_to_uid: dict[str, str] = {}
    for uid, record in tasks.items():
        if uid == task_uid or not isinstance(record, dict):
            continue
        branch = str(record.get("task_branch") or "")
        if branch and branch in branches:
            if branch in branch_to_uid and branch_to_uid[branch] != uid:
                raise RetirementError(f"ambiguous task ownership for branch {branch}")
            branch_to_uid[branch] = str(uid)
    owners_by_path = {
        str(pathlib.Path(record.get("canonical_worktree") or "").resolve()): str(uid)
        for uid, record in tasks.items()
        if uid != task_uid and isinstance(record, dict) and record.get("canonical_worktree")
    }
    for worktree in worktrees:
        if not worktree.get("branch"):
            # A registered detached worktree is attributable only when one
            # different active task's exact path/branch mapping names it and
            # its HEAD equals that branch ref. This supports an explicit
            # detached snapshot without inferring ownership from a shared OID.
            mapped = [
                (str(uid), record)
                for uid, record in tasks.items()
                if uid != task_uid and isinstance(record, dict)
                and str(pathlib.Path(str(record.get("canonical_worktree") or "")).resolve()) == str(worktree["path"])
                and str(record.get("task_branch") or "") in branch_tips
            ]
            if len(mapped) == 1:
                uid, record = mapped[0]
                if branch_tips[str(record["task_branch"])] == worktree.get("head_oid"):
                    worktree["branch"] = str(record["task_branch"])
                    worktree["task_uid"] = uid
        branch = worktree.get("branch")
        mapped_owner = owners_by_path.get(str(worktree["path"]))
        branch_owner = branch_to_uid.get(str(branch)) if branch else None
        if mapped_owner and branch_owner and mapped_owner != branch_owner:
            raise RetirementError(f"worktree path and branch disagree on task owner: {worktree['path']}")
        worktree["task_uid"] = mapped_owner or branch_owner
        if branch and branch in branch_tips and branch_tips[branch] != worktree.get("head_oid"):
            raise RetirementError(f"registered worktree HEAD differs from mapped branch {branch}")
    branch_records = [
        {
            "name": branch,
            "worktree": next((item["path"] for item in worktrees if item.get("branch") == branch), ""),
            "task_uid": branch_to_uid.get(branch, ""),
        }
        for branch in branches
    ]
    return worktrees, branch_records, {
        "common_dir": str(common),
        "origin_repository": origin_repo,
        "branches": branches,
        "remotes": remotes,
        "remote_branches": remote_branches,
    }


def _artifact_discovery(
    root: pathlib.Path,
    mapping: dict[str, Any],
    task_uid: str,
    project_items: list[dict[str, Any]],
    pull_requests: list[dict[str, Any]],
) -> dict[str, Any]:
    project = mapping.get("project") or {}
    repository = str(project.get("repo") or "")
    worktrees, branches, git_info = _worktree_records(root, mapping, task_uid)
    candidate_row = (mapping.get("tasks") or {}).get(task_uid) or {}
    candidate_artifacts: dict[str, list[dict[str, Any]]] = {
        "worktrees": [],
        "branches": [],
        "bootstrap_snapshots": [],
        "execution_evidence": [],
        "terminal_receipts": [],
        "reciprocal_prs": [],
    }
    for worktree in worktrees:
        path = pathlib.Path(worktree["path"])
        if (task_uid in path.parts or task_uid in str(path)) and path != root.resolve():
            candidate_artifacts["worktrees"].append(dict(worktree))
        if worktree.get("task_uid") == task_uid:
            candidate_artifacts["worktrees"].append(dict(worktree))
        scratch = path / ".pm" / "scratch" / task_uid
        if scratch.exists():
            for artifact in sorted(scratch.rglob("*")):
                if not artifact.is_file():
                    continue
                entry = {"path": str(artifact.resolve()), "sha256": digest_bytes(artifact.read_bytes())}
                if artifact.name == "bootstrap-task-snapshot.json":
                    candidate_artifacts["bootstrap_snapshots"].append(entry)
                else:
                    candidate_artifacts["execution_evidence"].append(entry)
    cached_path_text = str(candidate_row.get("canonical_worktree") or "").strip()
    if cached_path_text:
        cached_path = pathlib.Path(cached_path_text).expanduser().resolve()
        registered_paths = {pathlib.Path(item["path"]).resolve() for item in worktrees}
        if cached_path.exists() and cached_path not in registered_paths:
            foreign_alias = any(
                uid != task_uid and isinstance(record, dict)
                and str(pathlib.Path(str(record.get("canonical_worktree") or "")).expanduser().resolve()) == str(cached_path)
                and str(record.get("task_branch") or "") == str(candidate_row.get("task_branch") or "")
                for uid, record in (mapping.get("tasks") or {}).items()
            )
            if not foreign_alias:
                candidate_artifacts["worktrees"].append({"path": str(cached_path), "registered": False, "branch": None})
            scratch = cached_path / ".pm" / "scratch" / task_uid
            if scratch.exists():
                for artifact in sorted(scratch.rglob("*")):
                    if not artifact.is_file():
                        continue
                    entry = {"path": str(artifact.resolve()), "sha256": digest_bytes(artifact.read_bytes())}
                    if artifact.name == "bootstrap-task-snapshot.json":
                        candidate_artifacts["bootstrap_snapshots"].append(entry)
                    else:
                        candidate_artifacts["execution_evidence"].append(entry)
    for branch in git_info["branches"]:
        if task_uid in branch:
            candidate_artifacts["branches"].append({"name": branch})
        elif branch == str(candidate_row.get("task_branch") or ""):
            other = [
                record for uid, record in (mapping.get("tasks") or {}).items()
                if uid != task_uid and isinstance(record, dict)
                and record.get("task_branch") == branch
                and record.get("canonical_worktree") == candidate_row.get("canonical_worktree")
            ]
            if not other:
                candidate_artifacts["branches"].append({"name": branch})
    for branch in git_info["remote_branches"]:
        exact_foreign_owner = any(
            uid != task_uid and isinstance(record, dict)
            and str(record.get("task_branch") or "") == branch["name"]
            and str(pathlib.Path(str(record.get("canonical_worktree") or "")).expanduser().resolve())
            == str(pathlib.Path(str(candidate_row.get("canonical_worktree") or "")).expanduser().resolve())
            for uid, record in (mapping.get("tasks") or {}).items()
        )
        if task_uid in branch["name"] or (branch["name"] == str(candidate_row.get("task_branch") or "") and not exact_foreign_owner):
            candidate_artifacts["branches"].append(dict(branch))
    receipts_root = pathlib.Path(git_info["common_dir"]) / "oasis7-workflow-receipts" / task_uid
    if receipts_root.exists():
        for artifact in sorted(receipts_root.rglob("*")):
            if artifact.is_file():
                candidate_artifacts["terminal_receipts"].append(
                    {"path": str(artifact.resolve()), "sha256": digest_bytes(artifact.read_bytes())}
                )
    foreign_owners = _collect_foreign_owners(
        root, mapping, task_uid, repository, worktrees, branches, git_info, project_items, pull_requests
    )
    return {
        "complete": True,
        "worktrees_complete": True,
        "branches_complete": True,
        "remote_branches_complete": True,
        "snapshots_complete": True,
        "execution_evidence_complete": True,
        "terminal_receipts_complete": True,
        "pull_requests_complete": True,
        "worktrees": worktrees,
        "branches": branches,
        "remotes": git_info["remotes"],
        "remote_branches": git_info["remote_branches"],
        "candidate_artifacts": candidate_artifacts,
        "foreign_owners": foreign_owners,
        "pull_requests": pull_requests,
    }


def _collect_pull_requests(repository: str) -> list[dict[str, Any]]:
    pages = _gh_pages(f"repos/{repository}/pulls?state=all&per_page=100")
    return [_pull_request(item, repository) for item in _flatten_rest_pages(pages, "Pull request")]


def _collect_foreign_owners(
    root: pathlib.Path,
    mapping: dict[str, Any],
    task_uid: str,
    repository: str,
    worktrees: list[dict[str, Any]],
    branches: list[dict[str, Any]],
    git_info: dict[str, Any],
    project_items: list[dict[str, Any]],
    pull_requests: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    tasks = mapping.get("tasks") or {}
    candidate = tasks.get(task_uid) or {}
    cached_path = str(pathlib.Path(str(candidate.get("canonical_worktree") or "")).resolve()) if candidate.get("canonical_worktree") else ""
    cached_branch = str(candidate.get("task_branch") or "")
    conflicts = [
        (str(uid), record)
        for uid, record in tasks.items()
        if uid != task_uid and isinstance(record, dict)
        and (str(pathlib.Path(str(record.get("canonical_worktree") or "")).resolve()) == cached_path
             or str(record.get("task_branch") or "") == cached_branch)
        and (cached_path or cached_branch)
    ]
    if not conflicts:
        return []
    exact = [
        (uid, record) for uid, record in conflicts
        if str(pathlib.Path(str(record.get("canonical_worktree") or "")).resolve()) == cached_path
        and str(record.get("task_branch") or "") == cached_branch
    ]
    if len(conflicts) != 1 or len(exact) != 1:
        raise RetirementError("cached worktree/branch has ambiguous foreign task ownership")
    foreign_uid, record = exact[0]
    if _terminal_record(record):
        raise RetirementError("cached foreign task is terminal; duplicate mapping needs separate reconciliation")
    issue_number = int(record.get("issue_number") or 0)
    issue = _read_issue(repository, issue_number)
    foreign_items = [
        value for value in project_items
        if value.get("issue_number") == issue_number and value.get("task_uid") == foreign_uid
        and value.get("repository") == repository
    ]
    if len(foreign_items) != 1:
        raise RetirementError("foreign task has no unique live Project item")
    item = foreign_items[0]
    worktree = next((value for value in worktrees if value.get("path") == cached_path), None)
    branch_item = next((value for value in branches if value.get("name") == cached_branch), None)
    if worktree is None or not worktree.get("registered") or worktree.get("branch") != cached_branch:
        raise RetirementError("foreign task canonical worktree is not registered on its mapped branch")
    if branch_item is None or branch_item.get("task_uid") != foreign_uid or branch_item.get("worktree") != cached_path:
        raise RetirementError("foreign task local branch identity does not match its mapping")
    snapshot_path = pathlib.Path(cached_path) / ".pm" / "scratch" / foreign_uid / "bootstrap-task-snapshot.json"
    try:
        snapshot_bytes = snapshot_path.read_bytes()
        snapshot = json.loads(snapshot_bytes)
    except (OSError, json.JSONDecodeError) as exc:
        raise RetirementError(f"foreign task immutable bootstrap snapshot is unavailable: {exc}") from exc
    snapshot_digest = snapshot.get("digest") if isinstance(snapshot, dict) else None
    unsigned_snapshot = dict(snapshot) if isinstance(snapshot, dict) else {}
    unsigned_snapshot.pop("digest", None)
    if snapshot_digest != canonical_digest(unsigned_snapshot):
        raise RetirementError("foreign task bootstrap snapshot digest is invalid")
    _validate_foreign_snapshot_payload(
        snapshot,
        record,
        issue,
        repository,
        mapping.get("project") or {},
        cached_path,
        cached_branch,
    )
    prs = [pr for pr in pull_requests if _pr_binds_task(pr, record, foreign_uid)]
    mapped_pr = int(record.get("pr_number") or 0)
    if mapped_pr:
        matches = [pr for pr in pull_requests if pr.get("number") == mapped_pr]
        if len(matches) != 1 or len(prs) != 1 or prs[0] != matches[0]:
            raise RetirementError("foreign task mapped PR is not uniquely reciprocal in the complete live PR list")
        pr = matches[0]
        if (
            pr.get("repository") != repository
            or pr.get("base_repository") != repository
            or pr.get("head_repository") != repository
            or pr.get("head_branch") != cached_branch
            or pr.get("issue_number") != issue_number
            or pr.get("task_uid") != foreign_uid
        ):
            raise RetirementError("foreign task live PR identity differs from its Issue/worktree")
    elif prs:
        raise RetirementError("foreign task has a live reciprocal PR omitted by its mapping")
    else:
        matches = []
    if mapped_pr and str(record.get("pr_url") or "").rstrip("/").endswith(f"/pull/{mapped_pr}") is False:
        raise RetirementError("foreign task mapped PR URL does not match its PR number")
    return [{
        "task_uid": foreign_uid,
        "issue": issue,
        "project_item": item,
        "mapping_record": copy.deepcopy(record),
        "worktree": worktree,
        "branch": branch_item,
        "snapshot": {"path": str(snapshot_path), "sha256": digest_bytes(snapshot_bytes), "payload_digest": snapshot_digest},
        "pull_requests": matches,
    }]


def _terminal_record(record: dict[str, Any]) -> bool:
    return str(record.get("status") or "") in {"done", "deferred"} or str(record.get("workflow_phase") or "") in {
        "task_done", "main_sync", "closed_without_merge", "post_merge_done"
    }


def _validate_foreign_snapshot_payload(
    payload: Any,
    record: dict[str, Any],
    issue: dict[str, Any],
    repository: str,
    project: dict[str, Any],
    expected_worktree: str,
    expected_branch: str,
) -> None:
    _require(
        isinstance(payload, dict) and payload.get("schema") == "oasis7.bootstrap-task-snapshot/v1",
        "foreign immutable snapshot schema is unsupported",
    )
    task = payload.get("task")
    _require(isinstance(task, dict), "foreign immutable snapshot task identity is malformed")
    snapshot_issue = task.get("issue")
    snapshot_project = task.get("project")
    snapshot_git = payload.get("git")
    snapshot_base = snapshot_git.get("base") if isinstance(snapshot_git, dict) else None
    snapshot_request = payload.get("request")
    _require(isinstance(snapshot_issue, dict), "foreign immutable snapshot Issue identity is malformed")
    _require(isinstance(snapshot_project, dict), "foreign immutable snapshot Project identity is malformed")
    _require(isinstance(snapshot_git, dict), "foreign immutable snapshot Git identity is malformed")
    _require(isinstance(snapshot_base, dict), "foreign immutable snapshot base identity is malformed")
    _require(isinstance(snapshot_request, dict), "foreign immutable snapshot request identity is malformed")

    acceptance = task.get("acceptance")
    _require(
        isinstance(acceptance, list) and bool(acceptance)
        and snapshot_request.get("acceptance") == acceptance
        and isinstance(snapshot_request.get("identity"), str) and bool(snapshot_request["identity"]),
        "foreign immutable snapshot request/acceptance binding is incomplete",
    )
    if isinstance(record.get("acceptance"), list):
        _require(acceptance == record["acceptance"], "foreign immutable snapshot acceptance differs from its task mapping")
    expected_epoch = record.get("bootstrap_epoch", 1)
    _require(type(expected_epoch) is int and expected_epoch > 0, "foreign task mapping bootstrap epoch is malformed")
    _require(task.get("bootstrap_epoch") == expected_epoch, "foreign immutable snapshot epoch differs from its task mapping")
    _require(
        task.get("uid") == record.get("task_uid")
        and task.get("owner_role") == record.get("owner_role")
        and snapshot_issue.get("number") == record.get("issue_number")
        and snapshot_issue.get("url") == issue.get("url")
        and snapshot_project.get("owner") == project.get("owner")
        and int(snapshot_project.get("number") or 0) == int(project.get("number") or 0)
        and snapshot_project.get("item_id") == record.get("project_item_id")
        and snapshot_project.get("status") == record.get("status")
        and payload.get("repository") == repository,
        "foreign immutable snapshot Project/task identity differs from its task mapping",
    )
    _require(
        str(pathlib.Path(str(snapshot_git.get("worktree") or "")).resolve()) == expected_worktree
        and snapshot_git.get("branch") == expected_branch,
        "foreign immutable snapshot worktree/branch identity differs from its task mapping",
    )
    default_branch = record.get("default_branch")
    base_oid = str(snapshot_base.get("oid") or "")
    head_oid = str(snapshot_git.get("head") or "")
    _require(
        isinstance(snapshot_base.get("branch"), str) and bool(snapshot_base["branch"])
        and (not default_branch or snapshot_base.get("branch") == default_branch)
        and isinstance(snapshot_base.get("ref"), str) and bool(snapshot_base["ref"])
        and re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", base_oid) is not None
        and re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", head_oid) is not None,
        "foreign immutable snapshot Git base/head identity is malformed",
    )
    _require(
        isinstance(payload.get("producer"), str) and bool(payload["producer"])
        and isinstance(payload.get("created_at"), str) and bool(payload["created_at"]),
        "foreign immutable snapshot producer provenance is incomplete",
    )


def _read_issue(repository: str, issue_number: int) -> dict[str, Any]:
    if issue_number <= 0:
        raise RetirementError("mapped task Issue number is invalid")
    raw = _gh_json(f"repos/{repository}/issues/{issue_number}")
    return _issue_from_rest(raw, repository, issue_number)


def _pr_binds_task(pr: dict[str, Any], record: dict[str, Any], task_uid: str) -> bool:
    issue_number = int(record.get("issue_number") or 0)
    return bool(
        pr.get("task_uid") == task_uid
        or (pr.get("issue_number") == issue_number and pr.get("head_branch") == record.get("task_branch"))
        or (task_uid in str(pr.get("body") or "") and pr.get("head_branch") == record.get("task_branch"))
    )


class LiveProofProvider:
    """Collect raw, complete GitHub/Git readbacks for the shared validator."""

    def read_live_proof(self, mapping_root: pathlib.Path, task_uid: str) -> dict[str, Any]:
        mapping_path = mapping_root / ".pm" / "github-project-sync" / "tasks.json"
        mapping = STORE.read_mapping(mapping_path, {"version": 1, "tasks": {}})
        row = (mapping.get("tasks") or {}).get(task_uid)
        _require(isinstance(row, dict), f"active task mapping row is missing for {task_uid}")
        project = mapping.get("project") or {}
        repository = str(project.get("repo") or row.get("repository") or "")
        _require(bool(GITHUB_REPO_RE.fullmatch(repository)), "canonical mapping repository is malformed")
        owner, name = repository.split("/", 1)
        project_owner = str(project.get("owner") or owner)
        project_number = int(project.get("number") or 0)
        project_id = str(project.get("id") or "")
        _require(project_number > 0 and project_id, "canonical Project identity is incomplete")
        issue_number = int(row.get("issue_number") or 0)
        candidate_issue = _read_issue(repository, issue_number)
        candidate_comments_raw = _comments_for_issue(repository, issue_number)
        marked = [comment for comment in candidate_comments_raw if DISPOSITION_MARKER in str(comment.get("body") or "")]
        if len(marked) != 1:
            raise RetirementError("candidate Issue must have exactly one live duplicate-disposition comment")
        selected_comment_id = int(marked[0].get("id") or 0)
        if selected_comment_id <= 0:
            raise RetirementError("duplicate-disposition comment has no server ID")
        comment_by_id_raw = _gh_json(f"repos/{repository}/issues/comments/{selected_comment_id}")
        comment_by_id = _normalize_comment(comment_by_id_raw, issue_number)
        comment_by_id["body"] = str(comment_by_id_raw.get("body") or "")
        if comment_by_id["id"] != selected_comment_id:
            raise RetirementError("comment-by-ID readback returned a different server comment")
        disposition = _parse_disposition(comment_by_id)
        replacement = disposition.get("replacement") or {}
        replacement_issue_number = int(replacement.get("issue_number") or 0)
        replacement_issue = _read_issue(repository, replacement_issue_number)
        project_live, project_items = _collect_project_items(project_owner, project_number, repository)
        issues = _collect_repository_issues(owner, name, repository)
        prs = _collect_pull_requests(repository)
        discovery = _artifact_discovery(mapping_root, mapping, task_uid, project_items, prs)
        _require_issue_in_complete_collection(issues, candidate_issue, "candidate")
        _require_issue_in_complete_collection(issues, replacement_issue, "replacement")
        for owner_proof in discovery["foreign_owners"]:
            _require_issue_in_complete_collection(issues, owner_proof["issue"], "foreign-owner")
        permission_raw = _gh_json(f"repos/{repository}/collaborators/{comment_by_id['user']['login']}/permission")
        permission = {
            "login": str((permission_raw.get("user") or {}).get("login") or comment_by_id["user"]["login"]),
            "permission": str(permission_raw.get("permission") or "").lower(),
        }
        comments = [_normalize_comment(comment, issue_number) for comment in candidate_comments_raw]
        normalized_issues = [_normalize_issue_for_proof(issue) for issue in issues]
        return {
            "repository": repository,
            "project": project_live,
            "issues": {"complete": True, "items": normalized_issues},
            "project_items": {"complete": True, "items": project_items},
            "candidate_comments": {"complete": True, "items": comments},
            "comment_by_id": comment_by_id,
            "repository_permissions": {"complete": True, "items": [permission]},
            "artifact_discovery": discovery,
        }


def _parse_disposition(comment: dict[str, Any]) -> dict[str, Any]:
    body = str(comment.get("body") or "")
    marker, separator, encoded = body.partition("\n")
    _require(separator == "\n" and marker == DISPOSITION_MARKER, "disposition comment marker/LF envelope is invalid")
    try:
        disposition = json.loads(encoded)
    except json.JSONDecodeError as exc:
        raise RetirementError(f"disposition comment JSON is invalid: {exc}") from exc
    _require(isinstance(disposition, dict), "disposition comment JSON must be an object")
    if encoded != json.dumps(disposition, ensure_ascii=False, sort_keys=True, separators=(",", ":")):
        raise RetirementError("disposition comment JSON is not canonical compact JSON")
    return disposition


def _checked_mapping_path(mapping_root: pathlib.Path) -> tuple[pathlib.Path, pathlib.Path]:
    requested_root = pathlib.Path(mapping_root).expanduser()
    if requested_root.is_symlink():
        raise RetirementError("mapping root must not be a symlink")
    try:
        root = requested_root.resolve(strict=True)
    except OSError as exc:
        raise RetirementError(f"mapping root cannot be resolved: {exc}") from exc
    if not root.is_dir():
        raise RetirementError("mapping root is not a directory")
    mapping_path = root / ".pm" / "github-project-sync" / "tasks.json"
    current = root
    for name, expect_directory in ((".pm", True), ("github-project-sync", True), ("tasks.json", False)):
        current = current / name
        try:
            mode = current.lstat().st_mode
        except OSError as exc:
            raise RetirementError(f"canonical task mapping path is unavailable at {current}: {exc}") from exc
        if stat.S_ISLNK(mode):
            raise RetirementError(f"canonical task mapping path contains symlink component: {current}")
        if expect_directory and not stat.S_ISDIR(mode):
            raise RetirementError(f"canonical task mapping parent is not a directory: {current}")
        if not expect_directory and not stat.S_ISREG(mode):
            raise RetirementError(f"canonical task mapping file is not a regular file: {current}")
    try:
        resolved_mapping = mapping_path.resolve(strict=True)
        resolved_mapping.relative_to(root)
    except (OSError, ValueError) as exc:
        raise RetirementError("canonical task mapping path escapes its mapping root") from exc
    if resolved_mapping != mapping_path:
        raise RetirementError("canonical task mapping path resolves through a symlink")
    lock_path = mapping_path.with_name(mapping_path.name + ".lock")
    if lock_path.is_symlink():
        raise RetirementError(f"canonical task mapping lock path is a symlink: {lock_path}")
    return root, mapping_path


def _registered_root(mapping_root: pathlib.Path, mapping: dict[str, Any]) -> pathlib.Path:
    root = mapping_root.expanduser().resolve()
    if not root.is_dir():
        raise RetirementError("mapping root is not a directory")
    top = pathlib.Path(_run(["git", "-C", str(root), "rev-parse", "--show-toplevel"])).resolve()
    if top != root:
        raise RetirementError("mapping root does not resolve to its own Git worktree")
    project = mapping.get("project") or {}
    repository = str(project.get("repo") or "")
    origin = _repository_identity(_run(["git", "-C", str(root), "config", "--get", "remote.origin.url"]))
    if not repository or origin != repository:
        raise RetirementError("mapping root GitHub origin does not match its task mapping")
    return root


def _unique_issue(items: list[dict[str, Any]], issue_number: int, task_uid: str, label: str) -> dict[str, Any]:
    matches = [item for item in items if int(item.get("number") or 0) == issue_number]
    _require(len(matches) == 1, f"{label} Issue #{issue_number} is missing or ambiguous")
    issue = matches[0]
    observed_uid = issue.get("task_uid") or _task_uid_from_body(str(issue.get("body") or ""))
    _require(observed_uid == task_uid, f"{label} Issue Task UID does not match")
    return issue


def _unique_project_item(
    items: list[dict[str, Any]], project: dict[str, Any], repository: str,
    issue_number: int, task_uid: str, label: str,
) -> dict[str, Any]:
    matches = [
        item for item in items
        if item.get("issue_number") == issue_number and item.get("task_uid") == task_uid
        and item.get("repository") == repository
    ]
    _require(len(matches) == 1, f"{label} Project item is missing or ambiguous")
    item = matches[0]
    _require(not item.get("archived"), f"{label} Project item is archived")
    _require(
        item.get("project_id") == project.get("id")
        and item.get("project_owner") == project.get("owner")
        and item.get("project_number") == project.get("number"),
        f"{label} Project item belongs to a different Project",
    )
    _require(item.get("_field_values_complete", True) is True, f"{label} Project fields are incomplete")
    return item


def _validate_issue_identity(issue: dict[str, Any], repository: str, issue_number: int, task_uid: str, label: str) -> dict[str, str]:
    _require(issue.get("repository") == repository and issue.get("number") == issue_number, f"{label} Issue repository/number mismatch")
    _require(str(issue.get("url") or "").endswith(f"/issues/{issue_number}"), f"{label} Issue URL mismatch")
    metadata = _issue_metadata(issue)
    _require(metadata["task_uid"] == task_uid, f"{label} Issue must bind exactly one canonical task_uid")
    return metadata


def _validate_live_proof(mapping: dict[str, Any], task_uid: str, row: dict[str, Any], comment_id: int, proof: Any) -> dict[str, Any]:
    _require(isinstance(proof, dict), "live proof provider returned no proof object")
    project_map = mapping.get("project") or {}
    repository = str(project_map.get("repo") or row.get("repository") or "")
    owner = str(project_map.get("owner") or repository.partition("/")[0])
    project_number = int(project_map.get("number") or 0)
    project_id = str(project_map.get("id") or "")
    _require(proof.get("repository") == repository, "live proof repository does not match canonical mapping")
    project = proof.get("project")
    _require(
        isinstance(project, dict)
        and project.get("repo", repository) == repository
        and project.get("owner") == owner
        and int(project.get("number") or 0) == project_number
        and project.get("id") == project_id,
        "live Project identity differs from canonical mapping",
    )
    if not GITHUB_REPO_RE.fullmatch(repository):
        raise RetirementError("canonical mapping repository is malformed")
    _require(row.get("task_uid") == task_uid, "active mapping row UID differs from its key")
    _require(row.get("repository") == repository, "candidate mapping repository differs from canonical Project")
    _require(str(row.get("status") or "") == "candidate", "candidate mapping status is no longer candidate")
    _require(str(row.get("workflow_phase") or "") == "bootstrap", "candidate mapping has workflow-started evidence")
    issue_number = int(row.get("issue_number") or 0)
    _require(issue_number > 0, "candidate mapping has no valid Issue number")

    issues_payload = proof.get("issues")
    _require(isinstance(issues_payload, dict) and issues_payload.get("complete") is True, "Issue pagination is incomplete")
    issues = issues_payload.get("items")
    _require(isinstance(issues, list) and all(isinstance(item, dict) for item in issues), "Issue collection is malformed")
    candidate_issue = _unique_issue(issues, issue_number, task_uid, "candidate")
    _require_unique_issue_uid(issues, task_uid, issue_number, "candidate")
    candidate_metadata = _validate_issue_identity(candidate_issue, repository, issue_number, task_uid, "candidate")
    _require(candidate_issue.get("state") == "CLOSED" and candidate_issue.get("state_reason") == "DUPLICATE", "candidate Issue is not closed as DUPLICATE")
    _require(candidate_metadata["status"] == "candidate" and candidate_metadata["workflow_phase"] == "bootstrap", "candidate Issue has started workflow")
    _require(not candidate_metadata["worktree_hint"] and not candidate_metadata["pr_number"] and not candidate_metadata["pr_url"], "candidate Issue records worktree or PR activity")
    items_payload = proof.get("project_items")
    _require(isinstance(items_payload, dict) and items_payload.get("complete") is True, "Project item pagination is incomplete")
    project_items = items_payload.get("items")
    _require(isinstance(project_items, list) and all(isinstance(item, dict) for item in project_items), "Project item collection is malformed")
    candidate_item = _unique_project_item(project_items, project, repository, issue_number, task_uid, "candidate")
    _require_unique_project_uid(project_items, repository, task_uid, issue_number, "candidate")
    _require(candidate_item.get("id") == row.get("project_item_id"), "candidate Project item differs from cached mapping")
    fields = candidate_item.get("fields") or {}
    _require(fields.get("Status") == "Done", "candidate Project Status is not Done")
    _require(fields.get("PM Status") == "candidate", "candidate Project PM Status is not candidate")
    _require(fields.get("Workflow Phase") == "bootstrap", "candidate Project workflow phase is not bootstrap")
    _require(not str(fields.get("Canonical Worktree") or "").strip(), "candidate Project item has a canonical worktree")

    comments_payload = proof.get("candidate_comments")
    _require(isinstance(comments_payload, dict) and comments_payload.get("complete") is True, "candidate comment pagination is incomplete")
    comments = comments_payload.get("items")
    _require(isinstance(comments, list) and all(isinstance(item, dict) for item in comments), "candidate comments are malformed")
    marked = [item for item in comments if str(item.get("body") or "").startswith(DISPOSITION_MARKER)]
    _require(len(marked) == 1, "candidate Issue must have exactly one canonical disposition comment")
    listed_comment = marked[0]
    _require(int(listed_comment.get("id") or 0) == comment_id, "selected disposition comment ID is not the unique server marker")
    comment = proof.get("comment_by_id")
    _require(isinstance(comment, dict), "server comment-ID readback is unavailable")
    _require(int(comment.get("id") or 0) == comment_id and int(comment.get("issue_number") or 0) == issue_number, "server comment-ID readback identity mismatch")
    _require(comment.get("body") == listed_comment.get("body"), "server comment-ID body differs from paginated comment read")
    author = str((comment.get("user") or {}).get("login") or "")
    _require(bool(author), "server disposition comment has no author")
    disposition = _parse_disposition(comment)
    required_keys = {"candidate", "reason", "no_source_work", "no_workflow_start", "no_worktree", "no_pr", "replacement"}
    _require(set(disposition) == required_keys, "disposition comment has missing or unexpected fields")
    expected_candidate = {
        "repository": repository,
        "issue_number": issue_number,
        "task_uid": task_uid,
        "project_item_id": str(candidate_item.get("id") or ""),
    }
    _require(disposition.get("candidate") == expected_candidate, "disposition candidate binding differs from live Issue/Project")
    _require(disposition.get("reason") == "duplicate", "disposition reason is not duplicate")
    for flag in ("no_source_work", "no_workflow_start", "no_worktree", "no_pr"):
        _require(disposition.get(flag) is True, f"disposition attestation {flag} is not true")
    replacement_binding = disposition.get("replacement")
    _require(isinstance(replacement_binding, dict) and set(replacement_binding) == {"repository", "issue_number", "task_uid", "project_item_id"}, "replacement binding is malformed")
    _require(replacement_binding.get("repository") == repository, "replacement belongs to a different repository")
    replacement_uid = str(replacement_binding.get("task_uid") or "")
    _require(TASK_UID_RE.fullmatch(replacement_uid) is not None and replacement_uid != task_uid, "replacement Task UID is invalid or aliases candidate")
    replacement_issue_number = int(replacement_binding.get("issue_number") or 0)
    _require(replacement_issue_number > 0 and replacement_issue_number != issue_number, "replacement Issue number is invalid or aliases candidate")
    replacement_issue = _unique_issue(issues, replacement_issue_number, replacement_uid, "replacement")
    _require_unique_issue_uid(issues, replacement_uid, replacement_issue_number, "replacement")
    replacement_metadata = _validate_issue_identity(replacement_issue, repository, replacement_issue_number, replacement_uid, "replacement")
    _require(replacement_issue.get("state_reason") != "DUPLICATE", "replacement Issue is itself closed as DUPLICATE")
    replacement_item = _unique_project_item(project_items, project, repository, replacement_issue_number, replacement_uid, "replacement")
    _require_unique_project_uid(project_items, repository, replacement_uid, replacement_issue_number, "replacement")
    _require(replacement_item.get("id") == replacement_binding.get("project_item_id"), "replacement Project item differs from disposition binding")
    if replacement_issue.get("state") == "CLOSED" or replacement_metadata["workflow_phase"] in {"task_done", "main_sync", "post_merge_done"}:
        replacement_row = (mapping.get("tasks") or {}).get(replacement_uid)
        _require(isinstance(replacement_row, dict) and _terminal_record(replacement_row), "terminal replacement lacks its canonical terminal mapping receipts")
        receipts = replacement_row.get("phase_receipts") or {}
        _require(bool(replacement_row.get("merge_receipt") or receipts.get("merge") or receipts.get("task_done")), "terminal replacement lacks merge/task-done receipt")

    permissions_payload = proof.get("repository_permissions")
    _require(isinstance(permissions_payload, dict) and permissions_payload.get("complete") is True, "repository permission lookup is incomplete")
    permission_items = permissions_payload.get("items")
    _require(isinstance(permission_items, list), "repository permission lookup is malformed")
    permission_matches = [item for item in permission_items if isinstance(item, dict) and item.get("login") == author]
    _require(len(permission_matches) == 1 and permission_matches[0].get("permission") == "admin", "disposition author does not currently have unique repository admin permission")
    _require(permission_matches[0].get("login") == author, "admin permission identity differs from live comment author")
    comment_sha = digest_bytes(str(comment.get("body") or "").encode("utf-8"))

    discovery = proof.get("artifact_discovery")
    _require(isinstance(discovery, dict) and discovery.get("complete") is True, "Git/PR artifact discovery is incomplete")
    for flag in (
        "worktrees_complete", "branches_complete", "remote_branches_complete", "snapshots_complete", "execution_evidence_complete",
        "terminal_receipts_complete", "pull_requests_complete",
    ):
        _require(discovery.get(flag) is True, f"Git/PR artifact discovery is incomplete: {flag}")
    for collection in ("worktrees", "branches", "remotes", "remote_branches", "pull_requests"):
        _require(isinstance(discovery.get(collection), list), f"Git/PR artifact discovery lacks {collection}")
    remotes = discovery["remotes"]
    _require(
        all(isinstance(remote, str) and re.fullmatch(r"[A-Za-z0-9._-]+", remote) and not remote.startswith("-") for remote in remotes)
        and len(remotes) == len(set(remotes)),
        "configured remote inventory is malformed or ambiguous",
    )
    remote_branch_keys: set[tuple[str, str]] = set()
    for remote_branch in discovery["remote_branches"]:
        _require(
            isinstance(remote_branch, dict)
            and remote_branch.get("remote") in remotes
            and isinstance(remote_branch.get("name"), str)
            and bool(remote_branch.get("name"))
            and re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", str(remote_branch.get("oid") or "")) is not None,
            "remote branch evidence is malformed or not bound to a configured remote",
        )
        key = (remote_branch["remote"], remote_branch["name"])
        _require(key not in remote_branch_keys, "remote branch evidence is duplicated")
        remote_branch_keys.add(key)
    candidate_artifacts = discovery.get("candidate_artifacts")
    _require(isinstance(candidate_artifacts, dict), "candidate artifact inventory is malformed")
    for key in ("worktrees", "branches", "bootstrap_snapshots", "execution_evidence", "terminal_receipts", "reciprocal_prs"):
        _require(isinstance(candidate_artifacts.get(key), list), f"candidate artifact inventory lacks {key}")
        _require(not candidate_artifacts[key], f"candidate-owned {key.replace('_', ' ')} block retirement")
    foreign_owners = discovery.get("foreign_owners")
    _require(isinstance(foreign_owners, list), "foreign owner evidence is malformed")
    cached_path = str(pathlib.Path(str(row.get("canonical_worktree") or "")).resolve()) if row.get("canonical_worktree") else ""
    cached_branch = str(row.get("task_branch") or "")
    aliased_rows = [
        (str(uid), value) for uid, value in (mapping.get("tasks") or {}).items()
        if uid != task_uid and isinstance(value, dict)
        and (str(pathlib.Path(str(value.get("canonical_worktree") or "")).resolve()) == cached_path
             or str(value.get("task_branch") or "") == cached_branch)
        and (cached_path or cached_branch)
    ]
    if aliased_rows:
        _require(len(aliased_rows) == 1 and len(foreign_owners) == 1, "cached identity has ambiguous foreign task owners")
        foreign_uid, foreign = aliased_rows[0]
        _require(
            str(pathlib.Path(str(foreign.get("canonical_worktree") or "")).resolve()) == cached_path
            and str(foreign.get("task_branch") or "") == cached_branch
            and foreign.get("task_uid") == foreign_uid
            and not _terminal_record(foreign),
            "cached worktree/branch does not bind one different nonterminal foreign UID",
        )
        owner = foreign_owners[0]
        _require(isinstance(owner, dict) and owner.get("task_uid") == foreign_uid, "live foreign owner evidence UID mismatch")
        foreign_issue_number = int(foreign.get("issue_number") or 0)
        foreign_issue = owner.get("issue")
        _require(isinstance(foreign_issue, dict), "live foreign Issue evidence is missing")
        _require_unique_issue_uid(issues, foreign_uid, foreign_issue_number, "foreign")
        enumerated_foreign_issue = _unique_issue(issues, foreign_issue_number, foreign_uid, "foreign")
        _require(
            _normalize_issue_for_proof(enumerated_foreign_issue) == _normalize_issue_for_proof(foreign_issue),
            "foreign Issue differs from its complete paginated Issue collection entry",
        )
        _validate_issue_identity(foreign_issue, repository, foreign_issue_number, foreign_uid, "foreign")
        _require(foreign_issue.get("state") != "CLOSED" or foreign_issue.get("state_reason") != "DUPLICATE", "foreign task Issue is itself a closed duplicate")
        foreign_item = owner.get("project_item")
        _require(isinstance(foreign_item, dict), "live foreign Project item evidence is missing")
        enumerated_foreign_item = _require_unique_project_uid(
            project_items, repository, foreign_uid, foreign_issue_number, "foreign"
        )
        _require(
            enumerated_foreign_item.get("id") == foreign_item.get("id"),
            "foreign Project item differs from its complete Project collection entry",
        )
        _require(
            foreign_item.get("repository") == repository
            and foreign_item.get("issue_number") == foreign_issue_number
            and foreign_item.get("task_uid") == foreign_uid
            and foreign_item.get("project_id") == project_id
            and foreign_item.get("project_owner") == project.get("owner")
            and foreign_item.get("project_number") == project_number
            and foreign_item.get("archived") is False,
            "live foreign Project identity differs from its task mapping",
        )
        _require(foreign_item.get("id") == foreign.get("project_item_id"), "foreign Project item differs from its task mapping")
        foreign_worktree = owner.get("worktree")
        foreign_branch = owner.get("branch")
        if foreign_branch is None:
            branch_matches = [
                value for value in discovery.get("branches", [])
                if isinstance(value, dict) and value.get("name") == cached_branch
                and value.get("task_uid") == foreign_uid
            ]
            foreign_branch = branch_matches[0] if len(branch_matches) == 1 else None
        _require(
            isinstance(foreign_worktree, dict) and foreign_worktree.get("registered") is True
            and str(pathlib.Path(str(foreign_worktree.get("path") or "")).resolve()) == cached_path
            and foreign_worktree.get("branch") == cached_branch,
            "foreign registered worktree identity differs from the cached alias",
        )
        _require(
            isinstance(foreign_branch, dict) and foreign_branch.get("name") == cached_branch
            and str(pathlib.Path(str(foreign_branch.get("worktree") or "")).resolve()) == cached_path
            and foreign_branch.get("task_uid") == foreign_uid,
            "foreign branch identity differs from the cached alias",
        )
        snapshot = owner.get("snapshot")
        _require(isinstance(snapshot, dict) and str(snapshot.get("sha256") or "").startswith("sha256:"), "foreign immutable snapshot evidence is missing")
        expected_snapshot_path = pathlib.Path(cached_path) / ".pm" / "scratch" / foreign_uid / "bootstrap-task-snapshot.json"
        _require(
            str(pathlib.Path(str(snapshot.get("path") or "")).resolve()) == str(expected_snapshot_path.resolve()),
            "foreign immutable snapshot path differs from its canonical task worktree",
        )
        try:
            snapshot_bytes = expected_snapshot_path.read_bytes()
            snapshot_payload = json.loads(snapshot_bytes)
        except (OSError, json.JSONDecodeError) as exc:
            raise RetirementError(f"foreign immutable snapshot cannot be read: {exc}") from exc
        _require(digest_bytes(snapshot_bytes) == snapshot.get("sha256"), "foreign immutable snapshot byte digest is stale")
        snapshot_payload_digest = snapshot_payload.get("digest") if isinstance(snapshot_payload, dict) else None
        unsigned_snapshot = dict(snapshot_payload) if isinstance(snapshot_payload, dict) else {}
        unsigned_snapshot.pop("digest", None)
        _require(snapshot_payload_digest == canonical_digest(unsigned_snapshot), "foreign immutable snapshot payload digest is invalid")
        _validate_foreign_snapshot_payload(
            snapshot_payload,
            foreign,
            foreign_issue,
            repository,
            project,
            cached_path,
            cached_branch,
        )
        prs = owner.get("pull_requests")
        _require(isinstance(prs, list), "foreign PR evidence is malformed")
        mapped_pr = int(foreign.get("pr_number") or 0)
        if mapped_pr:
            _require(len(prs) == 1, "foreign task live PR is missing or ambiguous")
            pr = prs[0]
            _require(
                isinstance(pr, dict)
                and pr.get("number") == mapped_pr
                and pr.get("repository") == repository
                and pr.get("base_repository", pr.get("repository")) == repository
                and pr.get("head_repository") == repository
                and pr.get("head_branch") == cached_branch
                and pr.get("issue_number") == foreign_issue_number
                and pr.get("task_uid") == foreign_uid,
                "foreign task PR does not reciprocally bind its Issue and branch",
            )
        else:
            _require(not prs, "foreign task has an unrecorded reciprocal PR")
    else:
        _require(not foreign_owners, "foreign owner evidence exists without a cached identity alias")

    pull_requests = discovery.get("pull_requests")
    for pr in pull_requests:
        _require(isinstance(pr, dict), "live PR collection is malformed")
        if pr.get("task_uid") == task_uid or pr.get("issue_number") == issue_number:
            raise RetirementError("candidate has a live reciprocal pull request")
        if cached_branch and pr.get("head_branch") == cached_branch and not aliased_rows:
            raise RetirementError("candidate cached branch has a live pull request")

    evidence = {
        "candidate_issue": {"repository": repository, "issue_number": issue_number, "task_uid": task_uid},
        "candidate_project_item": copy.deepcopy(candidate_item),
        "disposition_comment": {
            "id": comment_id,
            "body_sha256": comment_sha,
            "author": author,
            "url": str(comment.get("html_url") or ""),
            "created_at": str(comment.get("created_at") or ""),
        },
        "permission": {"login": author, "permission": "admin"},
        "replacement": {
            "repository": repository,
            "issue_number": replacement_issue_number,
            "task_uid": replacement_uid,
            "project_item_id": replacement_item["id"],
            "issue": copy.deepcopy(replacement_issue),
            "project_item": copy.deepcopy(replacement_item),
        },
        "artifact_discovery": copy.deepcopy(discovery),
    }
    return evidence


class _AlreadyRetired(Exception):
    def __init__(self, tombstone: dict[str, Any]):
        self.tombstone = tombstone


def retire_candidate(
    mapping_root: pathlib.Path,
    task_uid: str,
    disposition_comment_id: int,
    *,
    mode: str,
    live_proof_provider: Any,
) -> dict[str, Any]:
    """Mutation-free preflight or exact, repeat-read atomic retirement."""
    if mode not in {"preflight", "apply"}:
        raise RetirementError("only preflight and apply modes are permitted")
    if TASK_UID_RE.fullmatch(task_uid) is None:
        raise RetirementError("task UID is malformed")
    if not isinstance(disposition_comment_id, int) or disposition_comment_id <= 0:
        raise RetirementError("disposition server comment ID must be positive")
    root, mapping_path = _checked_mapping_path(mapping_root)
    try:
        initial = STORE.read_mapping(mapping_path, {"version": 1, "tasks": {}})
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        raise RetirementError(f"cannot validate task mapping: {exc}") from exc
    _registered_root(root, initial)
    current_tombstone = STORE.retired_task(initial, task_uid)
    if current_tombstone is not None:
        return {"status": "already_retired", "task_uid": task_uid, "digest": current_tombstone["digest"]}
    row = (initial.get("tasks") or {}).get(task_uid)
    if not isinstance(row, dict):
        raise RetirementError(f"task UID not found in active mapping or retirement ledger: {task_uid}")
    before_row = copy.deepcopy(row)
    if mode == "preflight":
        proof = live_proof_provider.read_live_proof(root, task_uid)
        evidence = _validate_live_proof(initial, task_uid, before_row, disposition_comment_id, proof)
        try:
            _checked_mapping_path(root)
            latest = STORE.read_mapping(mapping_path, {"version": 1, "tasks": {}})
        except (OSError, ValueError, json.JSONDecodeError) as exc:
            raise RetirementError(f"cannot re-read task mapping after no-write preflight: {exc}") from exc
        STORE.validate_retirement_ledger(latest)
        if STORE.retired_task(latest, task_uid) is not None:
            tombstone = STORE.retired_task(latest, task_uid)
            return {"status": "already_retired", "task_uid": task_uid, "digest": tombstone["digest"]}
        if (latest.get("tasks") or {}).get(task_uid) != before_row:
            raise RetirementError("candidate mapping row changed during no-write preflight")
        if list(latest.get("retired_duplicate_candidates", [])) != list(initial.get("retired_duplicate_candidates", [])):
            raise RetirementError("retirement ledger changed during no-write preflight")
        _registered_root(root, latest)
        return {"status": "preflight_ok", "task_uid": task_uid, "evidence_digest": canonical_digest(evidence)}
    try:
        # Validate every mapping component immediately before the durable
        # store opens or creates the lock sidecar.
        _checked_mapping_path(root)
        with STORE.locked_json(
            mapping_path,
            {"version": 1, "tasks": {}},
            write_back=True,
            allow_retirement_append=True,
        ) as locked_mapping:
            _checked_mapping_path(root)
            STORE.validate_retirement_ledger(locked_mapping)
            tombstone = STORE.retired_task(locked_mapping, task_uid)
            if tombstone is not None:
                raise _AlreadyRetired(tombstone)
            locked_row = (locked_mapping.get("tasks") or {}).get(task_uid)
            if locked_row != before_row:
                raise RetirementError("candidate mapping row changed before lock-held validation")
            live_root = _registered_root(root, locked_mapping)
            proof = live_proof_provider.read_live_proof(live_root, task_uid)
            evidence = _validate_live_proof(locked_mapping, task_uid, before_row, disposition_comment_id, proof)

            # Defend against a writer that bypasses the shared lock: read its
            # latest bytes before commit, reject a changed candidate row, and
            # base the atomic write on the latest unrelated task records.
            try:
                _checked_mapping_path(root)
                latest_bytes = mapping_path.read_bytes()
                latest = json.loads(latest_bytes)
            except (OSError, json.JSONDecodeError) as exc:
                raise RetirementError(f"cannot re-read task mapping for compare-and-swap: {exc}") from exc
            STORE.validate_retirement_ledger(latest)
            latest_row = (latest.get("tasks") or {}).get(task_uid)
            if latest_row != before_row:
                raise RetirementError("candidate mapping row changed during live proof; compare-and-swap refused")
            if list(latest.get("retired_duplicate_candidates", [])) != list(locked_mapping.get("retired_duplicate_candidates", [])):
                raise RetirementError("retirement ledger changed during live proof; compare-and-swap refused")
            locked_mapping.clear()
            locked_mapping.update(latest)
            now = dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")
            retirement = {
                "schema": SCHEMA,
                "task_uid": task_uid,
                "old_record": before_row,
                "old_record_sha256": canonical_digest(before_row),
                "evidence": evidence,
                "issuer": evidence["permission"]["login"],
                "time": now,
                "reason": "duplicate",
                "transaction_id": str(uuid.uuid4()),
            }
            retirement["digest"] = canonical_digest(retirement)
            tasks = locked_mapping.get("tasks")
            if not isinstance(tasks, dict) or tasks.get(task_uid) != before_row:
                raise RetirementError("candidate task row failed final compare-and-swap")
            del tasks[task_uid]
            locked_mapping.setdefault("retired_duplicate_candidates", []).append(retirement)
            STORE.validate_retirement_ledger(locked_mapping)
            result = {"status": "retired", "task_uid": task_uid, "digest": retirement["digest"]}
        # locked_json has atomically replaced and read-closed the mapping here.
        _checked_mapping_path(root)
        readback = STORE.read_mapping(mapping_path)
        retired = STORE.retired_task(readback, task_uid)
        if retired is None or retired.get("digest") != result["digest"] or task_uid in readback.get("tasks", {}):
            raise RetirementError("atomic retirement readback does not match the committed tombstone")
        return result
    except _AlreadyRetired as exc:
        return {"status": "already_retired", "task_uid": task_uid, "digest": exc.tombstone["digest"]}
    except RetirementError:
        raise
    except Exception as exc:
        raise RetirementError(f"retirement transaction failed without confirmed commit: {exc}") from exc


def _live_provider() -> LiveProofProvider:
    # The source code and durable store are always resolved relative to this
    # exact executing helper; callers cannot redirect the trust root.
    if not STORE_PATH.is_file() or STORE_PATH.resolve().parent != pathlib.Path(__file__).resolve().parent:
        raise RetirementError("pinned durable-store helper is unavailable beside this entrypoint")
    return LiveProofProvider()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Safely reconcile one closed duplicate candidate mapping.")
    parser.add_argument("--mapping-root", required=True, help="registered target Git worktree containing .pm/github-project-sync/tasks.json")
    parser.add_argument("--task-uid", required=True)
    parser.add_argument("--disposition-comment-id", required=True, type=int, help="server comment ID locator; body/author are re-read live")
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--preflight", action="store_true", help="read-only complete live proof")
    action.add_argument("--apply", action="store_true", help="atomically retire only the selected cache row")
    args = parser.parse_args(argv)
    try:
        result = retire_candidate(
            mapping_root=pathlib.Path(args.mapping_root),
            task_uid=args.task_uid,
            disposition_comment_id=args.disposition_comment_id,
            mode="apply" if args.apply else "preflight",
            live_proof_provider=_live_provider(),
        )
    except (RetirementError, OSError, ValueError) as exc:
        print(f"closed duplicate candidate retirement blocked: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(result, ensure_ascii=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
