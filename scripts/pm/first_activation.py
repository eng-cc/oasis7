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
from datetime import datetime
from typing import Any


OVERLAY_SCHEMA = "oasis7-cargo-dependency-floor-overlay/v1"
OVERLAY_MARKER = "<!-- oasis7-cargo-dependency-floor-overlay/v1 -->"
ACTIVATION_SCHEMA = "oasis7-cargo-dependency-floor-activation/v1"
ACTIVATION_MARKER = "<!-- oasis7-cargo-dependency-floor-activation/v1 -->"
REVIEW_SCHEMA = "oasis7-cargo-first-activation-review/v1"
REVIEW_MARKER = "<!-- oasis7-cargo-first-activation-review/v1 -->"
FIRST_REVIEW_PLAN_SCHEMA = "oasis7-cargo-first-activation-review-plan/v1"
FIRST_REVIEW_PLAN_MARKER = "<!-- oasis7-cargo-first-activation-review-plan/v1 -->"
FIRST_REVIEW_RETURN_SCHEMA = "oasis7-cargo-first-activation-return/v1"
FIRST_REVIEW_RETURN_MARKER = "<!-- oasis7-cargo-first-activation-return/v1 -->"
FIRST_REVIEW_WRITE_SCOPE = "read-only review; no code changes"
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


def canonical_first_review_plan_body(value: dict[str, Any]) -> str:
    return FIRST_REVIEW_PLAN_MARKER + "\n" + _canonical(value).decode("utf-8") + "\n"


def canonical_first_review_return_body(value: dict[str, Any]) -> str:
    return FIRST_REVIEW_RETURN_MARKER + "\n" + _canonical(value).decode("utf-8") + "\n"


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


def _decode_marked_json(body: str, marker: str, label: str) -> dict[str, Any]:
    normalized = body.replace("\r\n", "\n")
    if normalized != body:
        _fail(f"{label} must use canonical LF framing")
    if not normalized.startswith(marker + "\n"):
        _fail(f"{label} marker is malformed")
    try:
        value = json.loads(normalized[len(marker) + 1:], object_pairs_hook=_unique_object)
    except (ValueError, json.JSONDecodeError) as exc:
        _fail(f"{label} JSON is malformed: {exc}")
    if not isinstance(value, dict) or marker + "\n" + _canonical(value).decode("utf-8") + "\n" != normalized:
        _fail(f"{label} is not canonical immutable JSON")
    return value


def _human_issue_comment(comment: dict[str, Any], issue_url: str, label: str) -> None:
    user = comment.get("user")
    login = user.get("login") if isinstance(user, dict) else None
    if (comment.get("issue_url") != issue_url or not isinstance(login, str) or not login
            or login.endswith("[bot]") or type(comment.get("id")) is not int):
        _fail(f"{label} is not a human-authored comment on the canonical Task Issue")


def _comment_sha256(comment: dict[str, Any], label: str) -> str:
    body = comment.get("body")
    if not isinstance(body, str):
        _fail(f"{label} body is unavailable")
    return "sha256:" + hashlib.sha256(body.encode("utf-8")).hexdigest()


def _validate_plan_slice_rows(value: Any, *, label: str, roles: set[str],
                              readonly: bool) -> list[dict[str, Any]]:
    if not isinstance(value, list) or not value:
        _fail(f"first-activation {label} must be a non-empty array")
    rows: list[dict[str, Any]] = []
    seen: set[str] = set()
    for item in value:
        row = _object(item, {"role", "slice_id", "packet_sha256", "write_scope"},
                      f"first-activation {label} row")
        role, slice_id, write_scope = row["role"], row["slice_id"], row["write_scope"]
        if (role not in roles or not isinstance(slice_id, str)
                or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}", slice_id)
                or slice_id in seen or not isinstance(write_scope, str) or not write_scope.strip()):
            _fail(f"first-activation {label} role/slice/scope is malformed or duplicated")
        _digest(row["packet_sha256"], f"first-activation {label} packet digest")
        if readonly and write_scope != FIRST_REVIEW_WRITE_SCOPE:
            _fail("first-activation reviewer write_scope must be exactly read-only")
        seen.add(slice_id)
        rows.append({"role": role, "slice_id": slice_id,
                     "packet_sha256": row["packet_sha256"], "write_scope": write_scope})
    return rows


