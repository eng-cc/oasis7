"""Resolve narrowly scoped GitHub process-exception evidence.

This module is deliberately a pure validator. Callers must provide freshly read
Task-Issue comments and current GitHub permission observations; it never writes
GitHub state or turns an unsuccessful test into a successful result.
"""
from __future__ import annotations

import json
import re
from datetime import datetime
from typing import Any, Callable


MARKER = "<!-- oasis7-process-exception/v1 -->"
SCHEMA = "oasis7-process-exception/v1"
DECISIONS = {"retry", "supersede", "waive_process_check", "revoke"}
TASK_UID_RE = re.compile(r"^task_[0-9a-f]{32}$")
OID_RE = re.compile(r"^[0-9a-f]{40,64}$")
REQUIRED_FIELDS = {
    "schema", "task_uid", "action", "head_oid", "scope", "decision",
    "reason", "authorized_decision", "replacement_evidence",
}
OPTIONAL_FIELDS = {"target_comment_id"}


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            raise ValueError("exception record contains a duplicate object key")
        value[key] = item
    return value


def _record_from_comment(comment: dict[str, Any]) -> dict[str, Any]:
    body = comment.get("body")
    if (not isinstance(body, str) or not body.startswith(MARKER + "\n")
            or body.count(MARKER) != 1):
        raise ValueError("exception marker must appear once as the exact first line")
    raw = body[len(MARKER) + 1:]
    try:
        record = json.loads(raw, object_pairs_hook=_unique_object)
    except (TypeError, json.JSONDecodeError) as exc:
        raise ValueError("exception record is not valid JSON") from exc
    if not isinstance(record, dict):
        raise ValueError("exception record must be one JSON object")
    if set(record) - REQUIRED_FIELDS - OPTIONAL_FIELDS or not REQUIRED_FIELDS <= set(record):
        raise ValueError("exception record has missing or unknown fields")
    if record.get("schema") != SCHEMA:
        raise ValueError("exception record schema is unsupported")
    if not isinstance(record.get("task_uid"), str) or not TASK_UID_RE.fullmatch(record["task_uid"]):
        raise ValueError("exception record task UID is malformed")
    if not isinstance(record.get("action"), str) or not record["action"]:
        raise ValueError("exception record action is missing")
    if not isinstance(record.get("head_oid"), str) or not OID_RE.fullmatch(record["head_oid"]):
        raise ValueError("exception record head OID is malformed")
    if not isinstance(record.get("scope"), str) or not record["scope"]:
        raise ValueError("exception record scope is missing")
    if record.get("decision") not in DECISIONS:
        raise ValueError("exception record decision is unsupported")
    if not isinstance(record.get("reason"), str) or not record["reason"].strip():
        raise ValueError("exception record reason is missing")
    if not isinstance(record.get("authorized_decision"), str) or not record["authorized_decision"].strip():
        raise ValueError("exception record authorized decision is missing")
    replacement = record.get("replacement_evidence")
    if record["decision"] == "waive_process_check":
        if not isinstance(replacement, dict) or not replacement:
            raise ValueError("process-check waiver requires replacement evidence")
    elif replacement is not None and not isinstance(replacement, dict):
        raise ValueError("replacement evidence must be an object or null")
    if record["decision"] in {"supersede", "revoke"}:
        target = record.get("target_comment_id")
        if type(target) is not int or target < 1:
            raise ValueError("supersession or revocation requires a server comment target")
    elif "target_comment_id" in record:
        raise ValueError("target comment is only valid for supersession or revocation")
    if type(comment.get("id")) is not int or comment["id"] < 1:
        raise ValueError("exception comment lacks a server-assigned positive ID")
    created_at = comment.get("created_at")
    updated_at = comment.get("updated_at")
    if (not isinstance(created_at, str) or not created_at
            or not isinstance(updated_at, str) or updated_at != created_at):
        raise ValueError("exception comment is missing creation time or was edited")
    try:
        timestamp = datetime.fromisoformat(created_at.replace("Z", "+00:00"))
    except ValueError as exc:
        raise ValueError("exception comment creation time is malformed") from exc
    if timestamp.tzinfo is None:
        raise ValueError("exception comment creation time lacks timezone")
    login = (comment.get("user") or {}).get("login") if isinstance(comment.get("user"), dict) else None
    if not isinstance(login, str) or not login:
        raise ValueError("exception comment server author is missing")
    return record


def _comment_order(comment: dict[str, Any]) -> tuple[float, int]:
    timestamp = datetime.fromisoformat(comment["created_at"].replace("Z", "+00:00"))
    return timestamp.timestamp(), comment["id"]


