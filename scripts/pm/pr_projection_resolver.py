#!/usr/bin/env python3
"""Resolve a projection from a PR body without widening identity."""
from __future__ import annotations
from typing import Any
from projection_publication_contract import ContractError, decode_marker

class ResolverError(ContractError):
    pass

def resolve(body: str, *, task_uid: str, source_head_oid: str, scope_base_oid: str,
            live: dict[str, Any] | None = None) -> dict[str, Any]:
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

def resolve_legacy(*, protocol: str, **kwargs: Any) -> dict[str, Any]:
    if protocol != "v2":
        raise ResolverError("UNSUPPORTED_RUN_PROTOCOL")
    return resolve(**kwargs)
