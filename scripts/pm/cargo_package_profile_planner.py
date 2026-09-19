#!/usr/bin/env python3
"""Deterministic Cargo package/profile plan producer."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import subprocess
import tarfile
import tempfile
from typing import Any, Iterable
from urllib.parse import urlparse


SCHEMA = "oasis7-cargo-package-profile-plan/v1"
SUPPORTED_TARGETS = {"native", "wasm32-unknown-unknown"}
SUPPORTED_FEATURES = {"wasm", "libp2p", "wasmtime", "webgl2_runtime", "std"}
MAX_INDEPENDENT_WORKSPACES = 128
AUTHORITY_STAGE = "normative_source"
PLANNER_STAGE = "planner_authority"
AUTHORITY_PATH = "doc/engineering/workflow/source-of-truth.md"
AUTHORITY_FRAGMENT = "cargo-checker-authority-upgrade"
APPROVED_NORMATIVE_REPOSITORY = "eng-cc/oasis7"
APPROVED_NORMATIVE_ISSUE = 3814
APPROVED_NORMATIVE_COMMENT = 5743059557
APPROVED_NORMATIVE_PR = 3815
CURRENT_PLANNER_REPOSITORY = "eng-cc/oasis7"
CURRENT_PLANNER_ISSUE = 3818
CURRENT_PLANNER_COMMENT = 5743122124
PLANNER_WRITE_SCOPE = (
    "scripts/pm/cargo_package_profile_planner.py",
    "scripts/pm/cargo-package-profile-planner.test.py",
)
OID_PATTERN = re.compile(r"^[0-9a-f]{40}$")
DIGEST_PATTERN = re.compile(r"^sha256:[0-9a-f]{64}$")


class PlanError(RuntimeError):
    pass


def _git(repo: Path, *args: str, text: bool = True) -> Any:
    result = subprocess.run(
        ["git", "-C", str(repo), *args], capture_output=True, text=text, check=False
    )
    if result.returncode != 0:
        detail = result.stderr if text else result.stderr.decode(errors="replace")
        raise PlanError(f"git authority/range failure: {detail.strip()}")
    return result.stdout


def _is_ancestor(repo: Path, ancestor: str, descendant: str) -> bool:
    result = subprocess.run(
        ["git", "-C", str(repo), "merge-base", "--is-ancestor", ancestor, descendant],
        capture_output=True,
        check=False,
    )
    return result.returncode == 0


def _blob(repo: Path, oid: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "-C", str(repo), "show", f"{oid}:{path}"], capture_output=True, check=False
    )
    if result.returncode != 0:
        raise PlanError(f"trusted authority is missing {path} at {oid}")
    return result.stdout


def _digest_bytes(value: bytes) -> str:
    return "sha256:" + hashlib.sha256(value).hexdigest()


def _json_digest(value: Any) -> str:
    return _digest_bytes(json.dumps(value, sort_keys=True, separators=(",", ":")).encode())


def _require_oid(value: Any, field: str) -> str:
    if not isinstance(value, str) or OID_PATTERN.fullmatch(value) is None:
        raise PlanError(f"trusted authority {field} must be a full commit/tree/blob OID")
    return value


def _require_digest(value: Any, field: str) -> str:
    if isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value):
        value = "sha256:" + value
    if not isinstance(value, str) or DIGEST_PATTERN.fullmatch(value) is None:
        raise PlanError(f"trusted authority {field} must be a SHA-256 digest")
    return value


def _canonical_repository(repo: Path) -> str:
    try:
        remote = _git(repo, "remote", "get-url", "origin").strip()
    except PlanError as exc:
        raise PlanError("trusted authority repository readback is unavailable") from exc
    if remote.startswith("git@") and ":" in remote:
        repository = remote.split(":", 1)[1]
    elif "://" in remote:
        repository = urlparse(remote).path.strip("/")
    else:
        raise PlanError("trusted authority repository remote is unsupported")
    if repository.endswith(".git"):
        repository = repository[:-4]
    if "/" not in repository or any(not part for part in repository.split("/")):
        raise PlanError("trusted authority repository remote is invalid")
    return repository


def _canonical_default_branch(repo: Path) -> str:
    try:
        symbolic = _git(repo, "symbolic-ref", "--short", "refs/remotes/origin/HEAD").strip()
    except PlanError as exc:
        raise PlanError("trusted authority default branch readback is unavailable") from exc
    prefix = "origin/"
    if not symbolic.startswith(prefix) or not symbolic[len(prefix):]:
        raise PlanError("trusted authority default branch readback is invalid")
    return symbolic[len(prefix):]


def _fragment_bytes(value: bytes, anchor: str) -> bytes:
    marker = f'<a id="{anchor}"></a>'.encode("utf-8")
    lines = value.splitlines(keepends=True)
    matches = [line for line in lines if line.startswith(marker)]
    if len(matches) != 1 or not matches[0].endswith(b"\n"):
        raise PlanError("trusted authority stable fragment is missing or ambiguous")
    return matches[0]


def _parse_approved_normative_comment(body: str) -> dict[str, Any]:
    """Parse the fixed live readback comment, never a candidate-local file."""
    if "stage=normative_source" not in body or "immutable, not candidate authority" not in body:
        raise PlanError("live approved normative source comment is not an immutable readback")

    def match(pattern: str, field: str) -> str:
        found = re.search(pattern, body)
        if found is None:
            raise PlanError(f"live approved normative source is missing {field}")
        return found.group(1)

    pr_number = int(match(r"\bPR=(\d+)\b", "PR identity"))
    if pr_number != APPROVED_NORMATIVE_PR:
        raise PlanError("live approved normative source PR identity mismatch")
    return {
        "repository": match(r"\brepository=([^;]+)", "repository"),
        "default_branch": match(r"\bdefault_branch=([^;]+)", "default branch"),
        "stage": AUTHORITY_STAGE,
        "task_uid": match(r"\btask_uid=([^;]+)", "task identity"),
        "pr_number": pr_number,
        "source_head": match(r"\bsource_head=([0-9a-f]{40})\b", "source head"),
        "source_scope_base": match(
            r"\btrusted_predecessor/source_scope_base=([0-9a-f]{40})\b",
            "predecessor source scope",
        ),
        "authority_path": match(r"\bpredecessor authority path=([^;]+)", "authority path"),
        "predecessor_file_sha256": "sha256:" + match(
            r"\bpredecessor file sha256=([0-9a-f]{64})\b", "predecessor file digest"
        ),
        "merged_commit": match(r"(?i)\bmerged into commit=([0-9a-f]{40})\b", "merged commit"),
        "merged_tree": match(r"\blive git/commits API tree=([0-9a-f]{40})\b", "merged tree"),
        "authority_blob": match(r"\blive contents API path=[^ ]+ blob=([0-9a-f]{40})\b", "authority blob"),
        "authority_size": int(match(r"\bsize=(\d+)\b", "authority size")),
        "authority_bytes_sha256": "sha256:" + match(
            r"\bdecoded bytes sha256=([0-9a-f]{64})\b", "authority file digest"
        ),
        "stable_fragment": match(r"\bStable fragment anchor=([^ ]+)\b", "stable fragment"),
        "stable_fragment_sha256": "sha256:" + match(
            r"\bsha256 including final LF=([0-9a-f]{64})\b", "stable fragment digest"
        ),
    }


def _read_approved_normative_source_from_github(repo: Path) -> dict[str, Any]:
    """Read the immutable predecessor receipt from the fixed GitHub comment."""
    if _canonical_repository(repo) != APPROVED_NORMATIVE_REPOSITORY:
        raise PlanError("live approved normative source repository mismatch")
    comment_result = subprocess.run(
        [
            "gh",
            "api",
            f"repos/{APPROVED_NORMATIVE_REPOSITORY}/issues/comments/{APPROVED_NORMATIVE_COMMENT}",
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    if comment_result.returncode != 0 or not comment_result.stdout.strip():
        raise PlanError("live approved normative source GitHub readback is unavailable")
    try:
        comment = json.loads(comment_result.stdout)
    except json.JSONDecodeError as exc:
        raise PlanError("live approved normative source comment is not valid JSON") from exc
    expected_issue_url = (
        f"https://api.github.com/repos/{APPROVED_NORMATIVE_REPOSITORY}/issues/{APPROVED_NORMATIVE_ISSUE}"
    )
    expected_comment_url = (
        f"https://github.com/{APPROVED_NORMATIVE_REPOSITORY}/issues/{APPROVED_NORMATIVE_ISSUE}"
        f"#issuecomment-{APPROVED_NORMATIVE_COMMENT}"
    )
    if comment.get("issue_url") != expected_issue_url or comment.get("html_url") != expected_comment_url:
        raise PlanError("live approved normative source issue identity mismatch")
    body = comment.get("body")
    if not isinstance(body, str) or not body.strip():
        raise PlanError("live approved normative source comment body is unavailable")
    receipt = _parse_approved_normative_comment(body)

    pr_result = subprocess.run(
        ["gh", "api", f"repos/{APPROVED_NORMATIVE_REPOSITORY}/pulls/{APPROVED_NORMATIVE_PR}"],
        capture_output=True,
        text=True,
        check=False,
    )
    if pr_result.returncode != 0 or not pr_result.stdout.strip():
        raise PlanError("live approved normative source PR readback is unavailable")
    try:
        pr = json.loads(pr_result.stdout)
    except json.JSONDecodeError as exc:
        raise PlanError("live approved normative source PR is not valid JSON") from exc
    if pr.get("state") != "closed" or pr.get("merged") is not True:
        raise PlanError("live approved normative source PR is not merged")
    if pr.get("merge_commit_sha") != receipt["merged_commit"]:
        raise PlanError("live approved normative source merged commit mismatch")
    head = pr.get("head")
    base = pr.get("base")
    if not isinstance(head, dict) or not isinstance(base, dict):
        raise PlanError("live approved normative source PR head/base metadata is missing")
    head_repo = head.get("repo")
    base_repo = base.get("repo")
    if not isinstance(head_repo, dict) or not isinstance(base_repo, dict):
        raise PlanError("live approved normative source PR head/base repository is missing")
    if head_repo.get("full_name") != APPROVED_NORMATIVE_REPOSITORY:
        raise PlanError("live approved normative source head repository mismatch")
    if base_repo.get("full_name") != APPROVED_NORMATIVE_REPOSITORY:
        raise PlanError("live approved normative source base repository mismatch")
    if base.get("ref") != receipt["default_branch"]:
        raise PlanError("live approved normative source base branch mismatch")
    if head.get("sha") != receipt["source_head"]:
        raise PlanError("live approved normative source head SHA mismatch")
    return receipt


def _read_current_planner_task_from_github(repo: Path) -> dict[str, Any]:
    """Read the fixed task identity that authorizes the planner-stage binding."""
    if _canonical_repository(repo) != CURRENT_PLANNER_REPOSITORY:
        raise PlanError("live current planner task repository mismatch")
    issue_result = subprocess.run(
        ["gh", "api", f"repos/{CURRENT_PLANNER_REPOSITORY}/issues/{CURRENT_PLANNER_ISSUE}"],
        capture_output=True,
        text=True,
        check=False,
    )
    if issue_result.returncode != 0 or not issue_result.stdout.strip():
        raise PlanError("live current planner task readback is unavailable")
    try:
        issue = json.loads(issue_result.stdout)
    except json.JSONDecodeError as exc:
        raise PlanError("live current planner task is not valid JSON") from exc
    if issue.get("number") != CURRENT_PLANNER_ISSUE:
        raise PlanError("live current planner task issue identity mismatch")
    body = issue.get("body")
    if not isinstance(body, str):
        raise PlanError("live current planner task body is unavailable")
    task_match = re.search(r"(?m)^task_uid:\s*(task_[0-9a-f]+)\s*$", body)
    if task_match is None:
        raise PlanError("live current planner task UID is missing")
    pr_match = re.search(
        rf"https://github\.com/{re.escape(CURRENT_PLANNER_REPOSITORY)}/pull/(\d+)\b",
        body,
    )
    if pr_match is None:
        raise PlanError("live current planner reciprocal PR link is unavailable")
    current_pr_number = int(pr_match.group(1))

    comment_result = subprocess.run(
        [
            "gh",
            "api",
            f"repos/{CURRENT_PLANNER_REPOSITORY}/issues/comments/{CURRENT_PLANNER_COMMENT}",
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    if comment_result.returncode != 0 or not comment_result.stdout.strip():
        raise PlanError("live current planner task evidence is unavailable")
    try:
        comment = json.loads(comment_result.stdout)
    except json.JSONDecodeError as exc:
        raise PlanError("live current planner task evidence is not valid JSON") from exc
    expected_issue_url = (
        f"https://api.github.com/repos/{CURRENT_PLANNER_REPOSITORY}/issues/{CURRENT_PLANNER_ISSUE}"
    )
    expected_comment_url = (
        f"https://github.com/{CURRENT_PLANNER_REPOSITORY}/issues/{CURRENT_PLANNER_ISSUE}"
        f"#issuecomment-{CURRENT_PLANNER_COMMENT}"
    )
    if comment.get("issue_url") != expected_issue_url or comment.get("html_url") != expected_comment_url:
        raise PlanError("live current planner task evidence identity mismatch")
    evidence = comment.get("body")
    if not isinstance(evidence, str):
        raise PlanError("live current planner task evidence body is unavailable")
    task_evidence = re.search(
        rf"(?:Task UID:\s*|identity authority=UID\s*){re.escape(task_match.group(1))}\b",
        evidence,
    )
    if task_evidence is None:
        raise PlanError("live current planner task evidence UID mismatch")
    base_match = re.search(r"trusted base ([0-9a-f]{40})\b", evidence)
    branch_match = re.search(r"branch ([^,\s]+)", evidence)
    if base_match is None or branch_match is None:
        raise PlanError("live current planner task base or branch is missing")
    pr_result = subprocess.run(
        ["gh", "api", f"repos/{CURRENT_PLANNER_REPOSITORY}/pulls/{current_pr_number}"],
        capture_output=True,
        text=True,
        check=False,
    )
    if pr_result.returncode != 0 or not pr_result.stdout.strip():
        raise PlanError("live current planner PR readback is unavailable")
    try:
        pr = json.loads(pr_result.stdout)
    except json.JSONDecodeError as exc:
        raise PlanError("live current planner PR is not valid JSON") from exc
    if pr.get("number") != current_pr_number or pr.get("state") != "open":
        raise PlanError("live current planner PR identity or state mismatch")
    head = pr.get("head")
    base = pr.get("base")
    if not isinstance(head, dict) or not isinstance(base, dict):
        raise PlanError("live current planner PR head/base metadata is missing")
    head_repo = head.get("repo")
    base_repo = base.get("repo")
    if not isinstance(head_repo, dict) or not isinstance(base_repo, dict):
        raise PlanError("live current planner PR head/base repository is missing")
    if head_repo.get("full_name") != CURRENT_PLANNER_REPOSITORY:
        raise PlanError("live current planner PR head repository mismatch")
    if base_repo.get("full_name") != CURRENT_PLANNER_REPOSITORY:
        raise PlanError("live current planner PR base repository mismatch")
    if base.get("ref") != _canonical_default_branch(repo):
        raise PlanError("live current planner PR base branch mismatch")
    live_base_sha = base.get("sha")
    if not isinstance(live_base_sha, str) or OID_PATTERN.fullmatch(live_base_sha) is None:
        raise PlanError("live current planner PR base SHA is missing")
    if not _is_ancestor(repo, base_match.group(1), live_base_sha):
        raise PlanError("live current planner PR base is not descended from trusted task base")
    if head.get("ref") != branch_match.group(1):
        raise PlanError("live current planner PR head branch mismatch")
    head_sha = head.get("sha")
    if not isinstance(head_sha, str) or OID_PATTERN.fullmatch(head_sha) is None:
        raise PlanError("live current planner PR head SHA is missing")
    if head_sha != _git(repo, "rev-parse", "HEAD").strip():
        raise PlanError("live current planner PR head SHA is not checked out")
    return {
        "repository": CURRENT_PLANNER_REPOSITORY,
        "issue_number": CURRENT_PLANNER_ISSUE,
        "task_uid": task_match.group(1),
        "initial_base": base_match.group(1),
        "integration_base": live_base_sha,
        "branch": branch_match.group(1),
        "pr_number": current_pr_number,
        "head_sha": head_sha,
        "head_repository": head_repo["full_name"],
        "head_ref": head["ref"],
        "base_sha": live_base_sha,
        "base_repository": base_repo["full_name"],
        "base_ref": base["ref"],
    }


def _authority_receipt_identity(receipt: dict[str, Any]) -> dict[str, Any]:
    if isinstance(receipt.get("approved_normative_source"), dict):
        receipt = receipt["approved_normative_source"]
    fields = (
        "repository",
        "default_branch",
        "stage",
        "task_uid",
        "pr_number",
        "source_head",
        "source_scope_base",
        "authority_path",
        "predecessor_file_sha256",
        "merged_commit",
        "merged_tree",
        "authority_blob",
        "authority_size",
        "authority_bytes_sha256",
        "stable_fragment",
        "stable_fragment_sha256",
    )
    identity = {field: receipt.get(field) for field in fields}
    for field in ("predecessor_file_sha256", "authority_bytes_sha256", "stable_fragment_sha256"):
        identity[field] = _require_digest(identity[field], field)
    return identity


def _binding_value(binding: dict[str, Any], field: str) -> Any:
    if field not in binding:
        raise PlanError(f"planner authority binding is missing {field}")
    return binding[field]


def _validate_approved_normative_source(
    repo: Path,
    receipt: dict[str, Any],
    *,
    authority_binding: dict[str, Any],
    current_task: dict[str, Any],
    integration_base: str,
    source_head: str,
    source_scope_base: str,
    changed_names: set[str],
    expected_task_uid: str | None,
    expected_pr_number: int | None,
) -> dict[str, Any]:
    """Validate the immutable readback consumed by the planner stage.

    The receipt is evidence from the already merged normative-source stage.  It
    is never read from the candidate tree.  The binding is the current
    planner-stage task/PR contract and is intentionally explicit so a caller
    cannot turn a candidate comment or local file into authority.
    """
    if not isinstance(receipt, dict):
        raise PlanError("approved normative source must be an object")
    if isinstance(receipt.get("approved_normative_source"), dict):
        receipt = receipt["approved_normative_source"]
    if not isinstance(authority_binding, dict):
        raise PlanError("planner authority binding is required")
    if not isinstance(current_task, dict):
        raise PlanError("live current planner task identity is required")

    canonical_repository = _canonical_repository(repo)
    canonical_branch = _canonical_default_branch(repo)
    for field, actual in (
        ("repository", canonical_repository),
        ("default_branch", canonical_branch),
    ):
        if receipt.get(field) != actual:
            raise PlanError(f"trusted authority {field} mismatch")
        if authority_binding.get(field) != actual:
            raise PlanError(f"planner authority binding {field} mismatch")

    if current_task.get("repository") != canonical_repository:
        raise PlanError("live current planner task repository mismatch")
    current_task_uid = current_task.get("task_uid")
    current_task_issue_number = current_task.get("issue_number")
    current_task_pr_number = current_task.get("pr_number")
    current_task_initial_base = current_task.get("initial_base")
    current_task_base = current_task.get("integration_base")
    current_task_branch = current_task.get("branch")
    current_task_head_sha = current_task.get("head_sha")
    current_task_head_repository = current_task.get("head_repository")
    current_task_head_ref = current_task.get("head_ref")
    current_task_base_sha = current_task.get("base_sha")
    current_task_base_repository = current_task.get("base_repository")
    current_task_base_ref = current_task.get("base_ref")
    if not isinstance(current_task_uid, str) or not current_task_uid.startswith("task_"):
        raise PlanError("live current planner task UID is invalid")
    if current_task_issue_number != CURRENT_PLANNER_ISSUE:
        raise PlanError("live current planner task issue identity is invalid")
    if not isinstance(current_task_pr_number, int) or current_task_pr_number <= 0:
        raise PlanError("live current planner PR number is invalid")
    if not isinstance(current_task_initial_base, str) or OID_PATTERN.fullmatch(current_task_initial_base) is None:
        raise PlanError("live current planner initial base is invalid")
    if not isinstance(current_task_base, str) or OID_PATTERN.fullmatch(current_task_base) is None:
        raise PlanError("live current planner task base is invalid")
    if not isinstance(current_task_branch, str) or not current_task_branch:
        raise PlanError("live current planner task branch is invalid")
    if not isinstance(current_task_head_sha, str) or OID_PATTERN.fullmatch(current_task_head_sha) is None:
        raise PlanError("live current planner PR head SHA is invalid")
    if current_task_head_repository != canonical_repository:
        raise PlanError("live current planner PR head repository is invalid")
    if current_task_head_ref != current_task_branch:
        raise PlanError("live current planner PR head branch is invalid")
    if not isinstance(current_task_base_sha, str) or OID_PATTERN.fullmatch(current_task_base_sha) is None:
        raise PlanError("live current planner PR base SHA is invalid")
    if current_task_base_repository != canonical_repository:
        raise PlanError("live current planner PR base repository is invalid")
    if current_task_base_ref != canonical_branch:
        raise PlanError("live current planner PR base branch is invalid")

    if receipt.get("stage") != AUTHORITY_STAGE:
        raise PlanError("trusted authority stage mismatch")
    if authority_binding.get("stage") != PLANNER_STAGE:
        raise PlanError("planner authority stage mismatch")

    expected_scope = sorted(PLANNER_WRITE_SCOPE)
    write_scope = authority_binding.get("write_scope")
    if not isinstance(write_scope, list) or sorted(write_scope) != expected_scope:
        raise PlanError("planner authority write scope mismatch")
    if sorted(changed_names) != expected_scope:
        raise PlanError("planner authority candidate changes do not match write scope")

    expected_current = {
        "integration_base": integration_base,
        "source_head": source_head,
        "source_scope_base": source_scope_base,
    }
    for field, expected in expected_current.items():
        if authority_binding.get(field) != expected:
            raise PlanError(f"planner authority binding {field} mismatch")

    predecessor_fields = {
        "task_uid": "predecessor_task_uid",
        "pr_number": "predecessor_pr_number",
        "source_head": "predecessor_source_head",
        "source_scope_base": "predecessor_source_scope_base",
        "predecessor_file_sha256": "predecessor_file_sha256",
    }
    for receipt_field, binding_field in predecessor_fields.items():
        if receipt_field not in receipt:
            raise PlanError(f"trusted authority {receipt_field} binding mismatch")
        if receipt_field == "predecessor_file_sha256":
            receipt_value = _require_digest(receipt[receipt_field], receipt_field)
            binding_value = _require_digest(authority_binding.get(binding_field), binding_field)
        else:
            receipt_value = receipt[receipt_field]
            binding_value = authority_binding.get(binding_field)
        if binding_value != receipt_value:
            raise PlanError(f"trusted authority {receipt_field} binding mismatch")
    if authority_binding.get("predecessor_authority_digest") != authority_binding.get(
        "predecessor_file_sha256"
    ):
        raise PlanError("planner authority predecessor digest binding mismatch")

    task_uid = _binding_value(authority_binding, "task_uid")
    pr_number = _binding_value(authority_binding, "pr_number")
    if expected_task_uid is None or expected_pr_number is None:
        raise PlanError("planner authority expected task/PR identity is required")
    if task_uid != expected_task_uid or pr_number != expected_pr_number:
        raise PlanError("planner authority task/PR identity mismatch")
    if task_uid != current_task_uid or pr_number != current_task_pr_number:
        raise PlanError("planner authority task/PR identity is not live task truth")
    if (
        integration_base != current_task_base
        or source_scope_base != current_task_base
        or current_task_base_sha != current_task_base
    ):
        raise PlanError("planner authority base is not live task truth")
    if not _is_ancestor(repo, current_task_initial_base, current_task_base):
        raise PlanError("planner authority live PR base is not descended from trusted task base")
    if _git(repo, "branch", "--show-current").strip() != current_task_branch:
        raise PlanError("planner authority branch is not live task truth")
    if _git(repo, "rev-parse", "HEAD").strip() != source_head:
        raise PlanError("planner authority source head is not the checked-out head")
    if current_task_head_sha != source_head:
        raise PlanError("planner authority source head is not live PR truth")
    predecessor_task_uid = receipt.get("task_uid")
    predecessor_pr_number = receipt.get("pr_number")
    if not isinstance(predecessor_task_uid, str) or not predecessor_task_uid.startswith("task_"):
        raise PlanError("trusted authority predecessor task identity is invalid")
    if not isinstance(predecessor_pr_number, int) or predecessor_pr_number <= 0:
        raise PlanError("trusted authority predecessor PR identity is invalid")
    if not isinstance(task_uid, str) or not task_uid.startswith("task_"):
        raise PlanError("planner authority task identity is invalid")
    if not isinstance(pr_number, int) or pr_number <= 0:
        raise PlanError("planner authority PR identity is invalid")

    merged_commit = _require_oid(receipt.get("merged_commit"), "merged commit")
    merged_tree = _require_oid(receipt.get("merged_tree"), "merged tree")
    authority_blob = _require_oid(receipt.get("authority_blob"), "authority blob")
    predecessor_scope = _require_oid(receipt.get("source_scope_base"), "predecessor source scope")
    predecessor_head = _require_oid(receipt.get("source_head"), "predecessor source head")
    if receipt.get("authority_path") != AUTHORITY_PATH:
        raise PlanError("trusted authority path mismatch")
    if receipt.get("stable_fragment") != AUTHORITY_FRAGMENT:
        raise PlanError("trusted authority stable fragment mismatch")

    if not _is_ancestor(repo, merged_commit, source_scope_base):
        raise PlanError("trusted authority merged commit is not an ancestor of current source scope")
    live_default_tip = _require_oid(
        _git(repo, "rev-parse", f"refs/remotes/origin/{canonical_branch}").strip(),
        "live default branch tip",
    )
    if not _is_ancestor(repo, source_scope_base, live_default_tip):
        raise PlanError("current source scope is not an ancestor of the live default branch")
    if _git(repo, "show", "-s", "--format=%T", merged_commit).strip() != merged_tree:
        raise PlanError("trusted authority merged tree mismatch")
    if _git(repo, "rev-parse", f"{merged_commit}:{AUTHORITY_PATH}").strip() != authority_blob:
        raise PlanError("trusted authority path blob mismatch")

    merged_bytes = _blob(repo, merged_commit, AUTHORITY_PATH)
    merged_digest = _digest_bytes(merged_bytes)
    if _require_digest(receipt.get("authority_bytes_sha256"), "authority bytes") != merged_digest:
        raise PlanError("trusted authority file digest mismatch")
    if receipt.get("authority_size") != len(merged_bytes):
        raise PlanError("trusted authority file size mismatch")
    fragment = _fragment_bytes(merged_bytes, AUTHORITY_FRAGMENT)
    if _require_digest(receipt.get("stable_fragment_sha256"), "stable fragment") != _digest_bytes(fragment):
        raise PlanError("trusted authority fragment digest mismatch")
    live_authority_bytes = _blob(repo, live_default_tip, AUTHORITY_PATH)
    scope_authority_bytes = _blob(repo, source_scope_base, AUTHORITY_PATH)
    if _digest_bytes(scope_authority_bytes) != merged_digest:
        raise PlanError("current source scope authority path changed after approval")
    if _digest_bytes(live_authority_bytes) != _digest_bytes(scope_authority_bytes):
        raise PlanError("live default branch authority path changed after approval")
    scope_fragment = _fragment_bytes(scope_authority_bytes, AUTHORITY_FRAGMENT)
    if _digest_bytes(scope_fragment) != _digest_bytes(fragment):
        raise PlanError("current source scope authority fragment changed after approval")
    live_fragment = _fragment_bytes(live_authority_bytes, AUTHORITY_FRAGMENT)
    if _digest_bytes(live_fragment) != _digest_bytes(scope_fragment):
        raise PlanError("live default branch authority fragment changed after approval")

    predecessor_bytes = _blob(repo, predecessor_scope, AUTHORITY_PATH)
    predecessor_digest = _require_digest(receipt.get("predecessor_file_sha256"), "predecessor file")
    if predecessor_digest != _digest_bytes(predecessor_bytes):
        raise PlanError("trusted predecessor file digest mismatch")
    if _digest_bytes(_blob(repo, predecessor_head, AUTHORITY_PATH)) != merged_digest:
        raise PlanError("trusted predecessor source head does not match merged authority")

    if _digest_bytes(_blob(repo, source_head, AUTHORITY_PATH)) != merged_digest:
        raise PlanError("candidate source head does not match approved normative source")
    if AUTHORITY_PATH in changed_names:
        raise PlanError("candidate normative source self-modification is forbidden")

    return {
        "repository": canonical_repository,
        "default_branch": canonical_branch,
        "stage": AUTHORITY_STAGE,
        "task_uid": predecessor_task_uid,
        "pr_number": predecessor_pr_number,
        "source_head": predecessor_head,
        "source_scope_base": predecessor_scope,
        "merged_commit": merged_commit,
        "merged_tree": merged_tree,
        "authority_path": AUTHORITY_PATH,
        "authority_blob": authority_blob,
        "authority_bytes_sha256": merged_digest,
        "authority_size": len(merged_bytes),
        "stable_fragment": AUTHORITY_FRAGMENT,
        "stable_fragment_sha256": _digest_bytes(fragment),
        "predecessor_file_sha256": predecessor_digest,
        "predecessor_authority_digest": predecessor_digest,
        "planner_stage": PLANNER_STAGE,
        "planner_task_uid": task_uid,
        "planner_pr_number": pr_number,
        "planner_integration_base": integration_base,
        "planner_source_head": source_head,
        "planner_source_scope_base": source_scope_base,
        "planner_write_scope": expected_scope,
    }


def _extract(repo: Path, oid: str, destination: Path) -> None:
    archive = subprocess.run(
        ["git", "-C", str(repo), "archive", "--format=tar", oid],
        capture_output=True,
        check=False,
    )
    if archive.returncode != 0:
        raise PlanError("trusted source tree is unavailable")
    destination.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile() as handle:
        handle.write(archive.stdout)
        handle.flush()
        with tarfile.open(handle.name, "r:") as tar:
            for member in tar.getmembers():
                path = PurePosixPath(member.name)
                if path.is_absolute() or ".." in path.parts:
                    raise PlanError("unsafe path in trusted source archive")
            tar.extractall(destination)


def _metadata(root: Path, manifest: Path | None = None) -> dict[str, Any]:
    command = ["cargo", "metadata", "--no-deps", "--format-version", "1"]
    if manifest is not None:
        command.extend(("--manifest-path", str(manifest)))
    result = subprocess.run(
        command,
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise PlanError(f"cargo metadata unavailable: {result.stderr.strip()}")
    return json.loads(result.stdout)


def _metadata_with_independent_workspaces(root: Path) -> dict[str, Any]:
    primary = _metadata(root)
    packages = {str(package["manifest_path"]): package for package in primary["packages"]}
    oasis_metadata = (primary.get("metadata") or {}).get("oasis7") or {}
    configured = oasis_metadata.get("independent_profile_workspaces") or []
    if not isinstance(configured, list) or any(not isinstance(path, str) for path in configured):
        raise PlanError("independent profile workspace configuration is invalid")
    discovered = {
        manifest.parent.relative_to(root).as_posix()
        for pattern in (
            "crates/oasis7_builtin_wasm_modules/*/Cargo.toml",
            "tools/*/Cargo.toml",
        )
        for manifest in root.glob(pattern)
        if "[workspace]" in manifest.read_text(encoding="utf-8", errors="replace")
    }
    independent = sorted(set(configured) | discovered)
    if len(independent) > MAX_INDEPENDENT_WORKSPACES:
        raise PlanError("independent profile workspace discovery budget exceeded")
    for relative in independent:
        manifest = root / relative / "Cargo.toml"
        if not manifest.is_file():
            raise PlanError(f"independent profile workspace is missing: {relative}")
        try:
            loaded = _metadata(root / relative, manifest)["packages"]
        except PlanError:
            text = manifest.read_text(encoding="utf-8")
            name_match = re.search(r'(?ms)^\s*\[package\].*?^\s*name\s*=\s*"([^"]+)"', text)
            if name_match is None:
                raise
            dependencies = [
                {
                    "name": match.group(1),
                    "path": str((manifest.parent / match.group(2)).resolve()),
                }
                for match in re.finditer(
                    r'(?m)^\s*([A-Za-z0-9_-]+)\s*=\s*\{[^\n}]*\bpath\s*=\s*"([^"]+)"[^\n}]*\}',
                    text,
                )
            ]
            loaded = [
                {
                    "name": name_match.group(1),
                    "manifest_path": str(manifest.resolve()),
                    "dependencies": dependencies,
                }
            ]
        for package in loaded:
            packages[str(package["manifest_path"])] = package
    return {"packages": list(packages.values())}


def _package_map(root: Path, metadata: dict[str, Any]) -> dict[str, str]:
    result: dict[str, str] = {}
    for package in metadata["packages"]:
        manifest = Path(package["manifest_path"]).resolve()
        relative = manifest.relative_to(root.resolve()).as_posix()
        package_root = str(Path(relative).parent).replace("\\", "/")
        result["" if package_root == "." else package_root] = package["name"]
    return result


def _owner(path: str, packages: dict[str, str]) -> str | None:
    matches = [
        (len(root), name)
        for root, name in packages.items()
        if not root or path == root or path.startswith(root + "/")
    ]
    return max(matches)[1] if matches else None


def _edges(root: Path, metadata: dict[str, Any], packages: dict[str, str]) -> set[tuple[str, str]]:
    edges: set[tuple[str, str]] = set()
    package_names = set(packages.values())
    for package in metadata["packages"]:
        for dependency in package.get("dependencies", []):
            raw = dependency.get("path")
            if not raw:
                continue
            try:
                relative = Path(raw).resolve().relative_to(root.resolve()).as_posix()
            except ValueError:
                continue
            target = _owner(relative, packages)
            if target is None and dependency.get("name") in package_names:
                target = str(dependency["name"])
            if target and target != package["name"]:
                edges.add((package["name"], target))
    return edges


def _profiles(raw_profiles: Iterable[Any]) -> tuple[list[dict[str, Any]], list[str]]:
    profiles: list[dict[str, Any]] = []
    reasons: list[str] = []
    for raw in raw_profiles:
        if isinstance(raw, str):
            item = {"id": raw, "target": raw, "features": []}
            if raw == "native":
                item["target"] = "native"
        elif isinstance(raw, dict):
            item = {
                "id": str(raw.get("id", "unknown")),
                "target": str(raw.get("target", "unknown")),
                "features": sorted({str(value) for value in raw.get("features", [])}),
            }
        else:
            item = {"id": "unknown", "target": "unknown", "features": []}
        if item["target"] not in SUPPORTED_TARGETS or not set(item["features"]).issubset(SUPPORTED_FEATURES):
            reasons.append("unknown")
        profiles.append(item)
    profiles.sort(key=lambda item: item["id"])
    return profiles, sorted(set(reasons))


def plan_package_profiles(
    repo_root: str | Path,
    *,
    integration_base: str,
    source_head: str,
    policy_path: str,
    checker_path: str,
    profiles: Iterable[Any],
    approved_normative_source: dict[str, Any] | None = None,
    authority_binding: dict[str, Any] | None = None,
    expected_task_uid: str | None = None,
    expected_pr_number: int | None = None,
) -> dict[str, Any]:
    """Produce a deterministic package/profile plan.

    ``authority_binding`` enables stage-2 authority.  The fixed GitHub
    predecessor readback is always fetched and verified before the legacy
    policy/checker authority is consumed; ``approved_normative_source`` is
    only an optional caller-supplied consistency check against that readback.
    Omitting the binding preserves the current conservative planner behavior
    and does not activate any reduced checker route.
    """
    repo = Path(repo_root).resolve()
    source_scope_base = _git(repo, "merge-base", integration_base, source_head).strip()
    tested_tree = _git(repo, "merge-tree", "--write-tree", integration_base, source_head).strip()

    changed_names = set(
        _git(repo, "diff", "--name-only", source_scope_base, source_head).splitlines()
    )
    trusted_policy = json.loads(_blob(repo, source_scope_base, policy_path).decode("utf-8"))
    _blob(repo, source_scope_base, checker_path)
    protected = set(trusted_policy.get("protected_paths", [])) | {policy_path, checker_path}
    if protected & changed_names:
        raise PlanError("trusted authority self-modification is forbidden")

    approved_authority = None
    if approved_normative_source is not None or authority_binding is not None:
        if authority_binding is None:
            raise PlanError("planner authority binding is required")
        live_receipt = _read_approved_normative_source_from_github(repo)
        live_task = _read_current_planner_task_from_github(repo)
        if (
            approved_normative_source is not None
            and _authority_receipt_identity(approved_normative_source)
            != _authority_receipt_identity(live_receipt)
        ):
            raise PlanError("candidate approved normative source does not match live GitHub readback")
        approved_authority = _validate_approved_normative_source(
            repo,
            live_receipt,
            authority_binding=authority_binding,
            current_task=live_task,
            integration_base=integration_base,
            source_head=source_head,
            source_scope_base=source_scope_base,
            changed_names=changed_names,
            expected_task_uid=expected_task_uid,
            expected_pr_number=expected_pr_number,
        )

    with tempfile.TemporaryDirectory(prefix="cargo-profile-base-") as base_dir, tempfile.TemporaryDirectory(
        prefix="cargo-profile-head-"
    ) as head_dir, tempfile.TemporaryDirectory(prefix="cargo-profile-tested-") as tested_dir:
        base_root, head_root, tested_root = Path(base_dir), Path(head_dir), Path(tested_dir)
        _extract(repo, source_scope_base, base_root)
        _extract(repo, source_head, head_root)
        _extract(repo, tested_tree, tested_root)
        base_metadata = _metadata_with_independent_workspaces(base_root)
        head_metadata = _metadata_with_independent_workspaces(head_root)
        tested_metadata = _metadata_with_independent_workspaces(tested_root)
        base_packages = _package_map(base_root, base_metadata)
        head_packages = _package_map(head_root, head_metadata)
        tested_packages = _package_map(tested_root, tested_metadata)
        union_packages = dict(base_packages)
        union_packages.update(head_packages)
        union_packages.update(tested_packages)
        changed_packages = sorted(
            {owner for path in changed_names if (owner := _owner(path, union_packages))}
        )
        union_edges = _edges(base_root, base_metadata, base_packages) | _edges(
            head_root, head_metadata, head_packages
        ) | _edges(tested_root, tested_metadata, tested_packages)

    affected = set(changed_packages)
    for source, target in union_edges:
        if source in changed_packages:
            affected.add(target)
    consumer_frontier = set(changed_packages)
    while consumer_frontier:
        consumers = {
            source
            for source, target in union_edges
            if target in consumer_frontier and source not in affected
        }
        affected.update(consumers)
        consumer_frontier = consumers
    normalized_profiles, escalation_reasons = _profiles(profiles)
    executable_packages = set(tested_packages.values())
    tested_package_roots = {name: root for root, name in tested_packages.items()}
    items = []
    if not escalation_reasons:
        for package in sorted(affected):
            if package not in executable_packages:
                continue
            for profile in normalized_profiles:
                manifest = str(Path(tested_package_roots[package]) / "Cargo.toml")
                command = [
                    "cargo", "test" if profile["target"] == "native" else "check",
                    "--manifest-path", manifest, "-p", package,
                ]
                if profile["target"] != "native":
                    command.extend(("--target", profile["target"]))
                if profile["features"]:
                    command.extend(("--features", ",".join(profile["features"])))
                items.append({
                    "id": f"{package}-{profile['id']}",
                    "package": package,
                    "profile": profile["id"],
                    "target": profile["target"],
                    "features": profile["features"],
                    "command": command,
                    "command_digest": _json_digest(command),
                })
    if escalation_reasons:
        execution_disposition = "full_escalation"
        disposition_validated = True
    elif not items:
        # The legacy required tier remains mandatory while package/profile
        # activation is guarded.  An empty reduced plan is therefore an
        # explicit, validated hand-off to that coverage, not an implicit pass.
        execution_disposition = "legacy_required_coverage"
        disposition_validated = True
    else:
        execution_disposition = "planned_items"
        disposition_validated = False
    plan: dict[str, Any] = {
        "schema": SCHEMA,
        "source_scope_base": source_scope_base,
        "integration_base": integration_base,
        "source_head": source_head,
        "tested_tree": tested_tree,
        "changed_packages": changed_packages,
        "affected_packages": sorted(affected),
        "dependency_graph_sources": ["base", "head"],
        "profiles": normalized_profiles,
        "selected_items": [item["id"] for item in items],
        "items": items,
        "scope": "full" if escalation_reasons else "targeted",
        "escalation_reasons": escalation_reasons,
        "execution_disposition": execution_disposition,
        "disposition_validated": disposition_validated,
        "trusted_authority": {
            "policy": policy_path,
            "checker": checker_path,
            "policy_sha256": _digest_bytes(_blob(repo, source_scope_base, policy_path)),
            "planner_sha256": _digest_bytes(Path(__file__).read_bytes()),
            "profile_config_sha256": _json_digest(normalized_profiles),
            "toolchain": "rust-toolchain.toml@" + (
                _digest_bytes(_blob(repo, source_scope_base, "rust-toolchain.toml"))
                if subprocess.run(
                    ["git", "-C", str(repo), "cat-file", "-e", f"{source_scope_base}:rust-toolchain.toml"],
                    capture_output=True,
                ).returncode == 0
                else "absent"
            ),
        },
    }
    if approved_authority is not None:
        plan["trusted_authority"]["approved_normative_source"] = approved_authority
        plan["trusted_authority"]["authority_stage"] = PLANNER_STAGE
    identity = json.dumps(plan, sort_keys=True, separators=(",", ":")).encode("utf-8")
    plan["plan_id"] = "sha256:" + hashlib.sha256(identity).hexdigest()
    return plan


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--integration-base", required=True)
    parser.add_argument("--source-head", required=True)
    parser.add_argument("--policy", required=True)
    parser.add_argument("--checker", required=True)
    parser.add_argument("--profile", action="append", required=True)
    parser.add_argument("--approved-normative-source")
    parser.add_argument("--authority-binding")
    parser.add_argument("--task-uid")
    parser.add_argument("--pr-number", type=int)
    parser.add_argument("--output")
    args = parser.parse_args()
    profiles: list[Any] = []
    for raw in args.profile:
        try:
            profiles.append(json.loads(raw))
        except json.JSONDecodeError:
            profiles.append(raw)
    approved_normative_source = (
        json.loads(Path(args.approved_normative_source).read_text(encoding="utf-8"))
        if args.approved_normative_source
        else None
    )
    authority_binding = (
        json.loads(Path(args.authority_binding).read_text(encoding="utf-8"))
        if args.authority_binding
        else None
    )
    plan = plan_package_profiles(
        args.repo_root,
        integration_base=args.integration_base,
        source_head=args.source_head,
        policy_path=args.policy,
        checker_path=args.checker,
        profiles=profiles,
        approved_normative_source=approved_normative_source,
        authority_binding=authority_binding,
        expected_task_uid=args.task_uid,
        expected_pr_number=args.pr_number,
    )
    rendered = json.dumps(plan, sort_keys=True, separators=(",", ":")) + "\n"
    if args.output:
        Path(args.output).write_text(rendered, encoding="utf-8")
    else:
        print(rendered, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
