"""Typed checks for the admin-published professional authority map."""

from __future__ import annotations

from copy import deepcopy
import hashlib
import json
import re
from typing import Any


MARKER = "oasis7-loop-approval-authority"
SCHEMA = "oasis7.loop-approval-authority/v1"
DIGEST = re.compile(r"sha256:[0-9a-f]{64}\Z")
LOGIN = re.compile(r"[A-Za-z0-9-]+\Z")
PERMISSION_RANK = {"read": 1, "triage": 2, "write": 3, "maintain": 4, "admin": 5}


def canonical_digest(value: Any) -> str:
    encoded = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    return "sha256:" + hashlib.sha256(encoded).hexdigest()


def authority_digest(value: dict[str, Any]) -> str:
    unsigned = deepcopy(value)
    unsigned["authority_digest"] = ""
    return canonical_digest(unsigned)


def build_authority_map(*, task_uid: str, role: str, account: str, permission_floor: str = "admin") -> dict[str, Any]:
    value = {
        "marker": MARKER,
        "schema": SCHEMA,
        "task_uid": task_uid,
        "role": role,
        "account": account,
        "permission_floor": permission_floor,
    }
    value["authority_digest"] = authority_digest(value)
    return value


def validate_authority_map(value: Any, *, task_uid: str, expected_role: str) -> list[str]:
    if not isinstance(value, dict):
        return ["published approval authority map is missing"]
    errors: list[str] = []
    if value.get("marker") != MARKER:
        errors.append("approval authority map marker mismatch")
    if value.get("schema") != SCHEMA:
        errors.append("approval authority map schema mismatch")
    if value.get("task_uid") != task_uid:
        errors.append("approval authority map task UID mismatch")
    if value.get("role") != expected_role:
        errors.append("approval authority map role mismatch")
    account = value.get("account")
    if not isinstance(account, str) or not LOGIN.fullmatch(account):
        errors.append("approval authority map account is invalid")
    floor = value.get("permission_floor")
    if floor not in PERMISSION_RANK:
        errors.append("approval authority map permission floor is invalid")
    digest = value.get("authority_digest")
    if not isinstance(digest, str) or not DIGEST.fullmatch(digest):
        errors.append("approval authority map digest is invalid")
    return errors


def permission_at_least(observed: Any, floor: str) -> bool:
    return PERMISSION_RANK.get(str(observed), 0) >= PERMISSION_RANK.get(floor, 99)
