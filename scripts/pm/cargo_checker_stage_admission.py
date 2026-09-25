#!/usr/bin/env python3
"""Trusted bridge for admitting a checker-stage Cargo scope run.

The bridge is deliberately separate from both the planner and checker.  It
reads the two immutable authority receipts from GitHub, recomputes their
server-side bytes, and then binds the current checker PR and the actual
planner/checker execution to a two-phase receipt.  No local receipt or
caller-provided authority document is accepted as an authority source.
"""

from __future__ import annotations

import argparse
import base64
import datetime as dt
import hashlib
import io
import json
import os
from pathlib import Path
import re
import subprocess
import sys
from typing import Any
import zipfile


SCHEMA = "oasis7-cargo-checker-stage-admission/v1"
REPOSITORY = "eng-cc/oasis7"
DEFAULT_BRANCH = "main"
NORMATIVE_COMMENT = 5822424913
NORMATIVE_PR = 3997
NORMATIVE_ISSUE = 3996
NORMATIVE_TASK_UID = "task_a14e1a1d51a44519ab0aa94632d90df4"
PLANNER_COMMENT = 5836594318
PLANNER_PR = 4000
PLANNER_ISSUE = 3999
PLANNER_TASK_UID = "task_978dcc0005b9415cbc45e59b21b095e0"
PLANNER_REVIEW_COMMENT = 5836285037
PLANNER_INTEGRATION_COMMENT = 5836026051
PLANNER_CLEANUP_COMMENT = 5836399698
PLANNER_INTEGRATION_RUN = 36162478921
PLANNER_SOURCE_HEAD = "930217dbc32a033359b41aa0aeff1bfe07ccb6dd"
PLANNER_SOURCE_SCOPE_BASE = "b5dbf139f1f608cde4797eea3218d1f7bb1eaf9e"
PLANNER_INTEGRATION_BASE = "a5b8da54d40f5626106b78efb19eaec4976a9452"
PLANNER_MERGED_COMMIT = "9a27e39fcc714435af65777ddfd9e83e4860158d"
PLANNER_MERGED_TREE = "8a99ec25e5cdaa62de0008ec08247faca9a84b46"
PLANNER_BYTES_SHA256 = "sha256:7c8a2de3e4b9b5e4c752e5147438bfe0eb80edb7a4406bc8d2b59aff498774fc"
PLANNER_FRAGMENT_SHA256 = "sha256:63a0a1735d83d5b9431c1ecd07aa54b4437e84128dcf5cf821591489dfa4f8bb"
PLANNER_TEST_PATH = "scripts/pm/cargo-package-profile-planner.test.py"
PLANNER_TEST_BLOB = "d800de58b8b043887a3ae3290e95355b79b12a97"
PLANNER_TEST_SIZE = 67265
PLANNER_TEST_BYTES_SHA256 = "sha256:d3d38d31f9b339061ab5807ed2ed1e0a10c76ab877c8a34bbfb91814dd8e6725"
PLANNER_FRAGMENT_DESCRIPTION = (
    "APPROVED_NORMATIVE_EXPECTED from its assignment through closing brace plus final LF"
)
PLANNER_PREDECESSOR_FRAGMENT = "cargo-checker-authority-upgrade"
PLANNER_PREDECESSOR_FRAGMENT_SHA256 = (
    "sha256:16c3593deb204c7e38ae551ac553075abd24fd81e5f09d6ac9af3b0a38a9f45a"
)
PLANNER_TESTED_TREE = "03aa54afe762ae6b875339a60912349280a6a71e"
# Frozen from the unique canonical Project task mapping during this ordered
# Stage2 authority delivery. Issue #3827 remains historical predecessor truth;
# it is never a runtime fallback for the successor binding below.
LEGACY_CHECKER_ISSUE = 3827
LEGACY_CHECKER_TASK_UID = "task_be264ac2833044969d3c2c50b2b83cea"
CHECKER_ISSUE = 3971
CHECKER_TASK_UID = "task_4a631678a50b4fcb952a3c2778b15677"
CHECKER_PR = 3972
GITHUB_ACTIONS_APP_ID = 15368
NORMATIVE_PATH = "doc/engineering/workflow/source-of-truth.md"
PLANNER_PATH = "scripts/pm/cargo_package_profile_planner.py"
NORMATIVE_FRAGMENT = "cargo-package-scope-and-impact-scoped-verification"
NORMATIVE_CONTRACT_FRAGMENT = "cargo-checker-authority-upgrade"
NORMATIVE_AUTHORITY_EXPECTED = {
    "repository": REPOSITORY,
    "default_branch": DEFAULT_BRANCH,
    "task_uid": NORMATIVE_TASK_UID,
    "pr_number": NORMATIVE_PR,
    "source_head": "6dcc49fc1a3a726bc09ab88a96ac3065a7535bf0",
    "source_scope_base": "747750afc6788d8a16421f84099edae8ab3e8520",
    "authority_path": NORMATIVE_PATH,
    "predecessor_file_sha256": "sha256:6af4fbc942407cb8c44bdc383793062d1463b62ffc3fbeedbc81f79f36324ed5",
    "merged_commit": "4f97540c34aefca41d8bf790cbf09b3176f07de3",
    "merged_tree": "82b8e5ad75d44c48739165b8bd26b5896eaa783a",
    "authority_blob": "b6d1c5fa854bde6f63d14db1cfd31eb5f8a23705",
    "authority_size": 159647,
    "authority_bytes_sha256": "sha256:7ffc099266de2affb9b9726468edc45b91af55cb531537749ef8361407f4902e",
    "stable_fragment": NORMATIVE_FRAGMENT,
    "stable_fragment_sha256": "sha256:03bb8e833ed707662267633eec3747b46eaa698765a89e6a44d3c3fa679bbae0",
    "authority_contract_fragment": NORMATIVE_CONTRACT_FRAGMENT,
    "authority_contract_fragment_sha256": PLANNER_PREDECESSOR_FRAGMENT_SHA256,
}
PLANNER_AUTHORITY_EXPECTED = {
    "repository": REPOSITORY,
    "default_branch": DEFAULT_BRANCH,
    "task_uid": PLANNER_TASK_UID,
    "issue_number": PLANNER_ISSUE,
    "pr_number": PLANNER_PR,
    "source_head": PLANNER_SOURCE_HEAD,
    "source_scope_base": PLANNER_SOURCE_SCOPE_BASE,
    "trusted_integration_base": PLANNER_INTEGRATION_BASE,
    "trusted_integration_tested_tree": PLANNER_TESTED_TREE,
    "merged_commit": PLANNER_MERGED_COMMIT,
    "merged_tree": PLANNER_MERGED_TREE,
    "authority_path": PLANNER_PATH,
    "authority_blob": "7f594b7fe8495689dae011dd993803e4c743d8dc",
    "authority_size": 55453,
    "authority_bytes_sha256": PLANNER_BYTES_SHA256,
    "focused_test_path": PLANNER_TEST_PATH,
    "focused_test_blob": PLANNER_TEST_BLOB,
    "focused_test_size": PLANNER_TEST_SIZE,
    "focused_test_bytes_sha256": PLANNER_TEST_BYTES_SHA256,
    "stable_fragment": "APPROVED_NORMATIVE_EXPECTED",
    "stable_fragment_size": 1222,
    "stable_fragment_sha256": PLANNER_FRAGMENT_SHA256,
    "predecessor_issue_number": NORMATIVE_ISSUE,
    "predecessor_pr_number": NORMATIVE_PR,
    "predecessor_comment": NORMATIVE_COMMENT,
    "predecessor_merged_commit": NORMATIVE_AUTHORITY_EXPECTED["merged_commit"],
    "predecessor_merged_tree": NORMATIVE_AUTHORITY_EXPECTED["merged_tree"],
    "predecessor_fragment": NORMATIVE_CONTRACT_FRAGMENT,
    "predecessor_fragment_sha256": PLANNER_PREDECESSOR_FRAGMENT_SHA256,
    "cleanup_comment": PLANNER_CLEANUP_COMMENT,
}
CHECKER_SCOPE = (
    "scripts/pm/check-cargo-package-scope",
    "scripts/pm/check-cargo-package-scope.test.py",
)
OID_RE = re.compile(r"^[0-9a-f]{40}$")
DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")


class AdmissionError(RuntimeError):
    """Raised for any missing, stale, or unverifiable authority."""


def _require_oid(value: Any, field: str) -> str:
    if not isinstance(value, str) or OID_RE.fullmatch(value) is None:
        raise AdmissionError(f"{field} is missing or is not a full OID")
    return value


def _require_digest(value: Any, field: str) -> str:
    if isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value):
        value = "sha256:" + value
    if not isinstance(value, str) or DIGEST_RE.fullmatch(value) is None:
        raise AdmissionError(f"{field} is missing or is not a SHA-256 digest")
    return value


def _digest(data: bytes) -> str:
    return "sha256:" + hashlib.sha256(data).hexdigest()


def _json_digest(value: Any) -> str:
    return _digest(json.dumps(value, sort_keys=True, separators=(",", ":")).encode())


def command_digest(argv: list[str]) -> str:
    if not argv or any(not isinstance(value, str) or not value for value in argv):
        raise AdmissionError("checker command argv is missing or malformed")
    return _json_digest(argv)


def preflight_digest(receipt: dict[str, Any]) -> str:
    unsigned = dict(receipt)
    unsigned.pop("preflight_digest", None)
    return _json_digest(unsigned)


def durable_receipt_digest(receipt: dict[str, Any]) -> str:
    unsigned = dict(receipt)
    unsigned.pop("receipt_digest", None)
    return _json_digest(unsigned)


