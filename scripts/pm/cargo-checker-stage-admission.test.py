#!/usr/bin/env python3
import base64
import hashlib
import json
import importlib.util
import io
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import zipfile


MODULE_PATH = Path(__file__).with_name("cargo_checker_stage_admission.py")
SPEC = importlib.util.spec_from_file_location("cargo_checker_stage_admission", MODULE_PATH)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)
PINNED_NORMATIVE_AUTHORITY_EXPECTED = dict(MODULE.NORMATIVE_AUTHORITY_EXPECTED)
PINNED_PLANNER_AUTHORITY_EXPECTED = dict(MODULE.PLANNER_AUTHORITY_EXPECTED)


REPOSITORY = "eng-cc/oasis7"
NORMATIVE_COMMIT = "a" * 40
NORMATIVE_TREE = "b" * 40
NORMATIVE_BLOB = "c" * 40
PLANNER_COMMIT = "d" * 40
PLANNER_TREE = "e" * 40
PLANNER_BLOB = "f" * 40
PLANNER_SOURCE_HEAD = "88988102318fa2e3c9e84f483d01cb99575e5387"
PLANNER_SOURCE_REF = f"refs/pull/{MODULE.PLANNER_PR}/head"
CHECKER_BASE = "1" * 40
CHECKER_HEAD = "2" * 40
CHECKER_SCOPE = "3" * 40
TESTED_TREE = "4" * 40
TASK_UID = "task_4a631678a50b4fcb952a3c2778b15677"
SUCCESSOR_ISSUE = 3971
CHECKER_PR = 3972
INTEGRATION_RUN = MODULE.PLANNER_INTEGRATION_RUN
PLANNER_REQUIRED_RUN = 36162285257
CHECK_RUN_ID = 123

CURRENT_PLANNER_RECEIPT = (
    "Post-merge approved_planner_authority live server readback for ordered C1 admission "
    "(stage=planner_authority; immutable merged authority, not candidate self-approval): "
    "repository=eng-cc/oasis7; default_branch=main; "
    "task_uid=task_978dcc0005b9415cbc45e59b21b095e0; PR=4000; "
    "source_scope_base=b5dbf139f1f608cde4797eea3218d1f7bb1eaf9e; "
    "source_head=930217dbc32a033359b41aa0aeff1bfe07ccb6dd; "
    "trusted integration base=a5b8da54d40f5626106b78efb19eaec4976a9452. "
    "GitHub live PR readback: MERGED, merge commit=9a27e39fcc714435af65777ddfd9e83e4860158d; "
    "live git/commits API tree=8a99ec25e5cdaa62de0008ec08247faca9a84b46. "
    "Live contents API at that exact commit: authority path=scripts/pm/cargo_package_profile_planner.py, "
    "blob=7f594b7fe8495689dae011dd993803e4c743d8dc, size=55453, decoded bytes "
    "sha256=7c8a2de3e4b9b5e4c752e5147438bfe0eb80edb7a4406bc8d2b59aff498774fc; "
    "focused test path=scripts/pm/cargo-package-profile-planner.test.py, "
    "blob=d800de58b8b043887a3ae3290e95355b79b12a97, size=67265, decoded bytes "
    "sha256=d3d38d31f9b339061ab5807ed2ed1e0a10c76ab877c8a34bbfb91814dd8e6725. "
    "Stable planner fragment APPROVED_NORMATIVE_EXPECTED from its assignment through closing brace "
    "plus final LF: 1222 bytes, sha256=63a0a1735d83d5b9431c1ecd07aa54b4437e84128dcf5cf821591489dfa4f8bb. "
    "Predecessor approved normative source: #3996/PR #3997 receipt "
    "https://github.com/eng-cc/oasis7/issues/3996#issuecomment-5822424913, "
    "merged commit=4f97540c34aefca41d8bf790cbf09b3176f07de3, "
    "tree=82b8e5ad75d44c48739165b8bd26b5896eaa783a, normative fragment "
    "cargo-checker-authority-upgrade sha256=16c3593deb204c7e38ae551ac553075abd24fd81e5f09d6ac9af3b0a38a9f45a. "
    "Exact-head required-gate and strict integration passed, formal RH/QA r8 review closed no findings, "
    "PR live gate ready, and terminal cleanup receipt "
    "https://github.com/eng-cc/oasis7/issues/3999#issuecomment-5836399698. "
    "C1 must independently re-read and verify repository, branch, identity, commit, tree, paths, fragment, "
    "digests, stage and predecessor before admission; this receipt does not authorize bypassing its own review or checks."
)

CURRENT_NORMATIVE_RECEIPT = (
    "Post-merge approved_normative_source readback (stage=normative_source; immutable, not candidate authority): "
    "repository=eng-cc/oasis7; default_branch=main; task_uid=task_a14e1a1d51a44519ab0aa94632d90df4; "
    "PR=3997; trusted_predecessor/source_scope_base=747750afc6788d8a16421f84099edae8ab3e8520; "
    "source_head=6dcc49fc1a3a726bc09ab88a96ac3065a7535bf0; "
    "predecessor authority path=doc/engineering/workflow/source-of-truth.md; "
    "predecessor file sha256=6af4fbc942407cb8c44bdc383793062d1463b62ffc3fbeedbc81f79f36324ed5. "
    "GitHub live PR readback reports MERGED into commit=4f97540c34aefca41d8bf790cbf09b3176f07de3; "
    "live git/commits API tree=82b8e5ad75d44c48739165b8bd26b5896eaa783a; "
    "live contents API path=doc/engineering/workflow/source-of-truth.md "
    "blob=b6d1c5fa854bde6f63d14db1cfd31eb5f8a23705, size=159647, decoded bytes "
    "sha256=7ffc099266de2affb9b9726468edc45b91af55cb531537749ef8361407f4902e. "
    "Stable fragment anchor=cargo-package-scope-and-impact-scoped-verification, "
    "sha256 including final LF=03bb8e833ed707662267633eec3747b46eaa698765a89e6a44d3c3fa679bbae0. "
    "Adjacent staged-authority fragment anchor=cargo-checker-authority-upgrade, "
    "sha256 including final LF=16c3593deb204c7e38ae551ac553075abd24fd81e5f09d6ac9af3b0a38a9f45a. "
    "Live readbacks were from merged commit APIs, not the candidate worktree; N1 role review and exact-target required-gate passed. "
    "This receipt may be consumed only by the ordered P1 planner stage, not as checker self-approval."
)


def _authority_api():
    normative = _normative_bytes()
    planner = _planner_bytes()
    bodies = {
        MODULE.NORMATIVE_COMMENT: _comment(
            "normative_source", NORMATIVE_COMMIT, NORMATIVE_TREE, NORMATIVE_BLOB,
            normative, "task_7bbce5924333467f9edfae48e1c73889", 3815,
            MODULE.NORMATIVE_COMMENT, normative,
        ),
        MODULE.PLANNER_COMMENT: _comment(
            "planner_authority", PLANNER_COMMIT, PLANNER_TREE, PLANNER_BLOB,
            planner, "task_e21604f5cdb3476c8e146332a68a05b4", 3821,
            MODULE.PLANNER_COMMENT, planner[:planner.index(b"\ndef _extract") + 1],
        ),
    }
    commits = {
        NORMATIVE_COMMIT: {"sha": NORMATIVE_COMMIT, "commit": {"tree": {"sha": NORMATIVE_TREE}}},
        PLANNER_COMMIT: {"sha": PLANNER_COMMIT, "commit": {"tree": {"sha": PLANNER_TREE}}},
    }
    contents = {
        NORMATIVE_COMMIT: _content(normative, NORMATIVE_BLOB),
        PLANNER_COMMIT: _content(planner, PLANNER_BLOB),
    }
    prs = {
        3815: {"number": 3815, "state": "closed", "merged": True,
               "merge_commit_sha": NORMATIVE_COMMIT,
               "base": {"ref": "main", "sha": CHECKER_BASE, "repo": {"full_name": REPOSITORY}},
               "head": {"sha": "7" * 40, "repo": {"full_name": REPOSITORY}}},
        3821: {"number": 3821, "state": "closed", "merged": True,
               "merge_commit_sha": PLANNER_COMMIT,
               "base": {"ref": "main", "sha": CHECKER_BASE, "repo": {"full_name": REPOSITORY}},
               "head": {"sha": "8" * 40, "repo": {"full_name": REPOSITORY}}},
        CHECKER_PR: {"number": CHECKER_PR, "state": "open", "merged": False, "draft": True,
               "created_at": "2026-09-24T11:53:53Z",
               "body": f"Task: {TASK_UID}\n\nRefs #{SUCCESSOR_ISSUE}\n", "base": {"ref": "main", "sha": CHECKER_BASE,
               "repo": {"full_name": REPOSITORY}}, "head": {"sha": CHECKER_HEAD,
               "repo": {"full_name": REPOSITORY}},},
    }
    def api(path):
        if path.endswith(f"issues/{SUCCESSOR_ISSUE}"):
            return {
                "number": SUCCESSOR_ISSUE,
                "state": "open",
                "repository_url": f"https://api.github.com/repos/{REPOSITORY}",
                "created_at": "2026-09-24T11:42:55Z",
                "body": (
                    "<!-- oasis7-pm-task -->\n"
                    f"task_uid: {TASK_UID}\n"
                    f"- pr_url: `https://github.com/eng-cc/oasis7/pull/{CHECKER_PR}`\n"
                    f"- pr_number: `{CHECKER_PR}`\n"
                ),
            }
        if path.endswith(f"issues/comments/{MODULE.NORMATIVE_COMMENT}"):
            return {"body": bodies[MODULE.NORMATIVE_COMMENT]}
        if path.endswith(f"issues/comments/{MODULE.PLANNER_COMMENT}"):
            return {"body": bodies[MODULE.PLANNER_COMMENT]}
        if "/compare/" in path:
            return {"merge_base_commit": {"sha": CHECKER_SCOPE}}
        if "/actions/runs/" in path:
            return {
                "id": 35463292968,
                "run_attempt": 1,
                "status": "completed",
                "conclusion": "success",
                "event": "workflow_dispatch",
                "head_branch": "main",
                "head_sha": CHECKER_BASE,
                "display_title": (
                    "oasis7-ci|workflow_dispatch|integration_revalidation|"
                    "task_e21604f5cdb3476c8e146332a68a05b4|3821|"
                    + CHECKER_BASE + "|" + "8" * 40
                ),
            }
        if "/check-runs?" in path:
            return {"check_runs": [{
                "id": CHECK_RUN_ID, "name": "required-gate", "head_sha": CHECKER_BASE,
                "details_url": f"https://github.com/{REPOSITORY}/actions/runs/{INTEGRATION_RUN}/job/1",
                "status": "completed", "conclusion": "success",
                "app": {"id": 15368, "slug": "github-actions"},
            }]}
        if "/commits/" in path:
            return commits[path.rsplit("/", 1)[1]]
        if "/contents/" in path:
            commit = path.split("?ref=", 1)[1]
            return contents[commit]
        if path.endswith(f"/pulls/{CHECKER_PR}/files?per_page=100&page=1"):
            return [{"filename": value, "status": "modified"} for value in MODULE.CHECKER_SCOPE]
        if "/pulls/" in path:
            return prs[int(path.rsplit("/", 1)[1])]
        if path == f"repos/{REPOSITORY}":
            return {"default_branch": "main"}
        raise AssertionError(path)
    return api


# These fixtures model the current immutable #3996/#3999 authority chain. Keep
# receipt bytes in the current production grammar; legacy #3821 bodies above
# are intentionally replaced at module load by the helpers below.
def _sha256(data):
    return "sha256:" + hashlib.sha256(data).hexdigest()


def _normative_bytes():
    return (
        b'<a id="cargo-package-scope-and-impact-scoped-verification"></a> scope contract\n'
        b'<a id="cargo-checker-authority-upgrade"></a> staged authority contract\n'
    )


def _planner_bytes():
    return (
        b"APPROVED_NORMATIVE_EXPECTED = {\n"
        b"    'stage': 'approved_normative_source',\n"
        b"}\n\n"
        b"def _next():\n"
        b"    return None\n"
    )


def _planner_test_bytes():
    return b"# focused planner authority test fixture\n"


