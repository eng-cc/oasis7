#!/usr/bin/env python3
import base64
import hashlib
import json
import importlib.util
import io
import subprocess
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


REPOSITORY = "eng-cc/oasis7"
NORMATIVE_COMMIT = "a" * 40
NORMATIVE_TREE = "b" * 40
NORMATIVE_BLOB = "c" * 40
PLANNER_COMMIT = "d" * 40
PLANNER_TREE = "e" * 40
PLANNER_BLOB = "f" * 40
PLANNER_SOURCE_HEAD = "88988102318fa2e3c9e84f483d01cb99575e5387"
PLANNER_SOURCE_REF = "refs/pull/3821/head"
CHECKER_BASE = "1" * 40
CHECKER_HEAD = "2" * 40
CHECKER_SCOPE = "3" * 40
TESTED_TREE = "4" * 40
TASK_UID = "task_4a631678a50b4fcb952a3c2778b15677"
SUCCESSOR_ISSUE = 3971
CHECKER_PR = 3972
INTEGRATION_RUN = 35463292968
CHECK_RUN_ID = 123


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


def _profile_artifacts():
    plan = {
        "schema": "oasis7-cargo-package-profile-plan/v1",
        "plan_id": "sha256:" + "1" * 64,
        "integration_base": CHECKER_BASE,
        "source_head": "8" * 40,
        "tested_tree": PLANNER_TREE,
    }
    results = [{
        "status": "passed", "exit_code": 0,
        "integration_base": CHECKER_BASE, "source_head": "8" * 40,
        "tested_tree": PLANNER_TREE,
    }]
    receipt = {
        "status": "passed", "plan_id": plan["plan_id"],
        "integration_base": CHECKER_BASE, "source_head": "8" * 40,
        "tested_tree": PLANNER_TREE,
    }
    payloads = {}
    for key, value in (("plan", plan), ("results", results), ("receipt", receipt)):
        payload = json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
        payloads[key] = {"value": value, "bytes": payload}
    envelope = {
        "schema": "oasis7-cargo-package-profile-envelope/v1",
        "repository": REPOSITORY, "task_uid": "task_e21604f5cdb3476c8e146332a68a05b4",
        "pr_number": 3821, "run_id": INTEGRATION_RUN, "run_attempt": 1,
        "check_name": "required-gate", "check_app_id": 15368,
        "check_run_id": CHECK_RUN_ID, "integration_base": CHECKER_BASE,
        "source_head": "8" * 40, "tested_tree": PLANNER_TREE,
        "plan_digest": "sha256:" + hashlib.sha256(payloads["plan"]["bytes"]).hexdigest(),
        "results_digest": "sha256:" + hashlib.sha256(payloads["results"]["bytes"]).hexdigest(),
        "receipt_digest": "sha256:" + hashlib.sha256(payloads["receipt"]["bytes"]).hexdigest(),
    }
    payloads["envelope"] = {"value": envelope, "bytes": json.dumps(envelope, sort_keys=True, separators=(",", ":")).encode()}
    return payloads


