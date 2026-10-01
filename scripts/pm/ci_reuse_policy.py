#!/usr/bin/env python3
"""Resolve CI reuse policy and planner authority from the trusted workflow W.

The first hosted policy is intentionally disabled.  The policy is code in the
trusted workflow revision so a workflow-dispatch input or artifact cannot turn
reuse on by supplying a different policy object or digest.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import sys
from typing import Any

WORKFLOW_PATH = ".github/workflows/rust.yml"
POLICY_PATH = "scripts/pm/ci_reuse_policy.py"
PLANNER_CONFIG_PATH = "scripts/ci-required-scope.v2.json"
POLICY_IDENTITY_SCHEMA = "oasis7-ci-effective-policy-identity/v1"
PLANNER_AUTHORITY_SCHEMA = "oasis7-planner-inventory-authority/v1"
GITHUB_ACTIONS_APP_ID = 15368
TRUSTED_EFFECTIVE_POLICY = {
    "enabled_capabilities": [],
    "approved_executor_contract_digests": [],
    "check_app_id": GITHUB_ACTIONS_APP_ID,
}
_OID_RE = re.compile(r"[0-9a-f]{40,64}\Z")
_REPOSITORY_RE = re.compile(r"[^/\s]+/[^/\s]+\Z")
_BRANCH_RE = re.compile(r"[^\s~^:?*\[\\]+\Z")


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def canonical_digest(value: Any) -> str:
    return "sha256:" + hashlib.sha256(canonical_bytes(value)).hexdigest()


def default_effective_policy() -> dict[str, Any]:
    """Return the W-owned default; callers cannot override these fields."""
    return json.loads(json.dumps(TRUSTED_EFFECTIVE_POLICY))


def effective_policy_identity(policy: dict[str, Any]) -> dict[str, str]:
    """Use the existing C3 effective-policy identity envelope."""
    if policy != default_effective_policy():
        raise ValueError("trusted effective policy differs from the W-owned disabled default")
    digest = canonical_digest({"schema": POLICY_IDENTITY_SCHEMA, "policy": policy})
    return {"schema": POLICY_IDENTITY_SCHEMA, "digest": digest}


def resolve_trusted_policy_context(
    *,
    repository: str,
    default_branch: str,
    workflow_ref: str,
    workflow_sha: str,
    default_branch_sha: str,
    policy_source: bytes,
    local_policy_source: bytes,
    planner_config: bytes,
) -> dict[str, Any]:
    """Resolve policy only when local helper bytes and W's helper agree exactly.

    `default_branch_sha` must come from a live GitHub ref read in hosted CI or
    from the authenticated default-branch API in the local dispatcher.
    """
    if not isinstance(repository, str) or not _REPOSITORY_RE.fullmatch(repository):
        raise ValueError("trusted policy repository identity is invalid")
    if not isinstance(default_branch, str) or not _BRANCH_RE.fullmatch(default_branch):
        raise ValueError("trusted policy default branch is invalid")
    if not isinstance(workflow_sha, str) or not _OID_RE.fullmatch(workflow_sha):
        raise ValueError("trusted policy workflow SHA is invalid")
    if not isinstance(default_branch_sha, str) or not _OID_RE.fullmatch(default_branch_sha):
        raise ValueError("trusted policy default-branch SHA is invalid")
    expected_ref = f"{repository}/{WORKFLOW_PATH}@refs/heads/{default_branch}"
    if workflow_ref != expected_ref:
        raise ValueError("trusted workflow ref is not the canonical default branch")
    if workflow_sha != default_branch_sha:
        raise ValueError("trusted workflow SHA is not the live default-branch head")
    if not isinstance(policy_source, bytes) or not isinstance(local_policy_source, bytes):
        raise ValueError("trusted policy source bytes are unavailable")
    if policy_source != local_policy_source:
        raise ValueError("local policy helper differs from trusted workflow W")
    if not isinstance(planner_config, bytes):
        raise ValueError("trusted planner configuration bytes are unavailable")
    try:
        config = json.loads(planner_config.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ValueError("trusted planner configuration is malformed") from exc
    if not isinstance(config, dict) or config.get("schema") != "oasis7-ci-required-scope/v2":
        raise ValueError("trusted planner configuration schema is unsupported")

    policy = default_effective_policy()
    return {
        "schema": "oasis7-trusted-ci-reuse-policy-context/v1",
        "repository": repository,
        "workflow_ref": expected_ref,
        "workflow_sha": workflow_sha,
        "policy_source_sha256": "sha256:" + hashlib.sha256(policy_source).hexdigest(),
        "effective_policy": policy,
        "effective_policy_identity": effective_policy_identity(policy),
        "planner_inventory_authority": {
            "schema": PLANNER_AUTHORITY_SCHEMA,
            "repository": repository,
            "workflow_ref": expected_ref,
            "planner_authority_oid": workflow_sha,
            "planner_config_sha256": "sha256:" + hashlib.sha256(planner_config).hexdigest(),
        },
    }


def _git_show(root: Path, revision: str, path: str) -> bytes:
    import subprocess

    return subprocess.check_output(
        ["git", "-C", str(root), "show", f"{revision}:{path}"],
    )


def _write_context(args: argparse.Namespace) -> None:
    root = Path(args.root).resolve()
    local_policy = Path(__file__).resolve().read_bytes()
    context = resolve_trusted_policy_context(
        repository=args.repository,
        default_branch=args.default_branch,
        workflow_ref=args.workflow_ref,
        workflow_sha=args.workflow_sha,
        default_branch_sha=args.default_branch_sha,
        policy_source=_git_show(root, args.workflow_sha, POLICY_PATH),
        local_policy_source=local_policy,
        planner_config=_git_show(root, args.workflow_sha, PLANNER_CONFIG_PATH),
    )
    destination = Path(args.output)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(canonical_bytes(context) + b"\n")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["trusted-context"])
    parser.add_argument("--root", default=".")
    parser.add_argument("--repository", required=True)
    parser.add_argument("--default-branch", required=True)
    parser.add_argument("--workflow-ref", required=True)
    parser.add_argument("--workflow-sha", required=True)
    parser.add_argument("--default-branch-sha", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args(argv)
    try:
        _write_context(args)
    except (OSError, ValueError, KeyError) as exc:
        print(f"ci-reuse-policy: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