def _fixture_authority_expectations():
    normative = _normative_bytes()
    normative_lines = normative.splitlines(keepends=True)
    planner = _planner_bytes()
    planner_test = _planner_test_bytes()
    planner_fragment = planner[:planner.index(b"}\n") + 2]
    normative_expected = {
        **MODULE.NORMATIVE_AUTHORITY_EXPECTED,
        "source_head": "7" * 40,
        "source_scope_base": "9" * 40,
        "predecessor_file_sha256": "sha256:" + "8" * 64,
        "merged_commit": NORMATIVE_COMMIT,
        "merged_tree": NORMATIVE_TREE,
        "authority_blob": NORMATIVE_BLOB,
        "authority_size": len(normative),
        "authority_bytes_sha256": _sha256(normative),
        "stable_fragment_sha256": _sha256(normative_lines[0]),
        "authority_contract_fragment_sha256": _sha256(normative_lines[1]),
    }
    planner_expected = {
        **MODULE.PLANNER_AUTHORITY_EXPECTED,
        "source_head": PLANNER_SOURCE_HEAD,
        "source_scope_base": CHECKER_SCOPE,
        "trusted_integration_base": CHECKER_BASE,
        "trusted_integration_tested_tree": TESTED_TREE,
        "merged_commit": PLANNER_COMMIT,
        "merged_tree": PLANNER_TREE,
        "authority_blob": PLANNER_BLOB,
        "authority_size": len(planner),
        "authority_bytes_sha256": _sha256(planner),
        "focused_test_blob": "5" * 40,
        "focused_test_size": len(planner_test),
        "focused_test_bytes_sha256": _sha256(planner_test),
        "stable_fragment_size": len(planner_fragment),
        "stable_fragment_sha256": _sha256(planner_fragment),
        "predecessor_merged_commit": NORMATIVE_COMMIT,
        "predecessor_merged_tree": NORMATIVE_TREE,
        "predecessor_fragment_sha256": _sha256(normative_lines[1]),
    }
    return normative_expected, planner_expected


def _content(data, oid, path):
    return {
        "path": path,
        "encoding": "base64",
        "content": base64.b64encode(data).decode() + "\n",
        "sha": oid,
        "size": len(data),
    }


def _profile_artifacts():
    plan = {
        "schema": "oasis7-cargo-package-profile-plan/v1",
        "plan_id": "sha256:" + "1" * 64,
        "integration_base": CHECKER_BASE,
        "source_head": PLANNER_SOURCE_HEAD,
        "tested_tree": TESTED_TREE,
    }
    results = [{
        "status": "passed", "exit_code": 0,
        "integration_base": CHECKER_BASE,
        "source_head": PLANNER_SOURCE_HEAD,
        "tested_tree": TESTED_TREE,
    }]
    receipt = {
        "status": "passed", "plan_id": plan["plan_id"],
        "integration_base": CHECKER_BASE,
        "source_head": PLANNER_SOURCE_HEAD,
        "tested_tree": TESTED_TREE,
    }
    payloads = {}
    for key, value in (("plan", plan), ("results", results), ("receipt", receipt)):
        payload = json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
        payloads[key] = {"value": value, "bytes": payload}
    envelope = {
        "schema": "oasis7-cargo-package-profile-envelope/v1",
        "repository": REPOSITORY,
        "task_uid": MODULE.PLANNER_TASK_UID,
        "pr_number": MODULE.PLANNER_PR,
        "run_id": INTEGRATION_RUN,
        "run_attempt": 1,
        "check_name": "required-gate",
        "check_app_id": 15368,
        "check_run_id": CHECK_RUN_ID,
        "integration_base": CHECKER_BASE,
        "source_head": PLANNER_SOURCE_HEAD,
        "tested_tree": TESTED_TREE,
        "workflow_ref": f"{REPOSITORY}/.github/workflows/rust.yml@refs/heads/main",
        "workflow_sha": CHECKER_BASE,
        "plan_digest": _sha256(payloads["plan"]["bytes"]),
        "results_digest": _sha256(payloads["results"]["bytes"]),
        "receipt_digest": _sha256(payloads["receipt"]["bytes"]),
    }
    payloads["envelope"] = {
        "value": envelope,
        "bytes": json.dumps(envelope, sort_keys=True, separators=(",", ":")).encode(),
    }
    return payloads


def _comment_object(comment_id, issue_number, body):
    timestamp = "2026-09-25T00:00:00Z"
    return {
        "id": comment_id,
        "issue_url": f"https://api.github.com/repos/{REPOSITORY}/issues/{issue_number}",
        "html_url": f"https://github.com/{REPOSITORY}/issues/{issue_number}#issuecomment-{comment_id}",
        "user": {"login": "eng-cc"},
        "created_at": timestamp,
        "updated_at": timestamp,
        "body": body,
    }


def _normative_receipt(expected, data):
    lines = data.splitlines(keepends=True)
    return (
        "Post-merge approved_normative_source live server readback "
        "(stage=normative_source; immutable, not candidate authority): "
        f"repository={REPOSITORY}; default_branch=main; task_uid={MODULE.NORMATIVE_TASK_UID}; "
        f"PR={MODULE.NORMATIVE_PR}; trusted_predecessor/source_scope_base={expected['source_scope_base']}; "
        f"source_head={expected['source_head']}; predecessor authority path={MODULE.NORMATIVE_PATH}; "
        f"predecessor file sha256={expected['predecessor_file_sha256'].removeprefix('sha256:')}. "
        f"GitHub live PR readback: MERGED into commit={expected['merged_commit']}; "
        f"live git/commits API tree={expected['merged_tree']}. Live contents API at that exact commit: "
        f"authority path={MODULE.NORMATIVE_PATH}; blob={expected['authority_blob']}, size={len(data)}, "
        f"decoded bytes sha256={hashlib.sha256(data).hexdigest()}. "
        f"Stable fragment anchor={MODULE.NORMATIVE_FRAGMENT}, sha256 including final LF="
        f"{hashlib.sha256(lines[0]).hexdigest()}. Adjacent staged-authority fragment anchor="
        f"{MODULE.NORMATIVE_CONTRACT_FRAGMENT}, sha256 including final LF="
        f"{hashlib.sha256(lines[1]).hexdigest()}."
    )


def _planner_receipt(expected):
    data = _planner_bytes()
    test_data = _planner_test_bytes()
    fragment = data[:data.index(b"}\n") + 2]
    predecessor = MODULE.NORMATIVE_AUTHORITY_EXPECTED
    return (
        "Post-merge approved_planner_authority live server readback for ordered C1 admission "
        "(stage=planner_authority; immutable merged authority, not candidate self-approval): "
        f"repository={REPOSITORY}; default_branch=main; task_uid={MODULE.PLANNER_TASK_UID}; PR={MODULE.PLANNER_PR}; "
        f"source_scope_base={expected['source_scope_base']}; source_head={expected['source_head']}; "
        f"trusted integration base={expected['trusted_integration_base']}. GitHub live PR readback: MERGED, "
        f"merge commit={expected['merged_commit']}; live git/commits API tree={expected['merged_tree']}. "
        f"Live contents API at that exact commit: authority path={MODULE.PLANNER_PATH}, "
        f"blob={expected['authority_blob']}, size={len(data)}, decoded bytes sha256={hashlib.sha256(data).hexdigest()}; "
        f"focused test path={MODULE.PLANNER_TEST_PATH}, blob={expected['focused_test_blob']}, "
        f"size={len(test_data)}, decoded bytes sha256={hashlib.sha256(test_data).hexdigest()}. "
        "Stable planner fragment APPROVED_NORMATIVE_EXPECTED from its assignment through closing brace "
        f"plus final LF: {len(fragment)} bytes, sha256={hashlib.sha256(fragment).hexdigest()}. "
        f"Predecessor approved normative source: #{MODULE.NORMATIVE_ISSUE}/PR #{MODULE.NORMATIVE_PR} receipt "
        f"https://github.com/{REPOSITORY}/issues/{MODULE.NORMATIVE_ISSUE}#issuecomment-{MODULE.NORMATIVE_COMMENT}, "
        f"merged commit={predecessor['merged_commit']}, tree={predecessor['merged_tree']}, normative fragment "
        f"{MODULE.NORMATIVE_CONTRACT_FRAGMENT} sha256="
        f"{_sha256(_normative_bytes().splitlines(keepends=True)[1]).removeprefix('sha256:')}. "
        "Exact-head required-gate and strict integration passed, formal RH/QA r8 review closed no findings, "
        "PR live gate ready, and terminal cleanup receipt "
        f"https://github.com/{REPOSITORY}/issues/{MODULE.PLANNER_ISSUE}#issuecomment-{MODULE.PLANNER_CLEANUP_COMMENT}. "
        "C1 must independently re-read and verify repository, branch, identity, commit, tree, paths, fragment, "
        "digests, stage and predecessor before admission; this receipt does not authorize bypassing its own review or checks."
    )


