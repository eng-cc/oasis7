#!/usr/bin/env python3
"""Stable review identity for a trusted CI-ready receipt."""

from __future__ import annotations

import hashlib
import json
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
    return {field: receipt[field] for field in AUTHORITY_FIELDS}


def review_evidence_digest(receipt: dict[str, Any]) -> str:
    canonical = json.dumps(
        review_evidence_identity(receipt), sort_keys=True, separators=(",", ":")
    ).encode()
    return hashlib.sha256(canonical).hexdigest()
