#!/usr/bin/env python3
"""Exercise exact-W validation replay against a local bare GitHub-shaped fixture."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
GIT = shutil.which("git")
if GIT is None:
    raise RuntimeError("git is required for exact-W boundary fixture")

SPEC = importlib.util.spec_from_file_location(
    "ci_reuse_validation_readback_boundary",
    Path(__file__).with_name("ci_reuse_validation_readback.py"),
)
readback = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(readback)
contract = readback.contract


def run_git(*args: str, cwd: Path | None = None, check: bool = True) -> str:
    result = subprocess.run(
        [GIT, *args], cwd=cwd, check=check, capture_output=True, text=True,
    )
    if check:
        return result.stdout.strip()
    return result.stdout.strip() if result.returncode == 0 else result.stderr.strip()


def load_w_module(w_root: Path, name: str):
    return readback._load_from_root(w_root, name)


def projection_fixture(w_root: Path, *, task_uid: str, head: str, scope: str,
                       projection_digest: str) -> tuple[str, tuple[dict, ...]]:
    projection_contract = load_w_module(w_root, "projection_publication_contract")
    publication_module = load_w_module(w_root, "pr_projection_publication")
    config_path = w_root / "scripts" / "ci-required-scope.v2.json"
    config_digest = "sha256:" + hashlib.sha256(config_path.read_bytes()).hexdigest()
    publication = publication_module.build_task_publication(
        repository=contract.REPOSITORY,
        repository_id=1234,
        task_uid=task_uid,
        bootstrap_epoch=1,
        source_repository_id=1234,
        source_ref="qa/exact-w-boundary",
        target_ref="main",
        source_head_oid=head,
        source_scope_oid=scope,
        planner_authority_oid=run_git("rev-parse", "HEAD", cwd=w_root),
        planner_config_sha256=config_digest,
        policy_digest=projection_contract.digest({"fixture_policy": "exact-w-boundary"}),
        projection_digest=projection_digest,
    )
    _, marker = publication_module.prepare(
        task_uid=task_uid, source_head_oid=head, scope_base_oid=scope,
        projection_digest=projection_digest,
    )
    body = f"Task: {task_uid}\nRefs #{contract.TASK_ISSUE_NUMBER}\n\n{marker}"
    binding = publication_module.build_publication_binding(
        publication, contract.PR_NUMBER,
        f"https://github.com/{contract.REPOSITORY}/pull/{contract.PR_NUMBER}",
    )
    comments = (
        {"body": publication_module.publication_comment(publication)},
        {"body": publication_module.publication_binding_comment(binding)},
    )
    return body, comments


def validation_comments(context) -> tuple[list[dict], dict[str, dict]]:
    """Create reciprocal local Task Issue records for one trusted inventory unit."""
    c = contract
    unit = "required_gate_baseline"
    if unit not in context.planner_unit_obligations:
        raise AssertionError("exact-W full inventory omitted required_gate_baseline")
    authorization = {
        "schema": c.AUTHORIZATION_SCHEMA,
        "repository": c.REPOSITORY,
        "task_uid": context.task_uid,
        "task_issue_number": c.TASK_ISSUE_NUMBER,
        "pr_number": c.PR_NUMBER,
        "head_oid": context.head_oid,
        "integration_base_oid": "4" * 40,
        "source_scope_oid": context.source_scope_oid,
        "projection_digest": context.projection_digest,
        "validation_units": [unit],
        "purpose": c.PURPOSE,
        "decision": c.REQUEST_DECISION,
    }
    authorization_body = c.AUTHORIZATION_MARKER + "\n" + c.canonical_json_bytes(authorization).decode("ascii")
    request = {
        "schema": c.REQUEST_SCHEMA,
        "repository": c.REPOSITORY,
        "task_uid": context.task_uid,
        "task_issue_number": c.TASK_ISSUE_NUMBER,
        "pr_number": c.PR_NUMBER,
        "head_oid": context.head_oid,
        "integration_base_oid": authorization["integration_base_oid"],
        "source_scope_oid": context.source_scope_oid,
        "projection_digest": context.projection_digest,
        "validation_units": [unit],
        "purpose": c.PURPOSE,
        "authorization_decision": c.REQUEST_DECISION,
        "authorization_source": {
            "issue_number": c.TASK_ISSUE_NUMBER,
            "comment_id": 10,
            "body_digest": c.body_digest(authorization_body),
        },
        "authorized_actor": "approval-admin",
    }
    request["request_digest"] = c.request_digest(request)
    request_body = c.REQUEST_MARKER + "\n" + c.canonical_json_bytes(request).decode("ascii")
    pin = {
        "schema": c.PIN_SCHEMA,
        "repository": c.REPOSITORY,
        "task_uid": context.task_uid,
        "task_issue_number": c.TASK_ISSUE_NUMBER,
        "pr_number": c.PR_NUMBER,
        "request_comment_id": 20,
        "request_body_digest": c.body_digest(request_body),
        "request_digest": request["request_digest"],
        "purpose": c.PIN_PURPOSE,
    }
    pin_body = c.PIN_MARKER + "\n" + c.canonical_json_bytes(pin).decode("ascii")
    timestamp = "2026-01-01T00:00:00Z"
    later = "2026-01-01T00:01:00Z"
    latest = "2026-01-01T00:02:00Z"
    comments = [
        {"id": 10, "body": authorization_body, "user": {"login": "approval-admin"},
         "created_at": timestamp, "updated_at": timestamp},
        {"id": 20, "body": request_body, "user": {"login": "requester"},
         "created_at": later, "updated_at": later},
        {"id": 30, "body": pin_body, "user": {"login": "pin-admin"},
         "created_at": latest, "updated_at": latest},
    ]
    permissions = {
        "approval-admin": {"login": "approval-admin", "permission": "admin"},
        "pin-admin": {"login": "pin-admin", "permission": "admin"},
    }
    return comments, permissions


class ExactWBoundaryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temporary = tempfile.TemporaryDirectory(prefix="oasis7-ci-reuse-boundary-test-")
        cls.temp = Path(cls.temporary.name)
        cls.bare = cls.temp / "fixture.git"
        cls.checkout = cls.temp / "checkout"
        cls.w_root = cls.temp / "fixture-w"
        cls.bin = cls.temp / "bin"
        cls.bin.mkdir()
        cls.workflow_sha = run_git("rev-parse", "HEAD", cwd=ROOT)

        subprocess.run([GIT, "init", "--bare", "--quiet", str(cls.bare)], check=True,
                       capture_output=True, text=True)
        run_git("-C", str(ROOT), "push", "--quiet", str(cls.bare),
                f"{cls.workflow_sha}:refs/heads/main")
        run_git("--git-dir", str(cls.bare), "symbolic-ref", "HEAD", "refs/heads/main")
        subprocess.run([GIT, "clone", "--quiet", str(cls.bare), str(cls.checkout)], check=True,
                       capture_output=True, text=True)
        run_git("config", "user.name", "Boundary fixture", cwd=cls.checkout)
        run_git("config", "user.email", "qa-boundary@example.invalid", cwd=cls.checkout)
        changed = cls.checkout / "scripts" / "pm" / "ci-reuse-boundary-fixture.txt"
        changed.write_text("exact-W boundary test input\n", encoding="utf-8")
        run_git("add", str(changed.relative_to(cls.checkout)), cwd=cls.checkout)
        run_git("commit", "--quiet", "-m", "Add local exact-W fixture input", cwd=cls.checkout)
        cls.head = run_git("rev-parse", "HEAD", cwd=cls.checkout)
        run_git("push", "--quiet", "origin", f"{cls.head}:refs/pull/4060/head", cwd=cls.checkout)
        run_git("worktree", "add", "--quiet", "--detach", str(cls.w_root), cls.workflow_sha,
                cwd=cls.checkout)

        gh = cls.bin / "gh"
        gh.write_text(
            "#!/usr/bin/env python3\n"
            "import os, sys\n"
            "if sys.argv[1:4] != ['repo', 'clone', 'eng-cc/oasis7'] or len(sys.argv) != 5:\n"
            "    raise SystemExit(2)\n"
            "os.execv(os.environ['OASIS7_BOUNDARY_TEST_GIT'], [\n"
            "    os.environ['OASIS7_BOUNDARY_TEST_GIT'], 'clone', '--quiet',\n"
            "    os.environ['OASIS7_BOUNDARY_TEST_BARE'], sys.argv[4],\n"
            "])\n",
            encoding="utf-8",
        )
        gh.chmod(0o755)

        cls.task_uid = "task_" + "a" * 32
        cls.base = cls.workflow_sha
        cls.source_scope = cls.base
        cls.projection_digest = load_w_module(cls.w_root, "projection_publication_contract").digest(
            {"fixture_projection": "exact-W source H/B/S/D"},
        )
        cls.pr, cls.issue_comments = cls._pr_and_comments(
            cls.source_scope, cls.projection_digest,
        )
        cls.context = {
            "task_uid": cls.task_uid,
            "head_oid": cls.head,
            "integration_base_oid": cls.base,
            "source_scope_oid": cls.source_scope,
            "projection_digest": cls.projection_digest,
            "repository_id": 1234,
            "source_repository_id": 1234,
        }
        cls.run_identity = {"id": 9001, "run_attempt": 1}
        cls.check = {"id": 9002, "app_id": contract.GITHUB_ACTIONS_APP_ID}

    @classmethod
    def _pr_and_comments(cls, scope: str, digest: str):
        body, issue_comments = projection_fixture(
            cls.w_root, task_uid=cls.task_uid, head=cls.head,
            scope=scope, projection_digest=digest,
        )
        pr = {
            "number": contract.PR_NUMBER,
            "state": "open",
            "merged": False,
            "body": body,
            "head": {"sha": cls.head, "repo": {"full_name": contract.REPOSITORY, "id": 1234}},
            "base": {"sha": cls.base, "ref": "main",
                     "repo": {"full_name": contract.REPOSITORY, "id": 1234}},
        }
        return pr, issue_comments

    @classmethod
    def tearDownClass(cls):
        cls.temporary.cleanup()

    def readback_inventory(self, context=None, pr=None, comments=None):
        env = {
            "PATH": (str(self.bin) + os.pathsep + str(Path(sys.executable).parent)
                     + os.pathsep + os.environ.get("PATH", "")),
            "OASIS7_BOUNDARY_TEST_BARE": str(self.bare),
            "OASIS7_BOUNDARY_TEST_GIT": GIT,
            "GITHUB_REPOSITORY": contract.REPOSITORY,
            "GITHUB_EVENT_NAME": "workflow_dispatch",
            "GITHUB_RUN_ID": str(self.run_identity["id"]),
            "GITHUB_RUN_ATTEMPT": str(self.run_identity["run_attempt"]),
        }
        with patch.dict(os.environ, env):
            return readback._trusted_inventory(
                dict(context or self.context), self.run_identity, self.check,
                pr or self.pr, tuple(comments or self.issue_comments),
                self.workflow_sha, "main",
            )

    def test_real_exact_w_replay_returns_full_units_and_rejects_wrong_m_t(self):
        trusted, merge_oid, tree_oid = self.readback_inventory()
        self.assertEqual(self.task_uid, trusted.task_uid)
        self.assertEqual(self.head, trusted.head_oid)
        self.assertEqual(self.source_scope, trusted.source_scope_oid)
        self.assertEqual(self.projection_digest, trusted.projection_digest)
        self.assertEqual(self.base, self.source_scope)
        self.assertRegex(merge_oid, r"^[0-9a-f]{40}$")
        self.assertRegex(tree_oid, r"^[0-9a-f]{40}$")
        self.assertEqual(tuple(sorted(set(trusted.planner_unit_ids))), trusted.planner_unit_ids)
        self.assertIn("required_gate_baseline", trusted.planner_unit_ids)
        self.assertGreater(len(trusted.planner_unit_ids), 1)
        self.assertEqual(set(trusted.planner_unit_ids), set(trusted.planner_unit_obligations))
        self.assertTrue(all(values and all(type(value) is str for value in values)
                            for values in trusted.planner_unit_obligations.values()))

        context = contract.TrustedRequestContext(
            task_uid=trusted.task_uid, head_oid=trusted.head_oid,
            source_scope_oid=trusted.source_scope_oid,
            projection_digest=trusted.projection_digest,
            planner_unit_ids=trusted.planner_unit_ids,
            planner_unit_obligations=trusted.planner_unit_obligations,
        )
        comments, permissions = validation_comments(context)
        authority = contract.resolve_authority(comments, context, permissions)
        run = {
            "repository": contract.REPOSITORY,
            "id": self.run_identity["id"], "workflow_id": 9003,
            "workflow_path": contract.WORKFLOW_PATH,
            "workflow_ref": contract.WORKFLOW_REF,
            "workflow_sha": self.workflow_sha,
            "event": "workflow_dispatch",
            "display_title": contract.expected_run_title(authority),
            "dispatched_head_sha": self.workflow_sha,
            "run_attempt": self.run_identity["run_attempt"],
            "status": "completed", "conclusion": "success",
            "created_at": "2026-01-01T00:03:00Z",
            "tested_merge_oid": merge_oid, "tested_tree_oid": tree_oid,
        }
        check = {
            "name": contract.CHECK_NAME, "id": self.check["id"],
            "app_id": self.check["app_id"], "head_sha": self.workflow_sha,
            "run_id": self.run_identity["id"], "run_attempt": self.run_identity["run_attempt"],
            "status": "completed", "conclusion": "success",
        }
        record = contract.build_authority_record(authority, run)
        selected = authority.request["validation_units"]
        payload = {
            **record,
            "schema": contract.PAYLOAD_SCHEMA,
            "run_attempt": run["run_attempt"],
            "check_name": check["name"], "check_run_id": check["id"],
            "check_app_id": check["app_id"],
            "selected_obligations": {
                unit: list(authority.planner_unit_obligations[unit]) for unit in selected
            },
            "result_digests": {unit: "sha256:" + "a" * 64 for unit in selected},
            "authority_digest": contract.authority_digest(record),
            "capability_under_test": contract.CAPABILITY,
            "tested_merge_oid": merge_oid, "tested_tree_oid": tree_oid,
            "event_inputs": contract.expected_event_inputs(authority),
        }
        contract.verify_payload(payload, authority, run, check)
        for field, value in (("tested_merge_oid", self.base),
                             ("tested_tree_oid", "0" * 40)):
            forged = dict(payload)
            forged[field] = value
            with self.subTest(field=field), self.assertRaises(contract.ContractError):
                contract.verify_payload(forged, authority, run, check)

    def test_frozen_head_scope_and_projection_mismatches_fail_closed(self):
        bad_head = dict(self.context)
        bad_head["head_oid"] = self.base
        with self.subTest(identity="H"), self.assertRaises(readback.ReadbackError):
            self.readback_inventory(context=bad_head)

        bad_digest = dict(self.context)
        bad_digest["projection_digest"] = "sha256:" + "f" * 64
        with self.subTest(identity="D"), self.assertRaises(readback.ReadbackError):
            self.readback_inventory(context=bad_digest)

        # Publish a self-consistent but false S in both PR and Task publication.
        # The exact B/H merge-base check, rather than a stale D comparison, must reject it.
        false_scope = self.head
        bad_pr, bad_comments = self._pr_and_comments(false_scope, self.projection_digest)
        bad_scope = dict(self.context)
        bad_scope["source_scope_oid"] = false_scope
        with self.subTest(identity="S"), self.assertRaisesRegex(readback.ReadbackError, "merge-base"):
            self.readback_inventory(context=bad_scope, pr=bad_pr, comments=bad_comments)


if __name__ == "__main__":
    unittest.main()
