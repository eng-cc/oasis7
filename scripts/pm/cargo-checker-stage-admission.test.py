#!/usr/bin/env python3
import base64
import hashlib
import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


MODULE_PATH = Path(__file__).with_name("cargo_checker_stage_admission.py")
SPEC = importlib.util.spec_from_file_location("cargo_checker_stage_admission", MODULE_PATH)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


REPOSITORY = "eng-cc/oasis7"
NORMATIVE_COMMIT = "a" * 40
NORMATIVE_TREE = "b" * 40
NORMATIVE_BLOB = "c" * 40
PLANNER_COMMIT = "d" * 40
PLANNER_TREE = "e" * 40
PLANNER_BLOB = "f" * 40
CHECKER_BASE = "1" * 40
CHECKER_HEAD = "2" * 40
CHECKER_SCOPE = "3" * 40
TESTED_TREE = "4" * 40
TASK_UID = "task_be264ac2833044969d3c2c50b2b83cea"


def _normative_bytes():
    return b'<a id="cargo-checker-authority-upgrade"></a> trusted\n'


def _planner_bytes():
    return (
        b"def _validate_approved_normative_source(\n"
        b"    value\n"
        b"\n"
        b"def _extract(\n"
    )


def _content(data, oid):
    return {
        "encoding": "base64",
        "content": base64.b64encode(data).decode() + "\n",
        "sha": oid,
        "size": len(data),
    }


def _comment(stage, commit, tree, blob, data, task_uid, pr, comment, fragment):
    digest = hashlib.sha256(data).hexdigest()
    fragment_digest = hashlib.sha256(fragment).hexdigest()
    if stage == "normative_source":
        return (
            "Post-merge approved_normative_source readback "
            "(stage=normative_source; immutable, not candidate authority): "
            f"repository={REPOSITORY}; default_branch=main; task_uid={task_uid}; "
            f"PR={pr}; trusted_predecessor/source_scope_base={CHECKER_SCOPE}; "
            f"source_head={'7' * 40}; "
            "predecessor authority path=doc/engineering/workflow/source-of-truth.md; "
            f"predecessor file sha256={'9' * 64}. GitHub live PR readback reports "
            f"MERGED into commit={commit}; live git/commits API tree={tree}; "
            f"live contents API path=doc/engineering/workflow/source-of-truth.md "
            f"blob={blob}, size={len(data)}, decoded bytes sha256={digest}. "
            "Stable fragment anchor=cargo-checker-authority-upgrade, "
            f"sha256 including final LF={fragment_digest}."
        )
    return (
        "Post-merge server readback for planner authority stage "
        f"(stage=approved_planner_authority): repository={REPOSITORY}; "
        "default_branch=main; task_uid=task_e21604f5cdb3476c8e146332a68a05b4; "
        "issue=3818; pr=3821; source_head=" + "8" * 40 + "; "
        f"source_scope_base={CHECKER_SCOPE}; trusted_integration_base={CHECKER_BASE}; "
        f"predecessor_normative_commit={NORMATIVE_COMMIT}; merged_commit={commit}; "
        f"merged_tree={tree}; authority_path=scripts/pm/cargo_package_profile_planner.py; "
        f"authority_blob={blob}; authority_size={len(data)}; "
        f"authority_bytes_sha256={digest}; stable_fragment=def _validate_approved_normative_source( "
        f"through before next top-level def _extract( ; stable_fragment_sha256={fragment_digest}. "
        "Verification commands/results bound to this exact source head: "
        "python3 scripts/pm/cargo-package-profile-planner.test.py 23/23 PASS; "
        "python3 scripts/pm/check-cargo-package-scope.test.py 13/13 PASS; "
        "./scripts/pm/lint.sh PASS; ./scripts/doc-governance-check.sh PASS; "
        "./scripts/pm/workflow-lint.sh --task-uid task_e21604f5cdb3476c8e146332a68a05b4 --phase current PASS; "
        "git diff --check PASS; "
        "trusted exact integration run 35463292968 at base " + CHECKER_BASE +
        " PASS, tested_tree=" + PLANNER_TREE + "; terminal finalizer PASS."
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
        3827: {"number": 3827, "state": "open", "merged": False, "draft": True,
               "body": f"Task: {TASK_UID}\n", "base": {"ref": "main", "sha": CHECKER_BASE,
               "repo": {"full_name": REPOSITORY}}, "head": {"sha": CHECKER_HEAD,
               "repo": {"full_name": REPOSITORY}},},
    }
    def api(path):
        if path.endswith(f"issues/comments/{MODULE.NORMATIVE_COMMENT}"):
            return {"body": bodies[MODULE.NORMATIVE_COMMENT]}
        if path.endswith(f"issues/comments/{MODULE.PLANNER_COMMENT}"):
            return {"body": bodies[MODULE.PLANNER_COMMENT]}
        if "/commits/" in path:
            return commits[path.rsplit("/", 1)[1]]
        if "/contents/" in path:
            commit = path.split("?ref=", 1)[1]
            return contents[commit]
        if path.endswith("/pulls/3827/files?per_page=100&page=1"):
            return [{"filename": value} for value in MODULE.CHECKER_SCOPE]
        if "/pulls/" in path:
            return prs[int(path.rsplit("/", 1)[1])]
        if path == f"repos/{REPOSITORY}":
            return {"default_branch": "main"}
        raise AssertionError(path)
    return api