def _validate_first_review_plan(
    plan: Any,
    overlay: dict[str, Any],
    *,
    repository: str,
    issue_number: int,
    comments: list[dict[str, Any]],
) -> dict[str, Any]:
    row = _object(plan, {
        "schema", "task_uid", "issue_number", "bootstrap_epoch", "snapshot_sha256",
        "request_sha256", "acceptance_sha256", "raw_primary_package",
        "effective_primary_package", "dependency", "base_oid", "head_oid",
        "authorization", "workflow", "workflow_change_paths", "implementation_slices",
        "review_slices",
    }, "first-activation review plan")
    payload = overlay["_overlay"]
    expected_workflow = {key: payload["workflow"][key]
                         for key in ("id", "path", "ref", "sha", "file_sha256")}
    expected = {
        "schema": FIRST_REVIEW_PLAN_SCHEMA,
        "task_uid": payload["task_uid"],
        "issue_number": issue_number,
        "bootstrap_epoch": payload["bootstrap_epoch"],
        "snapshot_sha256": payload["snapshot_sha256"],
        "request_sha256": payload["request_sha256"],
        "acceptance_sha256": payload["acceptance_sha256"],
        "raw_primary_package": payload["raw_primary_package"],
        "effective_primary_package": payload["effective_primary_package"],
        "dependency": payload["dependency"],
        "base_oid": payload["base_oid"],
        "head_oid": payload["head_oid"],
        "authorization": payload["authorization"],
        "workflow": expected_workflow,
        "workflow_change_paths": payload["workflow"]["change_paths"],
    }
    if any(row[key] != value for key, value in expected.items()):
        _fail("first-activation review plan differs from the exact Task/overlay/workflow binding")
    implementation = row["implementation_slices"]
    if not isinstance(implementation, list) or not implementation:
        _fail("first-activation implementation slice ownership is unavailable")
    canonical_issue_url = f"https://api.github.com/repos/{repository}/issues/{issue_number}"
    normalized_impl: list[dict[str, Any]] = []
    impl_ids: set[str] = set()
    dispatch_ids: set[int] = set()
    covered_paths: set[str] = set()
    for item in implementation:
        impl = _object(item, {"role", "slice_id", "dispatch_comments", "write_scope_paths"},
                       "first-activation implementation slice")
        if (not isinstance(impl["role"], str) or not impl["role"]
                or not isinstance(impl["slice_id"], str)
                or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}", impl["slice_id"])
                or impl["slice_id"] in impl_ids):
            _fail("first-activation implementation slice identity is malformed or duplicated")
        paths = impl["write_scope_paths"]
        if (not isinstance(paths, list) or not paths or any(
                not isinstance(path, str) or _validate_path(path) != path for path in paths)
                or paths != sorted(set(paths))):
            _fail("first-activation implementation write-scope paths are malformed")
        if any(path not in FIRST_ACTIVATION_REVIEWED_PATHS for path in paths):
            _fail("implementation slice claims a path outside the approved first-activation surface")
        refs = impl["dispatch_comments"]
        if not isinstance(refs, list) or not refs:
            _fail("implementation slice dispatch comment bindings are unavailable")
        normalized_refs: list[dict[str, Any]] = []
        comment_text = []
        ref_ids: list[int] = []
        for ref in refs:
            ref = _object(ref, {"comment_id", "body_sha256"}, "implementation dispatch comment reference")
            comment_id = ref["comment_id"]
            if type(comment_id) is not int or comment_id < 1 or comment_id in dispatch_ids:
                _fail("implementation dispatch comment ID is invalid or reused")
            _digest(ref["body_sha256"], "implementation dispatch comment digest")
            comment = _unique_comment(comments, comment_id, "implementation dispatch")
            _human_issue_comment(comment, canonical_issue_url, "implementation dispatch")
            if _comment_sha256(comment, "implementation dispatch") != ref["body_sha256"]:
                _fail("implementation dispatch comment bytes changed")
            normalized_refs.append({"comment_id": comment_id, "body_sha256": ref["body_sha256"]})
            comment_text.append(str(comment["body"]))
            ref_ids.append(comment_id)
            dispatch_ids.add(comment_id)
        if ref_ids != sorted(set(ref_ids)):
            _fail("implementation dispatch comments are not canonically ordered")
        # The source dispatch records are the authority for slice ownership.  The
        # approved plan may organize one logical slice across several records, but
        # every listed path must be present in those exact, digest-bound records.
        combined_text = "\n".join(comment_text)
        if any(not _dispatch_scope_mentions(path, combined_text) for path in paths):
            _fail("implementation path scope is not present in its pre-dispatch Issue evidence")
        covered_paths.update(paths)
        impl_ids.add(impl["slice_id"])
        normalized_impl.append({"role": impl["role"], "slice_id": impl["slice_id"],
                                "dispatch_comments": normalized_refs,
                                "write_scope_paths": paths})
    if normalized_impl != sorted(normalized_impl, key=lambda item: item["slice_id"]):
        _fail("first-activation implementation slice list is not canonically sorted")
    if covered_paths != {item["path"] for item in payload["workflow"]["change_paths"]}:
        _fail("implementation slice scopes do not exactly cover the reviewed workflow change paths")
    review = _validate_plan_slice_rows(
        row["review_slices"], label="review slices",
        roles={"repository_health_engineer", "qa_engineer"}, readonly=True,
    )
    if ([item["role"] for item in review] != ["qa_engineer", "repository_health_engineer"]
            or len(review) != 2 or impl_ids.intersection(item["slice_id"] for item in review)):
        _fail("first-activation review slices are incomplete or collide with implementation slices")
    if [item["role"] for item in review] != sorted(item["role"] for item in review):
        _fail("first-activation review slices are not canonically ordered")
    return {**row, "implementation_slices": normalized_impl, "review_slices": review}


