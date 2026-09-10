#!/usr/bin/env python3
"""Stable review identity for a trusted CI-ready receipt."""

from __future__ import annotations

import hashlib
import json
import re
from typing import Any

AUTHORITY_FIELDS = (
    "receipt_type", "issuer", "repository", "task_uid", "task_issue_number",
    "pr_number", "base_oid", "head_oid", "check_name", "check_app_id",
    "check_run_id", "planner_digest", "planner_config_sha256",
    "run_rust_baseline", "conclusion",
)


def review_evidence_identity(receipt: dict[str, Any]) -> dict[str, Any]:
    missing = [field for field in AUTHORITY_FIELDS if field not in receipt]
    if missing:
        raise ValueError("CI receipt is missing review authority fields: " + ",".join(missing))
    result = {field: receipt[field] for field in AUTHORITY_FIELDS}
    if 'scope_base_oid' in receipt or 'integration_base_oid' in receipt:
        if not re.fullmatch(r'[0-9a-f]{40,64}', str(receipt.get('scope_base_oid', ''))) or receipt.get('integration_base_oid') != receipt['base_oid']:
            raise ValueError('CI receipt scope/integration authority is incomplete')
        result.update(scope_base_oid=receipt['scope_base_oid'], integration_base_oid=receipt['integration_base_oid'])
    if 'integration_run_id' in receipt:
        for key in ('integration_run_id','tested_tree_oid','tested_commit_oid','workflow_sha'):
            if key not in receipt: raise ValueError('manual integration receipt authority incomplete')
            result[key]=receipt[key]
    return result


def review_evidence_digest(receipt: dict[str, Any]) -> str:
    canonical = json.dumps(
        review_evidence_identity(receipt), sort_keys=True, separators=(",", ":")
    ).encode()
    return hashlib.sha256(canonical).hexdigest()
