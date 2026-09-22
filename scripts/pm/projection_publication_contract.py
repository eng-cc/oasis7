#!/usr/bin/env python3
"""Strict, dependency-free contract for publishing CI impact projections.

The module intentionally contains no GitHub client.  It validates immutable
publication markers and leaves remote side effects to the caller.
"""
from __future__ import annotations

import hashlib
import json
import re
from typing import Any

SCHEMA = "oasis7-ci-impact-publication/v2"
UID_RE = re.compile(r"task_[0-9a-f]{32}$")
OID_RE = re.compile(r"[0-9a-f]{40,64}$")

class ContractError(ValueError):
    pass

def digest(value: Any) -> str:
    raw = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
    return "sha256:" + hashlib.sha256(raw).hexdigest()

def _identity(value: Any, pattern: re.Pattern[str], field: str) -> str:
    if not isinstance(value, str) or not pattern.fullmatch(value):
        raise ContractError(f"{field} is not a valid immutable identity")
    return value

def build_contract(*, task_uid: str, source_head_oid: str, scope_base_oid: str,
                   projection_digest: str, revision: int = 2,
                   clauses: list[str] | None = None) -> dict[str, Any]:
    _identity(task_uid, UID_RE, "task_uid")
    _identity(source_head_oid, OID_RE, "source_head_oid")
    _identity(scope_base_oid, OID_RE, "scope_base_oid")
    if not isinstance(projection_digest, str) or not re.fullmatch(r"sha256:[0-9a-f]{64}", projection_digest):
        raise ContractError("projection_digest is invalid")
    if type(revision) is not int or revision < 1:
        raise ContractError("revision must be positive")
    body = {"schema": SCHEMA, "revision": revision, "task_uid": task_uid,
            "source_head_oid": source_head_oid, "scope_base_oid": scope_base_oid,
            "projection_digest": projection_digest,
            "clauses": sorted(set(clauses or []))}
    body["contract_digest"] = digest(body)
    return body

def validate_contract(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict) or value.get("schema") != SCHEMA:
        raise ContractError("unsupported publication contract schema")
    required = {"schema", "revision", "task_uid", "source_head_oid", "scope_base_oid",
                "projection_digest", "clauses", "contract_digest"}
    if set(value) != required:
        raise ContractError("publication contract fields do not match schema")
    _identity(value["task_uid"], UID_RE, "task_uid")
    _identity(value["source_head_oid"], OID_RE, "source_head_oid")
    _identity(value["scope_base_oid"], OID_RE, "scope_base_oid")
    if type(value["revision"]) is not int or value["revision"] < 1:
        raise ContractError("revision must be positive")
    if not isinstance(value["clauses"], list) or value["clauses"] != sorted(set(value["clauses"])):
        raise ContractError("clauses must be sorted and unique")
    expected = digest({k: value[k] for k in required if k != "contract_digest"})
    if value["contract_digest"] != expected:
        raise ContractError("publication contract digest mismatch")
    return value

def encode_marker(contract: dict[str, Any]) -> str:
    value = validate_contract(contract)
    return "<!-- oasis7-ci-impact-publication:v2 -->\n" + json.dumps(value, sort_keys=True, separators=(",", ":"))

def decode_marker(body: str) -> dict[str, Any]:
    if not isinstance(body, str):
        raise ContractError("publication body must be text")
    marker = "<!-- oasis7-ci-impact-publication:v2 -->"
    matches = body.count(marker)
    if matches != 1:
        raise ContractError("publication marker must occur exactly once")
    raw = body.split(marker, 1)[1].strip().splitlines()[0]
    try:
        return validate_contract(json.loads(raw))
    except (json.JSONDecodeError, ContractError) as exc:
        raise ContractError("invalid publication marker") from exc