def _comment_time(comment: dict[str, Any], label: str) -> datetime:
    value = comment.get("created_at")
    if not isinstance(value, str):
        _fail(f"{label} creation time is unavailable")
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        _fail(f"{label} creation time is malformed")
    if parsed.tzinfo is None:
        _fail(f"{label} creation time lacks a timezone")
    return parsed


def _plan_comment(comments: list[dict[str, Any]], issue_url: str) -> tuple[dict[str, Any], dict[str, Any], str]:
    marked = [item for item in comments if isinstance(item.get("body"), str)
              and FIRST_REVIEW_PLAN_MARKER in item["body"]]
    if len(marked) != 1:
        _fail("Task Issue must contain exactly one first-activation review plan")
    comment = marked[0]
    _human_issue_comment(comment, issue_url, "first-activation review plan")
    plan = _decode_marked_json(str(comment["body"]), FIRST_REVIEW_PLAN_MARKER,
                               "first-activation review plan")
    return comment, plan, _comment_sha256(comment, "first-activation review plan")


def _validate_typed_return(
    comments: list[dict[str, Any]], *, issue_url: str, digest: str, role: str,
    task_uid: str, issue_number: int, slice_id: str, base_oid: str, head_oid: str,
    plan_sha256: str, packet_sha256: str,
) -> dict[str, Any]:
    matches = [item for item in comments if isinstance(item.get("body"), str)
               and _comment_sha256(item, "first-activation return") == digest]
    if len(matches) != 1:
        _fail(f"{role} typed return comment is unavailable or ambiguous")
    comment = matches[0]
    _human_issue_comment(comment, issue_url, f"{role} typed return")
    returned = _decode_marked_json(str(comment["body"]), FIRST_REVIEW_RETURN_MARKER,
                                   f"{role} typed return")
    _object(returned, {
        "schema", "task_uid", "issue_number", "role", "slice_id", "base_oid", "head_oid",
        "review_plan_sha256", "admitted_packet_sha256", "return_status", "disposition",
        "findings", "unresolved_findings", "residual_risk",
    }, f"{role} typed return")
    if (returned["schema"] != FIRST_REVIEW_RETURN_SCHEMA or returned["task_uid"] != task_uid
            or returned["issue_number"] != issue_number or returned["role"] != role
            or returned["slice_id"] != slice_id or returned["base_oid"] != base_oid
            or returned["head_oid"] != head_oid or returned["review_plan_sha256"] != plan_sha256
            or returned["admitted_packet_sha256"] != packet_sha256
            or returned["return_status"] != "completed" or returned["disposition"] != "no_findings"
            or returned["findings"] != [] or returned["unresolved_findings"] != []
            or not isinstance(returned["residual_risk"], list)):
        _fail(f"{role} typed return is incomplete or differs from its frozen plan/packet")
    return comment


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
    plan_comment, plan_payload, plan_sha = _plan_comment(comments, canonical_issue_url)
    plan = _validate_first_review_plan(plan_payload, overlay, repository=repository,
                                       issue_number=issue_number, comments=comments)
    plan_time = _comment_time(plan_comment, "first-activation review plan")
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
    if _comment_time(auth_comment, "user authorization") >= plan_time:
        _fail("review plan must follow explicit user authorization")
    for impl in plan["implementation_slices"]:
        for ref in impl["dispatch_comments"]:
            dispatch = _unique_comment(comments, ref["comment_id"], "implementation dispatch")
            if _comment_time(dispatch, "implementation dispatch") >= plan_time:
                _fail("implementation dispatch evidence must precede the frozen review plan")
    plan_review_slices = {item["role"]: item for item in plan["review_slices"]}
    for role in ("repository_health", "qa"):
        plan_role = "repository_health_engineer" if role == "repository_health" else "qa_engineer"
        plan_review = plan_review_slices[plan_role]
        row = fields["reviews"][role]
        comment = _unique_comment(comments, row["comment_id"], f"{role} review")
        _human_issue_comment(comment, canonical_issue_url, f"{role} review")
        body = str(comment["body"])
        if "sha256:" + hashlib.sha256(body.encode("utf-8")).hexdigest() != row["body_sha256"]:
            _fail(f"{role} review bytes changed")
        normalized = body.replace("\r\n", "\n")
        if normalized != body:
            _fail(f"{role} review must use canonical LF framing")
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
                or review["slice_id"] != plan_review["slice_id"]
                or review["base_oid"] != overlay["base_oid"] or review["head_oid"] != head):
            _fail(f"{role} review is not bound to the exact Task, Issue, base, head, or role")
        if review["workflow_change_paths"] != fields["workflow"]["change_paths"]:
            _fail(f"{role} review does not bind the exact reviewed workflow path set")
        for digest_name in ("review_plan_sha256", "admitted_packet_sha256", "return_sha256"):
            _digest(review[digest_name], f"{role} {digest_name}")
        if (review["review_plan_sha256"] != plan_sha
                or review["admitted_packet_sha256"] != plan_review["packet_sha256"]):
            _fail(f"{role} review does not bind the exact no-PR plan and admitted packet")
        if review["return_status"] != "completed" or review["findings"] != [] or review["unresolved_findings"] != []:
            _fail(f"{role} review has incomplete return or unresolved findings")
        returned_comment = _validate_typed_return(
            comments, issue_url=canonical_issue_url, digest=review["return_sha256"],
            role=plan_role, task_uid=uid, issue_number=issue_number,
            slice_id=plan_review["slice_id"], base_oid=overlay["base_oid"], head_oid=head,
            plan_sha256=plan_sha, packet_sha256=plan_review["packet_sha256"],
        )
        if (_comment_time(plan_comment, f"{role} review plan") >= _comment_time(returned_comment, f"{role} typed return")
                or _comment_time(returned_comment, f"{role} typed return") >= _comment_time(comment, f"{role} review")):
            _fail(f"{role} typed return must follow the plan and precede its review envelope")
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
    overlay_time = _comment_time(comment, "dependency-floor overlay")
    referenced_ids = {payload["authorization"]["comment_id"],
                      payload["reviews"]["repository_health"]["comment_id"],
                      payload["reviews"]["qa"]["comment_id"]}
    if int(comment["id"]) in referenced_ids:
        _fail("overlay cannot reference itself as authority evidence")
    plan_comment, _, _ = _plan_comment(
        comments, f"https://api.github.com/repos/{repository}/issues/{issue['number']}"
    )
    if _comment_time(plan_comment, "first-activation review plan") >= overlay_time:
        _fail("dependency-floor overlay must follow the frozen review plan")
    for role in ("repository_health", "qa"):
        review_comment = _unique_comment(
            comments, payload["reviews"][role]["comment_id"], f"{role} review",
        )
        if _comment_time(review_comment, f"{role} review") >= overlay_time:
            _fail("dependency-floor overlay must follow both independent role returns")
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
    "doc/.governance/document-corpus/objects/48/4840d720cacf3f7d531e8a361fc146277494b75857c9bd904bbc4b0f700c6f41.json",
    ".github/workflows/rust.yml",
    "scripts/ci-tests.sh",
    "scripts/ci-required-scope-audit-contract.test.sh",
    "scripts/ci-required-capability-test-inventory.tsv",
    "scripts/pm/check-cargo-package-scope",
    "scripts/pm/check-cargo-package-scope.test.py",
    "scripts/pm/first_activation.py",
    "scripts/pm/first_activation.test.py",
    "scripts/pm/github-project-task.py",
    "scripts/pm/pr-lifecycle-gate.py",
    "scripts/prepare-task-pr.sh",
    "scripts/prepare-task-pr.test.sh",
}

