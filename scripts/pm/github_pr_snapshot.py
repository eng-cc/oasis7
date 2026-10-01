#!/usr/bin/env python3
"""Strict, single-request PR/review snapshots over the shared GitHub client."""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import pathlib
import re
import subprocess
import sys
import types
from datetime import datetime, timezone
from typing import Any
from urllib.parse import urlsplit


SNAPSHOT_SCHEMA = "oasis7.github-pr-snapshot/v1"
QUERY_VERSION = "github-pr-snapshot/v1"
GITHUB_API_VERSION = "2022-11-28"
_API_MODULES: dict[pathlib.Path, types.ModuleType] = {}


def _github_api_module() -> types.ModuleType:
    path = pathlib.Path(__file__).with_name("github_api.py").resolve()
    module = _API_MODULES.get(path)
    if module is not None:
        return module
    if not path.is_file():
        raise RuntimeError(f"shared GitHub API client is unavailable at {path}")
    module_name = "_oasis7_github_api_" + hashlib.sha256(str(path).encode("utf-8")).hexdigest()[:16]
    module = sys.modules.get(module_name)
    if not isinstance(module, types.ModuleType):
        spec = importlib.util.spec_from_file_location(module_name, path)
        if spec is None or spec.loader is None:
            raise RuntimeError(f"cannot load shared GitHub API client at {path}")
        module = importlib.util.module_from_spec(spec)
        sys.modules[module_name] = module
        spec.loader.exec_module(module)
    _API_MODULES[path] = module
    return module


def _api_error(message: str, *, kind: str = "malformed_response", uncertain: bool = False,
               mutation_started: bool = False, details: dict[str, Any] | None = None) -> Exception:
    return _github_api_module().APIError(
        message,
        kind=kind,
        uncertain=uncertain,
        mutation_started=mutation_started,
        details=details,
    )


def _selector(repository: str, number: int) -> tuple[str, str]:
    match = re.fullmatch(r"([^/]+)/([^/]+)", repository.strip())
    if match is None or any(part in {".", ".."} for part in match.groups()):
        raise ValueError("repository must be owner/name")
    if type(number) is not int or number <= 0:
        raise ValueError("pull request number must be a positive integer")
    return match.group(1), match.group(2)


def _canonical_repository_hint(value: str | None) -> str | None:
    if value is None or not value.strip():
        return None
    candidate = value.strip()
    if candidate.startswith("git@github.com:"):
        candidate = candidate.split(":", 1)[1]
    elif "://" in candidate:
        parsed = urlsplit(candidate)
        if parsed.hostname is None or parsed.hostname.casefold() != "github.com":
            raise ValueError("repository hint URL must use github.com")
        parts = [part for part in parsed.path.split("/") if part]
        if len(parts) < 2:
            raise ValueError("repository hint URL must include owner and repository")
        candidate = "/".join(parts[:2])
    candidate = candidate.removesuffix(".git").strip("/")
    _selector(candidate, 1)
    return candidate


def _git_output(*args: str) -> str:
    try:
        result = subprocess.run(
            ["git", *args], check=True, text=True, stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL, timeout=5,
        )
    except (OSError, subprocess.CalledProcessError, subprocess.TimeoutExpired) as exc:
        raise ValueError("cannot resolve a GitHub PR selector from this worktree") from exc
    return result.stdout.strip()


def _origin_repository() -> str:
    repository = _canonical_repository_hint(_git_output("remote", "get-url", "origin"))
    if repository is None:
        raise ValueError("origin remote does not identify a GitHub repository")
    return repository