def _authority_api():
    normative = _normative_bytes()
    planner = _planner_bytes()
    planner_test = _planner_test_bytes()
    normative_expected, planner_expected = _fixture_authority_expectations()
    bodies = {
        MODULE.NORMATIVE_COMMENT: _normative_receipt(normative_expected, normative),
        MODULE.PLANNER_COMMENT: _planner_receipt(planner_expected),
    }
    commits = {
        NORMATIVE_COMMIT: {"sha": NORMATIVE_COMMIT, "commit": {"tree": {"sha": NORMATIVE_TREE}}},
        PLANNER_COMMIT: {"sha": PLANNER_COMMIT, "commit": {"tree": {"sha": PLANNER_TREE}}},
    }
    contents = {
        (NORMATIVE_COMMIT, MODULE.NORMATIVE_PATH): _content(normative, NORMATIVE_BLOB, MODULE.NORMATIVE_PATH),
        (PLANNER_COMMIT, MODULE.PLANNER_PATH): _content(planner, PLANNER_BLOB, MODULE.PLANNER_PATH),
        (PLANNER_COMMIT, MODULE.PLANNER_TEST_PATH): _content(
            planner_test, planner_expected["focused_test_blob"], MODULE.PLANNER_TEST_PATH
        ),
    }
    issues = {
        MODULE.NORMATIVE_ISSUE: {
            "number": MODULE.NORMATIVE_ISSUE, "state": "closed",
            "repository_url": f"https://api.github.com/repos/{REPOSITORY}",
            "created_at": "2026-09-23T10:00:00Z",
            "body": (f"task_uid: {MODULE.NORMATIVE_TASK_UID}\n"
                     f"- pr_url: `https://github.com/{REPOSITORY}/pull/{MODULE.NORMATIVE_PR}`\n"
                     f"- pr_number: `{MODULE.NORMATIVE_PR}`\n"),
        },
        MODULE.PLANNER_ISSUE: {
            "number": MODULE.PLANNER_ISSUE, "state": "closed",
            "repository_url": f"https://api.github.com/repos/{REPOSITORY}",
            "created_at": "2026-09-24T21:31:40Z",
            "body": (f"task_uid: {MODULE.PLANNER_TASK_UID}\n"
                     f"- pr_url: `https://github.com/{REPOSITORY}/pull/{MODULE.PLANNER_PR}`\n"
                     f"- pr_number: `{MODULE.PLANNER_PR}`\n"),
        },
        SUCCESSOR_ISSUE: {
            "number": SUCCESSOR_ISSUE, "state": "open",
            "repository_url": f"https://api.github.com/repos/{REPOSITORY}",
            "created_at": "2026-09-24T11:42:55Z",
            "body": ("<!-- oasis7-pm-task -->\n" f"task_uid: {TASK_UID}\n"
                     f"- pr_url: `https://github.com/{REPOSITORY}/pull/{CHECKER_PR}`\n"
                     f"- pr_number: `{CHECKER_PR}`\n"),
        },
    }
    pulls = {
        MODULE.NORMATIVE_PR: {
            "number": MODULE.NORMATIVE_PR, "state": "closed", "merged": True,
            "merge_commit_sha": NORMATIVE_COMMIT, "created_at": "2026-09-23T11:00:00Z",
            "body": f"Task: {MODULE.NORMATIVE_TASK_UID}\n\nRefs #{MODULE.NORMATIVE_ISSUE}\n",
            "base": {"ref": "main", "sha": normative_expected["source_scope_base"], "repo": {"full_name": REPOSITORY}},
            "head": {"sha": normative_expected["source_head"], "repo": {"full_name": REPOSITORY}},
        },
        MODULE.PLANNER_PR: {
            "number": MODULE.PLANNER_PR, "state": "closed", "merged": True,
            "merge_commit_sha": PLANNER_COMMIT, "created_at": "2026-09-24T22:12:15Z",
            "body": f"Task: {MODULE.PLANNER_TASK_UID}\n\nRefs #{MODULE.PLANNER_ISSUE}\n",
            "base": {"ref": "main", "sha": CHECKER_BASE, "repo": {"full_name": REPOSITORY}},
            "head": {"sha": PLANNER_SOURCE_HEAD, "repo": {"full_name": REPOSITORY}},
        },
        CHECKER_PR: {
            "number": CHECKER_PR, "state": "open", "merged": False, "draft": True,
            "created_at": "2026-09-24T11:53:53Z",
            "body": f"Task: {TASK_UID}\n\nRefs #{SUCCESSOR_ISSUE}\n",
            "base": {"ref": "main", "sha": CHECKER_BASE, "repo": {"full_name": REPOSITORY}},
            "head": {"sha": CHECKER_HEAD, "ref": "task/engineering-checker-successor-admission",
                     "repo": {"full_name": REPOSITORY}},
        },
    }
    comments = {
        MODULE.NORMATIVE_COMMENT: _comment_object(
            MODULE.NORMATIVE_COMMENT, MODULE.NORMATIVE_ISSUE, bodies[MODULE.NORMATIVE_COMMENT]
        ),
        MODULE.PLANNER_COMMENT: _comment_object(
            MODULE.PLANNER_COMMENT, MODULE.PLANNER_ISSUE, bodies[MODULE.PLANNER_COMMENT]
        ),
        MODULE.PLANNER_REVIEW_COMMENT: _comment_object(
            MODULE.PLANNER_REVIEW_COMMENT, MODULE.PLANNER_ISSUE,
            f"- Task UID: {MODULE.PLANNER_TASK_UID}\n- Source Head: {PLANNER_SOURCE_HEAD}\n"
            "- Review Roles: repository_health_engineer,qa_engineer\n"
            "- Review Evidence: repository_health_engineer: no_findings; qa_engineer: no_findings\n"
            "- Review Findings Disposition: no_findings\n",
        ),
        MODULE.PLANNER_INTEGRATION_COMMENT: _comment_object(
            MODULE.PLANNER_INTEGRATION_COMMENT, MODULE.PLANNER_ISSUE,
            f"Identity: UID {MODULE.PLANNER_TASK_UID}, task Issue #{MODULE.PLANNER_ISSUE}, "
            f"sole PR #{MODULE.PLANNER_PR}, frozen H={PLANNER_SOURCE_HEAD}, S/B={CHECKER_BASE}\n"
            f"Strict integration run {INTEGRATION_RUN} dispatched; PR required-gate runs independently.\n",
        ),
    }

    def api(path):
        if path == f"repos/{REPOSITORY}":
            return {"default_branch": "main"}
        if "/issues/comments/" in path:
            return comments[int(path.rsplit("/", 1)[1])]
        if "/issues/" in path and "/comments/" not in path:
            return issues[int(path.rsplit("/", 1)[1])]
        if "/compare/" in path:
            return {"merge_base_commit": {"sha": CHECKER_SCOPE}}
        if path.endswith(f"/commits/{PLANNER_SOURCE_HEAD}/check-runs?per_page=100"):
            return {"total_count": 1, "check_runs": [{
                "id": 456, "name": "required-gate", "head_sha": PLANNER_SOURCE_HEAD,
                "details_url": f"https://github.com/{REPOSITORY}/actions/runs/{PLANNER_REQUIRED_RUN}/job/1",
                "status": "completed", "conclusion": "success", "app": {"id": 15368},
            }]}
        if path.endswith(f"/commits/{CHECKER_BASE}/check-runs?per_page=100"):
            return {"total_count": 1, "check_runs": [{
                "id": CHECK_RUN_ID, "name": "required-gate", "head_sha": CHECKER_BASE,
                "details_url": f"https://github.com/{REPOSITORY}/actions/runs/{INTEGRATION_RUN}/job/1",
                "status": "completed", "conclusion": "success", "app": {"id": 15368},
            }]}
        if "/actions/runs/" in path:
            run_id = int(path.rsplit("/", 1)[1])
            if run_id == PLANNER_REQUIRED_RUN:
                return {
                    "id": run_id, "run_attempt": 1, "status": "completed", "conclusion": "success",
                    "event": "pull_request", "path": ".github/workflows/rust.yml",
                    "head_branch": pulls[MODULE.PLANNER_PR]["head"].get("ref"),
                    "head_sha": PLANNER_SOURCE_HEAD, "head_repository": {"full_name": REPOSITORY},
                }
            return {
                "id": INTEGRATION_RUN, "run_attempt": 1, "status": "completed", "conclusion": "success",
                "event": "workflow_dispatch", "path": ".github/workflows/rust.yml",
                "head_branch": "main", "head_sha": CHECKER_BASE,
                "display_title": (
                    f"oasis7-ci|workflow_dispatch|integration_revalidation|{MODULE.PLANNER_TASK_UID}|"
                    f"{MODULE.PLANNER_PR}|{CHECKER_BASE}|{PLANNER_SOURCE_HEAD}"
                ),
            }
        if "/commits/" in path:
            return commits[path.rsplit("/", 1)[1]]
        if "/contents/" in path:
            path_and_ref = path.split("/contents/", 1)[1]
            file_path, commit = path_and_ref.split("?ref=", 1)
            return contents[(commit, file_path)]
        if path.endswith(f"/pulls/{CHECKER_PR}/files?per_page=100&page=1"):
            return [{"filename": value, "status": "modified"} for value in MODULE.CHECKER_SCOPE]
        if "/pulls/" in path:
            return pulls[int(path.rsplit("/", 1)[1])]
        raise AssertionError(path)
    return api


