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
CI_PUBLICATION_V1_SCHEMA = "oasis7-ci-publication/v1"
CI_PUBLICATION_V2_SCHEMA = "oasis7-ci-publication/v2"
# Kept as the legacy schema name for callers that construct history fixtures.
CI_PUBLICATION_SCHEMA = CI_PUBLICATION_V1_SCHEMA
WORKFLOW_IMPACT_PROJECTION_SCHEMA = "oasis7-workflow-impact-projection/v2"
MAX_PROJECTION_BYTES = 32 * 1024
MAX_BODY_BYTES = 60 * 1024
UID_RE = re.compile(r"task_[0-9a-f]{32}$")
OID_RE = re.compile(r"[0-9a-f]{40,64}$")
DIGEST_RE = re.compile(r"sha256:[0-9a-f]{64}$")

CI_PUBLICATION_IDENTITY_FIELDS = (
    "schema", "repository", "repository_id", "task_uid", "bootstrap_epoch",
    "source_repository_id", "source_ref", "target_ref", "source_head_oid",
    "source_scope_oid", "planner_authority_oid", "planner_config_sha256",
    "policy_digest", "projection_digest", "projection_required",
)
CI_PUBLICATION_FIELDS = frozenset({
    *CI_PUBLICATION_IDENTITY_FIELDS, "publication_id",
})
CI_PUBLICATION_V2_FIELDS = frozenset({
    *CI_PUBLICATION_FIELDS, "workflow_impact_projection",
})
CI_PUBLICATION_OPTIONAL_FIELDS = frozenset({"pr_number"})

# Match the trusted projection helper's complete v2 leaf shape. Semantic and
# repository evidence validation still belongs to that exact trusted helper.
WORKFLOW_IMPACT_PROJECTION_FIELDS = frozenset({
    "schema", "task_uid", "source_head_oid", "scope_base_oid",
    "changed_paths", "changed_paths_digest", "change_class", "manual_roles",
    "domain_role", "test_profile", "declared_tests", "consumed_contracts",
    "public_semantics", "affected_consumers", "closure_status", "ci_scope",
    "ci_capabilities", "ci_reasons", "review_roles", "ordered_role_ids",
    "review_scope", "review_escalated", "review_reasons",
    "planner_config_sha256", "planner_identity", "planner_digest",
    "verification_affected", "projection_digest",
})

class ContractError(ValueError):
    pass

def digest(value: Any) -> str:
    raw = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
    return "sha256:" + hashlib.sha256(raw).hexdigest()

