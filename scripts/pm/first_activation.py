#!/usr/bin/env python3
"""Read-only validation and local admission helpers for one Cargo gate activation."""
from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import pathlib
import re
import subprocess
import tempfile
import sys
import argparse
from typing import Any


OVERLAY_SCHEMA = "oasis7-cargo-dependency-floor-overlay/v1"
OVERLAY_MARKER = "<!-- oasis7-cargo-dependency-floor-overlay/v1 -->"
ACTIVATION_SCHEMA = "oasis7-cargo-dependency-floor-activation/v1"
ACTIVATION_MARKER = "<!-- oasis7-cargo-dependency-floor-activation/v1 -->"
REVIEW_SCHEMA = "oasis7-cargo-first-activation-review/v1"
REVIEW_MARKER = "<!-- oasis7-cargo-first-activation-review/v1 -->"
TASK_UID_RE = re.compile(r"task_[0-9a-f]{32}\Z")
OID_RE = re.compile(r"[0-9a-f]{40}\Z")
SHA_RE = re.compile(r"sha256:[0-9a-f]{64}\Z")
PACKAGE_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9_-]*\Z")
VERSION_RE = re.compile(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\Z")
MAX_PAGES = 100


class OverlayError(ValueError):
    pass


def _fail(message: str) -> None:
    raise OverlayError(message)


def _object(value: Any, keys: set[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        _fail(f"{label} has an unexpected shape")
    return value


def _digest(value: Any, label: str) -> str:
    if not isinstance(value, str) or not SHA_RE.fullmatch(value):
        _fail(f"{label} is not a canonical sha256 digest")
    return value


def _canonical(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def canonical_overlay_body(value: dict[str, Any]) -> str:
    return OVERLAY_MARKER + "\n" + _canonical(value).decode("utf-8") + "\n"


def _raw_primary(value: Any) -> dict[str, Any]:
    row = _object(value, {"present", "value"}, "raw_primary_package")
    if type(row["present"]) is not bool:
        _fail("raw_primary_package.present must be boolean")
    raw = row["value"]
    if raw is not None and (not isinstance(raw, str) or not PACKAGE_RE.fullmatch(raw)):
        _fail("raw_primary_package.value is malformed")
    if not row["present"] and raw is not None:
        _fail("absent raw primary package cannot carry a value")
    return {"present": row["present"], "value": raw}


def _validate_path(value: Any) -> str:
    if not isinstance(value, str) or not value or value.startswith("/") or "\\" in value:
        _fail("workflow change path is not normalized")
    parts = value.split("/")
    if any(part in {"", ".", ".."} for part in parts):
        _fail("workflow change path is not normalized")
    return value


def validate_overlay(
    value: Any,
    *,
    task_uid: str,
    issue_number: int,
    base_oid: str,
    head_oid: str,
    expected_raw_primary: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Validate immutable overlay bytes against caller assertions (which never select authority)."""
    top = _object(value, {
        "schema", "task_uid", "issue_number", "bootstrap_epoch", "snapshot_sha256",
        "request_sha256", "acceptance_sha256", "raw_primary_package",
        "effective_primary_package", "mode", "dependency", "base_oid", "head_oid",
        "authorization", "reviews", "workflow",
    }, "overlay")
    if top["schema"] != OVERLAY_SCHEMA or top["task_uid"] != task_uid or not TASK_UID_RE.fullmatch(task_uid):
        _fail("overlay schema or Task UID mismatch")
    if type(top["issue_number"]) is not int or top["issue_number"] != issue_number or issue_number < 1:
        _fail("overlay Issue identity mismatch")
    if type(top["bootstrap_epoch"]) is not int or top["bootstrap_epoch"] < 1:
        _fail("overlay bootstrap epoch is invalid")
    for key in ("snapshot_sha256", "request_sha256", "acceptance_sha256"):
        _digest(top[key], key)
    raw_primary = _raw_primary(top["raw_primary_package"])
    if expected_raw_primary is not None and raw_primary != _raw_primary(expected_raw_primary):
        _fail("overlay raw primary package differs from immutable Task")
    package = top["effective_primary_package"]
    if not isinstance(package, str) or not PACKAGE_RE.fullmatch(package):
        _fail("effective primary package is malformed")
    if top["mode"] != "dependency_floor_update":
        _fail("overlay mode is not the supported dependency-floor update")
    dependency = _object(top["dependency"], {"name", "base_requirement", "head_requirement"}, "dependency")
    if not isinstance(dependency["name"], str) or not PACKAGE_RE.fullmatch(dependency["name"]):
        _fail("dependency name is malformed")
    base_version = VERSION_RE.fullmatch(str(dependency["base_requirement"]))
    head_version = VERSION_RE.fullmatch(str(dependency["head_requirement"]))
    if not base_version or not head_version:
        _fail("dependency requirements must be canonical stable M.m.p versions")
    if base_version.group(1, 2) != head_version.group(1, 2) or int(head_version.group(3)) <= int(base_version.group(3)):
        _fail("dependency floor must raise only the patch version")
    if (not isinstance(top["base_oid"], str) or not OID_RE.fullmatch(top["base_oid"])
            or top["base_oid"] != base_oid or not isinstance(top["head_oid"], str)
            or not OID_RE.fullmatch(top["head_oid"]) or top["head_oid"] != head_oid):
        _fail("overlay base/head differs from frozen invocation")
    authorization = _object(top["authorization"], {"comment_id", "body_sha256", "scope"}, "authorization")
    if type(authorization["comment_id"]) is not int or authorization["comment_id"] < 1:
        _fail("authorization comment ID is invalid")
    _digest(authorization["body_sha256"], "authorization body digest")
    if authorization["scope"] != "effective_package_dependency_floor":
        _fail("authorization scope is not exact")
    reviews = _object(top["reviews"], {"repository_health", "qa"}, "reviews")
    for role in ("repository_health", "qa"):
        row = _object(reviews[role], {"comment_id", "body_sha256"}, f"{role} review")
        if type(row["comment_id"]) is not int or row["comment_id"] < 1:
            _fail(f"{role} review comment ID is invalid")
        _digest(row["body_sha256"], f"{role} review digest")
    ids = [authorization["comment_id"], reviews["repository_health"]["comment_id"], reviews["qa"]["comment_id"]]
    if len(set(ids)) != len(ids):
        _fail("authorization and independent review evidence must use distinct comments")
    workflow = _object(top["workflow"], {"id", "path", "ref", "sha", "file_sha256", "change_paths"}, "workflow")
    if type(workflow["id"]) is not int or workflow["id"] < 1:
        _fail("workflow ID is invalid")
    workflow_path = _validate_path(workflow["path"])
    if not isinstance(workflow["ref"], str) or not workflow["ref"].startswith("refs/heads/"):
        _fail("workflow ref must be a candidate branch ref")
    if workflow["sha"] != head_oid or not OID_RE.fullmatch(str(workflow["sha"])):
        _fail("workflow SHA differs from frozen head")
    _digest(workflow["file_sha256"], "workflow file digest")
    paths = workflow["change_paths"]
    if not isinstance(paths, list) or not paths:
        _fail("workflow change path set is empty")
    normalized: list[dict[str, str]] = []
    seen: set[str] = set()
    for item in paths:
        row = _object(item, {"path", "base_sha256", "head_sha256"}, "workflow change path")
        path = _validate_path(row["path"])
        if path in seen:
            _fail("workflow change path set contains duplicates")
        seen.add(path)
        normalized.append({"path": path,
                           "base_sha256": _digest(row["base_sha256"], "base path digest"),
                           "head_sha256": _digest(row["head_sha256"], "head path digest")})
    if normalized != sorted(normalized, key=lambda row: row["path"]):
        _fail("workflow change path set is not canonically sorted")
    if any(row["path"] not in FIRST_ACTIVATION_REVIEWED_PATHS for row in normalized):
        _fail("workflow change path set contains a path outside the approved first-activation surface")
    workflow_rows = [row for row in normalized if row["path"] == workflow_path]
    if len(workflow_rows) != 1 or workflow_rows[0]["head_sha256"] != workflow["file_sha256"]:
        _fail("workflow file digest is not bound by the reviewed path set")
    return {
        "schema": OVERLAY_SCHEMA,
        "task_uid": task_uid,
        "issue_number": issue_number,
        "mode": "dependency_floor_update",
        "effective_primary_package": package,
        "dependency_name": dependency["name"],
        "base_requirement": dependency["base_requirement"],
        "head_requirement": dependency["head_requirement"],
        "base_oid": base_oid,
        "head_oid": head_oid,
        "workflow_change_paths": normalized,
        "workflow_id": workflow["id"],
        "workflow_path": workflow_path,
        "workflow_ref": workflow["ref"],
        "workflow_sha": workflow["sha"],
        "workflow_file_sha256": workflow["file_sha256"],
        "overlay_comment_id": None,
        "overlay_sha256": None,
        "validation_only": True,
        "_overlay": top,
    }


def _github_client(repo_root: pathlib.Path):
    path = repo_root / "scripts/pm/github_api.py"
    spec = importlib.util.spec_from_file_location("first_activation_github_api", path)
    if spec is None or spec.loader is None:
        _fail("shared GitHub API client is unavailable")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module.GitHubAPIClient.from_gh()


def _pages(client: Any, path: str, operation: str) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    seen_ids: set[int] = set()
    for page in range(1, MAX_PAGES + 1):
        separator = "&" if "?" in path else "?"
        result = client.rest("GET", f"{path}{separator}per_page=100&page={page}", operation=operation)
        if not isinstance(result, list) or len(result) > 100:
            _fail(f"{operation} pagination returned a malformed page")
        for item in result:
            if not isinstance(item, dict) or type(item.get("id")) is not int or item["id"] < 1:
                _fail(f"{operation} pagination returned an invalid identity")
            if item["id"] in seen_ids:
                _fail(f"{operation} pagination repeated an identity")
            seen_ids.add(item["id"])
            rows.append(item)
        if len(result) < 100:
            return rows
    _fail(f"{operation} pagination limit exhausted")


def _body_uid(body: str) -> list[str]:
    return re.findall(r"(?m)^task_uid:\s*(task_[0-9a-f]{32})\s*$", body.replace("\r\n", "\n"))


def _raw_primary_from_issue(body: str) -> dict[str, Any]:
    rows = re.findall(r"(?m)^- primary_package:(.*)$", body.replace("\r\n", "\n"))
    if not rows:
        return {"present": False, "value": None}
    if len(rows) != 1:
        _fail("Task Issue primary_package field is ambiguous")
    value = rows[0].strip()
    if value in {"null", "None"}:
        return {"present": True, "value": None}
    tick = chr(96)
    match = re.fullmatch(tick + r"([^`\n]+)" + tick, value)
    if not match or not PACKAGE_RE.fullmatch(match.group(1)):
        _fail("Task Issue primary_package field is malformed")
    return {"present": True, "value": match.group(1)}


def _issue_by_uid(client: Any, repository: str, task_uid: str) -> dict[str, Any]:
    issues = _pages(client, f"repos/{repository}/issues?state=all&sort=created&direction=asc", "first_activation_issue_scan")
    matches = []
    for issue in issues:
        if "pull_request" in issue:
            continue
        body = issue.get("body")
        if body is not None and not isinstance(body, str):
            _fail("Issue body is malformed")
        if task_uid in _body_uid(body or ""):
            if _body_uid(body or "") != [task_uid]:
                _fail("Task Issue UID marker is ambiguous")
            matches.append(issue)
    if len(matches) != 1:
        _fail("Task UID does not resolve to exactly one live Issue")
    issue = matches[0]
    if issue.get("state") != "open" or type(issue.get("number")) is not int:
        _fail("Task Issue is not open or has invalid number")
    return issue


def _comments(client: Any, repository: str, issue_number: int) -> list[dict[str, Any]]:
    return _pages(client, f"repos/{repository}/issues/{issue_number}/comments", "first_activation_comment_scan")


def _authenticated_login(client: Any) -> str:
    identity = client.rest("GET", "user", operation="first_activation_operator_identity")
    login = identity.get("login") if isinstance(identity, dict) else None
    if not isinstance(login, str) or not login or login.endswith("[bot]"):
        _fail("authenticated GitHub operator identity is unavailable")
    return login


def _unique_comment(comments: list[dict[str, Any]], comment_id: int, label: str) -> dict[str, Any]:
    rows = [row for row in comments if row.get("id") == comment_id]
    if len(rows) != 1 or not isinstance(rows[0].get("body"), str):
        _fail(f"{label} comment is unavailable or ambiguous")
    return rows[0]


def _decode_overlay_comment(body: str) -> dict[str, Any]:
    normalized = body.replace("\r\n", "\n")
    if not normalized.startswith(OVERLAY_MARKER + "\n"):
        _fail("overlay comment marker is malformed")
    raw = normalized[len(OVERLAY_MARKER) + 1:]
    try:
        value = json.loads(raw, object_pairs_hook=_unique_object)
    except (ValueError, json.JSONDecodeError) as exc:
        _fail(f"overlay comment JSON is malformed: {exc}")
    if not isinstance(value, dict) or canonical_overlay_body(value) != normalized:
        _fail("overlay comment is not canonical immutable JSON")
    return value


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _validate_referenced_evidence(
    repository: str, issue: dict[str, Any], comments: list[dict[str, Any]], overlay: dict[str, Any],
) -> None:
    issue_number = int(issue["number"])
    canonical_issue_url = f"https://api.github.com/repos/{repository}/issues/{issue_number}"
    fields = overlay["_overlay"]
    dependency = fields["dependency"]
    package = fields["effective_primary_package"]
    uid = fields["task_uid"]
    head = fields["head_oid"]
    auth = fields["authorization"]
    auth_comment = _unique_comment(comments, auth["comment_id"], "user authorization")
    if auth_comment.get("issue_url") != canonical_issue_url:
        _fail("user authorization is attached to a different Issue")
    auth_body = str(auth_comment["body"])
    if "sha256:" + hashlib.sha256(auth_body.encode("utf-8")).hexdigest() != auth["body_sha256"]:
        _fail("user authorization bytes changed")
    auth_user = auth_comment.get("user")
    auth_login = auth_user.get("login") if isinstance(auth_user, dict) else None
    if not isinstance(auth_login, str) or not auth_login or auth_login.endswith("[bot]") or not auth_body.strip():
        _fail("user authorization author/body is invalid")
    for role in ("repository_health", "qa"):
        row = fields["reviews"][role]
        comment = _unique_comment(comments, row["comment_id"], f"{role} review")
        if comment.get("issue_url") != canonical_issue_url:
            _fail(f"{role} review is attached to a different Issue")
        body = str(comment["body"])
        if "sha256:" + hashlib.sha256(body.encode("utf-8")).hexdigest() != row["body_sha256"]:
            _fail(f"{role} review bytes changed")
        normalized = body.replace("\r\n", "\n")
        if not normalized.startswith(REVIEW_MARKER + "\n"):
            _fail(f"{role} review is not a typed first-activation record")
        try:
            review = json.loads(normalized[len(REVIEW_MARKER) + 1:], object_pairs_hook=_unique_object)
        except (ValueError, json.JSONDecodeError) as exc:
            _fail(f"{role} review record JSON is malformed: {exc}")
        if not isinstance(review, dict) or REVIEW_MARKER + "\n" + _canonical(review).decode("utf-8") + "\n" != normalized:
            _fail(f"{role} review record is not canonical immutable JSON")
        if set(review) != {
            "schema", "task_uid", "issue_number", "role", "slice_id", "base_oid", "head_oid",
            "workflow_change_paths", "review_plan_sha256", "admitted_packet_sha256",
            "return_sha256", "return_status", "findings", "unresolved_findings",
        }:
            _fail(f"{role} review record has an unexpected shape")
        if (review["schema"] != REVIEW_SCHEMA or review["task_uid"] != uid
                or review["issue_number"] != issue_number or review["role"] != role
                or not isinstance(review["slice_id"], str) or not review["slice_id"].strip()
                or review["base_oid"] != overlay["base_oid"] or review["head_oid"] != head):
            _fail(f"{role} review is not bound to the exact Task, Issue, base, head, or role")
        if review["workflow_change_paths"] != fields["workflow"]["change_paths"]:
            _fail(f"{role} review does not bind the exact reviewed workflow path set")
        for digest_name in ("review_plan_sha256", "admitted_packet_sha256", "return_sha256"):
            _digest(review[digest_name], f"{role} {digest_name}")
        if review["return_status"] != "completed" or review["findings"] != [] or review["unresolved_findings"] != []:
            _fail(f"{role} review has incomplete return or unresolved findings")
        reviewer = comment.get("user")
        reviewer_name = reviewer.get("login") if isinstance(reviewer, dict) else None
        if (not isinstance(reviewer_name, str) or not reviewer_name
                or reviewer_name.endswith("[bot]")):
            _fail(f"{role} review publication author is unavailable")
    expected_terms = (package, dependency["name"], dependency["head_requirement"])
    if any(term not in auth_body for term in expected_terms):
        _fail("user authorization does not name the exact package and dependency floor")


def read_issue_overlay(
    repo_root: pathlib.Path,
    repository: str,
    task_uid: str,
    base_oid: str,
    head_oid: str,
    *,
    client: Any = None,
) -> dict[str, Any]:
    """Issue-only validation for candidate CI; this never admits a task or Project item."""
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
        _fail("repository is malformed")
    client = client or _github_client(pathlib.Path(repo_root).resolve())
    issue = _issue_by_uid(client, repository, task_uid)
    comments = _comments(client, repository, int(issue["number"]))
    matches = [item for item in comments if isinstance(item.get("body"), str)
               and item["body"].replace("\r\n", "\n").startswith(OVERLAY_MARKER)]
    if len(matches) != 1:
        _fail("Task Issue must contain exactly one dependency-floor overlay")
    comment = matches[0]
    payload = _decode_overlay_comment(comment["body"])
    parsed = validate_overlay(payload, task_uid=task_uid, issue_number=int(issue["number"]),
                              base_oid=base_oid, head_oid=head_oid)
    _validate_referenced_evidence(repository, issue, comments, parsed)
    referenced_ids = {payload["authorization"]["comment_id"],
                      payload["reviews"]["repository_health"]["comment_id"],
                      payload["reviews"]["qa"]["comment_id"]}
    if int(comment["id"]) in referenced_ids:
        _fail("overlay cannot reference itself as authority evidence")
    parsed["overlay_comment_id"] = int(comment["id"])
    parsed["overlay_sha256"] = "sha256:" + hashlib.sha256(comment["body"].encode("utf-8")).hexdigest()
    parsed.pop("_overlay", None)
    return parsed


def has_issue_overlay(repo_root: pathlib.Path, repository: str, task_uid: str,
                      *, client: Any = None) -> bool:
    """Return whether the unique live Task Issue contains a first-activation marker."""
    root = pathlib.Path(repo_root).resolve()
    client = client or _github_client(root)
    issue = _issue_by_uid(client, repository, task_uid)
    comments = _comments(client, repository, int(issue["number"]))
    return any(isinstance(item.get("body"), str)
               and item["body"].replace("\r\n", "\n").startswith(OVERLAY_MARKER)
               for item in comments)


def read_project_activation(
    repo_root: pathlib.Path,
    repository: str,
    task_uid: str,
    base_oid: str,
    head_oid: str,
    *,
    mapping_path: pathlib.Path,
    client: Any = None,
    log_reader=None,
) -> dict[str, Any]:
    """Fresh local Project/Task and hosted run readback for activated authority."""
    root = pathlib.Path(repo_root).resolve()
    client = client or _github_client(root)
    overlay = read_project_overlay(root, repository, task_uid, base_oid, head_oid,
                                   mapping_path=mapping_path, client=client)
    comments = _comments(client, repository, overlay["issue_number"])
    overlay_comment = _unique_comment(comments, overlay["overlay_comment_id"], "overlay")
    overlay_payload = _decode_overlay_comment(str(overlay_comment["body"]))
    context = dict(overlay)
    context["_overlay"] = overlay_payload
    matches = [item for item in comments if isinstance(item.get("body"), str)
               and item["body"].replace("\r\n", "\n").startswith(ACTIVATION_MARKER + "\n")]
    if len(matches) != 1:
        _fail("Task Issue must contain exactly one activation evidence comment")
    activation_comment = matches[0]
    payload = _decode_activation_comment(str(activation_comment["body"]))
    run_id, run_attempt, recorded_proof = _validate_activation_payload(payload, context)
    proof = reconstruct_full_required_run(root, repository, task_uid, context, run_id,
                                          run_attempt, client=client, log_reader=log_reader)
    expected = _activation_payload(context, proof, run_id, run_attempt)
    if recorded_proof != proof or payload != expected:
        _fail("activation evidence does not match reconstructed full hosted required tier")
    activation_author = activation_comment.get("user")
    activation_login = activation_author.get("login") if isinstance(activation_author, dict) else None
    if not isinstance(activation_login, str) or not activation_login or activation_login.endswith("[bot]"):
        _fail("activation evidence author is unavailable or not a human account")
    if activation_comment.get("issue_url") != f"https://api.github.com/repos/{repository}/issues/{overlay['issue_number']}":
        _fail("activation evidence is attached to a different Task Issue")
    result = dict(overlay)
    result.update({
        "activated": True,
        "activation_comment_id": activation_comment.get("id"),
        "activation_sha256": "sha256:" + hashlib.sha256(str(activation_comment["body"]).encode("utf-8")).hexdigest(),
        "required_tier_proof": proof,
    })
    return result


def publish_activation(
    repo_root: pathlib.Path,
    repository: str,
    task_uid: str,
    base_oid: str,
    head_oid: str,
    run_id: int,
    run_attempt: int,
    *,
    mapping_path: pathlib.Path,
    client: Any = None,
    log_reader=None,
) -> dict[str, Any]:
    """Reconstruct the live authority/run and append one immutable activation record."""
    root = pathlib.Path(repo_root).resolve()
    client = client or _github_client(root)
    overlay = read_project_overlay(root, repository, task_uid, base_oid, head_oid,
                                   mapping_path=mapping_path, client=client)
    comments = _comments(client, repository, overlay["issue_number"])
    if any(isinstance(item.get("body"), str)
           and item["body"].replace("\r\n", "\n").startswith(ACTIVATION_MARKER + "\n")
           for item in comments):
        _fail("activation evidence already exists; immutable records cannot be replaced")
    overlay_comment = _unique_comment(comments, overlay["overlay_comment_id"], "overlay")
    context = dict(overlay)
    context["_overlay"] = _decode_overlay_comment(str(overlay_comment["body"]))
    proof = reconstruct_full_required_run(root, repository, task_uid, context, run_id,
                                          run_attempt, client=client, log_reader=log_reader)
    payload = _activation_payload(context, proof, run_id, run_attempt)
    body = ACTIVATION_MARKER + "\n" + _canonical(payload).decode("utf-8") + "\n"
    operator_login = _authenticated_login(client)
    posted = client.rest(
        "POST", f"repos/{repository}/issues/{overlay['issue_number']}/comments",
        {"body": body}, operation="first_activation_publish_evidence",
    )
    if not isinstance(posted, dict) or type(posted.get("id")) is not int or posted["id"] < 1:
        _fail("activation evidence write did not return a server comment identity")
    observed = client.rest("GET", f"repos/{repository}/issues/comments/{posted['id']}",
                           operation="first_activation_activation_readback")
    if (not isinstance(observed, dict) or observed.get("id") != posted["id"]
            or observed.get("issue_url") != f"https://api.github.com/repos/{repository}/issues/{overlay['issue_number']}"
            or observed.get("body") != body
            or (observed.get("user") or {}).get("login") != operator_login):
        _fail("activation evidence write readback differs from exact immutable bytes")
    result = read_project_activation(root, repository, task_uid, base_oid, head_oid,
                                     mapping_path=mapping_path, client=client, log_reader=log_reader)
    if result.get("activation_comment_id") != posted["id"]:
        _fail("activation evidence is not the unique exact live activation record")
    return result


def validate_workflow_run_provenance(
    run: Any,
    *,
    repository: str,
    overlay: dict[str, Any],
    run_id: int,
    run_attempt: int,
    event_name: str,
    workflow_ref: str,
    workflow_sha: str,
    allow_in_progress: bool = False,
) -> None:
    """Bind dispatch assertions to the server-read live workflow run."""
    if not isinstance(run, dict):
        _fail("live workflow run is malformed")
    status = run.get("status")
    expected_statuses = {"in_progress", "queued", "requested"} if allow_in_progress else {"completed"}
    expected_conclusion = None if allow_in_progress else "success"
    if (run.get("id") != run_id or run.get("run_attempt") != run_attempt
            or run.get("event") != event_name or event_name != "workflow_dispatch"
            or run.get("workflow_id") != overlay.get("workflow_id")
            or run.get("path") not in {
                overlay.get("workflow_path"),
                f"{overlay.get('workflow_path')}@{overlay.get('workflow_ref')}",
            }
            or run.get("head_branch") != str(overlay.get("workflow_ref", "")).removeprefix("refs/heads/")
            or run.get("head_sha") != overlay.get("workflow_sha")
            or (run.get("repository") or {}).get("full_name") != repository
            or workflow_ref != overlay.get("workflow_ref")
            or workflow_sha != overlay.get("workflow_sha")
            or status not in expected_statuses
            or (expected_conclusion is not None and run.get("conclusion") != expected_conclusion)):
        _fail("live workflow run differs from the exact reviewed dispatch provenance")


def _git(root: pathlib.Path, *args: str) -> str:
    try:
        result = subprocess.run(["git", "-C", str(root), *args], check=True,
                                capture_output=True, text=True, timeout=30)
    except (OSError, subprocess.SubprocessError) as exc:
        _fail(f"Git evidence read failed: {exc}")
    return result.stdout.strip()


def _trusted_base_worktree(repo_root: pathlib.Path, base_oid: str) -> pathlib.Path:
    raw = _git(repo_root, "worktree", "list", "--porcelain")
    rows: list[pathlib.Path] = []
    current_path: pathlib.Path | None = None
    current_head = ""
    for line in raw.splitlines() + [""]:
        if line.startswith("worktree "):
            current_path = pathlib.Path(line.removeprefix("worktree ")).resolve()
        elif line.startswith("HEAD "):
            current_head = line.removeprefix("HEAD ")
        elif not line:
            if current_path is not None and current_head == base_oid:
                rows.append(current_path)
            current_path = None
            current_head = ""
    if len(rows) != 1:
        _fail("trusted frozen-base worktree is not uniquely available for inventory derivation")
    status = _git(rows[0], "status", "--porcelain", "--untracked-files=all")
    if status:
        _fail("trusted frozen-base planner worktree is not clean")
    return rows[0]


def _changed_paths(repo_root: pathlib.Path, base_oid: str, head_oid: str) -> list[str]:
    raw = _git(repo_root, "diff", "--name-only", "-z", base_oid, head_oid)
    paths = [path for path in raw.split("\x00") if path]
    if len(set(paths)) != len(paths):
        _fail("frozen base/head diff has duplicate paths")
    return sorted(paths)


def reconstruct_full_required_run(
    repo_root: pathlib.Path,
    repository: str,
    task_uid: str,
    overlay: dict[str, Any],
    run_id: int,
    run_attempt: int,
    *,
    client: Any,
    log_reader=None,
) -> dict[str, Any]:
    """Rebuild full-tier command/unit expectations from trusted B and live run evidence."""
    if type(run_id) is not int or run_id < 1 or type(run_attempt) is not int or run_attempt < 1:
        _fail("workflow run identity is malformed")
    base_oid = str(overlay.get("base_oid") or "")
    head_oid = str(overlay.get("head_oid") or "")
    run = client.rest("GET", f"repos/{repository}/actions/runs/{run_id}",
                      operation="first_activation_workflow_run")
    workflow_ref = str(overlay.get("workflow_ref") or "")
    workflow_sha = str(overlay.get("workflow_sha") or "")
    validate_workflow_run_provenance(
        run, repository=repository, overlay=overlay, run_id=run_id,
        run_attempt=run_attempt, event_name="workflow_dispatch", workflow_ref=workflow_ref,
        workflow_sha=workflow_sha,
    )
    if _git(pathlib.Path(repo_root).resolve(), "rev-parse", "HEAD") != head_oid:
        _fail("local candidate worktree is not the exact frozen head")
    if _git(pathlib.Path(repo_root).resolve(), "status", "--porcelain", "--untracked-files=all"):
        _fail("local candidate worktree is not clean")
    job_path = f"repos/{repository}/actions/runs/{run_id}/attempts/{run_attempt}/jobs"
    jobs = _pages(client, job_path, "first_activation_workflow_jobs")
    gates = [job for job in jobs if job.get("name") == "required-gate"]
    if len(gates) != 1:
        _fail("live validation attempt lacks exactly one required-gate job")
    gate = gates[0]
    if gate.get("status") != "completed" or gate.get("conclusion") != "success":
        _fail("live required-gate job did not complete successfully")
    for job in jobs:
        if job is gate:
            continue
        if job.get("status") != "completed" or job.get("conclusion") not in {"success", "skipped"}:
            _fail("another applicable workflow job failed or is incomplete")
    steps = gate.get("steps")
    if not isinstance(steps, list) or not steps:
        _fail("required-gate step evidence is unavailable")
    for step in steps:
        if not isinstance(step, dict) or step.get("status") != "completed" or step.get("conclusion") not in {"success", "skipped"}:
            _fail("required-gate has a failed, skipped-unknown, or incomplete step")
    check_url = str(gate.get("check_run_url") or "")
    match = re.fullmatch(r"https://api\.github\.com/repos/[^/]+/[^/]+/check-runs/([1-9][0-9]*)", check_url)
    if not match:
        _fail("required-gate job lacks a canonical check-run identity")
    check_run_id = int(match.group(1))
    check_run = client.rest("GET", f"repos/{repository}/check-runs/{check_run_id}",
                            operation="first_activation_required_gate_check")
    check_app_id = ((check_run.get("app") or {}).get("id") if isinstance(check_run, dict) else None)
    if (not isinstance(check_run, dict) or check_run.get("name") != "required-gate"
            or check_run.get("head_sha") != head_oid or check_run.get("status") != "completed"
            or check_run.get("conclusion") != "success" or type(check_app_id) is not int or check_app_id < 1):
        _fail("server-observed required-gate check run is not exact successful evidence")
    try:
        default_branch = client.rest("GET", f"repos/{repository}", operation="first_activation_repository")
        branch = default_branch.get("default_branch") if isinstance(default_branch, dict) else None
    except Exception as exc:
        _fail(f"repository default branch read failed: {exc}")
    if not isinstance(branch, str) or not branch:
        _fail("repository default branch is unavailable")
    trusted_root = _trusted_base_worktree(pathlib.Path(repo_root).resolve(), base_oid)
    paths = _changed_paths(pathlib.Path(repo_root).resolve(), base_oid, head_oid)
    planner = trusted_root / "scripts/plan-rust-required-scope.py"
    planner_command = [sys.executable, str(planner), "--event-name", "workflow_dispatch",
                       "--run-mode", "full_escalation", "--base-ref", base_oid,
                       "--head-ref", head_oid, "--task-uid", task_uid,
                       "--scope-base-oid", base_oid]
    try:
        planner_output = subprocess.run(planner_command, cwd=trusted_root, check=True,
                                        capture_output=True, text=True, timeout=120).stdout
        planner_plan = dict(line.split("=", 1) for line in planner_output.splitlines() if "=" in line)
        spec = importlib.util.spec_from_file_location("first_activation_inventory", trusted_root / "scripts/pm/ci_required_inventory.py")
        if spec is None or spec.loader is None:
            _fail("trusted required-tier inventory helper is unavailable")
        inventory_module = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = inventory_module
        spec.loader.exec_module(inventory_module)
        environ_keys = ("GITHUB_REPOSITORY", "GITHUB_EVENT_NAME", "GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT")
        saved_env = {key: os.environ.get(key) for key in environ_keys}
        os.environ.update({"GITHUB_REPOSITORY": repository, "GITHUB_EVENT_NAME": "workflow_dispatch",
                           "GITHUB_RUN_ID": str(run_id), "GITHUB_RUN_ATTEMPT": str(run_attempt)})
        try:
            inventory = inventory_module.build_required_inventory(
                trusted_root, pathlib.Path(repo_root).resolve(), head_oid, planner_plan,
                repository=repository,
                workflow_ref=f"{repository}/.github/workflows/rust.yml@refs/heads/{branch}",
                planner_authority_oid=base_oid, event_name="workflow_dispatch",
                run_mode="full_escalation", changed_paths=paths, base_ref=base_oid,
                head_ref=head_oid, task_uid=task_uid, scope_base_oid=base_oid,
                run_id=run_id, run_attempt=run_attempt, check_app_id=check_app_id,
                check_run_id=check_run_id,
            )
        finally:
            for key, value in saved_env.items():
                if value is None:
                    os.environ.pop(key, None)
                else:
                    os.environ[key] = value
    except OverlayError:
        raise
    except Exception as exc:
        _fail(f"trusted full-tier inventory could not be reconstructed: {exc}")
    if inventory.get("planner_output", {}).get("scope") != "full":
        _fail("trusted full-tier planner did not produce full scope")
    if log_reader is None:
        try:
            log_result = subprocess.run(
                ["gh", "run", "view", str(run_id), "--repo", repository,
                 "--attempt", str(run_attempt), "--log"],
                check=True, capture_output=True, text=True, timeout=180,
            )
        except (OSError, subprocess.SubprocessError) as exc:
            _fail(f"live workflow logs are unavailable: {exc}")
        logs = log_result.stdout
    else:
        logs = log_reader(run_id, run_attempt)
    if not isinstance(logs, str) or not logs.strip():
        _fail("live workflow log payload is empty")
    obligations: set[str] = set()
    for unit in inventory.get("unit_specs") or []:
        if not isinstance(unit, dict) or not isinstance(unit.get("obligation_set"), list):
            _fail("trusted full-tier command/unit inventory is malformed")
        obligations.update(str(item) for item in unit["obligation_set"] if isinstance(item, str) and item)
    missing = sorted(obligation for obligation in obligations if obligation not in logs)
    if not obligations or missing:
        _fail("live workflow logs do not prove every trusted full-tier command/unit")
    issuer = inventory.get("planner_inventory_issuer") or {}
    inventory_digest = issuer.get("inventory_digest")
    if not isinstance(inventory_digest, str) or not re.fullmatch(r"sha256:[0-9a-f]{64}", inventory_digest):
        _fail("trusted full-tier inventory digest is missing")
    return {
        "schema": "oasis7-cargo-dependency-floor-required-tier-proof/v1",
        "workflow_run_id": run_id,
        "run_attempt": run_attempt,
        "required_gate_job_id": gate.get("id"),
        "required_gate_check_run_id": check_run_id,
        "required_gate_check_app_id": check_app_id,
        "inventory_digest": inventory_digest,
        "obligation_count": len(obligations),
        "workflow_logs_sha256": "sha256:" + hashlib.sha256(logs.encode("utf-8")).hexdigest(),
    }


def _load_json(path: pathlib.Path, label: str) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=_unique_object)
    except (OSError, json.JSONDecodeError, ValueError) as exc:
        _fail(f"{label} is unavailable or malformed: {exc}")
    if not isinstance(value, dict):
        _fail(f"{label} must be an object")
    return value


def _read_project_task_binding(
    repo_root: pathlib.Path,
    repository: str,
    task_uid: str,
    *,
    mapping_path: pathlib.Path,
    client: Any = None,
) -> dict[str, Any]:
    """Fresh-read local Project mapping, immutable snapshot, live item, and Task Issue."""
    root = pathlib.Path(repo_root).resolve()
    mapping = _load_json(pathlib.Path(mapping_path), "Project task mapping")
    task = (mapping.get("tasks") or {}).get(task_uid)
    project = mapping.get("project")
    if not isinstance(task, dict) or not isinstance(project, dict):
        _fail("Task UID is not uniquely mapped to a Project")
    if task.get("repository") != repository or type(task.get("issue_number")) is not int:
        _fail("local Task mapping identity differs from requested repository")
    if not isinstance(task.get("project_item_id"), str) or not task["project_item_id"]:
        _fail("Task mapping lacks Project item locator")
    worktree = pathlib.Path(str(task.get("canonical_worktree") or "")).resolve()
    snapshot_path = worktree / ".pm/scratch" / task_uid / "bootstrap-task-snapshot.json"
    snapshot = _load_json(snapshot_path, "immutable bootstrap snapshot")
    snapshot_module_path = root / "scripts/pm/bootstrap-task-snapshot.py"
    spec = importlib.util.spec_from_file_location("first_activation_snapshot", snapshot_module_path)
    if spec is None or spec.loader is None:
        _fail("bootstrap snapshot validator is unavailable")
    snapshot_module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = snapshot_module
    spec.loader.exec_module(snapshot_module)
    if snapshot.get("digest") != snapshot_module.digest(snapshot):
        _fail("immutable bootstrap snapshot digest is invalid")
    client = client or _github_client(root)
    query = """
    query($item: ID!) {
      node(id: $item) {
        ... on ProjectV2Item {
          id
          project { id number owner { ... on Organization { login } ... on User { login } } }
          content { ... on Issue { number url repository { nameWithOwner } body } }
        }
      }
    }
    """
    data = client.graphql(query, {"item": task["project_item_id"]}, operation="first_activation_project_readback")
    node = data.get("node") if isinstance(data, dict) else None
    owner = ((node or {}).get("project") or {}).get("owner") or {}
    content = (node or {}).get("content") or {}
    if (not isinstance(node, dict) or node.get("id") != task["project_item_id"]
            or (node.get("project") or {}).get("id") != project.get("id")
            or (node.get("project") or {}).get("number") != project.get("number")
            or owner.get("login") != project.get("owner")
            or content.get("number") != task["issue_number"]
            or content.get("repository", {}).get("nameWithOwner") != repository
            or task_uid not in _body_uid(str(content.get("body") or ""))):
        _fail("live Project item does not bind exactly to Task Issue")
    live_issue = client.rest("GET", f"repos/{repository}/issues/{task['issue_number']}",
                             operation="first_activation_task_issue_readback")
    if (not isinstance(live_issue, dict) or live_issue.get("number") != task["issue_number"]
            or live_issue.get("state") != "open"
            or f"https://api.github.com/repos/{repository}/issues/{task['issue_number']}" != live_issue.get("url")
            or live_issue.get("body") != content.get("body")):
        _fail("live Project content differs from the live Task Issue")
    issue_body = str(live_issue.get("body") or "")
    if _body_uid(issue_body) != [task_uid]:
        _fail("live Task Issue UID is ambiguous")
    snapshot_task = snapshot.get("task") or {}
    if _raw_primary_from_issue(issue_body) != {
        "present": "primary_package" in snapshot_task,
        "value": snapshot_task.get("primary_package"),
    }:
        _fail("live Task Issue primary_package differs from immutable snapshot")
    task_helper_path = root / "scripts/pm/github-project-task.py"
    helper_spec = importlib.util.spec_from_file_location("first_activation_task_helper", task_helper_path)
    if helper_spec is None or helper_spec.loader is None:
        _fail("trusted Task Issue parser is unavailable")
    task_helper = importlib.util.module_from_spec(helper_spec)
    sys.modules[helper_spec.name] = task_helper
    helper_spec.loader.exec_module(task_helper)
    live_fields = task_helper.issue_task_fields(issue_body)
    request_snapshot = snapshot.get("request") or {}
    if live_fields.get("acceptance") != snapshot_task.get("acceptance"):
        _fail("live Task Issue acceptance differs from immutable snapshot")
    if task_helper.normalize_issue_title(str(live_issue.get("title") or "")) != request_snapshot.get("identity"):
        _fail("live Task Issue request title differs from immutable snapshot")
    return {"root": root, "mapping": mapping, "task": task, "project": project,
            "worktree": worktree, "snapshot": snapshot, "snapshot_task": snapshot_task,
            "request_snapshot": request_snapshot, "issue": live_issue, "client": client}


FIRST_ACTIVATION_REVIEWED_PATHS = {
    "doc/engineering/workflow/source-of-truth.md",
    ".github/workflows/rust.yml",
    "scripts/ci-tests.sh",
    "scripts/ci-required-scope-audit-contract.test.sh",
    "scripts/pm/check-cargo-package-scope",
    "scripts/pm/check-cargo-package-scope.test.py",
    "scripts/pm/first_activation.py",
    "scripts/pm/first_activation.test.py",
    "scripts/prepare-task-pr.sh",
    "scripts/prepare-task-pr.test.sh",
}


def _git_blob_sha256(root: pathlib.Path, oid: str, path: str) -> str:
    try:
        result = subprocess.run(["git", "-C", str(root), "show", f"{oid}:{path}"],
                                check=True, capture_output=True, timeout=30)
    except subprocess.CalledProcessError:
        return "sha256:" + hashlib.sha256(b"").hexdigest()
    except (OSError, subprocess.SubprocessError) as exc:
        _fail(f"frozen workflow path read failed: {exc}")
    return "sha256:" + hashlib.sha256(result.stdout).hexdigest()


def _validate_payload_snapshot(payload: dict[str, Any], binding: dict[str, Any],
                               base_oid: str, head_oid: str) -> dict[str, Any]:
    snapshot = binding["snapshot"]
    snapshot_task = binding["snapshot_task"]
    request_snapshot = binding["request_snapshot"]
    issue_number = binding["task"]["issue_number"]
    raw = {"present": "primary_package" in snapshot_task,
           "value": snapshot_task.get("primary_package")}
    parsed = validate_overlay(payload, task_uid=str(snapshot_task.get("uid") or ""),
                              issue_number=issue_number, base_oid=base_oid,
                              head_oid=head_oid, expected_raw_primary=raw)
    expected_snapshot_sha = "sha256:" + str(snapshot.get("digest", "")).removeprefix("sha256:")
    request_sha = "sha256:" + hashlib.sha256(_canonical(request_snapshot.get("identity")).encode("utf-8")).hexdigest()
    acceptance_sha = "sha256:" + hashlib.sha256(_canonical(request_snapshot.get("acceptance")).encode("utf-8")).hexdigest()
    if (payload["bootstrap_epoch"] != snapshot_task.get("bootstrap_epoch")
            or payload["snapshot_sha256"] != expected_snapshot_sha
            or payload["request_sha256"] != request_sha
            or payload["acceptance_sha256"] != acceptance_sha):
        _fail("overlay differs from immutable Task epoch, snapshot, request, or acceptance")
    actual_head = _git(binding["root"], "rev-parse", "HEAD")
    if actual_head != head_oid or _git(binding["root"], "status", "--porcelain", "--untracked-files=all"):
        _fail("overlay must be published from a clean worktree at the frozen head")
    branch = _git(binding["root"], "symbolic-ref", "--quiet", "--short", "HEAD")
    if payload["workflow"]["ref"] != "refs/heads/" + branch:
        _fail("overlay workflow ref differs from the frozen candidate branch")
    changed = _changed_paths(binding["root"], base_oid, head_oid)
    workflow_rows = payload["workflow"]["change_paths"]
    workflow_paths = [row["path"] for row in workflow_rows]
    if workflow_paths != sorted(workflow_paths) or len(workflow_paths) != len(set(workflow_paths)):
        _fail("overlay workflow path set is not uniquely sorted")
    if any(path not in FIRST_ACTIVATION_REVIEWED_PATHS for path in workflow_paths):
        _fail("overlay workflow path set contains an unapproved first-activation path")
    actual_nonbusiness = [path for path in changed if path in FIRST_ACTIVATION_REVIEWED_PATHS]
    if actual_nonbusiness != workflow_paths:
        _fail("overlay workflow path set differs from actual reviewed first-activation changes")
    business_paths = sorted(set(changed) - set(workflow_paths))
    manifests = [path for path in business_paths if path.endswith("/Cargo.toml") and path != "Cargo.toml"]
    if business_paths != sorted(["Cargo.lock", *manifests]) or len(manifests) != 1:
        _fail("first-activation source changes are not exactly Cargo.lock and one package manifest")
    for row in workflow_rows:
        for commit, field in ((base_oid, "base_sha256"), (head_oid, "head_sha256")):
            if _git_blob_sha256(binding["root"], commit, row["path"]) != row[field]:
                _fail(f"overlay workflow path digest differs from frozen {field}: {row['path']}")
    workflow = payload["workflow"]
    if workflow["file_sha256"] != _git_blob_sha256(binding["root"], head_oid, workflow["path"]):
        _fail("overlay workflow file digest differs from frozen candidate bytes")
    live_workflow = binding["client"].rest(
        "GET", f"repos/{binding['task']['repository']}/actions/workflows/{workflow['id']}",
        operation="first_activation_workflow_identity",
    )
    if (not isinstance(live_workflow, dict) or live_workflow.get("id") != workflow["id"]
            or live_workflow.get("path") != workflow["path"]):
        _fail("overlay workflow ID/path differs from live repository workflow")
    return parsed


def _inside_repo_file(root: pathlib.Path, value: str, label: str) -> pathlib.Path:
    if not isinstance(value, str) or not value:
        _fail(f"{label} path is missing")
    candidate = pathlib.Path(value)
    path = (candidate if candidate.is_absolute() else root / candidate).resolve()
    try:
        path.relative_to(root.resolve())
    except ValueError:
        _fail(f"{label} path escapes the canonical worktree")
    if path.is_symlink() or not path.is_file():
        _fail(f"{label} path is unavailable or symlinked")
    return path


def _validate_local_review_artifacts(repo_root: pathlib.Path, task_uid: str,
                                     overlay: dict[str, Any], comments: list[dict[str, Any]]) -> None:
    """Revalidate the frozen plan, packet, return and ledger behind typed role records."""
    root = pathlib.Path(repo_root).resolve()
    task_scratch = root / ".pm/scratch" / task_uid
    plan_dir = task_scratch / "review-plans"
    matching: list[tuple[pathlib.Path, dict[str, Any], bytes]] = []
    if not plan_dir.is_dir() or plan_dir.is_symlink():
        _fail("frozen professional review plan is unavailable")
    for path in sorted(plan_dir.glob("*.json")):
        if path.is_symlink() or not path.is_file():
            continue
        try:
            raw = path.read_bytes()
            plan = json.loads(raw, object_pairs_hook=_unique_object)
        except (OSError, json.JSONDecodeError, ValueError):
            continue
        if (isinstance(plan, dict) and plan.get("schema") == "oasis7-review-plan/v2"
                and plan.get("task_uid") == task_uid and plan.get("frozen_head") == overlay["head_oid"]
                and plan.get("comparison_oid") == overlay["base_oid"]):
            matching.append((path, plan, raw))
    if len(matching) != 1:
        _fail("frozen professional review plan is not uniquely bound to this base/head")
    plan_path, plan, plan_raw = matching[0]
    _inside_repo_file(root, str(plan.get("batch_path") or ""), "review batch")
    expected_slices = plan.get("expected_slices")
    preflight = plan.get("preflight")
    if not isinstance(preflight, dict):
        _fail("review plan lacks its preflight ledger binding")
    ledger_path = _inside_repo_file(root, str(preflight.get("ledger_path") or ""), "review slice ledger")
    if "scripts/pm/review-plan.py" in _changed_paths(root, overlay["base_oid"], overlay["head_oid"]):
        _fail("review-plan validator changed inside the first-activation candidate")
    review_plan_spec = importlib.util.spec_from_file_location("first_activation_review_plan", root / "scripts/pm/review-plan.py")
    if review_plan_spec is None or review_plan_spec.loader is None:
        _fail("trusted review-plan validator is unavailable")
    review_plan_module = importlib.util.module_from_spec(review_plan_spec)
    sys.modules[review_plan_spec.name] = review_plan_module
    review_plan_spec.loader.exec_module(review_plan_module)
    try:
        checked_plan, plan_digest, collection = review_plan_module.validate_prior_plan(
            root, plan_path, task_uid,
        )
    except Exception as exc:
        _fail(f"professional review plan, batch, ledger, or returns are invalid: {exc}")
    if checked_plan != plan:
        _fail("professional review plan changed during local readback")
    packet_refs = plan.get("packet_refs")
    if not isinstance(packet_refs, list):
        _fail("review plan packet references are malformed")
    ledger_rows = []
    try:
        for line in ledger_path.read_text(encoding="utf-8").splitlines():
            if line.strip():
                row = json.loads(line, object_pairs_hook=_unique_object)
                if not isinstance(row, dict):
                    _fail("review ledger contains a non-object row")
                ledger_rows.append(row)
    except (OSError, UnicodeDecodeError, json.JSONDecodeError, ValueError) as exc:
        _fail(f"review ledger cannot be read canonically: {exc}")
    for role, plan_role in (("repository_health", "repository_health_engineer"),
                            ("qa", "qa_engineer")):
        review_row = overlay["reviews"][role]
        comment = _unique_comment(comments, review_row["comment_id"], f"{role} review")
        body = str(comment["body"]).replace("\r\n", "\n")
        try:
            review = json.loads(body[len(REVIEW_MARKER) + 1:], object_pairs_hook=_unique_object)
        except (ValueError, json.JSONDecodeError) as exc:
            _fail(f"{role} typed review record cannot be decoded: {exc}")
        slice_id = review.get("slice_id") if isinstance(review, dict) else None
        if not isinstance(slice_id, str) or not slice_id:
            _fail(f"{role} review slice identity is unavailable")
        expected_row = {"role": plan_role, "slice_id": slice_id}
        if expected_row not in expected_slices:
            _fail(f"{role} review slice is absent from the frozen role plan")
        packet_candidates = [item for item in packet_refs
                             if isinstance(item, dict) and item.get("role") == plan_role
                             and item.get("slice_id") == slice_id]
        ledger_matches = [item for item in ledger_rows
                          if item.get("role") == plan_role and item.get("slice_id") == slice_id]
        if len(packet_candidates) != 1 or len(ledger_matches) != 1:
            _fail(f"{role} review packet/return is not uniquely bound by the frozen plan")
        packet_path = _inside_repo_file(root, str(packet_candidates[0].get("packet_ref") or ""),
                                        f"{role} admitted packet")
        packet_raw = packet_path.read_bytes()
        if "sha256:" + hashlib.sha256(packet_raw).hexdigest() != review_row.get("admitted_packet_sha256"):
            _fail(f"{role} admitted packet digest differs from typed review record")
        packet = _load_json(packet_path, f"{role} admitted packet")
        slice_contract = packet.get("slice")
        identity = packet.get("identity")
        if (not isinstance(slice_contract, dict) or not isinstance(identity, dict)
                or slice_contract.get("role") != plan_role or slice_contract.get("slice_id") != slice_id
                or identity.get("task_uid") != task_uid or identity.get("head") != overlay["head_oid"]):
            _fail(f"{role} admitted packet identity differs from typed review record")
        write_scope = str(slice_contract.get("write_scope") or "")
        scope_lower = write_scope.lower()
        if (slice_contract.get("slice_type") != "professional_review"
                or not any(term in write_scope.lower() for term in
                           ("read-only", "read only", "review-only", "review only", "no code changes"))
                or re.search(r"\b(?:will|may|can|must|should)\s+(?:implement|modify|edit|write)\b",
                             scope_lower)):
            _fail(f"{role} review write scope is missing or includes implementation paths")
        returned_refs = ledger_matches[0].get("artifacts")
        if not isinstance(returned_refs, list) or len(returned_refs) != 1:
            _fail(f"{role} review ledger does not name exactly one return artifact")
        return_path = review_plan_module.resolve_collected_artifact(root, ledger_path, returned_refs[0])
        return_raw = return_path.read_bytes()
        if "sha256:" + hashlib.sha256(return_raw).hexdigest() != review_row.get("return_sha256"):
            _fail(f"{role} structured return digest differs from typed review record")
        returned = _load_json(return_path, f"{role} structured return")
        if (returned.get("schema") != "oasis7-review-return/v1"
                or returned.get("task_uid") != task_uid or returned.get("role") != plan_role
                or returned.get("slice_id") != slice_id or returned.get("head") != overlay["head_oid"]
                or returned.get("status") != "completed" or returned.get("disposition") != "no_findings"
                or returned.get("findings") != []):
            _fail(f"{role} structured return is not a completed no-findings review")
        if plan_digest != review_row.get("review_plan_sha256"):
            _fail(f"{role} review plan digest differs from typed review record")
    packet_dir = task_scratch / "slice-packets"
    implementation_slice_ids: set[str] = set()
    if not packet_dir.is_dir() or packet_dir.is_symlink():
        _fail("pre-dispatch Task slice packets are unavailable")
    for path in sorted(packet_dir.glob("*.json")):
        if path.is_symlink() or not path.is_file():
            _fail("pre-dispatch Task slice packet is symlinked or not a regular file")
        packet = _load_json(path, "pre-dispatch Task slice packet")
        packet_identity = packet.get("identity")
        slice_contract = packet.get("slice")
        if not isinstance(packet_identity, dict) or not isinstance(slice_contract, dict):
            _fail("pre-dispatch Task slice packet identity is malformed")
        if packet_identity.get("task_uid") != task_uid or packet_identity.get("head") != overlay["head_oid"]:
            _fail("pre-dispatch Task slice packet is not bound to the frozen Task head")
        slice_id = slice_contract.get("slice_id")
        if not isinstance(slice_id, str) or not slice_id:
            _fail("pre-dispatch Task slice ID is missing")
        if slice_contract.get("slice_type") != "professional_review":
            implementation_slice_ids.add(slice_id)
    for role in ("repository_health", "qa"):
        review_comment = _unique_comment(
            comments, overlay["reviews"][role]["comment_id"], f"{role} review",
        )
        review_body = str(review_comment["body"]).replace("\r\n", "\n")
        review = json.loads(review_body[len(REVIEW_MARKER) + 1:], object_pairs_hook=_unique_object)
        if review["slice_id"] in implementation_slice_ids:
            _fail(f"{role} review slice ID collides with an implementation slice")


def publish_overlay(
    repo_root: pathlib.Path,
    repository: str,
    task_uid: str,
    base_oid: str,
    head_oid: str,
    payload: Any,
    *,
    mapping_path: pathlib.Path,
    client: Any = None,
) -> dict[str, Any]:
    """Publish one immutable overlay after independent live Project/Task readback."""
    binding = _read_project_task_binding(repo_root, repository, task_uid,
                                        mapping_path=mapping_path, client=client)
    if not isinstance(payload, dict):
        _fail("overlay publication payload must be an object")
    parsed = _validate_payload_snapshot(payload, binding, base_oid, head_oid)
    comments = _comments(binding["client"], repository, binding["task"]["issue_number"])
    if any(isinstance(item.get("body"), str)
           and item["body"].replace("\r\n", "\n").startswith(OVERLAY_MARKER + "\n")
           for item in comments):
        _fail("dependency-floor overlay already exists; immutable records cannot be replaced")
    parsed["_overlay"] = payload
    _validate_referenced_evidence(repository, binding["issue"], comments, parsed)
    body = canonical_overlay_body(payload)
    operator_login = _authenticated_login(binding["client"])
    posted = binding["client"].rest(
        "POST", f"repos/{repository}/issues/{binding['task']['issue_number']}/comments",
        {"body": body}, operation="first_activation_publish_overlay",
    )
    if not isinstance(posted, dict) or type(posted.get("id")) is not int or posted["id"] < 1:
        _fail("overlay write did not return a server comment identity")
    observed = binding["client"].rest("GET", f"repos/{repository}/issues/comments/{posted['id']}",
                                     operation="first_activation_overlay_readback")
    if (not isinstance(observed, dict) or observed.get("id") != posted["id"]
            or observed.get("issue_url") != f"https://api.github.com/repos/{repository}/issues/{binding['task']['issue_number']}"
            or observed.get("body") != body
            or (observed.get("user") or {}).get("login") != operator_login):
        _fail("overlay write readback differs from exact immutable bytes")
    return read_project_overlay(pathlib.Path(repo_root), repository, task_uid,
                                base_oid, head_oid, mapping_path=mapping_path,
                                client=binding["client"])


def _decode_activation_comment(body: str) -> dict[str, Any]:
    normalized = body.replace("\r\n", "\n")
    if not normalized.startswith(ACTIVATION_MARKER + "\n"):
        _fail("activation comment marker is malformed")
    value = json.loads(normalized[len(ACTIVATION_MARKER) + 1:], object_pairs_hook=_unique_object)
    if not isinstance(value, dict) or ACTIVATION_MARKER + "\n" + _canonical(value).decode("utf-8") + "\n" != normalized:
        _fail("activation comment is not canonical immutable JSON")
    return value


def _activation_payload(overlay: dict[str, Any], proof: dict[str, Any],
                        run_id: int, run_attempt: int) -> dict[str, Any]:
    source = overlay["_overlay"]
    return {
        "schema": ACTIVATION_SCHEMA,
        "task_uid": overlay["task_uid"],
        "issue_number": overlay["issue_number"],
        "bootstrap_epoch": source["bootstrap_epoch"],
        "snapshot_sha256": source["snapshot_sha256"],
        "request_sha256": source["request_sha256"],
        "acceptance_sha256": source["acceptance_sha256"],
        "overlay_comment_id": overlay["overlay_comment_id"],
        "overlay_sha256": overlay["overlay_sha256"],
        "authorization": source["authorization"],
        "reviews": source["reviews"],
        "base_oid": overlay["base_oid"],
        "head_oid": overlay["head_oid"],
        "workflow": {
            "id": overlay["workflow_id"], "path": overlay["workflow_path"],
            "ref": overlay["workflow_ref"], "sha": overlay["workflow_sha"],
            "file_sha256": overlay["workflow_file_sha256"],
            "event": "workflow_dispatch", "run_id": run_id, "run_attempt": run_attempt,
        },
        "required_tier_proof": proof,
    }


def _validate_activation_payload(payload: Any, overlay: dict[str, Any]) -> tuple[int, int, dict[str, Any]]:
    source = overlay["_overlay"]
    top = _object(payload, {
        "schema", "task_uid", "issue_number", "bootstrap_epoch", "snapshot_sha256",
        "request_sha256", "acceptance_sha256", "overlay_comment_id", "overlay_sha256",
        "authorization", "reviews", "base_oid", "head_oid", "workflow", "required_tier_proof",
    }, "activation evidence")
    workflow = _object(top["workflow"], {
        "id", "path", "ref", "sha", "file_sha256", "event", "run_id", "run_attempt",
    }, "activation workflow")
    if (top["schema"] != ACTIVATION_SCHEMA or top["task_uid"] != overlay["task_uid"]
            or top["issue_number"] != overlay["issue_number"]
            or top["bootstrap_epoch"] != source["bootstrap_epoch"]
            or top["snapshot_sha256"] != source["snapshot_sha256"]
            or top["request_sha256"] != source["request_sha256"]
            or top["acceptance_sha256"] != source["acceptance_sha256"]
            or top["overlay_comment_id"] != overlay["overlay_comment_id"]
            or top["overlay_sha256"] != overlay["overlay_sha256"]
            or top["authorization"] != source["authorization"]
            or top["reviews"] != source["reviews"]
            or top["base_oid"] != overlay["base_oid"] or top["head_oid"] != overlay["head_oid"]):
        _fail("activation evidence differs from fresh Project, Task, overlay, or review authority")
    if (workflow["id"] != overlay["workflow_id"] or workflow["path"] != overlay["workflow_path"]
            or workflow["ref"] != overlay["workflow_ref"] or workflow["sha"] != overlay["workflow_sha"]
            or workflow["file_sha256"] != overlay["workflow_file_sha256"]
            or workflow["event"] != "workflow_dispatch"):
        _fail("activation evidence differs from reviewed workflow identity")
    run_id, run_attempt = workflow["run_id"], workflow["run_attempt"]
    if type(run_id) is not int or run_id < 1 or type(run_attempt) is not int or run_attempt < 1:
        _fail("activation workflow run identity is invalid")
    proof = _object(top["required_tier_proof"], {
        "schema", "workflow_run_id", "run_attempt", "required_gate_job_id",
        "required_gate_check_run_id", "required_gate_check_app_id", "inventory_digest",
        "obligation_count", "workflow_logs_sha256",
    }, "required-tier proof")
    if (proof["schema"] != "oasis7-cargo-dependency-floor-required-tier-proof/v1"
            or proof["workflow_run_id"] != run_id or proof["run_attempt"] != run_attempt
            or type(proof["required_gate_job_id"]) is not int or proof["required_gate_job_id"] < 1
            or type(proof["required_gate_check_run_id"]) is not int or proof["required_gate_check_run_id"] < 1
            or type(proof["required_gate_check_app_id"]) is not int or proof["required_gate_check_app_id"] < 1
            or type(proof["obligation_count"]) is not int or proof["obligation_count"] < 1):
        _fail("activation required-tier proof is malformed")
    _digest(proof["inventory_digest"], "required-tier inventory digest")
    _digest(proof["workflow_logs_sha256"], "required-tier log digest")
    return run_id, run_attempt, proof


def read_project_overlay(
    repo_root: pathlib.Path,
    repository: str,
    task_uid: str,
    base_oid: str,
    head_oid: str,
    *,
    mapping_path: pathlib.Path,
    client: Any = None,
) -> dict[str, Any]:
    """Fresh local Project + immutable snapshot readback; does not validate a hosted run."""
    root = pathlib.Path(repo_root).resolve()
    mapping = _load_json(pathlib.Path(mapping_path), "Project task mapping")
    task = (mapping.get("tasks") or {}).get(task_uid)
    project = mapping.get("project")
    if not isinstance(task, dict) or not isinstance(project, dict):
        _fail("Task UID is not uniquely mapped to a Project")
    if task.get("repository") != repository or type(task.get("issue_number")) is not int:
        _fail("local Task mapping identity differs from requested repository")
    if not isinstance(task.get("project_item_id"), str) or not task["project_item_id"]:
        _fail("Task mapping lacks Project item locator")
    worktree = pathlib.Path(str(task.get("canonical_worktree") or "")).resolve()
    snapshot_path = worktree / ".pm/scratch" / task_uid / "bootstrap-task-snapshot.json"
    snapshot = _load_json(snapshot_path, "immutable bootstrap snapshot")
    snapshot_module_path = root / "scripts/pm/bootstrap-task-snapshot.py"
    spec = importlib.util.spec_from_file_location("first_activation_snapshot", snapshot_module_path)
    if spec is None or spec.loader is None:
        _fail("bootstrap snapshot validator is unavailable")
    snapshot_module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = snapshot_module
    spec.loader.exec_module(snapshot_module)
    if snapshot.get("digest") != snapshot_module.digest(snapshot):
        _fail("immutable bootstrap snapshot digest is invalid")
    client = client or _github_client(root)
    query = """
    query($item: ID!) {
      node(id: $item) {
        ... on ProjectV2Item {
          id
          project { id number owner { ... on Organization { login } ... on User { login } } }
          content { ... on Issue { number url repository { nameWithOwner } body } }
        }
      }
    }
    """
    data = client.graphql(query, {"item": task["project_item_id"]}, operation="first_activation_project_readback")
    node = data.get("node") if isinstance(data, dict) else None
    owner = ((node or {}).get("project") or {}).get("owner") or {}
    content = (node or {}).get("content") or {}
    if (not isinstance(node, dict) or node.get("id") != task["project_item_id"]
            or (node.get("project") or {}).get("id") != project.get("id")
            or (node.get("project") or {}).get("number") != project.get("number")
            or owner.get("login") != project.get("owner")
            or content.get("number") != task["issue_number"]
            or content.get("repository", {}).get("nameWithOwner") != repository
            or task_uid not in _body_uid(str(content.get("body") or ""))):
        _fail("live Project item does not bind exactly to Task Issue")
    live_issue = client.rest("GET", f"repos/{repository}/issues/{task['issue_number']}",
                             operation="first_activation_task_issue_readback")
    if (not isinstance(live_issue, dict) or live_issue.get("number") != task["issue_number"]
            or live_issue.get("state") != "open"
            or f"https://api.github.com/repos/{repository}/issues/{task['issue_number']}" != live_issue.get("url")
            or live_issue.get("body") != content.get("body")):
        _fail("live Project content differs from the live Task Issue")
    issue_body = str(live_issue.get("body") or "")
    if _body_uid(issue_body) != [task_uid]:
        _fail("live Task Issue UID is ambiguous")
    snapshot_task = snapshot.get("task") or {}
    if _raw_primary_from_issue(issue_body) != {
        "present": "primary_package" in snapshot_task,
        "value": snapshot_task.get("primary_package"),
    }:
        _fail("live Task Issue primary_package differs from immutable snapshot")
    task_helper_path = root / "scripts/pm/github-project-task.py"
    helper_spec = importlib.util.spec_from_file_location("first_activation_task_helper", task_helper_path)
    if helper_spec is None or helper_spec.loader is None:
        _fail("trusted Task Issue parser is unavailable")
    task_helper = importlib.util.module_from_spec(helper_spec)
    sys.modules[helper_spec.name] = task_helper
    helper_spec.loader.exec_module(task_helper)
    live_fields = task_helper.issue_task_fields(issue_body)
    request_snapshot = snapshot.get("request") or {}
    if live_fields.get("acceptance") != snapshot_task.get("acceptance"):
        _fail("live Task Issue acceptance differs from immutable snapshot")
    if task_helper.normalize_issue_title(str(live_issue.get("title") or "")) != request_snapshot.get("identity"):
        _fail("live Task Issue request title differs from immutable snapshot")
    parsed = read_issue_overlay(root, repository, task_uid, base_oid, head_oid, client=client)
    # `read_issue_overlay` intentionally emits a compact producer result; recover the exact
    # typed payload from the selected immutable comment for snapshot comparison.
    comment = _unique_comment(_comments(client, repository, task["issue_number"]), parsed["overlay_comment_id"], "overlay")
    overlay_payload = _decode_overlay_comment(str(comment["body"]))
    expected_raw = {"present": "primary_package" in (snapshot.get("task") or {}),
                    "value": (snapshot.get("task") or {}).get("primary_package")}
    validate_overlay(overlay_payload, task_uid=task_uid, issue_number=task["issue_number"],
                     base_oid=base_oid, head_oid=head_oid, expected_raw_primary=expected_raw)
    expected_snapshot_sha = "sha256:" + str(snapshot.get("digest", "")).removeprefix("sha256:")
    if overlay_payload["snapshot_sha256"] != expected_snapshot_sha:
        _fail("overlay does not bind the immutable bootstrap snapshot")
    if overlay_payload["bootstrap_epoch"] != (snapshot.get("task") or {}).get("bootstrap_epoch"):
        _fail("overlay bootstrap epoch differs from immutable snapshot")
    request_sha = "sha256:" + hashlib.sha256(_canonical(request_snapshot.get("identity")).decode("utf-8").encode("utf-8")).hexdigest()
    acceptance_sha = "sha256:" + hashlib.sha256(_canonical(request_snapshot.get("acceptance")).decode("utf-8").encode("utf-8")).hexdigest()
    if overlay_payload["request_sha256"] != request_sha or overlay_payload["acceptance_sha256"] != acceptance_sha:
        _fail("overlay request or acceptance digest differs from immutable snapshot")
    review_comments = _comments(client, repository, task["issue_number"])
    _validate_local_review_artifacts(worktree, task_uid, overlay_payload, review_comments)
    parsed.pop("_overlay", None)
    parsed["project_item_id"] = task["project_item_id"]
    return parsed


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=pathlib.Path, required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--task-uid", required=True)
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", required=True)
    parser.add_argument("--mapping", type=pathlib.Path)
    actions = parser.add_mutually_exclusive_group(required=True)
    actions.add_argument("--verify-activation", action="store_true")
    actions.add_argument("--publish-activation", action="store_true")
    actions.add_argument("--publish-overlay", type=pathlib.Path)
    parser.add_argument("--run-id", type=int)
    parser.add_argument("--run-attempt", type=int)
    args = parser.parse_args(argv)
    root = args.repo_root.resolve()
    mapping_path = args.mapping or (root / ".pm/github-project-sync/tasks.json")
    try:
        if args.verify_activation:
            result = read_project_activation(root, args.repository, args.task_uid,
                                             args.base, args.head,
                                             mapping_path=mapping_path)
        elif args.publish_activation:
            if args.run_id is None or args.run_attempt is None:
                parser.error("--publish-activation requires --run-id and --run-attempt")
            result = publish_activation(root, args.repository, args.task_uid,
                                        args.base, args.head, args.run_id,
                                        args.run_attempt, mapping_path=mapping_path)
        else:
            if args.run_id is not None or args.run_attempt is not None:
                parser.error("--run-id and --run-attempt apply only to --publish-activation")
            payload = _load_json(args.publish_overlay, "overlay payload")
            result = publish_overlay(root, args.repository, args.task_uid,
                                     args.base, args.head, payload,
                                     mapping_path=mapping_path)
        print(json.dumps(result, ensure_ascii=False, sort_keys=True, indent=2))
        return 0
    except (OverlayError, OSError, ValueError, KeyError, TypeError,
            subprocess.SubprocessError) as exc:
        print(f"first-activation blocked: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