class CheckerStageAdmissionTest(unittest.TestCase):
    def setUp(self):
        normative_expected, planner_expected = _fixture_authority_expectations()
        self.normative_expected_patch = patch.object(
            MODULE, "NORMATIVE_AUTHORITY_EXPECTED", normative_expected
        )
        self.normative_expected_patch.start()
        self.addCleanup(self.normative_expected_patch.stop)
        self.planner_expected_patch = patch.object(
            MODULE, "PLANNER_AUTHORITY_EXPECTED", planner_expected
        )
        self.planner_expected_patch.start()
        self.addCleanup(self.planner_expected_patch.stop)
        self.profile_patch = patch.object(MODULE, "_read_profile_artifacts", return_value=_profile_artifacts())
        self.profile_patch.start()
        self.addCleanup(self.profile_patch.stop)

    def test_workflow_checker_route_does_not_depend_on_optional_impact_marker(self):
        workflow = (Path(__file__).parents[2] / ".github/workflows/rust.yml").read_text(
            encoding="utf-8"
        )
        self.assertIn("id: checker-stage", workflow)
        self.assertIn(
            'git diff --name-status --find-renames --find-copies --find-copies-harder "${CHECKER_BASE_SHA}" "${CHECKER_HEAD_SHA}"',
            workflow,
        )
        self.assertIn("cargo_checker_stage_admission.py", workflow)
        self.assertIn("${CHECKER_BASE_SHA}:scripts/pm/cargo_checker_stage_admission.py", workflow)
        self.assertNotIn("issues/3827", workflow)
        self.assertIn("checker-stage rename/copy or non-modification changes are not admissible", workflow)
        self.assertIn('"${CHECKER_BASE_SHA}:scripts/pm/cargo_checker_stage_admission.py"', workflow)
        self.assertIn(
            "OASIS7_CARGO_STAGE_PR_NUMBER: ${{ steps.checker-stage.outputs.pr_number }}",
            workflow,
        )
        self.assertNotIn(
            "steps.scope.outputs.task_uid == 'task_be264ac2833044969d3c2c50b2b83cea'",
            workflow,
        )

    def test_workflow_defers_non_modification_rejection_until_checker_scope_is_known(self):
        workflow = (Path(__file__).parents[2] / ".github/workflows/rust.yml").read_text(
            encoding="utf-8"
        )
        self.assertIn("checker_non_modification=true", workflow)
        ordinary_route = workflow.index('if [[ "${checker_path_touched}" != true ]]')
        checker_rejection = workflow.index('if [[ "${checker_non_modification}" == true ]]')
        self.assertLess(ordinary_route, checker_rejection)
        self.assertIn('changed_paths+=("${first_path}" "${second_path}")', workflow)

    def test_workflow_changed_path_classifier_fixtures(self):
        workflow = (Path(__file__).parents[2] / ".github/workflows/rust.yml").read_text(
            encoding="utf-8"
        )
        start = workflow.index("          checker_stage_path_route() {")
        end = workflow.index('          changed_path_route="$(checker_stage_path_route ', start)
        classifier = workflow[start:end]
        cases = {
            "ordinary add": ("A\\tordinary-added.txt\\n", "ordinary"),
            "ordinary delete": ("D\\tordinary-deleted.txt\\n", "ordinary"),
            "unrelated rename": ("R100\\told-name.txt\\tnew-name.txt\\n", "ordinary"),
            "unrelated copy": ("C100\\tcopy-source.txt\\tcopy-target.txt\\n", "ordinary"),
            "exact checker modification": (
                "M\\tscripts/pm/check-cargo-package-scope\\n"
                "M\\tscripts/pm/check-cargo-package-scope.test.py\\n",
                "checker",
            ),
            "checker rename": (
                "R100\\tscripts/pm/check-cargo-package-scope\\tother-checker.txt\\n",
                "reject",
            ),
            "checker copy": (
                "C100\\tscripts/pm/check-cargo-package-scope\\tother-checker.txt\\n",
                "reject",
            ),
            "checker add": ("A\\tscripts/pm/check-cargo-package-scope\\n", "reject"),
            "checker delete": ("D\\tscripts/pm/check-cargo-package-scope\\n", "reject"),
            "checker and unrelated mixed scope": (
                "M\\tscripts/pm/check-cargo-package-scope\\n"
                "M\\tscripts/pm/check-cargo-package-scope.test.py\\n"
                "A\\tordinary-added.txt\\n",
                "reject",
            ),
        }
        script = (
            'checker_paths=("scripts/pm/check-cargo-package-scope" '
            '"scripts/pm/check-cargo-package-scope.test.py")\n'
            + classifier
            + 'checker_stage_path_route "$1"\n'
        )
        with tempfile.TemporaryDirectory() as temp_dir:
            status_path = Path(temp_dir) / "changed-paths.txt"
            for name, (status_records, expected) in cases.items():
                with self.subTest(name=name):
                    status_path.write_text(status_records.replace("\\t", "\t").replace("\\n", "\n"))
                    result = subprocess.run(
                        ["bash", "-c", script, "checker-stage-fixture", str(status_path)],
                        text=True,
                        capture_output=True,
                        check=False,
                    )
                    actual = result.stdout.strip() if result.returncode == 0 else "reject"
                    self.assertEqual(expected, actual, result.stderr)

    def test_workflow_bypasses_generic_profile_planner_only_for_exact_stage_route(self):
        workflow = (Path(__file__).parents[2] / ".github/workflows/rust.yml").read_text(
            encoding="utf-8"
        )
        stage_branch = workflow.index('if [[ -n "${OASIS7_CARGO_STAGE_PR_NUMBER:-}" ]]')
        planner_call = workflow.index('python3 "${OASIS7_CARGO_PROFILE_PLANNER}"')
        self.assertLess(stage_branch, planner_call)
        self.assertIn("OASIS7_CARGO_STAGE_TASK_UID", workflow)
        self.assertIn("cargo-checker-stage-admission-receipt-", workflow)
        self.assertIn("steps.checker-stage.outputs.pr_number == ''", workflow)

    def test_workflow_produces_receipt_on_separate_success_dependent_runner(self):
        workflow = (Path(__file__).parents[2] / ".github/workflows/rust.yml").read_text(
            encoding="utf-8"
        )
        required_gate_start = workflow.index("  required-gate:")
        producer_start = workflow.index("  checker-stage-receipt:")
        following_job = workflow.index("\n  windows-package-rollout-behavior:", producer_start)
        required_gate_block = workflow[required_gate_start:producer_start]
        producer_block = workflow[producer_start:following_job]
        tests = required_gate_block.index("      - name: Run required test tier")
        required_test_lines = required_gate_block[tests:].splitlines()
        required_test_block = [required_test_lines[0]]
        for line in required_test_lines[1:]:
            if line.startswith("      - "):
                break
            required_test_block.append(line)
        upload_in_tests_job = required_gate_block.find("Upload Cargo checker-stage admission receipt")
        self.assertGreater(tests, 0)
        self.assertIn("        id: required-gate-tests", "\n".join(required_test_block))
        self.assertEqual(-1, upload_in_tests_job)
        self.assertIn("needs: required-gate", producer_block)
        self.assertIn(
            "if: ${{ needs.required-gate.result == 'success' && needs.required-gate.outputs.checker_stage_pr_number != '' }}",
            producer_block,
        )
        self.assertIn("ref: ${{ needs.required-gate.outputs.checker_stage_integration_base_oid }}", producer_block)
        self.assertIn("refs/pull/${CHECKER_PR_NUMBER}/head", producer_block)
        self.assertIn('git show "${CHECKER_SOURCE_SCOPE}:scripts/pm/cargo_checker_stage_admission.py"', producer_block)
        self.assertIn('subprocess.run(preflight["checker_command"], check=False)', producer_block)
        self.assertIn("steps.checker-stage-producer.outcome == 'success'", producer_block)
        self.assertIn("steps.checker-stage-producer.outputs.receipt", producer_block)
        producer_steps = [line.strip() for line in producer_block.splitlines() if line.startswith("      - ")]
        producer_index = producer_steps.index("- id: checker-stage-producer")
        upload_index = producer_steps.index("- name: Upload Cargo checker-stage admission receipt")
        self.assertEqual(producer_index + 1, upload_index)
        self.assertIn('checked_out_base="$(git rev-parse "HEAD^{commit}")"', producer_block)
        self.assertNotIn("CI_VERBOSE=", producer_block)
        self.assertNotIn("./scripts/ci-tests.sh", producer_block)
        source_truth = (Path(__file__).parents[2] / "doc/engineering/workflow/source-of-truth.md").read_text(
            encoding="utf-8"
        )
        stage_contract = source_truth.split('<a id="cargo-checker-stage-integration-route">', 1)[1].split(
            '<a id="', 1
        )[0]
        self.assertIn("separate GitHub-hosted producer job on a fresh runner", stage_contract)
        self.assertIn("completed successfully after required-gate", stage_contract)

        ci_tests = (Path(__file__).parents[2] / "scripts/ci-tests.sh").read_text(
            encoding="utf-8"
        )
        self.assertNotIn('"$stage_adapter" post-run', ci_tests)

    def test_workflow_uses_source_head_for_checker_stage_check_lookup(self):
        workflow = (Path(__file__).parents[2] / ".github/workflows/rust.yml").read_text(
            encoding="utf-8"
        )
        self.assertIn("OASIS7_CARGO_SCOPE_HEAD: ${{ steps.scope.outputs.head_oid }}", workflow)
        self.assertIn(
            "OASIS7_CARGO_STAGE_CHECK_HEAD: ${{ steps.scope.outputs.head_oid }}", workflow
        )

    def test_authority_freezes_the_project_verified_successor_without_legacy_fallback(self):
        self.assertEqual(SUCCESSOR_ISSUE, MODULE.CHECKER_ISSUE)
        self.assertEqual(TASK_UID, MODULE.CHECKER_TASK_UID)
        self.assertEqual(CHECKER_PR, MODULE.CHECKER_PR)
        self.assertNotEqual(3827, MODULE.CHECKER_ISSUE)

    def test_stale_legacy_binding_is_never_used_for_the_frozen_successor_pr(self):
        calls = []
        api = _authority_api()

        def recording_api(path):
            calls.append(path)
            response = api(path)
            if path.endswith(f"/issues/{SUCCESSOR_ISSUE}"):
                response = dict(response)
                response["body"] = response["body"].replace(TASK_UID, MODULE.LEGACY_CHECKER_TASK_UID)
            return response

        with patch.object(MODULE, "gh_api", side_effect=recording_api):
            with self.assertRaisesRegex(MODULE.AdmissionError, "Issue UID"):
                MODULE.classify_checker_stage(
                    REPOSITORY, CHECKER_PR, TASK_UID, CHECKER_BASE, CHECKER_HEAD
                )
        self.assertTrue(any(path.endswith(f"/issues/{SUCCESSOR_ISSUE}") for path in calls))
        self.assertFalse(any(path.endswith("/issues/3827") for path in calls))

    def test_ordinary_pr_route_does_not_depend_on_successor_issue_availability(self):
        def unavailable_successor(path):
            if path.endswith(f"/issues/{SUCCESSOR_ISSUE}"):
                raise AssertionError("ordinary PR must not read the successor Issue")
            raise AssertionError(f"unexpected API read: {path}")

        with patch.object(MODULE, "gh_api", side_effect=unavailable_successor):
            self.assertFalse(
                MODULE.classify_checker_stage(
                    REPOSITORY, 5000, TASK_UID, CHECKER_BASE, CHECKER_HEAD
                )
            )

    def test_stage_classifier_rechecks_live_successor_pr_and_strict_creation_order(self):
        mutations = {
            "base_repository": lambda pull: pull["base"].update(repo={"full_name": "other/repo"}),
            "base_branch": lambda pull: pull["base"].update(ref="release"),
            "base_head": lambda pull: pull["base"].update(sha="9" * 40),
            "head_repository": lambda pull: pull["head"].update(repo={"full_name": "fork/oasis7"}),
            "head_sha": lambda pull: pull["head"].update(sha="9" * 40),
            "pr_number": lambda pull: pull.update(number=CHECKER_PR + 1),
            "pr_task_uid": lambda pull: pull.update(body=f"Task: task_{'0' * 32}\n\nRefs #{SUCCESSOR_ISSUE}"),
            "pr_issue_ref": lambda pull: pull.update(body=f"Task: {TASK_UID}\n\nRefs #4021"),
            "issue_before_equal": lambda pull: pull.update(created_at="2026-09-24T11:42:55Z"),
            "issue_after_pr": lambda pull: pull.update(created_at="2026-09-24T12:00:00Z"),
            "issue_bad_timestamp": lambda pull: pull.update(created_at="yesterday"),
            "issue_timezone_missing": lambda pull: pull.update(created_at="2026-09-24T11:42:55"),
            "pr_bad_timestamp": lambda pull: pull.update(created_at="yesterday"),
        }
        for changed, mutate in mutations.items():
            api = _authority_api()

            def changed_api(path, *, api=api, mutate=mutate):
                response = api(path)
                if path.endswith(f"/pulls/{CHECKER_PR}"):
                    response = dict(response)
                    response["base"] = dict(response["base"])
                    response["head"] = dict(response["head"])
                    mutate(response)
                elif path.endswith(f"/issues/{SUCCESSOR_ISSUE}") and changed in {
                    "issue_before_equal", "issue_after_pr", "issue_bad_timestamp", "issue_timezone_missing"
                }:
                    response = dict(response)
                    issue_times = {
                        "issue_before_equal": "2026-09-24T11:53:53Z",
                        "issue_after_pr": "2026-09-24T12:00:00Z",
                        "issue_bad_timestamp": "yesterday",
                        "issue_timezone_missing": "2026-09-24T11:42:55",
                    }
                    response["created_at"] = issue_times[changed]
                return response

            with self.subTest(changed=changed), patch.object(MODULE, "gh_api", side_effect=changed_api):
                with self.assertRaisesRegex(
                    MODULE.AdmissionError,
                    "checker PR|checker task Issue|timestamp|timezone|predate|reciprocal PR|reciprocal Issue",
                ):
                    MODULE.classify_checker_stage(
                        REPOSITORY, CHECKER_PR, TASK_UID, CHECKER_BASE, CHECKER_HEAD
                    )

    def test_checker_pr_changed_file_api_rejects_rename_copy_metadata(self):
        for changed in ("renamed", "copied"):
            api = _authority_api()

            def changed_api(path, *, api=api, changed=changed):
                response = api(path)
                if path.endswith(f"/pulls/{CHECKER_PR}/files?per_page=100&page=1"):
                    response = [dict(item) for item in response]
                    response[0]["previous_filename"] = "scripts/pm/renamed-from-checker.py"
                    if changed == "renamed":
                        response[0]["status"] = "renamed"
                return response

            with self.subTest(changed=changed), patch.object(MODULE, "gh_api", side_effect=changed_api):
                with self.assertRaisesRegex(MODULE.AdmissionError, "rename/copy"):
                    MODULE.verify_checker_pr(
                        REPOSITORY, CHECKER_PR, TASK_UID, CHECKER_BASE, CHECKER_HEAD,
                        CHECKER_SCOPE, TESTED_TREE, Path("."),
                    )

    def test_stage_classifier_rejects_non_modification_or_mixed_live_file_scope(self):
        for changed in ("renamed", "copied", "added", "removed", "mixed"):
            api = _authority_api()

            def changed_api(path, *, api=api, changed=changed):
                response = api(path)
                if path.endswith(f"/pulls/{CHECKER_PR}/files?per_page=100&page=1"):
                    response = [dict(item) for item in response]
                    if changed == "renamed":
                        response[0].update(status="renamed", previous_filename="old-path")
                    elif changed == "copied":
                        response[0]["previous_filename"] = "copied-from-path"
                    elif changed == "added":
                        response[0]["status"] = "added"
                    elif changed == "removed":
                        response[0]["status"] = "removed"
                    elif changed == "mixed":
                        response.append({"filename": "crates/unrelated/src/lib.rs", "status": "modified"})
                return response

            with self.subTest(changed=changed), patch.object(MODULE, "gh_api", side_effect=changed_api):
                with self.assertRaisesRegex(MODULE.AdmissionError, "rename/copy|non-modification|exact checker-only"):
                    MODULE.classify_checker_stage(
                        REPOSITORY, CHECKER_PR, TASK_UID, CHECKER_BASE, CHECKER_HEAD
                    )

    def test_pr_check_head_uses_source_head_without_overwriting_workflow_shas(self):
        source_head = "5" * 40
        synthetic_merge_sha = "6" * 40
        workflow_sha = "7" * 40
        args = type("Args", (), {"head_oid": source_head, "check_head": synthetic_merge_sha})()
        with patch.dict(
            MODULE.os.environ,
            {
                "GITHUB_EVENT_NAME": "pull_request",
                "GITHUB_SHA": synthetic_merge_sha,
                "GITHUB_WORKFLOW_SHA": workflow_sha,
                "OASIS7_CARGO_STAGE_CHECK_HEAD": source_head,
            },
        ):
            self.assertEqual(source_head, MODULE.resolve_live_check_head(args))
            self.assertEqual(synthetic_merge_sha, MODULE.os.environ["GITHUB_SHA"])
            self.assertEqual(workflow_sha, MODULE.os.environ["GITHUB_WORKFLOW_SHA"])
        with patch.dict(
            MODULE.os.environ,
            {
                "GITHUB_EVENT_NAME": "pull_request",
                "GITHUB_SHA": synthetic_merge_sha,
                "OASIS7_CARGO_STAGE_CHECK_HEAD": "8" * 40,
            },
        ):
            with self.assertRaisesRegex(MODULE.AdmissionError, "source check head does not match"):
                MODULE.resolve_live_check_head(args)

    def test_pr_check_head_rejects_missing_stage_source_head(self):
        args = type("Args", (), {"head_oid": "5" * 40, "check_head": "6" * 40})()
        with patch.dict(
            MODULE.os.environ,
            {"GITHUB_EVENT_NAME": "pull_request", "GITHUB_SHA": "6" * 40},
            clear=True,
        ):
            with self.assertRaisesRegex(MODULE.AdmissionError, "source check head"):
                MODULE.resolve_live_check_head(args)

    def test_pr_check_head_rejects_malformed_stage_source_head(self):
        args = type("Args", (), {"head_oid": "5" * 40, "check_head": "6" * 40})()
        with patch.dict(
            MODULE.os.environ,
            {
                "GITHUB_EVENT_NAME": "pull_request",
                "GITHUB_SHA": "6" * 40,
                "OASIS7_CARGO_STAGE_CHECK_HEAD": "not-a-full-oid",
            },
            clear=True,
        ):
            with self.assertRaisesRegex(MODULE.AdmissionError, "source check head"):
                MODULE.resolve_live_check_head(args)

    def test_authority_chain_reads_both_fixed_live_server_readbacks(self):
        with patch.object(MODULE, "gh_api", side_effect=_authority_api()), \
             patch.object(MODULE, "_recompute_planner_integration_tree", return_value=TESTED_TREE) as tree_read:
            chain = MODULE.verify_authority_chain(REPOSITORY, Path("."))
        self.assertEqual(NORMATIVE_COMMIT, chain["normative"]["merged_commit"])
        self.assertEqual(PLANNER_COMMIT, chain["planner"]["merged_commit"])
        self.assertEqual(REPOSITORY, chain["planner"]["repository"])
        tree_read.assert_called_once_with(Path("."), CHECKER_BASE, PLANNER_SOURCE_HEAD)

    def test_current_main_planner_receipt_schema_is_readable_without_legacy_fallback(self):
        parsed = MODULE._parse_planner_receipt(CURRENT_PLANNER_RECEIPT)
        self.assertEqual("planner_authority", parsed["stage"])
        self.assertEqual(4000, parsed["pr_number"])
        self.assertEqual(3999, parsed["issue_number"])
        self.assertEqual("task_978dcc0005b9415cbc45e59b21b095e0", parsed["task_uid"])
        self.assertEqual(
            "sha256:7c8a2de3e4b9b5e4c752e5147438bfe0eb80edb7a4406bc8d2b59aff498774fc",
            parsed["authority_bytes_sha256"],
        )
        self.assertEqual(
            "sha256:63a0a1735d83d5b9431c1ecd07aa54b4437e84128dcf5cf821591489dfa4f8bb",
            parsed["stable_fragment_sha256"],
        )

    def test_live_normative_and_planner_receipts_match_frozen_authority_identity(self):
        normative = MODULE._parse_normative_receipt(CURRENT_NORMATIVE_RECEIPT)
        planner = MODULE._parse_planner_receipt(CURRENT_PLANNER_RECEIPT)
        MODULE._assert_expected_fields(
            normative, PINNED_NORMATIVE_AUTHORITY_EXPECTED, "normative authority"
        )
        MODULE._assert_expected_fields(
            {
                **planner,
                "trusted_integration_tested_tree": PINNED_PLANNER_AUTHORITY_EXPECTED[
                    "trusted_integration_tested_tree"
                ],
            },
            PINNED_PLANNER_AUTHORITY_EXPECTED,
            "planner authority",
        )
        self.assertEqual(3999, planner["issue_number"])
        self.assertEqual(4000, planner["pr_number"])
        self.assertNotEqual(
            MODULE.PLANNER_TESTED_TREE,
            MODULE.PLANNER_MERGED_TREE,
            "integration tested tree T is distinct from the squash merge commit tree",
        )

    def test_legacy_planner_receipt_schema_is_not_a_fallback(self):
        legacy_receipt = CURRENT_PLANNER_RECEIPT.replace(
            "stage=planner_authority; immutable merged authority, not candidate self-approval",
            "stage=approved_planner_authority; post-merge server readback",
        )
        with self.assertRaisesRegex(MODULE.AdmissionError, "immutable planner-authority readback"):
            MODULE._parse_planner_receipt(legacy_receipt)

    def test_planner_authority_rejects_wrong_live_pr_repository_or_base(self):
        changes = (
            ("base_repository", lambda pull: pull["base"].update(repo={"full_name": "other/repo"}), "base repository"),
            ("base_branch", lambda pull: pull["base"].update(ref="release"), "base repository/branch"),
            ("base_sha", lambda pull: pull["base"].update(sha="9" * 40), "base identity"),
            ("head_repository", lambda pull: pull["head"].update(repo={"full_name": "fork/oasis7"}), "head repository"),
        )
        for label, mutate, expected in changes:
            api = _authority_api()

            def wrong_pull(path, *, api=api, mutate=mutate):
                response = api(path)
                if path.endswith(f"/pulls/{MODULE.PLANNER_PR}"):
                    response = json.loads(json.dumps(response))
                    mutate(response)
                return response

            with self.subTest(label=label), patch.object(MODULE, "gh_api", side_effect=wrong_pull):
                with self.assertRaisesRegex(MODULE.AdmissionError, expected):
                    MODULE.verify_authority_chain(REPOSITORY)

    def test_planner_integration_tree_is_recomputed_from_base_and_source_head(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)

            def git(*args):
                return subprocess.check_output(
                    ["git", "-C", str(root), *args], text=True, stderr=subprocess.PIPE
                ).strip()

            git("init", "-q")
            git("config", "user.name", "test")
            git("config", "user.email", "test@example.invalid")
            (root / "shared.txt").write_text("base\n", encoding="utf-8")
            git("add", "shared.txt")
            git("commit", "-qm", "base")
            common = git("rev-parse", "HEAD")
            git("checkout", "-qb", "integration")
            (root / "base-only.txt").write_text("base\n", encoding="utf-8")
            git("add", "base-only.txt")
            git("commit", "-qm", "integration base")
            base = git("rev-parse", "HEAD")
            git("checkout", "-qb", "source", common)
            (root / "head-only.txt").write_text("head\n", encoding="utf-8")
            git("add", "head-only.txt")
            git("commit", "-qm", "source head")
            head = git("rev-parse", "HEAD")
            expected_tree = git("merge-tree", "--write-tree", base, head)

            actual_tree = MODULE._recompute_planner_integration_tree(root, base, head)

        self.assertEqual(expected_tree, actual_tree)

    def test_authority_chain_rejects_tampered_server_tree(self):
        api = _authority_api()
        original = api
        def tampered(path):
            response = original(path)
            if "/commits/" in path and path.endswith(PLANNER_COMMIT):
                response = dict(response)
                response["commit"] = {"tree": {"sha": "0" * 40}}
            return response
        with patch.object(MODULE, "gh_api", side_effect=tampered):
            with self.assertRaisesRegex(MODULE.AdmissionError, "tree"):
                MODULE.verify_authority_chain(REPOSITORY)

    def test_authority_chain_rejects_tampered_server_bytes(self):
        api = _authority_api()
        original = api
        def tampered(path):
            response = original(path)
            if "/contents/" in path and path.endswith(PLANNER_COMMIT):
                response = dict(response)
                response["content"] = base64.b64encode(b"tampered").decode()
                response["size"] = len(b"tampered")
            return response
        with patch.object(MODULE, "gh_api", side_effect=tampered):
            with self.assertRaisesRegex(MODULE.AdmissionError, "digest|size"):
                MODULE.verify_authority_chain(REPOSITORY)

    def test_executing_planner_rejects_dirty_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "scripts/pm").mkdir(parents=True)
            planner = root / MODULE.PLANNER_PATH
            planner.write_bytes(b"dirty")
            with self.assertRaisesRegex(MODULE.AdmissionError, "planner.*bytes"):
                MODULE.verify_executing_planner(
                    planner, {"authority_bytes": _planner_bytes(), "authority_bytes_sha256":
                              "sha256:" + hashlib.sha256(_planner_bytes()).hexdigest(),
                              "authority_size": len(_planner_bytes())}, root,
                )

    def test_executing_planner_rejects_missing_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            planner = root / "scripts/pm/cargo_package_profile_planner.py"
            with self.assertRaisesRegex(MODULE.AdmissionError, "missing"):
                MODULE.verify_executing_planner(
                    planner, {"authority_bytes": _planner_bytes(), "authority_bytes_sha256":
                              "sha256:" + hashlib.sha256(_planner_bytes()).hexdigest(),
                              "authority_size": len(_planner_bytes())}, root,
                )

    def _run_missing_planner_source_case(
        self,
        *,
        advertised_oid=PLANNER_SOURCE_HEAD,
        advertised_ref=PLANNER_SOURCE_REF,
        source_bytes=None,
        remote_unavailable=False,
    ):
        """Model a merge checkout missing the approved planner source object.

        The trusted planner receipt still binds the exact source-head OID.  The
        RED contract requires the adapter to read the advertised PR ref,
        fetch only that ref, and then verify the fetched planner bytes against
        the already verified merged authority bytes.
        """
        calls = []
        fetched = False
        authority_bytes = _planner_bytes()
        source_bytes = authority_bytes if source_bytes is None else source_bytes

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            planner = root / MODULE.PLANNER_PATH
            planner.parent.mkdir(parents=True)
            planner.write_bytes(authority_bytes)

            def check_output(command, **kwargs):
                nonlocal fetched
                argv = [str(value) for value in command]
                calls.append(argv)
                if "ls-remote" in argv:
                    if remote_unavailable:
                        raise subprocess.CalledProcessError(128, argv, stderr=b"remote unavailable")
                    return f"{advertised_oid}\t{advertised_ref}\n".encode()
                if "fetch" in argv:
                    fetched = True
                    return b""
                if "show" in argv:
                    spec = argv[-1]
                    if spec == f"{PLANNER_COMMIT}:{MODULE.PLANNER_PATH}":
                        return authority_bytes
                    if spec == f"{PLANNER_SOURCE_HEAD}:{MODULE.PLANNER_PATH}":
                        if not fetched:
                            raise subprocess.CalledProcessError(
                                128, argv, stderr=b"fatal: bad object: approved source head unavailable"
                            )
                        return source_bytes
                raise AssertionError(f"unexpected git command: {argv}")

            authority = {
                "authority_path": MODULE.PLANNER_PATH,
                "authority_bytes": authority_bytes,
                "authority_bytes_sha256": "sha256:" + hashlib.sha256(authority_bytes).hexdigest(),
                "authority_size": len(authority_bytes),
                "merged_commit": PLANNER_COMMIT,
                "source_head": PLANNER_SOURCE_HEAD,
                "source_ref": PLANNER_SOURCE_REF,
            }
            try:
                with patch.object(MODULE.subprocess, "check_output", side_effect=check_output):
                    result = MODULE.verify_executing_planner(planner, authority, root)
            except Exception as exc:  # return the RED failure for negative cases
                return None, calls, exc
            return result, calls, None

    def test_executing_planner_recovers_missing_source_object_from_exact_pr_ref(self):
        result, calls, error = self._run_missing_planner_source_case()
        self.assertIsNone(error, error)
        self.assertEqual(PLANNER_SOURCE_HEAD, result["source_head"])
        advertised = [argv for argv in calls if "ls-remote" in argv]
        self.assertTrue(advertised, calls)
        self.assertIn(PLANNER_SOURCE_REF, advertised[0])
        fetched = [argv for argv in calls if "fetch" in argv]
        self.assertTrue(fetched, calls)
        self.assertEqual(
            fetched[0][3:],
            ["fetch", "--no-write-fetch-head", "--no-tags", "origin", PLANNER_SOURCE_REF],
        )

    def test_executing_planner_rejects_wrong_advertised_source_oid(self):
        result, calls, error = self._run_missing_planner_source_case(advertised_oid="0" * 40)
        self.assertIsNone(result)
        self.assertTrue(any("ls-remote" in argv for argv in calls), calls)
        self.assertIsNotNone(error)
        self.assertRegex(str(error), r"source.*(?:OID|identity)|advertised|ref")

    def test_executing_planner_rejects_wrong_advertised_source_ref(self):
        result, calls, error = self._run_missing_planner_source_case(
            advertised_ref="refs/heads/main"
        )
        self.assertIsNone(result)
        self.assertTrue(any("ls-remote" in argv for argv in calls), calls)
        self.assertIsNotNone(error)
        self.assertRegex(str(error), r"source.*(?:ref|OID|identity)|advertised")

    def test_executing_planner_rejects_unavailable_source_ref(self):
        result, calls, error = self._run_missing_planner_source_case(remote_unavailable=True)
        self.assertIsNone(result)
        self.assertTrue(any("ls-remote" in argv for argv in calls), calls)
        self.assertIsNotNone(error)
        self.assertRegex(str(error), r"source.*(?:object|ref)|fetch|unavailable")

    def test_executing_planner_rejects_tampered_fetched_source_bytes(self):
        result, calls, error = self._run_missing_planner_source_case(source_bytes=b"tampered")
        self.assertIsNone(result)
        self.assertTrue(any("ls-remote" in argv for argv in calls), calls)
        self.assertIsNotNone(error)
        self.assertRegex(str(error), r"planner.*(?:bytes|match|authority)")

    def test_checker_identity_rejects_wrong_head_and_scope(self):
        api = _authority_api()
        with patch.object(MODULE, "gh_api", side_effect=api):
            with self.assertRaisesRegex(MODULE.AdmissionError, "head"):
                MODULE.verify_checker_pr(
                    REPOSITORY, CHECKER_PR, TASK_UID, CHECKER_BASE, "9" * 40,
                    CHECKER_SCOPE, TESTED_TREE, Path("."),
                )

    def test_stage_classifier_keeps_ordinary_prs_conservative_and_blocks_suspicious_checker(self):
        with patch.object(MODULE, "gh_api", side_effect=_authority_api()):
            self.assertFalse(MODULE.classify_checker_stage(
                REPOSITORY, 9999, None, CHECKER_BASE, CHECKER_HEAD
            ))
            self.assertTrue(MODULE.classify_checker_stage(
                REPOSITORY, CHECKER_PR, TASK_UID, CHECKER_BASE, CHECKER_HEAD
            ))
            with self.assertRaisesRegex(MODULE.AdmissionError, "trusted task"):
                MODULE.classify_checker_stage(
                    REPOSITORY, CHECKER_PR, "task_00000000000000000000000000000000",
                    CHECKER_BASE, CHECKER_HEAD,
                )

    def test_checker_task_binding_rejects_issue_uid_or_reciprocal_pr_drift(self):
        api = _authority_api()
        original = api

        def wrong_issue(path):
            response = original(path)
            if path.endswith(f"issues/{SUCCESSOR_ISSUE}"):
                response = dict(response)
                response["body"] = response["body"].replace(
                    TASK_UID, "task_00000000000000000000000000000000"
                )
            return response

        with patch.object(MODULE, "gh_api", side_effect=wrong_issue):
            with self.assertRaisesRegex(MODULE.AdmissionError, "Issue UID"):
                MODULE.verify_checker_pr(
                    REPOSITORY, CHECKER_PR, TASK_UID, CHECKER_BASE, CHECKER_HEAD,
                    CHECKER_SCOPE, TESTED_TREE, Path("."),
                )

        api = _authority_api()
        original = api

        def missing_reciprocal(path):
            response = original(path)
            if path.endswith(f"issues/{SUCCESSOR_ISSUE}"):
                response = dict(response)
                response["body"] = f"task_uid: {TASK_UID}\n"
            return response

        with patch.object(MODULE, "gh_api", side_effect=missing_reciprocal):
            with self.assertRaisesRegex(MODULE.AdmissionError, "reciprocal PR"):
                MODULE.verify_checker_pr(
                    REPOSITORY, CHECKER_PR, TASK_UID, CHECKER_BASE, CHECKER_HEAD,
                    CHECKER_SCOPE, TESTED_TREE, Path("."),
                )

        api = _authority_api()
        original = api

        def ambiguous_reciprocal(path):
            response = original(path)
            if path.endswith(f"issues/{SUCCESSOR_ISSUE}"):
                response = dict(response)
                response["body"] = response["body"].replace(
                    f"- pr_number: `{CHECKER_PR}`", f"- pr_number: `{CHECKER_PR + 1}`"
                )
            return response

        with patch.object(MODULE, "gh_api", side_effect=ambiguous_reciprocal):
            with self.assertRaisesRegex(MODULE.AdmissionError, "ambiguous"):
                MODULE.verify_checker_pr(
                    REPOSITORY, CHECKER_PR, TASK_UID, CHECKER_BASE, CHECKER_HEAD,
                    CHECKER_SCOPE, TESTED_TREE, Path("."),
                )

    def test_checker_task_binding_requires_one_exact_uid_field_line(self):
        cases = (
            ("duplicate", lambda body: body + f"task_uid: {TASK_UID}\n"),
            ("malformed", lambda body: body + "task_uid: malformed\n"),
            ("tab_separator", lambda body: body.replace(
                f"task_uid: {TASK_UID}", f"task_uid:\t{TASK_UID}"
            )),
            ("extra_space", lambda body: body.replace(
                f"task_uid: {TASK_UID}", f"task_uid:  {TASK_UID}"
            )),
        )
        for name, mutate_body in cases:
            api = _authority_api()

            def extra_uid(path, *, api=api, mutate_body=mutate_body):
                response = api(path)
                if path.endswith(f"issues/{SUCCESSOR_ISSUE}"):
                    response = dict(response)
                    response["body"] = mutate_body(response["body"])
                return response

            with self.subTest(name=name), patch.object(MODULE, "gh_api", side_effect=extra_uid):
                with self.assertRaisesRegex(MODULE.AdmissionError, "Issue UID"):
                    MODULE._read_checker_task_binding(REPOSITORY)

    def test_checker_task_binding_normalizes_crlf_before_exact_uid_line_check(self):
        issue = _authority_api()(f"repos/{REPOSITORY}/issues/{SUCCESSOR_ISSUE}")
        issue["body"] = f"task_uid: {TASK_UID}\r\n"
        with patch.object(MODULE, "gh_api", return_value=issue):
            with self.assertRaisesRegex(MODULE.AdmissionError, "reciprocal PR binding is unavailable"):
                MODULE._read_checker_task_binding(REPOSITORY)

    def test_checker_identity_rejects_wrong_current_task_uid_even_when_body_is_live(self):
        api = _authority_api()
        with patch.object(MODULE, "gh_api", side_effect=api), \
             patch.object(MODULE, "_git", side_effect=[CHECKER_SCOPE, TESTED_TREE]):
            with self.assertRaisesRegex(MODULE.AdmissionError, "task.*(?:identity|UID)"):
                MODULE.verify_checker_pr(
                    REPOSITORY, CHECKER_PR, "task_00000000000000000000000000000000",
                    CHECKER_BASE, CHECKER_HEAD,
                    CHECKER_SCOPE, TESTED_TREE, Path("."),
                )

    def test_authority_chain_rejects_wrong_predecessor_task_issue_or_pr_head(self):
        for replacement, message in (
            (MODULE.PLANNER_TASK_UID, "task"),
            (f"PR={MODULE.PLANNER_PR}", "PR identity"),
        ):
            api = _authority_api()
            original = api
            def wrong_receipt(path, replacement=replacement):
                response = original(path)
                if path.endswith(f"issues/comments/{MODULE.PLANNER_COMMENT}"):
                    response = dict(response)
                    body = response["body"]
                    if replacement.startswith("task_"):
                        body = body.replace(replacement, "task_00000000000000000000000000000000")
                    else:
                        body = body.replace(replacement, "PR=9999")
                    response["body"] = body
                return response
            with self.subTest(message=message), patch.object(MODULE, "gh_api", side_effect=wrong_receipt):
                with self.assertRaisesRegex(MODULE.AdmissionError, message):
                    MODULE.verify_authority_chain(REPOSITORY)
        api = _authority_api()
        original = api
        def wrong_head(path):
            response = original(path)
            if f"/pulls/{MODULE.PLANNER_PR}" in path:
                response = dict(response)
                response["head"] = {"sha": "6" * 40, "repo": {"full_name": REPOSITORY}}
            return response
        with patch.object(MODULE, "gh_api", side_effect=wrong_head):
            with self.assertRaisesRegex(MODULE.AdmissionError, "source head"):
                MODULE.verify_authority_chain(REPOSITORY)

        for field, replacement in (
            ("trusted_integration_base", "6" * 40),
            ("source_scope_base", "7" * 40),
        ):
            api = _authority_api()
            original = api

            def wrong_base(path, field=field, replacement=replacement):
                response = original(path)
                if path.endswith(f"issues/comments/{MODULE.PLANNER_COMMENT}"):
                    response = dict(response)
                    if field == "trusted_integration_base":
                        response["body"] = response["body"].replace(
                            f"trusted integration base={CHECKER_BASE}",
                            f"trusted integration base={replacement}",
                        )
                    else:
                        response["body"] = response["body"].replace(
                            f"source_scope_base={CHECKER_SCOPE}",
                            f"source_scope_base={replacement}",
                        )
                return response

            with self.subTest(field=field), patch.object(MODULE, "gh_api", side_effect=wrong_base):
                with self.assertRaisesRegex(MODULE.AdmissionError, "base|merge-base"):
                    MODULE.verify_authority_chain(REPOSITORY)

        api = _authority_api()
        original = api

        def wrong_normative_base(path):
            response = original(path)
            if path.endswith(f"issues/comments/{MODULE.NORMATIVE_COMMENT}"):
                response = dict(response)
                response["body"] = response["body"].replace(
                    f"trusted_predecessor/source_scope_base={'9' * 40}",
                    "trusted_predecessor/source_scope_base=" + "6" * 40,
                )
            return response

        with patch.object(MODULE, "gh_api", side_effect=wrong_normative_base):
            with self.assertRaisesRegex(MODULE.AdmissionError, "base"):
                MODULE.verify_authority_chain(REPOSITORY)

        api = _authority_api()
        original = api

        def wrong_integration_run(path):
            response = original(path)
            if path.endswith(f"/actions/runs/{INTEGRATION_RUN}"):
                response = dict(response)
                response["head_sha"] = "8" * 40
            return response

        with patch.object(MODULE, "gh_api", side_effect=wrong_integration_run):
            with self.assertRaisesRegex(MODULE.AdmissionError, "integration run base"):
                MODULE.verify_authority_chain(REPOSITORY)

    def test_authority_issue_readback_rejects_duplicate_or_malformed_uid_lines(self):
        cases = (
            ("duplicate", lambda body: body + f"task_uid: {MODULE.PLANNER_TASK_UID}\n"),
            ("malformed", lambda body: body.replace(
                f"task_uid: {MODULE.PLANNER_TASK_UID}", "task_uid:\t" + MODULE.PLANNER_TASK_UID
            )),
            ("extra whitespace", lambda body: body.replace(
                f"task_uid: {MODULE.PLANNER_TASK_UID}", "task_uid:  " + MODULE.PLANNER_TASK_UID
            )),
        )
        for label, mutate in cases:
            api = _authority_api()

            def tampered_issue(path, *, api=api, mutate=mutate):
                response = api(path)
                if path.endswith(f"/issues/{MODULE.PLANNER_ISSUE}"):
                    response = dict(response)
                    response["body"] = mutate(response["body"])
                return response

            with self.subTest(label=label), patch.object(MODULE, "gh_api", side_effect=tampered_issue):
                with self.assertRaisesRegex(MODULE.AdmissionError, "UID field"):
                    MODULE.verify_authority_chain(REPOSITORY)

    def test_integration_artifact_tested_tree_divergence_is_rejected(self):
        for field, value, message in (
            ("tested_tree", "0" * 40, "tested tree"),
            ("run_attempt", 2, "envelope identity"),
            ("workflow_ref", "eng-cc/oasis7/.github/workflows/other.yml@refs/heads/main", "envelope identity"),
            ("workflow_sha", "9" * 40, "envelope identity"),
        ):
            artifacts = _profile_artifacts()
            artifacts["envelope"]["value"] = dict(artifacts["envelope"]["value"], **{field: value})
            with self.subTest(field=field), \
                 patch.object(MODULE, "gh_api", side_effect=_authority_api()), \
                 patch.object(MODULE, "_read_profile_artifacts", return_value=artifacts):
                with self.assertRaisesRegex(MODULE.AdmissionError, message):
                    MODULE.verify_authority_chain(REPOSITORY)

    def test_empty_integration_results_require_an_explicit_empty_validated_plan(self):
        artifacts = _profile_artifacts()
        plan = dict(
            artifacts["plan"]["value"],
            items=[],
            selected_items=[],
            disposition_validated=True,
        )
        results = []
        artifacts["plan"] = {
            "value": plan,
            "bytes": json.dumps(plan, sort_keys=True, separators=(",", ":")).encode(),
        }
        artifacts["results"] = {
            "value": results,
            "bytes": json.dumps(results, sort_keys=True, separators=(",", ":")).encode(),
        }
        artifacts["envelope"]["value"]["plan_digest"] = _sha256(artifacts["plan"]["bytes"])
        artifacts["envelope"]["value"]["results_digest"] = _sha256(artifacts["results"]["bytes"])

        with patch.object(MODULE, "gh_api", side_effect=_authority_api()), \
             patch.object(MODULE, "_read_profile_artifacts", return_value=artifacts):
            MODULE.verify_authority_chain(REPOSITORY)

        plan["selected_items"] = ["profile"]
        artifacts["plan"] = {
            "value": plan,
            "bytes": json.dumps(plan, sort_keys=True, separators=(",", ":")).encode(),
        }
        artifacts["envelope"]["value"]["plan_digest"] = _sha256(artifacts["plan"]["bytes"])
        with patch.object(MODULE, "gh_api", side_effect=_authority_api()), \
             patch.object(MODULE, "_read_profile_artifacts", return_value=artifacts):
            with self.assertRaisesRegex(MODULE.AdmissionError, "empty results"):
                MODULE.verify_authority_chain(REPOSITORY)

    def test_integration_artifact_byte_tamper_is_rejected_against_envelope_digest(self):
        artifacts = _profile_artifacts()
        artifacts["plan"]["bytes"] += b" "
        with patch.object(MODULE, "gh_api", side_effect=_authority_api()), \
             patch.object(MODULE, "_read_profile_artifacts", return_value=artifacts):
            with self.assertRaisesRegex(MODULE.AdmissionError, "plan artifact digest mismatch"):
                MODULE.verify_authority_chain(REPOSITORY)

    def test_profile_artifact_reader_binds_exact_run_artifacts(self):
        # This case validates the reader itself; other tests replace it with
        # pre-decoded artifacts to avoid network and archive I/O.
        self.profile_patch.stop()
        payloads = _profile_artifacts()
        members = {
            "envelope": ("cargo-package-profile-envelope", "cargo-package-profile-envelope.json"),
            "plan": ("cargo-package-profile-plan", "cargo-package-profile-plan.json"),
            "results": ("cargo-package-profile-results", "cargo-package-profile-results.json"),
            "receipt": ("cargo-package-profile-receipt", "cargo-package-profile-receipt.json"),
        }
        artifacts = []
        archives = {}
        for index, (key, (name, member)) in enumerate(members.items(), start=1):
            artifacts.append({"id": index, "name": name, "expired": False,
                              "workflow_run": {"id": INTEGRATION_RUN}})
            stream = io.BytesIO()
            with zipfile.ZipFile(stream, "w", zipfile.ZIP_DEFLATED) as archive:
                archive.writestr(member, payloads[key]["bytes"])
            archives[index] = stream.getvalue()

        def api(path):
            if "/actions/runs/" in path and path.endswith("artifacts?per_page=100"):
                return {"artifacts": artifacts}
            raise AssertionError(path)

        def download(path):
            return archives[int(path.rsplit("/", 2)[1])]

        with patch.object(MODULE, "gh_api", side_effect=api), patch.object(MODULE, "_gh_download", side_effect=download):
            loaded = MODULE._read_profile_artifacts(REPOSITORY, INTEGRATION_RUN)
        self.assertEqual(TESTED_TREE, loaded["envelope"]["value"]["tested_tree"])

        for label, mutate, message in (
            ("duplicate", lambda items: items.append(dict(items[0])), "missing or ambiguous"),
            ("wrong run", lambda items: items[0].update(workflow_run={"id": INTEGRATION_RUN + 1}), "another run"),
            ("expired", lambda items: items[0].update(expired=True), "expired"),
        ):
            changed_items = json.loads(json.dumps(artifacts))
            mutate(changed_items)

            def changed_api(path, *, changed_items=changed_items):
                if path.endswith("artifacts?per_page=100"):
                    return {"artifacts": changed_items}
                raise AssertionError(path)

            with self.subTest(label=label), \
                 patch.object(MODULE, "gh_api", side_effect=changed_api), \
                 patch.object(MODULE, "_gh_download", side_effect=download):
                with self.assertRaisesRegex(MODULE.AdmissionError, message):
                    MODULE._read_profile_artifacts(REPOSITORY, INTEGRATION_RUN)

    def test_authority_chain_rejects_missing_planner_verification_evidence(self):
        api = _authority_api()
        original = api
        def missing_verification(path):
            response = original(path)
            if path.endswith(f"issues/comments/{MODULE.PLANNER_COMMENT}"):
                response = dict(response)
                response["body"] = response["body"].replace(
                    "Exact-head required-gate and strict integration passed",
                    "Exact-head required-gate status unknown",
                )
            return response
        with patch.object(MODULE, "gh_api", side_effect=missing_verification):
            with self.assertRaisesRegex(MODULE.AdmissionError, "verification"):
                MODULE.verify_authority_chain(REPOSITORY)

    def test_authority_chain_rejects_missing_pending_or_failed_required_gate(self):
        for status, conclusion in ((None, None), ("in_progress", None), ("completed", "failure")):
            api = _authority_api()
            original = api

            def tampered_check(path, status=status, conclusion=conclusion):
                response = original(path)
                if "/check-runs?" in path:
                    response = dict(response)
                    check = dict(response["check_runs"][0])
                    if status is None:
                        check.pop("status", None)
                    else:
                        check["status"] = status
                    if conclusion is None:
                        check.pop("conclusion", None)
                    else:
                        check["conclusion"] = conclusion
                    response["check_runs"] = [check]
                return response

            with self.subTest(status=status, conclusion=conclusion), patch.object(
                MODULE, "gh_api", side_effect=tampered_check
            ):
                with self.assertRaisesRegex(MODULE.AdmissionError, "completed successfully"):
                    MODULE.verify_authority_chain(REPOSITORY)

    def test_preflight_binds_scope_and_command_digest(self):
        self.assertTrue(hasattr(MODULE, "build_preflight"))
        expected = ["python3", "scripts/pm/check-cargo-package-scope"]
        self.assertEqual(MODULE.command_digest(expected), MODULE.command_digest(expected))

    def test_preflight_normal_path_binds_live_checker_and_runner_identity(self):
        planner_authority = {
            "authority_path": MODULE.PLANNER_PATH,
            "authority_bytes": _planner_bytes(),
            "authority_bytes_sha256": "sha256:" + hashlib.sha256(_planner_bytes()).hexdigest(),
            "authority_size": len(_planner_bytes()),
            "merged_commit": PLANNER_COMMIT,
            "source_head": "8" * 40,
        }
        chain = {"planner": planner_authority, "normative": {"stage": "normative_source"}}
        checker = {
            "task_uid": TASK_UID, "changed_paths": sorted(MODULE.CHECKER_SCOPE),
        }
        execution = self._checker_execution_fixture(CHECKER_SCOPE)
        with patch.object(MODULE, "verify_authority_chain", return_value=chain), \
             patch.object(MODULE, "classify_checker_stage", return_value=True), \
             patch.object(MODULE, "verify_executing_planner", return_value={"bytes_sha256": planner_authority["authority_bytes_sha256"]}), \
             patch.object(MODULE, "verify_checker_pr", return_value=checker), \
             patch.object(MODULE, "_git", return_value=TESTED_TREE), \
             patch.object(MODULE, "_checker_execution_context", return_value=execution):
            receipt = MODULE.build_preflight(
                repository=REPOSITORY, pr_number=CHECKER_PR, task_uid=TASK_UID,
                base_oid=CHECKER_BASE, head_oid=CHECKER_HEAD, scope_base_oid=CHECKER_SCOPE,
                repo_root=Path("."), planner_path=Path("planner.py"),
                checker_path=Path("checker"), policy_path=Path("policy"), primary_package="auto",
                run_id="99", run_attempt="1", workflow_ref="workflow", workflow_sha="w" * 40,
            )
        self.assertEqual("preflight", receipt["phase"])
        self.assertEqual(TESTED_TREE, receipt["tested_tree"])
        self.assertEqual(TASK_UID, receipt["task_uid"])
        self.assertEqual(execution, receipt["checker_execution"])
        self.assertEqual(receipt["preflight_digest"], MODULE.preflight_digest(receipt))

    @staticmethod
    def _checker_execution_fixture(scope_oid):
        checker_bytes = b"fixture trusted checker source"
        policy_bytes = b"fixture trusted policy source"
        blob_oid = lambda data: hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()
        return {
            "source_oid": scope_oid,
            "checker_source_path": MODULE.CHECKER_EXECUTION_PATH,
            "checker_source_blob": blob_oid(checker_bytes),
            "checker_source_size": len(checker_bytes),
            "checker_source_bytes_sha256": "sha256:" + hashlib.sha256(checker_bytes).hexdigest(),
            "policy_source_path": MODULE.CHECKER_POLICY_PATH,
            "policy_source_blob": blob_oid(policy_bytes),
            "policy_source_size": len(policy_bytes),
            "policy_source_bytes_sha256": "sha256:" + hashlib.sha256(policy_bytes).hexdigest(),
            "python": sys.executable,
            "checker_executable": "/tmp/trusted-check-cargo-package-scope",
            "policy_executable": "/tmp/cargo-package-scope-policy.json",
            "repo_root": "/tmp/oasis7",
            "primary_package": "auto",
        }

    def test_postrun_rejects_missing_failed_wrong_head_and_dirty_receipt(self):
        preflight = {
            "schema": MODULE.SCHEMA, "phase": "preflight", "repository": REPOSITORY,
            "task_uid": TASK_UID, "pr_number": CHECKER_PR, "base_oid": CHECKER_BASE,
            "head_oid": CHECKER_HEAD, "scope_base_oid": CHECKER_SCOPE,
            "tested_tree": TESTED_TREE, "checker_command": ["python3", "checker"],
            "checker_command_digest": MODULE.command_digest(["python3", "checker"]),
            "run_id": "99", "run_attempt": "1", "runner": {"run_id": "99", "run_attempt": "1"},
        }
        digest = MODULE.preflight_digest(preflight)
        with self.assertRaisesRegex(MODULE.AdmissionError, "status|result"):
            MODULE.verify_postrun(preflight, digest, status=None, exit_code=None,
                                  run_id="99", run_attempt="1")
        with self.assertRaisesRegex(MODULE.AdmissionError, "failed|exit"):
            MODULE.verify_postrun(preflight, digest, status="failed", exit_code=1,
                                  run_id="99", run_attempt="1")
        with self.assertRaisesRegex(MODULE.AdmissionError, "digest"):
            broken = dict(preflight, head_oid="8" * 40)
            MODULE.verify_postrun(broken, digest, status="passed",
                                  exit_code=0, run_id="99", run_attempt="1")
        with self.assertRaisesRegex(MODULE.AdmissionError, "digest"):
            MODULE.verify_postrun(preflight, "sha256:" + "0" * 64, status="passed",
                                  exit_code=0, run_id="99", run_attempt="1")

    def test_postrun_rejects_wrong_runner_attempt(self):
        preflight = {
            "schema": MODULE.SCHEMA, "phase": "preflight", "repository": REPOSITORY,
            "base_oid": CHECKER_BASE, "head_oid": CHECKER_HEAD,
            "scope_base_oid": CHECKER_SCOPE, "tested_tree": TESTED_TREE,
            "checker_command_digest": "sha256:" + "a" * 64,
            "runner": {"run_id": "99", "run_attempt": "1", "workflow_ref": "workflow", "workflow_sha": "w" * 40},
        }
        with self.assertRaisesRegex(MODULE.AdmissionError, "runner"):
            MODULE.verify_postrun(preflight, MODULE.preflight_digest(preflight),
                                  status="passed", exit_code=0, run_id="99", run_attempt="2")

    def test_postrun_binds_live_check_app_and_run_identity(self):
        check_head = "5" * 40
        response = {
            "check_runs": [{
                "id": 123, "name": "required-gate", "head_sha": check_head,
                "details_url": "https://github.com/eng-cc/oasis7/actions/runs/99/job/1",
                "app": {"id": 15368, "slug": "github-actions"},
            }]
        }
        with patch.object(MODULE, "gh_api", return_value=response):
            check = MODULE.verify_live_check_identity(REPOSITORY, check_head, "99")
        self.assertEqual(15368, check["check_app_id"])
        self.assertEqual(123, check["check_run_id"])
        with patch.object(MODULE, "gh_api", return_value={"check_runs": []}):
            with self.assertRaisesRegex(MODULE.AdmissionError, "missing or ambiguous"):
                MODULE.verify_live_check_identity(REPOSITORY, check_head, "99")
        wrong_run = json.loads(json.dumps(response))
        wrong_run["check_runs"][0]["details_url"] = (
            f"https://github.com/{REPOSITORY}/actions/runs/990/job/1"
        )
        with patch.object(MODULE, "gh_api", return_value=wrong_run):
            with self.assertRaisesRegex(MODULE.AdmissionError, "missing or ambiguous"):
                MODULE.verify_live_check_identity(REPOSITORY, check_head, "99")
        wrong_head = json.loads(json.dumps(response))
        wrong_head["check_runs"][0]["head_sha"] = "6" * 40
        with patch.object(MODULE, "gh_api", return_value=wrong_head):
            with self.assertRaisesRegex(MODULE.AdmissionError, "head identity mismatch"):
                MODULE.verify_live_check_identity(REPOSITORY, check_head, "99")
        wrong_app = json.loads(json.dumps(response))
        wrong_app["check_runs"][0]["app"] = {"id": 42, "slug": "untrusted-app"}
        with patch.object(MODULE, "gh_api", return_value=wrong_app):
            with self.assertRaisesRegex(MODULE.AdmissionError, "app identity mismatch"):
                MODULE.verify_live_check_identity(REPOSITORY, check_head, "99")

    def test_postrun_rejects_duplicate_live_check_run_matches(self):
        check_head = "5" * 40
        check = {
            "id": 123, "name": "required-gate", "head_sha": check_head,
            "details_url": f"https://github.com/{REPOSITORY}/actions/runs/99/job/1",
            "app": {"id": 15368, "slug": "github-actions"},
        }
        duplicate = dict(check, id=124)
        with patch.object(MODULE, "gh_api", return_value={"check_runs": [check, duplicate]}):
            with self.assertRaisesRegex(MODULE.AdmissionError, "missing or ambiguous"):
                MODULE.verify_live_check_identity(REPOSITORY, check_head, "99")

    def test_postrun_records_only_unique_current_producer_job_identity(self):
        job = {
            "id": 55, "run_id": 99, "run_attempt": 1,
            "name": MODULE.CHECKER_STAGE_PRODUCER_JOB,
            "status": "in_progress", "conclusion": None,
            "head_sha": "5" * 40, "labels": ["ubuntu-24.04"],
            "check_run_url": f"https://api.github.com/repos/{REPOSITORY}/check-runs/56",
        }
        with patch.object(MODULE, "gh_api", return_value={"total_count": 1, "jobs": [job]}):
            identity = MODULE.verify_live_producer_job_identity(
                REPOSITORY, "99", "1", MODULE.CHECKER_STAGE_PRODUCER_JOB
            )
        self.assertEqual({
            "job_name": MODULE.CHECKER_STAGE_PRODUCER_JOB,
            "job_id": 55, "check_run_id": 56, "workflow_run_id": 99,
            "run_attempt": 1, "head_sha": "5" * 40,
        }, identity)
        invalid = (
            (dict(job, name="other"), 1, "missing or ambiguous"),
            (dict(job, run_attempt=2), 1, "current workflow attempt"),
            (dict(job, status="completed", conclusion="success"), 1, "current workflow attempt"),
            (dict(job, head_sha="x" * 40), 1, "OID"),
            (dict(job, labels=["self-hosted"]), 1, "runner identity"),
            (dict(job, check_run_url="https://example.invalid/check/56"), 1, "check-run URL"),
            (dict(job), 2, "pagination is incomplete"),
        )
        for bad_job, total_count, message in invalid:
            with self.subTest(job=bad_job, total_count=total_count), patch.object(
                MODULE, "gh_api", return_value={"total_count": total_count, "jobs": [bad_job]}
            ):
                with self.assertRaisesRegex(MODULE.AdmissionError, message):
                    MODULE.verify_live_producer_job_identity(
                        REPOSITORY, "99", "1", MODULE.CHECKER_STAGE_PRODUCER_JOB
                    )
        with patch.object(MODULE, "gh_api", return_value={"total_count": 2, "jobs": [job, dict(job, id=57)]}):
            with self.assertRaisesRegex(MODULE.AdmissionError, "missing or ambiguous"):
                MODULE.verify_live_producer_job_identity(
                    REPOSITORY, "99", "1", MODULE.CHECKER_STAGE_PRODUCER_JOB
                )
        with self.assertRaisesRegex(MODULE.AdmissionError, "name is not trusted"):
            MODULE.verify_live_producer_job_identity(REPOSITORY, "99", "1", "candidate-job")

    def test_postrun_receipt_is_durable_and_complete(self):
        normative_authority = {**MODULE.NORMATIVE_AUTHORITY_EXPECTED, "stage": "normative_source"}
        planner_authority = {
            **MODULE.PLANNER_AUTHORITY_EXPECTED,
            "stage": "planner_authority",
            "trusted_integration_run_id": MODULE.PLANNER_INTEGRATION_RUN,
            "verification_evidence": "verified live planner publication fixture",
        }
        execution = self._checker_execution_fixture(CHECKER_SCOPE)
        command = MODULE._checker_command(
            execution, scope_base_oid=CHECKER_SCOPE, head_oid=CHECKER_HEAD
        )
        preflight = {
            "schema": MODULE.SCHEMA, "phase": "preflight", "repository": REPOSITORY,
            "task_uid": TASK_UID, "pr_number": CHECKER_PR, "base_oid": CHECKER_BASE,
            "head_oid": CHECKER_HEAD, "scope_base_oid": CHECKER_SCOPE,
            "tested_tree": TESTED_TREE, "checker_command": command,
            "checker_command_digest": MODULE.command_digest(command),
            "checker_execution": execution,
            "normative_authority": normative_authority,
            "planner_authority": planner_authority,
            "runner": {"run_id": "99", "run_attempt": "1", "workflow_ref": "workflow", "workflow_sha": "w" * 40},
        }
        receipt = MODULE.build_postrun_receipt(
            preflight,
            {"normative": normative_authority, "planner": planner_authority},
            {"path": MODULE.PLANNER_PATH, "authority_path": MODULE.PLANNER_PATH,
             "source_ref": f"refs/pull/{MODULE.PLANNER_PR}/head",
             "source_head": MODULE.PLANNER_AUTHORITY_EXPECTED["source_head"],
             "merged_commit": MODULE.PLANNER_AUTHORITY_EXPECTED["merged_commit"],
             "bytes_sha256": MODULE.PLANNER_AUTHORITY_EXPECTED["authority_bytes_sha256"],
             "size": MODULE.PLANNER_AUTHORITY_EXPECTED["authority_size"]},
            {"check_name": "required-gate", "check_app_id": 15368,
             "check_app_slug": "github-actions", "check_run_id": 123,
             "check_head": CHECKER_HEAD, "workflow_run_id": "99"},
            {"job_name": MODULE.CHECKER_STAGE_PRODUCER_JOB, "job_id": 55,
             "check_run_id": 56, "workflow_run_id": 99, "run_attempt": 1,
             "head_sha": CHECKER_HEAD},
            status="passed", exit_code=0,
        )
        self.assertEqual("provisional", receipt["activation"])
        for field in ("normative_authority", "planner_authority", "executing_planner", "check", "producer_job", "result"):
            self.assertIn(field, receipt)
        self.assertEqual("passed", receipt["result"]["status"])
        self.assertEqual(preflight["checker_execution"], receipt["checker_execution"])
        self.assertIs(MODULE.verify_durable_postrun_receipt(receipt), receipt)
        tampered = dict(receipt, head_oid="9" * 40)
        with self.assertRaisesRegex(MODULE.AdmissionError, "digest"):
            MODULE.verify_durable_postrun_receipt(tampered)

        changed_authorities = {
            "normative": normative_authority,
            "planner": {**planner_authority, "merged_commit": "9" * 40},
        }
        with self.assertRaisesRegex(MODULE.AdmissionError, "planner_authority identity changed"):
            MODULE._assert_preflight_authorities_unchanged(preflight, changed_authorities)

    def test_durable_postrun_rejects_recomputed_public_digest_with_forged_authority(self):
        normative_authority = {**MODULE.NORMATIVE_AUTHORITY_EXPECTED, "stage": "normative_source"}
        planner_authority = {
            **MODULE.PLANNER_AUTHORITY_EXPECTED,
            "stage": "planner_authority",
            "trusted_integration_run_id": MODULE.PLANNER_INTEGRATION_RUN,
            "verification_evidence": "verified live planner publication fixture",
        }
        execution = self._checker_execution_fixture(CHECKER_SCOPE)
        command = MODULE._checker_command(
            execution, scope_base_oid=CHECKER_SCOPE, head_oid=CHECKER_HEAD
        )
        preflight = {
            "schema": MODULE.SCHEMA, "phase": "preflight", "repository": REPOSITORY,
            "task_uid": TASK_UID, "pr_number": CHECKER_PR, "base_oid": CHECKER_BASE,
            "head_oid": CHECKER_HEAD, "scope_base_oid": CHECKER_SCOPE,
            "tested_tree": TESTED_TREE, "checker_command": command,
            "checker_command_digest": MODULE.command_digest(command),
            "checker_execution": execution,
            "normative_authority": normative_authority,
            "planner_authority": planner_authority,
            "runner": {"run_id": "99", "run_attempt": "1", "workflow_ref": "workflow", "workflow_sha": "w" * 40},
        }
        receipt = MODULE.build_postrun_receipt(
            preflight,
            {"normative": normative_authority, "planner": planner_authority},
            {"path": MODULE.PLANNER_PATH, "authority_path": MODULE.PLANNER_PATH,
             "source_ref": f"refs/pull/{MODULE.PLANNER_PR}/head",
             "source_head": MODULE.PLANNER_AUTHORITY_EXPECTED["source_head"],
             "merged_commit": MODULE.PLANNER_AUTHORITY_EXPECTED["merged_commit"],
             "bytes_sha256": MODULE.PLANNER_AUTHORITY_EXPECTED["authority_bytes_sha256"],
             "size": MODULE.PLANNER_AUTHORITY_EXPECTED["authority_size"]},
            {"check_name": "required-gate", "check_app_id": 15368,
             "check_app_slug": "github-actions", "check_run_id": 123,
             "check_head": CHECKER_HEAD, "workflow_run_id": "99"},
            {"job_name": MODULE.CHECKER_STAGE_PRODUCER_JOB, "job_id": 55,
             "check_run_id": 56, "workflow_run_id": 99, "run_attempt": 1,
             "head_sha": CHECKER_HEAD},
            status="passed", exit_code=0,
        )
        for field, forged in (
            ("normative_authority", {}),
            ("planner_authority", {}),
            ("planner_authority", {**planner_authority, "merged_commit": "9" * 40}),
            ("planner_authority", {**planner_authority, "untrusted_extra": "candidate"}),
        ):
            with self.subTest(field=field, forged=forged):
                tampered = json.loads(json.dumps(receipt))
                tampered[field] = forged
                tampered["receipt_digest"] = MODULE.durable_receipt_digest(tampered)
                with self.assertRaisesRegex(MODULE.AdmissionError, "authority"):
                    MODULE.verify_durable_postrun_receipt(tampered)

        forged_planner = json.loads(json.dumps(receipt))
        forged_planner["executing_planner"]["bytes_sha256"] = "sha256:" + "9" * 64
        forged_planner["receipt_digest"] = MODULE.durable_receipt_digest(forged_planner)
        with self.assertRaisesRegex(MODULE.AdmissionError, "planner.*authority|authority.*planner"):
            MODULE.verify_durable_postrun_receipt(forged_planner)

        forged_command = json.loads(json.dumps(receipt))
        command_args = forged_command["result"]["command"]
        command_args[command_args.index("--head") + 1] = "9" * 40
        command_digest = MODULE.command_digest(command_args)
        forged_command["checker_command_digest"] = command_digest
        forged_command["result"]["command_digest"] = command_digest
        forged_command["receipt_digest"] = MODULE.durable_receipt_digest(forged_command)
        with self.assertRaisesRegex(MODULE.AdmissionError, "checker command"):
            MODULE.verify_durable_postrun_receipt(forged_command)

        forged_producer = json.loads(json.dumps(receipt))
        forged_producer["producer_job"] = {}
        forged_producer["receipt_digest"] = MODULE.durable_receipt_digest(forged_producer)
        with self.assertRaisesRegex(MODULE.AdmissionError, "producer job"):
            MODULE.verify_durable_postrun_receipt(forged_producer)

    def test_postrun_receipt_authority_rechecks_live_receipts_and_command_sources(self):
        checker_bytes = b"fixture trusted checker source"
        policy_bytes = b"fixture trusted policy source"
        normative = {**MODULE.NORMATIVE_AUTHORITY_EXPECTED, "stage": "normative_source"}
        planner = {
            **MODULE.PLANNER_AUTHORITY_EXPECTED,
            "stage": "planner_authority",
            "trusted_integration_run_id": MODULE.PLANNER_INTEGRATION_RUN,
            "verification_evidence": "verified live planner publication fixture",
        }
        live_authorities = {
            "normative": {**normative, "authority_bytes": b"normative live bytes"},
            "planner": {**planner, "authority_bytes": _planner_bytes()},
        }
        receipt = {
            "normative_authority": normative,
            "planner_authority": planner,
            "executing_planner": {
                "authority_path": MODULE.PLANNER_PATH,
                "merged_commit": MODULE.PLANNER_AUTHORITY_EXPECTED["merged_commit"],
                "source_head": MODULE.PLANNER_AUTHORITY_EXPECTED["source_head"],
                "source_ref": f"refs/pull/{MODULE.PLANNER_PR}/head",
                "bytes_sha256": MODULE.PLANNER_AUTHORITY_EXPECTED["authority_bytes_sha256"],
                "size": MODULE.PLANNER_AUTHORITY_EXPECTED["authority_size"],
            },
            "scope_base_oid": CHECKER_SCOPE,
            "checker_execution": {
                **self._checker_execution_fixture(CHECKER_SCOPE),
                "checker_source_bytes_sha256": "sha256:" + hashlib.sha256(checker_bytes).hexdigest(),
                "policy_source_bytes_sha256": "sha256:" + hashlib.sha256(policy_bytes).hexdigest(),
            },
        }
        def live_source(path):
            data = checker_bytes if MODULE.CHECKER_EXECUTION_PATH in path else policy_bytes
            blob_oid = hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()
            return {"encoding": "base64", "content": base64.b64encode(data).decode("ascii"),
                    "sha": blob_oid, "size": len(data)}

        with patch.object(MODULE, "verify_authority_chain", return_value=live_authorities), \
             patch.object(MODULE, "verify_executing_planner", return_value=receipt["executing_planner"]), \
             patch.object(MODULE, "gh_api", side_effect=live_source):
            MODULE.verify_postrun_receipt_authority(receipt, REPOSITORY, Path("."))

        forged = json.loads(json.dumps(receipt))
        forged["checker_execution"]["checker_source_bytes_sha256"] = "sha256:" + "f" * 64
        with patch.object(MODULE, "verify_authority_chain", return_value=live_authorities), \
             patch.object(MODULE, "verify_executing_planner", return_value=receipt["executing_planner"]), \
             patch.object(MODULE, "gh_api", side_effect=live_source):
            with self.assertRaisesRegex(MODULE.AdmissionError, "command source digest"):
                MODULE.verify_postrun_receipt_authority(forged, REPOSITORY, Path("."))


if __name__ == "__main__":
    unittest.main()