def resolve_pr_selector(
    client: Any,
    selector: str | int | None,
    repository_hint: str | None,
    context: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Resolve numeric, URL, owner/repo#number, or branch PR selectors.

    Numeric and URL selectors are local projections and spend no lookup call.
    A branch selector makes one scoped search request. If omitted, the current
    worktree branch and origin remote provide the selector and repository.
    """
    hint = _canonical_repository_hint(repository_hint)
    requested = "" if selector is None else str(selector).strip()
    if not requested:
        requested = _git_output("branch", "--show-current")
        if not requested:
            raise ValueError("detached worktree has no current branch PR selector")

    selector_repository = hint
    selector_kind = "number"
    number_match = re.fullmatch(r"#?([1-9][0-9]*)", requested)
    if re.fullmatch(r"#?[0-9]+", requested) and number_match is None:
        raise ValueError("pull request number must be a positive integer")
    if number_match:
        number = int(number_match.group(1))
    else:
        parsed_url = urlsplit(requested)
        if parsed_url.scheme or parsed_url.netloc:
            if (parsed_url.scheme != "https" or parsed_url.hostname is None
                    or parsed_url.hostname.casefold() != "github.com"):
                raise ValueError("pull request URL must be an https://github.com URL")
            match = re.fullmatch(r"/([^/]+)/([^/]+)/pull/([1-9][0-9]*)/?", parsed_url.path)
            if match is None:
                raise ValueError("pull request URL must end in /owner/repo/pull/number")
            selector_repository = f"{match.group(1)}/{match.group(2)}"
            number = int(match.group(3))
            selector_kind = "pull_request_url"
        else:
            match = re.fullmatch(r"([^/]+/[^/#]+)#([1-9][0-9]*)", requested)
            if match:
                selector_repository = match.group(1).removesuffix(".git")
                number = int(match.group(2))
                selector_kind = "repository_number"
            else:
                branch = requested
                if hint is None:
                    hint = _origin_repository()
                if any(ord(char) < 32 or char.isspace() for char in branch) or '"' in branch:
                    raise ValueError("branch selector contains unsupported whitespace or quotes")
                query = """
                query PRSelectorByHead($query: String!) {
                  search(type: ISSUE, first: 2, query: $query) {
                    issueCount
                    pageInfo { hasNextPage endCursor }
                    nodes { __typename ... on PullRequest { number url headRefName repository { nameWithOwner } } }
                  }
                }
                """
                safe_context = {"script": "github_pr_snapshot.py", "operation": "pr_selector_branch_lookup"}
                if context:
                    safe_context.update(context)
                data = client.graphql(
                    query,
                    {"query": f"repo:{hint} is:pr head:{branch}"},
                    operation="pr_selector_branch_lookup",
                    context=safe_context,
                )
                search = data.get("search") if isinstance(data, dict) else None
                if not isinstance(search, dict):
                    raise _api_error("branch PR selector search is missing", kind="malformed_response")
                issue_count = search.get("issueCount")
                page_info = search.get("pageInfo")
                nodes = search.get("nodes")
                if type(issue_count) is not int or issue_count < 0:
                    raise _api_error("branch PR selector issueCount is missing or malformed")
                if (not isinstance(page_info, dict) or "endCursor" not in page_info
                        or not isinstance(page_info.get("hasNextPage"), bool)
                        or (page_info.get("endCursor") is not None
                            and not isinstance(page_info.get("endCursor"), str))):
                    raise _api_error("branch PR selector pageInfo is missing or malformed")
                if not isinstance(nodes, list) or any(not isinstance(node, dict) for node in nodes):
                    raise _api_error("branch PR selector nodes are missing or malformed")
                if issue_count == 0:
                    raise _api_error("no pull request matches the current branch", kind="not_found")
                if issue_count != 1 or page_info["hasNextPage"] or len(nodes) != 1:
                    raise _api_error("branch matches multiple pull requests; use a PR number or URL",
                                     kind="ambiguous_selector")
                node = nodes[0]
                if node.get("__typename") != "PullRequest":
                    raise _api_error("branch selector result is not a pull request")
                repository_node = node.get("repository")
                selector_repository = _required_string(
                    repository_node.get("nameWithOwner") if isinstance(repository_node, dict) else None,
                    "search.nodes[0].repository.nameWithOwner",
                )
                number = node.get("number")
                if type(number) is not int or number <= 0:
                    raise _api_error("branch selector PR number is missing or malformed")
                _required_string(node.get("url"), "search.nodes[0].url")
                if node.get("headRefName") != branch:
                    raise _api_error("branch selector result does not match the requested branch",
                                     kind="identity_mismatch")
                selector_kind = "branch"

    if hint is None:
        hint = selector_repository or _origin_repository()
    if selector_repository is None:
        selector_repository = hint
    if selector_repository.casefold() != hint.casefold():
        raise _api_error("pull request selector repository does not match the repository hint",
                         kind="identity_mismatch")
    _selector(selector_repository, number)
    return {
        "repository": selector_repository,
        "number": number,
        "url": f"https://github.com/{selector_repository}/pull/{number}",
        "selector_kind": selector_kind,
    }


def _required_string(value: Any, name: str, *, nonempty: bool = True) -> str:
    if not isinstance(value, str) or (nonempty and not value.strip()):
        raise _api_error(f"PR snapshot field {name} is missing or malformed")
    return value


def _page_info(connection: Any, name: str) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    if not isinstance(connection, dict):
        raise _api_error(f"PR snapshot connection {name} is missing or malformed")
    nodes = connection.get("nodes")
    page_info = connection.get("pageInfo")
    if not isinstance(nodes, list) or not isinstance(page_info, dict):
        raise _api_error(f"PR snapshot connection {name} lacks nodes or pageInfo")
    has_next_page = page_info.get("hasNextPage")
    end_cursor = page_info.get("endCursor")
    if ("hasNextPage" not in page_info or "endCursor" not in page_info
            or not isinstance(has_next_page, bool)
            or (end_cursor is not None and not isinstance(end_cursor, str))):
        raise _api_error(f"PR snapshot connection {name} has invalid pageInfo")
    if any(not isinstance(node, dict) for node in nodes):
        raise _api_error(f"PR snapshot connection {name} contains a malformed node")
    return nodes, {
        "complete": not has_next_page,
        "has_next_page": has_next_page,
        "end_cursor": end_cursor,
        "node_count": len(nodes),
    }


def _validate_comment_nodes(nodes: list[dict[str, Any]], name: str, *, include_body: bool) -> None:
    for index, node in enumerate(nodes):
        _required_string(node.get("id"), f"{name}[{index}].id")
        if include_body:
            _required_string(node.get("body"), f"{name}[{index}].body", nonempty=False)


def _query(include_comment_bodies: bool) -> str:
    pr_body_field = "body" if include_comment_bodies else ""
    comment_fields = "body url createdAt author { login } authorAssociation" if include_comment_bodies else ""
    thread_comment_fields = "body createdAt url author { login }" if include_comment_bodies else ""
    review_body_field = "body" if include_comment_bodies else ""
    return f"""
    query GitHubPRSnapshot($owner: String!, $repo: String!, $number: Int!) {{
      viewer {{ login }}
      rateLimit {{ cost remaining used resetAt limit }}
      repository(owner: $owner, name: $repo) {{
        nameWithOwner
        pullRequest(number: $number) {{
          number url state isDraft {pr_body_field} mergeable mergeStateStatus reviewDecision
          headRefName headRefOid baseRefName baseRefOid
          comments(first: 100) {{ pageInfo {{ hasNextPage endCursor }} nodes {{ id {comment_fields} }} }}
          reviews(first: 100) {{ pageInfo {{ hasNextPage endCursor }} nodes {{
            id {review_body_field} url submittedAt createdAt state author {{ login }}
          }} }}
          reviewThreads(first: 100) {{ pageInfo {{ hasNextPage endCursor }} nodes {{
            id isResolved isOutdated path line originalLine startLine originalStartLine
            comments(first: 20) {{ pageInfo {{ hasNextPage endCursor }} nodes {{ id {thread_comment_fields} }} }}
          }} }}
          commits(last: 1) {{ nodes {{ commit {{ oid statusCheckRollup {{
            contexts(first: 100) {{ pageInfo {{ hasNextPage endCursor }} nodes {{
              __typename
              ... on CheckRun {{ databaseId name conclusion status checkSuite {{ app {{ databaseId }} }} }}
              ... on StatusContext {{ context state }}
            }}
          }} }} }} }} }}
        }}
      }}
    }}
    """


def _identity_query() -> str:
    return """
    query GitHubPRIdentity($owner: String!, $repo: String!, $number: Int!) {
      viewer { login }
      rateLimit { cost remaining used resetAt limit }
      repository(owner: $owner, name: $repo) {
        nameWithOwner
        pullRequest(number: $number) {
          number url state isDraft body headRefName headRefOid baseRefName baseRefOid
        }
      }
    }
    """


def _response_root(data: Any, repository: str, number: int) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
    if not isinstance(data, dict):
        raise _api_error("PR snapshot GraphQL data is not an object")
    repo = data.get("repository")
    if not isinstance(repo, dict):
        raise _api_error("PR snapshot repository is missing")
    actual_repository = _required_string(repo.get("nameWithOwner"), "repository.nameWithOwner")
    if actual_repository.casefold() != repository.casefold():
        raise _api_error("PR snapshot repository identity does not match the requested repository",
                         kind="identity_mismatch")
    pr = repo.get("pullRequest")
    if not isinstance(pr, dict):
        raise _api_error("PR snapshot pull request is missing")
    actual_number = pr.get("number")
    if type(actual_number) is not int or actual_number != number:
        raise _api_error("PR snapshot number does not match the requested pull request",
                         kind="identity_mismatch")
    viewer = data.get("viewer")
    rate_limit = data.get("rateLimit")
    if not isinstance(viewer, dict) or not isinstance(rate_limit, dict):
        raise _api_error("PR snapshot lacks viewer or rate-limit metadata")
    _required_string(viewer.get("login"), "viewer.login")
    for key in ("cost", "remaining", "used", "limit"):
        if type(rate_limit.get(key)) is not int or rate_limit[key] < 0:
            raise _api_error(f"PR snapshot rateLimit.{key} is missing or malformed")
    _required_string(rate_limit.get("resetAt"), "rateLimit.resetAt")
    return repo, pr, {"viewer": viewer, "rateLimit": rate_limit}


def fetch_pr_snapshot(
    client: Any,
    repository: str,
    number: int,
    *,
    context: dict[str, Any] | None = None,
    fresh: bool = True,
    include_comment_bodies: bool = True,
) -> dict[str, Any]:
    """Fetch and strictly decode all bounded PR-review surfaces in one query.

    This function has no snapshot cache. Every invocation sends one fresh
    GraphQL read; observation deduplication, if needed, wraps this adapter.
    """
    owner, repo_name = _selector(repository, number)
    variables = {"owner": owner, "repo": repo_name, "number": number}
    safe_context = {"script": "github_pr_snapshot.py", "pr_number": number}
    if context:
        safe_context.update(context)
    data = client.graphql(
        _query(include_comment_bodies),
        variables,
        operation="pr_snapshot_full" if include_comment_bodies else "pr_snapshot_summary",
        context=safe_context,
    )
    repo_node, pr, observed = _response_root(data, repository, number)

    for key in ("url", "state", "headRefName", "headRefOid", "baseRefName", "baseRefOid"):
        _required_string(pr.get(key), key)
    if not isinstance(pr.get("isDraft"), bool):
        raise _api_error("PR snapshot isDraft is missing or malformed")
    if include_comment_bodies:
        _required_string(pr.get("body"), "body", nonempty=False)
    for key in ("mergeable", "mergeStateStatus", "reviewDecision"):
        if pr.get(key) is not None and not isinstance(pr.get(key), str):
            raise _api_error(f"PR snapshot {key} is malformed")

    connections: dict[str, dict[str, Any]] = {}
    comments, connections["comments"] = _page_info(pr.get("comments"), "comments")
    _validate_comment_nodes(comments, "comments.nodes", include_body=include_comment_bodies)
    reviews, connections["reviews"] = _page_info(pr.get("reviews"), "reviews")
    for index, review in enumerate(reviews):
        _required_string(review.get("id"), f"reviews[{index}].id")
        if include_comment_bodies:
            _required_string(review.get("body"), f"reviews[{index}].body", nonempty=False)
        _required_string(review.get("state"), f"reviews[{index}].state")

    threads, connections["reviewThreads"] = _page_info(pr.get("reviewThreads"), "reviewThreads")
    thread_comment_connections: dict[str, dict[str, Any]] = {}
    for index, thread in enumerate(threads):
        thread_id = _required_string(thread.get("id"), f"reviewThreads[{index}].id")
        if not isinstance(thread.get("isResolved"), bool) or not isinstance(thread.get("isOutdated"), bool):
            raise _api_error(f"reviewThreads[{index}] lacks resolution/outdated state")
        thread_comments, thread_meta = _page_info(thread.get("comments"), f"reviewThreads[{index}].comments")
        _validate_comment_nodes(
            thread_comments,
            f"reviewThreads[{index}].comments.nodes",
            include_body=include_comment_bodies,
        )
        thread_comment_connections[thread_id] = thread_meta

    commit_connection = pr.get("commits")
    commit_nodes = commit_connection.get("nodes") if isinstance(commit_connection, dict) else None
    if not isinstance(commit_nodes, list) or not commit_nodes or not isinstance(commit_nodes[-1], dict):
        raise _api_error("PR snapshot lacks the latest commit required for check status")
    latest_commit = commit_nodes[-1].get("commit")
    if not isinstance(latest_commit, dict):
        raise _api_error("PR snapshot latest commit is missing")
    check_rollup = latest_commit.get("statusCheckRollup")
    if not isinstance(check_rollup, dict):
        raise _api_error("PR snapshot statusCheckRollup is missing or malformed")
    checks, connections["statusCheckRollup"] = _page_info(check_rollup.get("contexts"), "statusCheckRollup")
    for index, check in enumerate(checks):
        typename = check.get("__typename")
        if typename == "CheckRun":
            _required_string(check.get("name"), f"statusCheckRollup[{index}].name")
        elif typename == "StatusContext":
            _required_string(check.get("context"), f"statusCheckRollup[{index}].context")
        else:
            raise _api_error(f"statusCheckRollup[{index}] has an unsupported check type")

    connection_meta = dict(connections)
    connection_meta["thread_comments"] = thread_comment_connections
    complete = all(item["complete"] for item in connections.values()) and all(
        item["complete"] for item in thread_comment_connections.values()
    )
    snapshot = {
        "repository": _required_string(repo_node.get("nameWithOwner"), "repository.nameWithOwner"),
        "number": number,
        "url": pr["url"],
        "state": pr["state"],
        "isDraft": pr["isDraft"],
        **({"body": pr["body"]} if include_comment_bodies else {}),
        "mergeable": pr.get("mergeable"),
        "mergeStateStatus": pr.get("mergeStateStatus"),
        "reviewDecision": pr.get("reviewDecision"),
        "headRefName": pr["headRefName"],
        "headRefOid": pr["headRefOid"],
        "baseRefName": pr["baseRefName"],
        "baseRefOid": pr["baseRefOid"],
        "comments": comments,
        "reviews": reviews,
        "threads": threads,
        "statusCheckRollup": checks,
        "viewer": observed["viewer"],
        "rateLimit": observed["rateLimit"],
        "snapshot_metadata": {
            "schema": SNAPSHOT_SCHEMA,
            "query_version": QUERY_VERSION,
            "api_version": GITHUB_API_VERSION,
            "fetched_at": datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z"),
            "fresh": True,
            "requested_fresh": bool(fresh),
            "complete": complete,
            "summary": not include_comment_bodies,
            "connections": connection_meta,
        },
    }
    return snapshot


def fetch_pr_identity(
    client: Any,
    repository: str,
    number: int,
    *,
    context: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Perform a separate uncached PR identity query for final gate readback."""
    owner, repo_name = _selector(repository, number)
    safe_context = {"script": "github_pr_snapshot.py", "pr_number": number}
    if context:
        safe_context.update(context)
    data = client.graphql(
        _identity_query(),
        {"owner": owner, "repo": repo_name, "number": number},
        operation="pr_identity_fresh",
        context=safe_context,
    )
    repo_node, pr, observed = _response_root(data, repository, number)
    for key in ("url", "state", "headRefName", "headRefOid", "baseRefName", "baseRefOid"):
        _required_string(pr.get(key), key)
    if not isinstance(pr.get("isDraft"), bool):
        raise _api_error("PR identity isDraft is missing or malformed")
    _required_string(pr.get("body"), "body", nonempty=False)
    return {
        "repository": repo_node["nameWithOwner"],
        "number": number,
        "url": pr["url"],
        "state": pr["state"],
        "isDraft": pr["isDraft"],
        "body": pr["body"],
        "headRefName": pr["headRefName"],
        "headRefOid": pr["headRefOid"],
        "baseRefName": pr["baseRefName"],
        "baseRefOid": pr["baseRefOid"],
        "identity_metadata": {
            "query_version": QUERY_VERSION + "/identity",
            "api_version": GITHUB_API_VERSION,
            "fetched_at": datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z"),
            "fresh": True,
            "viewer": observed["viewer"],
            "rateLimit": observed["rateLimit"],
        },
    }


def _read_thread_resolution(client: Any, repository: str, number: int, thread_id: str,
                            context: dict[str, Any] | None) -> bool | None:
    owner, repo_name = _selector(repository, number)
    query = """
    query PRReviewThreadReadback($owner:String!,$repo:String!,$number:Int!) {
      repository(owner:$owner,name:$repo) {
        nameWithOwner
        pullRequest(number:$number) {
          number
          reviewThreads(first:100) { pageInfo { hasNextPage endCursor } nodes { id isResolved } }
        }
      }
    }
    """
    safe_context = {"script": "github_pr_snapshot.py", "pr_number": number}
    if context:
        safe_context.update(context)
    data = client.graphql(
        query,
        {"owner": owner, "repo": repo_name, "number": number},
        operation="pr_review_thread_uncertain_readback",
        context=safe_context,
    )
    repo_node, pr, _observed = _response_root_for_thread(data, repository, number)
    nodes, page_info = _page_info(pr.get("reviewThreads"), "reviewThreads")
    if page_info["has_next_page"]:
        return None
    for node in nodes:
        if node.get("id") == thread_id:
            if not isinstance(node.get("isResolved"), bool):
                raise _api_error("review thread readback state is malformed")
            return node["isResolved"]
    return None


def _response_root_for_thread(data: Any, repository: str, number: int):
    if not isinstance(data, dict):
        raise _api_error("review thread readback data is malformed")
    repo = data.get("repository")
    if not isinstance(repo, dict):
        raise _api_error("review thread readback repository is missing")
    actual_repository = _required_string(repo.get("nameWithOwner"), "repository.nameWithOwner")
    if actual_repository.casefold() != repository.casefold():
        raise _api_error("review thread readback repository identity mismatch", kind="identity_mismatch")
    pr = repo.get("pullRequest")
    if not isinstance(pr, dict) or type(pr.get("number")) is not int or pr["number"] != number:
        raise _api_error("review thread readback PR identity mismatch", kind="identity_mismatch")
    return repo, pr, None


def _resolve_one(client: Any, repository: str, number: int, thread_id: str,
                 context: dict[str, Any] | None) -> dict[str, Any]:
    _selector(repository, number)
    _required_string(thread_id, "thread_id")
    mutation = """
    mutation ResolvePRReviewThread($threadId: ID!) {
      resolveReviewThread(input: {threadId: $threadId}) {
        thread { id isResolved }
      }
    }
    """
    safe_context = {"script": "github_pr_snapshot.py", "pr_number": number}
    if context:
        safe_context.update(context)
    try:
        data = client.graphql(
            mutation,
            {"threadId": thread_id},
            operation="pr_review_thread_resolve",
            mutation=True,
            context=safe_context,
        )
    except Exception as exc:
        if not (getattr(exc, "uncertain", False) or getattr(exc, "mutation_started", False)):
            raise
        try:
            resolved = _read_thread_resolution(client, repository, number, thread_id, context)
        except Exception as readback_error:
            raise exc from readback_error
        if resolved is True:
            return {"id": thread_id, "isResolved": True, "confirmed_after_uncertain": True}
        raise
    result = data.get("resolveReviewThread") if isinstance(data, dict) else None
    thread = result.get("thread") if isinstance(result, dict) else None
    if (not isinstance(thread, dict) or thread.get("id") != thread_id
            or thread.get("isResolved") is not True):
        malformed = _api_error(
            "resolveReviewThread response did not confirm the selected thread",
            uncertain=True,
            mutation_started=True,
            details={"thread_id": thread_id},
        )
        try:
            resolved = _read_thread_resolution(client, repository, number, thread_id, context)
        except Exception as readback_error:
            raise malformed from readback_error
        if resolved is True:
            return {"id": thread_id, "isResolved": True, "confirmed_after_uncertain": True}
        raise malformed
    return {"id": thread_id, "isResolved": True, "confirmed_after_uncertain": False}


def closeout_review_threads(
    client: Any,
    repository: str,
    number: int,
    *,
    thread_ids: list[str] | None = None,
    resolve_all_unresolved: bool = False,
    context: dict[str, Any] | None = None,
    include_comment_bodies: bool = True,
) -> tuple[dict[str, Any], list[str]]:
    """Resolve selected threads using one pre-read, N writes, and one post-read."""
    pre = fetch_pr_snapshot(client, repository, number, context=context,
                            include_comment_bodies=include_comment_bodies)
    if not pre["snapshot_metadata"]["complete"]:
        raise _api_error("review thread pre-scan is partial; refusing resolution", kind="incomplete_snapshot")
    unresolved = {item["id"] for item in pre["threads"] if not item["isResolved"]}
    if resolve_all_unresolved:
        selected = [item["id"] for item in pre["threads"] if not item["isResolved"]]
    else:
        selected = list(thread_ids or [])
    if any(not isinstance(thread_id, str) or not thread_id.strip() for thread_id in selected):
        raise ValueError("review thread IDs must be non-empty strings")
    if len(selected) != len(set(selected)):
        raise ValueError("duplicate review thread IDs are not allowed")
    invalid = [thread_id for thread_id in selected if thread_id not in unresolved]
    if invalid:
        raise ValueError("thread is not currently unresolved on the selected PR: " + ", ".join(invalid))
    if not selected:
        return pre, []
    for thread_id in selected:
        _resolve_one(client, repository, number, thread_id, context)
    post = fetch_pr_snapshot(client, repository, number, context=context,
                             include_comment_bodies=include_comment_bodies)
    if not post["snapshot_metadata"]["complete"]:
        raise _api_error("review thread post-scan is partial; resolution cannot be confirmed",
                         kind="incomplete_snapshot", uncertain=True, mutation_started=True,
                         details={"thread_ids": selected})
    post_threads = {item["id"]: item for item in post["threads"]}
    if any(thread_id not in post_threads or post_threads[thread_id]["isResolved"] is not True
           for thread_id in selected):
        raise _api_error("post-write review snapshot did not confirm every selected thread",
                         kind="mutation_readback_mismatch", uncertain=True, mutation_started=True,
                         details={"thread_ids": selected})
    return post, selected


def render_review_report(snapshot: dict[str, Any], *, unresolved_only: bool = False,
                         resolved_now: list[str] | None = None) -> dict[str, Any]:
    if not isinstance(snapshot, dict) or not isinstance(snapshot.get("snapshot_metadata"), dict):
        raise _api_error("review report snapshot is malformed")
    threads = snapshot.get("threads")
    if not isinstance(threads, list):
        raise _api_error("review report threads are malformed")
    connections = snapshot["snapshot_metadata"].get("connections") or {}
    partial_reasons = []
    for name, connection in connections.items():
        if name == "thread_comments":
            for thread_id, item in connection.items():
                if not item.get("complete"):
                    partial_reasons.append(f"comments(first:20) has additional pages for {thread_id}")
        elif isinstance(connection, dict) and not connection.get("complete"):
            label = "reviewThreads" if name == "reviewThreads" else name
            partial_reasons.append(f"{label} connection has additional pages")
    reported = []
    for thread in threads:
        if unresolved_only and thread["isResolved"]:
            continue
        comments_connection = thread.get("comments") or {}
        comments = comments_connection.get("nodes") or []
        latest = comments[-1] if comments else None
        entry = {
            "id": thread["id"],
            "is_resolved": thread["isResolved"],
            "is_outdated": thread["isOutdated"],
            "path": thread.get("path"),
            "line": thread.get("line"),
            "original_line": thread.get("originalLine"),
            "start_line": thread.get("startLine"),
            "original_start_line": thread.get("originalStartLine"),
            "comment_count": len(comments),
        }
        if latest and not snapshot["snapshot_metadata"].get("summary"):
            entry["latest_comment"] = {
                "author": ((latest.get("author") or {}).get("login")),
                "body": latest.get("body"),
                "created_at": latest.get("createdAt"),
                "url": latest.get("url"),
            }
        else:
            entry["latest_comment"] = None
        reported.append(entry)
    return {
        "pr": {
            "number": snapshot["number"],
            "url": snapshot["url"],
            "head_ref": snapshot["headRefName"],
            "base_ref": snapshot["baseRefName"],
            "review_decision": snapshot["reviewDecision"],
            "merge_state_status": snapshot["mergeStateStatus"],
        },
        "summary": {
            "total_threads": len(threads),
            "unresolved_threads": sum(1 for item in threads if not item["isResolved"]),
            "resolved_threads": sum(1 for item in threads if item["isResolved"]),
            "reported_threads": len(reported),
            "unresolved_only": unresolved_only,
            "partial_scan": bool(partial_reasons),
            "partial_reasons": partial_reasons,
            "summary": bool(snapshot["snapshot_metadata"].get("summary")),
        },
        "resolved_now": {"count": len(resolved_now or []), "thread_ids": list(resolved_now or [])},
        "threads": reported,
    }


def _make_client() -> Any:
    return _github_api_module().GitHubAPIClient.from_gh()


def _cli(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    snapshot_parser = subparsers.add_parser("snapshot")
    snapshot_parser.add_argument("selector", nargs="?")
    snapshot_parser.add_argument("--repository-hint")
    snapshot_parser.add_argument("--summary", action="store_true")
    closeout_parser = subparsers.add_parser("closeout")
    closeout_parser.add_argument("selector", nargs="?")
    closeout_parser.add_argument("--repository-hint")
    closeout_parser.add_argument("--summary", action="store_true")
    closeout_parser.add_argument("--unresolved-only", action="store_true")
    closeout_parser.add_argument("--resolve-all-unresolved", action="store_true")
    closeout_parser.add_argument("--resolve-thread", action="append", default=[])
    args = parser.parse_args(argv)
    client = _make_client()
    resolved = resolve_pr_selector(client, args.selector, args.repository_hint,
                                   context={"script": "pr-review-thread-closeout.sh"})
    repository = resolved["repository"]
    number = resolved["number"]
    context = {"script": "pr-review-thread-closeout.sh", "pr_number": number}
    include_bodies = not args.summary
    if args.command == "snapshot":
        payload = render_review_report(
            fetch_pr_snapshot(client, repository, number,
                              context=context, include_comment_bodies=include_bodies)
        )
        if args.summary:
            payload["threads"] = []
            payload["summary"]["reported_threads"] = 0
        print(json.dumps(payload, ensure_ascii=True, indent=2))
        return 0
    if args.resolve_all_unresolved and args.resolve_thread:
        parser.error("--resolve-all-unresolved cannot be combined with --resolve-thread")
    if not args.resolve_all_unresolved and not args.resolve_thread:
        payload = render_review_report(
            fetch_pr_snapshot(client, repository, number,
                              context=context, include_comment_bodies=include_bodies),
            unresolved_only=args.unresolved_only,
        )
        if payload["summary"]["partial_scan"]:
            raise _api_error("review thread scan is partial; refusing to continue",
                             kind="incomplete_snapshot",
                             details={"partial_reasons": payload["summary"]["partial_reasons"]})
    else:
        snapshot, resolved = closeout_review_threads(
            client,
            repository,
            number,
            thread_ids=args.resolve_thread,
            resolve_all_unresolved=args.resolve_all_unresolved,
            context=context,
            include_comment_bodies=include_bodies,
        )
        payload = render_review_report(snapshot, unresolved_only=args.unresolved_only,
                                       resolved_now=resolved)
    print(json.dumps(payload, ensure_ascii=True, indent=2))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(_cli())
    except Exception as exc:
        api = _github_api_module()
        if isinstance(exc, api.APIError):
            print(json.dumps(exc.as_dict(), sort_keys=True), file=sys.stderr)
            raise SystemExit(exc.exit_code)
        print(f"github_pr_snapshot: {exc}", file=sys.stderr)
        raise SystemExit(2)