def gh_api(path: str) -> dict[str, Any] | list[Any]:
    """Read a GitHub API object through the runner's authenticated ``gh``."""
    try:
        return json.loads(
            subprocess.check_output(["gh", "api", path], text=True, stderr=subprocess.PIPE)
        )
    except Exception as exc:  # pragma: no cover - exact gh failures vary by runner
        raise AdmissionError(f"live GitHub readback failed for {path}: {exc}") from exc


def _field(body: str, pattern: str, name: str) -> str:
    matches = re.findall(pattern, body)
    if len(matches) != 1:
        raise AdmissionError(f"live authority receipt {name} is missing or ambiguous")
    return matches[0]


def _parse_normative_receipt(body: str) -> dict[str, Any]:
    if re.findall(r"\bstage=([^;]+)", body) != ["normative_source"] or "immutable, not candidate authority" not in body:
        raise AdmissionError("normative receipt is not an immutable server readback")
    pr = int(_field(body, r"\bPR=(\d+)\b", "PR identity"))
    if pr != NORMATIVE_PR:
        raise AdmissionError("normative receipt PR identity mismatch")
    return {
        "repository": _field(body, r"\brepository=([^;]+)", "repository"),
        "default_branch": _field(body, r"\bdefault_branch=([^;]+)", "default branch"),
        "stage": "normative_source",
        "task_uid": _field(body, r"\btask_uid=([^;]+)", "task identity"),
        "pr_number": pr,
        "source_head": _field(body, r"\bsource_head=([0-9a-f]{40})\b", "source head"),
        "source_scope_base": _field(
            body, r"\btrusted_predecessor/source_scope_base=([0-9a-f]{40})\b", "scope base"
        ),
        "predecessor_file_sha256": "sha256:"
        + _field(body, r"\bpredecessor file sha256=([0-9a-f]{64})\b", "predecessor file digest"),
        "authority_path": _field(body, r"\bpredecessor authority path=([^;]+)", "path"),
        "merged_commit": _field(body, r"(?i)\bmerged into commit=([0-9a-f]{40})\b", "commit"),
        "merged_tree": _field(body, r"\blive git/commits API tree=([0-9a-f]{40})\b", "tree"),
        "authority_blob": _field(body, r"\bblob=([0-9a-f]{40})\b", "blob"),
        "authority_size": int(_field(body, r"\bsize=(\d+)\b", "size")),
        "authority_bytes_sha256": "sha256:"
        + _field(body, r"\bdecoded bytes sha256=([0-9a-f]{64})\b", "bytes digest"),
        "stable_fragment": _field(body, r"\bStable fragment anchor=([^, ]+)", "fragment"),
        "stable_fragment_sha256": "sha256:"
        + _field(
            body,
            r"Stable fragment anchor=[^,;]+[,;] sha256 including final LF=([0-9a-f]{64})\b",
            "fragment digest",
        ),
        "authority_contract_fragment": _field(
            body, r"\bAdjacent staged-authority fragment anchor=([^,;.]+)", "staged-authority fragment"
        ),
        "authority_contract_fragment_sha256": "sha256:"
        + _field(
            body,
            r"\bAdjacent staged-authority fragment anchor=[^,;.]+[,;.] sha256 including final LF=([0-9a-f]{64})\b",
            "staged-authority fragment digest",
        ),
    }


def _parse_planner_receipt(body: str) -> dict[str, Any]:
    if (
        re.findall(r"\bstage=([^;]+)", body) != ["planner_authority"]
        or "immutable merged authority, not candidate self-approval" not in body
    ):
        raise AdmissionError("planner receipt is not an immutable planner-authority readback")
    pr = int(_field(body, r"\bPR=(\d+)\b", "PR identity"))
    if pr != PLANNER_PR:
        raise AdmissionError("planner receipt PR identity mismatch")
    cleanup_comment = int(
        _field(
            body,
            r"terminal cleanup receipt https://github\.com/eng-cc/oasis7/issues/3999#issuecomment-(\d+)",
            "terminal cleanup receipt",
        )
    )
    predecessor_url = _field(
        body,
        r"Predecessor approved normative source: (#[0-9]+/PR #[0-9]+ receipt "
        r"https://github\.com/eng-cc/oasis7/issues/[0-9]+#issuecomment-[0-9]+)",
        "normative predecessor identity",
    )
    predecessor_match = re.fullmatch(
        r"#(\d+)/PR #(\d+) receipt https://github\.com/eng-cc/oasis7/issues/(\d+)#issuecomment-(\d+)",
        predecessor_url,
    )
    if predecessor_match is None:
        raise AdmissionError("planner normative predecessor reference is malformed")
    predecessor_issue, predecessor_pr, predecessor_url_issue, predecessor_comment = map(
        int, predecessor_match.groups()
    )
    if predecessor_issue != predecessor_url_issue:
        raise AdmissionError("planner normative predecessor issue URL mismatch")
    return {
        "repository": _field(body, r"\brepository=([^;]+)", "repository"),
        "default_branch": _field(body, r"\bdefault_branch=([^;]+)", "default branch"),
        "stage": "planner_authority",
        "task_uid": _field(body, r"\btask_uid=([^;]+)", "task identity"),
        "issue_number": PLANNER_ISSUE,
        "pr_number": pr,
        "source_head": _field(body, r"\bsource_head=([0-9a-f]{40})\b", "source head"),
        "source_scope_base": _field(body, r"\bsource_scope_base=([0-9a-f]{40})\b", "scope base"),
        "trusted_integration_base": _field(
            body, r"\btrusted integration base=([0-9a-f]{40})\b", "integration base"
        ),
        "merged_commit": _field(body, r"\bmerge commit=([0-9a-f]{40})\b", "commit"),
        "merged_tree": _field(body, r"\blive git/commits API tree=([0-9a-f]{40})\b", "tree"),
        "authority_path": _field(body, r"\bauthority path=([^,]+), blob=", "path"),
        "authority_blob": _field(body, r"\bauthority path=[^,]+, blob=([0-9a-f]{40})\b", "blob"),
        "authority_size": int(
            _field(body, r"authority path=[^,]+, blob=[0-9a-f]{40}, size=(\d+)\b", "size")
        ),
        "authority_bytes_sha256": "sha256:"
        + _field(
            body,
            r"authority path=[^,]+, blob=[0-9a-f]{40}, size=\d+, decoded bytes sha256=([0-9a-f]{64})\b",
            "bytes digest",
        ),
        "focused_test_path": _field(body, r"\bfocused test path=([^,]+), blob=", "focused test path"),
        "focused_test_blob": _field(body, r"\bfocused test path=[^,]+, blob=([0-9a-f]{40})\b", "focused test blob"),
        "focused_test_size": int(_field(body, r"\bfocused test path=[^,]+, blob=[0-9a-f]{40}, size=(\d+)\b", "focused test size")),
        "focused_test_bytes_sha256": "sha256:"
        + _field(body, r"focused test path=[^,]+, blob=[0-9a-f]{40}, size=\d+, decoded bytes sha256=([0-9a-f]{64})\b", "focused test bytes digest"),
        "stable_fragment": _field(body, r"\bStable planner fragment (.*?) from its assignment", "fragment"),
        "stable_fragment_size": int(_field(body, r"from its assignment through closing brace plus final LF: (\d+) bytes", "fragment size")),
        "stable_fragment_sha256": "sha256:"
        + _field(body, r"plus final LF: \d+ bytes, sha256=([0-9a-f]{64})\b", "fragment digest"),
        "predecessor_issue_number": predecessor_issue,
        "predecessor_pr_number": predecessor_pr,
        "predecessor_comment": predecessor_comment,
        "predecessor_merged_commit": _field(body, r"\bmerged commit=([0-9a-f]{40})\b", "normative predecessor commit"),
        "predecessor_merged_tree": _field(body, r"\btree=([0-9a-f]{40}), normative fragment", "normative predecessor tree"),
        "predecessor_fragment": _field(body, r"\bnormative fragment ([^ ]+) sha256=", "normative predecessor fragment"),
        "predecessor_fragment_sha256": "sha256:"
        + _field(body, r"normative fragment [^ ]+ sha256=([0-9a-f]{64})\b", "normative predecessor fragment digest"),
        "cleanup_comment": cleanup_comment,
        "verification_evidence": body,
    }


def _validate_planner_verification(parsed: dict[str, Any]) -> None:
    evidence = str(parsed.get("verification_evidence") or "")
    required_claims = (
        "Exact-head required-gate and strict integration passed",
        "formal RH/QA r8 review closed no findings",
        "PR live gate ready",
        f"https://github.com/{REPOSITORY}/issues/{PLANNER_ISSUE}#issuecomment-{PLANNER_CLEANUP_COMMENT}",
    )
    if any(claim not in evidence for claim in required_claims):
        raise AdmissionError("planner verification or terminal cleanup claim is incomplete")