def _comment(stage, commit, tree, blob, data, task_uid, pr, comment, fragment):
    digest = hashlib.sha256(data).hexdigest()
    fragment_digest = hashlib.sha256(fragment).hexdigest()
    if stage == "normative_source":
        return (
            "Post-merge approved_normative_source readback "
            "(stage=normative_source; immutable, not candidate authority): "
            f"repository={REPOSITORY}; default_branch=main; task_uid={task_uid}; "
            f"PR={pr}; trusted_predecessor/source_scope_base={CHECKER_BASE}; "
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


class CheckerStageAdmissionTest(unittest.TestCase):
    def setUp(self):
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
        self.assertNotEqual(3827, MODULE.CHECKER_ISSUE)

    def test_stale_legacy_checker_issue_is_never_used_as_a_fallback(self):
        calls = []
        api = _authority_api()

        def recording_api(path):
            calls.append(path)
            return api(path)

        with patch.object(MODULE, "gh_api", side_effect=recording_api):
            self.assertFalse(MODULE.classify_checker_stage(
                REPOSITORY, 3864, MODULE.LEGACY_CHECKER_TASK_UID, CHECKER_BASE, CHECKER_HEAD
            ))
        self.assertTrue(any(path.endswith(f"/issues/{SUCCESSOR_ISSUE}") for path in calls))
        self.assertFalse(any(path.endswith("/issues/3827") for path in calls))

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
                    response["body"] = response["body"].replace(
                        f"{field}={CHECKER_BASE if field == 'trusted_integration_base' else CHECKER_SCOPE}",
                        f"{field}={replacement}",
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
                    f"trusted_predecessor/source_scope_base={CHECKER_BASE}",
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
            if "/actions/runs/" in path:
                response = dict(response)
                response["head_sha"] = "8" * 40
            return response

        with patch.object(MODULE, "gh_api", side_effect=wrong_integration_run):
            with self.assertRaisesRegex(MODULE.AdmissionError, "integration run base"):
                MODULE.verify_authority_chain(REPOSITORY)

    def test_integration_artifact_tested_tree_divergence_is_rejected(self):
        artifacts = _profile_artifacts()
        artifacts["envelope"]["value"] = dict(artifacts["envelope"]["value"], tested_tree="0" * 40)
        with patch.object(MODULE, "gh_api", side_effect=_authority_api()), \
             patch.object(MODULE, "_read_profile_artifacts", return_value=artifacts):
            with self.assertRaisesRegex(MODULE.AdmissionError, "envelope identity"):
                MODULE.verify_authority_chain(REPOSITORY)

    def test_profile_artifact_reader_binds_exact_run_artifacts(self):
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
        self.assertEqual(PLANNER_TREE, loaded["envelope"]["value"]["tested_tree"])

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
        with patch.object(MODULE, "verify_authority_chain", return_value=chain), \
             patch.object(MODULE, "classify_checker_stage", return_value=True), \
             patch.object(MODULE, "verify_executing_planner", return_value={"bytes_sha256": planner_authority["authority_bytes_sha256"]}), \
             patch.object(MODULE, "verify_checker_pr", return_value=checker), \
             patch.object(MODULE, "_git", return_value=TESTED_TREE):
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
        self.assertEqual(receipt["preflight_digest"], MODULE.preflight_digest(receipt))

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

    def test_postrun_receipt_is_durable_and_complete(self):
        preflight = {
            "schema": MODULE.SCHEMA, "phase": "preflight", "repository": REPOSITORY,
            "task_uid": TASK_UID, "pr_number": CHECKER_PR, "base_oid": CHECKER_BASE,
            "head_oid": CHECKER_HEAD, "scope_base_oid": CHECKER_SCOPE,
            "tested_tree": TESTED_TREE, "checker_command": ["python3", "checker"],
            "checker_command_digest": MODULE.command_digest(["python3", "checker"]),
            "runner": {"run_id": "99", "run_attempt": "1"},
        }
        receipt = MODULE.build_postrun_receipt(
            preflight,
            {"normative": {"merged_commit": NORMATIVE_COMMIT},
             "planner": {"merged_commit": PLANNER_COMMIT}},
            {"path": MODULE.PLANNER_PATH, "source_head": "8" * 40,
             "merged_commit": PLANNER_COMMIT, "bytes_sha256": "sha256:" + "b" * 64},
            {"check_name": "required-gate", "check_app_id": 15368, "check_run_id": 123,
             "check_head": CHECKER_HEAD, "workflow_run_id": "99"},
            status="passed", exit_code=0,
        )
        self.assertEqual("provisional", receipt["activation"])
        for field in ("normative_authority", "planner_authority", "executing_planner", "check", "result"):
            self.assertIn(field, receipt)
        self.assertEqual("passed", receipt["result"]["status"])
        self.assertIs(MODULE.verify_durable_postrun_receipt(receipt), receipt)
        tampered = dict(receipt, head_oid="9" * 40)
        with self.assertRaisesRegex(MODULE.AdmissionError, "digest"):
            MODULE.verify_durable_postrun_receipt(tampered)


if __name__ == "__main__":
    unittest.main()