class CheckerStageAdmissionTest(unittest.TestCase):
    def test_authority_chain_reads_both_fixed_live_server_readbacks(self):
        with patch.object(MODULE, "gh_api", side_effect=_authority_api()):
            chain = MODULE.verify_authority_chain(REPOSITORY)
        self.assertEqual(NORMATIVE_COMMIT, chain["normative"]["merged_commit"])
        self.assertEqual(PLANNER_COMMIT, chain["planner"]["merged_commit"])
        self.assertEqual(REPOSITORY, chain["planner"]["repository"])

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

    def test_checker_identity_rejects_wrong_head_and_scope(self):
        api = _authority_api()
        with patch.object(MODULE, "gh_api", side_effect=api):
            with self.assertRaisesRegex(MODULE.AdmissionError, "head"):
                MODULE.verify_checker_pr(
                    REPOSITORY, 3827, TASK_UID, CHECKER_BASE, "9" * 40,
                    CHECKER_SCOPE, TESTED_TREE, Path("."),
                )

    def test_checker_identity_rejects_wrong_current_task_uid_even_when_body_is_live(self):
        api = _authority_api()
        with patch.object(MODULE, "gh_api", side_effect=api), \
             patch.object(MODULE, "_git", side_effect=[CHECKER_SCOPE, TESTED_TREE]):
            with self.assertRaisesRegex(MODULE.AdmissionError, "task identity"):
                MODULE.verify_checker_pr(
                    REPOSITORY, 3827, "task_00000000000000000000000000000000",
                    CHECKER_BASE, CHECKER_HEAD,
                    CHECKER_SCOPE, TESTED_TREE, Path("."),
                )

    def test_authority_chain_rejects_wrong_predecessor_task_issue_or_pr_head(self):
        for replacement, message in (
            ("task_e21604f5cdb3476c8e146332a68a05b4", "task"),
            ("issue=3818", "issue"),
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
                        body = body.replace(replacement, "issue=9999")
                    response["body"] = body
                return response
            with self.subTest(message=message), patch.object(MODULE, "gh_api", side_effect=wrong_receipt):
                with self.assertRaisesRegex(MODULE.AdmissionError, message):
                    MODULE.verify_authority_chain(REPOSITORY)
        api = _authority_api()
        original = api
        def wrong_head(path):
            response = original(path)
            if "/pulls/3821" in path:
                response = dict(response)
                response["head"] = {"sha": "6" * 40, "repo": {"full_name": REPOSITORY}}
            return response
        with patch.object(MODULE, "gh_api", side_effect=wrong_head):
            with self.assertRaisesRegex(MODULE.AdmissionError, "source head"):
                MODULE.verify_authority_chain(REPOSITORY)

    def test_authority_chain_rejects_missing_planner_verification_evidence(self):
        api = _authority_api()
        original = api
        def missing_verification(path):
            response = original(path)
            if path.endswith(f"issues/comments/{MODULE.PLANNER_COMMENT}"):
                response = dict(response)
                response["body"] = response["body"].split("Verification commands/results", 1)[0]
            return response
        with patch.object(MODULE, "gh_api", side_effect=missing_verification):
            with self.assertRaisesRegex(MODULE.AdmissionError, "verification"):
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
        with patch.object(MODULE, "verify_authority_chain", return_value=chain), \
             patch.object(MODULE, "verify_executing_planner", return_value={"bytes_sha256": planner_authority["authority_bytes_sha256"]}), \
             patch.object(MODULE, "verify_checker_pr", return_value=checker), \
             patch.object(MODULE, "_git", return_value=TESTED_TREE):
            receipt = MODULE.build_preflight(
                repository=REPOSITORY, pr_number=3827, task_uid=TASK_UID,
                base_oid=CHECKER_BASE, head_oid=CHECKER_HEAD, scope_base_oid=CHECKER_SCOPE,
                repo_root=Path("."), planner_path=Path("planner.py"),
                checker_path=Path("checker"), policy_path=Path("policy"), primary_package="auto",
                run_id="99", run_attempt="1", workflow_ref="workflow", workflow_sha="w" * 40,
            )
        self.assertEqual("preflight", receipt["phase"])
        self.assertEqual(TESTED_TREE, receipt["tested_tree"])
        self.assertEqual(TASK_UID, receipt["task_uid"])
        self.assertEqual(receipt["preflight_digest"], MODULE.preflight_digest(receipt))

    def test_postrun_rejects_missing_failed_wrong_head_and_dirty_receipt(self):
        preflight = {
            "schema": MODULE.SCHEMA, "phase": "preflight", "repository": REPOSITORY,
            "task_uid": TASK_UID, "pr_number": 3827, "base_oid": CHECKER_BASE,
            "head_oid": CHECKER_HEAD, "scope_base_oid": CHECKER_SCOPE,
            "tested_tree": TESTED_TREE, "checker_command_digest": "sha256:" + "a" * 64,
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
            "runner": {"run_id": "99", "run_attempt": "1"},
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

    def test_postrun_receipt_is_durable_and_complete(self):
        preflight = {
            "schema": MODULE.SCHEMA, "phase": "preflight", "repository": REPOSITORY,
            "task_uid": TASK_UID, "pr_number": 3827, "base_oid": CHECKER_BASE,
            "head_oid": CHECKER_HEAD, "scope_base_oid": CHECKER_SCOPE,
            "tested_tree": TESTED_TREE, "checker_command_digest": "sha256:" + "a" * 64,
            "runner": {"run_id": "99", "run_attempt": "1"},
        }
        receipt = MODULE.build_postrun_receipt(
            preflight,
            {"normative": {"merged_commit": NORMATIVE_COMMIT},
             "planner": {"merged_commit": PLANNER_COMMIT}},
            {"path": MODULE.PLANNER_PATH, "source_head": "8" * 40,
             "merged_commit": PLANNER_COMMIT, "bytes_sha256": "sha256:" + "b" * 64},
            {"check_name": "required-gate", "check_app_id": 15368, "check_run_id": 123},
            status="passed", exit_code=0,
        )
        for field in ("normative_authority", "planner_authority", "executing_planner", "check", "result"):
            self.assertIn(field, receipt)
        self.assertEqual("passed", receipt["result"]["status"])
        self.assertIs(MODULE.verify_durable_postrun_receipt(receipt), receipt)
        tampered = dict(receipt, head_oid="9" * 40)
        with self.assertRaisesRegex(MODULE.AdmissionError, "digest"):
            MODULE.verify_durable_postrun_receipt(tampered)


if __name__ == "__main__":
    unittest.main()