# A few already-published PlanGap comments identify a bounded test/module by
# its stem or role-owned category. Keep this mapping finite and exact: it is
# not a prefix, glob, or caller-controlled path expansion.
FIRST_ACTIVATION_DISPATCH_ALIASES = {
    "doc/engineering/workflow/source-of-truth.md": ("canonical source",),
    "scripts/pm/check-cargo-package-scope.test.py": (
        "check-cargo-package-scope.test.py", "check-cargo-package-scope and its tests",
    ),
    "scripts/pm/first_activation.py": ("one focused typed activation proof module/test",),
    "scripts/pm/first_activation.test.py": ("one focused typed activation proof module/test",),
    "scripts/prepare-task-pr.test.sh": ("prepare-task-pr.sh and test",),
}


def _dispatch_scope_mentions(path: str, comments_text: str) -> bool:
    return path in comments_text or any(
        alias in comments_text for alias in FIRST_ACTIVATION_DISPATCH_ALIASES.get(path, ())
    )


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
    unresolved = candidate if candidate.is_absolute() else root / candidate
    absolute = pathlib.Path(os.path.abspath(unresolved))
    try:
        relative = absolute.relative_to(root.resolve())
    except ValueError:
        _fail(f"{label} path escapes the canonical worktree")
    cursor = root.resolve()
    for part in relative.parts:
        cursor = cursor / part
        if cursor.is_symlink():
            _fail(f"{label} path contains a symlink")
    path = absolute.resolve()
    try:
        path.relative_to(root.resolve())
    except ValueError:
        _fail(f"{label} path escapes the canonical worktree")
    if not path.is_file():
        _fail(f"{label} path is unavailable")
    return path