def resolve_process_exception(
    comments: list[dict[str, Any]],
    *,
    repository: str,
    issue_number: int,
    task_uid: str,
    action: str,
    head_oid: str,
    scope: str,
    live_admin_by_login: Callable[[str], bool],
    replacement_validator: Callable[[dict[str, Any], dict[str, Any]], bool] | None = None,
    process_check_waivable: bool = False,
) -> dict[str, Any]:
    """Select an authenticated exception bound to one task/action/head/scope.

    Historical records for other scopes or heads are inert. Supersession and
    revocation only affect the exact target comment in the same binding. A
    waiver can authorize an action only when the caller permits this process
    check to be waived and independently validates its replacement evidence.
    """
    expected_issue_url = f"https://api.github.com/repos/{repository}/issues/{issue_number}"
    try:
        if (not isinstance(repository, str) or not repository or type(issue_number) is not int
                or issue_number < 1 or not TASK_UID_RE.fullmatch(task_uid)
                or not isinstance(action, str) or not action
                or not isinstance(head_oid, str) or not OID_RE.fullmatch(head_oid)
                or not isinstance(scope, str) or not scope
                or not callable(live_admin_by_login)):
            raise ValueError("requested exception binding is malformed")
        parsed: list[tuple[dict[str, Any], dict[str, Any]]] = []
        permission_uncertain: list[tuple[dict[str, Any], dict[str, Any]]] = []
        ignored_records = 0
        for comment in comments:
            if not isinstance(comment, dict):
                continue
            body = comment.get("body")
            if not isinstance(body, str) or MARKER not in body:
                continue
            if comment.get("issue_url") != expected_issue_url:
                continue
            try:
                record = _record_from_comment(comment)
            except (TypeError, ValueError, KeyError):
                ignored_records += 1
                continue
            binding = (record.get("task_uid"), record.get("action"), record.get("head_oid"), record.get("scope"))
            if binding != (task_uid, action, head_oid, scope):
                ignored_records += 1
                continue
            login = comment["user"]["login"]
            try:
                admin = live_admin_by_login(login)
            except Exception:
                admin = None
            if admin is True:
                parsed.append((comment, record))
            elif admin is False:
                ignored_records += 1
            else:
                permission_uncertain.append((comment, record))

        if not parsed:
            if permission_uncertain:
                return {"status": "rejected", "applicable": False, "decision": None,
                        "comment_id": None, "reason": "current permission for a matching exception author is unreadable",
                        "ignored_records": ignored_records}
            return {"status": "none", "applicable": False, "decision": None,
                    "comment_id": None, "ignored_records": ignored_records}

        parsed.sort(key=lambda item: _comment_order(item[0]))
        latest_comment, latest = parsed[-1]
        if permission_uncertain:
            newest_uncertain = max(permission_uncertain, key=lambda item: _comment_order(item[0]))
            if _comment_order(newest_uncertain[0]) > _comment_order(latest_comment):
                return {"status": "rejected", "applicable": False, "decision": None,
                        "comment_id": newest_uncertain[0].get("id"),
                        "reason": "a newer matching exception author permission is unreadable",
                        "ignored_records": ignored_records}

        if latest["decision"] in {"supersede", "revoke"}:
            target = latest["target_comment_id"]
            earlier = next((item for item in parsed if item[0]["id"] == target), None)
            if earlier is None or earlier[0]["id"] >= latest_comment["id"]:
                return {"status": "rejected", "applicable": False, "decision": latest["decision"],
                        "comment_id": latest_comment["id"], "reason": "target is not an earlier record in the same binding",
                        "ignored_records": ignored_records}
            return {"status": "superseded" if latest["decision"] == "supersede" else "revoked",
                    "applicable": False, "decision": latest["decision"],
                    "comment_id": latest_comment["id"], "target_comment_id": target,
                    "ignored_records": ignored_records}

        if latest["decision"] == "waive_process_check":
            replacement = latest["replacement_evidence"]
            try:
                valid = (
                    process_check_waivable
                    and replacement_validator is not None
                    and replacement_validator(replacement, latest) is True
                )
            except Exception:
                valid = False
            if not valid:
                return {"status": "rejected", "applicable": False, "decision": "waive_process_check",
                        "comment_id": latest_comment["id"],
                        "reason": "waiver scope is forbidden or replacement evidence is not independently valid",
                        "ignored_records": ignored_records}
            return {"status": "applied", "applicable": True, "decision": "waive_process_check",
                    "comment_id": latest_comment["id"], "replacement_evidence": replacement,
                    "ignored_records": ignored_records}

        return {"status": "recorded", "applicable": False, "decision": latest["decision"],
                "comment_id": latest_comment["id"], "reason": latest["reason"],
                "ignored_records": ignored_records}
    except (TypeError, ValueError, KeyError) as exc:
        return {"status": "rejected", "applicable": False, "decision": None, "comment_id": None,
                "reason": str(exc)}