def _verify_live_integration_run(
    repository: str, parsed: dict[str, Any], repo_root: Path | None = None
) -> None:
    run_id = parsed.get("trusted_integration_run_id")
    if not isinstance(run_id, int) or run_id <= 0:
        raise AdmissionError("planner integration run identity is invalid")
    run = gh_api(f"repos/{repository}/actions/runs/{run_id}")
    if not isinstance(run, dict) or run.get("id") != run_id:
        raise AdmissionError("planner integration run readback is unavailable")
    if run.get("status") != "completed" or run.get("conclusion") != "success":
        raise AdmissionError("planner integration run is not a successful completed run")
    if run.get("event") != "workflow_dispatch" or run.get("head_branch") != DEFAULT_BRANCH:
        raise AdmissionError("planner integration run provenance is not trusted")
    if run.get("head_sha") != parsed.get("trusted_integration_base"):
        raise AdmissionError("planner integration run base identity mismatch")
    expected_title = (
        f"oasis7-ci|workflow_dispatch|integration_revalidation|{PLANNER_TASK_UID}|"
        f"{PLANNER_PR}|{parsed['trusted_integration_base']}|{parsed['source_head']}"
    )
    if run.get("path") != ".github/workflows/rust.yml" or run.get("display_title") != expected_title:
        raise AdmissionError("planner integration workflow identity mismatch")
    checks = gh_api(
        f"repos/{repository}/commits/{run['head_sha']}/check-runs?per_page=100"
    )
    marker = f"/actions/runs/{run_id}"
    matches = [
        item
        for item in (checks.get("check_runs", []) if isinstance(checks, dict) else [])
        if item.get("name") == "required-gate" and marker in str(item.get("details_url") or "")
    ]
    if (
        not isinstance(checks, dict)
        or not isinstance(checks.get("check_runs"), list)
        or checks.get("total_count") != len(checks["check_runs"])
        or len(matches) != 1
    ):
        raise AdmissionError("planner integration required-gate identity is missing or ambiguous")
    check = matches[0]
    app_id = ((check.get("app") or {}).get("id"))
    if check.get("status") != "completed" or check.get("conclusion") != "success":
        raise AdmissionError(
            "planner integration required-gate is not completed successfully"
        )
    if app_id != GITHUB_ACTIONS_APP_ID or check.get("head_sha") != run.get("head_sha"):
        raise AdmissionError("planner integration required-gate identity is invalid")
    artifacts = _read_profile_artifacts(repository, run_id)
    envelope = artifacts["envelope"]["value"]
    expected = {
        "schema": "oasis7-cargo-package-profile-envelope/v1",
        "repository": repository,
        "task_uid": parsed.get("task_uid"),
        "pr_number": parsed.get("pr_number"),
        "run_id": run_id,
        "run_attempt": run.get("run_attempt"),
        "check_name": "required-gate",
        "check_app_id": app_id,
        "check_run_id": check.get("id"),
        "integration_base": parsed.get("trusted_integration_base"),
        "source_head": parsed.get("source_head"),
        "workflow_ref": f"{repository}/.github/workflows/rust.yml@refs/heads/{DEFAULT_BRANCH}",
        "workflow_sha": run.get("head_sha"),
    }
    tested_tree = _require_oid(envelope.get("tested_tree"), "planner integration tested tree")
    expected["tested_tree"] = tested_tree
    if tested_tree != PLANNER_AUTHORITY_EXPECTED["trusted_integration_tested_tree"]:
        raise AdmissionError("planner integration tested tree differs from the approved readback")
    if any(envelope.get(field) != value for field, value in expected.items()):
        raise AdmissionError("planner integration envelope identity mismatch")
    for field in ("plan", "results", "receipt"):
        expected_digest = _require_digest(envelope.get(f"{field}_digest"), f"{field} artifact")
        if expected_digest != _digest(artifacts[field]["bytes"]):
            raise AdmissionError(f"planner integration {field} artifact digest mismatch")
    plan = artifacts["plan"]["value"]
    if any(plan.get(field) != expected[field] for field in ("integration_base", "source_head", "tested_tree")):
        raise AdmissionError("planner integration plan identity mismatch")
    results = artifacts["results"]["value"]
    if not isinstance(results, list):
        raise AdmissionError("planner integration results identity or status mismatch")
    if results and any(
        not isinstance(item, dict)
        or item.get("status") != "passed"
        or item.get("exit_code") != 0
        or any(item.get(field) != expected[field] for field in ("integration_base", "source_head", "tested_tree"))
        for item in results
    ):
        raise AdmissionError("planner integration results identity or status mismatch")
    if not results and (
        plan.get("selected_items") != []
        or plan.get("items") != []
        or plan.get("disposition_validated") is not True
    ):
        raise AdmissionError("planner integration has empty results for a nonempty or unvalidated plan")
    receipt = artifacts["receipt"]["value"]
    if (
        not isinstance(receipt, dict)
        or receipt.get("status") != "passed"
        or receipt.get("plan_id") != plan.get("plan_id")
        or any(receipt.get(field) != expected[field] for field in ("integration_base", "source_head", "tested_tree"))
    ):
        raise AdmissionError("planner integration receipt identity or status mismatch")
    parsed["trusted_integration_envelope"] = envelope
    parsed["trusted_integration_tested_tree"] = tested_tree
    if repo_root is not None:
        actual_tree = _recompute_planner_integration_tree(
            repo_root,
            parsed["trusted_integration_base"],
            parsed["source_head"],
        )
        if actual_tree != tested_tree:
            raise AdmissionError("planner integration tested tree differs from merge-tree B/H")


