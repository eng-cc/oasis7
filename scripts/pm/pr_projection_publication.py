#!/usr/bin/env python3
"""Pure publication helpers; remote mutation is supplied by the caller."""
from __future__ import annotations
from typing import Callable, Any
from projection_publication_contract import build_contract, encode_marker, validate_contract

def prepare(*, task_uid: str, source_head_oid: str, scope_base_oid: str,
            projection_digest: str, clauses: list[str] | None = None,
            revision: int = 2) -> tuple[dict[str, Any], str]:
    contract = build_contract(task_uid=task_uid, source_head_oid=source_head_oid,
                              scope_base_oid=scope_base_oid, projection_digest=projection_digest,
                              clauses=clauses, revision=revision)
    return contract, encode_marker(contract)

def publish(update: Callable[[str], Any], **kwargs: Any) -> dict[str, Any]:
    contract, marker = prepare(**kwargs)
    update(marker)
    return validate_contract(contract)
