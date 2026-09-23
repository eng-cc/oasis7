#!/usr/bin/env python3
"""Resolve a projection from a PR body without widening identity."""
from __future__ import annotations
from typing import Any
from projection_publication_contract import ContractError, decode_marker

class ResolverError(ContractError):
    pass

def resolve(body: str, *, task_uid: str | None = None, source_head_oid: str | None = None,
            scope_base_oid: str | None = None,
            live: dict[str, Any] | None = None) -> dict[str, Any]:
    if not isinstance(body, str):
        raise ResolverError("publication body must be text")
    if len(body.encode()) > 60 * 1024:
        raise ResolverError("publication body exceeds 60KiB limit")
    # v1 publications remain readable for existing PRs, but are never treated
    # as a current contract or allowed to enter the v2 validation path.
    legacy_marker = "<!-- oasis7-ci-impact-publication:v1 -->"
    v2_marker = "<!-- oasis7-ci-impact-publication:v2 -->"
    v1_count = body.count(legacy_marker)
    v2_count = body.count(v2_marker)
    if v1_count and v2_count:
        raise ResolverError("mixed publication protocols")
    if v1_count:
        if v1_count != 1 or not body.startswith(legacy_marker):
            raise ResolverError("legacy publication marker must occur exactly once at the start")
        legacy_payload = body[len(legacy_marker):]
        if not legacy_payload.startswith("\n") or not legacy_payload[1:].strip():
            raise ResolverError("legacy publication payload is ambiguous")
        return resolve_legacy(protocol="v1", body=body)
    if not all((task_uid, source_head_oid, scope_base_oid)):
        raise ResolverError("v2 resolution requires immutable identity")
    contract = decode_marker(body)
    for field, expected in (("task_uid", task_uid), ("source_head_oid", source_head_oid),
                            ("scope_base_oid", scope_base_oid)):
        if contract[field] != expected:
            raise ResolverError(f"projection {field} identity mismatch")
    if live is not None:
        for field in ("task_uid", "source_head_oid", "scope_base_oid"):
            if field in live and live[field] != contract[field]:
                raise ResolverError(f"live projection {field} identity mismatch")
    return contract

def resolve_legacy(*, protocol: str, body: str | None = None, **kwargs: Any) -> dict[str, Any]:
    """Read the old v1 publication shape without upgrading or validating it."""
    if protocol != "v1":
        raise ResolverError("UNSUPPORTED_RUN_PROTOCOL")
    if body is not None:
        marker = "<!-- oasis7-ci-impact-publication:v1 -->"
        if (not isinstance(body, str) or body.count(marker) != 1 or
                not body.startswith(marker) or
                not body[len(marker):].startswith("\n") or
                not body[len(marker) + 1:].strip()):
            raise ResolverError("legacy publication marker missing or ambiguous")
    return {
        "protocol": "v1",
        "legacy": True,
        "status": "legacy-read",
        "upgrade_required": True,
    }