def _recompute_planner_integration_tree(repo_root: Path, integration_base: str, source_head: str) -> str:
    """Compute the exact B/H integration tree independently of squash-merge tree identity."""
    _require_oid(integration_base, "planner integration base")
    _require_oid(source_head, "planner source head")
    repo = repo_root.resolve()

    def has_commit(oid: str) -> bool:
        result = subprocess.run(
            ["git", "-C", str(repo), "cat-file", "-e", f"{oid}^{{commit}}"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        return result.returncode == 0

    if not has_commit(integration_base):
        subprocess.run(
            ["git", "-C", str(repo), "fetch", "--no-tags", "origin", DEFAULT_BRANCH],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            check=False,
        )
    if not has_commit(source_head):
        subprocess.run(
            [
                "git", "-C", str(repo), "fetch", "--no-tags", "origin",
                f"refs/pull/{PLANNER_PR}/head:refs/remotes/origin/trusted-planner-pr-{PLANNER_PR}",
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            check=False,
        )
        try:
            fetched_head = _git(
                repo, "rev-parse", f"refs/remotes/origin/trusted-planner-pr-{PLANNER_PR}"
            )
        except AdmissionError as exc:
            raise AdmissionError("planner source-head PR ref is unavailable") from exc
        if fetched_head != source_head:
            raise AdmissionError("planner source-head PR ref differs from approved source head")
    if not has_commit(integration_base) or not has_commit(source_head):
        raise AdmissionError("planner integration base/source objects are unavailable")
    return _require_oid(
        _git(repo, "merge-tree", "--write-tree", integration_base, source_head),
        "planner integration merge-tree",
    )


def _gh_download(path: str) -> bytes:
    try:
        return subprocess.check_output(["gh", "api", path], stderr=subprocess.PIPE)
    except Exception as exc:  # pragma: no cover - exact gh failures vary by runner
        raise AdmissionError(f"live GitHub artifact download failed for {path}: {exc}") from exc


def _read_profile_artifact(
    repository: str, artifact: dict[str, Any], member: str
) -> dict[str, Any]:
    artifact_id = artifact.get("id")
    if not isinstance(artifact_id, int) or artifact.get("expired"):
        raise AdmissionError("planner integration artifact is missing or expired")
    try:
        with zipfile.ZipFile(
            io.BytesIO(_gh_download(f"repos/{repository}/actions/artifacts/{artifact_id}/zip"))
        ) as archive:
            if archive.namelist() != [member]:
                raise ValueError(f"expected only {member}")
            payload = archive.read(member)
        value = json.loads(payload)
    except (OSError, ValueError, json.JSONDecodeError, zipfile.BadZipFile) as exc:
        raise AdmissionError(f"planner integration artifact {member} is malformed") from exc
    if not isinstance(value, (dict, list)):
        raise AdmissionError(f"planner integration artifact {member} has invalid JSON")
    return {"value": value, "bytes": payload}


def _read_profile_artifacts(repository: str, run_id: int) -> dict[str, dict[str, Any]]:
    names = {
        "envelope": ("cargo-package-profile-envelope", "cargo-package-profile-envelope.json"),
        "plan": ("cargo-package-profile-plan", "cargo-package-profile-plan.json"),
        "results": ("cargo-package-profile-results", "cargo-package-profile-results.json"),
        "receipt": ("cargo-package-profile-receipt", "cargo-package-profile-receipt.json"),
    }
    response = gh_api(f"repos/{repository}/actions/runs/{run_id}/artifacts?per_page=100")
    artifacts = response.get("artifacts") if isinstance(response, dict) else None
    if not isinstance(artifacts, list):
        raise AdmissionError("planner integration artifact listing is unavailable")
    result: dict[str, dict[str, Any]] = {}
    for key, (name, member) in names.items():
        matches = [item for item in artifacts if isinstance(item, dict) and item.get("name") == name]
        if len(matches) != 1:
            raise AdmissionError(f"planner integration artifact {name} is missing or ambiguous")
        if int((matches[0].get("workflow_run") or {}).get("id") or 0) != run_id:
            raise AdmissionError(f"planner integration artifact {name} belongs to another run")
        result[key] = _read_profile_artifact(repository, matches[0], member)
    return result


def _decode_contents(response: dict[str, Any], field: str) -> bytes:
    if response.get("encoding") != "base64" or not isinstance(response.get("content"), str):
        raise AdmissionError(f"live {field} contents encoding is unavailable")
    normalized = "".join(response["content"].split())
    try:
        return base64.b64decode(normalized, validate=True)
    except Exception as exc:
        raise AdmissionError(f"live {field} contents are malformed") from exc


def _authority_fragment(data: bytes, stage: str, anchor: str | None = None) -> bytes:
    if stage == "normative_source":
        if not anchor:
            raise AdmissionError("normative authority fragment anchor is missing")
        marker = f'<a id="{anchor}"></a>'.encode("utf-8")
        lines = [line for line in data.splitlines(keepends=True) if line.startswith(marker)]
        if len(lines) != 1 or not lines[0].endswith(b"\n"):
            raise AdmissionError("normative authority fragment is missing or ambiguous")
        return lines[0]
    if stage == "planner_authority":
        if anchor != "APPROVED_NORMATIVE_EXPECTED":
            raise AdmissionError("planner authority fragment description mismatch")
        matches = re.findall(
            rb"(?ms)^APPROVED_NORMATIVE_EXPECTED = \{\n.*?^\}\n",
            data,
        )
        if len(matches) != 1:
            raise AdmissionError("planner authority stable fragment is missing or ambiguous")
        return matches[0]
    raise AdmissionError("authority readback stage is unsupported")


def _assert_expected_fields(
    parsed: dict[str, Any], expected: dict[str, Any], label: str
) -> None:
    mismatches = [field for field, value in expected.items() if parsed.get(field) != value]
    if mismatches:
        raise AdmissionError(f"{label} readback identity/content mismatch: {', '.join(mismatches)}")


def _exact_task_uid(body: str, expected_uid: str, label: str) -> None:
    normalized = body.replace("\r\n", "\n").replace("\r", "\n")
    uid_fields = [
        line
        for line in normalized.split("\n")
        if re.match(r"^[ \t]*(?:[-*][ \t]*)?task_uid(?:\b|[ \t]*[:=])", line)
    ]
    if uid_fields != [f"task_uid: {expected_uid}"]:
        raise AdmissionError(f"live {label} task UID field is missing, malformed, or ambiguous")


def _verify_task_issue_binding(
    repository: str, issue_number: int, pr_number: int, task_uid: str, pull: dict[str, Any]
) -> dict[str, Any]:
    issue = gh_api(f"repos/{repository}/issues/{issue_number}")
    if not isinstance(issue, dict) or issue.get("number") != issue_number:
        raise AdmissionError("live authority task Issue readback is malformed")
    if issue.get("repository_url") != f"https://api.github.com/repos/{repository}":
        raise AdmissionError("live authority task Issue repository mismatch")
    if issue.get("state") != "closed":
        raise AdmissionError("live authority task Issue is not terminal")
    body = issue.get("body")
    if not isinstance(body, str):
        raise AdmissionError("live authority task Issue body is unavailable")
    _exact_task_uid(body, task_uid, "authority")
    expected_url = f"https://github.com/{repository}/pull/{pr_number}"
    pr_urls = re.findall(r"https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+/pull/\d+", body)
    pr_numbers = re.findall(r"(?m)^[ \t]*-[ \t]*pr_number:[ \t]*`?(\d+)`?[ \t]*$", body)
    if pr_urls != [expected_url] or pr_numbers != [str(pr_number)]:
        raise AdmissionError("live authority task Issue does not reciprocally bind its PR")

    created_at = issue.get("created_at")
    pr_created_at = pull.get("created_at")
    try:
        issue_created = dt.datetime.fromisoformat(str(created_at).replace("Z", "+00:00"))
        pr_created = dt.datetime.fromisoformat(str(pr_created_at).replace("Z", "+00:00"))
    except ValueError as exc:
        raise AdmissionError("live authority task Issue/PR creation chronology is malformed") from exc
    if issue_created.tzinfo is None or pr_created.tzinfo is None or not issue_created < pr_created:
        raise AdmissionError("live authority task Issue must predate its reciprocal PR")

    pr_body = pull.get("body")
    if not isinstance(pr_body, str):
        raise AdmissionError("live authority PR task references are unavailable")
    task_lines = re.findall(r"(?m)^Task: (task_[0-9a-f]+)[ \t]*$", pr_body)
    issue_refs = re.findall(r"(?m)^Refs #(\d+)[ \t]*$", pr_body)
    if task_lines != [task_uid] or issue_refs != [str(issue_number)]:
        raise AdmissionError("live authority PR does not reciprocally bind its task Issue/UID")
    return issue


def _verify_fixed_comment(comment: dict[str, Any], issue_number: int, comment_id: int) -> str:
    expected_issue_url = f"https://api.github.com/repos/{REPOSITORY}/issues/{issue_number}"
    expected_html_url = (
        f"https://github.com/{REPOSITORY}/issues/{issue_number}#issuecomment-{comment_id}"
    )
    user = comment.get("user") or {}
    if (
        comment.get("id") != comment_id
        or comment.get("issue_url") != expected_issue_url
        or comment.get("html_url") != expected_html_url
        or user.get("login") != "eng-cc"
        or comment.get("created_at") != comment.get("updated_at")
    ):
        raise AdmissionError("live authority evidence comment identity or immutability mismatch")
    body = comment.get("body")
    if not isinstance(body, str) or not body.strip():
        raise AdmissionError("live authority evidence comment body is unavailable")
    return body


def _verify_planner_review(repository: str, parsed: dict[str, Any]) -> None:
    comment = gh_api(f"repos/{repository}/issues/comments/{PLANNER_REVIEW_COMMENT}")
    if not isinstance(comment, dict):
        raise AdmissionError("planner source review readback is malformed")
    body = _verify_fixed_comment(comment, PLANNER_ISSUE, PLANNER_REVIEW_COMMENT)
    expected_lines = (
        f"Task UID: {PLANNER_TASK_UID}",
        f"Source Head: {parsed['source_head']}",
        "Review Roles: repository_health_engineer,qa_engineer",
        "Review Evidence: repository_health_engineer: no_findings; qa_engineer: no_findings",
        "Review Findings Disposition: no_findings",
    )
    normalized_lines = [line[2:] if line.startswith("- ") else line for line in body.splitlines()]
    if any(
        [line for line in normalized_lines if line.partition(":")[0] == expected.partition(":")[0]]
        != [expected]
        for expected in expected_lines
    ):
        raise AdmissionError("planner source review is missing exact-head RH/QA no-findings evidence")


def _read_planner_integration_run_id(repository: str, parsed: dict[str, Any]) -> int:
    comment = gh_api(f"repos/{repository}/issues/comments/{PLANNER_INTEGRATION_COMMENT}")
    if not isinstance(comment, dict):
        raise AdmissionError("planner strict-integration request evidence is malformed")
    body = _verify_fixed_comment(comment, PLANNER_ISSUE, PLANNER_INTEGRATION_COMMENT)
    identities = re.findall(
        r"Identity: UID (task_[0-9a-f]+),.*?sole PR #(\d+), frozen H=([0-9a-f]{40}), S/B=([0-9a-f]{40})",
        body,
    )
    run_ids = re.findall(r"\bStrict integration run (\d+) dispatched\b", body)
    if identities != [
        (
            PLANNER_TASK_UID,
            str(PLANNER_PR),
            parsed["source_head"],
            parsed["trusted_integration_base"],
        )
    ] or run_ids != [str(PLANNER_INTEGRATION_RUN)]:
        raise AdmissionError("planner strict-integration request identity is missing or ambiguous")
    return int(run_ids[0])


def _verify_live_required_gate(
    repository: str, parsed: dict[str, Any], pull: dict[str, Any]
) -> None:
    head_sha = parsed["source_head"]
    response = gh_api(f"repos/{repository}/commits/{head_sha}/check-runs?per_page=100")
    check_runs = response.get("check_runs") if isinstance(response, dict) else None
    total_count = response.get("total_count") if isinstance(response, dict) else None
    if not isinstance(check_runs, list) or total_count != len(check_runs):
        raise AdmissionError("planner required-gate check-run listing is incomplete")
    matches = [item for item in check_runs if isinstance(item, dict) and item.get("name") == "required-gate"]
    if len(matches) != 1:
        raise AdmissionError("planner exact-head required-gate is missing or ambiguous")
    check = matches[0]
    app_id = ((check.get("app") or {}).get("id"))
    if (
        check.get("head_sha") != head_sha
        or check.get("status") != "completed"
        or check.get("conclusion") != "success"
        or app_id != GITHUB_ACTIONS_APP_ID
    ):
        raise AdmissionError("planner exact-head required-gate is not completed successfully")
    details_url = check.get("details_url")
    run_match = re.fullmatch(
        rf"https://github\.com/{re.escape(repository)}/actions/runs/(\d+)/job/\d+",
        str(details_url or ""),
    )
    if run_match is None:
        raise AdmissionError("planner required-gate workflow run identity is malformed")
    run_id = int(run_match.group(1))
    run = gh_api(f"repos/{repository}/actions/runs/{run_id}")
    head = pull.get("head") or {}
    if (
        not isinstance(run, dict)
        or run.get("id") != run_id
        or run.get("status") != "completed"
        or run.get("conclusion") != "success"
        or run.get("event") != "pull_request"
        or run.get("path") != ".github/workflows/rust.yml"
        or run.get("head_sha") != head_sha
        or run.get("head_branch") != head.get("ref")
        or (run.get("head_repository") or {}).get("full_name") != repository
    ):
        raise AdmissionError("planner required-gate workflow run provenance mismatch")


def _verify_server_readback(
    repository: str,
    *,
    comment: int,
    pr_number: int,
    stage: str,
    repo_root: Path | None = None,
) -> dict[str, Any]:
    if repository != REPOSITORY:
        raise AdmissionError("authority repository is not canonical")
    repository_info = gh_api(f"repos/{repository}")
    if not isinstance(repository_info, dict):
        raise AdmissionError("live repository readback is malformed")
    if repository_info.get("default_branch") != DEFAULT_BRANCH:
        raise AdmissionError("authority default branch is not canonical")
    comment_object = gh_api(f"repos/{repository}/issues/comments/{comment}")
    if not isinstance(comment_object, dict):
        raise AdmissionError("authority readback comment is malformed")
    expected_issue = NORMATIVE_ISSUE if stage == "normative_source" else PLANNER_ISSUE
    body = _verify_fixed_comment(comment_object, expected_issue, comment)
    parsed = _parse_normative_receipt(body) if stage == "normative_source" else _parse_planner_receipt(body)
    if stage == "normative_source":
        _assert_expected_fields(parsed, NORMATIVE_AUTHORITY_EXPECTED, "normative authority")
    else:
        _assert_expected_fields(
            parsed,
            {
                key: value
                for key, value in PLANNER_AUTHORITY_EXPECTED.items()
                if key != "trusted_integration_tested_tree"
            },
            "planner authority",
        )
        _validate_planner_verification(parsed)
        _verify_planner_review(repository, parsed)
        parsed["trusted_integration_run_id"] = _read_planner_integration_run_id(
            repository, parsed
        )
    if parsed["repository"] != repository or parsed["default_branch"] != DEFAULT_BRANCH:
        raise AdmissionError("authority repository/default branch mismatch")
    expected_stage = "normative_source" if stage == "normative_source" else "planner_authority"
    if parsed["pr_number"] != pr_number or parsed["stage"] != expected_stage:
        raise AdmissionError("authority stage or PR identity mismatch")
    pull = gh_api(f"repos/{repository}/pulls/{pr_number}")
    if not isinstance(pull, dict) or pull.get("number") != pr_number:
        raise AdmissionError("authority PR readback is malformed")
    if pull.get("state") != "closed" or pull.get("merged") is not True:
        raise AdmissionError("authority PR is not merged")
    if pull.get("merge_commit_sha") != parsed["merged_commit"]:
        raise AdmissionError("authority merged commit differs from live PR")
    base = pull.get("base") or {}
    head = pull.get("head") or {}
    if base.get("ref") != DEFAULT_BRANCH or (base.get("repo") or {}).get("full_name") != repository:
        raise AdmissionError("authority PR base repository/branch mismatch")
    if (head.get("repo") or {}).get("full_name") != repository:
        raise AdmissionError("authority PR head repository mismatch")
    if head.get("sha") != parsed.get("source_head"):
        raise AdmissionError("authority PR head/source head identity mismatch")
    issue_number = NORMATIVE_ISSUE if stage == "normative_source" else PLANNER_ISSUE
    issue = _verify_task_issue_binding(
        repository, issue_number, pr_number, parsed["task_uid"], pull
    )
    live_base = base.get("sha")
    if not isinstance(live_base, str) or OID_RE.fullmatch(live_base) is None:
        raise AdmissionError("authority PR base SHA is unavailable")
    expected_base = (
        parsed.get("source_scope_base")
        if stage == "normative_source"
        else parsed.get("trusted_integration_base")
    )
    if expected_base != live_base:
        raise AdmissionError("authority receipt base identity mismatch")
    if stage == "planner_authority":
        source_scope_base = parsed.get("source_scope_base")
        for descendant in (live_base, parsed["source_head"]):
            comparison = gh_api(
                f"repos/{repository}/compare/{source_scope_base}...{descendant}"
            )
            merge_base = (
                ((comparison.get("merge_base_commit") or {}).get("sha"))
                if isinstance(comparison, dict)
                else None
            )
            if merge_base != source_scope_base:
                raise AdmissionError("planner source-scope base is not an ancestor of the live PR identity")
        _verify_live_required_gate(repository, parsed, pull)
        _verify_live_integration_run(repository, parsed, repo_root)
        _assert_expected_fields(parsed, PLANNER_AUTHORITY_EXPECTED, "planner authority")
    commit = gh_api(f"repos/{repository}/commits/{parsed['merged_commit']}")
    if not isinstance(commit, dict):
        raise AdmissionError("authority commit readback is malformed")
    if commit.get("sha") != parsed["merged_commit"]:
        raise AdmissionError("authority commit readback identity mismatch")
    tree = ((commit.get("commit") or {}).get("tree") or {}).get("sha")
    if tree != parsed["merged_tree"]:
        raise AdmissionError("authority commit tree mismatch")
    content = gh_api(
        f"repos/{repository}/contents/{parsed['authority_path']}?ref={parsed['merged_commit']}"
    )
    if not isinstance(content, dict):
        raise AdmissionError("authority contents readback is malformed")
    if content.get("path") != parsed["authority_path"]:
        raise AdmissionError("authority contents path identity mismatch")
    data = _decode_contents(content, stage)
    if content.get("sha") != parsed["authority_blob"]:
        raise AdmissionError("authority path blob mismatch")
    if int(content.get("size") or -1) != len(data) or len(data) != parsed["authority_size"]:
        raise AdmissionError("authority path size mismatch")
    if _digest(data) != _require_digest(parsed["authority_bytes_sha256"], "authority bytes"):
        raise AdmissionError("authority bytes digest mismatch")
    fragment = _authority_fragment(
        data,
        stage,
        parsed.get("stable_fragment"),
    )
    if stage == "planner_authority" and len(fragment) != parsed["stable_fragment_size"]:
        raise AdmissionError("planner authority stable fragment size mismatch")
    if _digest(fragment) != _require_digest(parsed["stable_fragment_sha256"], "stable fragment"):
        raise AdmissionError("authority stable fragment digest mismatch")
    if stage == "normative_source":
        contract_fragment = _authority_fragment(
            data,
            stage,
            parsed.get("authority_contract_fragment"),
        )
        if _digest(contract_fragment) != _require_digest(
            parsed["authority_contract_fragment_sha256"], "staged-authority fragment"
        ):
            raise AdmissionError("normative staged-authority fragment digest mismatch")
    else:
        test_content = gh_api(
            f"repos/{repository}/contents/{parsed['focused_test_path']}?ref={parsed['merged_commit']}"
        )
        if not isinstance(test_content, dict) or test_content.get("path") != parsed["focused_test_path"]:
            raise AdmissionError("planner focused-test contents readback is malformed")
        test_bytes = _decode_contents(test_content, "planner focused test")
        if (
            test_content.get("sha") != parsed["focused_test_blob"]
            or int(test_content.get("size") or -1) != len(test_bytes)
            or len(test_bytes) != parsed["focused_test_size"]
            or _digest(test_bytes)
            != _require_digest(parsed["focused_test_bytes_sha256"], "planner focused-test bytes")
        ):
            raise AdmissionError("planner focused-test path/blob/size/digest mismatch")
        parsed["focused_test_bytes"] = test_bytes
    parsed["authority_bytes"] = data
    parsed["authority_fragment_bytes"] = fragment
    parsed["task_created_at"] = issue.get("created_at")
    return parsed


def verify_authority_chain(
    repository: str = REPOSITORY, repo_root: Path | None = None
) -> dict[str, dict[str, Any]]:
    normative = _verify_server_readback(
        repository,
        comment=NORMATIVE_COMMENT,
        pr_number=NORMATIVE_PR,
        stage="normative_source",
        repo_root=repo_root,
    )
    planner = _verify_server_readback(
        repository,
        comment=PLANNER_COMMENT,
        pr_number=PLANNER_PR,
        stage="planner_authority",
        repo_root=repo_root,
    )
    if planner.get("predecessor_merged_commit") != normative.get("merged_commit"):
        raise AdmissionError("planner predecessor does not bind approved normative commit")
    if planner.get("predecessor_merged_tree") != normative.get("merged_tree"):
        raise AdmissionError("planner predecessor does not bind approved normative tree")
    if planner.get("predecessor_fragment") != normative.get("authority_contract_fragment"):
        raise AdmissionError("planner predecessor does not bind approved normative fragment")
    if planner.get("predecessor_fragment_sha256") != normative.get("authority_contract_fragment_sha256"):
        raise AdmissionError("planner predecessor does not bind approved normative fragment digest")
    if planner.get("predecessor_comment") != NORMATIVE_COMMENT:
        raise AdmissionError("planner predecessor normative receipt comment mismatch")
    if normative.get("authority_path") != NORMATIVE_PATH:
        raise AdmissionError("normative authority path mismatch")
    if planner.get("authority_path") != PLANNER_PATH:
        raise AdmissionError("planner authority path mismatch")
    return {"normative": normative, "planner": planner}


def _git(repo_root: Path, *args: str) -> str:
    try:
        return subprocess.check_output(
            ["git", "-C", str(repo_root), *args], text=True, stderr=subprocess.PIPE
        ).strip()
    except Exception as exc:
        raise AdmissionError(f"local git identity readback failed: {' '.join(args)}") from exc


def _git_bytes(repo_root: Path, *args: str) -> bytes:
    try:
        return subprocess.check_output(
            ["git", "-C", str(repo_root), *args], stderr=subprocess.PIPE
        )
    except Exception as exc:
        raise AdmissionError(f"local git object readback failed: {' '.join(args)}") from exc


def _recover_planner_source_object(
    repo_root: Path, source_head: str, authority_bytes: bytes
) -> None:
    """Fetch the approved planner source object through its immutable PR ref.

    A GitHub Actions merge checkout may not retain the source PR's commit
    object after squash merge and branch deletion.  The server-advertised PR
    ref is the only recovery locator: verify its exact OID first, fetch only
    that ref, then compare the resulting source-tree bytes with the already
    verified merged authority bytes.
    """
    source_ref = f"refs/pull/{PLANNER_PR}/head"
    try:
        advertised = _git_bytes(repo_root, "ls-remote", "origin", source_ref)
    except AdmissionError as exc:
        raise AdmissionError("approved planner source ref is unavailable") from exc

    records = [line.split() for line in advertised.decode("utf-8", errors="replace").splitlines() if line.strip()]
    if len(records) != 1 or len(records[0]) != 2 or records[0][1] != source_ref:
        raise AdmissionError("approved planner source ref readback is malformed or mismatched")
    advertised_oid = records[0][0]
    if OID_RE.fullmatch(advertised_oid) is None or advertised_oid != source_head:
        raise AdmissionError("approved planner source advertised OID mismatch")

    try:
        _git_bytes(repo_root, "fetch", "--no-write-fetch-head", "--no-tags", "origin", source_ref)
    except AdmissionError as exc:
        raise AdmissionError("approved planner source ref fetch failed") from exc
    try:
        fetched = _git_bytes(repo_root, "show", f"{source_head}:{PLANNER_PATH}")
    except AdmissionError as exc:
        raise AdmissionError("approved planner source object is unavailable after fetch") from exc
    if fetched != authority_bytes:
        raise AdmissionError("approved planner source-head bytes differ from merged authority")


def verify_executing_planner(
    planner_path: Path, authority: dict[str, Any], repo_root: Path
) -> dict[str, Any]:
    if authority.get("authority_path") not in (None, PLANNER_PATH):
        raise AdmissionError("planner authority path mismatch")
    try:
        data = planner_path.read_bytes()
    except OSError as exc:
        raise AdmissionError("executing approved planner bytes are missing") from exc
    expected = authority.get("authority_bytes")
    if not isinstance(expected, bytes):
        raise AdmissionError("live planner authority bytes are unavailable")
    expected_digest = _require_digest(authority.get("authority_bytes_sha256"), "planner bytes")
    if len(data) != int(authority.get("authority_size") or -1) or _digest(data) != expected_digest:
        raise AdmissionError("executing planner bytes are dirty or do not match authority")
    if data != expected:
        raise AdmissionError("executing planner bytes differ from live merged authority")
    merged_commit = _require_oid(authority.get("merged_commit"), "planner merged commit")
    try:
        merged_bytes = subprocess.check_output(
            ["git", "-C", str(repo_root), "show", f"{merged_commit}:{PLANNER_PATH}"],
            stderr=subprocess.PIPE,
        )
    except Exception as exc:
        raise AdmissionError("merged planner authority object is unavailable locally") from exc
    if merged_bytes != data:
        raise AdmissionError("executing planner differs from local merged authority object")
    source_head = authority.get("source_head")
    if isinstance(source_head, str) and OID_RE.fullmatch(source_head):
        try:
            source_bytes = subprocess.check_output(
                ["git", "-C", str(repo_root), "show", f"{source_head}:{PLANNER_PATH}"],
                stderr=subprocess.PIPE,
            )
        except Exception as exc:
            _recover_planner_source_object(repo_root, source_head, expected)
            source_bytes = expected
        if source_bytes != data:
            raise AdmissionError("executing planner does not match approved source head")
    return {
        "path": str(planner_path),
        "authority_path": PLANNER_PATH,
        "merged_commit": merged_commit,
        "source_head": source_head,
        "source_ref": f"refs/pull/{PLANNER_PR}/head",
        "bytes_sha256": expected_digest,
        "size": len(data),
    }


def _pr_files(repository: str, pr_number: int) -> list[dict[str, Any]]:
    files: list[dict[str, Any]] = []
    for page in range(1, 101):
        response = gh_api(f"repos/{repository}/pulls/{pr_number}/files?per_page=100&page={page}")
        if not isinstance(response, list):
            raise AdmissionError("checker PR changed-file readback is malformed")
        if any(not isinstance(item, dict) for item in response):
            raise AdmissionError("checker PR changed-file record is malformed")
        files.extend(response)
        if len(response) < 100:
            return files
    raise AdmissionError("checker PR changed-file pagination overflow")


def _parse_live_time(value: Any, field: str) -> dt.datetime:
    if not isinstance(value, str) or not value:
        raise AdmissionError(f"live {field} timestamp is unavailable")
    try:
        parsed = dt.datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise AdmissionError(f"live {field} timestamp is malformed") from exc
    if parsed.tzinfo is None or parsed.utcoffset() is None:
        raise AdmissionError(f"live {field} timestamp lacks a timezone")
    return parsed


def verify_checker_pr(
    repository: str,
    pr_number: int,
    task_uid: str | None,
    base_oid: str,
    head_oid: str,
    scope_base_oid: str,
    tested_tree: str,
    repo_root: Path,
) -> dict[str, Any]:
    if repository != REPOSITORY:
        raise AdmissionError("checker PR repository is not canonical")
    if pr_number != CHECKER_PR:
        raise AdmissionError("checker PR number differs from frozen successor authority")
    _require_oid(base_oid, "checker PR base")
    _require_oid(head_oid, "checker PR head")
    _require_oid(scope_base_oid, "checker scope base")
    _require_oid(tested_tree, "checker tested tree")
    pull = gh_api(f"repos/{repository}/pulls/{pr_number}")
    if pull.get("state") != "open" or pull.get("merged") is True:
        raise AdmissionError("checker PR is not open and unmerged")
    base = pull.get("base") or {}
    head = pull.get("head") or {}
    if base.get("ref") != DEFAULT_BRANCH or base.get("sha") != base_oid:
        raise AdmissionError("checker PR base identity mismatch")
    if head.get("sha") != head_oid:
        raise AdmissionError("checker PR head identity mismatch")
    if (base.get("repo") or {}).get("full_name") != repository:
        raise AdmissionError("checker PR base repository mismatch")
    if (head.get("repo") or {}).get("full_name") != repository:
        raise AdmissionError("checker PR head repository mismatch")
    verify_checker_task_binding(repository, pr_number, task_uid, base_oid, head_oid, pull)
    body = pull.get("body") or ""
    matches = re.findall(r"^Task: (task_[0-9a-f]{32})$", body, re.MULTILINE)
    if not isinstance(task_uid, str) or not task_uid.startswith("task_") or len(matches) != 1 or matches[0] != task_uid:
        raise AdmissionError("checker PR task identity mismatch")
    changed_files = _pr_files(repository, pr_number)
    if any(item.get("status") != "modified" or item.get("previous_filename") for item in changed_files):
        raise AdmissionError("checker PR contains rename/copy or non-modification paths")
    changed = sorted(str(item.get("filename") or "") for item in changed_files)
    if changed != sorted(CHECKER_SCOPE):
        raise AdmissionError("checker PR write scope is not the exact checker-only scope")
    local_scope = _git(repo_root, "merge-base", base_oid, head_oid)
    if local_scope != scope_base_oid:
        raise AdmissionError("checker PR scope-base identity mismatch")
    local_tree = _git(repo_root, "merge-tree", "--write-tree", base_oid, head_oid)
    if local_tree != tested_tree:
        raise AdmissionError("checker PR tested-tree identity mismatch")
    return {
        "repository": repository,
        "pr_number": pr_number,
        "task_uid": matches[0],
        "base_oid": base_oid,
        "head_oid": head_oid,
        "scope_base_oid": scope_base_oid,
        "tested_tree": tested_tree,
        "changed_paths": changed,
    }


def _read_checker_task_binding(repository: str) -> dict[str, Any]:
    if repository != REPOSITORY:
        raise AdmissionError("checker task repository is not canonical")
    issue = gh_api(f"repos/{repository}/issues/{CHECKER_ISSUE}")
    if (
        not isinstance(issue, dict)
        or issue.get("number") != CHECKER_ISSUE
        or issue.get("state") != "open"
        or issue.get("repository_url") != f"https://api.github.com/repos/{repository}"
    ):
        raise AdmissionError("checker task Issue readback is unavailable")
    body = issue.get("body")
    if not isinstance(body, str):
        raise AdmissionError("checker task Issue body is unavailable")
    body = body.replace("\r\n", "\n")
    uid_lines = re.findall(r"(?m)^task_uid:[^\n]*$", body)
    if uid_lines != [f"task_uid: {CHECKER_TASK_UID}"]:
        raise AdmissionError("checker task Issue UID binding mismatch")
    references = re.findall(
        rf"https://github\.com/{re.escape(repository)}/pull/(\d+)\b", body
    )
    references.extend(re.findall(r"(?m)^-?\s*pr_number:\s*`?(\d+)`?\s*$", body))
    unique = sorted(set(int(value) for value in references))
    if not unique:
        raise AdmissionError("checker task reciprocal PR binding is unavailable")
    if len(unique) != 1:
        raise AdmissionError("checker task reciprocal PR binding is ambiguous")
    return {
        "issue_number": CHECKER_ISSUE,
        "task_uid": CHECKER_TASK_UID,
        "pr_number": unique[0],
        "issue_created_at": issue.get("created_at"),
    }


def _verify_live_checker_task_pr(
    repository: str,
    pr_number: int,
    task_uid: str | None,
    base_oid: str,
    head_oid: str,
    binding: dict[str, Any],
    pull: dict[str, Any],
) -> None:
    if pull.get("number") != binding["pr_number"] or pr_number != binding["pr_number"]:
        raise AdmissionError("checker task reciprocal PR identity mismatch")
    if task_uid != binding["task_uid"]:
        raise AdmissionError("checker task UID is not the trusted Issue UID")
    if pull.get("state") != "open" or pull.get("merged") is True:
        raise AdmissionError("checker task reciprocal PR is not open and unmerged")
    base = pull.get("base") or {}
    head = pull.get("head") or {}
    if base.get("ref") != DEFAULT_BRANCH or base.get("sha") != base_oid:
        raise AdmissionError("checker PR base identity mismatch")
    if head.get("sha") != head_oid:
        raise AdmissionError("checker PR head identity mismatch")
    if (base.get("repo") or {}).get("full_name") != repository:
        raise AdmissionError("checker PR base repository mismatch")
    if (head.get("repo") or {}).get("full_name") != repository:
        raise AdmissionError("checker PR head repository mismatch")
    body = pull.get("body")
    if not isinstance(body, str):
        raise AdmissionError("checker PR reciprocal task references are unavailable")
    task_lines = re.findall(r"(?m)^Task:[^\n]*$", body)
    if task_lines != [f"Task: {CHECKER_TASK_UID}"]:
        raise AdmissionError("checker PR task UID reference is missing or ambiguous")
    issue_refs = re.findall(r"(?m)^Refs #(\d+)\s*$", body)
    if issue_refs != [str(CHECKER_ISSUE)]:
        raise AdmissionError("checker PR reciprocal Issue reference is missing or ambiguous")
    issue_created = _parse_live_time(binding.get("issue_created_at"), "checker task Issue")
    pr_created = _parse_live_time(pull.get("created_at"), "checker PR")
    if issue_created >= pr_created:
        raise AdmissionError("checker task Issue must predate its reciprocal PR")
    changed_files = _pr_files(repository, pr_number)
    if any(item.get("status") != "modified" or item.get("previous_filename") for item in changed_files):
        raise AdmissionError("checker PR contains rename/copy or non-modification paths")
    changed_paths = sorted(str(item.get("filename") or "") for item in changed_files)
    if changed_paths != sorted(CHECKER_SCOPE):
        raise AdmissionError("checker PR changed paths are not the exact checker-only scope")


def classify_checker_stage(
    repository: str,
    pr_number: int,
    task_uid: str | None,
    base_oid: str,
    head_oid: str,
) -> bool:
    """Return whether the exact trusted checker-stage route is selected.

    Ordinary PRs must retain the conservative path.  The checker PR number is
    frozen by the merged planner authority.  Only that suspected PR causes a
    live successor-Issue read; an unavailable task Issue must not fail ordinary
    PRs that cannot select the exception.
    """
    _require_oid(base_oid, "checker PR base")
    _require_oid(head_oid, "checker PR head")
    if repository != REPOSITORY:
        raise AdmissionError("checker task repository is not canonical")
    if pr_number != CHECKER_PR:
        return False
    binding = _read_checker_task_binding(repository)
    if pr_number != binding["pr_number"]:
        raise AdmissionError("checker task reciprocal PR identity differs from frozen authority")
    if task_uid != CHECKER_TASK_UID:
        raise AdmissionError("checker stage task identity is not the trusted task")
    pull = gh_api(f"repos/{repository}/pulls/{pr_number}")
    if not isinstance(pull, dict):
        raise AdmissionError("checker task reciprocal PR readback is unavailable")
    _verify_live_checker_task_pr(repository, pr_number, task_uid, base_oid, head_oid, binding, pull)
    return True


def verify_checker_task_binding(
    repository: str,
    pr_number: int,
    task_uid: str | None,
    base_oid: str,
    head_oid: str,
    pull: dict[str, Any],
) -> None:
    """Bind the candidate to the frozen successor Issue and reciprocal PR."""
    binding = _read_checker_task_binding(repository)
    _verify_live_checker_task_pr(repository, pr_number, task_uid, base_oid, head_oid, binding, pull)


def _serializable_authority(authority: dict[str, Any]) -> dict[str, Any]:
    return {key: value for key, value in authority.items() if not isinstance(value, bytes)}


def build_preflight(
    *,
    repository: str,
    pr_number: int,
    task_uid: str | None,
    base_oid: str,
    head_oid: str,
    scope_base_oid: str,
    repo_root: Path,
    planner_path: Path,
    checker_path: Path,
    policy_path: Path,
    primary_package: str,
    run_id: str,
    run_attempt: str,
    workflow_ref: str = "",
    workflow_sha: str = "",
) -> dict[str, Any]:
    if not isinstance(task_uid, str) or not task_uid.startswith("task_"):
        raise AdmissionError("current checker task UID is required")
    if not classify_checker_stage(repository, pr_number, task_uid, base_oid, head_oid):
        raise AdmissionError("checker stage route is not the trusted exact-stage PR")
    authorities = verify_authority_chain(repository, repo_root)
    planner = verify_executing_planner(planner_path, authorities["planner"], repo_root)
    tested_tree = _git(repo_root, "merge-tree", "--write-tree", base_oid, head_oid)
    checker = verify_checker_pr(
        repository, pr_number, task_uid, base_oid, head_oid, scope_base_oid, tested_tree, repo_root
    )
    checker_command = [
        sys.executable,
        str(checker_path),
        "--repo-root",
        str(repo_root),
        "--base",
        scope_base_oid,
        "--head",
        head_oid,
        "--primary-package",
        primary_package,
        "--policy",
        str(policy_path),
        "--json",
    ]
    _require_oid(base_oid, "checker PR base")
    _require_oid(head_oid, "checker PR head")
    identity = {
        "run_id": str(run_id),
        "run_attempt": str(run_attempt),
        "workflow_ref": workflow_ref,
        "workflow_sha": workflow_sha,
    }
    if not identity["run_id"] or not identity["run_attempt"]:
        raise AdmissionError("trusted runner identity is incomplete")
    receipt: dict[str, Any] = {
        "schema": SCHEMA,
        "phase": "preflight",
        "repository": repository,
        "default_branch": DEFAULT_BRANCH,
        "task_uid": checker["task_uid"],
        "pr_number": pr_number,
        "base_oid": base_oid,
        "head_oid": head_oid,
        "scope_base_oid": scope_base_oid,
        "tested_tree": tested_tree,
        "changed_paths": checker["changed_paths"],
        "checker_command": checker_command,
        "checker_command_digest": command_digest(checker_command),
        "policy_path": str(policy_path),
        "planner": planner,
        "normative_authority": _serializable_authority(authorities["normative"]),
        "planner_authority": _serializable_authority(authorities["planner"]),
        "runner": identity,
    }
    receipt["preflight_digest"] = preflight_digest(receipt)
    return receipt


def verify_postrun(
    preflight: dict[str, Any],
    expected_digest: str,
    *,
    status: str | None,
    exit_code: int | None,
    run_id: str,
    run_attempt: str,
) -> dict[str, Any]:
    if preflight.get("schema") != SCHEMA or preflight.get("phase") != "preflight":
        raise AdmissionError("preflight receipt schema/phase is invalid")
    if expected_digest != preflight_digest(preflight):
        raise AdmissionError("preflight receipt digest mismatch")
    if status != "passed" or exit_code != 0:
        raise AdmissionError("checker command result is missing, failed, or nonzero")
    runner = preflight.get("runner") or {}
    if str(run_id) != str(runner.get("run_id")) or str(run_attempt) != str(runner.get("run_attempt")):
        raise AdmissionError("post-run runner identity mismatch")
    for field in ("base_oid", "head_oid", "scope_base_oid", "tested_tree"):
        _require_oid(preflight.get(field), f"post-run {field}")
    return {
        "schema": SCHEMA,
        "phase": "post_run",
        "repository": preflight.get("repository"),
        "task_uid": preflight.get("task_uid"),
        "pr_number": preflight.get("pr_number"),
        "base_oid": preflight.get("base_oid"),
        "head_oid": preflight.get("head_oid"),
        "scope_base_oid": preflight.get("scope_base_oid"),
        "tested_tree": preflight.get("tested_tree"),
        "checker_command_digest": preflight.get("checker_command_digest"),
        "preflight_digest": expected_digest,
        "result_status": status,
        "exit_code": exit_code,
        "runner": runner,
    }


def build_postrun_receipt(
    preflight: dict[str, Any],
    authorities: dict[str, dict[str, Any]],
    executing_planner: dict[str, Any],
    check: dict[str, Any],
    *,
    status: str,
    exit_code: int,
) -> dict[str, Any]:
    """Create the durable, complete post-run evidence artifact."""
    _assert_preflight_authorities_unchanged(preflight, authorities)
    runner = preflight.get("runner") or {}
    receipt = verify_postrun(
        preflight,
        preflight_digest(preflight),
        status=status,
        exit_code=exit_code,
        run_id=str(runner.get("run_id") or ""),
        run_attempt=str(runner.get("run_attempt") or ""),
    )
    receipt.update(
        {
            "activation": "provisional",
            "normative_authority": _serializable_authority(authorities["normative"]),
            "planner_authority": _serializable_authority(authorities["planner"]),
            "executing_planner": executing_planner,
            "check": check,
            "result": {
                "status": status,
                "exit_code": exit_code,
                "command": preflight.get("checker_command"),
                "command_digest": preflight.get("checker_command_digest"),
                "base_oid": preflight.get("base_oid"),
                "head_oid": preflight.get("head_oid"),
                "scope_base_oid": preflight.get("scope_base_oid"),
                "tested_tree": preflight.get("tested_tree"),
            },
        }
    )
    receipt["receipt_digest"] = durable_receipt_digest(receipt)
    verify_durable_postrun_receipt(receipt)
    return receipt


def _assert_preflight_authorities_unchanged(
    preflight: dict[str, Any], authorities: dict[str, dict[str, Any]]
) -> None:
    for field, expected in (
        ("normative_authority", NORMATIVE_AUTHORITY_EXPECTED),
        ("planner_authority", PLANNER_AUTHORITY_EXPECTED),
    ):
        before = preflight.get(field)
        after = authorities.get("normative" if field == "normative_authority" else "planner")
        if not isinstance(before, dict) or not isinstance(after, dict):
            raise AdmissionError(f"post-run {field} readback is missing")
        if any(before.get(name) != after.get(name) for name in expected):
            raise AdmissionError(f"post-run {field} identity changed after preflight")


def verify_durable_postrun_receipt(receipt: dict[str, Any]) -> dict[str, Any]:
    """Validate the durable artifact before a workflow uploads it."""
    if receipt.get("schema") != SCHEMA or receipt.get("phase") != "post_run":
        raise AdmissionError("durable post-run receipt schema/phase is invalid")
    if receipt.get("activation") != "provisional":
        raise AdmissionError("durable post-run receipt is not marked provisional")
    if receipt.get("receipt_digest") != durable_receipt_digest(receipt):
        raise AdmissionError("durable post-run receipt digest mismatch")
    for field in ("normative_authority", "planner_authority", "executing_planner", "check", "result"):
        if not isinstance(receipt.get(field), dict):
            raise AdmissionError(f"durable post-run receipt is missing {field}")
    result = receipt["result"]
    if result.get("status") != "passed" or result.get("exit_code") != 0:
        raise AdmissionError("durable post-run result is not successful")
    check = receipt["check"]
    if not isinstance(check.get("check_app_id"), int) or not isinstance(check.get("check_run_id"), int):
        raise AdmissionError("durable post-run check identity is incomplete")
    _require_oid(check.get("check_head"), "durable check head")
    if str(check.get("workflow_run_id")) != str((receipt.get("runner") or {}).get("run_id")):
        raise AdmissionError("durable post-run workflow run identity mismatch")
    if not isinstance(result.get("command"), list) or command_digest(result["command"]) != result.get("command_digest"):
        raise AdmissionError("durable post-run command argv/digest is invalid")
    if result.get("command_digest") != receipt.get("checker_command_digest"):
        raise AdmissionError("durable post-run command identity mismatch")
    for field in ("base_oid", "head_oid", "scope_base_oid", "tested_tree"):
        if result.get(field) != receipt.get(field):
            raise AdmissionError(f"durable post-run result {field} mismatch")
    planner = receipt["executing_planner"]
    _require_oid(planner.get("merged_commit"), "durable planner merged commit")
    _require_oid(planner.get("source_head"), "durable planner source head")
    _require_digest(planner.get("bytes_sha256"), "durable planner bytes")
    for field in ("base_oid", "head_oid", "scope_base_oid", "tested_tree"):
        _require_oid(receipt.get(field), f"durable {field}")
    return receipt


def verify_live_check_identity(
    repository: str, check_head: str, run_id: str, check_name: str = "required-gate"
) -> dict[str, Any]:
    """Bind the post-run receipt to the current GitHub Actions check app."""
    _require_oid(check_head, "check head")
    if not str(run_id).isdigit() or int(run_id) <= 0:
        raise AdmissionError("check run identity is missing")
    response = gh_api(f"repos/{repository}/commits/{check_head}/check-runs?per_page=100")
    if not isinstance(response, dict) or not isinstance(response.get("check_runs"), list):
        raise AdmissionError("live check-run readback is malformed")
    run_path = re.compile(rf"/actions/runs/{re.escape(str(run_id))}(?:/|$)")
    matches = [
        item
        for item in response["check_runs"]
        if item.get("name") == check_name
        and run_path.search(str(item.get("details_url") or "")) is not None
    ]
    if len(matches) != 1:
        raise AdmissionError("live check-run/app identity is missing or ambiguous")
    check = matches[0]
    app = check.get("app") or {}
    app_id = app.get("id")
    if app_id != GITHUB_ACTIONS_APP_ID:
        raise AdmissionError("live check app identity mismatch")
    check_run_id = check.get("id")
    if not isinstance(check_run_id, int) or check_run_id <= 0:
        raise AdmissionError("live check-run identity is missing")
    if check.get("head_sha") != check_head:
        raise AdmissionError("live check-run head identity mismatch")
    return {
        "check_name": check_name,
        "check_run_id": check_run_id,
        "check_app_id": app_id,
        "check_app_slug": app.get("slug"),
        "check_head": check_head,
        "workflow_run_id": str(run_id),
    }


def resolve_live_check_head(args: argparse.Namespace) -> str:
    """Select the check's actual head while keeping runner/workflow SHAs intact."""
    if os.environ.get("GITHUB_EVENT_NAME") != "pull_request":
        return args.check_head or args.head_oid
    source_check_head = os.environ.get("OASIS7_CARGO_STAGE_CHECK_HEAD", "")
    _require_oid(source_check_head, "source check head")
    if source_check_head != args.head_oid:
        raise AdmissionError("source check head does not match validated PR head")
    return source_check_head


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=("classify", "preflight", "post-run", "postrun"))
    parser.add_argument("--repository", default=os.environ.get("GITHUB_REPOSITORY", REPOSITORY))
    parser.add_argument("--pr-number", type=int, required=True)
    parser.add_argument("--repo-root", type=Path, default=Path("."))
    parser.add_argument("--base", dest="base_oid", required=True)
    parser.add_argument("--head", dest="head_oid", required=True)
    parser.add_argument("--scope-base", dest="scope_base_oid")
    parser.add_argument("--planner-path", type=Path)
    parser.add_argument("--checker-path", type=Path)
    parser.add_argument("--policy-path", type=Path)
    parser.add_argument("--primary-package", default="auto")
    parser.add_argument("--task-uid")
    parser.add_argument("--run-id", default=os.environ.get("GITHUB_RUN_ID", ""))
    parser.add_argument("--run-attempt", default=os.environ.get("GITHUB_RUN_ATTEMPT", ""))
    parser.add_argument("--workflow-ref", default=os.environ.get("GITHUB_WORKFLOW_REF", ""))
    parser.add_argument("--workflow-sha", default=os.environ.get("GITHUB_WORKFLOW_SHA", ""))
    parser.add_argument("--check-head", default=os.environ.get("GITHUB_SHA", ""))
    parser.add_argument("--check-name", default="required-gate")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--preflight")
    parser.add_argument("--preflight-digest")
    parser.add_argument("--result-status")
    parser.add_argument("--exit-code", type=int)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        if args.phase == "classify":
            if not args.task_uid:
                raise AdmissionError("checker-stage classification requires the candidate task UID")
            if not classify_checker_stage(
                args.repository, args.pr_number, args.task_uid, args.base_oid, args.head_oid
            ):
                raise AdmissionError("checker stage route is not the trusted successor PR")
            print(f"{args.pr_number}\t{CHECKER_TASK_UID}")
            return 0
        if args.phase == "preflight":
            if args.scope_base_oid is None:
                raise AdmissionError("preflight requires scope-base")
            missing = [
                name
                for name, value in (
                    ("planner-path", args.planner_path),
                    ("checker-path", args.checker_path),
                    ("policy-path", args.policy_path),
                )
                if value is None
            ]
            if missing:
                raise AdmissionError("preflight requires " + ", ".join(missing))
            receipt = build_preflight(
                repository=args.repository,
                pr_number=args.pr_number,
                task_uid=args.task_uid,
                base_oid=args.base_oid,
                head_oid=args.head_oid,
                scope_base_oid=args.scope_base_oid,
                repo_root=args.repo_root,
                planner_path=args.planner_path,
                checker_path=args.checker_path,
                policy_path=args.policy_path,
                primary_package=args.primary_package,
                run_id=args.run_id,
                run_attempt=args.run_attempt,
                workflow_ref=args.workflow_ref,
                workflow_sha=args.workflow_sha,
            )
        else:
            if not args.scope_base_oid or not args.preflight or args.preflight_digest is None:
                raise AdmissionError("post-run requires preflight and preflight-digest")
            preflight = json.loads(Path(args.preflight).read_text(encoding="utf-8"))
            if preflight.get("repository") != args.repository or preflight.get("pr_number") != args.pr_number:
                raise AdmissionError("post-run repository/PR identity mismatch")
            for argument, field in (
                (args.base_oid, "base_oid"),
                (args.head_oid, "head_oid"),
                (args.scope_base_oid, "scope_base_oid"),
            ):
                if preflight.get(field) != argument:
                    raise AdmissionError(f"post-run {field} identity mismatch")
            verify_postrun(
                preflight,
                args.preflight_digest,
                status=args.result_status,
                exit_code=args.exit_code,
                run_id=args.run_id,
                run_attempt=args.run_attempt,
            )
            # Re-read all mutable live identities after the command completed.
            authorities = verify_authority_chain(args.repository, args.repo_root)
            planner_path = args.planner_path
            checker_path = args.checker_path
            policy_path = args.policy_path
            if planner_path is None or checker_path is None or policy_path is None:
                raise AdmissionError("post-run requires planner, checker, and policy paths")
            executing_planner = verify_executing_planner(
                planner_path, authorities["planner"], args.repo_root
            )
            tested_tree = _git(args.repo_root, "merge-tree", "--write-tree", args.base_oid, args.head_oid)
            if tested_tree != preflight["tested_tree"]:
                raise AdmissionError("post-run tested-tree identity changed")
            expected_command = [
                sys.executable,
                str(checker_path),
                "--repo-root",
                str(args.repo_root),
                "--base",
                args.scope_base_oid,
                "--head",
                args.head_oid,
                "--primary-package",
                args.primary_package,
                "--policy",
                str(policy_path),
                "--json",
            ]
            if command_digest(expected_command) != preflight.get("checker_command_digest"):
                raise AdmissionError("post-run checker command identity mismatch")
            verify_checker_pr(
                args.repository,
                args.pr_number,
                preflight.get("task_uid"),
                args.base_oid,
                args.head_oid,
                args.scope_base_oid,
                tested_tree,
                args.repo_root,
            )
            check = verify_live_check_identity(
                args.repository, resolve_live_check_head(args), args.run_id, args.check_name
            )
            receipt = build_postrun_receipt(
                preflight,
                authorities,
                executing_planner,
                check,
                status=args.result_status,
                exit_code=args.exit_code,
            )
        rendered = json.dumps(receipt, sort_keys=True, separators=(",", ":")) + "\n"
        if args.output:
            args.output.write_text(rendered, encoding="utf-8")
        print(rendered, end="")
        return 0
    except (AdmissionError, OSError, json.JSONDecodeError) as exc:
        print(f"cargo-checker-stage-admission: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