def validate_ci_publication(value: Any) -> dict[str, Any]:
    """Validate an immutable Task-evidence publication envelope.

    This is separate from ``SCHEMA`` and the existing PR-body marker contract:
    the PR-body marker remains ``oasis7-ci-impact-publication/v2`` and keeps
    its exact wire shape. Historical Task v1 remains readable; v2 binds its
    complete projection leaf through the canonical projection digest.
    ``pr_number`` is a post-create reciprocal binding and is deliberately
    excluded from the immutable publication ID.
    """
    if not isinstance(value, dict) or value.get("schema") not in {
        CI_PUBLICATION_V1_SCHEMA, CI_PUBLICATION_V2_SCHEMA,
    }:
        raise ContractError("unsupported CI publication schema")
    is_v2 = value["schema"] == CI_PUBLICATION_V2_SCHEMA
    required_fields = CI_PUBLICATION_V2_FIELDS if is_v2 else CI_PUBLICATION_FIELDS
    fields = set(value)
    missing = required_fields - fields
    unknown = fields - required_fields - CI_PUBLICATION_OPTIONAL_FIELDS
    if missing or unknown:
        details = []
        if missing:
            details.append("missing=" + ",".join(sorted(missing)))
        if unknown:
            details.append("unknown=" + ",".join(sorted(unknown)))
        raise ContractError("CI publication fields do not match schema: " + "; ".join(details))

    for field in ("repository_id", "source_repository_id"):
        if type(value[field]) is not int or value[field] <= 0:
            raise ContractError(f"{field} must be a positive repository ID")
    if "pr_number" in value and (type(value["pr_number"]) is not int or value["pr_number"] <= 0):
        raise ContractError("pr_number must be positive when present")
    if not isinstance(value["repository"], str) or not re.fullmatch(r"[^/\s]+/[^/\s]+", value["repository"]):
        raise ContractError("repository must be an owner/name identity")
    _identity(value["task_uid"], UID_RE, "task_uid")
    epoch = value["bootstrap_epoch"]
    if epoch is not None and (type(epoch) is not int or epoch < 1):
        raise ContractError("bootstrap_epoch must be positive or null for a legacy task")
    for field in ("source_ref", "target_ref"):
        if not isinstance(value[field], str) or not value[field].strip() or any(ord(ch) < 32 for ch in value[field]):
            raise ContractError(f"{field} must be non-empty ref text")
    for field in ("source_head_oid", "source_scope_oid", "planner_authority_oid"):
        _identity(value[field], OID_RE, field)
    for field in ("planner_config_sha256", "policy_digest", "projection_digest", "publication_id"):
        if not isinstance(value[field], str) or not DIGEST_RE.fullmatch(value[field]):
            raise ContractError(f"{field} must be a prefixed SHA-256 digest")
    if value["projection_required"] is not True:
        raise ContractError("projection_required must be true")

    if is_v2:
        projection = value["workflow_impact_projection"]
        if not isinstance(projection, dict):
            raise ContractError("workflow_impact_projection must be an object")
        if projection.get("schema") != WORKFLOW_IMPACT_PROJECTION_SCHEMA:
            raise ContractError("workflow_impact_projection schema is unsupported")
        if set(projection) != WORKFLOW_IMPACT_PROJECTION_FIELDS:
            raise ContractError("workflow_impact_projection fields do not match v2 schema")
        if (projection.get("task_uid") != value["task_uid"]
                or projection.get("source_head_oid") != value["source_head_oid"]
                or projection.get("scope_base_oid") != value["source_scope_oid"]):
            raise ContractError("workflow_impact_projection Task UID/head/scope identity mismatch")
        if projection.get("planner_config_sha256") != value["planner_config_sha256"]:
            raise ContractError("workflow_impact_projection planner config identity mismatch")
        if projection.get("planner_digest") != value["policy_digest"]:
            raise ContractError("workflow_impact_projection planner digest identity mismatch")
        if projection.get("projection_digest") != value["projection_digest"]:
            raise ContractError("workflow_impact_projection digest identity mismatch")
        projection_body = {key: item for key, item in projection.items()
                           if key != "projection_digest"}
        if projection["projection_digest"] != digest(projection_body):
            raise ContractError("workflow_impact_projection digest mismatch")
        if len(json.dumps(projection, sort_keys=True, separators=(",", ":"),
                          ensure_ascii=False).encode("utf-8")) > MAX_PROJECTION_BYTES:
            raise ContractError("workflow_impact_projection exceeds 32KiB limit")

    identity = {field: value[field] for field in CI_PUBLICATION_IDENTITY_FIELDS}
    if value["publication_id"] != digest(identity):
        raise ContractError("CI publication identity digest mismatch")
    return value

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
    if clauses is not None and (not isinstance(clauses, list) or
                                any(not isinstance(item, str) for item in clauses)):
        raise ContractError("clauses must contain only strings")
    body = {"schema": SCHEMA, "revision": revision, "task_uid": task_uid,
            "source_head_oid": source_head_oid, "scope_base_oid": scope_base_oid,
            "projection_digest": projection_digest,
            "clauses": sorted(set(clauses or []))}
    body["contract_digest"] = digest(body)
    if len(json.dumps(body, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()) > MAX_PROJECTION_BYTES:
        raise ContractError("projection exceeds 32KiB limit")
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
    if (not isinstance(value["clauses"], list) or
            any(not isinstance(item, str) for item in value["clauses"]) or
            value["clauses"] != sorted(set(value["clauses"]))):
        raise ContractError("clauses must be sorted and unique")
    expected = digest({k: value[k] for k in required if k != "contract_digest"})
    if value["contract_digest"] != expected:
        raise ContractError("publication contract digest mismatch")
    if len(json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()) > MAX_PROJECTION_BYTES:
        raise ContractError("projection exceeds 32KiB limit")
    return value

def encode_marker(contract: dict[str, Any]) -> str:
    value = validate_contract(contract)
    marker = "<!-- oasis7-ci-impact-publication:v2 -->\n" + json.dumps(value, sort_keys=True, separators=(",", ":"))
    if len(marker.encode()) > MAX_BODY_BYTES:
        raise ContractError("publication body exceeds 60KiB limit")
    return marker

def decode_marker(body: str) -> dict[str, Any]:
    if not isinstance(body, str):
        raise ContractError("publication body must be text")
    if len(body.encode()) > MAX_BODY_BYTES:
        raise ContractError("publication body exceeds 60KiB limit")
    marker = "<!-- oasis7-ci-impact-publication:v2 -->"
    matches = body.count(marker)
    if matches != 1:
        raise ContractError("publication marker must occur exactly once")
    payload = body.split(marker, 1)[1].lstrip()
    decoder = json.JSONDecoder(object_pairs_hook=_reject_duplicate_keys)
    try:
        parsed, end = decoder.raw_decode(payload)
        if payload[end:].strip():
            raise ContractError("trailing publication payload")
        return validate_contract(parsed)
    except (json.JSONDecodeError, ContractError) as exc:
        raise ContractError("invalid publication marker") from exc

def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ContractError("duplicate publication field")
        result[key] = value
    return result