def _validate_local_review_artifacts(repo_root: pathlib.Path, repository: str, task_uid: str,
                                     project_item_id: str,
                                     overlay: dict[str, Any], comments: list[dict[str, Any]]) -> None:
    """Revalidate no-PR Issue plan and local reviewer packets before Project admission."""
    root = pathlib.Path(repo_root).resolve()
    issue_url = f"https://api.github.com/repos/{repository}/issues/{overlay['issue_number']}"
    plan_comment, plan_payload, plan_sha = _plan_comment(comments, issue_url)
    wrapper = {"_overlay": overlay}
    plan = _validate_first_review_plan(plan_payload, wrapper, repository=repository,
                                       issue_number=overlay["issue_number"], comments=comments)
    reviewer_rows = {item["role"]: item for item in plan["review_slices"]}
    for role, plan_role in (("repository_health", "repository_health_engineer"),
                            ("qa", "qa_engineer")):
        review_row = overlay["reviews"][role]
        review_comment = _unique_comment(comments, review_row["comment_id"], f"{role} review")
        review = _decode_marked_json(str(review_comment["body"]), REVIEW_MARKER, f"{role} review")
        plan_row = reviewer_rows[plan_role]
        if review.get("review_plan_sha256") != plan_sha:
            _fail(f"{role} review does not bind the exact first-activation plan")
        if review.get("admitted_packet_sha256") != plan_row["packet_sha256"]:
            _fail(f"{role} review does not bind its planned reviewer packet")
        packet_path = root / ".pm" / "scratch" / task_uid / "slice-packets" / f"{plan_row['slice_id']}.json"
        packet_path = _inside_repo_file(root, str(packet_path), f"{role} admitted reviewer packet")
        packet_raw = packet_path.read_bytes()
        if "sha256:" + hashlib.sha256(packet_raw).hexdigest() != plan_row["packet_sha256"]:
            _fail(f"{role} reviewer packet bytes differ from the frozen Issue plan")
        packet = _load_json(packet_path, f"{role} admitted reviewer packet")
        identity = packet.get("identity")
        slice_contract = packet.get("slice")
        if (packet.get("schema") != "oasis7-subagent-task-packet/v1"
                or not isinstance(identity, dict) or not isinstance(slice_contract, dict)
                or packet.get("packet_digest") != _packet_digest(packet)
                or identity.get("task_uid") != task_uid or identity.get("head") != overlay["head_oid"]
                or identity.get("base_sha") != overlay["base_oid"]
                or identity.get("base_binding") != "immutable_oid"
                or identity.get("repository") != repository
                or identity.get("issue_url") != issue_url
                or identity.get("project_item_id") != project_item_id
                or identity.get("branch") != overlay["workflow"]["ref"].removeprefix("refs/heads/")
                or slice_contract.get("role") != plan_role
                or slice_contract.get("slice_id") != plan_row["slice_id"]
                or slice_contract.get("slice_type") != "professional_review"
                or slice_contract.get("write_scope") != FIRST_REVIEW_WRITE_SCOPE):
            _fail(f"{role} reviewer packet is not an exact read-only Task/base/head packet")


def _packet_digest(packet: dict[str, Any]) -> str:
    unsigned = {key: value for key, value in packet.items() if key != "packet_digest"}
    return hashlib.sha256(_canonical(unsigned)).hexdigest()


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
    _validate_local_review_artifacts(worktree, repository, task_uid, task["project_item_id"],
                                     overlay_payload, review_comments)
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
